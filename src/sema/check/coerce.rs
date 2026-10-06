//! Implicit conversions.
//!
//! At a few well-defined points — arguments, `let` with a type, `return`,
//! field initialisers — a value may be converted without the programmer
//! asking: `&mut T` is accepted as `&T`, `&String` as `&str`, `&[T; N]` as
//! `&[T]`, a function as a function pointer. Everywhere else types must
//! match exactly.

use super::FnCtxt;
use crate::sema::infer::VarKind;
use crate::sema::thir::*;
use crate::sema::ty::{Mutability, Ty};
use crate::syntax::diagnostic::{bail, Result};

impl FnCtxt<'_, '_> {
    /// Convert `expr` to `target`, inserting a coercion if one applies.
    pub fn coerce(&mut self, expr: Expr, target: &Ty) -> Result<Expr> {
        let from = self.infer.shallow(&expr.ty);
        let to = self.infer.shallow(target);
        let span = expr.span;

        // A tuple expression hands the coercion on to its elements, and a
        // block to its value: `(&mut a, &mut b)` can serve as a
        // `(&i32, &mut i32)`, and so can `{ ..; (&mut a, &mut b) }`.
        match (&expr.kind, &to) {
            (ExprKind::Tuple(elements), Ty::Tuple(targets)) if elements.len() == targets.len() => {
                let ExprKind::Tuple(elements) = expr.kind else { unreachable!() };
                let mut coerced = Vec::with_capacity(elements.len());
                for (element, target) in elements.into_iter().zip(targets) {
                    coerced.push(self.coerce(element, target)?);
                }
                return Ok(Expr { kind: ExprKind::Tuple(coerced), ty: to, span });
            }
            (ExprKind::Block(Block { expr: Some(_), .. }), _) if from != to => {
                let ExprKind::Block(Block { stmts, expr: Some(value) }) = expr.kind else { unreachable!() };
                let value = self.coerce(*value, &to)?;
                return Ok(Expr { kind: ExprKind::Block(Block { stmts, expr: Some(Box::new(value)) }), ty: to, span });
            }
            _ => {}
        }

        match (&from, &to) {
            // An expression that never finishes can stand in for any type.
            (Ty::Never, Ty::Infer(_)) => {
                self.diverging_vars.push(to.clone());
                return Ok(expr);
            }
            (Ty::Never, _) => return Ok(expr),

            (Ty::Ref(from_pointee, from_mut), Ty::Ref(to_pointee, to_mut))
                if *from_mut == *to_mut || from_mut.is_mut() =>
            {
                if self.infer.try_unify(from_pointee, to_pointee) {
                    if from_mut == to_mut {
                        return Ok(expr);
                    }
                    // `&mut T` used as `&T`: reborrow.
                    let place = Expr { kind: ExprKind::Deref(Box::new(expr)), ty: (**to_pointee).clone(), span };
                    return Ok(Expr { kind: ExprKind::AddrOf(Mutability::Not, Box::new(place)), ty: to, span });
                }
                if let Some(coerced) = self.coerce_unsized(&expr, from_pointee, to_pointee, &to)? {
                    return Ok(coerced);
                }
                if let Some(coerced) = self.coerce_through_deref(&expr, to_pointee, *to_mut, &to)? {
                    return Ok(coerced);
                }
            }

            // References decay to raw pointers; `*mut` is accepted as `*const`.
            (Ty::Ref(from_pointee, from_mut), Ty::Ptr(to_pointee, to_mut))
            | (Ty::Ptr(from_pointee, from_mut), Ty::Ptr(to_pointee, to_mut))
                if (from_mut.is_mut() || !to_mut.is_mut())
                    && (matches!(from, Ty::Ref(..)) || from_mut != to_mut)
                    && self.infer.try_unify(from_pointee, to_pointee) =>
            {
                return Ok(Expr { kind: ExprKind::Cast(Box::new(expr)), ty: to, span });
            }

            // `Box<T>` to `Box<dyn Trait>`, `Rc<[T; N]>` to `Rc<[T]>`: the
            // pointer inside gains a vtable or a length.
            (Ty::Adt(from_adt, from_args), Ty::Adt(to_adt, to_args))
                if from_adt == to_adt && !from_args.is_empty() && self.coerces_unsized(&from)? =>
            {
                let wanted = self.infer.shallow(&to_args[0]);
                // `Box::new(1)` as a trait object is a box of the literal's default type.
                let concrete = match self.infer.shallow(&from_args[0]) {
                    literal @ Ty::Infer(var)
                        if matches!(wanted, Ty::Dyn(..)) && self.infer.unbound_kind(var) != Some(VarKind::General) =>
                    {
                        self.structurally_resolve(&literal, span)?
                    }
                    other => other,
                };
                if let (false, Ty::Dyn(..)) = (matches!(concrete, Ty::Dyn(..) | Ty::Infer(_)), &wanted) {
                    if self.implements_dyn(&concrete, &wanted)? {
                        return Ok(Expr { kind: ExprKind::Unsize(Box::new(expr)), ty: to, span });
                    }
                }
                if let (Ty::Array(element, _), Ty::Slice(wanted_element)) = (&concrete, &wanted) {
                    if self.infer.try_unify(element, wanted_element) {
                        return Ok(Expr { kind: ExprKind::Unsize(Box::new(expr)), ty: to, span });
                    }
                }
            }

            (Ty::FnItem(..) | Ty::Closure(_), Ty::FnPtr(to_params, to_ret)) => {
                if let Some((params, ret)) = self.callable_sig(&from)? {
                    let same_arity = params.len() == to_params.len();
                    let matches = same_arity
                        && params.iter().zip(to_params).all(|(a, b)| self.infer.try_unify(a, b))
                        && self.infer.try_unify(&ret, to_ret);
                    if matches {
                        return Ok(Expr { kind: ExprKind::ReifyFnPointer(Box::new(expr)), ty: to, span });
                    }
                }
            }
            _ => {}
        }

        if self.infer.try_unify(&to, &from) {
            return Ok(expr);
        }
        bail!(span, "mismatched types: expected `{}`, found `{}`", self.show(&to), self.show(&from))
    }

