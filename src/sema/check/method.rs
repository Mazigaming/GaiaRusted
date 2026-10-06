//! Method calls: `receiver.name(args)`.
//!
//! The method is searched for on the receiver's type, then on what it
//! dereferences to, and so on (`Box<Vec<T>>` finds `len` on `[T]` three
//! steps down). Once found, the receiver is borrowed the way the method's
//! `self` parameter asks for.

use super::expr::MethodArgs;
use super::FnCtxt;
use crate::sema::context::TraitRef;
use crate::sema::defs::{FnKind, ImplId};
use crate::sema::thir::*;
use crate::sema::ty::{FnId, Mutability, TraitId, Ty};
use crate::syntax::ast::{self, SelfKind};
use crate::syntax::diagnostic::{bail, Result};
use crate::syntax::span::Span;

/// Somewhere a method named as requested was found for the receiver type.
#[derive(Clone, Copy, Debug)]
enum Candidate {
    /// Defined in this impl block.
    Impl(ImplId),
    /// The default body of a trait method, as provided through an impl of
    /// the trait that does not override it.
    TraitDefault(TraitId, FnId, ImplId),
    /// A method of the trait behind a `dyn Trait` receiver.
    Virtual(TraitId, FnId),
}

impl FnCtxt<'_, '_> {
    pub fn check_method_call(
        &mut self,
        receiver: Expr,
        method: &ast::Ident,
        turbofish: &[ast::Type],
        args: MethodArgs,
        expected: Option<&Ty>,
        span: Span,
    ) -> Result<Expr> {
        let name = method.name.as_str();
        let receiver_ty = receiver.ty.clone();

        // What the receiver is may hang on obligations still open: the
        // element type of `iter.collect::<Vec<_>>()` comes from its
        // `FromIterator` bound, and decides between `[String]::join` and
        // `[&str]::join`.
        if self.resolve(&receiver_ty).has_infer() {
            self.solve_pending()?;
        }

        if std::mem::take(&mut self.source_method_call) {
            self.method_scope = self.tcx.traits_in_scope(self.module);
        }
        let found = self.find_method(receiver, name, span);
        self.method_scope = None;
        let Some((mut place, candidates)) = found? else {
            bail!(method.span, "no method named `{name}` found for type `{}`", self.show(&receiver_ty));
        };
        let place_ty = self.resolve(&place.ty);

        // More than one candidate (impls of a generic trait that differ only
        // in the trait's arguments): the argument types decide, and the
        // type the result is expected to have.
        let candidates = self.assumed_candidates(candidates, &place_ty)?;
        let mut args = args;
        let candidate = if let [only] = candidates.as_slice() {
            *only
        } else {
            let checked = match args {
                MethodArgs::Checked(checked) => checked,
                MethodArgs::Unchecked(exprs) => {
                    exprs.iter().map(|arg| self.check_expr(arg, None)).collect::<Result<_>>()?
                }
            };
            let chosen = self.disambiguate(&candidates, &place_ty, name, &checked, expected, span)?;
            args = MethodArgs::Checked(checked);
            chosen
        };

        if let Candidate::Virtual(trait_id, def) = candidate {
            return self.virtual_call(place, trait_id, def, args, span);
        }
        let (def, owner_substs) = self.instantiate_candidate(candidate, &place_ty, name)?;
        self.check_fn_visible(def, method.span)?;
        self.apply_assumptions(candidate, &place_ty, &owner_substs)?;
        let own = self.own_generic_args(def, turbofish, span)?;
        let instance = Instance { def, substs: [owner_substs, own].concat() };

        let receiver = match self.self_kind(def).expect("candidates are methods") {
            SelfKind::Value => place,
            SelfKind::Ref => self.borrow_place(place, Mutability::Not),
            SelfKind::RefMut => {
                if place.is_place() {
                    self.require_mutable_place(&mut place)?;
                }
                self.borrow_place(place, Mutability::Mut)
            }
        };
        self.call_instance(instance, Some(receiver), args, None, span)
    }

