//! Program-wide semantic knowledge: declarations plus the queries over them
//! that type checking and code generation share.
//!
//! Generic code is handled by instantiation: a signature or field type is
//! always computed *for given type arguments*, which may be concrete types
//! or inference variables. There is no separate "generic type" form.

use super::defs::{AdtKind, Def, Defs, FnKind, FnOwner, ImplId, ModId, Resolution};
use super::infer::InferTable;
use super::thir::{Body, ClosureDef, Instance};
use super::ty::{AdtId, ClosureId, FloatTy, FnId, IntTy, Mutability, TraitId, Ty, TyHead};
use crate::syntax::ast::{self, SelfKind, TypeKind};
use crate::syntax::diagnostic::{bail, Diagnostic, Result};
use crate::syntax::span::{SourceMap, Span};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// The generic parameters in scope, by name, with the types they stand for.
pub type GenericEnv = HashMap<String, Ty>;

/// Where a written type is being interpreted.
#[derive(Clone, Copy)]
pub struct TypeScope<'s> {
    pub module: ModId,
    pub generics: &'s GenericEnv,
    /// What `Self` means here, if anything.
    pub self_ty: Option<&'s Ty>,
    /// The trait whose associated types `Self::Name` names: in a trait,
    /// that trait; in a trait impl, the trait implemented.
    pub self_trait: Option<&'s TraitRef>,
}

/// A trait with its type arguments: the `TryFrom<&[u8]>` of
/// `<[u8; 4] as TryFrom<&[u8]>>::Error`.
#[derive(Clone, Debug, PartialEq)]
pub struct TraitRef {
    pub trait_id: TraitId,
    pub args: Vec<Ty>,
}

pub struct FnSig {
    /// Parameter types, the receiver (`self`) first.
    pub params: Vec<Ty>,
    pub ret: Ty,
    /// A C variadic function: more arguments may follow the declared ones.
    pub variadic: bool,
}

/// `<base as _>::name` is still unknown because `base` is; `output` stands for it.
pub struct PendingProjection {
    pub base: Ty,
    pub name: String,
    /// The trait the associated type belongs to, when that is known; it
    /// tells apart `TryFrom<&[T]>::Error` and `TryFrom<Vec<T>>::Error`.
    pub trait_ref: Option<TraitRef>,
    pub output: Ty,
    pub span: Span,
}

/// What looking up an associated type came to.
pub enum AssocType {
    Known(Ty),
    /// The type it belongs to is not known well enough yet: it is still a
    /// variable, or it fits each of these impls and they disagree.
    Undecided(Vec<ImplId>),
    /// No impl for the type has an associated type of that name.
    Missing,
}

/// Standard-library definitions the compiler itself needs to know about.
#[derive(Default)]
pub struct LangItems {
    pub string: Option<AdtId>,
    pub option: Option<AdtId>,
    pub result: Option<AdtId>,
    pub vec: Option<AdtId>,
    pub boxed: Option<AdtId>,
    pub range: Option<AdtId>,
    pub range_inclusive: Option<AdtId>,
    pub range_from: Option<AdtId>,
    pub range_to: Option<AdtId>,
    pub range_full: Option<AdtId>,
    pub range_to_inclusive: Option<AdtId>,
    pub deref: Option<TraitId>,
    pub deref_mut: Option<TraitId>,
    pub index: Option<TraitId>,
    pub manually_drop: Option<AdtId>,
    pub copy: Option<TraitId>,
    pub drop: Option<TraitId>,
    /// The pointer types that can point to a value made unsized:
    /// `Rc<T>` to `Rc<dyn Trait>`.
    pub coerce_unsized: Option<TraitId>,
    /// `Fn`, `FnMut` and `FnOnce`.
    pub callable_traits: Vec<TraitId>,
}

pub struct Context<'a> {
    pub defs: Defs<'a>,
    pub sources: &'a SourceMap,
    pub lang: LangItems,
    /// Traits that declare a method of each name.
    traits_by_method: HashMap<String, Vec<TraitId>>,
    /// The impls of each trait, by the trait's index.
    impls_by_trait: Vec<Vec<ImplId>>,
    /// The impls that define an associated type of each name.
    impls_by_assoc_type: HashMap<String, Vec<ImplId>>,
    next_closure: Cell<u32>,
    closures: RefCell<HashMap<ClosureId, Rc<ClosureDef>>>,
    /// Signatures of closures whose defining function is still being
    /// checked; they may mention that function's inference variables.
    provisional_closure_sigs: RefCell<HashMap<ClosureId, (Vec<Ty>, Ty)>>,
    /// Associated types being worked out right now, as (impl, self type).
    assoc_in_progress: RefCell<Vec<(ImplId, Ty)>>,
    /// How deeply bounds with trait arguments are being checked inside one
    /// another, to cut off blanket impls that would go round forever.
    bound_depth: Cell<u32>,
    /// The traits whose methods each module of the program can call.
    traits_in_scope: RefCell<HashMap<ModId, Rc<HashSet<TraitId>>>>,
    needs_drop: RefCell<HashMap<Ty, bool>>,
    bodies: RefCell<HashMap<Instance, Rc<Body>>>,
    /// How each enum type laid out so far records its variant.
    pub(crate) tag_encodings: RefCell<HashMap<Ty, crate::ir::layout::TagEncoding>>,
    /// For each trait asked about, its blanket impl for callables, if any
    /// (see [`callable_impl_sig`](Self::callable_impl_sig)).
    callable_impls: RefCell<HashMap<TraitId, Option<ImplId>>>,
    /// The shape of each impl's self type, worked out on first use; `None`
    /// for an impl for any type (`impl<T> Trait for T`).
    impl_heads: RefCell<HashMap<ImplId, Option<TyHead>>>,
    /// The field types of each variant of each fully known type asked about.
    variant_field_types: RefCell<HashMap<(AdtId, Vec<Ty>, u32), Rc<Vec<Ty>>>>,
    /// The size and alignment of each type laid out so far.
    pub(crate) layouts: RefCell<HashMap<Ty, crate::ir::layout::Layout>>,
}

impl<'a> Context<'a> {
    pub fn new(defs: Defs<'a>, sources: &'a SourceMap) -> Context<'a> {
        let adt = |path: &[&str]| match defs.std_item(path) {
            Some(Def::Adt(id)) => Some(id),
            _ => None,
        };
        let trait_ = |path: &[&str]| match defs.std_item(path) {
            Some(Def::Trait(id)) => Some(id),
            _ => None,
        };
        let lang = LangItems {
            string: adt(&["string", "String"]),
            option: adt(&["option", "Option"]),
            result: adt(&["result", "Result"]),
            vec: adt(&["vec", "Vec"]),
            boxed: adt(&["boxed", "Box"]),
            range: adt(&["ops", "Range"]),
            range_inclusive: adt(&["ops", "RangeInclusive"]),
            range_from: adt(&["ops", "RangeFrom"]),
            range_to: adt(&["ops", "RangeTo"]),
            range_full: adt(&["ops", "RangeFull"]),
            range_to_inclusive: adt(&["ops", "RangeToInclusive"]),
            deref: trait_(&["ops", "Deref"]),
            deref_mut: trait_(&["ops", "DerefMut"]),
            index: trait_(&["ops", "Index"]),
            manually_drop: adt(&["mem", "ManuallyDrop"]),
            copy: trait_(&["marker", "Copy"]),
            drop: trait_(&["ops", "Drop"]),
            coerce_unsized: trait_(&["ops", "CoerceUnsized"]),
            callable_traits: ["Fn", "FnMut", "FnOnce"].iter().filter_map(|name| trait_(&["ops", name])).collect(),
        };

        let mut traits_by_method: HashMap<String, Vec<TraitId>> = HashMap::new();
        for (index, def) in defs.traits.iter().enumerate() {
            for name in def.methods.keys() {
                traits_by_method.entry(name.clone()).or_default().push(TraitId(index as u32));
            }
        }

        let mut impls_by_trait = vec![Vec::new(); defs.traits.len()];
        let mut impls_by_assoc_type: HashMap<String, Vec<ImplId>> = HashMap::new();
        for (index, block) in defs.impls.iter().enumerate() {
            let impl_id = ImplId(index as u32);
            if let Some(trait_id) = block.trait_id {
                impls_by_trait[trait_id.0 as usize].push(impl_id);
            }
            for name in block.assoc_types.keys() {
                impls_by_assoc_type.entry(name.clone()).or_default().push(impl_id);
            }
        }

