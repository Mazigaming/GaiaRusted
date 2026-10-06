//! Expressions that are checked the same way wherever they appear:
//! literals, operators, borrows, casts, field and index access, and the
//! literal forms of tuples, arrays, structs and ranges.

use super::FnCtxt;
use crate::sema::defs::{Def, FnOwner, Resolution};
use crate::sema::infer::VarKind;
use crate::sema::thir::*;
use crate::sema::ty::{AdtId, FloatTy, IntTy, Mutability, TraitId, Ty};
use crate::syntax::ast::{self, BinOp, UnOp};
use crate::syntax::build::AstBuilder;
use crate::syntax::diagnostic::{bail, Result};
use crate::syntax::span::Span;

impl FnCtxt<'_, '_> {
    /// Check `expr`. `expected` is a hint that helps inference (the type of
    /// a literal, the element type of an empty `Vec`); it is not enforced.
    pub fn check_expr(&mut self, expr: &ast::Expr, expected: Option<&Ty>) -> Result<Expr> {
        use ast::ExprKind as E;
        let span = expr.span;
        let typed = |kind, ty| Ok(Expr { kind, ty, span });

        match &expr.kind {
            E::Int(value, suffix) => {
                let ty = self.number_literal_ty(suffix.as_deref(), expected, VarKind::Int, span)?;
                match ty {
                    Ty::Float(_) => typed(ExprKind::Float(*value as f64), ty),
                    _ => typed(ExprKind::Int(*value), ty),
                }
            }
            E::Float(value, suffix) => {
                let ty = self.number_literal_ty(suffix.as_deref(), expected, VarKind::Float, span)?;
                typed(ExprKind::Float(*value), ty)
            }
            E::Bool(value) => typed(ExprKind::Bool(*value), Ty::Bool),
            E::Char(value) => typed(ExprKind::Char(*value), Ty::Char),
            E::Byte(value) => typed(ExprKind::Int(*value as u128), Ty::U8),
            E::Str(text) => typed(ExprKind::Str(text.clone()), Ty::str_ref()),
            E::ByteStr(bytes) => {
                let array = Ty::array(Ty::U8, bytes.len() as u64);
                typed(ExprKind::ByteStr(bytes.clone()), Ty::shared_ref(array))
            }

            E::Path(path) => self.check_path_expr(path, expected, span),
            E::Call(callee, args) => self.check_call(callee, args, expected, span),
            E::MethodCall { receiver, method, turbofish, args } => {
                let receiver = self.check_expr(receiver, None)?;
                self.source_method_call = true;
                self.check_method_call(receiver, method, turbofish, MethodArgs::Unchecked(args), expected, span)
            }

            E::Unary(op, operand) => self.check_unary(*op, operand, expected, span),
            E::Binary(op, lhs, rhs) => self.check_binary(*op, lhs, rhs, span),
            E::Assign(target, value) if is_destructuring(target) => self.check_destructuring_assign(target, value, span),
            E::Assign(target, value) => {
                let mut target = self.check_expr(target, None)?;
                self.require_mutable_place(&mut target)?;
                let value = self.check_expr_coerce(value, &target.ty.clone())?;
                typed(ExprKind::Assign(Box::new(target), Box::new(value)), Ty::UNIT)
            }
            E::AssignOp(op, target, value) => self.check_assign_op(*op, target, value, span),
            E::Cast(operand, ty) => self.check_cast(operand, ty, span),
            E::AddrOf { mutable, expr: operand } => {
                let hint = expected.and_then(|ty| match self.infer.shallow(ty) {
                    Ty::Ref(pointee, _) => Some(*pointee),
                    _ => None,
                });
                let mut operand = self.check_expr(operand, hint.as_ref())?;
                if *mutable && operand.is_place() {
                    self.require_mutable_place(&mut operand)?;
                }
                let ty = Ty::Ref(Box::new(operand.ty.clone()), Mutability::from_bool(*mutable));
                typed(ExprKind::AddrOf(Mutability::from_bool(*mutable), Box::new(operand)), ty)
            }

            E::Field(base, name) => {
                let base = self.check_expr(base, None)?;
                self.check_field(base, &name.name, name.span)
            }
            E::TupleField(base, index) => {
                let base = self.check_expr(base, None)?;
                self.check_field(base, &index.to_string(), span)
            }
            E::Index(base, index) => self.check_index(base, index, span),

            E::Tuple(elements) => {
                let hints = match expected.map(|ty| self.infer.shallow(ty)) {
                    Some(Ty::Tuple(hints)) if hints.len() == elements.len() => Some(hints),
                    _ => None,
                };
                let mut checked = Vec::new();
                for (i, element) in elements.iter().enumerate() {
                    checked.push(self.check_expr(element, hints.as_ref().map(|h| &h[i]))?);
                }
                let ty = Ty::Tuple(checked.iter().map(|e| e.ty.clone()).collect());
                typed(ExprKind::Tuple(checked), ty)
            }
            E::Array(elements) => {
                let element_ty = match expected.map(|ty| self.infer.shallow(ty)) {
                    Some(Ty::Array(element, _)) | Some(Ty::Slice(element)) => *element,
                    _ => self.infer.fresh_var(),
                };
                let mut checked = Vec::new();
                for element in elements {
                    checked.push(self.check_expr_coerce(element, &element_ty)?);
                }
                let ty = Ty::array(element_ty, checked.len() as u64);
                typed(ExprKind::Array(checked), ty)
            }
            E::Repeat(element, count) => {
                let count = match self.lower_const(count)? {
                    Ty::Const(count) if count >= 0 => count as u64,
                    Ty::Const(_) => bail!(span, "array length cannot be negative"),
                    _ => bail!(span, "the length of this array is not known"),
                };
                let element = self.check_expr(element, None)?;
                let ty = Ty::array(element.ty.clone(), count);
                typed(ExprKind::Repeat(Box::new(element), count), ty)
            }
            E::Struct { path, fields, base } => self.check_struct_literal(path, fields, base.as_deref(), expected, span),
            E::Range { lo, hi, inclusive } => self.check_range(lo.as_deref(), hi.as_deref(), *inclusive, span),

            E::Block(block) | E::Unsafe(block) => self.check_block_expr(block, expected),
            E::If { cond, then_block, else_expr } => {
                self.check_if(cond, then_block, else_expr.as_deref(), expected, span)
            }
            E::Let(..) => bail!(span, "`let` is only allowed as the condition of `if` or `while`"),
            E::Match { scrutinee, arms } => self.check_match(scrutinee, arms, expected, span),
            E::While { label, cond, body } => self.check_while(label, cond, body, span),
            E::Loop { label, body } => self.check_loop(label, body, expected, span),
            E::LabeledBlock { label, body } => self.check_labeled_block(label, body, expected, span),
            E::For { label, pat, iter, body } => self.check_for(label, pat, iter, body, span),
            E::Closure { params, ret, body, is_move } => {
                self.check_closure(params, ret.as_ref(), body, *is_move, expected, span)
            }
            E::Return(value) => self.check_return(value.as_deref(), span),
            E::Break { label, value } => self.check_break(label, value.as_deref(), span),
            E::Continue { label } => self.check_continue(label, span),
            E::Try(operand) => self.check_try(operand, expected, span),
            E::Macro(call) => self.check_macro(call, expected, span),
            E::Underscore => bail!(span, "`_` can only stand on the left of an assignment"),
        }
    }

