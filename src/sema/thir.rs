//! The typed tree: a function body after type checking.
//!
//! Compared to the AST, everything implicit has been made explicit:
//!
//! * every expression and pattern carries its [`Ty`];
//! * names are resolved to locals, functions and variants;
//! * method calls and overloaded operators are plain calls with the
//!   receiver's auto-borrow written out;
//! * `for` and `while` are loops with a `match` or `if` inside.
//!
//! This is what [`crate::ir`] turns into a control-flow graph.

use super::defs::ConstId;
use super::ty::{ClosureId, FnId, Mutability, Ty};
use crate::syntax::ast::BinOp;
use crate::syntax::span::Span;

/// A local variable (or parameter, or compiler temporary) of one function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId(pub u32);

/// Identifies a loop, as the target of `break` and `continue`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LoopId(pub u32);

/// A function together with the concrete types of its generic parameters:
/// the unit of code generation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Instance {
    pub def: FnId,
    pub substs: Vec<Ty>,
}

#[derive(Clone, Debug)]
pub struct LocalDecl {
    pub name: String,
    pub ty: Ty,
    pub mutable: bool,
}

/// The checked body of a function instance or closure.
#[derive(Clone, Debug)]
pub struct Body {
    pub locals: Vec<LocalDecl>,
    /// The parameters, in declaration order (`self` first).
    pub params: Vec<LocalId>,
    pub ret_ty: Ty,
    pub value: Expr,
}

