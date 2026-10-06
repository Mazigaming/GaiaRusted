//! Patterns.
//!
//! A pattern is checked against the type of the value it will be matched
//! with, top-down. Matching a reference with a non-reference pattern looks
//! through the reference and makes the bindings references themselves
//! (`match &opt { Some(x) => ... }` gives `x: &T`), as in Rust.

use super::FnCtxt;
use crate::sema::const_eval::eval_int;
use crate::sema::defs::{Def, VariantShape};
use crate::sema::thir::*;
use crate::sema::ty::{AdtId, Mutability, Ty};
use crate::syntax::ast::{self, PatternKind as P};
use crate::syntax::diagnostic::{bail, Result};
use crate::syntax::span::Span;
use std::collections::HashMap;

impl FnCtxt<'_, '_> {
    /// Check `pat` against a value of type `expected`. `mode` is how plain
    /// bindings bind: by value, or by reference once the pattern has looked
    /// through a reference.
    pub fn check_pat(&mut self, pat: &ast::Pattern, expected: &Ty, mode: BindingMode) -> Result<Pat> {
        let span = pat.span;

        // Looking through references. Patterns that can match a reference as
        // such (bindings, `_`, `&p`) are left alone.
        let sees_through = !matches!(pat.kind, P::Wild | P::Binding { .. } | P::Ref(_) | P::Or(_) | P::Rest)
            || self.names_a_constant_or_variant(pat);
        // A string literal pattern is itself a `&str`.
        let is_str_literal = matches!(&pat.kind, P::Literal(literal) if matches!(literal.kind, ast::ExprKind::Str(_)));
        if sees_through {
            if let Ty::Ref(pointee, mutability) = self.infer.shallow(expected) {
                if is_str_literal && self.infer.shallow(&pointee) == Ty::Str {
                    let literal = self.check_literal_pattern_of(pat, expected)?;
                    return Ok(Pat { kind: PatKind::Literal(literal), ty: expected.clone(), span });
                }
                let inner_mode = match (mode, mutability) {
                    (BindingMode::Ref(Mutability::Not), _) | (_, Mutability::Not) => {
                        BindingMode::Ref(Mutability::Not)
                    }
                    _ => BindingMode::Ref(Mutability::Mut),
                };
                let inner = self.check_pat(pat, &pointee, inner_mode)?;
                return Ok(Pat { kind: PatKind::Deref(Box::new(inner)), ty: expected.clone(), span });
            }
        }

        let kind = match &pat.kind {
            P::Wild => PatKind::Wild,
            P::Rest => bail!(span, "`..` is only allowed inside a tuple, tuple struct or slice pattern"),
            P::Binding { name, mutable, by_ref, sub } => {
                if by_ref.is_none() && !mutable && sub.is_none() {
                    if let Some(kind) = self.check_named_pattern(name, expected)? {
                        return Ok(Pat { kind, ty: expected.clone(), span });
                    }
                }
                let mode = match by_ref {
                    Some(mutable) => BindingMode::Ref(Mutability::from_bool(*mutable)),
                    None => mode,
                };
                let local_ty = match mode {
                    BindingMode::Value => expected.clone(),
                    BindingMode::Ref(mutability) => Ty::Ref(Box::new(expected.clone()), mutability),
                };
                let local = self.bind(name, local_ty, *mutable)?;
                let sub = match sub {
                    Some(sub) => Some(Box::new(self.check_pat(sub, expected, mode)?)),
                    None => None,
                };
                PatKind::Binding { local, mode, sub }
            }
            P::Literal(literal) => PatKind::Literal(self.check_literal_pattern(literal, expected)?),
            P::Range { lo, hi, inclusive } => {
                let mut bound = |bound: &Option<Box<ast::Expr>>| -> Result<Option<i128>> {
                    let Some(bound) = bound else { return Ok(None) };
                    self.check_expr_coerce(bound, expected)?;
                    Ok(Some(eval_int(self.tcx, self.module, bound)?))
                };
                PatKind::Range { lo: bound(lo)?, hi: bound(hi)?, inclusive: *inclusive }
            }
            P::Path(path) => {
                let Some((adt, args, variant)) = self.resolve_constructor(path)? else {
                    // Not a variant: a constant such as `limits::MAX`.
                    let value = eval_int(self.tcx, self.module, &path_expr(path))?;
                    return Ok(Pat { kind: PatKind::Literal(Literal::Int(value)), ty: expected.clone(), span });
                };
                self.variant_pattern(adt, args, variant, Vec::new(), expected, span)?
            }
            P::TupleStruct(path, elements) => {
                let Some((adt, args, variant)) = self.resolve_constructor(path)? else {
                    bail!(path.span, "cannot find tuple struct or variant `{}`", crate::sema::context::path_text(path));
                };
                let field_count = self.tcx.defs.adt(adt).variants[variant as usize].fields.len();
                if (0..field_count).any(|index| !self.field_visible(adt, variant, index)) {
                    bail!(span, "cannot match against a tuple struct which contains private fields");
                }
                let field_tys = self.tcx.variant_fields(adt, &args, variant, &mut self.infer)?;
                self.unify(expected, &Ty::Adt(adt, args.clone()), span)?;
                let fields = self.check_positional(elements, &field_tys, mode, span)?;
                self.variant_pattern(adt, args, variant, fields, expected, span)?
            }
            P::Struct { path, fields, has_rest } => {
                let Some((adt, args, variant)) = self.resolve_constructor(path)? else {
                    bail!(path.span, "cannot find struct or variant `{}`", crate::sema::context::path_text(path));
                };
                self.unify(expected, &Ty::Adt(adt, args.clone()), span)?;
                let field_tys = self.tcx.variant_fields(adt, &args, variant, &mut self.infer)?;
                let def = &self.tcx.defs.adt(adt).variants[variant as usize];
                let mut checked = Vec::new();
                for (name, field_pat) in fields {
                    let Some(index) = def.field_index(&name.name) else {
                        bail!(name.span, "`{}` has no field named `{}`", def.name, name.name);
                    };
                    if !self.field_visible(adt, variant, index) {
                        return Err(self.private_field(adt, variant, index, name.span));
                    }
                    checked.push((index, self.check_pat(field_pat, &field_tys[index], mode)?));
                }
                if !has_rest && checked.len() != def.fields.len() {
                    bail!(span, "pattern does not mention every field of `{}` (add `..` to ignore the rest)", def.name);
                }
                self.variant_pattern(adt, args, variant, checked, expected, span)?
            }
            P::Tuple(elements) => {
                let element_tys = match self.infer.shallow(expected) {
                    Ty::Tuple(tys) => tys,
                    Ty::Infer(_) if !elements.iter().any(|e| matches!(e.kind, P::Rest)) => {
                        let tys: Vec<Ty> = elements.iter().map(|_| self.infer.fresh_var()).collect();
                        self.unify(expected, &Ty::Tuple(tys.clone()), span)?;
                        tys
                    }
                    other => bail!(span, "expected `{}`, found a tuple pattern", self.show(&other)),
                };
                PatKind::Fields(self.check_positional(elements, &element_tys, mode, span)?)
            }
            P::Ref(inner) => {
                let pointee = self.infer.fresh_var();
                let shape = match self.infer.shallow(expected) {
                    Ty::Ref(_, mutability) => Ty::Ref(Box::new(pointee.clone()), mutability),
                    _ => Ty::shared_ref(pointee.clone()),
                };
                self.unify(expected, &shape, span)?;
                // An explicit `&` consumes the reference: bindings below are by value again.
                PatKind::Deref(Box::new(self.check_pat(inner, &pointee, BindingMode::Value)?))
            }
            P::Or(alternatives) => {
                let mut checked = Vec::new();
                let outer_bindings = self.or_bindings.take();
                for (index, alternative) in alternatives.iter().enumerate() {
                    if index == 0 {
                        let first_new = self.locals.len();
                        checked.push(self.check_pat(alternative, expected, mode)?);
                        // Later alternatives must bind the same names to the same locals.
                        let bound = (first_new..self.locals.len())
                            .map(|i| (self.locals[i].name.clone(), LocalId(i as u32)));
                        self.or_bindings = Some(bound.collect::<HashMap<_, _>>());
                    } else {
                        checked.push(self.check_pat(alternative, expected, mode)?);
                    }
                }
                self.or_bindings = outer_bindings;
                PatKind::Or(checked)
            }
            P::Slice(elements) => self.check_slice_pattern(elements, expected, mode, span)?,
        };
        Ok(Pat { kind, ty: expected.clone(), span })
    }

