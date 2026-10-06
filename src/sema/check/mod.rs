//! Type checking of function bodies.
//!
//! [`check_fn`] takes one function instance — a function plus concrete types
//! for its generic parameters — and produces its typed tree. The work is
//! split by construct:
//!
//! * [`expr`] — literals, operators, fields, indexing, struct literals
//! * [`path`] — names, and calls of functions and constructors
//! * [`method`] — method lookup with auto-deref and auto-borrow
//! * [`control`] — `if`, `match`, loops, closures, `return`, `?`
//! * [`pat`] — patterns
//! * [`coerce`] — the implicit conversions allowed at assignment points
//! * [`macros`] — the built-in macros, expanded into ordinary expressions

mod coerce;
mod control;
mod expr;
mod macros;
mod method;
mod pat;
mod path;

use super::context::{AssocType, Context, GenericEnv, PendingProjection, TraitRef, TypeScope};
use super::defs::{ConstId, FnKind, FnOwner, ImplId, ModId};
use super::infer::{InferTable, VarKind};
use super::thir::*;
use super::ty::{ClosureId, TraitId, Ty};
use crate::syntax::ast::{self, PatternKind, StmtKind};
use crate::syntax::diagnostic::{bail, Diagnostic, Result};
use crate::syntax::span::Span;
use std::collections::HashMap;

/// Check the body of a function instance.
pub fn check_fn(tcx: &Context, instance: &Instance) -> Result<Body> {
    let def = tcx.defs.fn_def(instance.def);
    let function = def.ast;
    let Some(ast_body) = &function.body else {
        bail!(function.span, "`{}` has no body to compile", def.path);
    };
    debug_assert_eq!(def.kind, FnKind::Defined);

    let mut infer = InferTable::default();
    let generics = tcx.fn_env(instance.def, &instance.substs);
    let self_ty = tcx.fn_self_ty(instance.def, &generics, &mut infer)?;
    let self_trait = tcx.fn_self_trait(instance.def, &generics, &mut infer)?;
    let sig = tcx.fn_sig(instance.def, &instance.substs, &mut infer, &mut Vec::new())?;

    let mut fcx = FnCtxt::new(tcx, infer, def.module, generics, self_ty, self_trait, sig.ret.clone());
    for (name, ty) in tcx.defs.fn_const_params(instance.def) {
        let ty = fcx.lower_ty(ty)?;
        fcx.const_params.insert(name.to_string(), ty);
    }
    if let FnOwner::Impl(impl_id) = def.owner {
        fcx.collect_assumptions(&tcx.defs.impl_def(impl_id).ast.generics)?;
    }
    fcx.collect_assumptions(&function.generics)?;

    // Parameters. A parameter written as a plain name is the local itself;
    // any other pattern is taken apart by a `let` at the top of the body.
    let mut params = Vec::new();
    let mut destructuring = Vec::new();
    let mut param_tys = sig.params.iter();
    if let Some(receiver) = &function.self_param {
        let ty = param_tys.next().expect("signature includes the receiver").clone();
        params.push(fcx.declare_local("self", ty, receiver.mutable));
    }
    for (param, ty) in function.params.iter().zip(param_tys) {
        match &param.pat.kind {
            PatternKind::Binding { name, mutable, by_ref: None, sub: None } => {
                params.push(fcx.declare_local(&name.name, ty.clone(), *mutable));
            }
            _ => {
                let local = fcx.new_temp(ty.clone());
                params.push(local);
                let pat = fcx.check_pat(&param.pat, ty, BindingMode::Value)?;
                let init = Expr { kind: ExprKind::Local(local), ty: ty.clone(), span: param.pat.span };
                destructuring.push(Stmt::Let { pat, init: Some(init), else_block: None });
            }
        }
    }

    let ret_ty = sig.ret.clone();
    // `-> impl Fn(A) -> R`: the returned closure is checked against that signature.
    if let Some(ast::Type { kind: ast::TypeKind::ImplTrait(bounds), .. }) = &function.ret {
        for (params, ret) in bounds.iter().filter_map(|bound| bound.fn_sugar.as_ref()) {
            let params = params.iter().map(|ty| fcx.lower_ty(ty)).collect::<Result<_>>()?;
            let ret = match ret {
                Some(ret) => fcx.lower_ty(ret)?,
                None => Ty::UNIT,
            };
            fcx.add_callable_bound(ret_ty.clone(), params, ret);
        }
    }
    let body_expr = fcx.check_block_expr(ast_body, Some(&ret_ty))?;
    let body_expr = fcx.coerce(body_expr, &ret_ty)?;
    let value = if destructuring.is_empty() {
        body_expr
    } else {
        let (ty, span) = (body_expr.ty.clone(), body_expr.span);
        let block = Block { stmts: destructuring, expr: Some(Box::new(body_expr)) };
        Expr { kind: ExprKind::Block(block), ty, span }
    };

    fcx.finish(Body { locals: Vec::new(), params, ret_ty, value }, function.span)
}

