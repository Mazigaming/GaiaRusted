//! Control flow: `if`, `match`, loops, closures, `return`, `break`, `?`.
//!
//! `while` and `for` have no counterpart in the typed tree; they are
//! rewritten here into `loop` with an `if` or `match` inside.

use super::{CaptureFrame, FnCtxt, LoopFrame, PendingClosure};
use crate::sema::infer::VarKind;
use crate::sema::thir::*;
use crate::sema::ty::Ty;
use crate::syntax::ast::{self, BinOp, PatternKind};
use crate::syntax::diagnostic::{bail, Result};
use crate::syntax::span::Span;

impl FnCtxt<'_, '_> {
    // -- small builders for generated code ---------------------------------------

    pub(super) fn let_local(&self, local: LocalId, init: Expr) -> Stmt {
        let pat = Pat {
            kind: PatKind::Binding { local, mode: BindingMode::Value, sub: None },
            ty: init.ty.clone(),
            span: init.span,
        };
        Stmt::Let { pat, init: Some(init), else_block: None }
    }

    pub(super) fn block_expr(&self, stmts: Vec<Stmt>, tail: Option<Expr>, ty: Ty, span: Span) -> Expr {
        Expr { kind: ExprKind::Block(Block { stmts, expr: tail.map(Box::new) }), ty, span }
    }

    // -- if and match -----------------------------------------------------------------

    /// A condition: a `bool`, a `let PATTERN = value`, or several joined by `&&`.
    /// Variables bound by `let` stay in the current scope for the body to use.
    fn check_condition(&mut self, cond: &ast::Expr) -> Result<Expr> {
        match &cond.kind {
            ast::ExprKind::Let(pat, value) => {
                let value = self.check_expr(value, None)?;
                let pat = self.check_pat(pat, &value.ty.clone(), BindingMode::Value)?;
                Ok(Expr { kind: ExprKind::Let(Box::new(pat), Box::new(value)), ty: Ty::Bool, span: cond.span })
            }
            ast::ExprKind::Binary(BinOp::And, lhs, rhs) => {
                let lhs = self.check_condition(lhs)?;
                let rhs = self.check_condition(rhs)?;
                let kind = ExprKind::Binary(BinOp::And, Box::new(lhs), Box::new(rhs));
                Ok(Expr { kind, ty: Ty::Bool, span: cond.span })
            }
            _ => self.check_expr_coerce(cond, &Ty::Bool),
        }
    }

    pub fn check_if(
        &mut self,
        cond: &ast::Expr,
        then_block: &ast::Block,
        else_expr: Option<&ast::Expr>,
        expected: Option<&Ty>,
        span: Span,
    ) -> Result<Expr> {
        let (cond, then_expr) = self.in_scope(|fcx| {
            let cond = fcx.check_condition(cond)?;
            Ok((cond, fcx.check_block_expr(then_block, expected)?))
        })?;

        let Some(else_expr) = else_expr else {
            let then_expr = self.coerce(then_expr, &Ty::UNIT)?;
            let kind = ExprKind::If(Box::new(cond), Box::new(then_expr), None);
            return Ok(Expr { kind, ty: Ty::UNIT, span });
        };
        let mut ty = None;
        let then_expr = self.unify_branch(&mut ty, then_expr, expected)?;
        let else_expr = self.check_expr(else_expr, ty.as_ref().or(expected))?;
        let else_expr = self.unify_branch(&mut ty, else_expr, expected)?;
        let kind = ExprKind::If(Box::new(cond), Box::new(then_expr), Some(Box::new(else_expr)));
        Ok(Expr { kind, ty: ty.unwrap_or(Ty::Never), span })
    }