/// One closure expression: its body plus what it captures from the
/// enclosing function.
#[derive(Clone, Debug)]
pub struct ClosureDef {
    pub body: Body,
    pub captures: Vec<Capture>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Capture {
    /// The captured variable of the enclosing function.
    pub local: LocalId,
    pub ty: Ty,
    /// `true`: the closure holds a pointer to the variable.
    /// `false` (`move` closures): the closure holds the value itself.
    pub by_ref: bool,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub expr: Option<Box<Expr>>,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let { pat: Pat, init: Option<Expr>, else_block: Option<Block> },
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(u128),
    Float(f64),
    Bool(bool),
    Char(char),
    /// A string literal; its type is `&str`.
    Str(String),
    /// A byte string literal; its type is `&[u8; N]`.
    ByteStr(Vec<u8>),
    /// A value of a zero-sized type: `()`, a function item, a unit struct.
    ZeroSized,

    Local(LocalId),
    Static(ConstId),

    Unary(UnaryOp, Box<Expr>),
    /// A built-in operator on primitive operands. `&&` and `||` short-circuit.
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// A primitive conversion from the operand's type to this expression's type.
    Cast(Box<Expr>),

    AddrOf(Mutability, Box<Expr>),
    Deref(Box<Expr>),
    /// A field of a struct or tuple, by index.
    Field(Box<Expr>, usize),
    /// An element of an array or slice; bounds-checked.
    Index(Box<Expr>, Box<Expr>),

    Call(Callee, Vec<Expr>),
    /// A struct literal or enum variant constructor.
    Adt { variant: u32, fields: Vec<(usize, Expr)>, base: Option<Box<Expr>> },
    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    Repeat(Box<Expr>, u64),
    Closure(ClosureId),

    /// Pointer-to-sized into pointer-to-unsized: `&[T; N]` to `&[T]`,
    /// `&T` to `&dyn Trait`, `Box<T>` to `Box<dyn Trait>`.
    Unsize(Box<Expr>),
    /// A function item or capture-free closure used as a function pointer.
    ReifyFnPointer(Box<Expr>),

    Block(Block),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    /// `let PATTERN = value` as a condition: tests the pattern and binds its
    /// variables when it matches.
    Let(Box<Pat>, Box<Expr>),
    Match(Box<Expr>, Vec<Arm>),
    Loop(LoopId, Block),
    Break(LoopId, Option<Box<Expr>>),
    Continue(LoopId),
    Return(Option<Box<Expr>>),

    Assign(Box<Expr>, Box<Expr>),
    /// `place op= value` on primitive operands.
    AssignOp(BinOp, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Clone, Debug)]
pub enum Callee {
    /// A known function: a direct call.
    Fn(Instance),
    /// A function pointer value: an indirect call.
    Pointer(Box<Expr>),
    /// A closure; the first argument of the call is a pointer to it.
    Closure(ClosureId),
    /// An entry of a trait object's vtable, looked up at run time. The first
    /// argument of the call is the (fat) pointer to the object.
    Virtual { slot: usize },
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pat: Pat,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Clone, Debug)]
pub struct Pat {
    pub kind: PatKind,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum PatKind {
    Wild,
    Binding { local: LocalId, mode: BindingMode, sub: Option<Box<Pat>> },
    Literal(Literal),
    /// An integer or character range; `None` bounds are open.
    Range { lo: Option<i128>, hi: Option<i128>, inclusive: bool },
    /// An enum variant with patterns for (some of) its fields.
    Variant { variant: u32, fields: Vec<(usize, Pat)> },
    /// A struct or tuple taken apart by field.
    Fields(Vec<(usize, Pat)>),
    /// Matches through a reference: the sub-pattern applies to the pointee.
    Deref(Box<Pat>),
    Or(Vec<Pat>),
    /// `[first, .., last]` on an array or slice: patterns for the first
    /// and last elements, and with `..`, one for those in between (of type
    /// `[T; K]` for an array, `[T]` for a slice).
    Slice { prefix: Vec<Pat>, rest: Option<Box<Pat>>, suffix: Vec<Pat> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingMode {
    /// The variable takes the matched value.
    Value,
    /// The variable becomes a reference to the matched place.
    Ref(Mutability),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    Int(i128),
    Bool(bool),
    Char(char),
    Str(String),
}

impl Expr {
    /// Does this expression denote a memory location (so that it can be
    /// assigned to or borrowed in place)?
    pub fn is_place(&self) -> bool {
        match &self.kind {
            ExprKind::Local(_) | ExprKind::Static(_) | ExprKind::Deref(_) => true,
            ExprKind::Field(base, _) | ExprKind::Index(base, _) => base.is_place(),
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Rewriting the types of a finished body
// ---------------------------------------------------------------------------

impl Body {
    /// Apply `f` to every type mentioned anywhere in the body. Used once
    /// inference is complete, to replace variables by their solutions.
    ///
    /// `f` is also told where in the source the type belongs, when it
    /// belongs to an expression or pattern.
    pub fn map_types(&mut self, f: &mut dyn FnMut(&Ty, Option<Span>) -> Ty) {
        self.map_types_and_calls(f, &mut |_, _| {});
    }

    /// Like [`map_types`](Self::map_types), also visiting the function
    /// every direct call calls.
    pub fn map_types_and_calls(&mut self, f: &mut dyn FnMut(&Ty, Option<Span>) -> Ty, calls: &mut dyn FnMut(&mut Instance, Span)) {
        self.value.map_types(f, calls);
        for local in &mut self.locals {
            local.ty = f(&local.ty, None);
        }
        self.ret_ty = f(&self.ret_ty, None);
    }
}

impl Block {
    fn map_types(&mut self, f: &mut dyn FnMut(&Ty, Option<Span>) -> Ty, calls: &mut dyn FnMut(&mut Instance, Span)) {
        for stmt in &mut self.stmts {
            match stmt {
                Stmt::Let { pat, init, else_block } => {
                    pat.map_types(f, calls);
                    if let Some(init) = init {
                        init.map_types(f, calls);
                    }
                    if let Some(block) = else_block {
                        block.map_types(f, calls);
                    }
                }
                Stmt::Expr(expr) => expr.map_types(f, calls),
            }
        }
        if let Some(tail) = &mut self.expr {
            tail.map_types(f, calls);
        }
    }
}

impl Expr {
    fn map_types(&mut self, f: &mut dyn FnMut(&Ty, Option<Span>) -> Ty, calls: &mut dyn FnMut(&mut Instance, Span)) {
        self.ty = f(&self.ty, Some(self.span));
        match &mut self.kind {
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::Char(_)
            | ExprKind::Str(_)
            | ExprKind::ByteStr(_)
            | ExprKind::ZeroSized
            | ExprKind::Local(_)
            | ExprKind::Static(_)
            | ExprKind::Closure(_)
            | ExprKind::Continue(_) => {}

            ExprKind::Unary(_, a)
            | ExprKind::Cast(a)
            | ExprKind::AddrOf(_, a)
            | ExprKind::Deref(a)
            | ExprKind::Field(a, _)
            | ExprKind::Repeat(a, _)
            | ExprKind::Unsize(a)
            | ExprKind::ReifyFnPointer(a) => a.map_types(f, calls),

            ExprKind::Binary(_, a, b)
            | ExprKind::Index(a, b)
            | ExprKind::Assign(a, b)
            | ExprKind::AssignOp(_, a, b) => {
                a.map_types(f, calls);
                b.map_types(f, calls);
            }

            ExprKind::Call(callee, args) => {
                match callee {
                    Callee::Fn(instance) => {
                        for ty in &mut instance.substs {
                            *ty = f(ty, Some(self.span));
                        }
                        calls(instance, self.span);
                    }
                    Callee::Pointer(pointer) => pointer.map_types(f, calls),
                    Callee::Closure(_) | Callee::Virtual { .. } => {}
                }
                args.iter_mut().for_each(|arg| arg.map_types(f, calls));
            }
            ExprKind::Adt { fields, base, .. } => {
                fields.iter_mut().for_each(|(_, value)| value.map_types(f, calls));
                if let Some(base) = base {
                    base.map_types(f, calls);
                }
            }
            ExprKind::Tuple(elements) | ExprKind::Array(elements) => {
                elements.iter_mut().for_each(|element| element.map_types(f, calls));
            }

            ExprKind::Block(block) | ExprKind::Loop(_, block) => block.map_types(f, calls),
            ExprKind::If(cond, then_expr, else_expr) => {
                cond.map_types(f, calls);
                then_expr.map_types(f, calls);
                if let Some(else_expr) = else_expr {
                    else_expr.map_types(f, calls);
                }
            }
            ExprKind::Let(pat, value) => {
                pat.map_types(f, calls);
                value.map_types(f, calls);
            }
            ExprKind::Match(scrutinee, arms) => {
                scrutinee.map_types(f, calls);
                for arm in arms {
                    arm.pat.map_types(f, calls);
                    if let Some(guard) = &mut arm.guard {
                        guard.map_types(f, calls);
                    }
                    arm.body.map_types(f, calls);
                }
            }
            ExprKind::Break(_, value) | ExprKind::Return(value) => {
                if let Some(value) = value {
                    value.map_types(f, calls);
                }
            }
        }
    }
}

impl Pat {
    fn map_types(&mut self, f: &mut dyn FnMut(&Ty, Option<Span>) -> Ty, calls: &mut dyn FnMut(&mut Instance, Span)) {
        self.ty = f(&self.ty, Some(self.span));
        match &mut self.kind {
            PatKind::Wild | PatKind::Literal(_) | PatKind::Range { .. } => {}
            PatKind::Binding { sub, .. } => {
                if let Some(sub) = sub {
                    sub.map_types(f, calls);
                }
            }
            PatKind::Variant { fields, .. } | PatKind::Fields(fields) => {
                fields.iter_mut().for_each(|(_, pat)| pat.map_types(f, calls));
            }
            PatKind::Deref(inner) => inner.map_types(f, calls),
            PatKind::Or(alternatives) => alternatives.iter_mut().for_each(|pat| pat.map_types(f, calls)),
            PatKind::Slice { prefix, rest, suffix } => {
                prefix.iter_mut().chain(rest.as_deref_mut()).chain(suffix).for_each(|pat| pat.map_types(f, calls));
            }
        }
    }
}