/// The initialiser of a static that is not plain data (`OnceLock::new()`),
/// checked as the body of a function computing the static's value, which
/// runs before `main`. Constant functions have no side effects, so running
/// one then gives what evaluating it at compile time would.
pub fn check_static_init(tcx: &Context, id: ConstId) -> Result<Body> {
    let constant = tcx.defs.const_def(id);
    let span = constant.ast.name.span;
    let Some(value) = &constant.ast.value else { bail!(span, "static `{}` has no value", constant.ast.name.name) };
    let mut infer = InferTable::default();
    let env = GenericEnv::new();
    let scope = TypeScope { module: constant.module, generics: &env, self_ty: None, self_trait: None };
    let ty = tcx.lower_ty(scope, &constant.ast.ty, &mut infer, &mut Vec::new())?;
    let mut fcx = FnCtxt::new(tcx, infer, constant.module, env, None, None, ty.clone());
    let value = fcx.check_expr_coerce(value, &ty)?;
    fcx.finish(Body { locals: Vec::new(), params: Vec::new(), ret_ty: ty, value }, span)
}

/// The state of checking one function body.
pub(super) struct FnCtxt<'c, 'a> {
    pub tcx: &'c Context<'a>,
    pub infer: InferTable,
    pub module: ModId,
    /// The function's generic parameters and the types they stand for here.
    pub generics: GenericEnv,
    /// The trait `Self::Name` refers to: the one this function belongs to,
    /// or the one its impl implements.
    pub self_trait: Option<TraitRef>,
    pub self_ty: Option<Ty>,

    pub locals: Vec<LocalDecl>,
    /// Lexical scopes, innermost last: which local each visible name refers to.
    scopes: Vec<HashMap<String, LocalId>>,
    /// The return type of the function or closure currently being checked.
    pub ret_ty: Ty,
    loops: Vec<LoopFrame>,
    next_loop: u32,

    /// One frame per closure body we are inside of.
    capture_frames: Vec<CaptureFrame>,
    pending_closures: Vec<PendingClosure>,

    /// Associated types waiting for their base type to be inferred.
    pub projections: Vec<PendingProjection>,
    /// `F: Fn(A) -> R` bounds waiting for `F` to be inferred.
    callable_bounds: Vec<CallableBound>,
    /// `C: Trait<Args>` bounds waiting for `C` to be inferred.
    trait_bounds: Vec<TraitBound>,
    /// Calls of `fn f() -> impl Trait` whose result type becomes known once
    /// the callee's own type arguments are.
    opaque_returns: Vec<(Instance, Ty)>,
    /// Calls of a trait's method whose impl could not be chosen yet
    /// because the receiver was a number literal of undecided type.
    deferred_methods: Vec<Instance>,
    /// Set once the deferred calls are being settled: from then on a call
    /// that is still ambiguous is an error.
    methods_settling: bool,
    /// Variables that a diverging expression (of type `!`) was coerced
    /// to. One that nothing else decides becomes `()`, as under rustc.
    diverging_vars: Vec<Ty>,
    /// While checking the later alternatives of an or-pattern: the locals
    /// the first alternative bound, which the others must bind again.
    or_bindings: Option<HashMap<String, LocalId>>,
    /// The const generic parameters in scope, with their declared types.
    /// Their values are in `generics`.
    const_params: HashMap<String, Ty>,
    /// What the function's own bounds with trait arguments say about the
    /// types it is instantiated with: `S: Into<String>` makes `s.into()` a
    /// `String`.
    assumptions: Vec<(Ty, TraitRef)>,
    /// Set for a method call written in the source, whose trait methods
    /// must come from traits in scope (unlike the calls behind operators,
    /// indexing and `for` loops).
    source_method_call: bool,
    /// While such a call is looked up: the traits it may use; `None` when
    /// every trait counts.
    method_scope: Option<std::rc::Rc<std::collections::HashSet<TraitId>>>,
}