    /// `(a, b) = value` and the like, as Rust defines them: the left side
    /// read as a pattern that binds a fresh variable for each place, then
    /// each place assigned from its variable, in order.
    ///
    /// ```text
    /// { let (__assign0, __assign1) = value; a = __assign0; b = __assign1; }
    /// ```
    fn check_destructuring_assign(&mut self, target: &ast::Expr, value: &ast::Expr, span: Span) -> Result<Expr> {
        let b = AstBuilder::new(span);
        let mut places = Vec::new();
        let pat = assignee_pattern(&b, target, &mut places)?;
        let mut stmts = vec![ast::Stmt {
            kind: ast::StmtKind::Let { pat, ty: None, init: Some(value.clone()), else_block: None },
            span,
        }];
        for (place, name) in places {
            let assign = ast::Expr { kind: ast::ExprKind::Assign(Box::new(place), Box::new(b.var(&name))), span };
            stmts.push(b.stmt(assign));
        }
        self.check_expr(&b.block(stmts, None), None)
    }

    /// Check `expr` and convert it to `ty`, or report a mismatch.
    pub fn check_expr_coerce(&mut self, expr: &ast::Expr, ty: &Ty) -> Result<Expr> {
        let checked = self.check_expr(expr, Some(ty))?;
        self.coerce(checked, ty)
    }

    /// The type of a number literal: its suffix if it has one, else the
    /// expected type if that is numeric, else a fresh literal variable.
    fn number_literal_ty(
        &mut self,
        suffix: Option<&str>,
        expected: Option<&Ty>,
        kind: VarKind,
        span: Span,
    ) -> Result<Ty> {
        if let Some(suffix) = suffix {
            return match (IntTy::from_name(suffix), suffix) {
                (Some(int), _) => Ok(Ty::Int(int)),
                (None, "f32") => Ok(Ty::Float(FloatTy::F32)),
                (None, "f64") => Ok(Ty::Float(FloatTy::F64)),
                _ => bail!(span, "invalid suffix `{suffix}` for a number literal"),
            };
        }
        Ok(match expected.map(|ty| self.infer.shallow(ty)) {
            Some(ty @ Ty::Int(_)) if kind == VarKind::Int => ty,
            Some(ty @ Ty::Float(_)) if kind == VarKind::Float => ty,
            _ => self.infer.fresh(kind),
        })
    }

    // -- operators -------------------------------------------------------------------

    /// Is `ty` a number, as far as is known? (Literal variables count.)
    fn is_numeric(&self, ty: &Ty) -> bool {
        match self.infer.shallow(ty) {
            Ty::Int(_) | Ty::Float(_) => true,
            Ty::Infer(var) => matches!(self.infer.unbound_kind(var), Some(VarKind::Int | VarKind::Float)),
            _ => false,
        }
    }

    fn is_integer(&self, ty: &Ty) -> bool {
        match self.infer.shallow(ty) {
            Ty::Int(_) => true,
            Ty::Infer(var) => self.infer.unbound_kind(var) == Some(VarKind::Int),
            _ => false,
        }
    }