    pub fn check_match(
        &mut self,
        scrutinee: &ast::Expr,
        arms: &[ast::Arm],
        expected: Option<&Ty>,
        span: Span,
    ) -> Result<Expr> {
        let scrutinee = self.check_expr(scrutinee, None)?;
        let scrutinee_ty = scrutinee.ty.clone();
        let mut ty = None;
        let mut checked = Vec::new();
        for arm in arms {
            let arm = self.in_scope(|fcx| {
                let pat = fcx.check_pat(&arm.pat, &scrutinee_ty, BindingMode::Value)?;
                let guard = match &arm.guard {
                    Some(guard) => Some(fcx.check_expr_coerce(guard, &Ty::Bool)?),
                    None => None,
                };
                let body = fcx.check_expr(&arm.body, ty.as_ref().or(expected))?;
                let body = fcx.unify_branch(&mut ty, body, expected)?;
                Ok(Arm { pat, guard, body })
            })?;
            checked.push(arm);
        }
        let kind = ExprKind::Match(Box::new(scrutinee), checked);
        Ok(Expr { kind, ty: ty.unwrap_or(Ty::Never), span })
    }

    // -- loops ------------------------------------------------------------------------

    fn enter_loop(&mut self, label: &Option<String>, break_ty: Option<Ty>) -> LoopId {
        let id = LoopId(self.next_loop);
        self.next_loop += 1;
        self.loops.push(LoopFrame { id, label: label.clone(), break_ty, has_break: false, is_block: false });
        id
    }

    /// The loop a `break` or `continue` refers to.
    fn target_loop(&mut self, label: &Option<String>, span: Span) -> Result<&mut LoopFrame> {
        let frame = match label {
            // An unlabeled `break` leaves the innermost loop, never a block.
            None => self.loops.iter_mut().rev().find(|frame| !frame.is_block),
            Some(_) => self.loops.iter_mut().rev().find(|frame| frame.label == *label),
        };
        match (frame, label) {
            (Some(frame), _) => Ok(frame),
            (None, Some(label)) => bail!(span, "no enclosing loop is labelled `'{label}`"),
            (None, None) => bail!(span, "`break` and `continue` are only allowed inside a loop"),
        }
    }

    pub fn check_loop(&mut self, label: &Option<String>, body: &ast::Block, expected: Option<&Ty>, span: Span) -> Result<Expr> {
        let break_ty = expected.cloned().unwrap_or_else(|| self.infer.fresh_var());
        let id = self.enter_loop(label, Some(break_ty.clone()));
        let body = self.check_block(body, None);
        let frame = self.loops.pop().expect("loop frame pushed above");
        let (body, _) = body?;
        // A `loop` without `break` never finishes.
        let ty = if frame.has_break { break_ty } else { Ty::Never };
        Ok(Expr { kind: ExprKind::Loop(id, body), ty, span })
    }

    /// `'label: { body }` is a loop that runs once: `loop { break 'label body }`,
    /// so that `break 'label value` inside can leave it early.
    pub fn check_labeled_block(&mut self, label: &str, body: &ast::Block, expected: Option<&Ty>, span: Span) -> Result<Expr> {
        let break_ty = expected.cloned().unwrap_or_else(|| self.infer.fresh_var());
        let id = self.enter_loop(&Some(label.to_string()), Some(break_ty.clone()));
        if let Some(frame) = self.loops.last_mut() {
            frame.is_block = true;
        }
        let value = self.check_block_expr(body, Some(&break_ty)).and_then(|value| self.coerce(value, &break_ty));
        self.loops.pop();
        let finish = Expr { kind: ExprKind::Break(id, Some(Box::new(value?))), ty: Ty::Never, span };
        let kind = ExprKind::Loop(id, Block { stmts: Vec::new(), expr: Some(Box::new(finish)) });
        Ok(Expr { kind, ty: break_ty, span })
    }

