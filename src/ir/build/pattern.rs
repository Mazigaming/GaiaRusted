//! Pattern matching.
//!
//! A pattern is lowered to a sequence of tests on the matched place. Each
//! test either falls through to the next or jumps to a "does not match"
//! block; variables are bound along the way. A `match` tries its arms in
//! order, each arm's failure block being the start of the next arm.

use super::drop::ScopeKind;
use super::FnBuilder;
use crate::ir::*;
use crate::sema::thir::{Arm, BindingMode, Expr, Literal, Pat, PatKind};
use crate::sema::ty::{IntTy, Mutability, Ty};
use crate::syntax::ast::BinOp;
use crate::syntax::diagnostic::Result;

impl FnBuilder<'_, '_, '_> {
    pub fn match_into(&mut self, scrutinee: &Expr, arms: &[Arm], dest: Place) -> Result<()> {
        // The matched value is evaluated exactly once, whatever the number of arms.
        let value = self.expr_place(scrutinee)?;
        let join = self.new_block();
        let owner = self.value_owner(scrutinee, &value);
        for arm in arms {
            let next_arm = self.new_block();
            self.push_scope(ScopeKind::Block);
            self.match_pattern(&arm.pat, value.clone(), next_arm)?;
            if let Some(guard) = &arm.guard {
                let body = self.new_block();
                self.cond(guard, body, next_arm)?;
                self.switch_to(body);
            }
            // Only now is the arm taken for certain.
            self.bindings_take_effect(&arm.pat, owner);
            self.expr_into(&arm.body, dest.clone())?;
            self.exit_scope()?;
            self.goto(join);
            self.switch_to(next_arm);
        }
        // Falling off the last arm cannot happen for an exhaustive match.
        self.terminate(Terminator::Unreachable);
        self.switch_to(join);
        Ok(())
    }

    /// The local that owns a matched value: the temporary it was put in, or
    /// the variable the scrutinee expression reads from.
    pub fn value_owner(&self, scrutinee: &Expr, value: &Place) -> Option<Local> {
        if scrutinee.is_place() {
            self.owner_of(scrutinee)
        } else {
            value.as_local()
        }
    }

    /// A pattern has matched for good: variables it bound by value now own
    /// what they hold, and the matched value's owner has given that up.
    ///
    /// This is separate from [`match_pattern`](Self::match_pattern) because
    /// a pattern can bind some variables and then fail further in, or match
    /// and be rejected by its guard; neither may transfer ownership.
    pub fn bindings_take_effect(&mut self, pat: &Pat, owner: Option<Local>) {
        let mut moved = false;
        self.for_each_value_binding(pat, &mut |builder, local| {
            builder.set_initialized(local);
            moved = true;
        });
        if let (true, Some(owner)) = (moved, owner) {
            self.set_moved(owner);
        }
    }

    /// Does the pattern bind any variable as a reference to the matched value?
    pub fn binds_by_reference(&self, pat: &Pat) -> bool {
        match &pat.kind {
            PatKind::Wild | PatKind::Literal(_) | PatKind::Range { .. } => false,
            PatKind::Binding { mode, sub, .. } => {
                matches!(mode, BindingMode::Ref(_)) || sub.as_ref().is_some_and(|sub| self.binds_by_reference(sub))
            }
            PatKind::Variant { fields, .. } | PatKind::Fields(fields) => {
                fields.iter().any(|(_, field)| self.binds_by_reference(field))
            }
            PatKind::Deref(inner) => self.binds_by_reference(inner),
            PatKind::Or(alternatives) => alternatives.iter().any(|pat| self.binds_by_reference(pat)),
            PatKind::Slice { prefix, rest, suffix } => {
                prefix.iter().chain(rest.as_deref()).chain(suffix).any(|element| self.binds_by_reference(element))
            }
        }
    }

    /// Create the variables a pattern binds, in the current scope.
    pub fn declare_bindings(&mut self, pat: &Pat) {
        match &pat.kind {
            PatKind::Wild | PatKind::Literal(_) | PatKind::Range { .. } => {}
            PatKind::Binding { local, sub, .. } => {
                self.var(*local);
                if let Some(sub) = sub {
                    self.declare_bindings(sub);
                }
            }
            PatKind::Variant { fields, .. } | PatKind::Fields(fields) => {
                fields.iter().for_each(|(_, field)| self.declare_bindings(field));
            }
            PatKind::Deref(inner) => self.declare_bindings(inner),
            PatKind::Or(alternatives) => alternatives.iter().for_each(|pat| self.declare_bindings(pat)),
            PatKind::Slice { prefix, rest, suffix } => {
                prefix.iter().chain(rest.as_deref()).chain(suffix).for_each(|element| self.declare_bindings(element));
            }
        }
    }

    /// Visit the locals `pat` binds by value whose types need dropping.
    fn for_each_value_binding(&mut self, pat: &Pat, visit: &mut dyn FnMut(&mut Self, Local)) {
        match &pat.kind {
            PatKind::Wild | PatKind::Literal(_) | PatKind::Range { .. } => {}
            PatKind::Binding { local, mode, sub } => {
                if *mode == BindingMode::Value && self.tcx.needs_drop(&pat.ty) {
                    if let Some(local) = self.var(*local).as_local() {
                        visit(self, local);
                    }
                }
                if let Some(sub) = sub {
                    self.for_each_value_binding(sub, visit);
                }
            }
            PatKind::Variant { fields, .. } | PatKind::Fields(fields) => {
                for (_, field) in fields {
                    self.for_each_value_binding(field, visit);
                }
            }
            PatKind::Deref(inner) => self.for_each_value_binding(inner, visit),
            // Every alternative binds the same variables.
            PatKind::Or(alternatives) => {
                if let Some(first) = alternatives.first() {
                    self.for_each_value_binding(first, visit);
                }
            }
            PatKind::Slice { prefix, rest, suffix } => {
                for element in prefix.iter().chain(rest.as_deref()).chain(suffix) {
                    self.for_each_value_binding(element, visit);
                }
            }
        }
    }

    /// Test `place` against `pat`, binding the pattern's variables. Control
    /// continues in the current block on a match and goes to `mismatch`
    /// otherwise. Ownership is settled separately, by
    /// [`bindings_take_effect`](Self::bindings_take_effect).
    pub fn match_pattern(&mut self, pat: &Pat, place: Place, mismatch: BlockId) -> Result<()> {
        match &pat.kind {
            PatKind::Wild => {}
            PatKind::Binding { local, mode, sub } => {
                let variable = self.var(*local);
                match mode {
                    BindingMode::Value if self.tcx.is_zero_sized(&pat.ty) => {}
                    BindingMode::Value => self.assign(variable, Rvalue::Use(Operand::Copy(place.clone()))),
                    BindingMode::Ref(_) => {
                        let address = self.address_of(place.clone());
                        self.assign(variable, address);
                    }
                }
                if let Some(sub) = sub {
                    self.match_pattern(sub, place, mismatch)?;
                }
            }
            PatKind::Literal(Literal::Str(text)) => {
                let data = self.program.bytes(text.as_bytes());
                let literal = Operand::Const(Const::Str(data, text.len() as u64));
                let equal = self.temp(Ty::Bool);
                let str_eq = self.runtime_fn("str_eq")?;
                self.push(Statement::Call {
                    dest: Place::local(equal),
                    callee: Callee::Direct(str_eq),
                    args: vec![Operand::Copy(place), literal],
                });
                self.continue_if(Operand::Copy(Place::local(equal)), mismatch);
            }
            PatKind::Literal(literal) => {
                let bits = match literal {
                    Literal::Int(value) => *value,
                    Literal::Bool(value) => *value as i128,
                    Literal::Char(value) => *value as i128,
                    Literal::Str(_) => unreachable!("handled above"),
                };
                self.compare(BinOp::Eq, &place, bits, &pat.ty, mismatch);
            }
            PatKind::Range { lo, hi, inclusive } => {
                if let Some(lo) = lo {
                    self.compare(BinOp::Ge, &place, *lo, &pat.ty, mismatch);
                }
                if let Some(hi) = hi {
                    let op = if *inclusive { BinOp::Le } else { BinOp::Lt };
                    self.compare(op, &place, *hi, &pat.ty, mismatch);
                }
            }
            PatKind::Variant { variant, fields } => {
                let Ty::Adt(adt, _) = &pat.ty else { unreachable!("variant pattern on a non-enum") };
                let tag_ty = Ty::Int(self.tcx.tag_type(&pat.ty));
                let tag = self.temp(tag_ty.clone());
                self.assign(Place::local(tag), Rvalue::Discriminant(place.clone()));
                let wanted = self.tcx.discriminant(*adt, *variant);
                self.compare(BinOp::Eq, &Place::local(tag), wanted, &tag_ty, mismatch);

                let payload = place.project(Projection::Downcast(*variant));
                for (index, field) in fields {
                    self.match_pattern(field, payload.clone().field(*index), mismatch)?;
                }
            }
            PatKind::Fields(fields) => {
                for (index, field) in fields {
                    self.match_pattern(field, place.clone().field(*index), mismatch)?;
                }
            }
            PatKind::Deref(inner) => self.match_pattern(inner, place.deref(), mismatch)?,
            PatKind::Or(alternatives) => {
                let matched = self.new_block();
                for alternative in alternatives {
                    let next_alternative = self.new_block();
                    self.match_pattern(alternative, place.clone(), next_alternative)?;
                    self.goto(matched);
                    self.switch_to(next_alternative);
                }
                self.goto(mismatch);
                self.switch_to(matched);
            }
            PatKind::Slice { prefix, rest, suffix } => self.match_slice(pat, place, prefix, rest.as_deref(), suffix, mismatch)?,
        }
        Ok(())
    }

    /// `[first, .., last]`: a slice must be long enough (exactly as long
    /// without `..`); an array's length was checked with its type. The
    /// suffix is counted from the end, and the rest is what lies between.
    fn match_slice(
        &mut self,
        pat: &Pat,
        place: Place,
        prefix: &[Pat],
        rest: Option<&Pat>,
        suffix: &[Pat],
        mismatch: BlockId,
    ) -> Result<()> {
        let (Ty::Array(element_ty, _) | Ty::Slice(element_ty)) = &pat.ty else {
            unreachable!("slice pattern on `{}`", self.tcx.display(&pat.ty))
        };
        let fixed = (prefix.len() + suffix.len()) as u64;
        let len = match pat.ty.array_len() {
            Some(len) => self.usize_const(len),
            None => {
                // A slice place is `*fat_pointer`; its length is the pointer's extra word.
                let mut pointer = place.clone();
                let last = pointer.projection.pop();
                debug_assert_eq!(last, Some(Projection::Deref), "a slice is only reachable through a pointer");
                let len = self.temp(Ty::USIZE);
                self.assign(Place::local(len), Rvalue::FatExtra(Operand::Copy(pointer)));
                let op = if rest.is_some() { BinOp::Ge } else { BinOp::Eq };
                self.compare(op, &Place::local(len), fixed as i128, &Ty::USIZE, mismatch);
                Operand::Copy(Place::local(len))
            }
        };
        let element_at = |builder: &mut Self, index: Rvalue| {
            let at = builder.temp(Ty::USIZE);
            builder.assign(Place::local(at), index);
            place.clone().project(Projection::Index(at))
        };
        for (index, element) in prefix.iter().enumerate() {
            let element_place = element_at(self, Rvalue::Use(self.usize_const(index as u64)));
            self.match_pattern(element, element_place, mismatch)?;
        }
        for (index, element) in suffix.iter().enumerate() {
            let from_end = self.usize_const((suffix.len() - index) as u64);
            let element_place = element_at(self, Rvalue::Binary(BinOp::Sub, len.clone(), from_end));
            self.match_pattern(element, element_place, mismatch)?;
        }
        let Some(Pat { kind: PatKind::Binding { local, mode, .. }, ty: rest_ty, .. }) = rest else {
            return Ok(());
        };
        // The elements in between start right after the prefix.
        let first = element_at(self, Rvalue::Use(self.usize_const(prefix.len() as u64)));
        let start = self.temp(Ty::Ptr(element_ty.clone(), Mutability::Not));
        self.assign(Place::local(start), Rvalue::AddrOf(first));
        let start = Operand::Copy(Place::local(start));
        let variable = self.var(*local);
        let start_ty = Ty::Ptr(element_ty.clone(), Mutability::Not);
        match (mode, rest_ty) {
            (BindingMode::Ref(_), Ty::Slice(_)) => {
                let count = self.temp(Ty::USIZE);
                self.assign(Place::local(count), Rvalue::Binary(BinOp::Sub, len, self.usize_const(fixed)));
                self.assign(variable, Rvalue::MakeFat(start, Operand::Copy(Place::local(count))));
            }
            (BindingMode::Ref(mutability), _) => {
                let to = Ty::Ref(Box::new(rest_ty.clone()), *mutability);
                self.assign(variable, Rvalue::Cast(start, start_ty, to));
            }
            (BindingMode::Value, _) => {
                let pointer = self.temp(Ty::Ptr(Box::new(rest_ty.clone()), Mutability::Not));
                self.assign(Place::local(pointer), Rvalue::Cast(start, start_ty, self.locals[pointer.0 as usize].ty.clone()));
                self.assign(variable, Rvalue::Use(Operand::Copy(Place::local(pointer).deref())));
            }
        }
        Ok(())
    }

    /// `&place` as a value. The address of `*pointer` is the pointer itself,
    /// which also keeps a fat pointer's extra word.
    pub fn address_of(&mut self, mut place: Place) -> Rvalue {
        if place.projection.last() == Some(&Projection::Deref) {
            place.projection.pop();
            Rvalue::Use(Operand::Copy(place))
        } else {
            Rvalue::AddrOf(place)
        }
    }

    /// `&place` for a place whose type is unsized, such as the last field
    /// of `*rc` in an `Rc<dyn Trait>`: its address, with the length or
    /// vtable of the pointer the place was reached through.
    pub fn address_of_unsized(&mut self, place: Place, dest: Place) {
        let through = place.projection.iter().rposition(|projection| *projection == Projection::Deref);
        let through = through.expect("an unsized place is reached through a pointer");
        let pointer = Place { local: place.local, projection: place.projection[..through].to_vec() };
        let address = self.temp(Ty::Ptr(Box::new(Ty::Int(IntTy::U8)), Mutability::Not));
        self.assign(Place::local(address), Rvalue::AddrOf(place));
        let extra = self.temp(Ty::USIZE);
        self.assign(Place::local(extra), Rvalue::FatExtra(Operand::Copy(pointer)));
        self.assign(dest, Rvalue::MakeFat(Operand::Copy(Place::local(address)), Operand::Copy(Place::local(extra))));
    }

    /// Carry on only if `place op constant` holds.
    fn compare(&mut self, op: BinOp, place: &Place, constant: i128, ty: &Ty, mismatch: BlockId) {
        let holds = self.temp(Ty::Bool);
        let constant = Operand::Const(Const::Int(constant as u128, ty.clone()));
        self.assign(Place::local(holds), Rvalue::Binary(op, Operand::Copy(place.clone()), constant));
        self.continue_if(Operand::Copy(Place::local(holds)), mismatch);
    }

    /// Branch: stay on this path if `condition` is true, else go to `otherwise`.
    fn continue_if(&mut self, condition: Operand, otherwise: BlockId) {
        let next = self.new_block();
        self.branch(condition, next, otherwise);
        self.switch_to(next);
    }
}
