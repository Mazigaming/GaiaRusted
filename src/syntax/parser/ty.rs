//! Types, paths, generic parameters and trait bounds.

use super::Parser;
use crate::syntax::ast::*;
use crate::syntax::diagnostic::Result;
use crate::syntax::token::{Keyword, TokenKind};

impl Parser<'_> {
    pub(super) fn parse_type(&mut self) -> Result<Type> {
        let start = self.span();
        let kind = match self.peek().clone() {
            TokenKind::And | TokenKind::AndAnd => {
                self.eat_ampersand();
                self.skip_lifetime();
                let mutable = self.eat_keyword(Keyword::Mut);
                TypeKind::Ref { mutable, inner: Box::new(self.parse_type()?) }
            }
            TokenKind::Star => {
                self.bump();
                let mutable = if self.eat_keyword(Keyword::Mut) {
                    true
                } else {
                    self.expect_keyword(Keyword::Const)?;
                    false
                };
                TypeKind::Ptr { mutable, inner: Box::new(self.parse_type()?) }
            }
            TokenKind::LParen => {
                self.bump();
                let mut elements = self.comma_separated(&TokenKind::RParen, Self::parse_type)?;
                // `(T)` is just `T`; a one-element tuple is spelled `(T,)`.
                let was_parenthesised =
                    elements.len() == 1 && self.tokens[self.pos - 2].kind != TokenKind::Comma;
                if was_parenthesised {
                    return Ok(elements.remove(0));
                }
                TypeKind::Tuple(elements)
            }
            TokenKind::LBracket => {
                self.bump();
                let element = Box::new(self.parse_type()?);
                if self.eat(&TokenKind::Semi) {
                    let len = Box::new(self.parse_expr()?);
                    self.expect(&TokenKind::RBracket)?;
                    TypeKind::Array(element, len)
                } else {
                    self.expect(&TokenKind::RBracket)?;
                    TypeKind::Slice(element)
                }
            }
            TokenKind::Not => {
                self.bump();
                TypeKind::Never
            }
            TokenKind::Underscore => {
                self.bump();
                TypeKind::Infer
            }
            TokenKind::Keyword(Keyword::Dyn) => {
                self.bump();
                TypeKind::Dyn(self.parse_bounds()?)
            }
            TokenKind::Keyword(Keyword::Impl) => {
                self.bump();
                TypeKind::ImplTrait(self.parse_bounds()?)
            }
            TokenKind::Keyword(Keyword::Fn | Keyword::Unsafe | Keyword::Extern) => {
                self.parse_fn_pointer_type()?
            }
            _ => TypeKind::Path(self.parse_type_path()?),
        };
        Ok(Type { kind, span: start.to(self.prev_span()) })
    }

    /// `fn(A, B) -> R`, optionally prefixed by `unsafe` and/or `extern "C"`.
    fn parse_fn_pointer_type(&mut self) -> Result<TypeKind> {
        self.eat_keyword(Keyword::Unsafe);
        if self.eat_keyword(Keyword::Extern) && matches!(self.peek(), TokenKind::Str(_)) {
            self.bump();
        }
        self.expect_keyword(Keyword::Fn)?;
        self.expect(&TokenKind::LParen)?;
        let params = self.comma_separated(&TokenKind::RParen, Self::parse_type)?;
        let ret = self.parse_return_type()?.map(Box::new);
        Ok(TypeKind::Fn { params, ret })
    }

    /// `-> Type`, if present.
    pub(super) fn parse_return_type(&mut self) -> Result<Option<Type>> {
        if self.eat(&TokenKind::Arrow) {
            Ok(Some(self.parse_type()?))
        } else {
            Ok(None)
        }
    }

    /// A path in type position, where generic arguments follow the name
    /// directly: `Vec<T>`, `std::collections::HashMap<K, V>`, `T::Item`.
    pub(super) fn parse_type_path(&mut self) -> Result<Path> {
        self.parse_path(false)
    }

    /// A path in expression position, where generic arguments need the
    /// turbofish: `Vec::<i32>::new`, `parse::<i64>`.
    pub(super) fn parse_expr_path(&mut self) -> Result<Path> {
        self.parse_path(true)
    }

    fn parse_path(&mut self, needs_turbofish: bool) -> Result<Path> {
        let start = self.span();
        let qself = if self.at(&TokenKind::Lt) {
            Some(Box::new(self.parse_qself()?))
        } else {
            self.eat(&TokenKind::PathSep); // a leading `::` changes nothing for us
            None
        };
        let mut segments = Vec::new();
        loop {
            let ident = self.parse_path_segment_name()?;
            let mut segment = PathSegment { ident, args: Vec::new(), bindings: Vec::new() };

            let has_args = if needs_turbofish {
                self.at(&TokenKind::PathSep) && *self.peek_nth(1) == TokenKind::Lt
            } else {
                self.at(&TokenKind::Lt)
            };
            if has_args {
                if needs_turbofish {
                    self.bump();
                }
                self.parse_generic_args(&mut segment)?;
            }
            segments.push(segment);

            let continues = self.at(&TokenKind::PathSep)
                && matches!(
                    self.peek_nth(1),
                    TokenKind::Ident(_)
                        | TokenKind::Keyword(
                            Keyword::SelfValue | Keyword::SelfType | Keyword::Super | Keyword::Crate
                        )
                );
            if !continues {
                break;
            }
            self.bump();
        }
        Ok(Path { qself, segments, span: start.to(self.prev_span()) })
    }

    /// `<Type as Trait>::` or `<Type>::`, which begins a qualified path.
    fn parse_qself(&mut self) -> Result<QSelf> {
        self.expect(&TokenKind::Lt)?;
        let ty = self.parse_type()?;
        let trait_ref = if self.eat_keyword(Keyword::As) { Some(self.parse_path(false)?) } else { None };
        self.expect_closing_angle()?;
        self.expect(&TokenKind::PathSep)?;
        Ok(QSelf { ty, trait_ref })
    }

    /// Path segments may be the keywords `self`, `Self`, `super` and `crate`.
    fn parse_path_segment_name(&mut self) -> Result<Ident> {
        let keyword_name = match self.peek() {
            TokenKind::Keyword(Keyword::SelfValue) => "self",
            TokenKind::Keyword(Keyword::SelfType) => "Self",
            TokenKind::Keyword(Keyword::Super) => "super",
            TokenKind::Keyword(Keyword::Crate) => "crate",
            _ => return self.expect_ident(),
        };
        Ok(Ident { name: keyword_name.to_string(), span: self.bump().span })
    }

    /// `<A, B, Name = C>` — lifetimes are accepted and ignored.
    fn parse_generic_args(&mut self, segment: &mut PathSegment) -> Result<()> {
        self.expect(&TokenKind::Lt)?;
        while !matches!(self.peek(), TokenKind::Gt | TokenKind::Shr | TokenKind::Ge | TokenKind::ShrEq)
        {
            let is_binding =
                matches!(self.peek(), TokenKind::Ident(_)) && *self.peek_nth(1) == TokenKind::Eq;
            let is_const = matches!(
                self.peek(),
                TokenKind::Int(..) | TokenKind::Char(_) | TokenKind::LBrace | TokenKind::Minus
            ) || self.at_keyword(Keyword::True)
                || self.at_keyword(Keyword::False);
            if matches!(self.peek(), TokenKind::Lifetime(_)) {
                self.bump();
            } else if is_binding {
                let name = self.expect_ident()?;
                self.bump();
                segment.bindings.push((name, self.parse_type()?));
            } else if is_const {
                segment.args.push(self.parse_const_arg()?);
            } else {
                segment.args.push(self.parse_type()?);
            }
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_closing_angle()
    }

    /// A constant generic argument: a literal, possibly negated, or a block.
    fn parse_const_arg(&mut self) -> Result<Type> {
        let start = self.span();
        let value = if self.at(&TokenKind::LBrace) {
            let block = self.parse_block()?;
            Expr { span: start.to(self.prev_span()), kind: ExprKind::Block(block) }
        } else {
            self.parse_unary()?
        };
        Ok(Type { span: start.to(value.span), kind: TypeKind::Const(Box::new(value)) })
    }

    fn skip_lifetime(&mut self) {
        if matches!(self.peek(), TokenKind::Lifetime(_)) {
            self.bump();
        }
    }

    /// `<'a, T, U: Bound + Other>` — an absent list parses as empty.
    pub(super) fn parse_generic_params(&mut self) -> Result<Vec<GenericParam>> {
        let mut params = Vec::new();
        if !self.eat(&TokenKind::Lt) {
            return Ok(params);
        }
        while !self.at(&TokenKind::Gt) {
            if matches!(self.peek(), TokenKind::Lifetime(_)) {
                self.bump();
                if self.eat(&TokenKind::Colon) {
                    self.parse_bounds()?;
                }
            } else if self.eat_keyword(Keyword::Const) {
                let name = self.expect_ident()?;
                self.expect(&TokenKind::Colon)?;
                let ty = self.parse_type()?;
                if self.eat(&TokenKind::Eq) {
                    self.parse_const_arg()?; // default arguments are not used
                }
                params.push(GenericParam { name, bounds: Vec::new(), const_ty: Some(ty), maybe_unsized: false });
            } else {
                let name = self.expect_ident()?;
                let (bounds, maybe_unsized) =
                    if self.eat(&TokenKind::Colon) { self.parse_bounds_relaxed()? } else { (Vec::new(), false) };
                if self.eat(&TokenKind::Eq) {
                    self.parse_type()?; // default type arguments are not used
                }
                params.push(GenericParam { name, bounds, const_ty: None, maybe_unsized });
            }
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_closing_angle()?;
        Ok(params)
    }

    /// `where T: A + B, Vec<U>: C`
    pub(super) fn parse_where_clauses(&mut self) -> Result<Vec<(Type, Vec<Bound>)>> {
        let mut clauses = Vec::new();
        if !self.eat_keyword(Keyword::Where) {
            return Ok(clauses);
        }
        while !matches!(self.peek(), TokenKind::LBrace | TokenKind::Semi | TokenKind::Eof) {
            if matches!(self.peek(), TokenKind::Lifetime(_)) {
                self.bump();
                self.expect(&TokenKind::Colon)?;
                self.parse_bounds()?;
            } else {
                let ty = self.parse_type()?;
                self.expect(&TokenKind::Colon)?;
                clauses.push((ty, self.parse_bounds()?));
            }
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        Ok(clauses)
    }

    /// `Bound + Bound + ...`; lifetime bounds and `?Sized` are accepted and dropped.
    pub(super) fn parse_bounds(&mut self) -> Result<Vec<Bound>> {
        Ok(self.parse_bounds_relaxed()?.0)
    }

    /// Bounds, and whether they include `?Sized`.
    fn parse_bounds_relaxed(&mut self) -> Result<(Vec<Bound>, bool)> {
        let mut bounds = Vec::new();
        let mut maybe_unsized = false;
        loop {
            if matches!(self.peek(), TokenKind::Lifetime(_)) {
                self.bump();
            } else if self.eat(&TokenKind::Question) {
                maybe_unsized |= self.parse_type_path()?.as_ident().is_some_and(|name| name.name == "Sized");
            } else {
                bounds.push(self.parse_bound()?);
            }
            if !self.eat(&TokenKind::Plus) {
                return Ok((bounds, maybe_unsized));
            }
        }
    }

    fn parse_bound(&mut self) -> Result<Bound> {
        let path = self.parse_type_path()?;
        // `Fn(A, B) -> R`
        let fn_sugar = if self.eat(&TokenKind::LParen) {
            let params = self.comma_separated(&TokenKind::RParen, Self::parse_type)?;
            Some((params, self.parse_return_type()?.map(Box::new)))
        } else {
            None
        };
        Ok(Bound { path, fn_sugar })
    }
}