    /// Is `pointer` a pointer type through which a value can be made
    /// unsized, as `Box`, `Rc` and `Arc` are?
    fn coerces_unsized(&mut self, pointer: &Ty) -> Result<bool> {
        match self.tcx.lang.coerce_unsized {
            Some(coerce_unsized) => self.tcx.implements(pointer, coerce_unsized, &mut self.infer),
            None => Ok(false),
        }
    }

    /// Can a value of type `concrete` serve as the trait object `wanted`?
    /// For a callable trait object the signatures must agree, which also
    /// settles the types of a closure that are still open.
    fn implements_dyn(&mut self, concrete: &Ty, wanted: &Ty) -> Result<bool> {
        let Ty::Dyn(trait_id, _) = wanted else { return Ok(false) };
        if !self.tcx.implements(concrete, *trait_id, &mut self.infer)? {
            return Ok(false);
        }
        let (Some((params, ret)), Some((wanted_params, wanted_ret))) =
            (self.callable_sig(concrete)?, self.callable_sig(wanted)?)
        else {
            // `&Length` as `&dyn Converter<T>`: the impl determines `T`.
            let Ty::Dyn(_, wanted_args) = wanted else { unreachable!() };
            let arity = self.tcx.defs.trait_def(*trait_id).ast.generics.params.len().min(wanted_args.len());
            let impls = self.tcx.trait_impls_for(*trait_id, concrete, &mut self.infer)?;
            if let [(impl_id, substs)] = impls.as_slice() {
                let actual = self.impl_trait_args(*trait_id, *impl_id, substs, concrete)?;
                if !actual.iter().zip(&wanted_args[..arity]).all(|(a, w)| self.infer.try_unify(a, w)) {
                    return Ok(false);
                }
            }
            // `Box<Range<u32>>` as `Box<dyn Iterator<Item = u32>>`: the
            // associated types must be the ones the object promises.
            let assoc_names = self.tcx.defs.trait_def(*trait_id).assoc_types.clone();
            for (name, wanted) in assoc_names.iter().zip(&wanted_args[arity..]) {
                if let Some(actual) = self.tcx.assoc_type(concrete, name, &mut self.infer)? {
                    if !self.infer.try_unify(&actual, wanted) {
                        return Ok(false);
                    }
                }
            }
            return Ok(true);
        };
        Ok(params.len() == wanted_params.len()
            && params.iter().zip(&wanted_params).all(|(a, b)| self.infer.try_unify(a, b))
            && self.infer.try_unify(&ret, &wanted_ret))
    }