struct LoopFrame {
    id: LoopId,
    label: Option<String>,
    /// The type `break value` must have; `None` for `while` and `for`.
    break_ty: Option<Ty>,
    /// Whether any `break` targets this loop (otherwise it never ends).
    has_break: bool,
    /// A labeled block rather than a loop: only `break 'label` reaches it.
    is_block: bool,
}

struct CaptureFrame {
    /// Locals with a smaller id belong to an enclosing function or closure.
    first_own_local: u32,
    captured: Vec<LocalId>,
}

struct PendingClosure {
    id: ClosureId,
    params: Vec<LocalId>,
    ret_ty: Ty,
    value: Expr,
    captured: Vec<LocalId>,
    is_move: bool,
    span: Span,
}

struct CallableBound {
    ty: Ty,
    params: Vec<Ty>,
    ret: Ty,
}

/// `ty: Trait<args>`. Once `ty` is known, its impl of the trait tells what
/// `args` are — which is how `collect::<Vec<_>>()` learns its element type
/// from `Vec<T>: FromIterator<T>`.
struct TraitBound {
    ty: Ty,
    trait_id: TraitId,
    args: Vec<Ty>,
}

impl<'c, 'a> FnCtxt<'c, 'a> {
    fn new(
        tcx: &'c Context<'a>,
        infer: InferTable,
        module: ModId,
        generics: GenericEnv,
        self_ty: Option<Ty>,
        self_trait: Option<TraitRef>,
        ret_ty: Ty,
    ) -> FnCtxt<'c, 'a> {
        FnCtxt {
            tcx,
            infer,
            module,
            generics,
            self_ty,
            self_trait,
            locals: Vec::new(),
            scopes: vec![HashMap::new()],
            ret_ty,
            loops: Vec::new(),
            next_loop: 0,
            capture_frames: Vec::new(),
            pending_closures: Vec::new(),
            projections: Vec::new(),
            callable_bounds: Vec::new(),
            trait_bounds: Vec::new(),
            opaque_returns: Vec::new(),
            deferred_methods: Vec::new(),
            methods_settling: false,
            diverging_vars: Vec::new(),
            or_bindings: None,
            const_params: HashMap::new(),
            assumptions: Vec::new(),
            source_method_call: false,
            method_scope: None,
        }
    }

    // -- types ------------------------------------------------------------------

    pub fn lower_ty(&mut self, ty: &ast::Type) -> Result<Ty> {
        let scope =
            TypeScope { module: self.module, generics: &self.generics, self_ty: self.self_ty.as_ref(), self_trait: self.self_trait.as_ref() };
        self.tcx.lower_ty(scope, ty, &mut self.infer, &mut self.projections)
    }

    /// Record what the bounds with trait arguments among `generics` say
    /// about this instantiation's types.
    fn collect_assumptions(&mut self, generics: &ast::Generics) -> Result<()> {
        let by_param = generics.params.iter().map(|param| {
            let subject = ast::Type {
                kind: ast::TypeKind::Path(ast::Path::new(
                    vec![ast::PathSegment { ident: param.name.clone(), args: Vec::new(), bindings: Vec::new() }],
                    param.name.span,
                )),
                span: param.name.span,
            };
            (subject, &param.bounds)
        });
        let clauses: Vec<(ast::Type, &Vec<ast::Bound>)> =
            by_param.chain(generics.where_clauses.iter().map(|(ty, bounds)| (ty.clone(), bounds))).collect();
        for (subject, bounds) in clauses {
            for bound in bounds.iter().filter(|bound| bound.fn_sugar.is_none() && !bound.path.last().args.is_empty()) {
                let subject = self.lower_ty(&subject)?;
                let scope = TypeScope {
                    module: self.module,
                    generics: &self.generics,
                    self_ty: self.self_ty.as_ref(),
                    self_trait: self.self_trait.as_ref(),
                };
                if let Some(trait_ref) = self.tcx.lower_trait_ref(scope, &bound.path, &mut self.infer, &mut self.projections)? {
                    self.assumptions.push((subject, trait_ref));
                }
            }
        }
        Ok(())
    }