    /// `while cond { body }` is `loop { if cond { body } else { break } }`.
    pub fn check_while(&mut self, label: &Option<String>, cond: &ast::Expr, body: &ast::Block, span: Span) -> Result<Expr> {
        let id = self.enter_loop(label, None);
        let result = self.in_scope(|fcx| {
            let cond = fcx.check_condition(cond)?;
            let body = fcx.check_block_expr(body, None)?;
            Ok((cond, fcx.coerce(body, &Ty::UNIT)?))
        });
        self.loops.pop();
        let (cond, body) = result?;

        let exit = Expr { kind: ExprKind::Break(id, None), ty: Ty::Never, span };
        let step = Expr { kind: ExprKind::If(Box::new(cond), Box::new(body), Some(Box::new(exit))), ty: Ty::UNIT, span };
        let kind = ExprKind::Loop(id, Block { stmts: Vec::new(), expr: Some(Box::new(step)) });
        Ok(Expr { kind, ty: Ty::UNIT, span })
    }

    pub fn check_for(
        &mut self,
        label: &Option<String>,
        pat: &ast::Pattern,
        iter: &ast::Expr,
        body: &ast::Block,
        span: Span,
    ) -> Result<Expr> {
        // Counting over an integer range needs no iterator object at all.
        if let ast::ExprKind::Range { lo: Some(lo), hi: Some(hi), inclusive } = &iter.kind {
            return self.check_counted_for(label, pat, lo, hi, *inclusive, body, span);
        }

        let iterable = self.check_expr(iter, None)?;
        let iterable_ty = self.structurally_resolve(&iterable.ty, iter.span)?;
        if let (Ty::Array(element, _), Some(len)) = (&iterable_ty, iterable_ty.array_len()) {
            if self.is_plain_copy(element) {
                return self.check_array_for(label, pat, iterable, (**element).clone(), len, body, span);
            }
        }
        let iterator = self.call_method_by_name(iterable, "into_iter", Vec::new(), iter.span)?;
        let iterator_local = self.new_temp(iterator.ty.clone());
        let bind_iterator = self.let_local(iterator_local, iterator);

        let next = self.call_method_by_name(self.local_expr(iterator_local, iter.span), "next", Vec::new(), iter.span)?;
        let Some(option) = self.tcx.lang.option else { bail!(span, "the standard library's `Option` is missing") };
        let item_ty = self.infer.fresh_var();
        self.unify(&Ty::Adt(option, vec![item_ty.clone()]), &next.ty, iter.span)?;
        let some = self.tcx.defs.adt(option).variant_index("Some").expect("`Option` has `Some`");

        let id = self.enter_loop(label, None);
        let result = self.in_scope(|fcx| {
            let pat = fcx.check_pat(pat, &item_ty, BindingMode::Value)?;
            let body = fcx.check_block_expr(body, None)?;
            Ok((pat, fcx.coerce(body, &Ty::UNIT)?))
        });
        self.loops.pop();
        let (pat, body) = result?;

        // loop { match iterator.next() { Some(pat) => body, _ => break } }
        let some_pat = Pat { kind: PatKind::Variant { variant: some, fields: vec![(0, pat)] }, ty: next.ty.clone(), span };
        let exit = Expr { kind: ExprKind::Break(id, None), ty: Ty::Never, span };
        let arms = vec![
            Arm { pat: some_pat, guard: None, body },
            Arm { pat: Pat { kind: PatKind::Wild, ty: next.ty.clone(), span }, guard: None, body: exit },
        ];
        let step = Expr { kind: ExprKind::Match(Box::new(next), arms), ty: Ty::UNIT, span };
        let looping = Expr {
            kind: ExprKind::Loop(id, Block { stmts: Vec::new(), expr: Some(Box::new(step)) }),
            ty: Ty::UNIT,
            span,
        };
        Ok(self.block_expr(vec![bind_iterator], Some(looping), Ty::UNIT, span))
    }

    /// Is `ty` known to be copied bitwise, number literals included?
    fn is_plain_copy(&self, ty: &Ty) -> bool {
        match self.resolve(ty) {
            Ty::Infer(var) => matches!(self.infer.unbound_kind(var), Some(VarKind::Int | VarKind::Float)),
            resolved => !resolved.has_infer() && self.tcx.is_copy(&resolved),
        }
    }

