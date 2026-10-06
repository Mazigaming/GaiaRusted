//! Patterns: the left-hand side of `let`, match arms, parameters.

use super::Parser;
use crate::syntax::ast::*;
use crate::syntax::diagnostic::Result;
use crate::syntax::token::{Keyword, TokenKind};

impl Parser<'_> {
    /// A pattern with `|` alternatives, as in match arms and `let`.
    pub(super) fn parse_pattern(&mut self) -> Result<Pattern> {
        let start = self.span();
        self.eat(&TokenKind::Or); // a leading `|` is allowed
        let first = self.parse_pattern_no_alternatives()?;
        if !self.at(&TokenKind::Or) {
            return Ok(first);
        }
        let mut alternatives = vec![first];
        while self.eat(&TokenKind::Or) {
            alternatives.push(self.parse_pattern_no_alternatives()?);
        }
        Ok(Pattern { kind: PatternKind::Or(alternatives), span: start.to(self.prev_span()) })
    }

    /// A single pattern. Closure parameters use this directly, because there
    /// a `|` ends the parameter list.
    pub(super) fn parse_pattern_no_alternatives(&mut self) -> Result<Pattern> {
        let start = self.span();
        let kind = match self.peek().clone() {
            TokenKind::Underscore => {
                self.bump();
                PatternKind::Wild
            }
            TokenKind::DotDot if !self.starts_literal(self.peek_nth(1)) => {
                self.bump();
                PatternKind::Rest
            }
            TokenKind::DotDot | TokenKind::DotDotEq => {
                let inclusive = self.bump().kind == TokenKind::DotDotEq;
                let hi = Some(Box::new(self.parse_pattern_literal()?));
                PatternKind::Range { lo: None, hi, inclusive }
            }
            TokenKind::And | TokenKind::AndAnd => {
                self.eat_ampersand();
                self.eat_keyword(Keyword::Mut);
                PatternKind::Ref(Box::new(self.parse_pattern_no_alternatives()?))
            }
            TokenKind::LParen => {
                self.bump();
                let mut elements = self.comma_separated(&TokenKind::RParen, Self::parse_pattern)?;
                let was_parenthesised = elements.len() == 1
                    && self.tokens[self.pos - 2].kind != TokenKind::Comma
                    && !matches!(elements[0].kind, PatternKind::Rest);
                if was_parenthesised {
                    return Ok(elements.remove(0));
                }
                PatternKind::Tuple(elements)
            }
            TokenKind::LBracket => {
                self.bump();
                PatternKind::Slice(self.comma_separated(&TokenKind::RBracket, Self::parse_pattern)?)
            }
            TokenKind::Keyword(Keyword::Mut) => {
                self.bump();
                let name = self.expect_ident()?;
                self.parse_binding(name, true, None)?
            }
            TokenKind::Keyword(Keyword::Ref) => {
                self.bump();
                let by_mut_ref = self.eat_keyword(Keyword::Mut);
                let name = self.expect_ident()?;
                self.parse_binding(name, false, Some(by_mut_ref))?
            }
            ref token if self.starts_literal(token) => {
                let lo = Box::new(self.parse_pattern_literal()?);
                self.parse_range_end(lo)?
            }
            _ => self.parse_path_pattern()?,
        };
        Ok(Pattern { kind, span: start.to(self.prev_span()) })
    }

    fn starts_literal(&self, token: &TokenKind) -> bool {
        matches!(
            token,
            TokenKind::Int(..)
                | TokenKind::Float(..)
                | TokenKind::Str(_)
                | TokenKind::Char(_)
                | TokenKind::Byte(_)
                | TokenKind::ByteStr(_)
                | TokenKind::Minus
                | TokenKind::Keyword(Keyword::True | Keyword::False)
        )
    }

    /// A literal as it may appear in a pattern: optionally negated.
    fn parse_pattern_literal(&mut self) -> Result<Expr> {
        let start = self.span();
        if self.eat(&TokenKind::Minus) {
            let operand = Box::new(self.parse_literal()?);
            let span = start.to(self.prev_span());
            return Ok(Expr { kind: ExprKind::Unary(UnOp::Neg, operand), span });
        }
        if matches!(self.peek(), TokenKind::Ident(_)) {
            // A named constant as a range bound: `0..=MAX`.
            let path = self.parse_expr_path()?;
            return Ok(Expr { span: path.span, kind: ExprKind::Path(path) });
        }
        self.parse_literal()
    }

    /// After a literal `lo`: is this `lo..=hi` / `lo..hi` / `lo..` or just `lo`?
    fn parse_range_end(&mut self, lo: Box<Expr>) -> Result<PatternKind> {
        let inclusive = match self.peek() {
            TokenKind::DotDotEq | TokenKind::DotDotDot => true,
            TokenKind::DotDot => false,
            _ => return Ok(PatternKind::Literal(lo)),
        };
        self.bump();
        let has_end = self.starts_literal(self.peek()) || matches!(self.peek(), TokenKind::Ident(_));
        let hi = if has_end { Some(Box::new(self.parse_pattern_literal()?)) } else { None };
        Ok(PatternKind::Range { lo: Some(lo), hi, inclusive })
    }

    /// `name`, `name @ sub`, with the `mut` / `ref` modifiers already consumed.
    fn parse_binding(&mut self, name: Ident, mutable: bool, by_ref: Option<bool>) -> Result<PatternKind> {
        let sub = if self.eat(&TokenKind::At) {
            Some(Box::new(self.parse_pattern_no_alternatives()?))
        } else {
            None
        };
        Ok(PatternKind::Binding { name, mutable, by_ref, sub })
    }

    /// Patterns that begin with a name: bindings, variants, structs, constants.
    fn parse_path_pattern(&mut self) -> Result<PatternKind> {
        if !matches!(
            self.peek(),
            TokenKind::Ident(_)
                | TokenKind::PathSep
                | TokenKind::Keyword(Keyword::SelfType | Keyword::Crate | Keyword::Super | Keyword::SelfValue)
        ) {
            return Err(self.unexpected("a pattern"));
        }
        let path = self.parse_expr_path()?;

        match self.peek() {
            TokenKind::LParen => {
                self.bump();
                let fields = self.comma_separated(&TokenKind::RParen, Self::parse_pattern)?;
                Ok(PatternKind::TupleStruct(path, fields))
            }
            TokenKind::LBrace => {
                self.bump();
                self.parse_struct_pattern_fields(path)
            }
            TokenKind::DotDotEq | TokenKind::DotDotDot | TokenKind::DotDot => {
                let lo = Box::new(Expr { span: path.span, kind: ExprKind::Path(path) });
                self.parse_range_end(lo)
            }
            _ => match path.as_ident() {
                Some(name) => self.parse_binding(name.clone(), false, None),
                None => Ok(PatternKind::Path(path)),
            },
        }
    }

    /// `{ x, y: pattern, ref z, .. }` — the opening brace is already consumed.
    fn parse_struct_pattern_fields(&mut self, path: Path) -> Result<PatternKind> {
        let mut fields = Vec::new();
        let mut has_rest = false;
        while !self.at(&TokenKind::RBrace) {
            if self.eat(&TokenKind::DotDot) {
                has_rest = true;
                break;
            }
            let start = self.span();
            let by_ref = if self.eat_keyword(Keyword::Ref) {
                Some(self.eat_keyword(Keyword::Mut))
            } else {
                None
            };
            let mutable = self.eat_keyword(Keyword::Mut);
            let name = self.expect_ident()?;
            let pattern = if self.eat(&TokenKind::Colon) {
                self.parse_pattern()?
            } else {
                // Shorthand: `x` means `x: x`.
                let kind = PatternKind::Binding { name: name.clone(), mutable, by_ref, sub: None };
                Pattern { kind, span: start.to(self.prev_span()) }
            };
            fields.push((name, pattern));
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(PatternKind::Struct { path, fields, has_rest })
    }
}
