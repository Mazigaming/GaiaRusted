//! Names in expression position, and calls.

use super::expr::MethodArgs;
use super::FnCtxt;
use crate::sema::context::{path_text, GenericEnv, PendingProjection, TypeScope};
use crate::sema::defs::{ConstId, Def, FnKind, ImplId, Resolution, VariantShape};
use crate::sema::infer::VarKind;
use crate::sema::thir::*;
use crate::sema::ty::{AdtId, FnId, Mutability, TraitId, Ty};
use crate::syntax::ast::{self, SelfKind};
use crate::syntax::build::AstBuilder;
use crate::syntax::diagnostic::{bail, Diagnostic, Result};
use crate::syntax::span::Span;

/// What a path in expression position refers to.
pub enum ValuePath {
    Local(LocalId),
    Fn(Instance),
    /// A struct or enum variant used as a constructor.
    Constructor(AdtId, Vec<Ty>, u32),
    Const(ConstId),
    Static(ConstId),
    /// `Trait::method`: which impl is meant depends on the argument types.
    TraitMethod(TraitId, FnId),
    /// `Type::method` where several impls of one trait give `Type` a
    /// `method` (`From<A> for B` and `From<B> for B`): the arguments decide.
    TraitMethodOn(TraitId, FnId, Ty),
    /// A trait's default value for one of its constants, for a type whose
    /// impl does not give its own.
    TraitConst(ConstId, Ty),
}

impl FnCtxt<'_, '_> {
    /// `Trait::method` as a value, as in `iter.map(ToString::to_string)`:
    /// which impl it means depends on the arguments it is given, so it is a
    /// closure that calls it, `|a, b| Trait::method(a, b)`.
    fn trait_method_value(&mut self, path: &ast::Path, method: FnId, expected: Option<&Ty>, span: Span) -> Result<Expr> {
        let function = self.tcx.defs.fn_def(method).ast;
        let arity = function.params.len() + function.self_param.is_some() as usize;
        self.calling_closure(path, arity, expected, span)
    }

    /// A closure taking `arity` arguments and passing them on to what
    /// `path` names: `|a, b| path(a, b)`.
    fn calling_closure(&mut self, path: &ast::Path, arity: usize, expected: Option<&Ty>, span: Span) -> Result<Expr> {
        let b = AstBuilder::new(span);
        let names: Vec<String> = (0..arity).map(|index| format!("__arg{index}")).collect();
        let params = names
            .iter()
            .map(|name| ast::ClosureParam {
                pat: ast::Pattern {
                    kind: ast::PatternKind::Binding { name: b.ident(name), mutable: false, by_ref: None, sub: None },
                    span,
                },
                ty: None,
            })
            .collect();
        let callee = ast::Expr { kind: ast::ExprKind::Path(path.clone()), span };
        let body = b.call(callee, names.iter().map(|name| b.var(name)).collect());
        let kind = ast::ExprKind::Closure { params, ret: None, body: Box::new(body), is_move: false };
        self.check_expr(&ast::Expr { kind, span }, expected)
    }

    /// `N` in the body of `fn f<const N: usize>`: the value it has here.
    fn const_param_value(&mut self, path: &ast::Path, span: Span) -> Option<Result<Expr>> {
        let name = &path.as_ident()?.name;
        let ty = self.const_params.get(name)?.clone();
        if self.lookup_local(name).is_some() {
            return None;
        }
        let Ty::Const(value) = self.resolve(&self.generics[name]) else {
            return Some(Err(Diagnostic::new(span, format!("the value of `{name}` is not known here"))));
        };
        let kind = match ty {
            Ty::Bool => ExprKind::Bool(value != 0),
            Ty::Char => ExprKind::Char(char::from_u32(value as u32).unwrap_or_default()),
            _ => ExprKind::Int(value as u128),
        };
        Some(Ok(Expr { kind, ty, span }))
    }

    pub fn resolve_value_path(&mut self, path: &ast::Path) -> Result<ValuePath> {
        if let Some(qself) = &path.qself {
            return self.resolve_qualified(qself, path);
        }
        self.tcx.defs.check_visible(self.module, path)?;
        if let Some(name) = path.as_ident() {
            if let Some(local) = self.lookup_local(&name.name) {
                return Ok(ValuePath::Local(local));
            }
        }
        if let Some((adt, args, variant)) = self.resolve_constructor(path)? {
            return Ok(ValuePath::Constructor(adt, args, variant));
        }

        match self.tcx.defs.resolve_path(self.module, path) {
            Some(Resolution { def: Def::Fn(def), rest: [] }) => {
                let substs = self.own_generic_args(def, &path.last().args, path.span)?;
                Ok(ValuePath::Fn(Instance { def, substs }))
            }
            Some(Resolution { def: Def::Const(id), rest: [] }) => {
                if self.tcx.defs.const_def(id).is_static {
                    Ok(ValuePath::Static(id))
                } else {
                    Ok(ValuePath::Const(id))
                }
            }
            Some(Resolution { def: Def::Trait(trait_id), rest: [member] }) => {
                match self.tcx.defs.trait_def(trait_id).methods.get(&member.ident.name) {
                    Some(&method) => Ok(ValuePath::TraitMethod(trait_id, method)),
                    None => bail!(member.ident.span, "trait has no method named `{}`", member.ident.name),
                }
            }
            Some(Resolution { def: Def::Mod(_) | Def::Trait(_), .. }) => {
                bail!(path.span, "expected a value, found `{}`", path_text(path))
            }
            Some(Resolution { def: Def::Fn(_) | Def::Const(_) | Def::Variant(..), .. }) => {
                bail!(path.span, "cannot find `{}` in this scope", path_text(path))
            }
            // What remains is relative to a type: `Vec::new`, `T::default`, `i64::MAX`.
            Some(Resolution { def: Def::Adt(_) | Def::Alias(_), .. }) | None => {
                if path.segments.len() < 2 {
                    bail!(path.span, "cannot find value `{}` in this scope", path_text(path));
                }
                let scope = TypeScope {
                    module: self.module,
                    generics: &self.generics,
                    self_ty: self.self_ty.as_ref(),
                    self_trait: self.self_trait.as_ref(),
                };
                let (ty, rest) = self.tcx.lower_path_ty_prefix(
                    scope,
                    path,
                    &mut self.infer,
                    &mut self.projections,
                    true,
                )?;
                let [member] = rest else {
                    bail!(path.span, "cannot find `{}` in this scope", path_text(path));
                };
                self.resolve_associated(&ty, member)
            }
        }
    }