    /// Call a method with operands that are already checked: overloaded
    /// operators and compiler-generated calls such as `next()` in a `for` loop.
    pub fn call_method_by_name(&mut self, receiver: Expr, name: &str, args: Vec<Expr>, span: Span) -> Result<Expr> {
        let ident = ast::Ident { name: name.to_string(), span };
        self.check_method_call(receiver, &ident, &[], MethodArgs::Checked(args), None, span)
    }

    /// `&place`, without stacking a borrow on top of a dereference that
    /// already had the right kind of reference underneath.
    pub fn borrow_place(&mut self, place: Expr, mutability: Mutability) -> Expr {
        if let ExprKind::Deref(pointer) = &place.kind {
            if matches!(self.infer.shallow(&pointer.ty), Ty::Ref(_, m) if m == mutability) {
                let ExprKind::Deref(pointer) = place.kind else { unreachable!() };
                return *pointer;
            }
        }
        let (ty, span) = (Ty::Ref(Box::new(place.ty.clone()), mutability), place.span);
        Expr { kind: ExprKind::AddrOf(mutability, Box::new(place)), ty, span }
    }

    /// Find what `receiver.name(..)` refers to. Returns the receiver adjusted
    /// to the `Self` type of the method found, and the candidates there.
    ///
    /// The search follows Rust's order. For each type along the receiver's
    /// dereference chain, a method is accepted if its `self` parameter can be
    /// the receiver
    ///
    /// 1. as it is: `self` methods of the type, or, when the type is a
    ///    reference, `&self` methods of what it points to;
    /// 2. borrowed: `&self` methods of the type, then `&mut self` methods.
    ///
    /// So with `x: &T`, `x.clone()` clones the `T` if `T: Clone`, and only
    /// otherwise copies the reference.
    fn find_method(&mut self, receiver: Expr, name: &str, span: Span) -> Result<Option<(Expr, Vec<Candidate>)>> {
        use SelfKind::{Ref, RefMut, Value};
        let mut place = receiver;
        loop {
            let ty = self.structurally_resolve(&place.ty, span)?;
            let here = self.method_candidates(&ty, name)?;
            if matches!(here.first(), Some(Candidate::Virtual(..))) {
                return Ok(Some((place, here)));
            }

            // 1. The receiver as it is.
            let by_value = self.with_self_kind(&here, name, &[Value]);
            if !by_value.is_empty() {
                return Ok(Some((place, by_value)));
            }
            if let Ty::Ref(pointee, mutability) = &ty {
                let kinds: &[SelfKind] = if mutability.is_mut() { &[Ref, RefMut] } else { &[Ref] };
                let of_pointee = self.method_candidates(pointee, name)?;
                let of_pointee = self.with_self_kind(&of_pointee, name, kinds);
                if !of_pointee.is_empty() {
                    let pointee = Expr { kind: ExprKind::Deref(Box::new(place)), ty: (**pointee).clone(), span };
                    return Ok(Some((pointee, of_pointee)));
                }
            }

            // 2. The receiver borrowed: `&self` methods of the type itself,
            // or `self` methods of the reference type (`impl Trait for &T`).
            for (kind, mutability) in [(Ref, Mutability::Not), (RefMut, Mutability::Mut)] {
                let borrowed = self.with_self_kind(&here, name, &[kind]);
                if !borrowed.is_empty() {
                    return Ok(Some((place, borrowed)));
                }
                let reference = Ty::Ref(Box::new(ty.clone()), mutability);
                let of_reference = self.method_candidates(&reference, name)?;
                let of_reference = self.with_self_kind(&of_reference, name, &[Value]);
                if !of_reference.is_empty() {
                    if mutability.is_mut() && place.is_place() {
                        self.require_mutable_place(&mut place)?;
                    }
                    return Ok(Some((self.borrow_place(place, mutability), of_reference)));
                }
            }

            place = match ty {
                // Arrays have the methods of slices.
                Ty::Array(element, _) => self.array_as_slice(place, *element, span),
                _ => match self.deref_once(place, span)? {
                    Some(inner) => inner,
                    None => return Ok(None),
                },
            };
        }
    }