    /// A constant written in this function: `[0; N]`'s length.
    pub fn lower_const(&mut self, value: &ast::Expr) -> Result<Ty> {
        let scope =
            TypeScope { module: self.module, generics: &self.generics, self_ty: self.self_ty.as_ref(), self_trait: self.self_trait.as_ref() };
        self.tcx.lower_const(scope, value)
    }

    pub fn resolve(&self, ty: &Ty) -> Ty {
        self.infer.resolve(ty)
    }

    /// `ty` with solved variables replaced, rendered for an error message.
    pub fn show(&self, ty: &Ty) -> String {
        self.tcx.display(&self.resolve(ty))
    }

    pub fn unify(&mut self, expected: &Ty, found: &Ty, span: Span) -> Result<()> {
        if self.infer.unify(expected, found).is_err() {
            bail!(span, "mismatched types: expected `{}`, found `{}`", self.show(expected), self.show(found));
        }
        Ok(())
    }

    /// The outermost shape of `ty`, forcing a decision if it is still open:
    /// pending obligations are solved first, and number literals take their
    /// default type. Needed wherever checking depends on the type itself
    /// (field access, method lookup, operators).
    pub fn structurally_resolve(&mut self, ty: &Ty, span: Span) -> Result<Ty> {
        let mut resolved = self.infer.shallow(ty);
        if matches!(resolved, Ty::Infer(_)) {
            self.solve_pending()?;
            resolved = self.infer.shallow(ty);
        }
        if matches!(resolved, Ty::Infer(var) if self.infer.unbound_kind(var) == Some(VarKind::General)) {
            self.solve_pending_with_defaults()?;
            resolved = self.infer.shallow(ty);
        }
        if let Ty::Infer(var) = resolved {
            let default = match self.infer.unbound_kind(var) {
                Some(VarKind::Int) => Ty::I32,
                Some(VarKind::Float) => Ty::F64,
                _ => bail!(span, "type annotations needed: the type of this expression is not known here"),
            };
            self.unify(&default, &resolved, span)?;
            resolved = default;
        }
        Ok(resolved)
    }

    // -- obligations -------------------------------------------------------------

    pub fn add_callable_bound(&mut self, ty: Ty, params: Vec<Ty>, ret: Ty) {
        self.callable_bounds.push(CallableBound { ty, params, ret });
    }

    pub fn add_trait_bound(&mut self, ty: Ty, trait_id: TraitId, args: Vec<Ty>) {
        self.trait_bounds.push(TraitBound { ty, trait_id, args });
    }

    /// The signature a closure is expected to have because of an
    /// `F: Fn(A) -> R` bound on the parameter it is passed to.
    pub fn expected_closure_sig(&self, ty: &Ty) -> Option<(Vec<Ty>, Ty)> {
        let target = self.infer.shallow(ty);
        self.callable_bounds
            .iter()
            .find(|bound| self.infer.shallow(&bound.ty) == target)
            .map(|bound| (bound.params.clone(), bound.ret.clone()))
    }

    /// The parameter and return types of something callable.
    pub fn callable_sig(&mut self, ty: &Ty) -> Result<Option<(Vec<Ty>, Ty)>> {
        self.tcx.callable_sig(ty, &mut self.infer)
    }

    /// The result of calling `instance`, a function declared `-> impl Trait`,
    /// is `output`: the type its body turns out to return.
    pub fn add_opaque_return(&mut self, instance: Instance, output: Ty) {
        self.opaque_returns.push((instance, output));
    }