    fn check_unary(&mut self, op: UnOp, operand: &ast::Expr, expected: Option<&Ty>, span: Span) -> Result<Expr> {
        match op {
            UnOp::Deref => {
                let operand = self.check_expr(operand, None)?;
                match self.deref_once(operand.clone(), span)? {
                    Some(place) => Ok(place),
                    None => bail!(span, "type `{}` cannot be dereferenced", self.show(&operand.ty)),
                }
            }
            UnOp::Neg | UnOp::Not => {
                let operand = self.check_expr(operand, expected)?;
                let ty = self.structurally_resolve_literal_aware(&operand.ty, span)?;
                let builtin = match op {
                    UnOp::Neg => self.is_numeric(&ty),
                    _ => self.is_integer(&ty) || ty == Ty::Bool,
                };
                if builtin {
                    let kind = if op == UnOp::Neg { UnaryOp::Neg } else { UnaryOp::Not };
                    return Ok(Expr { kind: ExprKind::Unary(kind, Box::new(operand)), ty, span });
                }
                let method = if op == UnOp::Neg { "neg" } else { "not" };
                self.call_method_by_name(operand, method, Vec::new(), span)
            }
        }
    }

    /// Like [`structurally_resolve`](Self::structurally_resolve), except that
    /// an undetermined number literal is left as it is instead of defaulted:
    /// `1 + x` should take its type from `x`.
    fn structurally_resolve_literal_aware(&mut self, ty: &Ty, span: Span) -> Result<Ty> {
        if self.is_numeric(ty) {
            Ok(self.infer.shallow(ty))
        } else {
            self.structurally_resolve(ty, span)
        }
    }