    /// The candidates whose method declares one of the given kinds of `self`.
    fn with_self_kind(&self, candidates: &[Candidate], name: &str, kinds: &[SelfKind]) -> Vec<Candidate> {
        let kind_of = |candidate: &Candidate| {
            let def = match candidate {
                Candidate::Impl(impl_id) => self.tcx.defs.impl_def(*impl_id).methods[name],
                Candidate::TraitDefault(_, def, _) | Candidate::Virtual(_, def) => *def,
            };
            self.self_kind(def)
        };
        candidates.iter().copied().filter(|c| kind_of(c).is_some_and(|kind| kinds.contains(&kind))).collect()
    }

    /// View an array place as the slice of all its elements.
    fn array_as_slice(&mut self, array: Expr, element: Ty, span: Span) -> Expr {
        let slice_ty = Ty::Slice(Box::new(element));
        let array_ref_ty = Ty::shared_ref(array.ty.clone());
        let array_ref = Expr { kind: ExprKind::AddrOf(Mutability::Not, Box::new(array)), ty: array_ref_ty, span };
        let slice_ref_ty = Ty::shared_ref(slice_ty.clone());
        let slice_ref = Expr { kind: ExprKind::Unsize(Box::new(array_ref)), ty: slice_ref_ty, span };
        Expr { kind: ExprKind::Deref(Box::new(slice_ref)), ty: slice_ty, span }
    }

    /// The candidates for `ty.name(...)` that the call can see: trait
    /// methods only of traits in scope, for a call written in the source.
    fn method_candidates(&mut self, ty: &Ty, name: &str) -> Result<Vec<Candidate>> {
        let mut candidates = self.all_method_candidates(ty, name)?;
        if let Some(visible) = &self.method_scope {
            candidates.retain(|candidate| match candidate {
                Candidate::Impl(impl_id) => {
                    self.tcx.defs.impl_def(*impl_id).trait_id.is_none_or(|trait_id| visible.contains(&trait_id))
                }
                Candidate::TraitDefault(trait_id, ..) => visible.contains(trait_id),
                // A trait object's own methods need no import.
                Candidate::Virtual(..) => true,
            });
        }
        Ok(candidates)
    }

    /// Everything that could be meant by `ty.name(...)`, without committing
    /// inference to any of them. Inherent methods shadow trait methods.
    fn all_method_candidates(&mut self, ty: &Ty, name: &str) -> Result<Vec<Candidate>> {
        if let Ty::Dyn(trait_id, _) = ty {
            let entries = self.tcx.vtable_entries(*trait_id);
            let named = entries.iter().find(|(_, method)| self.tcx.defs.fn_def(*method).ast.name.name == name);
            if let Some(&(_, def)) = named {
                return Ok(vec![Candidate::Virtual(*trait_id, def)]);
            }
        }

        let mut inherent = Vec::new();
        let mut from_traits = Vec::new();
        let impls = self.tcx.defs.impls_by_member.get(name).cloned().unwrap_or_default();
        for impl_id in impls {
            let block = self.tcx.defs.impl_def(impl_id);
            let is_method = block.methods.get(name).is_some_and(|&def| self.self_kind(def).is_some());
            if !is_method {
                continue;
            }
            let snapshot = self.infer.snapshot();
            let applies = self.tcx.match_impl(impl_id, ty, &mut self.infer)?.is_some();
            self.infer.rollback_to(snapshot);
            if applies {
                let list = if block.trait_id.is_none() { &mut inherent } else { &mut from_traits };
                list.push(Candidate::Impl(impl_id));
            }
        }
        if !inherent.is_empty() {
            return Ok(inherent);
        }

        for &trait_id in self.tcx.traits_with_method(name) {
            let def = self.tcx.defs.trait_def(trait_id).methods[name];
            let has_default = self.tcx.defs.fn_def(def).kind == FnKind::Defined;
            if !has_default || self.self_kind(def).is_none() {
                continue;
            }
            let overridden = from_traits.iter().any(|candidate| {
                matches!(candidate, Candidate::Impl(id) if self.tcx.defs.impl_def(*id).trait_id == Some(trait_id))
            });
            // Impls that write no trait arguments all give the default body
            // the same ones, `Self`'s: they make one candidate. Each impl that
            // writes its own makes another, as `String: PartialEq<str>` and
            // `String: PartialEq<&str>` give `ne` different parameter types.
            let mut implied_args_seen = overridden;
            for impl_id in self.tcx.probe_trait_impls(trait_id, ty, &mut self.infer)? {
                if self.tcx.defs.impl_def(impl_id).methods.contains_key(name) {
                    continue;
                }
                if !self.tcx.writes_trait_args(impl_id) {
                    if implied_args_seen {
                        continue;
                    }
                    implied_args_seen = true;
                }
                from_traits.push(Candidate::TraitDefault(trait_id, def, impl_id));
            }
        }
        Ok(from_traits)
    }

