//! Expressions, statements and blocks.
//!
//! Binary operators are parsed by precedence climbing; everything tighter
//! than a binary operator (prefix operators, postfix `.`/`()`/`[]`/`?`,
//! literals, control flow) is ordinary recursive descent.

use super::macros::Position;
use super::Parser;
use crate::syntax::ast::*;
use crate::syntax::diagnostic::{bail, Result};
use crate::syntax::token::{Keyword, TokenKind};

/// Binding power of `as`, which binds tighter than every binary operator.
const CAST_PRECEDENCE: u8 = 10;
/// Operands of `let` in a condition stop before `&&` and `||`.
const ABOVE_LAZY_BOOLEAN: u8 = 3;

fn binary_operator(token: &TokenKind) -> Option<(BinOp, u8)> {
    Some(match token {
        TokenKind::OrOr => (BinOp::Or, 1),
        TokenKind::AndAnd => (BinOp::And, 2),
        TokenKind::EqEq => (BinOp::Eq, 3),
        TokenKind::Ne => (BinOp::Ne, 3),
        TokenKind::Lt => (BinOp::Lt, 3),
        TokenKind::Le => (BinOp::Le, 3),
        TokenKind::Gt => (BinOp::Gt, 3),
        TokenKind::Ge => (BinOp::Ge, 3),
        TokenKind::Or => (BinOp::BitOr, 4),
        TokenKind::Caret => (BinOp::BitXor, 5),
        TokenKind::And => (BinOp::BitAnd, 6),
        TokenKind::Shl => (BinOp::Shl, 7),
        TokenKind::Shr => (BinOp::Shr, 7),
        TokenKind::Plus => (BinOp::Add, 8),
        TokenKind::Minus => (BinOp::Sub, 8),
        TokenKind::Star => (BinOp::Mul, 9),
        TokenKind::Slash => (BinOp::Div, 9),
        TokenKind::Percent => (BinOp::Rem, 9),
        _ => return None,
    })
}

fn compound_assignment(token: &TokenKind) -> Option<BinOp> {
    Some(match token {
        TokenKind::PlusEq => BinOp::Add,
        TokenKind::MinusEq => BinOp::Sub,
        TokenKind::StarEq => BinOp::Mul,
        TokenKind::SlashEq => BinOp::Div,
        TokenKind::PercentEq => BinOp::Rem,
        TokenKind::AndEq => BinOp::BitAnd,
        TokenKind::OrEq => BinOp::BitOr,
        TokenKind::CaretEq => BinOp::BitXor,
        TokenKind::ShlEq => BinOp::Shl,
        TokenKind::ShrEq => BinOp::Shr,
        _ => return None,
    })
}

/// Expressions that end with a block and so need no `;` as a statement.
fn is_block_like(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        ExprKind::Block(_)
            | ExprKind::Unsafe(_)
            | ExprKind::If { .. }
            | ExprKind::Match { .. }
            | ExprKind::While { .. }
            | ExprKind::Loop { .. }
            | ExprKind::LabeledBlock { .. }
            | ExprKind::For { .. }
    )
}

fn boxed(expr: Expr) -> Box<Expr> {
    Box::new(expr)
}