    /// `[first, second, .., last]` against an array or a slice.
    fn check_slice_pattern(
        &mut self,
        elements: &[ast::Pattern],
        expected: &Ty,
        mode: BindingMode,
        span: Span,
    ) -> Result<PatKind> {
        let mut rests = elements.iter().enumerate().filter(|(_, element)| is_rest(element)).map(|(index, _)| index);
        let rest_at = rests.next();
        if rests.next().is_some() {
            bail!(span, "`..` can be used only once in a slice pattern");
        }
        let fixed = elements.len() - rest_at.is_some() as usize;
        let resolved = self.structurally_resolve(expected, span)?;
        let (element_ty, array_len) = match &resolved {
            Ty::Array(element, _) => ((**element).clone(), resolved.array_len()),
            Ty::Slice(element) => ((**element).clone(), None),
            other => bail!(span, "expected `{}`, found a slice pattern", self.show(other)),
        };
        if let Some(len) = array_len {
            let fits = if rest_at.is_some() { fixed as u64 <= len } else { fixed as u64 == len };
            if !fits {
                bail!(span, "this pattern needs {fixed} element(s), but the array has {len}");
            }
        }
        let (before, rest, after) = match rest_at {
            Some(index) => (&elements[..index], Some(&elements[index]), &elements[index + 1..]),
            None => (elements, None, &[][..]),
        };
        let mut prefix = Vec::new();
        for element in before {
            prefix.push(self.check_pat(element, &element_ty, mode)?);
        }
        let rest = match rest {
            Some(rest) => {
                let rest_ty = match array_len {
                    Some(len) => Ty::array(element_ty.clone(), len - fixed as u64),
                    None => Ty::Slice(Box::new(element_ty.clone())),
                };
                Some(Box::new(self.check_rest_pattern(rest, &rest_ty, mode)?))
            }
            None => None,
        };
        let mut suffix = Vec::new();
        for element in after {
            suffix.push(self.check_pat(element, &element_ty, mode)?);
        }
        Ok(PatKind::Slice { prefix, rest, suffix })
    }