    /// Commit to a candidate: the function to call and the type arguments
    /// of its impl or trait.
    fn instantiate_candidate(&mut self, candidate: Candidate, ty: &Ty, name: &str) -> Result<(FnId, Vec<Ty>)> {
        match candidate {
            Candidate::Impl(impl_id) => {
                let substs = self.tcx.match_impl(impl_id, ty, &mut self.infer)?.expect("candidate matched before");
                Ok((self.tcx.defs.impl_def(impl_id).methods[name], substs))
            }
            Candidate::TraitDefault(trait_id, def, impl_id) => {
                // Without written trait arguments the impl stands for every
                // impl that applies (`{integer}: PartialEq` holds through each
                // integer type's), so it must not be committed to.
                let impl_substs = if self.tcx.writes_trait_args(impl_id) {
                    self.tcx.match_impl(impl_id, ty, &mut self.infer)?.expect("candidate matched before")
                } else {
                    Vec::new()
                };
                Ok((def, self.trait_default_substs(trait_id, impl_id, &impl_substs, ty)?))
            }
            Candidate::Virtual(..) => unreachable!("virtual calls are built separately"),
        }
    }

    /// Of several candidates from impls of one trait, those the function's
    /// bounds say are meant: with `S: AsRef<OsStr>` and `S = str`,
    /// `s.as_ref()` is `AsRef<OsStr>`'s, not `AsRef<[u8]>`'s.
    fn assumed_candidates(&mut self, candidates: Vec<Candidate>, ty: &Ty) -> Result<Vec<Candidate>> {
        if candidates.len() < 2 {
            return Ok(candidates);
        }
        // A bound on `&T` speaks for `T` as well: the method found on `T`
        // is what `&T`'s forwarding impl calls.
        let subject = self.resolve(ty);
        let assumed: Vec<TraitRef> = self
            .assumptions
            .iter()
            .filter(|(assumed, _)| match self.infer.resolve(assumed) {
                Ty::Ref(pointee, _) if *pointee == subject => true,
                assumed => assumed == subject,
            })
            .map(|(_, trait_ref)| trait_ref.clone())
            .collect();
        let mut kept = Vec::new();
        for &candidate in &candidates {
            let (trait_id, impl_id) = match candidate {
                Candidate::Impl(impl_id) => match self.tcx.defs.impl_def(impl_id).trait_id {
                    Some(trait_id) => (trait_id, impl_id),
                    None => {
                        kept.push(candidate);
                        continue;
                    }
                },
                Candidate::TraitDefault(trait_id, _, impl_id) => (trait_id, impl_id),
                Candidate::Virtual(..) => {
                    kept.push(candidate);
                    continue;
                }
            };
            let Some(trait_ref) = assumed.iter().find(|trait_ref| trait_ref.trait_id == trait_id) else {
                kept.push(candidate);
                continue;
            };
            let snapshot = self.infer.snapshot();
            let fits = self.tcx.match_impl_where(impl_id, ty, Some(&trait_ref.args), &mut self.infer)?.is_some();
            self.infer.rollback_to(snapshot);
            if fits {
                kept.push(candidate);
            }
        }
        Ok(if kept.is_empty() { candidates } else { kept })
    }