    /// Make progress on obligations whose subject type has become known.
    pub fn solve_pending(&mut self) -> Result<()> {
        loop {
            let mut progressed = false;

            for projection in std::mem::take(&mut self.projections) {
                if self.solve_projection(&projection)? {
                    progressed = true;
                } else {
                    self.projections.push(projection);
                }
            }

            for bound in std::mem::take(&mut self.callable_bounds) {
                match self.tcx.bound_callable_sig(&bound.ty, &mut self.infer)? {
                    Some((params, ret)) if params.len() == bound.params.len() => {
                        for (expected, found) in bound.params.iter().zip(&params) {
                            let _ = self.infer.unify(expected, found);
                        }
                        let _ = self.infer.unify(&bound.ret, &ret);
                        progressed = true;
                    }
                    Some(_) => progressed = true,
                    None => self.callable_bounds.push(bound),
                }
            }

            for (instance, output) in std::mem::take(&mut self.opaque_returns) {
                let substs: Vec<Ty> = instance.substs.iter().map(|ty| self.resolve(ty)).collect();
                if substs.iter().any(Ty::has_infer) || !substs.iter().all(|ty| self.publish_closure_sigs(ty)) {
                    self.opaque_returns.push((instance, output));
                    continue;
                }
                let body = self.tcx.body(&Instance { def: instance.def, substs })?;
                let _ = self.infer.unify(&output, &body.ret_ty);
                progressed = true;
            }

            for bound in std::mem::take(&mut self.trait_bounds) {
                let subject = self.resolve(&bound.ty);
                if matches!(subject, Ty::Infer(_)) {
                    self.trait_bounds.push(bound);
                    continue;
                }
                // `T: Trait<A>` says what `A` is once a single impl is left:
                // the only one for `T`, or the only one whose arguments can
                // be what is known of `A` (`i64: Sum<{integer}>` is
                // `Sum<i64>`, not `Sum<&i64>`).
                let probed = self.tcx.trait_impls_for(bound.trait_id, &subject, &mut self.infer)?;
                let mut impls: Vec<ImplId> = probed.iter().map(|(impl_id, _)| *impl_id).collect();
                if impls.len() > 1 {
                    impls = self.impls_with_trait_args(&subject, &impls, &bound.args)?;
                }
                match impls.as_slice() {
                    [only] => {
                        let substs = self.tcx.match_impl(*only, &subject, &mut self.infer)?.unwrap_or_default();
                        let args = self.impl_trait_args(bound.trait_id, *only, &substs, &subject)?;
                        let known_before = self.known_parts(&bound.args);
                        for (declared, actual) in bound.args.iter().zip(&args) {
                            let _ = self.infer.unify(declared, actual);
                        }
                        // While the impl's own parameters are open (`F` in
                        // `impl<R, F: FnOnce() -> R> Body<R> for Wrap<F>`),
                        // its bounds may yet say more about the arguments.
                        if substs.iter().any(|ty| self.resolve(ty).has_infer()) {
                            progressed |= self.known_parts(&bound.args) > known_before;
                            self.trait_bounds.push(bound);
                        } else {
                            progressed = true;
                        }
                    }
                    // Not for inference to settle; checking the callee's
                    // body with these types reports it.
                    [] => {}
                    _ => self.trait_bounds.push(bound),
                }
            }

            if !progressed {
                return Ok(());
            }
        }
    }

    /// An error if `def` is a function of an inherent impl, not declared
    /// `pub`, that this function's module is outside of.
    pub fn check_fn_visible(&self, def: crate::sema::ty::FnId, span: Span) -> Result<()> {
        let function = self.tcx.defs.fn_def(def);
        let FnOwner::Impl(impl_id) = function.owner else { return Ok(()) };
        let block = self.tcx.defs.impl_def(impl_id);
        if block.trait_id.is_some() || function.ast.is_pub || self.tcx.defs.sees_private(block.module, self.module) {
            return Ok(());
        }
        let kind = if function.ast.self_param.is_some() { "method" } else { "associated function" };
        bail!(span, "{kind} `{}` is private", function.ast.name.name)
    }

    /// Can this function use field `index` of a variant of `adt`?
    pub fn field_visible(&self, adt: crate::sema::ty::AdtId, variant: u32, index: usize) -> bool {
        let def = self.tcx.defs.adt(adt);
        def.variants[variant as usize].fields[index].is_pub || self.tcx.defs.sees_private(def.module, self.module)
    }

