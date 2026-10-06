//! The intermediate representation: a control-flow graph over typed locals.
//!
//! ```text
//! typed tree ──build──▶ IR ──(optimisation passes)──▶ IR ──x64──▶ assembly
//! ```
//!
//! A function is a list of basic blocks. A block is straight-line
//! [`Statement`]s ending in one [`Terminator`] that says where control goes
//! next. Values live in [`Local`]s, each with a concrete type; a [`Place`]
//! names a local or part of one (a field, an element, what a pointer points
//! to).
//!
//! By this point nothing is generic, nothing is implicit and there is no
//! nesting: `a + b * c` is two statements, a `match` is branches and jumps,
//! a method call is a call of a named symbol. That makes this the level at
//! which the program is analysed, optimised and finally turned into machine
//! code.

pub mod analysis;
pub mod build;
pub mod display;
pub mod layout;
pub mod opt;
pub mod visit;

use crate::sema::ty::Ty;
use crate::syntax::ast::BinOp;

/// Index of a local within its function. Local 0 holds the return value;
/// locals `1..=arg_count` are the parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Local(pub u32);

pub const RETURN_LOCAL: Local = Local(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u32);

/// Index of a function within the [`Program`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuncId(pub u32);

/// Index of a constant data blob (string literal, vtable) in the [`Program`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DataId(pub u32);

pub struct Program {
    pub functions: Vec<Function>,
    pub data: Vec<Data>,
    /// The user's `main`.
    pub entry: FuncId,
    /// Functions that compute the statics that are not plain data; they
    /// run before `main`, in this order.
    pub initializers: Vec<FuncId>,
}

/// Read-only (or, for `static mut`, writable) data emitted alongside the code.
pub struct Data {
    pub symbol: String,
    pub items: Vec<DataItem>,
    pub align: u64,
    pub writable: bool,
}

#[derive(Clone)]
pub enum DataItem {
    Bytes(Vec<u8>),
    /// An 8-byte integer.
    Word(u64),
    /// The address of a function.
    FuncAddr(FuncId),
    /// The address of another data blob.
    DataAddr(DataId),
}

pub struct Function {
    /// The linker symbol.
    pub symbol: String,
    /// A readable name for dumps and diagnostics: `Vec<i64>::push`.
    pub name: String,
    pub locals: Vec<LocalDecl>,
    pub arg_count: usize,
    pub blocks: Vec<Block>,
}

impl Function {
    pub fn ret_ty(&self) -> &Ty {
        &self.locals[0].ty
    }

    pub fn args(&self) -> impl Iterator<Item = Local> {
        (1..=self.arg_count as u32).map(Local)
    }

    /// Does this function call something that returns twice? Such a
    /// function stays out of line and keeps its locals in memory: on the
    /// second return, registers hold what they held at the first.
    pub fn calls_returns_twice(&self) -> bool {
        self.blocks.iter().flat_map(|block| &block.statements).any(|statement| match statement {
            Statement::Call { callee, .. } => callee.returns_twice(),
            _ => false,
        })
    }
}

#[derive(Clone)]
pub struct LocalDecl {
    pub ty: Ty,
    /// The source variable this local stands for, if any.
    pub name: Option<String>,
}

