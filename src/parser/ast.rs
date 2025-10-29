//! Abstract Syntax Tree (AST) Definitions
//!
//! The AST is a tree representation of the program's structure.
//! It removes syntactic details but preserves the semantic meaning.

use std::fmt;

/// A complete Rust program is a list of items (functions, structs, etc.)
pub type Program = Vec<Item>;

/// Top-level items in a Rust program
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// Function definition: `fn name(params) -> ReturnType { body }`
    Function {
        name: String,
        params: Vec<Parameter>,
        return_type: Option<Type>,
        body: Block,
    },
    /// Struct definition: `struct Name { field: Type, ... }`
    Struct {
        name: String,
        fields: Vec<StructField>,
    },
    /// Enum definition: `enum Name { Variant1, Variant2(Type), ... }`
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
    /// Trait definition: `trait Name { ... }`
    Trait {
        name: String,
        methods: Vec<Item>,
    },
    /// Impl block: `impl TraitName for StructName { ... }` or `impl StructName { ... }`
    Impl {
        trait_name: Option<String>,
        struct_name: String,
        methods: Vec<Item>,
    },
    /// Module definition: `mod name { ... }`
    Module {
        name: String,
        items: Vec<Item>,
    },
    /// Use statement: `use path::to::item;`
    Use {
        path: String,
    },
}

/// Function parameter: `name: Type` or `mut name: Type`
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub mutable: bool,
    pub ty: Type,
}

/// A struct field: `name: Type`
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
}

/// An enum variant: `Name`, `Name(Type)`, or `Name { field: Type }`
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariant {
    Unit(String),
    Tuple(String, Vec<Type>),
    Struct(String, Vec<StructField>),
}

/// A block of statements: `{ stmt1; stmt2; ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub expression: Option<Box<Expression>>,  // Last expression (return value)
}

/// A statement is an instruction that doesn't return a value (usually ends with ;)
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Variable declaration: `let x = 5;` or `let mut y: i32 = 10;`
    Let {
        name: String,
        mutable: bool,
        ty: Option<Type>,
        initializer: Expression,
    },
    /// Expression statement: `x + 1;`
    Expression(Expression),
    /// Return statement: `return value;` or just `return;`
    Return(Option<Box<Expression>>),
    /// Break statement: `break;` (in loops)
    Break,
    /// Continue statement: `continue;` (in loops)
    Continue,
    /// For loop statement: `for x in iter { ... }`
    For {
        var: String,
        iter: Box<Expression>,
        body: Block,
    },
    /// While loop statement: `while condition { ... }`
    While {
        condition: Box<Expression>,
        body: Block,
    },
    /// If statement: `if condition { ... } else { ... }`
    If {
        condition: Box<Expression>,
        then_body: Block,
        else_body: Option<Box<Statement>>, // Can be another If or else block
    },
}

/// An expression returns a value
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),

    // Variables and function calls
    Variable(String),
    FunctionCall {
        name: String,
        args: Vec<Expression>,
    },

    // Binary operations: `a + b`, `x && y`, etc.
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },

    // Unary operations: `-x`, `!flag`, `*ptr`
    Unary {
        op: UnaryOp,
        operand: Box<Expression>,
    },

    // Assignment: `x = 5`
    Assign {
        target: Box<Expression>,
        value: Box<Expression>,
    },

    // Compound assignment: `x += 5`
    CompoundAssign {
        target: Box<Expression>,
        op: CompoundOp,
        value: Box<Expression>,
    },

    // If expression: `if cond { ... } else { ... }`
    If {
        condition: Box<Expression>,
        then_body: Block,
        else_body: Option<Box<Expression>>, // Can be another If or a Block
    },

    // Match expression: `match value { pattern => expr, ... }`
    Match {
        scrutinee: Box<Expression>,
        arms: Vec<MatchArm>,
    },

    // Loop: `loop { ... }`
    Loop(Block),

    // While loop: `while condition { ... }`
    While {
        condition: Box<Expression>,
        body: Block,
    },

    // Block: `{ statements; value }`
    Block(Block),

    // Struct literal: `Point { x: 1, y: 2 }`
    StructLiteral {
        struct_name: String,
        fields: Vec<(String, Expression)>,
    },

    // Field access: `point.x`
    FieldAccess {
        object: Box<Expression>,
        field: String,
    },

    // Array: `[1, 2, 3]`
    Array(Vec<Expression>),

    // Array indexing: `arr[i]`
    Index {
        array: Box<Expression>,
        index: Box<Expression>,
    },

    // Range expressions: `1..5`, `1..=5`, `1..`, `..5`
    Range {
        start: Option<Box<Expression>>,
        end: Option<Box<Expression>>,
        inclusive: bool,
    },

    // Tuple: `(a, b, c)` or `(a,)` for single element
    Tuple(Vec<Expression>),

    // For loop: `for x in iter { ... }`
    For {
        var: String,
        iter: Box<Expression>,
        body: Block,
    },

    // Closure/Lambda: `|x, y| x + y`
    Closure {
        params: Vec<String>,
        body: Box<Expression>,
    },
}