    /// The error for using a field this function cannot see.
    pub fn private_field(&self, adt: crate::sema::ty::AdtId, variant: u32, index: usize, span: Span) -> Diagnostic {
        let def = self.tcx.defs.adt(adt);
        let field = &def.variants[variant as usize].fields[index].name;
        Diagnostic::new(span, format!("field `{field}` of struct `{}` is private", def.name))
    }

    /// How much of these types is known: the parts that are not
    /// inference variables.
    fn known_parts(&self, tys: &[Ty]) -> usize {
        let mut known = 0;
        for ty in tys {
            self.resolve(ty).walk(&mut |part| known += !matches!(part, Ty::Infer(_)) as usize);
        }
        known
    }

    /// Replace each call whose impl choice was deferred (see
    /// [`deferred_methods`](Self::deferred_methods)) by the implementation
    /// for the receiver's now known type.
    fn select_deferred_methods(&mut self, body: &mut Body) -> Result<()> {
        if self.deferred_methods.is_empty() {
            return Ok(());
        }
        self.methods_settling = true;
        let waiting: Vec<Instance> = self
            .deferred_methods
            .iter()
            .map(|instance| Instance { def: instance.def, substs: instance.substs.iter().map(|ty| self.infer.resolve_with_defaults(ty)).collect() })
            .collect();
        let mut calls = Vec::new();
        body.map_types_and_calls(&mut |ty, _| ty.clone(), &mut |instance, span| {
            if waiting.contains(instance) {
                calls.push((instance.clone(), span));
            }
        });
        let mut chosen = Vec::with_capacity(calls.len());
        for (declared, span) in calls {
            let FnOwner::Trait(trait_id) = self.tcx.defs.fn_def(declared.def).owner else { unreachable!("deferred calls name a trait's method") };
            let arity = self.tcx.defs.trait_def(trait_id).ast.generics.params.len();
            let own = declared.substs[1 + arity..].to_vec();
            let self_ty = declared.substs[0].clone();
            chosen.push(self.select_trait_method(trait_id, declared.def, &self_ty, declared.clone(), &own, span)?);
        }
        let mut chosen = chosen.into_iter();
        body.map_types_and_calls(&mut |ty, _| ty.clone(), &mut |instance, _| {
            if waiting.contains(instance) {
                *instance = chosen.next().expect("one choice per deferred call");
            }
        });
        Ok(())
    }

    /// Make the signatures of the closures `ty` mentions usable from other
    /// functions' checks. A closure being checked here has a signature in
    /// terms of this function's inference variables, which mean nothing
    /// elsewhere; once they are solved, the solved signature is published.
    /// Returns `false` if some signature is not fully known yet.
    fn publish_closure_sigs(&mut self, ty: &Ty) -> bool {
        let mut closures = Vec::new();
        ty.walk(&mut |nested| {
            if let Ty::Closure(id) = nested {
                closures.push(*id);
            }
        });
        for id in closures {
            if self.tcx.closure(id).is_some() {
                continue;
            }
            let Some((params, ret)) = self.tcx.closure_sig(id) else { return false };
            let params: Vec<Ty> = params.iter().map(|param| self.resolve(param)).collect();
            let ret = self.resolve(&ret);
            if params.iter().any(Ty::has_infer) || ret.has_infer() {
                return false;
            }
            self.tcx.set_provisional_closure_sig(id, params, ret);
        }
        true
    }

