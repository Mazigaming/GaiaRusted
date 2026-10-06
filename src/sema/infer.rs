//! Type inference by unification.
//!
//! While a function body is checked, types that are not known yet are
//! [`Ty::Infer`] variables. Every constraint the body imposes ("this argument
//! has the parameter's type", "both branches agree") becomes a call to
//! [`InferTable::unify`], which binds variables until everything is known.
//!
//! Number literals get their own kinds of variable: `1` may become any
//! integer type but never `bool`, and falls back to `i32` if nothing pins it
//! down — exactly Rust's rule.

use super::ty::{FloatTy, InferVar, IntTy, Ty};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarKind {
    /// Can become any type.
    General,
    /// An integer literal: can only become an integer type.
    Int,
    /// A float literal: can only become a float type.
    Float,
}

#[derive(Clone, Debug)]
enum VarState {
    Unbound(VarKind),
    Bound(Ty),
}

#[derive(Clone, Debug, Default)]
pub struct InferTable {
    vars: Vec<VarState>,
    /// Every binding made, with the state the variable had before, so that
    /// a [`Snapshot`] can be returned to by undoing the bindings since.
    undo_log: Vec<(InferVar, VarState)>,
}

/// A point in the life of an [`InferTable`] to roll back to.
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    vars: usize,
    undo_log: usize,
}

/// Two types that had to be equal were not.
#[derive(Debug)]
pub struct Mismatch;

impl InferTable {
    pub fn fresh(&mut self, kind: VarKind) -> Ty {
        self.vars.push(VarState::Unbound(kind));
        Ty::Infer(InferVar(self.vars.len() as u32 - 1))
    }

    pub fn fresh_var(&mut self) -> Ty {
        self.fresh(VarKind::General)
    }

    /// The kind of `var` if it is still unsolved.
    pub fn unbound_kind(&self, var: InferVar) -> Option<VarKind> {
        match &self.vars[var.0 as usize] {
            VarState::Unbound(kind) => Some(*kind),
            VarState::Bound(_) => None,
        }
    }

    /// Follow variable bindings at the top of `ty` only: the result is either
    /// a non-variable type or a still-unsolved variable.
    pub fn shallow(&self, ty: &Ty) -> Ty {
        let mut current = ty;
        while let Ty::Infer(var) = current {
            match &self.vars[var.0 as usize] {
                VarState::Bound(bound) => current = bound,
                VarState::Unbound(_) => break,
            }
        }
        current.clone()
    }

    /// Replace every solved variable anywhere inside `ty`.
    pub fn resolve(&self, ty: &Ty) -> Ty {
        ty.map(&mut |nested| match nested {
            Ty::Infer(_) => {
                let solved = self.shallow(&nested);
                if matches!(solved, Ty::Infer(_)) {
                    solved
                } else {
                    self.resolve(&solved)
                }
            }
            other => other,
        })
    }

    /// Like [`resolve`](Self::resolve), then give unsolved literal variables
    /// their default type (`i32` / `f64`). General variables stay as they are.
    pub fn resolve_with_defaults(&self, ty: &Ty) -> Ty {
        self.resolve(ty).map(&mut |nested| match nested {
            Ty::Infer(var) => match self.unbound_kind(var) {
                Some(VarKind::Int) => Ty::Int(IntTy::I32),
                Some(VarKind::Float) => Ty::Float(FloatTy::F64),
                _ => nested,
            },
            other => other,
        })
    }

    /// Give the unsolved literal variables inside `ty` their default types
    /// now, instead of at the end of the function. Returns whether any were
    /// found. Needed when a choice between impls hinges on such a variable
    /// (`Range<{integer}>` could be a range of any integer type).
    pub fn default_literal_vars(&mut self, ty: &Ty) -> bool {
        let mut literals = Vec::new();
        self.resolve(ty).walk(&mut |nested| {
            if let Ty::Infer(var) = nested {
                if matches!(self.unbound_kind(*var), Some(VarKind::Int | VarKind::Float)) {
                    literals.push(*var);
                }
            }
        });
        for var in &literals {
            let default = match self.unbound_kind(*var) {
                Some(VarKind::Int) => Ty::I32,
                Some(VarKind::Float) => Ty::F64,
                _ => continue,
            };
            self.set(*var, VarState::Bound(default));
        }
        !literals.is_empty()
    }

    /// Make `a` and `b` the same type, binding variables as needed.
    pub fn unify(&mut self, a: &Ty, b: &Ty) -> Result<(), Mismatch> {
        let a = self.shallow(a);
        let b = self.shallow(b);
        match (&a, &b) {
            (Ty::Infer(x), Ty::Infer(y)) if x == y => Ok(()),
            (Ty::Infer(x), Ty::Infer(y)) => {
                // Keep the more specific kind alive: bind the general one.
                let (x_kind, y_kind) = (self.unbound_kind(*x), self.unbound_kind(*y));
                match (x_kind, y_kind) {
                    (Some(VarKind::General), _) => self.set(*x, VarState::Bound(b)),
                    (_, Some(VarKind::General)) => self.set(*y, VarState::Bound(a)),
                    _ if x_kind == y_kind => self.set(*x, VarState::Bound(b)),
                    // An integer literal is never a float literal.
                    _ => return Err(Mismatch),
                }
                Ok(())
            }
            (Ty::Infer(var), other) | (other, Ty::Infer(var)) => self.bind(*var, other.clone()),

            (Ty::Tuple(xs), Ty::Tuple(ys)) => self.unify_all(xs, ys),
            (Ty::Adt(x, xs), Ty::Adt(y, ys)) if x == y => self.unify_all(xs, ys),
            (Ty::FnItem(x, xs), Ty::FnItem(y, ys)) if x == y => self.unify_all(xs, ys),
            (Ty::Dyn(x, xs), Ty::Dyn(y, ys)) if x == y => self.unify_all(xs, ys),
            (Ty::Array(x, x_len), Ty::Array(y, y_len)) => {
                self.unify(x_len, y_len)?;
                self.unify(x, y)
            }
            (Ty::Slice(x), Ty::Slice(y)) => self.unify(x, y),
            (Ty::Ref(x, x_mut), Ty::Ref(y, y_mut)) | (Ty::Ptr(x, x_mut), Ty::Ptr(y, y_mut))
                if x_mut == y_mut =>
            {
                self.unify(x, y)
            }
            (Ty::FnPtr(x_params, x_ret), Ty::FnPtr(y_params, y_ret)) => {
                self.unify_all(x_params, y_params)?;
                self.unify(x_ret, y_ret)
            }
            _ if a == b => Ok(()),
            _ => Err(Mismatch),
        }
    }