    fn check_binary(&mut self, op: BinOp, lhs: &ast::Expr, rhs: &ast::Expr, span: Span) -> Result<Expr> {
        if matches!(op, BinOp::And | BinOp::Or) {
            let lhs = self.check_expr_coerce(lhs, &Ty::Bool)?;
            let rhs = self.check_expr_coerce(rhs, &Ty::Bool)?;
            return Ok(Expr { kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)), ty: Ty::Bool, span });
        }
        let lhs = self.check_expr(lhs, None)?;
        let is_shift = matches!(op, BinOp::Shl | BinOp::Shr);
        let rhs_hint = if is_shift { None } else { Some(lhs.ty.clone()) };
        let rhs = self.check_expr(rhs, rhs_hint.as_ref())?;
        self.binary_op(op, lhs, rhs, span)
    }

    /// Apply a binary operator to two checked operands: directly for
    /// primitives, through the operator's trait method otherwise.
    pub fn binary_op(&mut self, op: BinOp, mut lhs: Expr, mut rhs: Expr, span: Span) -> Result<Expr> {
        let mut lhs_ty = self.structurally_resolve_literal_aware(&lhs.ty, span)?;
        let mut rhs_ty = self.structurally_resolve_literal_aware(&rhs.ty, span)?;

        // Comparing two references compares what they point to.
        if op.is_comparison() {
            while let (Ty::Ref(l, _), Ty::Ref(r, _)) = (&lhs_ty, &rhs_ty) {
                let (l, r) = ((**l).clone(), (**r).clone());
                lhs = Expr { kind: ExprKind::Deref(Box::new(lhs)), ty: l.clone(), span };
                rhs = Expr { kind: ExprKind::Deref(Box::new(rhs)), ty: r.clone(), span };
                lhs_ty = self.structurally_resolve_literal_aware(&l, span)?;
                rhs_ty = self.structurally_resolve_literal_aware(&r, span)?;
            }
        }

        // The standard library implements arithmetic for references to
        // numbers (`&a + 1`, `x * 2` with `x: &i64`): compute on the pointees.
        if !op.is_comparison() {
            for (operand, ty) in [(&mut lhs, &mut lhs_ty), (&mut rhs, &mut rhs_ty)] {
                while let Ty::Ref(pointee, _) = &*ty {
                    if !self.is_numeric(pointee) {
                        break;
                    }
                    let pointee = self.infer.shallow(pointee);
                    let reference = std::mem::replace(operand, Expr { kind: ExprKind::ZeroSized, ty: Ty::UNIT, span });
                    *operand = Expr { kind: ExprKind::Deref(Box::new(reference)), ty: pointee.clone(), span };
                    *ty = pointee;
                }
            }
        }

        let primitive = |ty: &Ty| matches!(ty, Ty::Bool | Ty::Char | Ty::Ptr(..));
        let both_numeric = self.is_numeric(&lhs_ty) && self.is_numeric(&rhs_ty);
        let builtin = match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => both_numeric,
            BinOp::Shl | BinOp::Shr => self.is_integer(&lhs_ty) && self.is_integer(&rhs_ty),
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                (self.is_integer(&lhs_ty) && self.is_integer(&rhs_ty))
                    || (lhs_ty == Ty::Bool && rhs_ty == Ty::Bool)
            }
            _ => both_numeric || (primitive(&lhs_ty) && primitive(&rhs_ty)) || (lhs_ty.is_unit() && rhs_ty.is_unit()),
        };

        if builtin {
            if !matches!(op, BinOp::Shl | BinOp::Shr) {
                self.unify(&lhs_ty, &rhs_ty, rhs.span)?;
            }
            let ty = if op.is_comparison() { Ty::Bool } else { lhs.ty.clone() };
            return Ok(Expr { kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)), ty, span });
        }

        // Comparison traits take both operands by reference.
        if op.is_comparison() {
            let rhs_ref_ty = Ty::shared_ref(rhs.ty.clone());
            rhs = Expr { kind: ExprKind::AddrOf(Mutability::Not, Box::new(rhs)), ty: rhs_ref_ty, span };
        }
        self.call_method_by_name(lhs, operator_method(op), vec![rhs], span).map_err(|mut error| {
            error.message = format!(
                "cannot apply `{}` to `{}` and `{}`",
                op.symbol(),
                self.show(&lhs_ty),
                self.show(&rhs_ty)
            );
            error
        })
    }

    fn check_assign_op(&mut self, op: BinOp, target: &ast::Expr, value: &ast::Expr, span: Span) -> Result<Expr> {
        let mut target = self.check_expr(target, None)?;
        self.require_mutable_place(&mut target)?;
        let target_ty = self.structurally_resolve_literal_aware(&target.ty, span)?;

        let builtin = self.is_numeric(&target_ty) || target_ty == Ty::Bool;
        if builtin {
            let is_shift = matches!(op, BinOp::Shl | BinOp::Shr);
            let mut value = self.check_expr(value, Some(&target_ty))?;
            // `total += x` with `x: &i64` adds what `x` points to.
            while let Ty::Ref(pointee, _) = self.infer.shallow(&value.ty) {
                if !self.is_numeric(&pointee) {
                    break;
                }
                value = Expr { kind: ExprKind::Deref(Box::new(value)), ty: *pointee, span };
            }
            if !is_shift {
                value = self.coerce(value, &target_ty)?;
            }
            let kind = ExprKind::AssignOp(op, Box::new(target), Box::new(value));
            return Ok(Expr { kind, ty: Ty::UNIT, span });
        }
        let value = self.check_expr(value, None)?;
        let method = format!("{}_assign", operator_method(op));
        self.call_method_by_name(target, &method, vec![value], span)
    }

    fn check_cast(&mut self, operand: &ast::Expr, ty: &ast::Type, span: Span) -> Result<Expr> {
        let target = self.lower_ty(ty)?;
        let operand = self.check_expr(operand, None)?;
        // A cast does not decide the type of a number literal: in
        // `let n = 4; v[n as usize % 2]; f(n)`, `f` still does.
        let source = self.structurally_resolve_literal_aware(&operand.ty, span)?;
        if matches!(source, Ty::Infer(_)) {
            let integer = self.is_integer(&source);
            match &target {
                Ty::Int(_) | Ty::Float(_) => {}
                Ty::Ptr(..) if integer => {}
                // Only a `u8` converts to a `char`.
                Ty::Char if integer => self.unify(&source, &Ty::Int(IntTy::U8), span)?,
                _ => bail!(span, "invalid cast: `{}` as `{}`", self.show(&source), self.show(&target)),
            }
            return Ok(Expr { kind: ExprKind::Cast(Box::new(operand)), ty: target, span });
        }

        // A cast may also make a pointer's target unsized, as a coercion
        // does: `&[1, 2] as &[u8]`, `&x as &dyn Display`. Other pointer casts
        // keep their own rules, which infer nothing from the target.
        let unsizes = matches!((&source, &target), (Ty::Ref(..), Ty::Ref(..) | Ty::Ptr(..)) | (Ty::Ptr(..), Ty::Ptr(..)))
            && target.pointee().is_some_and(|pointee| self.tcx.is_unsized(&self.resolve(pointee)))
            && source.pointee().is_some_and(|pointee| !self.tcx.is_unsized(&self.resolve(pointee)));
        if unsizes {
            let snapshot = self.infer.snapshot();
            match self.coerce(operand.clone(), &target) {
                Ok(coerced) => return Ok(coerced),
                Err(_) => self.infer.rollback_to(snapshot),
            }
        }

        let number_like = |ty: &Ty| matches!(ty, Ty::Int(_) | Ty::Float(_) | Ty::Bool | Ty::Char);
        let thin_pointer = |ty: &Ty| match ty {
            Ty::Ptr(pointee, _) | Ty::Ref(pointee, _) => !self.tcx.is_unsized(pointee),
            Ty::FnPtr(..) | Ty::FnItem(..) => true,
            _ => false,
        };
        let allowed = match (&source, &target) {
            (_, Ty::Int(_) | Ty::Float(_)) if number_like(&source) => true,
            (Ty::Int(IntTy::U8), Ty::Char) => true,
            // An enum without fields converts to its discriminant.
            (Ty::Adt(adt, _), Ty::Int(_)) if self.tcx.is_enum(*adt) => {
                self.tcx.defs.adt(*adt).variants.iter().all(|v| v.fields.is_empty())
            }
            (Ty::Int(_), Ty::Ptr(..)) => true,
            (_, Ty::Int(_) | Ty::Ptr(..)) if thin_pointer(&source) => true,
            // A pointer-to-pointer cast keeps a fat pointer's extra word or
            // discards it, but cannot make one up.
            (Ty::Ptr(from, _) | Ty::Ref(from, _), Ty::Ptr(to, _)) => {
                self.tcx.is_unsized(from) || !self.tcx.is_unsized(to)
            }
            _ => self.infer.try_unify(&source, &target),
        };
        if !allowed {
            bail!(span, "invalid cast: `{}` as `{}`", self.show(&source), self.show(&target));
        }
        Ok(Expr { kind: ExprKind::Cast(Box::new(operand)), ty: target, span })
    }

    // -- places ------------------------------------------------------------------------

    /// One step of dereferencing: `*reference`, `*pointer`, or the target of
    /// a smart pointer's `Deref` impl. `None` if `expr` is not a pointer.
    pub fn deref_once(&mut self, expr: Expr, span: Span) -> Result<Option<Expr>> {
        match self.structurally_resolve(&expr.ty, span)? {
            Ty::Ref(pointee, _) | Ty::Ptr(pointee, _) => {
                Ok(Some(Expr { kind: ExprKind::Deref(Box::new(expr)), ty: *pointee, span }))
            }
            ty @ Ty::Adt(..) => self.overloaded_deref(expr, &ty, Mutability::Not, span),
            _ => Ok(None),
        }
    }

    /// `*Deref::deref(&expr)` (or the `DerefMut` equivalent), if the type has such an impl.
    fn overloaded_deref(
        &mut self,
        expr: Expr,
        ty: &Ty,
        mutability: Mutability,
        span: Span,
    ) -> Result<Option<Expr>> {
        let (trait_id, method) = match mutability {
            Mutability::Not => (self.tcx.lang.deref, "deref"),
            Mutability::Mut => (self.tcx.lang.deref_mut, "deref_mut"),
        };
        let Some(trait_id) = trait_id else { return Ok(None) };
        let Some((impl_id, substs)) = self.tcx.trait_impls_for(trait_id, ty, &mut self.infer)?.pop() else {
            return Ok(None);
        };
        let Some(&def) = self.tcx.defs.impl_def(impl_id).methods.get(method) else { return Ok(None) };

        let sig = self.tcx.fn_sig(def, &substs, &mut self.infer, &mut self.projections)?;
        let Some(target) = sig.ret.pointee().cloned() else {
            bail!(span, "`{method}` must return a reference");
        };
        let receiver_ty = Ty::Ref(Box::new(expr.ty.clone()), mutability);
        let receiver = Expr { kind: ExprKind::AddrOf(mutability, Box::new(expr)), ty: receiver_ty, span };
        let call = ExprKind::Call(Callee::Fn(Instance { def, substs }), vec![receiver]);
        let pointer = Expr { kind: call, ty: sig.ret, span };
        Ok(Some(Expr { kind: ExprKind::Deref(Box::new(pointer)), ty: target, span }))
    }

    /// Prepare a place for being written through: every smart-pointer
    /// dereference on the way to it must go through `DerefMut`, and every
    /// overloaded index through `IndexMut`.
    pub fn require_mutable_place(&mut self, place: &mut Expr) -> Result<()> {
        if !place.is_place() {
            bail!(place.span, "cannot assign to this expression: it is not a place in memory");
        }
        let span = place.span;
        match &mut place.kind {
            ExprKind::Field(base, _) | ExprKind::Index(base, _) => {
                if base.is_place() {
                    self.require_mutable_place(base)?;
                }
            }
            ExprKind::Deref(pointer) => {
                let ExprKind::Call(Callee::Fn(instance), args) = &mut pointer.kind else {
                    return Ok(());
                };
                let owner = self.tcx.defs.fn_def(instance.def).owner;
                let implemented = |trait_id: Option<TraitId>| {
                    matches!(owner, FnOwner::Impl(id)
                        if trait_id.is_some() && self.tcx.defs.impl_def(id).trait_id == trait_id)
                };
                if implemented(self.tcx.lang.index) {
                    let [receiver, index] = <[Expr; 2]>::try_from(std::mem::take(args))
                        .unwrap_or_else(|_| unreachable!("`Index::index` takes a receiver and an index"));
                    *place = self.index_mut(receiver, index, span)?;
                    return Ok(());
                }
                if !implemented(self.tcx.lang.deref) {
                    return Ok(());
                }
                let ExprKind::AddrOf(_, base) = std::mem::replace(&mut args[0].kind, ExprKind::ZeroSized)
                else {
                    unreachable!("an overloaded deref borrows its operand");
                };
                let mut base = *base;
                if base.is_place() {
                    self.require_mutable_place(&mut base)?;
                }
                let base_ty = self.resolve(&base.ty);
                match self.overloaded_deref(base, &base_ty, Mutability::Mut, span)? {
                    Some(mutable_place) => *place = mutable_place,
                    None => bail!(span, "cannot write through `{}`: it does not implement `DerefMut`", self.show(&base_ty)),
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// The place `receiver[index]` for writing, through `IndexMut`, given
    /// the receiver and index of the `Index::index` call that read it.
    fn index_mut(&mut self, receiver: Expr, index: Expr, span: Span) -> Result<Expr> {
        let mut base = match receiver.kind {
            ExprKind::AddrOf(_, base) => *base,
            _ => {
                let pointee = receiver.ty.pointee().cloned().unwrap_or_else(|| receiver.ty.clone());
                Expr { kind: ExprKind::Deref(Box::new(receiver)), ty: pointee, span }
            }
        };
        if base.is_place() {
            self.require_mutable_place(&mut base)?;
        }
        let base_ty = self.resolve(&base.ty);
        let pointer = match self.call_method_by_name(base, "index_mut", vec![index], span) {
            Ok(pointer) => pointer,
            Err(_) => bail!(span, "cannot assign through an index of `{}`: it does not implement `IndexMut`", self.show(&base_ty)),
        };
        let Some(target) = pointer.ty.pointee().cloned() else {
            bail!(span, "`index_mut` must return a reference");
        };
        Ok(Expr { kind: ExprKind::Deref(Box::new(pointer)), ty: target, span })
    }

    /// `base.name`, dereferencing `base` as often as needed to find the field.
    fn check_field(&mut self, mut base: Expr, name: &str, span: Span) -> Result<Expr> {
        let original = base.ty.clone();
        // A private field is passed over, as rustc does: one further in may
        // be visible. If none is, it was the field meant.
        let mut private = None;
        loop {
            match self.structurally_resolve(&base.ty, span)? {
                Ty::Adt(adt, args) if !self.tcx.is_enum(adt) => {
                    if let Some(index) = self.tcx.defs.adt(adt).variants[0].field_index(name) {
                        if self.field_visible(adt, 0, index) {
                            let ty = self.tcx.variant_fields(adt, &args, 0, &mut self.infer)?.swap_remove(index);
                            return Ok(Expr { kind: ExprKind::Field(Box::new(base), index), ty, span });
                        }
                        private.get_or_insert((adt, index));
                    }
                }
                Ty::Tuple(elements) => {
                    if let Some(ty) = name.parse::<usize>().ok().and_then(|i| elements.get(i).map(|ty| (i, ty))) {
                        let (index, ty) = (ty.0, ty.1.clone());
                        return Ok(Expr { kind: ExprKind::Field(Box::new(base), index), ty, span });
                    }
                }
                _ => {}
            }
            match self.deref_once(base, span)? {
                Some(inner) => base = inner,
                None => match private {
                    Some((adt, index)) => return Err(self.private_field(adt, 0, index, span)),
                    None => bail!(span, "no field `{name}` on type `{}`", self.show(&original)),
                },
            }
        }
    }

    fn check_index(&mut self, base: &ast::Expr, index: &ast::Expr, span: Span) -> Result<Expr> {
        let mut base = self.check_expr(base, None)?;
        let index = self.check_expr(index, None)?;
        let original = base.ty.clone();

        if !self.is_integer(&index.ty) {
            // Slicing (`v[1..3]`) and keyed lookup (`map[&key]`) go through `Index::index`.
            let pointer = self.call_method_by_name(base, "index", vec![index], span)?;
            let Some(target) = pointer.ty.pointee().cloned() else {
                bail!(span, "`index` must return a reference");
            };
            return Ok(Expr { kind: ExprKind::Deref(Box::new(pointer)), ty: target, span });
        }
        let index = self.coerce(index, &Ty::USIZE)?;
        loop {
            let ty = self.structurally_resolve(&base.ty, span)?;
            if let Ty::Array(element, _) | Ty::Slice(element) = ty {
                let kind = ExprKind::Index(Box::new(base), Box::new(index));
                return Ok(Expr { kind, ty: *element, span });
            }
            // A collection that is not a slice underneath (`deque[0]`).
            if self.indexes_by_position(&ty)? {
                let pointer = self.call_method_by_name(base, "index", vec![index], span)?;
                let Some(target) = pointer.ty.pointee().cloned() else {
                    bail!(span, "`index` must return a reference");
                };
                return Ok(Expr { kind: ExprKind::Deref(Box::new(pointer)), ty: target, span });
            }
            base = match self.deref_once(base, span)? {
                Some(inner) => inner,
                None => bail!(span, "cannot index into a value of type `{}`", self.show(&original)),
            };
        }
    }

    /// Does `ty` implement `Index<usize>`?
    fn indexes_by_position(&mut self, ty: &Ty) -> Result<bool> {
        let Some(index_trait) = self.tcx.lang.index else { return Ok(false) };
        let impls: Vec<_> =
            self.tcx.trait_impls_for(index_trait, ty, &mut self.infer)?.into_iter().map(|(id, _)| id).collect();
        Ok(!impls.is_empty() && !self.impls_with_trait_args(ty, &impls, &[Ty::USIZE])?.is_empty())
    }

    // -- aggregate literals -----------------------------------------------------------------

    /// Which struct or enum variant a path names when used as a constructor.
    pub fn resolve_constructor(&mut self, path: &ast::Path) -> Result<Option<(AdtId, Vec<Ty>, u32)>> {
        if path.qself.is_some() {
            return Ok(None);
        }
        self.tcx.defs.check_visible(self.module, path)?;
        let first = path.segments[0].ident.name.as_str();
        if first == "Self" {
            // `Self::MAX` in `impl i32` names an associated item, not a constructor.
            let Some(Ty::Adt(adt, args)) = self.self_ty.clone() else {
                return Ok(None);
            };
            return Ok(match path.segments.as_slice() {
                [_] => Some((adt, args, 0)),
                [_, variant] => self.tcx.defs.adt(adt).variant_index(&variant.ident.name).map(|v| (adt, args, v)),
                _ => None,
            });
        }
        let Some(Resolution { def, rest: [] }) = self.tcx.defs.resolve_path(self.module, path) else {
            return Ok(None);
        };
        let (adt, variant, type_segment) = match def {
            Def::Adt(adt) if !self.tcx.is_enum(adt) => (adt, 0, path.segments.len() - 1),
            Def::Variant(adt, variant) => (adt, variant, path.segments.len().saturating_sub(2)),
            Def::Alias(_) => {
                let scope = crate::sema::context::TypeScope {
                    module: self.module,
                    generics: &self.generics,
                    self_ty: self.self_ty.as_ref(),
                    self_trait: self.self_trait.as_ref(),
                };
                let ty = self.tcx.lower_path_ty(scope, path, &mut self.infer, &mut self.projections, true)?;
                return Ok(match ty {
                    Ty::Adt(adt, args) if !self.tcx.is_enum(adt) => Some((adt, args, 0)),
                    _ => None,
                });
            }
            _ => return Ok(None),
        };

        // Explicit type arguments may sit on the type (`Option::<i64>::None`)
        // or on the variant (`None::<i64>`); otherwise they are inferred.
        let explicit = [&path.segments[type_segment], path.last()]
            .into_iter()
            .find(|segment| !segment.args.is_empty());
        let arity = self.tcx.defs.adt(adt).generics.params.len();
        let args: Vec<Ty> = match explicit {
            Some(segment) if segment.args.len() == arity => {
                segment.args.iter().map(|arg| self.lower_ty(arg)).collect::<Result<_>>()?
            }
            Some(segment) => bail!(segment.ident.span, "expected {arity} type argument(s)"),
            None => (0..arity).map(|_| self.infer.fresh_var()).collect(),
        };
        self.register_adt_bounds(adt, &args)?;
        Ok(Some((adt, args, variant)))
    }

    /// A struct declared `struct Counter<F: Fn(u32) -> u32>` gives a closure
    /// stored in it its signature.
    fn register_adt_bounds(&mut self, adt: AdtId, args: &[Ty]) -> Result<()> {
        let def = self.tcx.defs.adt(adt);
        let env = self.tcx.adt_env(adt, args);
        let scope = crate::sema::context::TypeScope { module: def.module, generics: &env, self_ty: None, self_trait: None };
        for (param, subject) in def.generics.params.iter().zip(args) {
            for (params, ret) in param.bounds.iter().filter_map(|bound| bound.fn_sugar.as_ref()) {
                let mut lowered = Vec::new();
                for ty in params {
                    lowered.push(self.tcx.lower_ty(scope, ty, &mut self.infer, &mut self.projections)?);
                }
                let ret = match ret {
                    Some(ret) => self.tcx.lower_ty(scope, ret, &mut self.infer, &mut self.projections)?,
                    None => Ty::UNIT,
                };
                self.add_callable_bound(subject.clone(), lowered, ret);
            }
        }
        Ok(())
    }

    fn check_struct_literal(
        &mut self,
        path: &ast::Path,
        fields: &[ast::FieldInit],
        base: Option<&ast::Expr>,
        expected: Option<&Ty>,
        span: Span,
    ) -> Result<Expr> {
        let Some((adt, args, variant)) = self.resolve_constructor(path)? else {
            bail!(path.span, "cannot find struct or variant `{}`", crate::sema::context::path_text(path));
        };
        let ty = Ty::Adt(adt, args.clone());
        if let Some(expected) = expected {
            // Lets the expected type fill in type arguments before the fields are checked.
            self.infer.try_unify(&ty, expected);
        }

        let field_tys = self.tcx.variant_fields(adt, &args, variant, &mut self.infer)?;
        let variant_def = &self.tcx.defs.adt(adt).variants[variant as usize];
        let mut inits: Vec<(usize, Expr)> = Vec::new();
        for field in fields {
            let Some(index) = variant_def.field_index(&field.name.name) else {
                bail!(field.name.span, "`{}` has no field named `{}`", variant_def.name, field.name.name);
            };
            if inits.iter().any(|(existing, _)| *existing == index) {
                bail!(field.name.span, "field `{}` is specified more than once", field.name.name);
            }
            if !self.field_visible(adt, variant, index) {
                return Err(self.private_field(adt, variant, index, field.name.span));
            }
            inits.push((index, self.check_expr_coerce(&field.value, &field_tys[index])?));
        }

        let base = match base {
            Some(base) => {
                // `..base` supplies the fields not listed, which must be visible too.
                let hidden = (0..variant_def.fields.len())
                    .find(|&index| !inits.iter().any(|(listed, _)| *listed == index) && !self.field_visible(adt, variant, index));
                if let Some(index) = hidden {
                    return Err(self.private_field(adt, variant, index, base.span));
                }
                Some(Box::new(self.check_expr_coerce(base, &ty)?))
            }
            None => {
                let missing = variant_def.fields.iter().enumerate().find(|(i, _)| !inits.iter().any(|(j, _)| j == i));
                if let Some((_, field)) = missing {
                    bail!(span, "missing field `{}` in initializer of `{}`", field.name, variant_def.name);
                }
                None
            }
        };
        Ok(Expr { kind: ExprKind::Adt { variant, fields: inits, base }, ty, span })
    }

    /// `a..b` and friends build the standard library's range structs.
    fn check_range(&mut self, lo: Option<&ast::Expr>, hi: Option<&ast::Expr>, inclusive: bool, span: Span) -> Result<Expr> {
        let lang = &self.tcx.lang;
        let adt = match (lo.is_some(), hi.is_some(), inclusive) {
            (true, true, false) => lang.range,
            (true, true, true) => lang.range_inclusive,
            (true, false, _) => lang.range_from,
            (false, true, false) => lang.range_to,
            (false, false, _) => lang.range_full,
            (false, true, true) => lang.range_to_inclusive,
        };
        let Some(adt) = adt else { bail!(span, "the standard library's range types are missing") };

        let mut fields = Vec::new();
        let mut bound_ty = None;
        for (index, bound) in lo.into_iter().chain(hi).enumerate() {
            let checked = match &bound_ty {
                Some(ty) => self.check_expr_coerce(bound, ty)?,
                None => self.check_expr(bound, None)?,
            };
            bound_ty.get_or_insert_with(|| checked.ty.clone());
            fields.push((index, checked));
        }
        let ty = Ty::Adt(adt, bound_ty.into_iter().collect());
        Ok(Expr { kind: ExprKind::Adt { variant: 0, fields, base: None }, ty, span })
    }
}

/// The trait method behind an overloadable operator.
fn operator_method(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "add",
        BinOp::Sub => "sub",
        BinOp::Mul => "mul",
        BinOp::Div => "div",
        BinOp::Rem => "rem",
        BinOp::BitAnd => "bitand",
        BinOp::BitOr => "bitor",
        BinOp::BitXor => "bitxor",
        BinOp::Shl => "shl",
        BinOp::Shr => "shr",
        BinOp::Eq => "eq",
        BinOp::Ne => "ne",
        BinOp::Lt => "lt",
        BinOp::Le => "le",
        BinOp::Gt => "gt",
        BinOp::Ge => "ge",
        BinOp::And | BinOp::Or => unreachable!("`&&` and `||` cannot be overloaded"),
    }
}

/// How the arguments of a method call are supplied.
pub enum MethodArgs<'e> {
    /// Source expressions, still to be checked against the parameter types.
    Unchecked(&'e [ast::Expr]),
    /// Already checked (operands of an overloaded operator, desugarings).
    Checked(Vec<Expr>),
}

/// Is the left side of this assignment a pattern of places rather than
/// one place: a tuple, array, struct literal or `_`?
fn is_destructuring(target: &ast::Expr) -> bool {
    use ast::ExprKind as E;
    match &target.kind {
        E::Tuple(_) | E::Array(_) | E::Struct { .. } | E::Underscore => true,
        // `Pair(a, b) = pair` names a tuple struct; a call is no place anyway.
        E::Call(callee, _) => matches!(callee.kind, E::Path(_)),
        _ => false,
    }
}

/// The pattern the left side of a destructuring assignment stands for.
/// Each place in it becomes a variable named `__assignN`, listed with the
/// place in `places`.
fn assignee_pattern(b: &AstBuilder, target: &ast::Expr, places: &mut Vec<(ast::Expr, String)>) -> Result<ast::Pattern> {
    use ast::{ExprKind as E, PatternKind as P};
    let each = |elements: &[ast::Expr], places: &mut Vec<(ast::Expr, String)>| {
        elements.iter().map(|element| assignee_pattern(b, element, places)).collect::<Result<Vec<_>>>()
    };
    let kind = match &target.kind {
        E::Underscore => P::Wild,
        E::Range { lo: None, hi: None, inclusive: false } => P::Rest,
        E::Tuple(elements) => P::Tuple(each(elements, places)?),
        E::Array(elements) => P::Slice(each(elements, places)?),
        E::Call(callee, elements) => match &callee.kind {
            E::Path(path) => P::TupleStruct(path.clone(), each(elements, places)?),
            _ => bail!(target.span, "invalid left-hand side of an assignment"),
        },
        E::Struct { path, fields, base } => {
            let has_rest = match base.as_deref() {
                None => false,
                Some(ast::Expr { kind: E::Range { lo: None, hi: None, inclusive: false }, .. }) => true,
                Some(base) => bail!(base.span, "functional update syntax cannot be assigned to"),
            };
            let mut patterns = Vec::new();
            for field in fields {
                patterns.push((field.name.clone(), assignee_pattern(b, &field.value, places)?));
            }
            P::Struct { path: path.clone(), fields: patterns, has_rest }
        }
        _ => {
            let name = format!("__assign{}", places.len());
            places.push((target.clone(), name.clone()));
            P::Binding { name: b.ident(&name), mutable: false, by_ref: None, sub: None }
        }
    };
    Ok(ast::Pattern { kind, span: target.span })
}