    /// `&[T; N]` to `&[T]`, and `&T` to `&dyn Trait` when `T: Trait`.
    fn coerce_unsized(&mut self, expr: &Expr, from: &Ty, to: &Ty, target: &Ty) -> Result<Option<Expr>> {
        // `&5` as a `&dyn Display` points to a number of the literal's default type.
        let from = match (self.infer.shallow(from), self.infer.shallow(to)) {
            (literal @ Ty::Infer(var), Ty::Dyn(..)) if self.infer.unbound_kind(var) != Some(VarKind::General) => {
                self.structurally_resolve(&literal, expr.span)?
            }
            (from, _) => from,
        };
        let applies = match (from, self.infer.shallow(to)) {
            (Ty::Array(element, _), Ty::Slice(wanted)) => self.infer.try_unify(&element, &wanted),
            (Ty::Infer(_), _) => false,
            (concrete, wanted @ Ty::Dyn(..)) if !self.tcx.is_unsized(&concrete) => {
                self.implements_dyn(&concrete, &wanted)?
            }
            _ => false,
        };
        Ok(applies.then(|| Expr {
            kind: ExprKind::Unsize(Box::new(expr.clone())),
            ty: target.clone(),
            span: expr.span,
        }))
    }

    /// `&String` to `&str`, `&Vec<T>` to `&[T]`, `&Box<T>` to `&T`, `&&T` to
    /// `&T`: dereference until the wanted pointee appears, then borrow again.
    fn coerce_through_deref(
        &mut self,
        expr: &Expr,
        wanted: &Ty,
        mutability: Mutability,
        target: &Ty,
    ) -> Result<Option<Expr>> {
        let span = expr.span;
        let Some(pointee) = self.infer.shallow(&expr.ty).pointee().cloned() else { return Ok(None) };
        let mut place = Expr { kind: ExprKind::Deref(Box::new(expr.clone())), ty: pointee, span };
        loop {
            if matches!(self.infer.shallow(&place.ty), Ty::Infer(_)) {
                return Ok(None);
            }
            place = match self.deref_once(place, span)? {
                Some(inner) => inner,
                None => return Ok(None),
            };
            if self.infer.try_unify(&place.ty, wanted) {
                if mutability.is_mut() {
                    self.require_mutable_place(&mut place)?;
                }
                let kind = ExprKind::AddrOf(mutability, Box::new(place));
                return Ok(Some(Expr { kind, ty: target.clone(), span }));
            }
        }
    }

    /// `expr` converted to `target` if a coercion makes it fit, else as it
    /// is: for an expected type, which guides an expression without binding
    /// it, so a mismatch is left for whoever needs the type to report.
    pub fn coerce_if_possible(&mut self, expr: Expr, target: &Ty) -> Expr {
        if self.infer.try_unify(&expr.ty, target) {
            return expr;
        }
        let snapshot = self.infer.snapshot();
        match self.coerce(expr.clone(), target) {
            Ok(coerced) => coerced,
            Err(_) => {
                self.infer.rollback_to(snapshot);
                expr
            }
        }
    }

    /// The common type of the branches of an `if` or `match`: the first
    /// branch's, after coercing it to the expected type if there is one. A
    /// branch that diverges does not constrain it.
    pub fn unify_branch(&mut self, result_ty: &mut Option<Ty>, branch: Expr, expected: Option<&Ty>) -> Result<Expr> {
        if self.infer.shallow(&branch.ty) == Ty::Never {
            return Ok(branch);
        }
        match result_ty {
            Some(ty) => self.coerce(branch, &ty.clone()),
            None => {
                let branch = match expected {
                    Some(expected) => self.coerce_if_possible(branch, expected),
                    None => branch,
                };
                *result_ty = Some(branch.ty.clone());
                Ok(branch)
            }
        }
    }
}