    /// `for x in array { body }` over `Copy` elements visits them by index:
    ///
    /// ```text
    /// let array = iterable; let mut index = 0;
    /// loop { if index < LEN { let x = array[index]; index += 1; body } else { break } }
    /// ```
    ///
    /// Elements that own something go through the array's `IntoIterator`,
    /// which drops those the loop does not reach.
    #[allow(clippy::too_many_arguments)]
    fn check_array_for(
        &mut self,
        label: &Option<String>,
        pat: &ast::Pattern,
        iterable: Expr,
        element: Ty,
        len: u64,
        body: &ast::Block,
        span: Span,
    ) -> Result<Expr> {
        let array = self.new_temp(iterable.ty.clone());
        let index = self.new_temp(Ty::USIZE);

        let id = self.enter_loop(label, None);
        let result = self.in_scope(|fcx| {
            let pat = fcx.check_pat(pat, &element, BindingMode::Value)?;
            let body = fcx.check_block_expr(body, None)?;
            Ok((pat, fcx.coerce(body, &Ty::UNIT)?))
        });
        self.loops.pop();
        let (pat, body) = result?;

        let usize_literal = |value| Expr { kind: ExprKind::Int(value), ty: Ty::USIZE, span };
        let current = Expr {
            kind: ExprKind::Index(Box::new(self.local_expr(array, span)), Box::new(self.local_expr(index, span))),
            ty: element,
            span,
        };
        let advance = Expr {
            kind: ExprKind::AssignOp(BinOp::Add, Box::new(self.local_expr(index, span)), Box::new(usize_literal(1))),
            ty: Ty::UNIT,
            span,
        };
        let in_range = Expr {
            kind: ExprKind::Binary(BinOp::Lt, Box::new(self.local_expr(index, span)), Box::new(usize_literal(len as u128))),
            ty: Ty::Bool,
            span,
        };
        let bind_item = Stmt::Let { pat, init: Some(current), else_block: None };
        let iteration = self.block_expr(vec![bind_item, Stmt::Expr(advance)], Some(body), Ty::UNIT, span);
        let exit = Expr { kind: ExprKind::Break(id, None), ty: Ty::Never, span };
        let step = Expr { kind: ExprKind::If(Box::new(in_range), Box::new(iteration), Some(Box::new(exit))), ty: Ty::UNIT, span };
        let looping = Expr {
            kind: ExprKind::Loop(id, Block { stmts: Vec::new(), expr: Some(Box::new(step)) }),
            ty: Ty::UNIT,
            span,
        };
        let setup = vec![self.let_local(array, iterable), self.let_local(index, usize_literal(0))];
        Ok(self.block_expr(setup, Some(looping), Ty::UNIT, span))
    }