#[derive(Clone, Default)]
pub struct Block {
    pub statements: Vec<Statement>,
    /// `None` only while the block is under construction.
    pub terminator: Option<Terminator>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Place {
    pub local: Local,
    pub projection: Vec<Projection>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Projection {
    /// A field of a struct, tuple or closure environment, or of the enum
    /// variant selected by a preceding [`Projection::Downcast`].
    Field(usize),
    /// What a reference or pointer points to.
    Deref,
    /// An element of an array or slice; the index is a `usize` local.
    Index(Local),
    /// View an enum as one specific variant.
    Downcast(u32),
}

impl Place {
    pub fn local(local: Local) -> Place {
        Place { local, projection: Vec::new() }
    }

    pub fn project(mut self, projection: Projection) -> Place {
        self.projection.push(projection);
        self
    }

    pub fn field(self, index: usize) -> Place {
        self.project(Projection::Field(index))
    }

    pub fn deref(self) -> Place {
        self.project(Projection::Deref)
    }

    pub fn as_local(&self) -> Option<Local> {
        self.projection.is_empty().then_some(self.local)
    }
}

impl From<Local> for Place {
    fn from(local: Local) -> Place {
        Place::local(local)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operand {
    /// The current value of a place.
    Copy(Place),
    Const(Const),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Const {
    /// An integer, `bool` or `char`, as the bits of its type.
    Int(u128, Ty),
    Float(f64, Ty),
    /// A `&str` pointing at constant data.
    Str(DataId, u64),
    /// The address of constant data.
    DataAddr(DataId, Ty),
    /// The address of a function.
    FuncAddr(FuncId, Ty),
    /// The only value of a zero-sized type.
    ZeroSized(Ty),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Rvalue {
    Use(Operand),
    /// Arithmetic, bitwise and comparison operators on primitives. Never
    /// `&&` / `||`: those are control flow.
    Binary(BinOp, Operand, Operand),
    Unary(UnaryOp, Operand),
    /// A primitive conversion between the two types.
    Cast(Operand, Ty, Ty),
    /// The address of a place.
    AddrOf(Place),
    /// The tag saying which variant an enum value holds.
    Discriminant(Place),
    /// Build a two-word pointer from a data pointer and its extra word
    /// (a slice length or a vtable address).
    MakeFat(Operand, Operand),
    /// The data pointer of a two-word pointer.
    FatData(Operand),
    /// The extra word of a two-word pointer.
    FatExtra(Operand),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    /// Square root of a float.
    Sqrt,
}

#[derive(Clone, Debug)]
pub enum Statement {
    Assign(Place, Rvalue),
    /// Record which variant an enum value holds.
    SetDiscriminant(Place, u32),
    /// Arguments are passed by value. One that does not travel in
    /// registers is handed over, memory and all: it belongs to the callee
    /// for the duration of the call, which may change it in place, and the
    /// caller does not look at it again.
    Call { dest: Place, callee: Callee, args: Vec<Operand> },
}

#[derive(Clone, Debug)]
pub enum Callee {
    /// A function of this program.
    Direct(FuncId),
    /// A foreign function, by its C symbol.
    Extern { symbol: String, variadic: bool },
    /// Through a function pointer.
    Indirect(Operand),
    /// Through word `index` of the vtable of the trait object that is the
    /// call's first argument.
    Virtual { index: usize },
}

impl Callee {
    /// Whether the call can return a second time, when a later jump goes
    /// back to it (`setjmp` and its relatives).
    pub fn returns_twice(&self) -> bool {
        matches!(self, Callee::Extern { symbol, .. }
            if matches!(symbol.as_str(), "setjmp" | "_setjmp" | "sigsetjmp" | "__sigsetjmp" | "vfork"))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Terminator {
    Goto(BlockId),
    /// Two-way branch on a `bool`.
    Branch { cond: Operand, then_block: BlockId, else_block: BlockId },
    /// Multi-way branch on an integer.
    Switch { value: Operand, arms: Vec<(u128, BlockId)>, otherwise: BlockId },
    Return,
    /// Control can never get here. Reaching it anyway is a compiler bug or
    /// undefined behaviour in `unsafe` code; the program is stopped.
    Unreachable,
}

impl Terminator {
    pub fn successors(&self) -> Vec<BlockId> {
        match self {
            Terminator::Goto(target) => vec![*target],
            Terminator::Branch { then_block, else_block, .. } => vec![*then_block, *else_block],
            Terminator::Switch { arms, otherwise, .. } => {
                arms.iter().map(|(_, block)| *block).chain([*otherwise]).collect()
            }
            Terminator::Return | Terminator::Unreachable => Vec::new(),
        }
    }
}

/// The vtable word holding the function that drops the object in place.
pub const VTABLE_DROP: usize = 2;
/// How many vtable words precede the method pointers: size, alignment and
/// the drop function.
pub const VTABLE_HEADER_WORDS: usize = 3;