    /// `<Type>::member` or `<Type as Trait>::member`.
    fn resolve_qualified(&mut self, qself: &ast::QSelf, path: &ast::Path) -> Result<ValuePath> {
        let ty = self.lower_ty(&qself.ty)?;
        let [member] = path.segments.as_slice() else {
            bail!(path.span, "expected one name after a qualified type, found `{}`", path_text(path));
        };
        let Some(trait_path) = &qself.trait_ref else {
            return self.resolve_associated(&ty, member);
        };
        let Some(Resolution { def: Def::Trait(trait_id), rest: [] }) = self.tcx.defs.resolve_path(self.module, trait_path)
        else {
            bail!(trait_path.span, "cannot find trait `{}`", path_text(trait_path));
        };
        let name = &member.ident.name;
        if let Some(&method) = self.tcx.defs.trait_def(trait_id).methods.get(name) {
            return Ok(ValuePath::TraitMethodOn(trait_id, method, ty));
        }
        if !self.tcx.defs.trait_def(trait_id).consts.contains_key(name) {
            bail!(member.ident.span, "trait has no method or constant named `{name}`");
        }
        // `<T as Trait>::CONST`: the impl's value, or else the trait's.
        let ty = self.structurally_resolve(&ty, path.span)?;
        if let Some((impl_id, _)) = self.tcx.trait_impls_for(trait_id, &ty, &mut self.infer)?.pop() {
            if let Some(&constant) = self.tcx.defs.impl_def(impl_id).consts.get(name) {
                return Ok(ValuePath::Const(constant));
            }
        }
        match self.trait_default_const(&ty, name, Some(trait_id))? {
            Some(constant) => Ok(ValuePath::TraitConst(constant, ty)),
            None => bail!(member.ident.span, "`{}` does not give the constant `{name}` a value", self.show(&ty)),
        }
    }

    /// Type arguments for a function's own generic parameters: the explicit
    /// turbofish if given, fresh inference variables otherwise.
    pub fn own_generic_args(&mut self, def: FnId, explicit: &[ast::Type], span: Span) -> Result<Vec<Ty>> {
        // Explicit arguments are for the parameters that have names; those
        // behind `impl Trait` arguments come after them and are inferred.
        let params = &self.tcx.defs.fn_def(def).ast.generics.params;
        let declared = params.iter().filter(|param| !crate::sema::impl_trait::is_anonymous(&param.name.name)).count();
        let total = params.len();
        if explicit.is_empty() {
            return Ok((0..total).map(|_| self.infer.fresh_var()).collect());
        }
        if explicit.len() != declared {
            bail!(span, "expected {declared} type argument(s), found {}", explicit.len());
        }
        // Anonymous `impl Trait` parameters cannot be named; they are inferred.
        let mut args = explicit.iter().map(|ty| self.lower_ty(ty)).collect::<Result<Vec<_>>>()?;
        args.extend((declared..total).map(|_| self.infer.fresh_var()));
        Ok(args)
    }

