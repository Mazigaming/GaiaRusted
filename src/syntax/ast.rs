//! The abstract syntax tree.
//!
//! This is a faithful picture of the source: nothing is resolved, inferred or
//! desugared here. Every node carries the [`Span`] it was parsed from.

use super::span::Span;

#[derive(Clone, Debug)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// `a::b::<T>::c` — a possibly generic, possibly qualified name.
#[derive(Clone, Debug)]
pub struct Path {
    /// The `<Type as Trait>` in front of a qualified path; the segments
    /// then name an item of that type.
    pub qself: Option<Box<QSelf>>,
    pub segments: Vec<PathSegment>,
    pub span: Span,
}

/// `<Type as Trait>`, or `<Type>` alone, at the start of a path.
#[derive(Clone, Debug)]
pub struct QSelf {
    pub ty: Type,
    pub trait_ref: Option<Path>,
}

#[derive(Clone, Debug)]
pub struct PathSegment {
    pub ident: Ident,
    /// Type arguments: the `i64` in `Vec<i64>` or `parse::<i64>`.
    pub args: Vec<Type>,
    /// Associated type bindings: the `Item = i64` in `Iterator<Item = i64>`.
    pub bindings: Vec<(Ident, Type)>,
}

impl Path {
    /// A path of plain segments, as written without a qualified prefix.
    pub fn new(segments: Vec<PathSegment>, span: Span) -> Path {
        Path { qself: None, segments, span }
    }

    /// The name, if this path is a single segment without type arguments.
    pub fn as_ident(&self) -> Option<&Ident> {
        match self.segments.as_slice() {
            [only] if only.args.is_empty() && self.qself.is_none() => Some(&only.ident),
            _ => None,
        }
    }

    pub fn last(&self) -> &PathSegment {
        self.segments.last().expect("a path has at least one segment")
    }
}

// ---------------------------------------------------------------------------
// Items
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Item {
    pub kind: ItemKind,
    pub attrs: Vec<Attribute>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ItemKind {
    Fn(Function),
    Struct(StructDef),
    Enum(EnumDef),
    Trait(TraitDef),
    Impl(ImplBlock),
    Const(ConstDef),
    Static(ConstDef),
    TypeAlias(TypeAlias),
    Mod(Module),
    Use(UseTree),
    /// `extern "C" { fn ...; }`
    ExternBlock(Vec<Function>),
}

/// `#[name(arg, arg)]`
#[derive(Clone, Debug)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Module {
    pub name: Ident,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, Default)]
pub struct Generics {
    pub params: Vec<GenericParam>,
    /// `where` clauses: each bounded type with its bounds.
    pub where_clauses: Vec<(Type, Vec<Bound>)>,
}

#[derive(Clone, Debug)]
pub struct GenericParam {
    pub name: Ident,
    pub bounds: Vec<Bound>,
    /// The type of a const parameter: the `usize` of `const N: usize`.
    pub const_ty: Option<Type>,
    /// Declared `?Sized`: the parameter may stand for `str`, `[T]` or a
    /// trait object. Every other one is implicitly `Sized`.
    pub maybe_unsized: bool,
}