impl Parser<'_> {
    // -- blocks and statements ------------------------------------------------

    pub(super) fn parse_block(&mut self) -> Result<Block> {
        let start = self.expect(&TokenKind::LBrace)?;
        self.with_struct_literals(true, |p| {
            let mut stmts = Vec::new();
            let mut tail = None;
            while !p.at(&TokenKind::RBrace) {
                if p.eat(&TokenKind::Semi) || p.expand_in_item_position(Position::Statements)? {
                    continue;
                }
                match p.parse_statement()? {
                    StatementOrTail::Statement(stmt) => stmts.push(stmt),
                    StatementOrTail::Tail(expr) => {
                        tail = Some(boxed(expr));
                        break;
                    }
                }
            }
            p.expect(&TokenKind::RBrace)?;
            Ok(Block { stmts, expr: tail, span: start.to(p.prev_span()) })
        })
    }

    fn parse_statement(&mut self) -> Result<StatementOrTail> {
        let start = self.span();
        // Attributes on an item belong to it. On a statement they are lint
        // settings (`#[allow(unused)]`), which change nothing it does.
        let attrs = self.parse_attributes()?;
        if self.at_item_start() {
            let item = Box::new(self.parse_item_after(attrs, start)?);
            let span = start.to(self.prev_span());
            return Ok(StatementOrTail::Statement(Stmt { kind: StmtKind::Item(item), span }));
        }
        if self.at_keyword(Keyword::Let) {
            let kind = self.parse_let()?;
            return Ok(StatementOrTail::Statement(Stmt { kind, span: start.to(self.prev_span()) }));
        }

        let expr = self.parse_statement_expr()?;
        if self.at(&TokenKind::RBrace) {
            return Ok(StatementOrTail::Tail(expr));
        }
        if !self.eat(&TokenKind::Semi) && !is_block_like(&expr) {
            return Err(self.unexpected("`;` or `}`"));
        }
        let span = start.to(self.prev_span());
        Ok(StatementOrTail::Statement(Stmt { kind: StmtKind::Expr(expr), span }))
    }

    /// `let PATTERN (: TYPE)? (= EXPR (else BLOCK)?)? ;`
    fn parse_let(&mut self) -> Result<StmtKind> {
        self.expect_keyword(Keyword::Let)?;
        let pat = self.parse_pattern()?;
        let ty = if self.eat(&TokenKind::Colon) { Some(self.parse_type()?) } else { None };
        let init = if self.eat(&TokenKind::Eq) { Some(self.parse_expr()?) } else { None };
        let else_block =
            if self.eat_keyword(Keyword::Else) { Some(self.parse_block()?) } else { None };
        self.expect(&TokenKind::Semi)?;
        Ok(StmtKind::Let { pat, ty, init, else_block })
    }

    /// An expression in statement position. A block-like expression there is
    /// a complete statement: `if a {} *p = 1;` is two statements, not a
    /// multiplication. Only `.method()` and `?` may continue it.
    fn parse_statement_expr(&mut self) -> Result<Expr> {
        let starts_block_like = matches!(
            self.peek(),
            TokenKind::LBrace
                | TokenKind::Lifetime(_)
                | TokenKind::Keyword(
                    Keyword::If
                        | Keyword::Match
                        | Keyword::While
                        | Keyword::Loop
                        | Keyword::For
                        | Keyword::Unsafe
                )
        );
        if !starts_block_like {
            return self.parse_expr();
        }
        let expr = self.parse_primary()?;
        if matches!(self.peek(), TokenKind::Dot | TokenKind::Question) {
            let expr = self.parse_postfix_operators(expr)?;
            let expr = self.parse_binary_rest(expr, 0)?;
            return self.parse_assignment_rest(expr);
        }
        Ok(expr)
    }

    // -- expressions, loosest to tightest --------------------------------------

    pub(super) fn parse_expr(&mut self) -> Result<Expr> {
        let lhs = self.parse_range()?;
        self.parse_assignment_rest(lhs)
    }

    /// `lhs = rhs` / `lhs += rhs`, right-associative.
    fn parse_assignment_rest(&mut self, lhs: Expr) -> Result<Expr> {
        let start = lhs.span;
        if self.eat(&TokenKind::Eq) {
            let rhs = self.parse_expr()?;
            let span = start.to(rhs.span);
            return Ok(Expr { kind: ExprKind::Assign(boxed(lhs), boxed(rhs)), span });
        }
        if let Some(op) = compound_assignment(self.peek()) {
            self.bump();
            let rhs = self.parse_expr()?;
            let span = start.to(rhs.span);
            return Ok(Expr { kind: ExprKind::AssignOp(op, boxed(lhs), boxed(rhs)), span });
        }
        Ok(lhs)
    }

    /// `a..b`, `a..=b`, `a..`, `..b`, `..`
    fn parse_range(&mut self) -> Result<Expr> {
        let start = self.span();
        let lo = if matches!(self.peek(), TokenKind::DotDot | TokenKind::DotDotEq) {
            None
        } else {
            let lhs = self.parse_binary(0)?;
            if !matches!(self.peek(), TokenKind::DotDot | TokenKind::DotDotEq) {
                return Ok(lhs);
            }
            Some(boxed(lhs))
        };
        let inclusive = self.bump().kind == TokenKind::DotDotEq;
        let hi = if self.at_expr_end() { None } else { Some(boxed(self.parse_binary(0)?)) };
        Ok(Expr { kind: ExprKind::Range { lo, hi, inclusive }, span: start.to(self.prev_span()) })
    }

    /// Does the current token end an expression (so an open range has no end)?
    fn at_expr_end(&self) -> bool {
        match self.peek() {
            TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::RBrace
            | TokenKind::Semi
            | TokenKind::Comma
            | TokenKind::FatArrow
            | TokenKind::Eof => true,
            TokenKind::LBrace => self.no_struct_literal,
            _ => false,
        }
    }

    fn parse_binary(&mut self, min_precedence: u8) -> Result<Expr> {
        let lhs = self.parse_unary()?;
        self.parse_binary_rest(lhs, min_precedence)
    }

    /// Continue a binary expression whose first operand is already parsed.
    fn parse_binary_rest(&mut self, mut lhs: Expr, min_precedence: u8) -> Result<Expr> {
        loop {
            if self.at_keyword(Keyword::As) && CAST_PRECEDENCE >= min_precedence {
                self.bump();
                let ty = self.parse_type()?;
                let span = lhs.span.to(ty.span);
                lhs = Expr { kind: ExprKind::Cast(boxed(lhs), ty), span };
                continue;
            }
            let Some((op, precedence)) = binary_operator(self.peek()) else {
                return Ok(lhs);
            };
            if precedence < min_precedence {
                return Ok(lhs);
            }
            self.bump();
            let rhs = self.parse_binary(precedence + 1)?;
            let span = lhs.span.to(rhs.span);
            lhs = Expr { kind: ExprKind::Binary(op, boxed(lhs), boxed(rhs)), span };
        }
    }

    pub(super) fn parse_unary(&mut self) -> Result<Expr> {
        let start = self.span();
        let kind = match self.peek() {
            TokenKind::Minus => {
                self.bump();
                ExprKind::Unary(UnOp::Neg, boxed(self.parse_unary()?))
            }
            TokenKind::Not => {
                self.bump();
                ExprKind::Unary(UnOp::Not, boxed(self.parse_unary()?))
            }
            TokenKind::Star => {
                self.bump();
                ExprKind::Unary(UnOp::Deref, boxed(self.parse_unary()?))
            }
            TokenKind::And | TokenKind::AndAnd => {
                self.eat_ampersand();
                let mutable = self.eat_keyword(Keyword::Mut);
                ExprKind::AddrOf { mutable, expr: boxed(self.parse_unary()?) }
            }
            TokenKind::Keyword(Keyword::Let) => {
                self.bump();
                let pattern = Box::new(self.parse_pattern()?);
                self.expect(&TokenKind::Eq)?;
                let scrutinee = self.parse_binary(ABOVE_LAZY_BOOLEAN)?;
                ExprKind::Let(pattern, boxed(scrutinee))
            }
            _ => {
                let primary = self.parse_primary()?;
                return self.parse_postfix_operators(primary);
            }
        };
        Ok(Expr { kind, span: start.to(self.prev_span()) })
    }

    /// `expr.field`, `expr.method(args)`, `expr(args)`, `expr[index]`, `expr?`
    fn parse_postfix_operators(&mut self, mut expr: Expr) -> Result<Expr> {
        loop {
            let start = expr.span;
            let kind = match self.peek() {
                TokenKind::Question => {
                    self.bump();
                    ExprKind::Try(boxed(expr))
                }
                TokenKind::LParen => {
                    self.bump();
                    ExprKind::Call(boxed(expr), self.parse_call_args()?)
                }
                TokenKind::LBracket => {
                    self.bump();
                    let index = self.with_struct_literals(true, Self::parse_expr)?;
                    self.expect(&TokenKind::RBracket)?;
                    ExprKind::Index(boxed(expr), boxed(index))
                }
                TokenKind::Dot => {
                    self.bump();
                    self.parse_member_access(expr)?
                }
                _ => return Ok(expr),
            };
            expr = Expr { kind, span: start.to(self.prev_span()) };
        }
    }

    /// What follows a `.`: a field, a tuple index, or a method call.
    fn parse_member_access(&mut self, receiver: Expr) -> Result<ExprKind> {
        match self.peek().clone() {
            TokenKind::Int(index, None) => {
                self.bump();
                Ok(ExprKind::TupleField(boxed(receiver), index as u32))
            }
            // `pair.0.1` reaches us as the float token `0.1`.
            TokenKind::Float(_, None) => {
                let token = self.bump();
                let text = self.sources.snippet(token.span).to_string();
                let indices: Option<Vec<u32>> = text.split('.').map(|i| i.parse().ok()).collect();
                let Some(&[outer, inner]) = indices.as_deref() else {
                    bail!(token.span, "expected a field name or tuple index");
                };
                let span = receiver.span.to(token.span);
                let outer_access =
                    Expr { kind: ExprKind::TupleField(boxed(receiver), outer), span };
                Ok(ExprKind::TupleField(boxed(outer_access), inner))
            }
            _ => {
                let name = self.expect_ident()?;
                let turbofish = if self.at(&TokenKind::PathSep) {
                    self.bump();
                    self.expect(&TokenKind::Lt)?;
                    let mut types = Vec::new();
                    while !matches!(self.peek(), TokenKind::Gt | TokenKind::Shr) {
                        types.push(self.parse_type()?);
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect_closing_angle()?;
                    types
                } else {
                    Vec::new()
                };
                if self.eat(&TokenKind::LParen) {
                    let args = self.parse_call_args()?;
                    Ok(ExprKind::MethodCall {
                        receiver: boxed(receiver),
                        method: name,
                        turbofish,
                        args,
                    })
                } else if turbofish.is_empty() {
                    Ok(ExprKind::Field(boxed(receiver), name))
                } else {
                    Err(self.unexpected("`(` after the method's type arguments"))
                }
            }
        }
    }

    /// Arguments up to the closing `)`; the opening one is already consumed.
    fn parse_call_args(&mut self) -> Result<Vec<Expr>> {
        self.with_struct_literals(true, |p| p.comma_separated(&TokenKind::RParen, Self::parse_expr))
    }

    // -- primary expressions ---------------------------------------------------

    pub(super) fn parse_literal(&mut self) -> Result<Expr> {
        let token = self.bump();
        let kind = match token.kind {
            TokenKind::Int(value, suffix) => ExprKind::Int(value, suffix),
            TokenKind::Float(value, suffix) => ExprKind::Float(value, suffix),
            TokenKind::Str(text) => ExprKind::Str(text),
            TokenKind::Char(c) => ExprKind::Char(c),
            TokenKind::Byte(b) => ExprKind::Byte(b),
            TokenKind::ByteStr(bytes) => ExprKind::ByteStr(bytes),
            TokenKind::Keyword(Keyword::True) => ExprKind::Bool(true),
            TokenKind::Keyword(Keyword::False) => ExprKind::Bool(false),
            other => bail!(token.span, "expected a literal, found {other}"),
        };
        Ok(Expr { kind, span: token.span })
    }

    pub(super) fn parse_primary(&mut self) -> Result<Expr> {
        if let Some(definition) = self.at_macro_invocation() {
            self.expand_invocation(&definition, Position::Expression)?;
        }
        let start = self.span();
        let kind = match self.peek().clone() {
            TokenKind::Int(..)
            | TokenKind::Float(..)
            | TokenKind::Str(_)
            | TokenKind::Char(_)
            | TokenKind::Byte(_)
            | TokenKind::ByteStr(_)
            | TokenKind::Keyword(Keyword::True | Keyword::False) => return self.parse_literal(),

            TokenKind::LParen => {
                self.bump();
                let mut elements = self.parse_call_args()?;
                let was_parenthesised =
                    elements.len() == 1 && self.tokens[self.pos - 2].kind != TokenKind::Comma;
                if was_parenthesised {
                    // Keep the inner expression but let its span cover the parentheses.
                    let inner = elements.remove(0);
                    return Ok(Expr { kind: inner.kind, span: start.to(self.prev_span()) });
                }
                ExprKind::Tuple(elements)
            }
            TokenKind::LBracket => {
                self.bump();
                self.with_struct_literals(true, Self::parse_array_body)?
            }
            TokenKind::LBrace => ExprKind::Block(self.parse_block()?),

            TokenKind::Keyword(Keyword::Unsafe) => {
                self.bump();
                ExprKind::Unsafe(self.parse_block()?)
            }
            TokenKind::Keyword(Keyword::If) => self.parse_if()?,
            TokenKind::Keyword(Keyword::Match) => self.parse_match()?,
            TokenKind::Keyword(Keyword::While | Keyword::Loop | Keyword::For) => {
                self.parse_loop(None)?
            }
            TokenKind::Lifetime(label) if *self.peek_nth(1) == TokenKind::Colon => {
                self.bump();
                self.bump();
                self.parse_loop(Some(label))?
            }

            TokenKind::Keyword(Keyword::Return) => {
                self.bump();
                ExprKind::Return(self.parse_optional_operand()?)
            }
            TokenKind::Keyword(Keyword::Break) => {
                self.bump();
                let label = self.parse_optional_label();
                ExprKind::Break { label, value: self.parse_optional_operand()? }
            }
            TokenKind::Keyword(Keyword::Continue) => {
                self.bump();
                ExprKind::Continue { label: self.parse_optional_label() }
            }

            TokenKind::Keyword(Keyword::Move) | TokenKind::Or | TokenKind::OrOr => {
                self.parse_closure()?
            }

            TokenKind::Ident(_)
            | TokenKind::PathSep
            | TokenKind::Keyword(
                Keyword::SelfValue | Keyword::SelfType | Keyword::Crate | Keyword::Super,
            ) => self.parse_path_expr()?,

            TokenKind::Lt => self.parse_path_expr()?,
            TokenKind::Underscore => {
                self.bump();
                ExprKind::Underscore
            }
            _ => return Err(self.unexpected("an expression")),
        };
        Ok(Expr { kind, span: start.to(self.prev_span()) })
    }

    /// The operand of `return` / `break`, which may be absent.
    fn parse_optional_operand(&mut self) -> Result<Option<Box<Expr>>> {
        if self.at_expr_end() {
            Ok(None)
        } else {
            Ok(Some(boxed(self.parse_expr()?)))
        }
    }

    fn parse_optional_label(&mut self) -> Option<String> {
        match self.peek().clone() {
            TokenKind::Lifetime(label) => {
                self.bump();
                Some(label)
            }
            _ => None,
        }
    }

    /// `[a, b, c]` or `[value; count]`; the opening bracket is already consumed.
    fn parse_array_body(&mut self) -> Result<ExprKind> {
        if self.eat(&TokenKind::RBracket) {
            return Ok(ExprKind::Array(Vec::new()));
        }
        let first = self.parse_expr()?;
        if self.eat(&TokenKind::Semi) {
            let count = self.parse_expr()?;
            self.expect(&TokenKind::RBracket)?;
            return Ok(ExprKind::Repeat(boxed(first), boxed(count)));
        }
        let mut elements = vec![first];
        if self.eat(&TokenKind::Comma) {
            elements.extend(self.comma_separated(&TokenKind::RBracket, Self::parse_expr)?);
        } else {
            self.expect(&TokenKind::RBracket)?;
        }
        Ok(ExprKind::Array(elements))
    }

    /// A condition: struct literals are off, so `if x { ... }` reads `x` alone.
    fn parse_condition(&mut self) -> Result<Expr> {
        self.with_struct_literals(false, Self::parse_expr)
    }

    fn parse_if(&mut self) -> Result<ExprKind> {
        self.expect_keyword(Keyword::If)?;
        let cond = boxed(self.parse_condition()?);
        let then_block = self.parse_block()?;
        let else_expr = if self.eat_keyword(Keyword::Else) {
            let start = self.span();
            let kind = if self.at_keyword(Keyword::If) {
                self.parse_if()?
            } else {
                ExprKind::Block(self.parse_block()?)
            };
            Some(boxed(Expr { kind, span: start.to(self.prev_span()) }))
        } else {
            None
        };
        Ok(ExprKind::If { cond, then_block, else_expr })
    }

    fn parse_match(&mut self) -> Result<ExprKind> {
        self.expect_keyword(Keyword::Match)?;
        let scrutinee = boxed(self.parse_condition()?);
        self.expect(&TokenKind::LBrace)?;
        let arms = self.with_struct_literals(true, |p| {
            let mut arms = Vec::new();
            while !p.at(&TokenKind::RBrace) {
                let pat = p.parse_pattern()?;
                let guard = if p.eat_keyword(Keyword::If) { Some(p.parse_expr()?) } else { None };
                p.expect(&TokenKind::FatArrow)?;
                // Statement rules apply to the body: `{ ... }` ends the arm, so a
                // following `(a, b) => ...` is the next arm and not a call.
                let body = p.parse_statement_expr()?;
                // A block-bodied arm may omit the comma; any other arm needs
                // one unless it is the last.
                let had_comma = p.eat(&TokenKind::Comma);
                if !had_comma && !is_block_like(&body) && !p.at(&TokenKind::RBrace) {
                    return Err(p.unexpected("`,` or `}` after the match arm"));
                }
                arms.push(Arm { pat, guard, body });
            }
            Ok(arms)
        })?;
        self.expect(&TokenKind::RBrace)?;
        Ok(ExprKind::Match { scrutinee, arms })
    }

    fn parse_loop(&mut self, label: Option<String>) -> Result<ExprKind> {
        if let (Some(label), TokenKind::LBrace) = (&label, self.peek()) {
            return Ok(ExprKind::LabeledBlock { label: label.clone(), body: self.parse_block()? });
        }
        match self.bump().kind {
            TokenKind::Keyword(Keyword::Loop) => {
                Ok(ExprKind::Loop { label, body: self.parse_block()? })
            }
            TokenKind::Keyword(Keyword::While) => {
                let cond = boxed(self.parse_condition()?);
                Ok(ExprKind::While { label, cond, body: self.parse_block()? })
            }
            TokenKind::Keyword(Keyword::For) => {
                let pat = Box::new(self.parse_pattern()?);
                self.expect_keyword(Keyword::In)?;
                let iter = boxed(self.parse_condition()?);
                Ok(ExprKind::For { label, pat, iter, body: self.parse_block()? })
            }
            _ => Err(self.unexpected("a loop or block after the label")),
        }
    }

    /// `|a, b: T| body`, `move || body`, `|x| -> T { body }`
    fn parse_closure(&mut self) -> Result<ExprKind> {
        let is_move = self.eat_keyword(Keyword::Move);
        let mut params = Vec::new();
        if !self.eat(&TokenKind::OrOr) {
            self.expect(&TokenKind::Or)?;
            while !self.at(&TokenKind::Or) {
                let pat = self.parse_pattern_no_alternatives()?;
                let ty = if self.eat(&TokenKind::Colon) { Some(self.parse_type()?) } else { None };
                params.push(ClosureParam { pat, ty });
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::Or)?;
        }
        let ret = self.parse_return_type()?;
        let body = if ret.is_some() {
            // With an explicit return type the body must be a block.
            let block = self.parse_block()?;
            Expr { span: block.span, kind: ExprKind::Block(block) }
        } else {
            self.parse_expr()?
        };
        Ok(ExprKind::Closure { params, ret, body: boxed(body), is_move })
    }

    /// Something starting with a name: a path, a struct literal or a macro call.
    fn parse_path_expr(&mut self) -> Result<ExprKind> {
        let path = self.parse_expr_path()?;

        if self.at(&TokenKind::Not) && !matches!(self.peek_nth(1), TokenKind::Eq) {
            if let Some(name) = path.as_ident().cloned() {
                self.bump();
                if !self.expand_macros {
                    return self.skip_invocation(name.span);
                }
                return self.parse_macro_call(name);
            }
        }
        if self.at(&TokenKind::LBrace) && !self.no_struct_literal {
            self.bump();
            return self.parse_struct_literal(path);
        }
        Ok(ExprKind::Path(path))
    }

    /// `{ a: 1, b, ..base }` — the opening brace is already consumed.
    fn parse_struct_literal(&mut self, path: Path) -> Result<ExprKind> {
        let mut fields = Vec::new();
        let mut base = None;
        while !self.at(&TokenKind::RBrace) {
            if self.eat(&TokenKind::DotDot) {
                base = Some(boxed(self.parse_expr()?));
                break;
            }
            let name = match self.peek().clone() {
                // Tuple structs can be written `Pair { 0: a, 1: b }`.
                TokenKind::Int(index, None) => Ident { name: index.to_string(), span: self.bump().span },
                _ => self.expect_ident()?,
            };
            let value = if self.eat(&TokenKind::Colon) {
                self.parse_expr()?
            } else {
                // Shorthand: `x` means `x: x`.
                let segment = PathSegment { ident: name.clone(), args: Vec::new(), bindings: Vec::new() };
                let path = Path::new(vec![segment], name.span);
                Expr { kind: ExprKind::Path(path), span: name.span }
            };
            fields.push(FieldInit { name, value });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(ExprKind::Struct { path, fields, base })
    }

    /// The arguments of `name!(...)`; `name` and `!` are already consumed.
    fn parse_macro_call(&mut self, name: Ident) -> Result<ExprKind> {
        let close = match self.bump().kind {
            TokenKind::LParen => TokenKind::RParen,
            TokenKind::LBracket => TokenKind::RBracket,
            TokenKind::LBrace => TokenKind::RBrace,
            _ => bail!(name.span, "expected `(`, `[` or `{{` after `{}!`", name.name),
        };
        let args = self.with_struct_literals(true, |p| {
            if name.name == "matches" {
                let value = boxed(p.parse_expr()?);
                p.expect(&TokenKind::Comma)?;
                let pattern = Box::new(p.parse_pattern()?);
                let guard =
                    if p.eat_keyword(Keyword::If) { Some(boxed(p.parse_expr()?)) } else { None };
                p.eat(&TokenKind::Comma);
                p.expect(&close)?;
                return Ok(MacroArgs::Matches(value, pattern, guard));
            }
            if p.eat(&close) {
                return Ok(MacroArgs::List(Vec::new()));
            }
            let first = p.parse_expr()?;
            if p.eat(&TokenKind::Semi) {
                let count = boxed(p.parse_expr()?);
                p.expect(&close)?;
                return Ok(MacroArgs::Repeat(boxed(first), count));
            }
            let mut list = vec![first];
            if p.eat(&TokenKind::Comma) {
                list.extend(p.comma_separated(&close, Self::parse_expr)?);
            } else {
                p.expect(&close)?;
            }
            Ok(MacroArgs::List(list))
        })?;
        Ok(ExprKind::Macro(MacroCall { name, args }))
    }
}

enum StatementOrTail {
    Statement(Stmt),
    /// The final expression of a block, which gives the block its value.
    Tail(Expr),
}