    /// `ty::member`: an associated function or constant found in an impl
    /// for `ty`, or a default method of a trait `ty` implements.
    fn resolve_associated(&mut self, ty: &Ty, member: &ast::PathSegment) -> Result<ValuePath> {
        let name = member.ident.name.as_str();
        let span = member.ident.span;
        let ty = self.structurally_resolve(ty, span)?;

        if let Ty::Adt(adt, args) = &ty {
            if let Some(variant) = self.tcx.defs.adt(*adt).variant_index(name).filter(|_| self.tcx.is_enum(*adt)) {
                return Ok(ValuePath::Constructor(*adt, args.clone(), variant));
            }
        }

        let impls = self.tcx.defs.impls_by_member.get(name).cloned().unwrap_or_default();
        // Inherent impls take precedence over trait impls.
        let by_priority = impls
            .iter()
            .filter(|id| self.tcx.defs.impl_def(**id).trait_id.is_none())
            .chain(impls.iter().filter(|id| self.tcx.defs.impl_def(**id).trait_id.is_some()));
        for &impl_id in by_priority.collect::<Vec<_>>() {
            let snapshot = self.infer.snapshot();
            let Some(impl_substs) = self.tcx.match_impl(impl_id, &ty, &mut self.infer)? else {
                continue;
            };
            let block = self.tcx.defs.impl_def(impl_id);
            if let Some(trait_id) = block.trait_id.filter(|_| block.methods.contains_key(name)) {
                if self.tcx.probe_trait_impls(trait_id, &ty, &mut self.infer)?.len() > 1 {
                    // Which impl is meant is for the call to decide; this one
                    // must not have settled `ty` already.
                    self.infer.rollback_to(snapshot);
                    let method = self.tcx.defs.trait_def(trait_id).methods[name];
                    return Ok(ValuePath::TraitMethodOn(trait_id, method, ty));
                }
            }
            if let Some(&constant) = block.consts.get(name) {
                return Ok(ValuePath::Const(constant));
            }
            let def = block.methods[name];
            self.check_fn_visible(def, member.ident.span)?;
            let own = self.own_generic_args(def, &member.args, span)?;
            return Ok(ValuePath::Fn(Instance { def, substs: [impl_substs, own].concat() }));
        }

        for &trait_id in self.tcx.traits_with_method(name) {
            let def = self.tcx.defs.trait_def(trait_id).methods[name];
            if self.tcx.defs.fn_def(def).kind != FnKind::Defined {
                continue;
            }
            if let Some((impl_id, impl_substs)) = self.tcx.trait_impls_for(trait_id, &ty, &mut self.infer)?.pop() {
                let mut substs = self.trait_default_substs(trait_id, impl_id, &impl_substs, &ty)?;
                substs.extend(self.own_generic_args(def, &member.args, span)?);
                return Ok(ValuePath::Fn(Instance { def, substs }));
            }
        }
        if let Some(constant) = self.trait_default_const(&ty, name, None)? {
            return Ok(ValuePath::TraitConst(constant, ty));
        }
        bail!(span, "no function or associated item named `{name}` found for type `{}`", self.show(&ty))
    }

    /// The default value a trait gives its constant `name`, for `ty`, which
    /// implements the trait; with `only`, just that trait's.
    fn trait_default_const(&mut self, ty: &Ty, name: &str, only: Option<TraitId>) -> Result<Option<ConstId>> {
        for (index, def) in self.tcx.defs.traits.iter().enumerate() {
            let trait_id = TraitId(index as u32);
            let Some(&constant) = def.consts.get(name) else { continue };
            if only.is_some_and(|only| only != trait_id) || self.tcx.defs.const_def(constant).ast.value.is_none() {
                continue;
            }
            if self.tcx.implements(ty, trait_id, &mut self.infer)? {
                return Ok(Some(constant));
            }
        }
        Ok(None)
    }

    /// The type arguments a trait's default method runs with when used
    /// through `impl_id`: `Self`, then the trait's own type arguments.
    pub fn trait_default_substs(
        &mut self,
        trait_id: TraitId,
        impl_id: ImplId,
        impl_substs: &[Ty],
        self_ty: &Ty,
    ) -> Result<Vec<Ty>> {
        let mut substs = vec![self_ty.clone()];
        substs.extend(self.impl_trait_args(trait_id, impl_id, impl_substs, self_ty)?);
        Ok(substs)
    }

    /// The trait's type arguments as written in `impl Trait<Args> for Type`.
    pub fn impl_trait_args(
        &mut self,
        trait_id: TraitId,
        impl_id: ImplId,
        impl_substs: &[Ty],
        self_ty: &Ty,
    ) -> Result<Vec<Ty>> {
        self.tcx.impl_trait_args(trait_id, impl_id, impl_substs, self_ty, &mut self.infer)
    }

    pub fn check_path_expr(&mut self, path: &ast::Path, expected: Option<&Ty>, span: Span) -> Result<Expr> {
        if let Some(value) = self.const_param_value(path, span) {
            return value;
        }
        match self.resolve_value_path(path)? {
            ValuePath::Local(local) => Ok(self.local_expr(local, span)),
            ValuePath::Fn(instance) => {
                let ty = Ty::FnItem(instance.def, instance.substs);
                Ok(Expr { kind: ExprKind::ZeroSized, ty, span })
            }
            ValuePath::Constructor(adt, args, variant) => {
                let def = &self.tcx.defs.adt(adt).variants[variant as usize];
                match def.shape {
                    VariantShape::Unit => {}
                    // A tuple-like constructor is a function of its fields,
                    // as in `iter.map(Some)`.
                    VariantShape::Tuple => return self.calling_closure(path, def.fields.len(), expected, span),
                    VariantShape::Named => {
                        bail!(span, "`{}` needs its fields: write `{} {{ ... }}`", def.name, def.name)
                    }
                }
                let ty = Ty::Adt(adt, args);
                if let Some(expected) = expected {
                    self.infer.try_unify(&ty, expected);
                }
                Ok(Expr { kind: ExprKind::Adt { variant, fields: Vec::new(), base: None }, ty, span })
            }
            ValuePath::Const(id) => self.check_const_use(id, None, span),
            ValuePath::TraitConst(id, self_ty) => self.check_const_use(id, Some(self_ty), span),
            ValuePath::Static(id) => {
                let constant = self.tcx.defs.const_def(id);
                let empty = GenericEnv::new();
                let scope = TypeScope { module: constant.module, generics: &empty, self_ty: None, self_trait: None };
                let ty = self.tcx.lower_ty(scope, &constant.ast.ty, &mut self.infer, &mut Vec::new())?;
                Ok(Expr { kind: ExprKind::Static(id), ty, span })
            }
            ValuePath::TraitMethod(_, method) | ValuePath::TraitMethodOn(_, method, _) => {
                self.trait_method_value(path, method, expected, span)
            }
        }
    }