    /// `for i in lo..hi { body }` as a plain counting loop:
    ///
    /// ```text
    /// let mut counter = lo; let end = hi;
    /// loop { if counter < end { let i = counter; counter += 1; body } else { break } }
    /// ```
    ///
    /// The inclusive form must not step past `end` (which may be the largest
    /// value of the type), so it carries a "finished" flag instead.
    #[allow(clippy::too_many_arguments)]
    fn check_counted_for(
        &mut self,
        label: &Option<String>,
        pat: &ast::Pattern,
        lo: &ast::Expr,
        hi: &ast::Expr,
        inclusive: bool,
        body: &ast::Block,
        span: Span,
    ) -> Result<Expr> {
        let lo = self.check_expr(lo, None)?;
        let ty = lo.ty.clone();
        let hi = self.check_expr_coerce(hi, &ty)?;
        let counter = self.new_temp(ty.clone());
        let end = self.new_temp(ty.clone());
        let finished = self.new_temp(Ty::Bool);

        let id = self.enter_loop(label, None);
        let result = self.in_scope(|fcx| {
            let pat = fcx.check_pat(pat, &ty, BindingMode::Value)?;
            let body = fcx.check_block_expr(body, None)?;
            Ok((pat, fcx.coerce(body, &Ty::UNIT)?))
        });
        self.loops.pop();
        let (pat, body) = result?;

        let local = |fcx: &Self, id| fcx.local_expr(id, span);
        let compare = |op, a: Expr, b: Expr| Expr { kind: ExprKind::Binary(op, Box::new(a), Box::new(b)), ty: Ty::Bool, span };
        let one = Expr { kind: ExprKind::Int(1), ty: ty.clone(), span };
        let increment = Expr {
            kind: ExprKind::AssignOp(BinOp::Add, Box::new(local(self, counter)), Box::new(one)),
            ty: Ty::UNIT,
            span,
        };
        let bind_item = Stmt::Let { pat, init: Some(local(self, counter)), else_block: None };

        let (cond, advance) = if inclusive {
            // if !finished && counter <= end { ...; if counter == end { finished = true } else { counter += 1 } }
            let not_finished = Expr { kind: ExprKind::Unary(UnaryOp::Not, Box::new(local(self, finished))), ty: Ty::Bool, span };
            let in_range = compare(BinOp::Le, local(self, counter), local(self, end));
            let at_end = compare(BinOp::Eq, local(self, counter), local(self, end));
            let set_finished = Expr {
                kind: ExprKind::Assign(
                    Box::new(local(self, finished)),
                    Box::new(Expr { kind: ExprKind::Bool(true), ty: Ty::Bool, span }),
                ),
                ty: Ty::UNIT,
                span,
            };
            let advance = Expr {
                kind: ExprKind::If(Box::new(at_end), Box::new(set_finished), Some(Box::new(increment))),
                ty: Ty::UNIT,
                span,
            };
            (compare(BinOp::And, not_finished, in_range), advance)
        } else {
            (compare(BinOp::Lt, local(self, counter), local(self, end)), increment)
        };

        let iteration = self.block_expr(vec![bind_item, Stmt::Expr(advance)], Some(body), Ty::UNIT, span);
        let exit = Expr { kind: ExprKind::Break(id, None), ty: Ty::Never, span };
        let step = Expr { kind: ExprKind::If(Box::new(cond), Box::new(iteration), Some(Box::new(exit))), ty: Ty::UNIT, span };
        let looping = Expr {
            kind: ExprKind::Loop(id, Block { stmts: Vec::new(), expr: Some(Box::new(step)) }),
            ty: Ty::UNIT,
            span,
        };

        let setup = vec![
            self.let_local(counter, lo),
            self.let_local(end, hi),
            self.let_local(finished, Expr { kind: ExprKind::Bool(false), ty: Ty::Bool, span }),
        ];
        Ok(self.block_expr(setup, Some(looping), Ty::UNIT, span))
    }

    pub fn check_break(&mut self, label: &Option<String>, value: Option<&ast::Expr>, span: Span) -> Result<Expr> {
        let frame = self.target_loop(label, span)?;
        frame.has_break = true;
        let (id, break_ty) = (frame.id, frame.break_ty.clone());
        let value = match (value, break_ty) {
            (Some(value), Some(ty)) => Some(Box::new(self.check_expr_coerce(value, &ty)?)),
            (Some(value), None) => bail!(value.span, "only `loop` can `break` with a value"),
            (None, Some(ty)) => {
                self.unify(&ty, &Ty::UNIT, span)?;
                None
            }
            (None, None) => None,
        };
        Ok(Expr { kind: ExprKind::Break(id, value), ty: Ty::Never, span })
    }

    pub fn check_continue(&mut self, label: &Option<String>, span: Span) -> Result<Expr> {
        let frame = self.target_loop(label, span)?;
        if frame.is_block {
            bail!(span, "`continue` cannot go to a labeled block");
        }
        let id = frame.id;
        Ok(Expr { kind: ExprKind::Continue(id), ty: Ty::Never, span })
    }

    pub fn check_return(&mut self, value: Option<&ast::Expr>, span: Span) -> Result<Expr> {
        let ret_ty = self.ret_ty.clone();
        let value = match value {
            Some(value) => Some(Box::new(self.check_expr_coerce(value, &ret_ty)?)),
            None => {
                self.unify(&ret_ty, &Ty::UNIT, span)?;
                None
            }
        };
        Ok(Expr { kind: ExprKind::Return(value), ty: Ty::Never, span })
    }

