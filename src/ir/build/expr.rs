//! Expressions, statements and control flow.
//!
//! An expression is lowered in one of three ways, depending on what the
//! context needs:
//!
//! * [`expr_into`](FnBuilder::expr_into) computes it straight into a
//!   destination place — no temporary for `let p = Point { .. }`;
//! * [`expr_operand`](FnBuilder::expr_operand) yields something a statement
//!   can use directly, a constant or a place to read;
//! * [`expr_place`](FnBuilder::expr_place) yields the memory location the
//!   expression denotes.

use super::drop::ScopeKind;
use super::{FnBuilder, FuncKey, LoopScope};
use crate::ir::*;
use crate::sema::thir::{self, Expr, ExprKind, Stmt};
use crate::sema::ty::{IntTy, Ty};
use crate::syntax::ast::BinOp;
use crate::syntax::diagnostic::{bail, Result};

impl FnBuilder<'_, '_, '_> {
    /// Evaluate `expr` and store its value in `dest`.
    ///
    /// `dest` must be memory that `expr` cannot observe: a fresh local, a
    /// part of a value under construction, the return slot.
    pub fn expr_into(&mut self, expr: &Expr, dest: Place) -> Result<()> {
        // Set only for the initialiser of a `let` (and the parts of it that
        // end up stored in the variable): a temporary borrowed there must
        // live as long as the variable.
        let extends_temporaries = std::mem::take(&mut self.extend_temporaries);
        match &expr.kind {
            ExprKind::Int(value) => self.assign(dest, Rvalue::Use(self.int_const(*value, &expr.ty))),
            ExprKind::Bool(value) => self.assign(dest, Rvalue::Use(self.int_const(*value as u128, &expr.ty))),
            ExprKind::Char(value) => self.assign(dest, Rvalue::Use(self.int_const(*value as u128, &expr.ty))),
            ExprKind::Float(value) => {
                self.assign(dest, Rvalue::Use(Operand::Const(Const::Float(*value, expr.ty.clone()))))
            }
            ExprKind::Str(_) | ExprKind::ByteStr(_) => {
                let operand = self.expr_operand(expr)?;
                self.assign(dest, Rvalue::Use(operand));
            }
            ExprKind::ZeroSized => {}

            ExprKind::Local(_) | ExprKind::Static(_) | ExprKind::Deref(_) | ExprKind::Field(..) | ExprKind::Index(..) => {
                let place = self.expr_place(expr)?;
                if !self.tcx.is_zero_sized(&expr.ty) {
                    self.assign(dest, Rvalue::Use(Operand::Copy(place.clone())));
                }
                self.moved_out_of(expr, &place);
            }

            ExprKind::Unary(op, operand) => {
                let operand = self.expr_operand(operand)?;
                let op = match op {
                    thir::UnaryOp::Neg => UnaryOp::Neg,
                    thir::UnaryOp::Not => UnaryOp::Not,
                };
                self.assign(dest, Rvalue::Unary(op, operand));
            }
            ExprKind::Binary(BinOp::And | BinOp::Or, ..) | ExprKind::Let(..) => {
                // A condition used as a value: branch, then store `true` or `false`.
                let (then_block, else_block, join) = (self.new_block(), self.new_block(), self.new_block());
                self.cond(expr, then_block, else_block)?;
                for (block, value) in [(then_block, 1), (else_block, 0)] {
                    self.switch_to(block);
                    self.assign(dest.clone(), Rvalue::Use(self.int_const(value, &Ty::Bool)));
                    self.goto(join);
                }
                self.switch_to(join);
            }
            ExprKind::Binary(op, lhs, rhs) => {
                let lhs_operand = self.expr_operand(lhs)?;
                let rhs_operand = self.expr_operand(rhs)?;
                self.check_divisor(*op, &lhs.ty, &rhs_operand)?;
                self.binary_into(dest, *op, &lhs.ty, lhs_operand, rhs_operand);
            }
            ExprKind::Cast(operand) => {
                let mut from = operand.ty.clone();
                let mut value = self.expr_operand(operand)?;
                // An enum converts through its tag.
                if matches!(&from, Ty::Adt(adt, _) if self.tcx.is_enum(*adt)) {
                    let Operand::Copy(place) = value else { unreachable!("an enum value lives in a place") };
                    let tag_ty = Ty::Int(self.tcx.tag_type(&from));
                    let tag = self.temp(tag_ty.clone());
                    self.assign(Place::local(tag), Rvalue::Discriminant(place));
                    (from, value) = (tag_ty, Operand::Copy(Place::local(tag)));
                }
                match wide_float_conversion(&from, &expr.ty) {
                    Some(symbol) => self.call_runtime_symbol(dest, symbol, vec![value]),
                    None => self.assign(dest, Rvalue::Cast(value, from, expr.ty.clone())),
                }
            }

            ExprKind::AddrOf(_, operand) => {
                // `&*pointer` is the pointer itself (this is also how a
                // reference to unsized data keeps its length or vtable).
                if let ExprKind::Deref(pointer) = &operand.kind {
                    let pointer = self.expr_operand(pointer)?;
                    self.assign(dest, Rvalue::Use(pointer));
                } else {
                    let lifetime = if extends_temporaries { ScopeKind::Block } else { ScopeKind::Statement };
                    let place = self.expr_place_in(operand, lifetime)?;
                    if self.tcx.is_unsized(&operand.ty) {
                        self.address_of_unsized(place, dest);
                    } else {
                        self.assign(dest, Rvalue::AddrOf(place));
                    }
                }
            }

            ExprKind::Call(callee, args) => self.call_into(callee, args, expr, dest)?,
            ExprKind::Adt { variant, fields, base } => {
                let is_enum = matches!(&expr.ty, Ty::Adt(adt, _) if self.tcx.is_enum(*adt));
                if let Some(base) = base {
                    // `..base` supplies every field not listed explicitly.
                    let base = self.expr_operand(base)?;
                    self.assign(dest.clone(), Rvalue::Use(base));
                }
                for (index, value) in fields {
                    let target = if is_enum {
                        dest.clone().project(Projection::Downcast(*variant)).field(*index)
                    } else {
                        dest.clone().field(*index)
                    };
                    self.extend_temporaries = extends_temporaries;
                    self.expr_into(value, target)?;
                }
                if is_enum {
                    self.push(Statement::SetDiscriminant(dest, *variant));
                }
            }
            ExprKind::Tuple(elements) | ExprKind::Array(elements) => {
                for (index, element) in elements.iter().enumerate() {
                    self.extend_temporaries = extends_temporaries;
                    self.expr_into(element, dest.clone().field(index))?;
                }
            }
            ExprKind::Repeat(element, count) => self.repeat_into(element, *count, dest)?,
            ExprKind::Closure(id) => {
                let closure = self.program.closure(*id);
                for (index, capture) in closure.captures.iter().enumerate() {
                    let captured = self.var(capture.local);
                    let value = if capture.by_ref {
                        Rvalue::AddrOf(captured)
                    } else {
                        if let Some(owner) = captured.as_local() {
                            self.set_moved(owner);
                        }
                        Rvalue::Use(Operand::Copy(captured))
                    };
                    self.assign(dest.clone().field(index), value);
                }
                for (_, flag) in self.tcx.closure_drop_flags(*id) {
                    self.assign(dest.clone().field(flag), Rvalue::Use(self.int_const(1, &Ty::Bool)));
                }
            }

            ExprKind::Unsize(pointer) => {
                let extra = self.unsize_extra(&pointer.ty, &expr.ty)?;
                let is_box = matches!(&pointer.ty, Ty::Adt(..));
                let pointer = self.expr_operand(pointer)?;
                if is_box {
                    // The box is a struct around its pointer: widen that field.
                    let Operand::Copy(source) = pointer else { unreachable!("a box lives in a place") };
                    let thin = Operand::Copy(source.field(0));
                    self.assign(dest.field(0), Rvalue::MakeFat(thin, extra));
                } else {
                    self.assign(dest, Rvalue::MakeFat(pointer, extra));
                }
            }
            ExprKind::ReifyFnPointer(callable) => {
                let key = match &callable.ty {
                    Ty::FnItem(def, substs) => {
                        FuncKey::Instance(thir::Instance { def: *def, substs: substs.clone() })
                    }
                    Ty::Closure(id) => {
                        if !self.program.closure(*id).captures.is_empty() {
                            bail!(expr.span, "a closure that captures variables cannot be used as a function pointer");
                        }
                        FuncKey::ClosureAsFnPointer(*id)
                    }
                    other => unreachable!("`{}` is not a function", self.tcx.display(other)),
                };
                // The callable itself is zero-sized; evaluate it only for side effects.
                self.discard(callable)?;
                let func = self.program.func_id(key);
                self.assign(dest, Rvalue::Use(Operand::Const(Const::FuncAddr(func, expr.ty.clone()))));
            }

            ExprKind::Block(block) => self.block_into(block, dest)?,
            ExprKind::If(cond, then_expr, else_expr) => {
                let (then_block, else_block, join) = (self.new_block(), self.new_block(), self.new_block());
                // What the condition creates (variables bound by `if let`,
                // temporaries) is gone before the `else` branch runs.
                self.push_scope(ScopeKind::Block);
                let depth = self.scope_depth() - 1;
                self.cond(cond, then_block, else_block)?;
                self.switch_to(else_block);
                self.drop_scopes_from(depth)?;
                let else_entry = self.current;

                self.switch_to(then_block);
                if binds_variables(cond) {
                    self.expr_into(then_expr, dest.clone())?;
                    self.exit_scope()?;
                } else {
                    // A plain condition is a scope of its own: its
                    // temporaries are dropped before either branch runs.
                    self.exit_scope()?;
                    self.expr_into(then_expr, dest.clone())?;
                }
                self.goto(join);

                self.switch_to(else_entry);
                if let Some(else_expr) = else_expr {
                    self.expr_into(else_expr, dest)?;
                }
                self.goto(join);
                self.switch_to(join);
            }
            ExprKind::Match(scrutinee, arms) => self.match_into(scrutinee, arms, dest)?,
            ExprKind::Loop(id, body) => {
                let (head, exit) = (self.new_block(), self.new_block());
                self.goto(head);
                self.switch_to(head);
                let scope_depth = self.scope_depth();
                self.loops.push(LoopScope { id: *id, break_block: exit, continue_block: head, result: dest, scope_depth });
                let unit = self.temp(Ty::UNIT);
                self.block_into(body, Place::local(unit))?;
                self.loops.pop();
                self.goto(head);
                self.switch_to(exit);
            }
            ExprKind::Break(id, value) => {
                let scope = self.loops.iter().rev().find(|scope| scope.id == *id).expect("break targets an enclosing loop");
                let (target, result, depth) = (scope.break_block, scope.result.clone(), scope.scope_depth);
                if let Some(value) = value {
                    self.expr_into(value, result)?;
                }
                self.drop_scopes_from(depth)?;
                self.terminate(Terminator::Goto(target));
            }
            ExprKind::Continue(id) => {
                let scope = self.loops.iter().rev().find(|scope| scope.id == *id).expect("continue targets an enclosing loop");
                let (target, depth) = (scope.continue_block, scope.scope_depth);
                self.drop_scopes_from(depth)?;
                self.terminate(Terminator::Goto(target));
            }
            ExprKind::Return(value) => {
                if let Some(value) = value {
                    self.expr_into(value, Place::local(RETURN_LOCAL))?;
                }
                self.drop_scopes_from(0)?;
                self.terminate(Terminator::Return);
            }

            ExprKind::Assign(target, value) => {
                // The new value is computed in full before the old one is
                // overwritten: `p = Point { x: p.y, y: p.x }` reads the old `p`.
                let value = self.expr_operand(value)?;
                let place = self.expr_place(target)?;
                self.drop_overwritten(target, &place)?;
                if !self.tcx.is_zero_sized(&target.ty) {
                    self.assign(place.clone(), Rvalue::Use(value));
                }
                if let Some(local) = place.as_local() {
                    self.set_initialized(local);
                }
            }
            ExprKind::AssignOp(op, target, value) => {
                let value = self.expr_operand(value)?;
                let place = self.expr_place(target)?;
                self.check_divisor(*op, &target.ty, &value)?;
                self.binary_into(place.clone(), *op, &target.ty, Operand::Copy(place), value);
            }
        }
        Ok(())
    }

    /// Evaluate `expr` for its effects only. Its value is a temporary of the
    /// current statement.
    pub fn discard(&mut self, expr: &Expr) -> Result<()> {
        let scratch = self.temp(expr.ty.clone());
        self.expr_into(expr, Place::local(scratch))?;
        self.own(scratch, ScopeKind::Statement);
        self.set_initialized(scratch);
        Ok(())
    }

    /// An assignment destroys the value its target held before.
    fn drop_overwritten(&mut self, target: &Expr, place: &Place) -> Result<()> {
        if !self.tcx.needs_drop(&target.ty) {
            return Ok(());
        }
        // A local or a field of one: only what it still holds.
        if self.local_part(place).is_some() {
            return self.drop_part_if_owned(place.clone());
        }
        match self.owner_of(target).and_then(|owner| self.flags.get(&owner).copied()) {
            // Owned by a variable: only if that variable still holds a value.
            Some(flag) => self.drop_place_if(place.clone(), &target.ty, flag),
            // Behind a pointer: such a place always holds a value.
            None => self.drop_place(place.clone(), &target.ty),
        }
    }

    /// Evaluate `expr` to something a statement can read: a constant, or a
    /// place (a fresh temporary unless `expr` already is one).
    pub fn expr_operand(&mut self, expr: &Expr) -> Result<Operand> {
        Ok(match &expr.kind {
            ExprKind::Int(value) => self.int_const(*value, &expr.ty),
            ExprKind::Bool(value) => self.int_const(*value as u128, &expr.ty),
            ExprKind::Char(value) => self.int_const(*value as u128, &expr.ty),
            ExprKind::Float(value) => Operand::Const(Const::Float(*value, expr.ty.clone())),
            ExprKind::Str(text) => {
                let data = self.program.bytes(text.as_bytes());
                Operand::Const(Const::Str(data, text.len() as u64))
            }
            ExprKind::ByteStr(bytes) => {
                let data = self.program.bytes(bytes);
                Operand::Const(Const::DataAddr(data, expr.ty.clone()))
            }
            ExprKind::ZeroSized => Operand::Const(Const::ZeroSized(expr.ty.clone())),
            _ if expr.is_place() => {
                let place = self.expr_place(expr)?;
                self.moved_out_of(expr, &place);
                Operand::Copy(place)
            }
            // The value goes straight to whoever asked for the operand, so
            // no scope has to own this temporary.
            _ => {
                let temp = self.temp(expr.ty.clone());
                self.expr_into(expr, Place::local(temp))?;
                Operand::Copy(Place::local(temp))
            }
        })
    }

    fn int_const(&self, value: u128, ty: &Ty) -> Operand {
        Operand::Const(Const::Int(value, ty.clone()))
    }

    /// The memory location `expr` denotes. A value that is not stored
    /// anywhere yet is put in a temporary of the current statement first.
    pub fn expr_place(&mut self, expr: &Expr) -> Result<Place> {
        self.expr_place_in(expr, ScopeKind::Statement)
    }

    /// Like [`expr_place`](Self::expr_place); `lifetime` says which kind of
    /// scope owns the temporary, if one is needed.
    pub fn expr_place_in(&mut self, expr: &Expr, lifetime: ScopeKind) -> Result<Place> {
        Ok(match &expr.kind {
            ExprKind::Local(id) => self.var(*id),
            ExprKind::Static(id) => {
                let data = self.program.static_data(*id)?;
                let pointer_ty = Ty::mut_ref(expr.ty.clone());
                let pointer = self.temp(pointer_ty.clone());
                self.assign(Place::local(pointer), Rvalue::Use(Operand::Const(Const::DataAddr(data, pointer_ty))));
                Place::local(pointer).deref()
            }
            ExprKind::Deref(pointer) => self.expr_place_in(pointer, lifetime)?.deref(),
            ExprKind::Field(base, index) => self.expr_place_in(base, lifetime)?.field(*index),
            ExprKind::Index(base, index) => {
                let index_operand = self.expr_operand(index)?;
                let index_local = match index_operand {
                    Operand::Copy(Place { local, projection }) if projection.is_empty() => local,
                    other => {
                        let local = self.temp(Ty::USIZE);
                        self.assign(Place::local(local), Rvalue::Use(other));
                        local
                    }
                };
                let base_place = self.expr_place(base)?;
                let len = match &base.ty {
                    array @ Ty::Array(..) => {
                        self.usize_const(array.array_len().expect("array lengths are known after type checking"))
                    }
                    // A slice place is `*fat_pointer`; its length is the pointer's extra word.
                    Ty::Slice(_) => {
                        let mut pointer = base_place.clone();
                        let last = pointer.projection.pop();
                        debug_assert_eq!(last, Some(Projection::Deref), "a slice is only reachable through a pointer");
                        let len = self.temp(Ty::USIZE);
                        self.assign(Place::local(len), Rvalue::FatExtra(Operand::Copy(pointer)));
                        Operand::Copy(Place::local(len))
                    }
                    other => unreachable!("cannot index `{}`", self.tcx.display(other)),
                };
                self.bounds_check(index_local, len)?;
                base_place.project(Projection::Index(index_local))
            }
            _ => {
                let temp = self.temp(expr.ty.clone());
                self.expr_into(expr, Place::local(temp))?;
                self.own(temp, lifetime);
                self.set_initialized(temp);
                Place::local(temp)
            }
        })
    }

    /// Stop the program if `index >= len`.
    fn bounds_check(&mut self, index: Local, len: Operand) -> Result<()> {
        let in_bounds = self.temp(Ty::Bool);
        let index = Operand::Copy(Place::local(index));
        self.assign(Place::local(in_bounds), Rvalue::Binary(BinOp::Lt, index.clone(), len.clone()));
        let (ok, out_of_bounds) = (self.new_block(), self.new_block());
        self.branch(Operand::Copy(Place::local(in_bounds)), ok, out_of_bounds);

        self.switch_to(out_of_bounds);
        let panic = self.runtime_fn("panic_bounds_check")?;
        let never = self.temp(Ty::Never);
        self.push(Statement::Call { dest: Place::local(never), callee: Callee::Direct(panic), args: vec![index, len] });
        self.terminate(Terminator::Unreachable);
        self.switch_to(ok);
        Ok(())
    }

    /// `dest = lhs op rhs`. Dividing 128-bit integers is a call: the
    /// machine has no instruction for it.
    fn binary_into(&mut self, dest: Place, op: BinOp, ty: &Ty, lhs: Operand, rhs: Operand) {
        let symbol = match (op, ty) {
            (BinOp::Div, Ty::Int(IntTy::I128)) => "__divti3",
            (BinOp::Div, Ty::Int(IntTy::U128)) => "__udivti3",
            (BinOp::Rem, Ty::Int(IntTy::I128)) => "__modti3",
            (BinOp::Rem, Ty::Int(IntTy::U128)) => "__umodti3",
            _ => return self.assign(dest, Rvalue::Binary(op, lhs, rhs)),
        };
        self.call_runtime_symbol(dest, symbol, vec![lhs, rhs]);
    }

    /// Call one of the C runtime's support routines (`libgcc`).
    fn call_runtime_symbol(&mut self, dest: Place, symbol: &str, args: Vec<Operand>) {
        let callee = Callee::Extern { symbol: symbol.to_string(), variadic: false };
        self.push(Statement::Call { dest, callee, args });
    }

    /// Integer division and remainder stop the program on a zero divisor.
    fn check_divisor(&mut self, op: BinOp, ty: &Ty, divisor: &Operand) -> Result<()> {
        if !matches!(op, BinOp::Div | BinOp::Rem) || !ty.is_integer() {
            return Ok(());
        }
        if let Operand::Const(Const::Int(value, _)) = divisor {
            if *value != 0 {
                return Ok(());
            }
        }
        let is_zero = self.temp(Ty::Bool);
        let zero = Operand::Const(Const::Int(0, ty.clone()));
        self.assign(Place::local(is_zero), Rvalue::Binary(BinOp::Eq, divisor.clone(), zero));
        let (by_zero, ok) = (self.new_block(), self.new_block());
        self.branch(Operand::Copy(Place::local(is_zero)), by_zero, ok);

        self.switch_to(by_zero);
        let panic = self.runtime_fn("panic_div_zero")?;
        let never = self.temp(Ty::Never);
        self.push(Statement::Call { dest: Place::local(never), callee: Callee::Direct(panic), args: Vec::new() });
        self.terminate(Terminator::Unreachable);
        self.switch_to(ok);
        Ok(())
    }

    /// `[value; count]`: evaluate `value` once, then store it `count` times.
    fn repeat_into(&mut self, element: &Expr, count: u64, dest: Place) -> Result<()> {
        let value = self.expr_operand(element)?;
        if count <= 8 {
            for index in 0..count {
                self.assign(dest.clone().field(index as usize), Rvalue::Use(value.clone()));
            }
            return Ok(());
        }
        let index = self.temp(Ty::USIZE);
        self.assign(Place::local(index), Rvalue::Use(self.usize_const(0)));
        let (head, body, exit) = (self.new_block(), self.new_block(), self.new_block());
        self.goto(head);

        self.switch_to(head);
        let more = self.temp(Ty::Bool);
        let index_value = Operand::Copy(Place::local(index));
        self.assign(Place::local(more), Rvalue::Binary(BinOp::Lt, index_value.clone(), self.usize_const(count)));
        self.branch(Operand::Copy(Place::local(more)), body, exit);

        self.switch_to(body);
        self.assign(dest.project(Projection::Index(index)), Rvalue::Use(value));
        self.assign(Place::local(index), Rvalue::Binary(BinOp::Add, index_value, self.usize_const(1)));
        self.goto(head);
        self.switch_to(exit);
        Ok(())
    }

    // -- blocks, statements and conditions -------------------------------------------

    pub fn block_into(&mut self, block: &thir::Block, dest: Place) -> Result<()> {
        let extends_temporaries = std::mem::take(&mut self.extend_temporaries);
        self.push_scope(ScopeKind::Block);
        for stmt in &block.stmts {
            self.push_scope(ScopeKind::Statement);
            self.statement(stmt)?;
            self.exit_scope()?;
        }
        if let Some(tail) = &block.expr {
            self.extend_temporaries = extends_temporaries;
            self.expr_into(tail, dest)?;
        }
        self.exit_scope()
    }

    fn statement(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Expr(expr) => self.discard(expr),
            Stmt::Let { pat, init: None, .. } => {
                // `let x;` — the variable belongs to this block even though
                // it gets its value later, possibly in a nested one.
                self.declare_bindings(pat);
                Ok(())
            }
            Stmt::Let { pat, init: Some(init), else_block } => {
                // `let name = value;` builds the value directly in the variable.
                if let thir::PatKind::Binding { local, mode: thir::BindingMode::Value, sub: None } = &pat.kind {
                    let place = self.var(*local);
                    self.extend_temporaries = true;
                    self.expr_into(init, place.clone())?;
                    if let Some(local) = place.as_local() {
                        self.set_initialized(local);
                    }
                    return Ok(());
                }
                // If the pattern binds references into the value, a temporary
                // holding it must live as long as they do: to the end of the
                // block. Otherwise it is done with at the end of the statement
                // (`let _ = f();` drops the result at once).
                let lifetime = if self.binds_by_reference(pat) { ScopeKind::Block } else { ScopeKind::Statement };
                self.extend_temporaries = lifetime == ScopeKind::Block;
                let value = self.expr_place_in(init, lifetime)?;
                self.extend_temporaries = false;
                let owner = self.value_owner(init, &value);
                let mismatch = self.new_block();
                self.match_pattern(pat, value, mismatch)?;
                self.bindings_take_effect(pat, owner);
                let after = self.new_block();
                self.goto(after);

                // Without `else` the pattern always matches and this block is dead.
                self.switch_to(mismatch);
                if let Some(block) = else_block {
                    let never = self.temp(Ty::Never);
                    self.block_into(block, Place::local(never))?;
                }
                self.terminate(Terminator::Unreachable);
                self.switch_to(after);
                Ok(())
            }
        }
    }

    /// Branch on a condition without materialising it as a `bool` where that
    /// can be avoided: `a && b`, `!a` and `let` patterns become control flow.
    pub fn cond(&mut self, expr: &Expr, then_block: BlockId, else_block: BlockId) -> Result<()> {
        // (`binds_variables` below must agree with what this treats as a `let`.)
        match &expr.kind {
            ExprKind::Binary(BinOp::And, lhs, rhs) => {
                let rhs_block = self.new_block();
                self.cond(lhs, rhs_block, else_block)?;
                self.switch_to(rhs_block);
                self.cond(rhs, then_block, else_block)
            }
            ExprKind::Binary(BinOp::Or, lhs, rhs) => {
                let rhs_block = self.new_block();
                self.cond(lhs, then_block, rhs_block)?;
                self.switch_to(rhs_block);
                self.cond(rhs, then_block, else_block)
            }
            ExprKind::Unary(thir::UnaryOp::Not, operand) if operand.ty == Ty::Bool => {
                self.cond(operand, else_block, then_block)
            }
            ExprKind::Let(pat, scrutinee) => {
                let value = self.expr_place(scrutinee)?;
                let owner = self.value_owner(scrutinee, &value);
                self.match_pattern(pat, value, else_block)?;
                self.bindings_take_effect(pat, owner);
                self.goto(then_block);
                Ok(())
            }
            ExprKind::Bool(value) => {
                self.goto(if *value { then_block } else { else_block });
                Ok(())
            }
            _ => {
                let value = self.expr_operand(expr)?;
                self.branch(value, then_block, else_block);
                Ok(())
            }
        }
    }
}