    /// A use of a `const` is replaced by its initialiser, checked in the
    /// scope where the constant was declared.
    /// A constant's value, checked where it is defined. A trait's default
    /// value is checked with `Self` the type it is used for, `self_ty`.
    fn check_const_use(&mut self, id: ConstId, self_ty: Option<Ty>, span: Span) -> Result<Expr> {
        let constant = self.tcx.defs.const_def(id);
        let Some(value) = &constant.ast.value else {
            bail!(span, "constant `{}` has no value", constant.ast.name.name);
        };

        let saved_module = std::mem::replace(&mut self.module, constant.module);
        let saved_scopes = std::mem::replace(&mut self.scopes, vec![Default::default()]);
        let saved_generics = std::mem::take(&mut self.generics);
        let saved_self = self.self_ty.take();
        // An associated constant sees its impl's `Self` (`Self::LOW * 2`),
        // with the impl's parameters left for inference.
        if let Some(owner) = constant.owner {
            let names = self.tcx.defs.impl_generics(owner);
            self.generics = names.iter().map(|name| (name.to_string(), self.infer.fresh_var())).collect();
            self.self_ty = Some(self.tcx.impl_self_ty(owner, &self.generics, &mut self.infer)?);
        }
        if let (Some(trait_id), Some(self_ty)) = (constant.trait_owner, self_ty) {
            self.generics.insert("Self".to_string(), self_ty.clone());
            for param in &self.tcx.defs.trait_def(trait_id).ast.generics.params {
                self.generics.insert(param.name.name.clone(), self.infer.fresh_var());
            }
            self.self_ty = Some(self_ty);
        }
        let result = self
            .lower_ty(&constant.ast.ty)
            .and_then(|ty| self.check_expr_coerce(value, &ty));
        self.module = saved_module;
        self.scopes = saved_scopes;
        self.generics = saved_generics;
        self.self_ty = saved_self;

        let mut expr = result?;
        expr.span = span;
        Ok(expr)
    }

    // -- calls -----------------------------------------------------------------------

    pub fn check_call(
        &mut self,
        callee: &ast::Expr,
        args: &[ast::Expr],
        expected: Option<&Ty>,
        span: Span,
    ) -> Result<Expr> {
        if let ast::ExprKind::Path(path) = &callee.kind {
            match self.resolve_value_path(path)? {
                ValuePath::Fn(instance) => {
                    return self.call_instance(instance, None, MethodArgs::Unchecked(args), expected, span);
                }
                ValuePath::Constructor(adt, type_args, variant) => {
                    return self.check_tuple_constructor(adt, type_args, variant, args, expected, span);
                }
                ValuePath::TraitMethod(trait_id, method) => {
                    return self.check_trait_method_call(trait_id, method, None, path, args, expected, span);
                }
                ValuePath::TraitMethodOn(trait_id, method, self_ty) => {
                    return self.check_trait_method_call(trait_id, method, Some(self_ty), path, args, expected, span);
                }
                ValuePath::Local(_) | ValuePath::Const(_) | ValuePath::Static(_) | ValuePath::TraitConst(..) => {}
            }
        }

        // What is called may sit behind references or a `Box`.
        let mut callee = self.check_expr(callee, None)?;
        let original_ty = callee.ty.clone();
        let (callee_ty, params, ret) = loop {
            let ty = self.structurally_resolve(&callee.ty, callee.span)?;
            if let Some((params, ret)) = self.callable_sig(&ty)? {
                break (ty, params, ret);
            }
            callee = match self.deref_once(callee, span)? {
                Some(inner) => inner,
                None => bail!(span, "expected a function, found `{}`", self.show(&original_ty)),
            };
        };
        match callee_ty {
            Ty::Dyn(..) => {
                // A callable trait object: its vtable's only entry is the call.
                let object = self.borrow_place(callee, Mutability::Not);
                let mut call_args = vec![object];
                call_args.extend(self.check_args(&params, false, MethodArgs::Unchecked(args), span)?);
                Ok(Expr { kind: ExprKind::Call(Callee::Virtual { slot: 0 }, call_args), ty: ret, span })
            }
            Ty::FnItem(def, substs) => {
                self.call_instance(Instance { def, substs }, None, MethodArgs::Unchecked(args), expected, span)
            }
            Ty::Closure(id) => {
                // A closure is called through a pointer to its captured state.
                let env_ty = Ty::mut_ref(callee.ty.clone());
                let env = Expr { kind: ExprKind::AddrOf(Mutability::Mut, Box::new(callee)), ty: env_ty, span };
                let mut call_args = vec![env];
                call_args.extend(self.check_args(&params, false, MethodArgs::Unchecked(args), span)?);
                Ok(Expr { kind: ExprKind::Call(Callee::Closure(id), call_args), ty: ret, span })
            }
            _ => {
                let call_args = self.check_args(&params, false, MethodArgs::Unchecked(args), span)?;
                let kind = ExprKind::Call(Callee::Pointer(Box::new(callee)), call_args);
                Ok(Expr { kind, ty: ret, span })
            }
        }
    }