/// Match arm: `pattern => expression`
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expression>>, // `if condition`
    pub body: Expression,
}

/// Patterns for destructuring in match and let bindings
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard: `_`
    Wildcard,
    /// Literal: `42`, `"hello"`, `true`
    Literal(Expression),
    /// Identifier: `x` (binds the value to x)
    Identifier(String),
    /// Tuple pattern: `(x, y)`
    Tuple(Vec<Pattern>),
    /// Struct pattern: `Point { x, y }`
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
    },
}

/// Types in Rust
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Basic types: i32, i64, f64, bool, char, str
    Named(String),
    /// Reference: `&T` or `&mut T`
    Reference {
        mutable: bool,
        inner: Box<Type>,
    },
    /// Pointer: `*T` or `*mut T` (for unsafe)
    Pointer {
        mutable: bool,
        inner: Box<Type>,
    },
    /// Array: `[T; size]`
    Array {
        element: Box<Type>,
        size: Option<Box<Expression>>, // None means slice
    },
    /// Function type: `fn(T1, T2) -> R`
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    /// Tuple type: `(T1, T2)`
    Tuple(Vec<Type>),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Named(name) => write!(f, "{}", name),
            Type::Reference { mutable, inner } => {
                write!(f, "&{}{}", if *mutable { "mut " } else { "" }, inner)
            }
            Type::Pointer { mutable, inner } => {
                write!(f, "*{}{}", if *mutable { "mut " } else { "const " }, inner)
            }
            Type::Array { element, size } => {
                if let Some(_sz) = size {
                    write!(f, "[{:?}; ...]", element)
                } else {
                    write!(f, "[{}]", element)
                }
            }
            Type::Function { params, return_type } => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", return_type)
            }
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Binary operators: `+`, `-`, `*`, `/`, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,        // +
    Subtract,   // -
    Multiply,   // *
    Divide,     // /
    Modulo,     // %

    // Comparison
    Equal,      // ==
    NotEqual,   // !=
    Less,       // <
    LessEq,     // <=
    Greater,    // >
    GreaterEq,  // >=

    // Logical
    And,        // &&
    Or,         // ||

    // Bitwise
    BitwiseAnd, // &
    BitwiseOr,  // |
    BitwiseXor, // ^
    LeftShift,  // <<
    RightShift, // >>
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Subtract => write!(f, "-"),
            BinaryOp::Multiply => write!(f, "*"),
            BinaryOp::Divide => write!(f, "/"),
            BinaryOp::Modulo => write!(f, "%"),
            BinaryOp::Equal => write!(f, "=="),
            BinaryOp::NotEqual => write!(f, "!="),
            BinaryOp::Less => write!(f, "<"),
            BinaryOp::LessEq => write!(f, "<="),
            BinaryOp::Greater => write!(f, ">"),
            BinaryOp::GreaterEq => write!(f, ">="),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Or => write!(f, "||"),
            BinaryOp::BitwiseAnd => write!(f, "&"),
            BinaryOp::BitwiseOr => write!(f, "|"),
            BinaryOp::BitwiseXor => write!(f, "^"),
            BinaryOp::LeftShift => write!(f, "<<"),
            BinaryOp::RightShift => write!(f, ">>"),
        }
    }
}

/// Unary operators: `-x`, `!flag`, `*ptr`, `&var`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,      // -
    Not,         // !
    BitwiseNot,  // ~
    Dereference, // *
    Reference,   // & (borrows)
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnaryOp::Negate => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
            UnaryOp::BitwiseNot => write!(f, "~"),
            UnaryOp::Dereference => write!(f, "*"),
            UnaryOp::Reference => write!(f, "&"),
        }
    }
}

/// Compound assignment operators: `+=`, `-=`, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompoundOp {
    AddAssign,       // +=
    SubtractAssign,  // -=
    MultiplyAssign,  // *=
    DivideAssign,    // /=
    ModuloAssign,    // %=
    AndAssign,       // &=
    OrAssign,        // |=
    XorAssign,       // ^=
    LeftShiftAssign, // <<=
    RightShiftAssign,// >>=
}

impl fmt::Display for CompoundOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompoundOp::AddAssign => write!(f, "+="),
            CompoundOp::SubtractAssign => write!(f, "-="),
            CompoundOp::MultiplyAssign => write!(f, "*="),
            CompoundOp::DivideAssign => write!(f, "/="),
            CompoundOp::ModuloAssign => write!(f, "%="),
            CompoundOp::AndAssign => write!(f, "&="),
            CompoundOp::OrAssign => write!(f, "|="),
            CompoundOp::XorAssign => write!(f, "^="),
            CompoundOp::LeftShiftAssign => write!(f, "<<="),
            CompoundOp::RightShiftAssign => write!(f, ">>="),
        }
    }
}