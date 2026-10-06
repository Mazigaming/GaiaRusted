//! Constructing AST by hand, for code the compiler generates (macro
//! expansions). Every node gets the span of the construct it came from, so
//! errors inside generated code still point somewhere meaningful.

use super::ast::*;
use super::span::Span;

#[derive(Clone, Copy)]
pub struct AstBuilder {
    pub span: Span,
}

impl AstBuilder {
    pub fn new(span: Span) -> AstBuilder {
        AstBuilder { span }
    }

    fn expr(&self, kind: ExprKind) -> Expr {
        Expr { kind, span: self.span }
    }

    pub fn ident(&self, name: &str) -> Ident {
        Ident { name: name.to_string(), span: self.span }
    }

    /// `a::b::c` as a path (not yet an expression).
    pub fn path_of(&self, segments: &[&str]) -> Path {
        let segments = segments
            .iter()
            .map(|name| PathSegment { ident: self.ident(name), args: Vec::new(), bindings: Vec::new() })
            .collect();
        Path::new(segments, self.span)
    }

    pub fn path(&self, segments: &[&str]) -> Expr {
        self.expr(ExprKind::Path(self.path_of(segments)))
    }

    pub fn var(&self, name: &str) -> Expr {
        self.path(&[name])
    }

    pub fn str(&self, text: &str) -> Expr {
        self.expr(ExprKind::Str(text.to_string()))
    }

    pub fn int(&self, value: u128, suffix: &str) -> Expr {
        self.expr(ExprKind::Int(value, Some(suffix.to_string())))
    }

    pub fn char(&self, value: char) -> Expr {
        self.expr(ExprKind::Char(value))
    }

    pub fn bool(&self, value: bool) -> Expr {
        self.expr(ExprKind::Bool(value))
    }

    pub fn unit(&self) -> Expr {
        self.expr(ExprKind::Tuple(Vec::new()))
    }

    pub fn call(&self, callee: Expr, args: Vec<Expr>) -> Expr {
        self.expr(ExprKind::Call(Box::new(callee), args))
    }

    /// `a::b::f(args)`
    pub fn call_path(&self, segments: &[&str], args: Vec<Expr>) -> Expr {
        self.call(self.path(segments), args)
    }

    pub fn method(&self, receiver: Expr, name: &str, args: Vec<Expr>) -> Expr {
        self.expr(ExprKind::MethodCall {
            receiver: Box::new(receiver),
            method: self.ident(name),
            turbofish: Vec::new(),
            args,
        })
    }

    pub fn addr_of(&self, operand: Expr) -> Expr {
        self.expr(ExprKind::AddrOf { mutable: false, expr: Box::new(operand) })
    }

    pub fn addr_of_mut(&self, operand: Expr) -> Expr {
        self.expr(ExprKind::AddrOf { mutable: true, expr: Box::new(operand) })
    }

    pub fn deref(&self, operand: Expr) -> Expr {
        self.expr(ExprKind::Unary(UnOp::Deref, Box::new(operand)))
    }

    pub fn not(&self, operand: Expr) -> Expr {
        self.expr(ExprKind::Unary(UnOp::Not, Box::new(operand)))
    }

    pub fn binary(&self, op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
        self.expr(ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)))
    }

    /// `let name = init;` / `let mut name = init;`
    pub fn let_(&self, name: &str, mutable: bool, init: Expr) -> Stmt {
        let kind = PatternKind::Binding { name: self.ident(name), mutable, by_ref: None, sub: None };
        let pat = Pattern { kind, span: self.span };
        Stmt { kind: StmtKind::Let { pat, ty: None, init: Some(init), else_block: None }, span: self.span }
    }

    pub fn stmt(&self, expr: Expr) -> Stmt {
        Stmt { kind: StmtKind::Expr(expr), span: self.span }
    }

    pub fn block_of(&self, stmts: Vec<Stmt>, tail: Option<Expr>) -> Block {
        Block { stmts, expr: tail.map(Box::new), span: self.span }
    }

    pub fn block(&self, stmts: Vec<Stmt>, tail: Option<Expr>) -> Expr {
        self.expr(ExprKind::Block(self.block_of(stmts, tail)))
    }

    /// `match (v0, v1, ..) { (n0, n1, ..) => body }`: `body` with each name
    /// bound to its value. Unlike `let`s, this keeps the temporaries the
    /// values borrow from alive until the end of the enclosing statement.
    pub fn bind(&self, bindings: Vec<(String, Expr)>, body: Expr) -> Expr {
        if bindings.is_empty() {
            return body;
        }
        let (names, values): (Vec<String>, Vec<Expr>) = bindings.into_iter().unzip();
        let binding = |name: &String| Pattern {
            kind: PatternKind::Binding { name: self.ident(name), mutable: false, by_ref: None, sub: None },
            span: self.span,
        };
        let pat = Pattern { kind: PatternKind::Tuple(names.iter().map(binding).collect()), span: self.span };
        let scrutinee = self.expr(ExprKind::Tuple(values));
        self.expr(ExprKind::Match { scrutinee: Box::new(scrutinee), arms: vec![Arm { pat, guard: None, body }] })
    }

    /// `if cond { then }`
    pub fn if_(&self, cond: Expr, then: Vec<Stmt>) -> Expr {
        self.expr(ExprKind::If {
            cond: Box::new(cond),
            then_block: self.block_of(then, None),
            else_expr: None,
        })
    }
}