    // -- closures ---------------------------------------------------------------------

    pub fn check_closure(
        &mut self,
        params: &[ast::ClosureParam],
        ret: Option<&ast::Type>,
        body: &ast::Expr,
        is_move: bool,
        expected: Option<&Ty>,
        span: Span,
    ) -> Result<Expr> {
        // What the context says the closure's signature should be: either an
        // `F: Fn(..)` bound on the parameter it is passed for, or a function
        // pointer type it is assigned to.
        let expected_sig = expected.and_then(|ty| match self.infer.shallow(ty) {
            Ty::FnPtr(params, ret) => Some((params, *ret)),
            other => self.expected_closure_sig(&other),
        });
        let expected_sig = expected_sig.filter(|(expected_params, _)| expected_params.len() == params.len());

        let id = self.tcx.fresh_closure_id();
        self.capture_frames.push(CaptureFrame { first_own_local: self.locals.len() as u32, captured: Vec::new() });
        let outer_loops = std::mem::take(&mut self.loops);
        let outer_ret = self.ret_ty.clone();

        let result = self.in_scope(|fcx| {
            let mut param_locals = Vec::new();
            let mut param_tys = Vec::new();
            let mut destructuring = Vec::new();
            for (index, param) in params.iter().enumerate() {
                let ty = match (&param.ty, &expected_sig) {
                    (Some(written), _) => fcx.lower_ty(written)?,
                    (None, Some((expected_params, _))) => expected_params[index].clone(),
                    (None, None) => fcx.infer.fresh_var(),
                };
                match &param.pat.kind {
                    PatternKind::Binding { name, mutable, by_ref: None, sub: None } => {
                        param_locals.push(fcx.declare_local(&name.name, ty.clone(), *mutable));
                    }
                    _ => {
                        let local = fcx.new_temp(ty.clone());
                        param_locals.push(local);
                        let pat = fcx.check_pat(&param.pat, &ty, BindingMode::Value)?;
                        let init = fcx.local_expr(local, param.pat.span);
                        destructuring.push(Stmt::Let { pat, init: Some(init), else_block: None });
                    }
                }
                param_tys.push(ty);
            }

            let ret_ty = match (ret, &expected_sig) {
                (Some(written), _) => fcx.lower_ty(written)?,
                (None, Some((_, expected_ret))) => expected_ret.clone(),
                (None, None) => fcx.infer.fresh_var(),
            };
            fcx.ret_ty = ret_ty.clone();
            let mut value = fcx.check_expr_coerce(body, &ret_ty)?;
            if !destructuring.is_empty() {
                let (ty, span) = (value.ty.clone(), value.span);
                value = fcx.block_expr(destructuring, Some(value), ty, span);
            }
            Ok((param_locals, param_tys, ret_ty, value))
        });

        self.ret_ty = outer_ret;
        self.loops = outer_loops;
        let frame = self.capture_frames.pop().expect("capture frame pushed above");
        let (param_locals, param_tys, ret_ty, value) = result?;

        self.tcx.set_provisional_closure_sig(id, param_tys, ret_ty.clone());
        self.pending_closures.push(PendingClosure {
            id,
            params: param_locals,
            ret_ty,
            value,
            captured: frame.captured,
            is_move,
            span,
        });
        Ok(Expr { kind: ExprKind::Closure(id), ty: Ty::Closure(id), span })
    }

    // -- the `?` operator -----------------------------------------------------------------