    /// A method of a trait the function assumes `ty` implements with given
    /// arguments (`S: Into<String>`) takes those arguments.
    fn apply_assumptions(&mut self, candidate: Candidate, ty: &Ty, owner_substs: &[Ty]) -> Result<()> {
        let (trait_id, trait_args) = match candidate {
            Candidate::Impl(impl_id) => {
                let Some(trait_id) = self.tcx.defs.impl_def(impl_id).trait_id else { return Ok(()) };
                (trait_id, self.impl_trait_args(trait_id, impl_id, owner_substs, ty)?)
            }
            Candidate::TraitDefault(trait_id, ..) => (trait_id, owner_substs[1..].to_vec()),
            Candidate::Virtual(..) => return Ok(()),
        };
        let subject = self.resolve(ty);
        let assumed = self.assumptions.iter().find(|(assumed, trait_ref)| {
            trait_ref.trait_id == trait_id && self.infer.resolve(assumed) == subject
        });
        if let Some((_, trait_ref)) = assumed.cloned() {
            for (actual, assumed) in trait_args.iter().zip(&trait_ref.args) {
                self.infer.try_unify(actual, assumed);
            }
        }
        Ok(())
    }

    /// Pick the candidate whose parameter types accept the given arguments.
    fn disambiguate(
        &mut self,
        candidates: &[Candidate],
        ty: &Ty,
        name: &str,
        args: &[Expr],
        expected: Option<&Ty>,
        span: Span,
    ) -> Result<Candidate> {
        let mut fitting = Vec::new();
        for &candidate in candidates {
            let snapshot = self.infer.snapshot();
            let (def, owner_substs) = self.instantiate_candidate(candidate, ty, name)?;
            let own = self.own_generic_args(def, &[], span)?;
            let sig = self.tcx.fn_sig(def, &[owner_substs, own].concat(), &mut self.infer, &mut Vec::new())?;
            let fits = sig.params.len() == args.len() + 1
                && sig.params[1..].iter().zip(args).all(|(param, arg)| self.infer.unify(param, &arg.ty).is_ok());
            let returns_expected = expected.is_none_or(|expected| self.infer.unify(&sig.ret, expected).is_ok());
            self.infer.rollback_to(snapshot);
            if fits {
                fitting.push((candidate, returns_expected));
            }
        }
        // Prefer the candidate whose result is what the context expects.
        let preferred = fitting.iter().find(|(_, returns_expected)| *returns_expected).or(fitting.first());
        match preferred {
            Some((candidate, _)) => Ok(*candidate),
            None => bail!(span, "no implementation of `{name}` for `{}` accepts these argument types", self.show(ty)),
        }
    }

    /// A call through a trait object's vtable.
    fn virtual_call(&mut self, place: Expr, trait_id: TraitId, def: FnId, args: MethodArgs, span: Span) -> Result<Expr> {
        let Ty::Dyn(_, trait_args) = self.resolve(&place.ty) else { unreachable!("virtual candidate on non-dyn") };
        let kind = self.self_kind(def).expect("candidates are methods");
        let mutability = match kind {
            SelfKind::Ref => Mutability::Not,
            SelfKind::RefMut => Mutability::Mut,
            SelfKind::Value => bail!(span, "a method taking `self` by value cannot be called on a trait object"),
        };
        if !self.tcx.defs.fn_def(def).ast.generics.params.is_empty() {
            bail!(span, "a generic method cannot be called on a trait object");
        }

        // The signature with `Self` standing for the trait object itself.
        // (A method inherited from a supertrait takes no trait arguments.)
        let declared_by_object_trait = self.tcx.defs.trait_def(trait_id).methods.values().any(|&m| m == def);
        let mut substs = vec![place.ty.clone()];
        if declared_by_object_trait {
            substs.extend(trait_args);
        }
        let sig = self.tcx.fn_sig(def, &substs, &mut self.infer, &mut self.projections)?;
        let receiver = self.borrow_place(place, mutability);
        let mut call_args = vec![receiver];
        call_args.extend(self.check_args(&sig.params[1..], false, args, span)?);

        let slot = self.tcx.vtable_slot(trait_id, def).expect("the candidate came from the vtable");
        Ok(Expr { kind: ExprKind::Call(Callee::Virtual { slot }, call_args), ty: sig.ret, span })
    }
}