        Context {
            defs,
            sources,
            lang,
            traits_by_method,
            impls_by_trait,
            impls_by_assoc_type,
            next_closure: Cell::new(0),
            closures: RefCell::default(),
            provisional_closure_sigs: RefCell::default(),
            assoc_in_progress: RefCell::default(),
            bound_depth: Cell::new(0),
            traits_in_scope: RefCell::default(),
            needs_drop: RefCell::default(),
            bodies: RefCell::default(),
            tag_encodings: RefCell::default(),
            callable_impls: RefCell::default(),
            impl_heads: RefCell::default(),
            variant_field_types: RefCell::default(),
            layouts: RefCell::default(),
        }
    }

    pub fn traits_with_method(&self, name: &str) -> &[TraitId] {
        self.traits_by_method.get(name).map_or(&[], Vec::as_slice)
    }

    // -- closures and cached bodies -------------------------------------------

    pub fn fresh_closure_id(&self) -> ClosureId {
        let id = self.next_closure.get();
        self.next_closure.set(id + 1);
        ClosureId(id)
    }

    pub fn register_closure(&self, id: ClosureId, def: ClosureDef) {
        self.closures.borrow_mut().insert(id, Rc::new(def));
    }

    pub fn set_provisional_closure_sig(&self, id: ClosureId, params: Vec<Ty>, ret: Ty) {
        self.provisional_closure_sigs.borrow_mut().insert(id, (params, ret));
    }

    /// The parameter and return types of a closure.
    pub fn closure_sig(&self, id: ClosureId) -> Option<(Vec<Ty>, Ty)> {
        if let Some(closure) = self.closure(id) {
            let body = &closure.body;
            let params = body.params.iter().map(|p| body.locals[p.0 as usize].ty.clone());
            return Some((params.collect(), body.ret_ty.clone()));
        }
        self.provisional_closure_sigs.borrow().get(&id).cloned()
    }

    /// A closure defined by a function that has already been checked.
    pub fn closure(&self, id: ClosureId) -> Option<Rc<ClosureDef>> {
        self.closures.borrow().get(&id).cloned()
    }

    /// The checked body of a function instance (checked on first request).
    pub fn body(&self, instance: &Instance) -> Result<Rc<Body>> {
        if let Some(body) = self.bodies.borrow().get(instance) {
            return Ok(body.clone());
        }
        let body = Rc::new(super::check::check_fn(self, instance)?);
        self.bodies.borrow_mut().insert(instance.clone(), body.clone());
        Ok(body)
    }

    // -- environments -----------------------------------------------------------

    /// Pair a function's generic parameter names with an instance's arguments.
    pub fn fn_env(&self, def: FnId, substs: &[Ty]) -> GenericEnv {
        let names = self.defs.fn_generics(def);
        debug_assert_eq!(names.len(), substs.len(), "wrong number of type arguments");
        names.into_iter().zip(substs.iter().cloned()).collect()
    }

    pub fn adt_env(&self, adt: AdtId, args: &[Ty]) -> GenericEnv {
        let params = &self.defs.adt(adt).generics.params;
        params.iter().map(|p| p.name.name.clone()).zip(args.iter().cloned()).collect()
    }

    /// What `Self` is inside the given function, under `env`.
    pub fn fn_self_ty(&self, def: FnId, env: &GenericEnv, infer: &mut InferTable) -> Result<Option<Ty>> {
        match self.defs.fn_def(def).owner {
            FnOwner::Free => Ok(None),
            FnOwner::Trait(_) => Ok(env.get("Self").cloned()),
            FnOwner::Impl(impl_id) => self.impl_self_ty(impl_id, env, infer).map(Some),
        }
    }

    /// The trait `Self::Name` refers to inside the given function, under
    /// `env`: its own trait's, or the trait its impl implements.
    pub fn fn_self_trait(&self, def: FnId, env: &GenericEnv, infer: &mut InferTable) -> Result<Option<TraitRef>> {
        match self.defs.fn_def(def).owner {
            FnOwner::Free => Ok(None),
            FnOwner::Trait(trait_id) => {
                let params = &self.defs.trait_def(trait_id).ast.generics.params;
                let args = params.iter().map(|param| env[&param.name.name].clone()).collect();
                Ok(Some(TraitRef { trait_id, args }))
            }
            FnOwner::Impl(impl_id) => {
                let Some(trait_id) = self.defs.impl_def(impl_id).trait_id else { return Ok(None) };
                let substs: Vec<Ty> = self.defs.impl_generics(impl_id).iter().map(|name| env[*name].clone()).collect();
                let self_ty = self.impl_self_ty(impl_id, env, infer)?;
                let args = self.impl_trait_args(trait_id, impl_id, &substs, &self_ty, infer)?;
                Ok(Some(TraitRef { trait_id, args }))
            }
        }
    }

    /// The trait a path such as `Iterator<Item = T>` or `TryFrom<U>` names,
    /// with its type arguments: those written, or fresh variables.
    pub fn lower_trait_ref(
        &self,
        scope: TypeScope,
        path: &ast::Path,
        infer: &mut InferTable,
        pending: &mut Vec<PendingProjection>,
    ) -> Result<Option<TraitRef>> {
        self.defs.check_visible(scope.module, path)?;
        let Some(Resolution { def: Def::Trait(trait_id), rest: [] }) = self.defs.resolve_path(scope.module, path) else {
            return Ok(None);
        };
        let arity = self.defs.trait_def(trait_id).ast.generics.params.len();
        let written = &path.last().args;
        let args = if written.len() == arity {
            written.iter().map(|arg| self.lower_ty(scope, arg, infer, pending)).collect::<Result<_>>()?
        } else {
            (0..arity).map(|_| infer.fresh_var()).collect()
        };
        Ok(Some(TraitRef { trait_id, args }))
    }

    /// The shape of the self type of an impl, if it has a fixed one.
    fn impl_head(&self, id: ImplId) -> Option<TyHead> {
        if let Some(&head) = self.impl_heads.borrow().get(&id) {
            return head;
        }
        let mut infer = InferTable::default();
        let env: GenericEnv = self.defs.impl_generics(id).iter().map(|name| (name.to_string(), infer.fresh_var())).collect();
        let head = self.impl_self_ty(id, &env, &mut infer).ok().and_then(|ty| infer.shallow(&ty).head());
        self.impl_heads.borrow_mut().insert(id, head);
        head
    }

    pub fn impl_self_ty(&self, id: ImplId, env: &GenericEnv, infer: &mut InferTable) -> Result<Ty> {
        let def = self.defs.impl_def(id);
        let scope = TypeScope { module: def.module, generics: env, self_ty: None, self_trait: None };
        self.lower_ty(scope, &def.ast.self_ty, infer, &mut Vec::new())
    }

    // -- signatures and fields ----------------------------------------------------

    pub fn fn_sig(
        &self,
        def: FnId,
        substs: &[Ty],
        infer: &mut InferTable,
        pending: &mut Vec<PendingProjection>,
    ) -> Result<FnSig> {
        let function = self.defs.fn_def(def);
        let env = self.fn_env(def, substs);
        let self_ty = self.fn_self_ty(def, &env, infer)?;
        let self_trait = self.fn_self_trait(def, &env, infer)?;
        let scope = TypeScope { module: function.module, generics: &env, self_ty: self_ty.as_ref(), self_trait: self_trait.as_ref() };

        let mut params = Vec::new();
        if let Some(receiver) = &function.ast.self_param {
            let Some(self_ty) = self_ty.clone() else {
                bail!(receiver.span, "`self` parameter outside of an `impl` or `trait`");
            };
            params.push(match receiver.kind {
                SelfKind::Value => self_ty,
                SelfKind::Ref => Ty::shared_ref(self_ty),
                SelfKind::RefMut => Ty::mut_ref(self_ty),
            });
        }
        for param in &function.ast.params {
            params.push(self.lower_ty(scope, &param.ty, infer, pending)?);
        }
        let ret = match &function.ast.ret {
            // `-> impl Trait` is whatever type the body returns; the caller
            // of `fn_sig` finds out what that is.
            Some(ast::Type { kind: TypeKind::ImplTrait(_), .. }) => infer.fresh_var(),
            Some(ty) => self.lower_ty(scope, ty, infer, pending)?,
            None => Ty::UNIT,
        };
        Ok(FnSig { params, ret, variadic: function.ast.is_variadic })
    }

    /// The field types of one variant of `adt<args>`.
    pub fn variant_fields(
        &self,
        adt: AdtId,
        args: &[Ty],
        variant: u32,
        infer: &mut InferTable,
    ) -> Result<Vec<Ty>> {
        let def = self.defs.adt(adt);
        let env = self.adt_env(adt, args);
        let scope = TypeScope { module: def.module, generics: &env, self_ty: None, self_trait: None };
        let fields = &def.variants[variant as usize].fields;
        fields.iter().map(|field| self.lower_ty(scope, field.ty, infer, &mut Vec::new())).collect()
    }

    /// Field types of a variant of a fully concrete type.
    /// The captures a closure holds by value and must drop, each with the
    /// field of the environment that says whether it still holds it: the
    /// body clears that flag when it moves the capture out (`move || drop(x)`),
    /// so dropping the closure afterwards leaves it alone. The flags follow
    /// the captures in the environment.
    pub fn closure_drop_flags(&self, id: ClosureId) -> Vec<(usize, usize)> {
        let Some(closure) = self.closure(id) else { return Vec::new() };
        let owned = closure
            .captures
            .iter()
            .enumerate()
            .filter(|(_, capture)| !capture.by_ref && self.needs_drop(&capture.ty))
            .map(|(index, _)| index);
        owned.enumerate().map(|(flag, capture)| (capture, closure.captures.len() + flag)).collect()
    }

    /// Types with no compile-time size: `str`, slices, trait objects, and
    /// structs whose last field is one of those (`Path` holds a `str`). A
    /// pointer to one carries the extra word of what it ends with.
    /// The signature a closure must have to implement `trait_id` through a
    /// blanket impl for callables, `impl<F: FnMut(char) -> bool> Pattern for
    /// F`, if the trait has one: a closure passed for a `P: Pattern` gets
    /// its parameter types from there.
    pub fn callable_impl_sig(&self, trait_id: TraitId, infer: &mut InferTable) -> Result<Option<(Vec<Ty>, Ty)>> {
        let Some(impl_id) = self.callable_impl(trait_id) else { return Ok(None) };
        let block = self.defs.impl_def(impl_id);
        let param = block.ast.generics.params.iter().find(|param| {
            matches!(&block.ast.self_ty.kind, ast::TypeKind::Path(path) if path.as_ident().is_some_and(|name| name.name == param.name.name))
        });
        let (params, ret) = param.and_then(|param| param.bounds.iter().find_map(|bound| bound.fn_sugar.as_ref())).expect("found as callable");
        let names = self.defs.impl_generics(impl_id);
        let env: GenericEnv = names.iter().map(|name| (name.to_string(), infer.fresh_var())).collect();
        let scope = TypeScope { module: block.module, generics: &env, self_ty: None, self_trait: None };
        let params = params.iter().map(|ty| self.lower_ty(scope, ty, infer, &mut Vec::new())).collect::<Result<Vec<_>>>()?;
        let ret = match ret {
            Some(ret) => self.lower_ty(scope, ret, infer, &mut Vec::new())?,
            None => Ty::UNIT,
        };
        Ok(Some((params, ret)))
    }

    /// The impl through which any type `F: Fn..(..)` implements `trait_id`,
    /// if there is one: an impl for every `F: FnMut(char) -> bool`, or for
    /// every `P: CharPredicate` where `CharPredicate` has such an impl.
    fn callable_impl(&self, trait_id: TraitId) -> Option<ImplId> {
        if let Some(&found) = self.callable_impls.borrow().get(&trait_id) {
            return found;
        }
        // Blanket impls can lead round in a circle; meanwhile, there is none.
        self.callable_impls.borrow_mut().insert(trait_id, None);
        let found = self.defs.impls.iter().enumerate().find_map(|(index, block)| {
            if block.trait_id != Some(trait_id) {
                return None;
            }
            let ast::TypeKind::Path(self_path) = &block.ast.self_ty.kind else { return None };
            let self_name = self_path.as_ident()?;
            let param = block.ast.generics.params.iter().find(|param| param.name.name == self_name.name)?;
            if param.bounds.iter().any(|bound| bound.fn_sugar.is_some()) {
                return Some(ImplId(index as u32));
            }
            param.bounds.iter().find_map(|bound| match self.defs.resolve_path(block.module, &bound.path) {
                Some(Resolution { def: Def::Trait(inner), rest: [] }) if bound.path.last().args.is_empty() => {
                    self.callable_impl(inner)
                }
                _ => None,
            })
        });
        self.callable_impls.borrow_mut().insert(trait_id, found);
        found
    }

    /// Can a value pointed to by this pointer type be made unsized through
    /// it (`Box`, `Rc`, `Arc`)?
    pub fn coerces_unsized(&self, adt: AdtId) -> bool {
        let Some(coerce_unsized) = self.lang.coerce_unsized else { return false };
        let mut infer = InferTable::default();
        let args = self.defs.adt(adt).generics.params.iter().map(|_| infer.fresh_var()).collect();
        self.implements(&Ty::Adt(adt, args), coerce_unsized, &mut infer).unwrap_or(false)
    }

    pub fn is_unsized(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Adt(adt, args) if !self.is_enum(*adt) => {
                let fields = self.concrete_variant_fields(*adt, args, 0);
                fields.last().is_some_and(|last| self.is_unsized(last))
            }
            other => other.is_unsized(),
        }
    }

    pub fn concrete_variant_fields(&self, adt: AdtId, args: &[Ty], variant: u32) -> Vec<Ty> {
        let known = !args.iter().any(Ty::has_infer);
        if known {
            if let Some(fields) = self.variant_field_types.borrow().get(&(adt, args.to_vec(), variant)) {
                return fields.as_ref().clone();
            }
        }
        let fields = self.compute_variant_fields(adt, args, variant);
        if known {
            self.variant_field_types.borrow_mut().insert((adt, args.to_vec(), variant), Rc::new(fields.clone()));
        }
        fields
    }

    fn compute_variant_fields(&self, adt: AdtId, args: &[Ty], variant: u32) -> Vec<Ty> {
        let mut infer = InferTable::default();
        let fields = self
            .variant_fields(adt, args, variant, &mut infer)
            .expect("field types were validated when the type was first used");
        // An associated type in a field (`Option<I::Item>`) is worked out by
        // matching an impl, whose parameters live in this table. Arguments
        // still being inferred belong to the caller's table instead.
        if args.iter().any(Ty::has_infer) {
            return fields;
        }
        fields.iter().map(|field| infer.resolve(field)).collect()
    }

    pub fn is_extern_or_intrinsic(&self, def: FnId) -> bool {
        matches!(self.defs.fn_def(def).kind, FnKind::Extern | FnKind::Intrinsic)
    }

    // -- impl selection -------------------------------------------------------------

    /// Does `impl_id` apply to `self_ty`? On success, returns the types its
    /// generic parameters take (possibly inference variables).
    pub fn match_impl(
        &self,
        impl_id: ImplId,
        self_ty: &Ty,
        infer: &mut InferTable,
    ) -> Result<Option<Vec<Ty>>> {
        self.match_impl_where(impl_id, self_ty, None, infer)
    }

    /// Match an impl of a trait for `self_ty` whose trait arguments are
    /// `trait_args`. Knowing them before the impl's bounds are checked lets
    /// `impl<T, U: Into<T>> TryFrom<U> for T` be ruled out for `[u8; 3]`
    /// and `&[u8]`, which no `From` impl connects.
    pub fn match_impl_where(
        &self,
        impl_id: ImplId,
        self_ty: &Ty,
        trait_args: Option<&[Ty]>,
        infer: &mut InferTable,
    ) -> Result<Option<Vec<Ty>>> {
        if let (Some(impl_head), Some(head)) = (self.impl_head(impl_id), infer.shallow(self_ty).head()) {
            if impl_head != head {
                return Ok(None);
            }
        }
        let names = self.defs.impl_generics(impl_id);
        let snapshot = infer.snapshot();
        let substs: Vec<Ty> = names.iter().map(|_| infer.fresh_var()).collect();
        let env: GenericEnv = names.iter().map(|n| n.to_string()).zip(substs.iter().cloned()).collect();

        let impl_self = self.impl_self_ty(impl_id, &env, infer)?;
        let mut matches = infer.unify(&impl_self, self_ty).is_ok();
        if let (true, Some(wanted), Some(trait_id)) = (matches, trait_args, self.defs.impl_def(impl_id).trait_id) {
            let actual = self.impl_trait_args(trait_id, impl_id, &substs, self_ty, infer)?;
            matches = actual.len() == wanted.len()
                && actual.iter().zip(wanted).all(|(actual, wanted)| infer.unify(actual, wanted).is_ok());
        }
        let matches = matches
            && self.impl_bounds_hold(impl_id, &impl_self, &env, infer)?
            && self.bounds_agree(impl_id, &env, infer)?;
        if !matches {
            infer.rollback_to(snapshot);
            return Ok(None);
        }
        Ok(Some(substs))
    }

    /// Does `ty` implement the trait with the given trait arguments?
    /// Inference is left untouched.
    pub fn implements_with_args(&self, ty: &Ty, trait_id: TraitId, args: &[Ty], infer: &mut InferTable) -> Result<bool> {
        /// Deeper than any chain of blanket impls a program builds on purpose.
        const MAX_DEPTH: u32 = 16;
        if self.bound_depth.get() >= MAX_DEPTH {
            return Ok(true);
        }
        self.bound_depth.set(self.bound_depth.get() + 1);
        let mut found = Ok(false);
        for impl_id in self.probe_trait_impls(trait_id, ty, infer)? {
            let snapshot = infer.snapshot();
            let fits = self.match_impl_where(impl_id, ty, Some(args), infer);
            infer.rollback_to(snapshot);
            match fits {
                Ok(Some(_)) => {
                    found = Ok(true);
                    break;
                }
                Ok(None) => {}
                Err(error) => {
                    found = Err(error);
                    break;
                }
            }
        }
        self.bound_depth.set(self.bound_depth.get() - 1);
        found
    }

    /// The bounds on an impl's parameters carry information beyond "this
    /// trait is implemented":
    ///
    /// * `I: Iterator<Item = &T>` says what an associated type is;
    /// * `K: Borrow<Q>` relates two parameters through the impl that
    ///   provides `Borrow` for `K`;
    /// * `F: FnMut() -> Option<B>` says what calling a closure returns.
    ///
    /// Besides restricting when the impl applies, this is how parameters
    /// that do not appear in the impl's self type (`T`, `Q` and `B` above)
    /// get their value.
    fn bounds_agree(&self, impl_id: ImplId, env: &GenericEnv, infer: &mut InferTable) -> Result<bool> {
        let block = self.defs.impl_def(impl_id);
        let scope = TypeScope { module: block.module, generics: env, self_ty: None, self_trait: None };
        for param in &block.ast.generics.params {
            let subject = infer.resolve(&env[&param.name.name]);
            if matches!(subject, Ty::Infer(_)) {
                continue;
            }
            for bound in &param.bounds {
                let last = bound.path.last();
                for (name, wanted) in &last.bindings {
                    let Some(actual) = self.assoc_type(&subject, &name.name, infer)? else { return Ok(false) };
                    let wanted = self.lower_ty(scope, wanted, infer, &mut Vec::new())?;
                    if infer.unify(&actual, &wanted).is_err() {
                        return Ok(false);
                    }
                }
                if let Some((written_params, written_ret)) = &bound.fn_sugar {
                    // `F: FnMut() -> Option<T>` gives `T` the value of what
                    // calling `F` returns.
                    let Some((params, ret)) = self.bound_callable_sig(&subject, infer)? else { continue };
                    if params.len() != written_params.len() {
                        return Ok(false);
                    }
                    for (written, actual) in written_params.iter().zip(&params) {
                        let written = self.lower_ty(scope, written, infer, &mut Vec::new())?;
                        if infer.unify(&written, actual).is_err() {
                            return Ok(false);
                        }
                    }
                    let written_ret = match written_ret {
                        Some(ty) => self.lower_ty(scope, ty, infer, &mut Vec::new())?,
                        None => Ty::UNIT,
                    };
                    if infer.unify(&written_ret, &ret).is_err() {
                        return Ok(false);
                    }
                    continue;
                }
                if last.args.is_empty() {
                    continue;
                }
                let Some(Resolution { def: Def::Trait(trait_id), rest: [] }) =
                    self.defs.resolve_path(block.module, &bound.path)
                else {
                    continue;
                };
                // Only an unambiguous impl tells us anything.
                if let [(provider, substs)] = self.trait_impls_for(trait_id, &subject, infer)?.as_slice() {
                    let actual = self.impl_trait_args(trait_id, *provider, substs, &subject, infer)?;
                    for (written, actual) in last.args.iter().zip(&actual) {
                        let written = self.lower_ty(scope, written, infer, &mut Vec::new())?;
                        if infer.unify(&written, actual).is_err() {
                            return Ok(false);
                        }
                    }
                }
            }
        }
        Ok(true)
    }

    /// The trait's type arguments as written in `impl Trait<Args> for Type`.
    /// Omitted arguments default to `Self`, as in `impl Add for Point`.
    pub fn impl_trait_args(
        &self,
        trait_id: TraitId,
        impl_id: ImplId,
        impl_substs: &[Ty],
        self_ty: &Ty,
        infer: &mut InferTable,
    ) -> Result<Vec<Ty>> {
        let arity = self.defs.trait_def(trait_id).ast.generics.params.len();
        let block = self.defs.impl_def(impl_id);
        let written = &block.ast.trait_ref.as_ref().expect("a trait impl names its trait").last().args;
        if written.is_empty() {
            return Ok(vec![self_ty.clone(); arity]);
        }
        let names = self.defs.impl_generics(impl_id);
        let env: GenericEnv = names.iter().map(|n| n.to_string()).zip(impl_substs.iter().cloned()).collect();
        let scope = TypeScope { module: block.module, generics: &env, self_ty: Some(self_ty), self_trait: None };
        written.iter().map(|ty| self.lower_ty(scope, ty, infer, &mut Vec::new())).collect()
    }

    /// The traits whose methods code in `module` can call with method
    /// syntax: those it declares or imports, and the prelude's. `None` in
    /// the standard library, which sees every trait.
    pub fn traits_in_scope(&self, module: ModId) -> Option<Rc<HashSet<TraitId>>> {
        let mut root = module;
        while let Some(parent) = self.defs.module(root).parent {
            root = parent;
        }
        if root == super::defs::STD_ROOT {
            return None;
        }
        if let Some(known) = self.traits_in_scope.borrow().get(&module) {
            return Some(known.clone());
        }
        let traits_of = |module: ModId| {
            self.defs.module(module).names.values().filter_map(|def| match def {
                Def::Trait(trait_id) => Some(*trait_id),
                _ => None,
            })
        };
        let visible: Rc<HashSet<TraitId>> = Rc::new(traits_of(module).chain(traits_of(self.defs.prelude)).collect());
        self.traits_in_scope.borrow_mut().insert(module, visible.clone());
        Some(visible)
    }

    /// Does the impl name its trait's type arguments (`impl PartialEq<str>
    /// for String`) rather than leave them to default to `Self`?
    pub fn writes_trait_args(&self, impl_id: ImplId) -> bool {
        let block = self.defs.impl_def(impl_id).ast;
        block.trait_ref.as_ref().is_some_and(|trait_ref| !trait_ref.last().args.is_empty())
    }

    /// Is this `impl<T> Trait for T`, an impl for every type at once?
    fn is_blanket_impl(&self, impl_id: ImplId) -> bool {
        let block = self.defs.impl_def(impl_id).ast;
        let TypeKind::Path(path) = &block.self_ty.kind else { return false };
        path.as_ident().is_some_and(|name| block.generics.params.iter().any(|param| param.name.name == name.name))
    }

    /// Do the impl's parameters satisfy their trait bounds? `impl<T: Bound>
    /// Trait for T` matches every type syntactically, so for it the bounds
    /// are what decide whether it applies at all; `impl<I: Iterator>
    /// Iterator for &mut I` must not apply to `&mut &[T]`.
    ///
    /// A parameter whose type is not known yet cannot be checked, and only
    /// the blanket impl's own parameter is then taken not to hold.
    fn impl_bounds_hold(&self, impl_id: ImplId, impl_self: &Ty, env: &GenericEnv, infer: &mut InferTable) -> Result<bool> {
        let block = self.defs.impl_def(impl_id).ast;
        let module = self.defs.impl_def(impl_id).module;
        let blanket_param = match &block.self_ty.kind {
            TypeKind::Path(path) if self.is_blanket_impl(impl_id) => path.as_ident().map(|name| name.name.clone()),
            _ => None,
        };
        if let Some(name) = &blanket_param {
            let resolved = infer.resolve(impl_self);
            if matches!(resolved, Ty::Infer(_)) {
                return Ok(false);
            }
            // `impl<T> Trait for T` is for sized types only, unless `T: ?Sized`.
            let param = block.generics.params.iter().find(|param| param.name.name == *name);
            if self.is_unsized(&resolved) && param.is_some_and(|param| !param.maybe_unsized) {
                return Ok(false);
            }
        }
        for param in &block.generics.params {
            let Some(subject) = env.get(&param.name.name) else { continue };
            let subject = infer.resolve(subject);
            if matches!(subject, Ty::Infer(_)) {
                continue;
            }
            for bound in &param.bounds {
                let Some(Resolution { def: Def::Trait(required), .. }) = self.defs.resolve_path(module, &bound.path) else {
                    continue;
                };
                let written = &bound.path.last().args;
                let holds = if bound.fn_sugar.is_none() && !written.is_empty() {
                    let scope = TypeScope { module, generics: env, self_ty: None, self_trait: None };
                    let args = written
                        .iter()
                        .map(|arg| self.lower_ty(scope, arg, infer, &mut Vec::new()))
                        .collect::<Result<Vec<_>>>()?;
                    self.implements_with_args(&subject, required, &args, infer)?
                } else {
                    self.implements(&subject, required, infer)?
                };
                if !holds {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// The impls of `trait_id` that apply to `self_ty`, with their type arguments.
    pub fn trait_impls_for(
        &self,
        trait_id: TraitId,
        self_ty: &Ty,
        infer: &mut InferTable,
    ) -> Result<Vec<(ImplId, Vec<Ty>)>> {
        let found = self.probe_trait_impls(trait_id, self_ty, infer)?;
        // Commit to the bindings only when the choice is unambiguous.
        let mut result = Vec::new();
        let commit = found.len() == 1;
        for impl_id in found {
            let snapshot = infer.snapshot();
            let substs = self.match_impl(impl_id, self_ty, infer)?.expect("matched a moment ago");
            if !commit {
                infer.rollback_to(snapshot);
            }
            result.push((impl_id, substs));
        }
        Ok(result)
    }

    /// The impls of `trait_id` that could apply to `self_ty`, leaving
    /// inference untouched.
    pub fn probe_trait_impls(&self, trait_id: TraitId, self_ty: &Ty, infer: &mut InferTable) -> Result<Vec<ImplId>> {
        let mut found = Vec::new();
        for &impl_id in &self.impls_by_trait[trait_id.0 as usize] {
            let snapshot = infer.snapshot();
            let matched = self.match_impl(impl_id, self_ty, infer)?;
            infer.rollback_to(snapshot);
            if matched.is_some() {
                found.push(impl_id);
            }
        }
        Ok(found)
    }

    /// The parameter and return types of something that can be called.
    pub fn callable_sig(&self, ty: &Ty, infer: &mut InferTable) -> Result<Option<(Vec<Ty>, Ty)>> {
        Ok(match infer.shallow(ty) {
            Ty::FnPtr(params, ret) => Some((params, *ret)),
            Ty::FnItem(def, substs) => {
                let sig = self.fn_sig(def, &substs, infer, &mut Vec::new())?;
                Some((sig.params, sig.ret))
            }
            Ty::Closure(id) => self.closure_sig(id),
            Ty::Dyn(trait_id, args) if self.lang.callable_traits.contains(&trait_id) => match args.as_slice() {
                [Ty::FnPtr(params, ret)] => Some((params.clone(), (**ret).clone())),
                _ => None,
            },
            _ => None,
        })
    }

    /// The signature an `F: Fn(..)` bound sees `ty` callable with: its own,
    /// or that of what a `Box` or reference holds, since those implement
    /// the `Fn` traits too (`Box<dyn Fn(i32) -> i32>` is an `Fn(i32) -> i32`).
    pub fn bound_callable_sig(&self, ty: &Ty, infer: &mut InferTable) -> Result<Option<(Vec<Ty>, Ty)>> {
        let mut ty = infer.shallow(ty);
        loop {
            if let Some(sig) = self.callable_sig(&ty, infer)? {
                return Ok(Some(sig));
            }
            ty = match ty {
                Ty::Ref(inner, _) => infer.shallow(&inner),
                Ty::Adt(adt, args) if Some(adt) == self.lang.boxed => infer.shallow(&args[0]),
                _ => return Ok(None),
            };
        }
    }

    pub fn implements(&self, ty: &Ty, trait_id: TraitId, infer: &mut InferTable) -> Result<bool> {
        // Functions and closures implement the `Fn` traits by nature.
        if self.lang.callable_traits.contains(&trait_id) {
            return Ok(self.bound_callable_sig(ty, infer)?.is_some());
        }
        // Pointers, tuples, arrays and closures are `Copy` by their shape.
        if Some(trait_id) == self.lang.copy {
            let resolved = infer.resolve(ty);
            if !matches!(resolved, Ty::Adt(..) | Ty::Infer(_)) && !resolved.has_infer() {
                return Ok(self.is_copy(&resolved));
            }
        }
        // A trait object implements its trait and that trait's supertraits.
        if let Ty::Dyn(object_trait, _) = infer.shallow(ty) {
            if self.vtable_entries(object_trait).iter().any(|&(of_trait, _)| of_trait == trait_id) {
                return Ok(true);
            }
        }
        let snapshot = infer.snapshot();
        let mut found = false;
        for &impl_id in &self.impls_by_trait[trait_id.0 as usize] {
            if self.match_impl(impl_id, ty, infer)?.is_some() {
                found = true;
                break;
            }
        }
        infer.rollback_to(snapshot);
        Ok(found)
    }

    /// Is a value of this type duplicated by a plain bitwise copy?
    pub fn is_copy(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Int(_) | Ty::Float(_) | Ty::Bool | Ty::Char | Ty::Never => true,
            Ty::Ptr(..) | Ty::FnPtr(..) | Ty::FnItem(..) => true,
            Ty::Ref(_, mutability) => !mutability.is_mut(),
            Ty::Tuple(elements) => elements.iter().all(|element| self.is_copy(element)),
            Ty::Array(element, _) => self.is_copy(element),
            Ty::Adt(..) => self.lang.copy.is_some_and(|copy| {
                self.implements(ty, copy, &mut InferTable::default()).unwrap_or(false)
            }),
            Ty::Closure(id) => self.closure(*id).is_some_and(|closure| {
                closure.captures.iter().all(|capture| capture.by_ref || self.is_copy(&capture.ty))
            }),
            Ty::Str | Ty::Slice(_) | Ty::Dyn(..) | Ty::Const(_) | Ty::Infer(_) => false,
        }
    }

    /// The `Drop::drop` implementation for a type, if it has one.
    pub fn drop_impl(&self, ty: &Ty) -> Option<Instance> {
        let drop = self.lang.drop?;
        let mut infer = InferTable::default();
        let (impl_id, substs) = self.trait_impls_for(drop, ty, &mut infer).ok()?.pop()?;
        let def = *self.defs.impl_def(impl_id).methods.get("drop")?;
        Some(Instance { def, substs: substs.iter().map(|ty| infer.resolve(ty)).collect() })
    }

    /// Does anything have to happen when a value of this type goes out of
    /// scope? True if the type implements `Drop` or owns something that does.
    pub fn needs_drop(&self, ty: &Ty) -> bool {
        if let Some(&known) = self.needs_drop.borrow().get(ty) {
            return known;
        }
        let needs = match ty {
            // What it holds is dropped when its owner says so.
            Ty::Adt(adt, _) if Some(*adt) == self.lang.manually_drop => false,
            Ty::Adt(adt, args) => {
                self.drop_impl(ty).is_some() || {
                    let variants = 0..self.defs.adt(*adt).variants.len() as u32;
                    variants.into_iter().any(|variant| {
                        self.concrete_variant_fields(*adt, args, variant).iter().any(|field| self.needs_drop(field))
                    })
                }
            }
            Ty::Tuple(elements) => elements.iter().any(|element| self.needs_drop(element)),
            Ty::Array(element, len) => **len != Ty::Const(0) && self.needs_drop(element),
            // A trait object may be anything; its vtable knows.
            Ty::Dyn(..) => true,
            // A closure drops what it holds by value, unless its body has
            // moved it out: see `closure_drop_flags`.
            Ty::Closure(id) => !self.closure_drop_flags(*id).is_empty(),
            _ => false,
        };
        self.needs_drop.borrow_mut().insert(ty.clone(), needs);
        needs
    }

    /// What calling a function or closure returns: the associated type
    /// `Output` of the callable traits.
    fn call_output(&self, self_ty: &Ty, name: &str, infer: &mut InferTable) -> Result<Option<Ty>> {
        if name != "Output" {
            return Ok(None);
        }
        Ok(self.bound_callable_sig(self_ty, infer)?.map(|(_, ret)| ret))
    }

    /// The impls that define an associated type `name` and apply to
    /// `self_ty`, leaving inference untouched. They all implement one trait;
    /// more than one means `self_ty` is not known well enough to choose, as
    /// with `Range<{integer}>`.
    ///
    /// Two traits may both name an associated type `Item`. A type's own impls
    /// then come before blanket impls, which only pass on what those say
    /// (`IntoIterator::Item` of an iterator is its `Iterator::Item`).
    ///
    /// An impl cannot define its associated type in terms of itself: in
    /// `impl<I: Iterator> IntoIterator for I { type Item = I::Item; }` the
    /// inner `Item` is `Iterator`'s. So an impl whose associated type is
    /// being evaluated for this very type is left out.
    ///
    /// With `trait_ref`, only impls of that trait with those trait arguments
    /// count.
    pub fn assoc_type_impls(
        &self,
        self_ty: &Ty,
        name: &str,
        trait_ref: Option<&TraitRef>,
        infer: &mut InferTable,
    ) -> Result<Vec<ImplId>> {
        let resolved_self = infer.resolve(self_ty);
        let mut matching = Vec::new();
        let candidates = self.impls_by_assoc_type.get(name).map_or(&[][..], Vec::as_slice);
        for &impl_id in candidates {
            let def = self.defs.impl_def(impl_id);
            let in_progress = self.assoc_in_progress.borrow().iter().any(|(id, ty)| *id == impl_id && *ty == resolved_self);
            if in_progress {
                continue;
            }
            if trait_ref.is_some_and(|trait_ref| def.trait_id != Some(trait_ref.trait_id)) {
                continue;
            }
            let snapshot = infer.snapshot();
            let matched = self.match_impl_where(impl_id, self_ty, trait_ref.map(|t| t.args.as_slice()), infer)?;
            infer.rollback_to(snapshot);
            if matched.is_some() {
                matching.push(impl_id);
            }
        }
        if matching.iter().any(|&impl_id| !self.is_blanket_impl(impl_id)) {
            matching.retain(|&impl_id| !self.is_blanket_impl(impl_id));
        }
        if let Some(&first) = matching.first() {
            let trait_id = self.defs.impl_def(first).trait_id;
            matching.retain(|&impl_id| self.defs.impl_def(impl_id).trait_id == trait_id);
        }
        Ok(matching)
    }

    /// The associated type `name` as `impl_id` defines it for `self_ty`.
    /// Commits inference to that impl. `None` if the impl does not apply.
    pub fn assoc_type_in_impl(
        &self,
        impl_id: ImplId,
        self_ty: &Ty,
        name: &str,
        trait_ref: Option<&TraitRef>,
        infer: &mut InferTable,
        pending: &mut Vec<PendingProjection>,
    ) -> Result<Option<Ty>> {
        let def = self.defs.impl_def(impl_id);
        let trait_args = trait_ref.map(|trait_ref| trait_ref.args.as_slice());
        let Some(substs) = self.match_impl_where(impl_id, self_ty, trait_args, infer)? else { return Ok(None) };
        let names = self.defs.impl_generics(impl_id);
        let env: GenericEnv = names.iter().map(|n| n.to_string()).zip(substs).collect();
        let scope = TypeScope { module: def.module, generics: &env, self_ty: Some(self_ty), self_trait: None };
        self.assoc_in_progress.borrow_mut().push((impl_id, infer.resolve(self_ty)));
        let ty = self.lower_ty(scope, def.assoc_types[name], infer, pending);
        self.assoc_in_progress.borrow_mut().pop();
        ty.map(Some)
    }

    /// The variable of `self_ty` that every one of `impls` makes the
    /// associated type, if there is one.
    ///
    /// `Range<{integer}>` fits the `Iterator` impl of every integer type, but
    /// in each of them `Item` is that same integer. So `Item` is known to be
    /// the literal's type without deciding yet what type that is.
    fn assoc_type_shared_var(
        &self,
        impls: &[ImplId],
        self_ty: &Ty,
        name: &str,
        trait_ref: Option<&TraitRef>,
        infer: &mut InferTable,
    ) -> Result<Option<Ty>> {
        let mut shared = Vec::new();
        infer.resolve(self_ty).walk(&mut |ty| {
            if matches!(ty, Ty::Infer(_)) && !shared.contains(ty) {
                shared.push(ty.clone());
            }
        });
        for &impl_id in impls {
            let snapshot = infer.snapshot();
            let assoc = self.assoc_type_in_impl(impl_id, self_ty, name, trait_ref, infer, &mut Vec::new())?;
            let assoc = assoc.map(|ty| infer.resolve(&ty));
            shared.retain(|var| Some(infer.resolve(var)) == assoc);
            infer.rollback_to(snapshot);
        }
        Ok(match shared.as_slice() {
            [only] => Some(only.clone()),
            _ => None,
        })
    }

    /// The associated type, if every one of `impls` makes it the same fully
    /// known type: `u8`'s `TryFrom<i32>` and `TryFrom<i64>` impls both have
    /// `TryFromIntError` as their `Error`, so which one is meant does not
    /// matter.
    fn assoc_type_agreed(
        &self,
        impls: &[ImplId],
        self_ty: &Ty,
        name: &str,
        trait_ref: Option<&TraitRef>,
        infer: &mut InferTable,
    ) -> Result<Option<Ty>> {
        let mut agreed: Option<Ty> = None;
        for &impl_id in impls {
            let snapshot = infer.snapshot();
            let assoc = self.assoc_type_in_impl(impl_id, self_ty, name, trait_ref, infer, &mut Vec::new())?;
            let assoc = assoc.map(|ty| infer.resolve(&ty));
            infer.rollback_to(snapshot);
            match (assoc, &agreed) {
                (Some(ty), _) if ty.has_infer() => return Ok(None),
                (Some(ty), None) => agreed = Some(ty),
                (Some(ty), Some(previous)) if ty == *previous => {}
                _ => return Ok(None),
            }
        }
        Ok(agreed)
    }

    /// Look up the associated type `name` of `self_ty` as far as what is
    /// known about `self_ty` allows, without guessing.
    pub fn lookup_assoc_type(
        &self,
        self_ty: &Ty,
        name: &str,
        trait_ref: Option<&TraitRef>,
        infer: &mut InferTable,
        pending: &mut Vec<PendingProjection>,
    ) -> Result<AssocType> {
        if let Ty::Dyn(trait_id, args) = infer.shallow(self_ty) {
            if let Some(value) = self.object_assoc_type(trait_id, &args, name) {
                return Ok(AssocType::Known(value));
            }
        }
        if let Some(output) = self.call_output(self_ty, name, infer)? {
            return Ok(AssocType::Known(output));
        }
        if matches!(infer.shallow(self_ty), Ty::Infer(_)) {
            return Ok(AssocType::Undecided(Vec::new()));
        }
        let impls = self.assoc_type_impls(self_ty, name, trait_ref, infer)?;
        Ok(match impls.as_slice() {
            [] => AssocType::Missing,
            [only] => {
                let ty = self.assoc_type_in_impl(*only, self_ty, name, trait_ref, infer, pending)?;
                AssocType::Known(ty.expect("the impl was found to apply"))
            }
            _ => match self.assoc_type_shared_var(&impls, self_ty, name, trait_ref, infer)? {
                Some(var) => AssocType::Known(var),
                None => match self.assoc_type_agreed(&impls, self_ty, name, trait_ref, infer)? {
                    Some(ty) => AssocType::Known(ty),
                    None => AssocType::Undecided(impls),
                },
            },
        })
    }

    /// The value a trait object gives an associated type of its trait:
    /// `u32` for the `Item` of `dyn Iterator<Item = u32>`.
    pub fn object_assoc_type(&self, trait_id: TraitId, args: &[Ty], name: &str) -> Option<Ty> {
        let def = self.defs.trait_def(trait_id);
        let index = def.assoc_types.iter().position(|assoc| assoc == name)?;
        args.get(def.ast.generics.params.len() + index).cloned()
    }

    /// The value of associated type `name` for `self_ty`, e.g. `Item` for an
    /// iterator. If several impls could apply because a number literal's type
    /// is still open, the literal takes its default type first.
    pub fn assoc_type(&self, self_ty: &Ty, name: &str, infer: &mut InferTable) -> Result<Option<Ty>> {
        let mut found = self.lookup_assoc_type(self_ty, name, None, infer, &mut Vec::new())?;
        if matches!(found, AssocType::Undecided(_)) && infer.default_literal_vars(self_ty) {
            found = self.lookup_assoc_type(self_ty, name, None, infer, &mut Vec::new())?;
        }
        match found {
            AssocType::Known(ty) => Ok(Some(ty)),
            AssocType::Undecided(impls) => match impls.first() {
                Some(&first) => self.assoc_type_in_impl(first, self_ty, name, None, infer, &mut Vec::new()),
                None => Ok(None),
            },
            AssocType::Missing => Ok(None),
        }
    }

    // -- type lowering ----------------------------------------------------------------

    /// Interpret a written type.
    ///
    /// `_` becomes a fresh inference variable. An associated type of a type
    /// that is not known yet (`I::Item` while `I` is still a variable) becomes
    /// a variable too, and the obligation to solve it is added to `pending`.
    pub fn lower_ty(
        &self,
        scope: TypeScope,
        ty: &ast::Type,
        infer: &mut InferTable,
        pending: &mut Vec<PendingProjection>,
    ) -> Result<Ty> {
        let mut lower = |inner: &ast::Type| self.lower_ty(scope, inner, infer, pending);
        Ok(match &ty.kind {
            TypeKind::Path(path) => return self.lower_path_ty(scope, path, infer, pending, false),
            TypeKind::Ref { mutable, inner } => {
                Ty::Ref(Box::new(lower(inner)?), Mutability::from_bool(*mutable))
            }
            TypeKind::Ptr { mutable, inner } => {
                Ty::Ptr(Box::new(lower(inner)?), Mutability::from_bool(*mutable))
            }
            TypeKind::Tuple(elements) => Ty::Tuple(elements.iter().map(lower).collect::<Result<_>>()?),
            TypeKind::Slice(element) => Ty::Slice(Box::new(lower(element)?)),
            TypeKind::Array(element, len) => {
                let element = lower(element)?;
                let len = self.lower_const(scope, len)?;
                if matches!(len, Ty::Const(value) if value < 0) {
                    bail!(ty.span, "array length cannot be negative");
                }
                Ty::Array(Box::new(element), Box::new(len))
            }
            TypeKind::Const(value) => self.lower_const(scope, value)?,
            TypeKind::Fn { params, ret } => {
                let params = params.iter().map(&mut lower).collect::<Result<_>>()?;
                let ret = match ret {
                    Some(ret) => lower(ret)?,
                    None => Ty::UNIT,
                };
                Ty::FnPtr(params, Box::new(ret))
            }
            TypeKind::Dyn(bounds) => {
                // `+ Send` and the like add no methods.
                let is_auto = |bound: &&ast::Bound| {
                    matches!(bound.path.last().ident.name.as_str(), "Send" | "Sync" | "Unpin")
                };
                let principal: Vec<&ast::Bound> = bounds.iter().filter(|bound| !is_auto(bound)).collect();
                let [bound] = principal.as_slice() else {
                    bail!(ty.span, "a trait object must name exactly one trait");
                };
                let Some(Resolution { def: Def::Trait(trait_id), rest: [] }) =
                    self.defs.resolve_path(scope.module, &bound.path)
                else {
                    bail!(bound.path.span, "cannot find trait `{}`", bound.path.last().ident.name);
                };
                // `dyn Fn(A) -> R` carries its signature as the trait's argument.
                let args = match &bound.fn_sugar {
                    Some((params, ret)) => {
                        let params = params.iter().map(&mut lower).collect::<Result<_>>()?;
                        let ret = match ret {
                            Some(ret) => lower(ret)?,
                            None => Ty::UNIT,
                        };
                        vec![Ty::FnPtr(params, Box::new(ret))]
                    }
                    None => {
                        let last = bound.path.last();
                        let mut args: Vec<Ty> = last.args.iter().map(&mut lower).collect::<Result<_>>()?;
                        // `dyn Iterator<Item = u32>`: the associated types
                        // follow the trait's arguments, in declaration order.
                        for name in &self.defs.trait_def(trait_id).assoc_types {
                            let Some((_, value)) = last.bindings.iter().find(|(assoc, _)| assoc.name == *name) else {
                                bail!(bound.path.span, "the associated type `{name}` of this trait object must be given");
                            };
                            args.push(lower(value)?);
                        }
                        args
                    }
                };
                Ty::Dyn(trait_id, args)
            }
            TypeKind::ImplTrait(_) => bail!(ty.span, "`impl Trait` types are not supported yet"),
            TypeKind::Never => Ty::Never,
            TypeKind::Infer => infer.fresh_var(),
        })
    }

    /// Interpret a path as a type. With `elide_args`, omitted generic
    /// arguments (`Vec` for `Vec<_>`) become inference variables, which is
    /// allowed in expressions and patterns but not in signatures.
    pub fn lower_path_ty(
        &self,
        scope: TypeScope,
        path: &ast::Path,
        infer: &mut InferTable,
        pending: &mut Vec<PendingProjection>,
        elide_args: bool,
    ) -> Result<Ty> {
        self.defs.check_visible(scope.module, path)?;
        let (base, rest) = self.lower_path_ty_prefix(scope, path, infer, pending, elide_args)?;
        // `<T as Trait>::Item` names its trait; `Self::Error` means the
        // trait of the surrounding trait or impl.
        let mut trait_ref = match &path.qself {
            Some(qself) => match &qself.trait_ref {
                Some(trait_path) => self.lower_trait_ref(scope, trait_path, infer, pending)?,
                None => None,
            },
            None if path.segments[0].ident.name == "Self" => scope.self_trait.cloned(),
            None => None,
        };
        let mut ty = base;
        for segment in rest {
            ty = self.project(&ty, &segment.ident, trait_ref.take().as_ref(), infer, pending)?;
        }
        Ok(ty)
    }

    /// Lower the longest prefix of `path` that names a type; the remaining
    /// segments are associated items of that type.
    pub fn lower_path_ty_prefix<'p>(
        &self,
        scope: TypeScope,
        path: &'p ast::Path,
        infer: &mut InferTable,
        pending: &mut Vec<PendingProjection>,
        elide_args: bool,
    ) -> Result<(Ty, &'p [ast::PathSegment])> {
        // `<T as Trait>::Item`: every segment is an associated type of `T`.
        if let Some(qself) = &path.qself {
            return Ok((self.lower_ty(scope, &qself.ty, infer, pending)?, &path.segments));
        }
        let first = &path.segments[0];
        let name = first.ident.name.as_str();

        // Names that are not declarations: generic parameters, `Self`, primitives.
        let builtin = if let Some(ty) = scope.generics.get(name) {
            Some(ty.clone())
        } else if name == "Self" {
            match scope.self_ty {
                Some(ty) => Some(ty.clone()),
                None => bail!(first.ident.span, "`Self` is only available in impls and traits"),
            }
        } else {
            primitive_type(name)
        };
        if let Some(ty) = builtin {
            if !first.args.is_empty() {
                bail!(first.ident.span, "type arguments are not allowed on `{name}`");
            }
            return Ok((ty, &path.segments[1..]));
        }

        let Some(Resolution { def, rest }) = self.defs.resolve_path(scope.module, path) else {
            bail!(path.span, "cannot find type `{}` in this scope", path_text(path));
        };
        let type_segment = &path.segments[path.segments.len() - rest.len() - 1];
        let mut args = Vec::new();
        for arg in &type_segment.args {
            args.push(self.lower_ty(scope, arg, infer, pending)?);
        }

        let ty = match def {
            Def::Adt(adt) => {
                let expected = self.defs.adt(adt).generics.params.len();
                if args.is_empty() && expected > 0 && elide_args {
                    args = (0..expected).map(|_| infer.fresh_var()).collect();
                }
                if args.len() != expected {
                    bail!(
                        type_segment.ident.span,
                        "`{}` takes {expected} type argument(s) but {} were supplied",
                        self.defs.adt(adt).name,
                        args.len()
                    );
                }
                Ty::Adt(adt, args)
            }
            Def::Alias(alias) => {
                let alias = self.defs.alias(alias);
                let params = &alias.ast.generics.params;
                if args.len() != params.len() {
                    bail!(type_segment.ident.span, "wrong number of type arguments for this alias");
                }
                let env: GenericEnv = params.iter().map(|p| p.name.name.clone()).zip(args).collect();
                let alias_scope = TypeScope { module: alias.module, generics: &env, self_ty: None, self_trait: None };
                self.lower_ty(alias_scope, &alias.ast.ty, infer, pending)?
            }
            // `Buffer<SIZE>`: a constant item as a const generic argument.
            Def::Const(_) => {
                let value = ast::Expr { kind: ast::ExprKind::Path(path.clone()), span: path.span };
                Ty::Const(super::const_eval::eval_int_in(self, scope.module, scope.generics, &value)?)
            }
            Def::Trait(_) => bail!(path.span, "expected a type, found a trait (write `dyn Trait`)"),
            _ => bail!(path.span, "`{}` is not a type", path_text(path)),
        };
        Ok((ty, rest))
    }

    /// The associated type `base::name`.
    fn project(
        &self,
        base: &Ty,
        name: &ast::Ident,
        trait_ref: Option<&TraitRef>,
        infer: &mut InferTable,
        pending: &mut Vec<PendingProjection>,
    ) -> Result<Ty> {
        let resolved = infer.resolve(base);
        match self.lookup_assoc_type(&resolved, &name.name, trait_ref, infer, pending)? {
            AssocType::Known(ty) => Ok(ty),
            AssocType::Missing => Err(Diagnostic::new(
                name.span,
                format!("no associated type `{}` found for `{}`", name.name, self.display(&resolved)),
            )),
            // To be answered once more is known about the base type.
            AssocType::Undecided(_) => {
                let output = infer.fresh_var();
                pending.push(PendingProjection {
                    base: resolved,
                    name: name.name.clone(),
                    trait_ref: trait_ref.cloned(),
                    output: output.clone(),
                    span: name.span,
                });
                Ok(output)
            }
        }
    }

    /// A constant written in a type: an array's length or a const generic
    /// argument. A const parameter stands for its value, which is still a
    /// variable while an impl is being matched.
    pub fn lower_const(&self, scope: TypeScope, value: &ast::Expr) -> Result<Ty> {
        let mut bare = value;
        while let ast::ExprKind::Block(block) = &bare.kind {
            match (block.stmts.as_slice(), &block.expr) {
                ([], Some(inner)) => bare = inner,
                _ => break,
            }
        }
        if let ast::ExprKind::Path(path) = &bare.kind {
            if let Some(ty) = path.as_ident().and_then(|name| scope.generics.get(&name.name)) {
                return Ok(ty.clone());
            }
        }
        Ok(Ty::Const(super::const_eval::eval_int_in(self, scope.module, scope.generics, value)?))
    }

    // -- display -----------------------------------------------------------------------

    /// Render a type the way it would be written in source.
    pub fn display(&self, ty: &Ty) -> String {
        let list = |types: &[Ty]| types.iter().map(|t| self.display(t)).collect::<Vec<_>>().join(", ");
        let with_args = |name: &str, args: &[Ty]| match args {
            [] => name.to_string(),
            _ => format!("{name}<{}>", list(args)),
        };
        match ty {
            Ty::Int(int) => int.name().to_string(),
            Ty::Float(float) => float.name().to_string(),
            Ty::Bool => "bool".to_string(),
            Ty::Char => "char".to_string(),
            Ty::Str => "str".to_string(),
            Ty::Never => "!".to_string(),
            Ty::Tuple(elements) if elements.len() == 1 => format!("({},)", list(elements)),
            Ty::Tuple(elements) => format!("({})", list(elements)),
            Ty::Array(element, len) => format!("[{}; {}]", self.display(element), self.display(len)),
            Ty::Const(value) => value.to_string(),
            Ty::Slice(element) => format!("[{}]", self.display(element)),
            Ty::Ref(inner, Mutability::Not) => format!("&{}", self.display(inner)),
            Ty::Ref(inner, Mutability::Mut) => format!("&mut {}", self.display(inner)),
            Ty::Ptr(inner, Mutability::Not) => format!("*const {}", self.display(inner)),
            Ty::Ptr(inner, Mutability::Mut) => format!("*mut {}", self.display(inner)),
            Ty::Adt(adt, args) => with_args(&self.defs.adt(*adt).name, args),
            Ty::FnPtr(params, ret) if ret.is_unit() => format!("fn({})", list(params)),
            Ty::FnPtr(params, ret) => format!("fn({}) -> {}", list(params), self.display(ret)),
            Ty::FnItem(def, _) => format!("fn item `{}`", self.defs.fn_def(*def).path),
            Ty::Closure(_) => "closure".to_string(),
            Ty::Dyn(trait_id, args) => match args.as_slice() {
                [Ty::FnPtr(params, ret)] if self.lang.callable_traits.contains(trait_id) => {
                    let name = &self.defs.trait_def(*trait_id).name;
                    if ret.is_unit() {
                        format!("dyn {name}({})", list(params))
                    } else {
                        format!("dyn {name}({}) -> {}", list(params), self.display(ret))
                    }
                }
                _ => {
                    let def = self.defs.trait_def(*trait_id);
                    let arity = def.ast.generics.params.len().min(args.len());
                    let mut shown: Vec<String> = args[..arity].iter().map(|arg| self.display(arg)).collect();
                    for (name, value) in def.assoc_types.iter().zip(&args[arity..]) {
                        shown.push(format!("{name} = {}", self.display(value)));
                    }
                    match shown.is_empty() {
                        true => format!("dyn {}", def.name),
                        false => format!("dyn {}<{}>", def.name, shown.join(", ")),
                    }
                }
            },
            Ty::Infer(_) => "_".to_string(),
        }
    }

    /// A type's name as `std::any::type_name` gives it: items by their full
    /// path, standard ones under the crate of Rust's library that defines
    /// them (`alloc::vec::Vec<i32>`, `core::option::Option<&str>`).
    pub fn type_name(&self, ty: &Ty) -> String {
        let list = |types: &[Ty]| types.iter().map(|t| self.type_name(t)).collect::<Vec<_>>().join(", ");
        let with_args = |path: String, args: &[Ty]| match args {
            [] => path,
            _ => format!("{path}<{}>", list(args)),
        };
        match ty {
            Ty::Tuple(elements) if elements.len() == 1 => format!("({},)", list(elements)),
            Ty::Tuple(elements) => format!("({})", list(elements)),
            Ty::Array(element, len) => format!("[{}; {}]", self.type_name(element), self.display(len)),
            Ty::Slice(element) => format!("[{}]", self.type_name(element)),
            Ty::Ref(inner, Mutability::Not) => format!("&{}", self.type_name(inner)),
            Ty::Ref(inner, Mutability::Mut) => format!("&mut {}", self.type_name(inner)),
            Ty::Ptr(inner, Mutability::Not) => format!("*const {}", self.type_name(inner)),
            Ty::Ptr(inner, Mutability::Mut) => format!("*mut {}", self.type_name(inner)),
            Ty::Adt(adt, args) => {
                let def = self.defs.adt(*adt);
                with_args(self.item_path(def.module, &def.name), args)
            }
            Ty::FnPtr(params, ret) if ret.is_unit() => format!("fn({})", list(params)),
            Ty::FnPtr(params, ret) => format!("fn({}) -> {}", list(params), self.type_name(ret)),
            Ty::FnItem(def, _) => {
                let def = self.defs.fn_def(*def);
                self.item_path(def.module, &def.ast.name.name)
            }
            Ty::Closure(_) => "{{closure}}".to_string(),
            Ty::Dyn(trait_id, args) => {
                let def = self.defs.trait_def(*trait_id);
                format!("dyn {}", with_args(self.item_path(def.module, &def.name), args))
            }
            _ => self.display(ty),
        }
    }

    /// What `module_path!()` gives for code in `module`: `main::shapes`.
    pub fn module_path(&self, module: ModId) -> String {
        let mut segments = Vec::new();
        let mut current = module;
        while let Some(parent) = self.defs.module(current).parent {
            segments.push(self.defs.module(current).name.clone());
            current = parent;
        }
        segments.push(if current == super::defs::STD_ROOT { "std".to_string() } else { self.defs.crate_name.clone() });
        segments.reverse();
        segments.join("::")
    }

    /// The path of the item `name` in `module`, starting from its crate.
    fn item_path(&self, module: ModId, name: &str) -> String {
        let mut segments = vec![name.to_string()];
        let mut current = module;
        while let Some(parent) = self.defs.module(current).parent {
            segments.push(self.defs.module(current).name.clone());
            current = parent;
        }
        segments.reverse();
        if current != super::defs::STD_ROOT {
            return format!("{}::{}", self.defs.crate_name, segments.join("::"));
        }
        // Where Rust's own library defines what this one keeps in `std`.
        let path = segments.join("::");
        match segments.first().map(String::as_str) {
            Some("collections") => match segments.get(1).map(String::as_str) {
                Some("hash_map") => format!("std::collections::hash::map::{name}"),
                Some("hash_set") => format!("std::collections::hash::set::{name}"),
                Some("btree_map") => format!("alloc::collections::btree::map::{name}"),
                Some("btree_set") => format!("alloc::collections::btree::set::{name}"),
                _ => format!("alloc::{path}"),
            },
            Some("string" | "vec" | "boxed" | "rc" | "borrow") => format!("alloc::{path}"),
            Some("io" | "fs" | "env" | "process" | "thread") => format!("std::{path}"),
            _ => format!("core::{path}"),
        }
    }

    /// The entries of a trait's vtables, in order: its own methods as
    /// declared, then those of its supertraits (`trait Shape: Debug` puts
    /// `Debug::fmt` in every `dyn Shape` vtable). Each entry names the trait
    /// that declares the method.
    pub fn vtable_entries(&self, trait_id: TraitId) -> Vec<(TraitId, FnId)> {
        let def = self.defs.trait_def(trait_id);
        let mut entries: Vec<(TraitId, FnId)> = def
            .ast
            .items
            .iter()
            .filter_map(|item| match item {
                ast::AssocItem::Fn(function) => def.methods.get(&function.name.name).map(|&m| (trait_id, m)),
                _ => None,
            })
            .collect();
        for bound in &def.ast.supertraits {
            if let Some(Resolution { def: Def::Trait(supertrait), rest: [] }) =
                self.defs.resolve_path(def.module, &bound.path)
            {
                entries.extend(self.vtable_entries(supertrait));
            }
        }
        entries
    }

    /// Position of `method` in the vtables of `dyn trait_id`, if it is there.
    pub fn vtable_slot(&self, trait_id: TraitId, method: FnId) -> Option<usize> {
        self.vtable_entries(trait_id).iter().position(|&(_, entry)| entry == method)
    }

    /// The function that implements `method` of a trait for a concrete type,
    /// or `None` if the method cannot be called through a trait object
    /// (it is generic, or takes `self` by value).
    pub fn resolve_trait_method(&self, trait_id: TraitId, method: FnId, self_ty: &Ty) -> Result<Option<Instance>> {
        let declaration = self.defs.fn_def(method).ast;
        let by_value = matches!(&declaration.self_param, Some(receiver) if receiver.kind == SelfKind::Value);
        if declaration.self_param.is_none() || by_value || !declaration.generics.params.is_empty() {
            return Ok(None);
        }

        let mut infer = InferTable::default();
        let Some((impl_id, impl_substs)) = self.trait_impls_for(trait_id, self_ty, &mut infer)?.pop() else {
            return Err(Diagnostic::global(format!(
                "`{}` does not implement `{}`",
                self.display(self_ty),
                self.defs.trait_def(trait_id).name
            )));
        };
        let substs: Vec<Ty> = impl_substs.iter().map(|ty| infer.resolve(ty)).collect();
        if let Some(&def) = self.defs.impl_def(impl_id).methods.get(&declaration.name.name) {
            return Ok(Some(Instance { def, substs }));
        }

        // The trait's default body, with `Self` and the trait's arguments filled in.
        let block = self.defs.impl_def(impl_id);
        let names = self.defs.impl_generics(impl_id);
        let env: GenericEnv = names.iter().map(|n| n.to_string()).zip(substs).collect();
        let scope = TypeScope { module: block.module, generics: &env, self_ty: Some(self_ty), self_trait: None };
        let written = &block.ast.trait_ref.as_ref().expect("a trait impl names its trait").last().args;
        let arity = self.defs.trait_def(trait_id).ast.generics.params.len();
        let mut default_substs = vec![self_ty.clone()];
        if written.is_empty() {
            default_substs.extend(std::iter::repeat(self_ty.clone()).take(arity));
        }
        for ty in written {
            default_substs.push(self.lower_ty(scope, ty, &mut infer, &mut Vec::new())?);
        }
        Ok(Some(Instance { def: method, substs: default_substs }))
    }

    pub fn is_enum(&self, adt: AdtId) -> bool {
        self.defs.adt(adt).kind == AdtKind::Enum
    }
}

pub fn primitive_type(name: &str) -> Option<Ty> {
    if let Some(int) = IntTy::from_name(name) {
        return Some(Ty::Int(int));
    }
    Some(match name {
        "f32" => Ty::Float(FloatTy::F32),
        "f64" => Ty::Float(FloatTy::F64),
        "bool" => Ty::Bool,
        "char" => Ty::Char,
        "str" => Ty::Str,
        _ => return None,
    })
}

pub fn path_text(path: &ast::Path) -> String {
    path.segments.iter().map(|s| s.ident.name.as_str()).collect::<Vec<_>>().join("::")
}