    /// The `..` of a slice pattern, or `name @ ..`, which binds the elements
    /// between the prefix and the suffix.
    fn check_rest_pattern(&mut self, pat: &ast::Pattern, rest_ty: &Ty, mode: BindingMode) -> Result<Pat> {
        let kind = match &pat.kind {
            P::Binding { name, mutable, by_ref, .. } => {
                let mode = match by_ref {
                    Some(mutable) => BindingMode::Ref(Mutability::from_bool(*mutable)),
                    None => mode,
                };
                let local_ty = match mode {
                    BindingMode::Value if rest_ty.is_unsized() => {
                        bail!(pat.span, "the rest of a slice can only be bound by reference")
                    }
                    BindingMode::Value => rest_ty.clone(),
                    BindingMode::Ref(mutability) => Ty::Ref(Box::new(rest_ty.clone()), mutability),
                };
                let local = self.bind(name, local_ty, *mutable)?;
                PatKind::Binding { local, mode, sub: None }
            }
            _ => PatKind::Wild,
        };
        Ok(Pat { kind, ty: rest_ty.clone(), span: pat.span })
    }

    /// Introduce the variable a binding pattern declares.
    fn bind(&mut self, name: &ast::Ident, ty: Ty, mutable: bool) -> Result<LocalId> {
        if let Some(existing) = self.or_bindings.as_ref().and_then(|bound| bound.get(&name.name).copied()) {
            let existing_ty = self.locals[existing.0 as usize].ty.clone();
            self.unify(&existing_ty, &ty, name.span)?;
            return Ok(existing);
        }
        if self.or_bindings.is_some() {
            bail!(name.span, "variable `{}` is not bound in all alternatives of the pattern", name.name);
        }
        Ok(self.declare_local(&name.name, ty, mutable))
    }

    /// Would a bare name in pattern position refer to a unit variant, a unit
    /// struct or a constant rather than introduce a variable?
    fn names_a_constant_or_variant(&self, pat: &ast::Pattern) -> bool {
        let P::Binding { name, mutable: false, by_ref: None, sub: None } = &pat.kind else { return false };
        match self.tcx.defs.lookup(self.module, &name.name) {
            Some(Def::Variant(adt, variant)) => {
                self.tcx.defs.adt(adt).variants[variant as usize].shape == VariantShape::Unit
            }
            Some(Def::Const(_)) => true,
            Some(Def::Adt(adt)) => {
                !self.tcx.is_enum(adt) && self.tcx.defs.adt(adt).variants[0].shape == VariantShape::Unit
            }
            _ => false,
        }
    }