    /// `value?` unwraps `Ok` / `Some` and returns early on `Err` / `None`:
    ///
    /// ```text
    /// match value { Ok(v) => v, Err(e) => return Err(From::from(e)) }
    /// ```
    pub fn check_try(&mut self, operand: &ast::Expr, expected: Option<&Ty>, span: Span) -> Result<Expr> {
        let operand = self.check_expr(operand, None)?;
        let mut operand_ty = self.structurally_resolve(&operand.ty, span)?;
        // `let n: i64 = text.parse()?` says what is parsed, and so what the
        // error is, before the error is compared with the function's.
        if let (Some(expected), Ty::Adt(_, args)) = (expected, &operand_ty) {
            if self.infer.try_unify(&args[0], expected) {
                self.solve_pending()?;
                operand_ty = self.resolve(&operand_ty);
            }
        }
        let ret_ty = self.structurally_resolve(&self.ret_ty.clone(), span)?;
        let lang = &self.tcx.lang;

        let (Ty::Adt(adt, args), Ty::Adt(ret_adt, ret_args)) = (&operand_ty, &ret_ty) else {
            bail!(span, "`?` can only be applied to `Result` or `Option` in a function that returns the same");
        };
        if adt != ret_adt || (Some(*adt) != lang.result && Some(*adt) != lang.option) {
            bail!(span, "`?` on `{}` cannot be used in a function returning `{}`", self.show(&operand_ty), self.show(&ret_ty));
        }
        let variant = |name: &str| self.tcx.defs.adt(*adt).variant_index(name).expect("std variant exists");

        let value_local = self.new_temp(args[0].clone());
        let unwrapped = self.local_expr(value_local, span);
        let bind = |local, ty: &Ty| Pat {
            kind: PatKind::Binding { local, mode: BindingMode::Value, sub: None },
            ty: ty.clone(),
            span,
        };

        let (success, failure_pat, early_value) = if Some(*adt) == lang.result {
            let (ok, err) = (variant("Ok"), variant("Err"));
            // The error may need converting to the function's error type.
            let (error_ty, wanted_ty) = (args[1].clone(), ret_args[1].clone());
            let error_local = self.new_temp(error_ty.clone());
            let error = self.local_expr(error_local, span);
            let converted = if self.infer.try_unify(&error_ty, &wanted_ty) {
                error
            } else {
                self.convert_error(error, &wanted_ty, span)?
            };
            let early = Expr {
                kind: ExprKind::Adt { variant: err, fields: vec![(0, converted)], base: None },
                ty: ret_ty.clone(),
                span,
            };
            let failure_pat = PatKind::Variant { variant: err, fields: vec![(0, bind(error_local, &error_ty))] };
            (ok, failure_pat, early)
        } else {
            let early = Expr {
                kind: ExprKind::Adt { variant: variant("None"), fields: Vec::new(), base: None },
                ty: ret_ty.clone(),
                span,
            };
            (variant("Some"), PatKind::Wild, early)
        };

        let success_pat = PatKind::Variant { variant: success, fields: vec![(0, bind(value_local, &args[0]))] };
        let early_return = Expr { kind: ExprKind::Return(Some(Box::new(early_value))), ty: Ty::Never, span };
        let arms = vec![
            Arm { pat: Pat { kind: success_pat, ty: operand_ty.clone(), span }, guard: None, body: unwrapped },
            Arm { pat: Pat { kind: failure_pat, ty: operand_ty.clone(), span }, guard: None, body: early_return },
        ];
        Ok(Expr { kind: ExprKind::Match(Box::new(operand), arms), ty: args[0].clone(), span })
    }

    /// `From::from(error)` for the error type the function returns.
    fn convert_error(&mut self, error: Expr, wanted: &Ty, span: Span) -> Result<Expr> {
        let Some(crate::sema::defs::Def::Trait(from)) = self.tcx.defs.std_item(&["convert", "From"]) else {
            bail!(span, "mismatched error types: expected `{}`, found `{}`", self.show(wanted), self.show(&error.ty));
        };
        let method = self.tcx.defs.trait_def(from).methods["from"];
        let declared = Instance { def: method, substs: vec![wanted.clone(), error.ty.clone()] };
        let instance = self.select_trait_method(from, method, wanted, declared, &[], span)?;
        Ok(Expr { kind: ExprKind::Call(Callee::Fn(instance), vec![error]), ty: wanted.clone(), span })
    }
}