    /// Try to settle `<base>::name == output`. Returns whether it is settled.
    ///
    /// When several impls could provide the associated type, what is already
    /// known about the output decides between them: of all the impls a
    /// half-known type fits, only one may give the `Item` that is expected.
    /// If that does not single one out either, the question stays open.
    fn solve_projection(&mut self, projection: &PendingProjection) -> Result<bool> {
        let base = self.resolve(&projection.base);
        let name = projection.name.as_str();
        let mut nested = Vec::new();
        let trait_ref = projection.trait_ref.as_ref();
        let found = self.tcx.lookup_assoc_type(&base, name, trait_ref, &mut self.infer, &mut nested)?;
        let ty = match found {
            AssocType::Known(ty) => ty,
            AssocType::Missing => {
                bail!(projection.span, "no associated type `{name}` found for `{}`", self.show(&base))
            }
            AssocType::Undecided(impls) => {
                let mut fitting = Vec::new();
                for impl_id in impls {
                    let snapshot = self.infer.snapshot();
                    let ty = self.tcx.assoc_type_in_impl(impl_id, &base, name, trait_ref, &mut self.infer, &mut Vec::new())?;
                    if ty.is_some_and(|ty| self.infer.unify(&projection.output, &ty).is_ok()) {
                        fitting.push(impl_id);
                    }
                    self.infer.rollback_to(snapshot);
                }
                let [only] = fitting.as_slice() else { return Ok(false) };
                let ty = self.tcx.assoc_type_in_impl(*only, &base, name, trait_ref, &mut self.infer, &mut nested)?;
                ty.expect("the impl was found to fit")
            }
        };
        self.projections.append(&mut nested);
        self.unify(&projection.output, &ty, projection.span)?;
        Ok(true)
    }

    /// Settle what [`solve_pending`](Self::solve_pending) had to leave open
    /// because number literals were still undecided: give those literals
    /// their default types, then solve again.
    pub fn solve_pending_with_defaults(&mut self) -> Result<()> {
        self.solve_pending()?;
        loop {
            let bases: Vec<Ty> = self.projections.iter().map(|projection| projection.base.clone()).collect();
            let mut defaulted = false;
            for base in bases {
                defaulted |= self.infer.default_literal_vars(&base);
            }
            if !defaulted {
                return Ok(());
            }
            self.solve_pending()?;
        }
    }

    // -- locals and scopes ---------------------------------------------------------