    /// A bare name that is a unit variant (`None`), unit struct or constant.
    fn check_named_pattern(&mut self, name: &ast::Ident, expected: &Ty) -> Result<Option<PatKind>> {
        let as_pattern = ast::Pattern {
            kind: P::Binding { name: name.clone(), mutable: false, by_ref: None, sub: None },
            span: name.span,
        };
        if !self.names_a_constant_or_variant(&as_pattern) {
            return Ok(None);
        }
        let segment = ast::PathSegment { ident: name.clone(), args: Vec::new(), bindings: Vec::new() };
        let path = ast::Path::new(vec![segment], name.span);
        match self.resolve_constructor(&path)? {
            Some((adt, args, variant)) => {
                self.variant_pattern(adt, args, variant, Vec::new(), expected, name.span).map(Some)
            }
            None => {
                let value = eval_int(self.tcx, self.module, &path_expr(&path))?;
                Ok(Some(PatKind::Literal(Literal::Int(value))))
            }
        }
    }

    /// A pattern naming `adt`'s variant, with sub-patterns for some fields.
    fn variant_pattern(
        &mut self,
        adt: AdtId,
        args: Vec<Ty>,
        variant: u32,
        fields: Vec<(usize, Pat)>,
        expected: &Ty,
        span: Span,
    ) -> Result<PatKind> {
        self.unify(expected, &Ty::Adt(adt, args), span)?;
        Ok(if self.tcx.is_enum(adt) {
            PatKind::Variant { variant, fields }
        } else {
            PatKind::Fields(fields)
        })
    }

    /// Positional sub-patterns, where a single `..` stands for any number of
    /// skipped elements.
    fn check_positional(
        &mut self,
        patterns: &[ast::Pattern],
        tys: &[Ty],
        mode: BindingMode,
        span: Span,
    ) -> Result<Vec<(usize, Pat)>> {
        let rest_at = patterns.iter().position(|p| matches!(p.kind, P::Rest));
        let explicit = patterns.len() - usize::from(rest_at.is_some());
        if explicit > tys.len() || (rest_at.is_none() && explicit != tys.len()) {
            bail!(span, "this pattern has {explicit} element(s) but the value has {}", tys.len());
        }
        let skipped = tys.len() - explicit;

        let mut fields = Vec::new();
        for (position, pattern) in patterns.iter().enumerate() {
            let index = match rest_at {
                Some(rest) if position == rest => continue,
                Some(rest) if position > rest => position - 1 + skipped,
                _ => position,
            };
            fields.push((index, self.check_pat(pattern, &tys[index], mode)?));
        }
        Ok(fields)
    }

    fn check_literal_pattern_of(&mut self, pat: &ast::Pattern, expected: &Ty) -> Result<Literal> {
        let P::Literal(literal) = &pat.kind else { unreachable!("caller checked for a literal pattern") };
        self.check_literal_pattern(literal, expected)
    }

    fn check_literal_pattern(&mut self, literal: &ast::Expr, expected: &Ty) -> Result<Literal> {
        let checked = self.check_expr_coerce(literal, expected)?;
        let negate = matches!(literal.kind, ast::ExprKind::Unary(ast::UnOp::Neg, _));
        Ok(match &literal_operand(literal).kind {
            ast::ExprKind::Int(value, _) => {
                let value = *value as i128;
                Literal::Int(if negate { -value } else { value })
            }
            ast::ExprKind::Byte(value) => Literal::Int(*value as i128),
            ast::ExprKind::Bool(value) => Literal::Bool(*value),
            ast::ExprKind::Char(value) => Literal::Char(*value),
            ast::ExprKind::Str(value) => Literal::Str(value.clone()),
            _ => bail!(checked.span, "this kind of literal cannot be used in a pattern"),
        })
    }
}

/// The literal under an optional leading minus sign.
fn literal_operand(expr: &ast::Expr) -> &ast::Expr {
    match &expr.kind {
        ast::ExprKind::Unary(ast::UnOp::Neg, operand) => operand,
        _ => expr,
    }
}

fn path_expr(path: &ast::Path) -> ast::Expr {
    ast::Expr { kind: ast::ExprKind::Path(path.clone()), span: path.span }
}

/// `..` or `name @ ..`: the rest of a slice pattern.
fn is_rest(pat: &ast::Pattern) -> bool {
    match &pat.kind {
        P::Rest => true,
        P::Binding { sub: Some(sub), .. } => matches!(sub.kind, P::Rest),
        _ => false,
    }
}