    fn unify_all(&mut self, xs: &[Ty], ys: &[Ty]) -> Result<(), Mismatch> {
        if xs.len() != ys.len() {
            return Err(Mismatch);
        }
        xs.iter().zip(ys).try_for_each(|(x, y)| self.unify(x, y))
    }

    fn bind(&mut self, var: InferVar, ty: Ty) -> Result<(), Mismatch> {
        let allowed = match self.unbound_kind(var).expect("only unsolved variables are bound") {
            VarKind::General => true,
            VarKind::Int => matches!(ty, Ty::Int(_)),
            VarKind::Float => matches!(ty, Ty::Float(_)),
        };
        // `?T = Vec<?T>` has no finite solution.
        let mut occurs = false;
        self.resolve(&ty).walk(&mut |nested| occurs |= *nested == Ty::Infer(var));
        if !allowed || occurs {
            return Err(Mismatch);
        }
        self.set(var, VarState::Bound(ty));
        Ok(())
    }

    fn set(&mut self, var: InferVar, state: VarState) {
        let old = std::mem::replace(&mut self.vars[var.0 as usize], state);
        self.undo_log.push((var, old));
    }

    /// Would `a` and `b` unify? Leaves the table as it was.
    pub fn can_unify(&mut self, a: &Ty, b: &Ty) -> bool {
        let snapshot = self.snapshot();
        let unified = self.unify(a, b).is_ok();
        self.rollback_to(snapshot);
        unified
    }

    /// Unify, but undo every binding made if it turns out to be a mismatch.
    pub fn try_unify(&mut self, a: &Ty, b: &Ty) -> bool {
        let snapshot = self.snapshot();
        let unified = self.unify(a, b).is_ok();
        if !unified {
            self.rollback_to(snapshot);
        }
        unified
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot { vars: self.vars.len(), undo_log: self.undo_log.len() }
    }

    /// Undo every binding since the snapshot, and forget the variables made
    /// since.
    pub fn rollback_to(&mut self, snapshot: Snapshot) {
        while self.undo_log.len() > snapshot.undo_log {
            let (var, old) = self.undo_log.pop().expect("longer than the snapshot's");
            self.vars[var.0 as usize] = old;
        }
        self.vars.truncate(snapshot.vars);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sema::ty::AdtId;

    #[test]
    fn variables_are_solved_through_structure() {
        let mut table = InferTable::default();
        let t = table.fresh_var();
        let vec_of_t = Ty::Adt(AdtId(0), vec![t.clone()]);
        let vec_of_i64 = Ty::Adt(AdtId(0), vec![Ty::I64]);
        assert!(table.unify(&vec_of_t, &vec_of_i64).is_ok());
        assert_eq!(table.resolve(&t), Ty::I64);
    }

    #[test]
    fn integer_literals_only_become_integers_and_default_to_i32() {
        let mut table = InferTable::default();
        let literal = table.fresh(VarKind::Int);
        assert!(table.unify(&literal, &Ty::Bool).is_err());
        assert_eq!(table.resolve_with_defaults(&literal), Ty::I32);
        assert!(table.unify(&literal, &Ty::U8).is_ok());
        assert_eq!(table.resolve_with_defaults(&literal), Ty::U8);
    }

    #[test]
    fn a_literal_keeps_its_kind_when_unified_with_a_general_variable() {
        let mut table = InferTable::default();
        let general = table.fresh_var();
        let literal = table.fresh(VarKind::Int);
        assert!(table.unify(&general, &literal).is_ok());
        assert!(table.unify(&general, &Ty::Bool).is_err());
    }

    #[test]
    fn two_literals_of_the_same_kind_unify() {
        let mut table = InferTable::default();
        let (a, b) = (table.fresh(VarKind::Int), table.fresh(VarKind::Int));
        assert!(table.unify(&a, &b).is_ok());
        assert!(table.unify(&b, &Ty::USIZE).is_ok());
        assert_eq!(table.resolve(&a), Ty::USIZE);
        let float = table.fresh(VarKind::Float);
        assert!(table.unify(&a, &float).is_err());
    }

    #[test]
    fn infinite_types_are_rejected() {
        let mut table = InferTable::default();
        let t = table.fresh_var();
        let list_of_t = Ty::Slice(Box::new(t.clone()));
        assert!(table.unify(&t, &list_of_t).is_err());
    }

    #[test]
    fn failed_attempts_leave_no_bindings_behind() {
        let mut table = InferTable::default();
        let t = table.fresh_var();
        let pair = Ty::Tuple(vec![t.clone(), Ty::Bool]);
        assert!(!table.try_unify(&pair, &Ty::Tuple(vec![Ty::I64, Ty::Char])));
        assert_eq!(table.resolve(&t), t);
    }
}