    /// Call a known function. `receiver`, if given, is the already adjusted
    /// first argument of a method call.
    pub fn call_instance(
        &mut self,
        instance: Instance,
        receiver: Option<Expr>,
        args: MethodArgs,
        expected: Option<&Ty>,
        span: Span,
    ) -> Result<Expr> {
        let sig = self.tcx.fn_sig(instance.def, &instance.substs, &mut self.infer, &mut self.projections)?;
        self.register_bounds(&instance)?;
        if matches!(&self.tcx.defs.fn_def(instance.def).ast.ret, Some(ty) if matches!(ty.kind, ast::TypeKind::ImplTrait(_))) {
            self.add_opaque_return(instance.clone(), sig.ret.clone());
        }
        if let Some(expected) = expected {
            self.hint_closure_signatures(&sig.ret, expected);
        }

        // Knowing the wanted result type early helps infer the arguments
        // (`let v: Vec<i64> = Vec::new()`). A failure only means a coercion
        // will be needed afterwards. Where reaching it may take an unsizing
        // coercion, the arguments come first: `Box::new(x)` expected to be a
        // `Box<dyn Trait>` is still a box of `x`'s own type, and
        // `Rc::new([1, 2])` one of an array, not a slice.
        let mut unsized_inside = false;
        if let Some(expected) = expected {
            self.resolve(expected).walk(&mut |ty| {
                unsized_inside |= match ty {
                    Ty::Dyn(..) => true,
                    Ty::Adt(adt, args) => {
                        matches!(args.first(), Some(Ty::Slice(_) | Ty::Str)) && self.tcx.coerces_unsized(*adt)
                    }
                    _ => false,
                }
            });
            if !unsized_inside && self.resolve(&sig.ret).has_infer() {
                self.infer.try_unify(&sig.ret, expected);
            }
        }

        let mut call_args = Vec::new();
        let mut params = sig.params.as_slice();
        if let Some(receiver) = receiver {
            call_args.push(self.coerce(receiver, &params[0])?);
            params = &params[1..];
        }
        call_args.extend(self.check_args(params, sig.variadic, args, span)?);
        // What the arguments left open, the wanted type may still say
        // (`Rc::from("text")` wanted as an `Rc<str>`).
        if let (true, Some(expected)) = (unsized_inside, expected) {
            if self.resolve(&sig.ret).has_infer() {
                self.infer.try_unify(&sig.ret, expected);
            }
        }
        Ok(Expr { kind: ExprKind::Call(Callee::Fn(instance), call_args), ty: sig.ret, span })
    }

    /// Where the call's result is expected to be a callable trait object
    /// (`Box<dyn Fn(i32) -> i32>` for `Box::new(|x| ..)`), tell the type
    /// variable in that position what signature a closure there should have.
    fn hint_closure_signatures(&mut self, found: &Ty, expected: &Ty) {
        match (self.infer.shallow(found), self.infer.shallow(expected)) {
            (variable @ Ty::Infer(_), wanted @ Ty::Dyn(..)) => {
                if let Ok(Some((params, ret))) = self.callable_sig(&wanted) {
                    self.add_callable_bound(variable, params, ret);
                }
            }
            (Ty::Adt(a, found_args), Ty::Adt(b, expected_args)) if a == b => {
                for (found, expected) in found_args.iter().zip(&expected_args) {
                    self.hint_closure_signatures(found, expected);
                }
            }
            (Ty::Ref(found, _), Ty::Ref(expected, _)) => self.hint_closure_signatures(&found, &expected),
            _ => {}
        }
    }

    /// Check call arguments against parameter types.
    pub fn check_args(
        &mut self,
        params: &[Ty],
        variadic: bool,
        args: MethodArgs,
        span: Span,
    ) -> Result<Vec<Expr>> {
        let supplied = match &args {
            MethodArgs::Unchecked(args) => args.len(),
            MethodArgs::Checked(args) => args.len(),
        };
        if supplied < params.len() || (supplied > params.len() && !variadic) {
            bail!(span, "this call takes {} argument(s) but {supplied} were supplied", params.len());
        }

        let mut checked = Vec::with_capacity(supplied);
        match args {
            MethodArgs::Checked(args) => {
                for (arg, param) in args.into_iter().zip(params) {
                    checked.push(self.coerce(arg, param)?);
                    self.solve_pending()?;
                }
            }
            MethodArgs::Unchecked(args) => {
                // Closures go last: their parameter types often come from
                // the other arguments (`apply(|x| x + 1, 5)`).
                let is_closure = |arg: &ast::Expr| matches!(arg.kind, ast::ExprKind::Closure { .. });
                let mut slots: Vec<Option<Expr>> = args.iter().map(|_| None).collect();
                for closures in [false, true] {
                    for (index, arg) in args.iter().enumerate() {
                        if is_closure(arg) != closures {
                            continue;
                        }
                        slots[index] = Some(match params.get(index) {
                            Some(param) => self.check_expr_coerce(arg, param)?,
                            None => {
                                // An extra argument of a C variadic function.
                                let extra = self.check_expr(arg, None)?;
                                self.structurally_resolve(&extra.ty, extra.span)?;
                                extra
                            }
                        });
                        self.solve_pending()?;
                    }
                }
                checked.extend(slots.into_iter().flatten());
            }
        }
        Ok(checked)
    }