    pub fn declare_local(&mut self, name: &str, ty: Ty, mutable: bool) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        self.locals.push(LocalDecl { name: name.to_string(), ty, mutable });
        self.scopes.last_mut().expect("at least one scope").insert(name.to_string(), id);
        id
    }

    /// A local that no source name refers to.
    pub fn new_temp(&mut self, ty: Ty) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        self.locals.push(LocalDecl { name: format!("tmp{}", id.0), ty, mutable: true });
        id
    }

    pub fn local_expr(&self, local: LocalId, span: Span) -> Expr {
        Expr { kind: ExprKind::Local(local), ty: self.locals[local.0 as usize].ty.clone(), span }
    }

    pub fn in_scope<T>(&mut self, body: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        self.scopes.push(HashMap::new());
        let result = body(self);
        self.scopes.pop();
        result
    }

    /// The local a name refers to. Using a variable of an enclosing function
    /// from inside a closure records it as captured.
    pub fn lookup_local(&mut self, name: &str) -> Option<LocalId> {
        let local = self.scopes.iter().rev().find_map(|scope| scope.get(name).copied())?;
        for frame in &mut self.capture_frames {
            if local.0 < frame.first_own_local && !frame.captured.contains(&local) {
                frame.captured.push(local);
            }
        }
        Some(local)
    }

    // -- blocks and statements ---------------------------------------------------------

    /// Check a block as an expression. `expected` guides inference of the
    /// tail expression but is not enforced.
    pub fn check_block_expr(&mut self, block: &ast::Block, expected: Option<&Ty>) -> Result<Expr> {
        let (checked, ty) = self.check_block(block, expected)?;
        Ok(Expr { kind: ExprKind::Block(checked), ty, span: block.span })
    }

    pub fn check_block(&mut self, block: &ast::Block, expected: Option<&Ty>) -> Result<(Block, Ty)> {
        self.in_scope(|fcx| {
            let mut stmts = Vec::new();
            let mut diverges = false;
            for stmt in &block.stmts {
                if let Some(checked) = fcx.check_stmt(stmt)? {
                    diverges |= match &checked {
                        Stmt::Expr(expr) | Stmt::Let { init: Some(expr), .. } => {
                            fcx.infer.shallow(&expr.ty) == Ty::Never
                        }
                        Stmt::Let { init: None, .. } => false,
                    };
                    stmts.push(checked);
                }
            }
            let tail = match &block.expr {
                Some(tail) => Some(Box::new(fcx.check_expr(tail, expected)?)),
                None => None,
            };
            let ty = match &tail {
                Some(tail) => tail.ty.clone(),
                None if diverges => Ty::Never,
                None => Ty::UNIT,
            };
            Ok((Block { stmts, expr: tail }, ty))
        })
    }

    fn check_stmt(&mut self, stmt: &ast::Stmt) -> Result<Option<Stmt>> {
        match &stmt.kind {
            // Items were collected up front; they are not executed.
            StmtKind::Item(_) => Ok(None),
            StmtKind::Expr(expr) => Ok(Some(Stmt::Expr(self.check_expr(expr, None)?))),
            StmtKind::Let { pat, ty, init, else_block } => {
                let declared = ty.as_ref().map(|ty| self.lower_ty(ty)).transpose()?;
                // The initialiser is checked before the pattern, so it cannot
                // see the names the pattern introduces.
                let init = match (init, &declared) {
                    (Some(init), Some(ty)) => Some(self.check_expr_coerce(init, ty)?),
                    (Some(init), None) => Some(self.check_expr(init, None)?),
                    (None, _) => None,
                };
                let pat_ty = match (&declared, &init) {
                    (Some(ty), _) => ty.clone(),
                    (None, Some(init)) => init.ty.clone(),
                    (None, None) => self.infer.fresh_var(),
                };
                let pat = self.check_pat(pat, &pat_ty, BindingMode::Value)?;
                let else_block = match else_block {
                    None => None,
                    Some(block) => {
                        let (checked, ty) = self.check_block(block, None)?;
                        if self.infer.shallow(&ty) != Ty::Never {
                            bail!(block.span, "the `else` block of a `let ... else` must diverge");
                        }
                        Some(checked)
                    }
                };
                Ok(Some(Stmt::Let { pat, init, else_block }))
            }
        }
    }

    // -- finishing -------------------------------------------------------------------------

    /// Inference is over: replace every variable by its solution, and hand
    /// the closures defined here over to the program-wide table.
    fn finish(mut self, mut body: Body, span: Span) -> Result<Body> {
        self.solve_pending_with_defaults()?;
        let mut fell_back = false;
        for var in std::mem::take(&mut self.diverging_vars) {
            if let Ty::Infer(var) = self.infer.shallow(&var) {
                fell_back |= self.infer.unify(&Ty::Infer(var), &Ty::UNIT).is_ok();
            }
        }
        if fell_back {
            self.solve_pending_with_defaults()?;
        }

        let infer = &self.infer;
        // The first place whose type is still unknown, for the error message.
        let mut unresolved: Option<Span> = None;
        let mut finalize = |ty: &Ty, at: Option<Span>| {
            let resolved = infer.resolve_with_defaults(ty);
            if resolved.has_infer() && unresolved.is_none() {
                unresolved = Some(at.unwrap_or(span));
            }
            resolved
        };

        body.map_types(&mut finalize);
        for local in &mut self.locals {
            local.ty = finalize(&local.ty, None);
        }
        body.locals = self.locals.clone();

        let mut closures = Vec::new();
        for closure in std::mem::take(&mut self.pending_closures) {
            let mut closure_body = Body {
                locals: self.locals.clone(),
                params: closure.params,
                ret_ty: closure.ret_ty,
                value: closure.value,
            };
            closure_body.map_types(&mut finalize);
            closures.push((closure.id, closure_body, closure.captured, closure.is_move, closure.span));
        }

        // Calls of a trait's method that waited for the receiver's type.
        self.select_deferred_methods(&mut body)?;
        for (id, mut closure_body, captured, is_move, closure_span) in closures {
            self.select_deferred_methods(&mut closure_body)?;
            let captures = captured
                .iter()
                .map(|&local| Capture { local, ty: closure_body.locals[local.0 as usize].ty.clone(), by_ref: !is_move })
                .collect();
            self.tcx.register_closure(id, ClosureDef { body: closure_body, captures, span: closure_span });
        }

        if let Some(at) = unresolved {
            return Err(Diagnostic::new(at, "type annotations needed")
                .with_note("the type of this expression could not be inferred"));
        }
        Ok(body)
    }
}