/// A trait bound such as `Clone`, `Iterator<Item = T>` or `Fn(i64) -> i64`.
#[derive(Clone, Debug)]
pub struct Bound {
    pub path: Path,
    /// The parenthesised form used by the closure traits: parameter types and
    /// return type of `Fn(A, B) -> R`.
    pub fn_sugar: Option<(Vec<Type>, Option<Box<Type>>)>,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: Ident,
    pub generics: Generics,
    pub self_param: Option<SelfParam>,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    /// `None` for trait methods without a default and for extern declarations.
    pub body: Option<Block>,
    pub is_unsafe: bool,
    /// Declared in an `extern "C"` block, or defined with `extern "C" fn`.
    pub is_extern_c: bool,
    /// Extern declaration ending in `...`.
    pub is_variadic: bool,
    /// Declared `pub` in an impl block; methods of trait impls are as
    /// visible as the trait.
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct SelfParam {
    pub kind: SelfKind,
    /// `mut self`
    pub mutable: bool,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelfKind {
    /// `self`
    Value,
    /// `&self`
    Ref,
    /// `&mut self`
    RefMut,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub pat: Pattern,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct StructDef {
    pub name: Ident,
    pub generics: Generics,
    pub fields: VariantFields,
    /// For each field, whether it is declared `pub` (in any form).
    pub public_fields: Vec<bool>,
}

#[derive(Clone, Debug)]
pub struct EnumDef {
    pub name: Ident,
    pub generics: Generics,
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub name: Ident,
    pub fields: VariantFields,
    /// Explicit discriminant: `Red = 3`.
    pub discriminant: Option<Expr>,
    /// Marked `#[default]`: what a derived `Default` gives.
    pub is_default: bool,
}

/// The three shapes a struct or an enum variant can take.
#[derive(Clone, Debug)]
pub enum VariantFields {
    /// `struct Marker;`
    Unit,
    /// `struct Pair(i32, i32);`
    Tuple(Vec<Type>),
    /// `struct Point { x: i32, y: i32 }`
    Named(Vec<FieldDef>),
}

#[derive(Clone, Debug)]
pub struct FieldDef {
    pub name: Ident,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct TraitDef {
    pub name: Ident,
    pub generics: Generics,
    pub supertraits: Vec<Bound>,
    pub items: Vec<AssocItem>,
}

#[derive(Clone, Debug)]
pub struct ImplBlock {
    pub generics: Generics,
    /// `Some` for `impl Trait for Type`, `None` for an inherent `impl Type`.
    pub trait_ref: Option<Path>,
    pub self_ty: Type,
    pub items: Vec<AssocItem>,
}

/// Something declared inside a `trait` or `impl` block.
#[derive(Clone, Debug)]
pub enum AssocItem {
    Fn(Function),
    /// `type Item;` in a trait, `type Item = i64;` in an impl.
    Type { name: Ident, ty: Option<Type> },
    Const(ConstDef),
}

/// A `const` or `static` item (also an associated constant).
#[derive(Clone, Debug)]
pub struct ConstDef {
    pub name: Ident,
    pub ty: Type,
    /// `None` only for a trait's constant without a default.
    pub value: Option<Expr>,
    pub mutable: bool,
}

#[derive(Clone, Debug)]
pub struct TypeAlias {
    pub name: Ident,
    pub generics: Generics,
    pub ty: Type,
}

/// `use a::b::{c, d as e, f::*};`
#[derive(Clone, Debug)]
pub struct UseTree {
    pub prefix: Vec<Ident>,
    pub kind: UseKind,
}

#[derive(Clone, Debug)]
pub enum UseKind {
    /// The prefix names the imported item; optionally renamed with `as`.
    Simple(Option<Ident>),
    Glob,
    Nested(Vec<UseTree>),
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TypeKind {
    /// `i32`, `Vec<T>`, `Self`, `T::Item`, `module::Name`
    Path(Path),
    Ref { mutable: bool, inner: Box<Type> },
    Ptr { mutable: bool, inner: Box<Type> },
    /// `(A, B)`; the unit type is the empty tuple.
    Tuple(Vec<Type>),
    Array(Box<Type>, Box<Expr>),
    Slice(Box<Type>),
    Fn { params: Vec<Type>, ret: Option<Box<Type>> },
    Dyn(Vec<Bound>),
    ImplTrait(Vec<Bound>),
    Never,
    /// `_`
    Infer,
    /// A constant given as a generic argument: the `3` of `Buffer<3>`, or
    /// `{ N + 1 }`. A bare name such as `N` parses as a path.
    Const(Box<Expr>),
}

// ---------------------------------------------------------------------------
// Statements and expressions
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// The trailing expression whose value the block evaluates to.
    pub expr: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Let {
        pat: Pattern,
        ty: Option<Type>,
        init: Option<Expr>,
        /// The diverging block of `let PATTERN = init else { ... };`
        else_block: Option<Block>,
    },
    /// An expression evaluated for its effect; its value is discarded.
    Expr(Expr),
    Item(Box<Item>),
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(u128, Option<String>),
    Float(f64, Option<String>),
    Bool(bool),
    Char(char),
    Str(String),
    Byte(u8),
    ByteStr(Vec<u8>),

    Path(Path),
    Unary(UnOp, Box<Expr>),
    /// `&expr` / `&mut expr`
    AddrOf { mutable: bool, expr: Box<Expr> },
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Assign(Box<Expr>, Box<Expr>),
    /// `a += b`
    AssignOp(BinOp, Box<Expr>, Box<Expr>),
    Cast(Box<Expr>, Type),

    Call(Box<Expr>, Vec<Expr>),
    MethodCall { receiver: Box<Expr>, method: Ident, turbofish: Vec<Type>, args: Vec<Expr> },
    Field(Box<Expr>, Ident),
    /// `pair.0`
    TupleField(Box<Expr>, u32),
    Index(Box<Expr>, Box<Expr>),

    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    /// `[value; count]`
    Repeat(Box<Expr>, Box<Expr>),
    Struct { path: Path, fields: Vec<FieldInit>, base: Option<Box<Expr>> },
    Range { lo: Option<Box<Expr>>, hi: Option<Box<Expr>>, inclusive: bool },

    Block(Block),
    Unsafe(Block),
    If { cond: Box<Expr>, then_block: Block, else_expr: Option<Box<Expr>> },
    /// `let PATTERN = expr`, valid only as (part of) an `if` / `while` condition.
    Let(Box<Pattern>, Box<Expr>),
    While { label: Option<String>, cond: Box<Expr>, body: Block },
    Loop { label: Option<String>, body: Block },
    /// `'label: { ... }`, which `break 'label value` leaves early.
    LabeledBlock { label: String, body: Block },
    For { label: Option<String>, pat: Box<Pattern>, iter: Box<Expr>, body: Block },
    Match { scrutinee: Box<Expr>, arms: Vec<Arm> },
    Closure { params: Vec<ClosureParam>, ret: Option<Type>, body: Box<Expr>, is_move: bool },

    Return(Option<Box<Expr>>),
    Break { label: Option<String>, value: Option<Box<Expr>> },
    Continue { label: Option<String> },
    /// `expr?`
    Try(Box<Expr>),

    Macro(MacroCall),
    /// `_`, which may only stand on the left of an assignment:
    /// `(_, rest) = pair`.
    Underscore,
}

#[derive(Clone, Debug)]
pub struct FieldInit {
    pub name: Ident,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pat: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Clone, Debug)]
pub struct ClosureParam {
    pub pat: Pattern,
    pub ty: Option<Type>,
}

/// An invocation of one of the built-in macros (`println!`, `vec!`, ...).
#[derive(Clone, Debug)]
pub struct MacroCall {
    pub name: Ident,
    pub args: MacroArgs,
}

#[derive(Clone, Debug)]
pub enum MacroArgs {
    /// Comma-separated expressions: `println!("{}", x)`, `vec![1, 2]`.
    List(Vec<Expr>),
    /// `vec![value; count]`
    Repeat(Box<Expr>, Box<Expr>),
    /// `matches!(value, pattern if guard)`
    Matches(Box<Expr>, Box<Pattern>, Option<Box<Expr>>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    Deref,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// Short-circuiting `&&`
    And,
    /// Short-circuiting `||`
    Or,
}

impl BinOp {
    pub fn is_comparison(self) -> bool {
        matches!(self, BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge)
    }

    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        }
    }
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum PatternKind {
    /// `_`
    Wild,
    /// `x`, `mut x`, `ref x`, `x @ subpattern`.
    ///
    /// A bare name may also turn out to be a unit variant or a constant
    /// (`None`, `MAX`); name resolution decides.
    Binding { name: Ident, mutable: bool, by_ref: Option<bool>, sub: Option<Box<Pattern>> },
    /// A literal, possibly negated: `1`, `-1`, `"text"`, `true`.
    Literal(Box<Expr>),
    Range { lo: Option<Box<Expr>>, hi: Option<Box<Expr>>, inclusive: bool },
    /// A qualified unit variant or constant: `Color::Red`.
    Path(Path),
    /// `Some(x)`, `Shape::Rect(w, h)`
    TupleStruct(Path, Vec<Pattern>),
    /// `Point { x, y: 0, .. }`
    Struct { path: Path, fields: Vec<(Ident, Pattern)>, has_rest: bool },
    Tuple(Vec<Pattern>),
    Slice(Vec<Pattern>),
    /// `&pattern` / `&mut pattern`
    Ref(Box<Pattern>),
    /// `a | b`
    Or(Vec<Pattern>),
    /// `..` inside a tuple, tuple-struct or slice pattern.
    Rest,
}