    /// Record what the bounds on the callee's generic parameters say about
    /// types that are still being inferred:
    ///
    /// * `F: Fn(A) -> R` gives a closure passed for `F` its signature, and
    ///   `R` its value once the closure is checked;
    /// * `I: Iterator<Item = T>` equates an associated type with `T`;
    /// * `C: FromIterator<T>` lets the impl chosen for `C` determine `T`.
    fn register_bounds(&mut self, instance: &Instance) -> Result<()> {
        let function = self.tcx.defs.fn_def(instance.def);
        let generics = &function.ast.generics;
        let param_bounds = generics.params.iter().map(|p| (p.name.name.as_str(), &p.bounds));
        let where_bounds = generics.where_clauses.iter().filter_map(|(ty, bounds)| match &ty.kind {
            ast::TypeKind::Path(path) => path.as_ident().map(|name| (name.name.as_str(), bounds)),
            _ => None,
        });
        let bounds: Vec<(&str, &ast::Bound)> = param_bounds
            .chain(where_bounds)
            .flat_map(|(name, bounds)| bounds.iter().map(move |bound| (name, bound)))
            .collect();
        if bounds.is_empty() {
            return Ok(());
        }

        let env = self.tcx.fn_env(instance.def, &instance.substs);
        let self_ty = self.tcx.fn_self_ty(instance.def, &env, &mut self.infer)?;
        let self_trait = self.tcx.fn_self_trait(instance.def, &env, &mut self.infer)?;
        let scope = TypeScope { module: function.module, generics: &env, self_ty: self_ty.as_ref(), self_trait: self_trait.as_ref() };
        for (name, bound) in bounds {
            let Some(subject) = env.get(name).cloned() else { continue };
            let lower = |fcx: &mut Self, ty: &ast::Type| {
                fcx.tcx.lower_ty(scope, ty, &mut fcx.infer, &mut fcx.projections)
            };

            if let Some((params, ret)) = &bound.fn_sugar {
                let params = params.iter().map(|ty| lower(self, ty)).collect::<Result<_>>()?;
                let ret = match ret {
                    Some(ret) => lower(self, ret)?,
                    None => Ty::UNIT,
                };
                self.add_callable_bound(subject, params, ret);
                continue;
            }

            let last = bound.path.last();
            // `P: Pattern`, where closures implement `Pattern` as `FnMut(char) -> bool`.
            if last.args.is_empty() && last.bindings.is_empty() {
                if let Some(Resolution { def: Def::Trait(trait_id), rest: [] }) =
                    self.tcx.defs.resolve_path(function.module, &bound.path)
                {
                    if let Some((params, ret)) = self.tcx.callable_impl_sig(trait_id, &mut self.infer)? {
                        self.add_callable_bound(subject, params, ret);
                    }
                }
                continue;
            }
            let bound_trait = match last.bindings.is_empty() {
                true => None,
                false => self.tcx.lower_trait_ref(scope, &bound.path, &mut self.infer, &mut self.projections)?,
            };
            for (assoc, ty) in &last.bindings {
                let output = lower(self, ty)?;
                self.projections.push(PendingProjection {
                    base: subject.clone(),
                    name: assoc.name.clone(),
                    trait_ref: bound_trait.clone(),
                    output,
                    span: assoc.span,
                });
            }
            if !last.args.is_empty() {
                if let Some(Resolution { def: Def::Trait(trait_id), rest: [] }) =
                    self.tcx.defs.resolve_path(function.module, &bound.path)
                {
                    let args = last.args.iter().map(|ty| lower(self, ty)).collect::<Result<_>>()?;
                    self.add_trait_bound(subject, trait_id, args);
                }
            }
        }
        Ok(())
    }

    /// `Point(1, 2)` / `Some(x)`: a tuple struct or tuple variant applied to its fields.
    fn check_tuple_constructor(
        &mut self,
        adt: AdtId,
        type_args: Vec<Ty>,
        variant: u32,
        args: &[ast::Expr],
        expected: Option<&Ty>,
        span: Span,
    ) -> Result<Expr> {
        let def = &self.tcx.defs.adt(adt).variants[variant as usize];
        if def.shape != VariantShape::Tuple {
            bail!(span, "`{}` is not a tuple struct or tuple variant", def.name);
        }
        if (0..def.fields.len()).any(|index| !self.field_visible(adt, variant, index)) {
            bail!(span, "cannot initialize a tuple struct which contains private fields");
        }
        let ty = Ty::Adt(adt, type_args.clone());
        if let Some(expected) = expected {
            self.infer.try_unify(&ty, expected);
        }
        let field_tys = self.tcx.variant_fields(adt, &type_args, variant, &mut self.infer)?;
        if args.len() != field_tys.len() {
            bail!(span, "`{}` takes {} field(s) but {} were supplied", def.name, field_tys.len(), args.len());
        }
        let mut fields = Vec::new();
        for (index, (arg, field_ty)) in args.iter().zip(&field_tys).enumerate() {
            fields.push((index, self.check_expr_coerce(arg, field_ty)?));
        }
        Ok(Expr { kind: ExprKind::Adt { variant, fields, base: None }, ty, span })
    }