/// Does a condition bind variables, as `if let` and `let` chains do?
fn binds_variables(cond: &Expr) -> bool {
    match &cond.kind {
        ExprKind::Let(..) => true,
        ExprKind::Binary(BinOp::And | BinOp::Or, lhs, rhs) => binds_variables(lhs) || binds_variables(rhs),
        ExprKind::Unary(thir::UnaryOp::Not, operand) => binds_variables(operand),
        _ => false,
    }
}

/// The C runtime routine that converts between a float and a 128-bit
/// integer, for a cast that needs one.
fn wide_float_conversion(from: &Ty, to: &Ty) -> Option<&'static str> {
    use crate::sema::ty::FloatTy::{F32, F64};
    Some(match (from, to) {
        (Ty::Int(IntTy::I128), Ty::Float(F64)) => "__floattidf",
        (Ty::Int(IntTy::U128), Ty::Float(F64)) => "__floatuntidf",
        (Ty::Int(IntTy::I128), Ty::Float(F32)) => "__floattisf",
        (Ty::Int(IntTy::U128), Ty::Float(F32)) => "__floatuntisf",
        (Ty::Float(F64), Ty::Int(IntTy::I128)) => "__fixdfti",
        (Ty::Float(F64), Ty::Int(IntTy::U128)) => "__fixunsdfti",
        (Ty::Float(F32), Ty::Int(IntTy::I128)) => "__fixsfti",
        (Ty::Float(F32), Ty::Int(IntTy::U128)) => "__fixunssfti",
        _ => return None,
    })
}