    /// `Trait::method(args)`. `Self` starts as an unknown and is pinned down
    /// by the arguments (or by the expected result, as in `Default::default()`);
    /// then the impl for that type is selected.
    /// A call of a trait's method, `Trait::method(..)` or, with `self_ty`
    /// known, `Type::method(..)`: the impl is chosen once the arguments
    /// are checked.
    #[allow(clippy::too_many_arguments)]
    fn check_trait_method_call(
        &mut self,
        trait_id: TraitId,
        method: FnId,
        self_ty: Option<Ty>,
        path: &ast::Path,
        args: &[ast::Expr],
        expected: Option<&Ty>,
        span: Span,
    ) -> Result<Expr> {
        let trait_arity = self.tcx.defs.trait_def(trait_id).ast.generics.params.len();
        let self_var = self.infer.fresh_var();
        if let Some(self_ty) = &self_ty {
            self.unify(&self_var, self_ty, span)?;
        }
        let mut substs = vec![self_var.clone()];
        // `Trait::<Args>::method` and `<T as Trait<Args>>::method` name the
        // trait's type arguments explicitly (in `Type::method`, the arguments
        // before `method` are the type's).
        let written: &[ast::Type] = match (&path.qself, &self_ty) {
            (Some(qself), _) => qself.trait_ref.as_ref().map_or(&[], |trait_path| &trait_path.last().args),
            (None, Some(_)) => &[],
            (None, None) => &path.segments[path.segments.len() - 2].args,
        };
        if written.is_empty() {
            substs.extend((0..trait_arity).map(|_| self.infer.fresh_var()));
        } else if written.len() == trait_arity {
            for arg in written {
                substs.push(self.lower_ty(arg)?);
            }
        } else {
            bail!(path.span, "expected {trait_arity} type argument(s) for this trait");
        }
        let own = self.own_generic_args(method, &path.last().args, span)?;
        substs.extend(own.iter().cloned());

        let declared = Instance { def: method, substs };
        let call = self.call_instance(declared, None, MethodArgs::Unchecked(args), expected, span)?;
        let ExprKind::Call(Callee::Fn(declared), call_args) = call.kind else {
            unreachable!("call_instance builds a direct call");
        };

        // The receiver may have been passed as a reference to `Self`. A
        // number literal keeps its type open: the impl can wait for it.
        self.solve_pending()?;
        let self_ty = match self.infer.shallow(&self_var) {
            Ty::Infer(var) if matches!(self.infer.unbound_kind(var), Some(VarKind::Int | VarKind::Float)) => Ty::Infer(var),
            _ => self.structurally_resolve(&self_var, span)?,
        };

        // Arrays have the trait impls of slices: `Debug::fmt(&[1, 2, 3], f)`
        // is `Debug::fmt(&[1, 2, 3][..], f)`.
        if let Ty::Array(element, _) = &self_ty {
            let slice = Ty::Slice(element.clone());
            let on_array = self.tcx.trait_impls_for(trait_id, &self_ty, &mut self.infer)?.is_empty();
            let takes_reference = matches!(call_args.first(), Some(first) if matches!(self.resolve(&first.ty), Ty::Ref(..)));
            if on_array && takes_reference {
                let mut call_args = call_args;
                let mut substs = declared.substs.clone();
                substs[0] = slice.clone();
                let Ty::Ref(_, mutability) = self.resolve(&call_args[0].ty) else { unreachable!() };
                let receiver = call_args.remove(0);
                let unsized_ty = Ty::Ref(Box::new(slice.clone()), mutability);
                call_args.insert(0, Expr { kind: ExprKind::Unsize(Box::new(receiver)), ty: unsized_ty, span });
                let declared = Instance { def: method, substs };
                let sig = self.tcx.fn_sig(method, &declared.substs, &mut self.infer, &mut self.projections)?;
                let instance = self.select_trait_method(trait_id, method, &slice, declared, &own, span)?;
                return Ok(Expr { kind: ExprKind::Call(Callee::Fn(instance), call_args), ty: sig.ret, span });
            }
        }

        // `Trait::method(object)` on a trait object goes through its vtable.
        if let Ty::Dyn(object_trait, _) = &self_ty {
            if let Some(slot) = self.tcx.vtable_slot(*object_trait, method) {
                return Ok(Expr { kind: ExprKind::Call(Callee::Virtual { slot }, call_args), ty: call.ty, span });
            }
        }

        let instance = self.select_trait_method(trait_id, method, &self_ty, declared, &own, span)?;
        Ok(Expr { kind: ExprKind::Call(Callee::Fn(instance), call_args), ty: call.ty, span })
    }

    /// Those of `impls`, all fitting `self_ty`, that implement the trait with
    /// arguments that can be `wanted`. Inference is left untouched.
    pub fn impls_with_trait_args(
        &mut self,
        self_ty: &Ty,
        impls: &[ImplId],
        wanted: &[Ty],
    ) -> Result<Vec<ImplId>> {
        // The wanted arguments go in before the impl's bounds are checked:
        // `impl<E: Error> From<E> for Box<dyn Error>` is no impl of
        // `From<&str>`, as `&str` is no `Error`.
        let mut fitting = Vec::new();
        for &impl_id in impls {
            let snapshot = self.infer.snapshot();
            let fits = self.tcx.match_impl_where(impl_id, self_ty, Some(wanted), &mut self.infer)?.is_some();
            self.infer.rollback_to(snapshot);
            if fits {
                fitting.push(impl_id);
            }
        }
        Ok(fitting)
    }

    /// Replace a call of a trait's method declaration by the implementation
    /// that `self_ty` provides (or keep the trait's default body).
    pub fn select_trait_method(
        &mut self,
        trait_id: TraitId,
        method: FnId,
        self_ty: &Ty,
        declared: Instance,
        own_generics: &[Ty],
        span: Span,
    ) -> Result<Instance> {
        let name = &self.tcx.defs.fn_def(method).ast.name.name;
        let trait_name = &self.tcx.defs.trait_def(trait_id).name;
        let mut impls = self.tcx.trait_impls_for(trait_id, self_ty, &mut self.infer)?;
        // `Display::fmt(&n, f)` with `n` a number literal: which impl is
        // meant depends on the type `n` ends up with, which code further on
        // may still decide (`{:>1$}` makes it a `usize`). The call keeps
        // naming the trait's method until the function has been checked.
        let undecided_literal = matches!(self.infer.shallow(self_ty), Ty::Infer(var)
            if matches!(self.infer.unbound_kind(var), Some(VarKind::Int | VarKind::Float)));
        if impls.len() > 1 && undecided_literal {
            self.deferred_methods.push(declared.clone());
            return Ok(declared);
        }
        // Only impls for any type at all (`impl<T: Display> ToString for T`)
        // would fit, and they need to know the type: the literal takes its
        // default one, as Rust does.
        if impls.is_empty() && undecided_literal && self.infer.default_literal_vars(self_ty) {
            impls = self.tcx.trait_impls_for(trait_id, self_ty, &mut self.infer)?;
        }

        // Several impls of a generic trait may fit the type alone
        // (`From<A> for T`, `From<B> for T`); the trait's arguments decide.
        if impls.len() > 1 {
            let wanted = &declared.substs[1..declared.substs.len() - own_generics.len()];
            let candidates: Vec<ImplId> = impls.iter().map(|(impl_id, _)| *impl_id).collect();
            match self.impls_with_trait_args(self_ty, &candidates, wanted)?.as_slice() {
                [only] => {
                    let substs = self.tcx.match_impl(*only, self_ty, &mut self.infer)?.unwrap_or_default();
                    impls = vec![(*only, substs)];
                }
                [] => {
                    let wanted: Vec<String> = wanted.iter().map(|ty| self.show(ty)).collect();
                    bail!(
                        span,
                        "the trait `{trait_name}<{}>` is not implemented for `{}`",
                        wanted.join(", "),
                        self.show(self_ty)
                    );
                }
                _ => {}
            }
        }

        let [(impl_id, impl_substs)] = impls.as_slice() else {
            if impls.is_empty() {
                bail!(span, "the trait `{trait_name}` is not implemented for `{}`", self.show(self_ty));
            }
            // Which impl is meant may follow from code further on
            // (`.map(Rc::from)` collected into a `Vec<Rc<str>>`): the call
            // keeps naming the trait's method until the function has been
            // checked, and the obligation keeps inference informed.
            let arity = declared.substs.len() - 1 - own_generics.len();
            let trait_args = declared.substs[1..1 + arity].to_vec();
            let open = self.resolve(self_ty).has_infer() || trait_args.iter().any(|arg| self.resolve(arg).has_infer());
            if open && !self.methods_settling {
                self.add_trait_bound(self_ty.clone(), trait_id, trait_args);
                self.deferred_methods.push(declared.clone());
                return Ok(declared);
            }
            bail!(span, "type annotations needed: several impls of `{trait_name}` could apply here");
        };
        let trait_args = self.impl_trait_args(trait_id, *impl_id, impl_substs, self_ty)?;
        for (declared_arg, impl_arg) in declared.substs[1..].iter().zip(&trait_args) {
            self.unify(impl_arg, declared_arg, span)?;
        }

        match self.tcx.defs.impl_def(*impl_id).methods.get(name) {
            Some(&def) => Ok(Instance { def, substs: [impl_substs.as_slice(), own_generics].concat() }),
            None if self.tcx.defs.fn_def(method).kind == FnKind::Defined => Ok(declared),
            None => bail!(span, "`{name}` is not implemented for `{}`", self.show(self_ty)),
        }
    }

    /// What kind of receiver a method declares, if it is a method at all.
    pub fn self_kind(&self, def: FnId) -> Option<SelfKind> {
        self.tcx.defs.fn_def(def).ast.self_param.as_ref().map(|receiver| receiver.kind)
    }
}
