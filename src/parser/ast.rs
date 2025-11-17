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
        generics: Vec<GenericParam>,
        params: Vec<Parameter>,
        return_type: Option<Type>,
        body: Block,
        where_clause: Vec<WhereConstraint>,
        is_unsafe: bool,
        is_async: bool,
        is_pub: bool,
        attributes: Vec<Attribute>,
        abi: Option<String>,
    },
    /// Struct definition: `struct Name { field: Type, ... }`
    Struct {
        name: String,
        generics: Vec<GenericParam>,
        fields: Vec<StructField>,
        where_clause: Vec<WhereConstraint>,
        is_pub: bool,
        attributes: Vec<Attribute>,
    },
    /// Enum definition: `enum Name { Variant1, Variant2(Type), ... }`
    Enum {
        name: String,
        generics: Vec<GenericParam>,
        variants: Vec<EnumVariant>,
        where_clause: Vec<WhereConstraint>,
        is_pub: bool,
        attributes: Vec<Attribute>,
    },
    /// Trait definition: `trait Name { ... }`
    Trait {
        name: String,
        generics: Vec<GenericParam>,
        supertraits: Vec<String>,
        methods: Vec<Item>,
        where_clause: Vec<WhereConstraint>,
        is_pub: bool,
        attributes: Vec<Attribute>,
    },
    /// Impl block: `impl TraitName for StructName { ... }` or `impl StructName { ... }`
    Impl {
        generics: Vec<GenericParam>,
        trait_name: Option<String>,
        struct_name: String,
        methods: Vec<Item>,
        where_clause: Vec<WhereConstraint>,
        is_unsafe: bool,
        attributes: Vec<Attribute>,
    },
    /// Module definition: `mod name { ... }` or `mod name;` (with path)
    Module {
        name: String,
        items: Vec<Item>,
        is_inline: bool,
        is_pub: bool,
        attributes: Vec<Attribute>,
    },
    /// Use statement: `use path::to::item;` or `use path::*;`
    Use {
        path: Vec<String>,
        is_glob: bool,
        is_public: bool,
        attributes: Vec<Attribute>,
    },
    /// Type alias: `type Name = Type;` or `type Name<T> = Type;`
    TypeAlias {
        name: String,
        generics: Vec<GenericParam>,
        ty: Type,
        is_pub: bool,
        attributes: Vec<Attribute>,
    },
    /// Constant: `const NAME: Type = value;`
    Const {
        name: String,
        ty: Type,
        value: Expression,
        is_pub: bool,
        attributes: Vec<Attribute>,
    },
    /// Static variable: `static NAME: Type = value;` or `static mut NAME: Type = value;`
    Static {
        name: String,
        ty: Type,
        value: Expression,
        is_mutable: bool,
        is_pub: bool,
        attributes: Vec<Attribute>,
    },
    /// Extern block: `extern "C" { ... }`
    ExternBlock {
        abi: String,
        items: Vec<Item>,
        attributes: Vec<Attribute>,
    },
    /// Macro definition: `macro_rules! name { ... }`
    MacroDefinition {
        name: String,
        rules: Vec<MacroRule>,
        attributes: Vec<Attribute>,
    },
    /// Associated type in trait: `type Item = T;` or `type Item;`
    AssociatedType {
        name: String,
        bounds: Vec<String>,
        ty: Option<Type>,
        attributes: Vec<Attribute>,
    },
}

/// Where clause constraint: `T: Trait1 + Trait2`
#[derive(Debug, Clone, PartialEq)]
pub struct WhereConstraint {
    pub param_name: String,
    pub bounds: Vec<String>,
}

/// Generic parameter: `T`, `T: Bound`, `'a`, `const N: usize`
#[derive(Debug, Clone, PartialEq)]
pub enum GenericParam {
    /// Type parameter: `T` or `T: Display`
    Type {
        name: String,
        bounds: Vec<String>,
        default: Option<Box<Type>>,
    },
    /// Lifetime parameter: `'a`
    Lifetime(String),
    /// Const parameter: `const N: usize`
    Const {
        name: String,
        ty: Type,
    },
}

/// Macro rule for macro_rules!
#[derive(Debug, Clone, PartialEq)]
pub struct MacroRule {
    pub pattern: String,  // Macro pattern
    pub body: String,     // Macro body (simplified)
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
    pub attributes: Vec<Attribute>,
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
        attributes: Vec<Attribute>,
        pattern: Option<Pattern>,
    },
    /// Expression statement: `x + 1;`
    Expression(Expression),
    /// Return statement: `return value;` or just `return;`
    Return(Option<Box<Expression>>),
    /// Break statement: `break;` (in loops) - can optionally break with value
    Break(Option<Box<Expression>>),
    /// Continue statement: `continue;` (in loops)
    
    // Macro invocation: `name!(args)` or `name!(a, b, c)`
    MacroInvocation {
        name: String,
        args: Vec<Expression>,
    },
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
    /// Unsafe block: `unsafe { ... }`
    UnsafeBlock(Block),
    /// Item definition (nested functions, structs, etc.)
    Item(Box<Item>),
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

    // Closure/Lambda: `|x, y| x + y` or `|x: i32| x + 1`
    Closure {
        params: Vec<(String, Option<Type>)>,
        body: Box<Expression>,
        is_move: bool,
    },

    // Method call: `obj.method(args)` or `obj.method::<T>(args)`
    MethodCall {
        receiver: Box<Expression>,
        method: String,
        type_args: Vec<Type>,
        args: Vec<Expression>,
    },

    // Cast expression: `value as Type`
    Cast {
        value: Box<Expression>,
        ty: Type,
    },

    // Try operator: `value?`
    Try {
        value: Box<Expression>,
    },

    // Unsafe block: `unsafe { ... }`
    UnsafeBlock(Block),

    // Async block: `async { ... }`
    AsyncBlock(Block),

    // Await expression: `future.await`
    Await {
        value: Box<Expression>,
    },

    // Path expression: `crate::module::item` or `Self::method`
    Path {
        segments: Vec<String>,
        is_absolute: bool, // true for ::global, false for relative
    },

    // Qualified path: `<Type as Trait>::associated_type`
    QualifiedPath {
        ty: Type,
        trait_name: Option<String>,
        name: String,
    },

    // String with format arguments: `"Hello {}"` - for macro support
    FormatString {
        parts: Vec<String>,
        args: Vec<Expression>,
    },

    // Vector literal: `vec![1, 2, 3]` - simplified macro support
    VecMacro {
        elements: Vec<Expression>,
    },

    // Generic function call: `function::<Type>(args)`
    GenericCall {
        name: String,
        type_args: Vec<Type>,
        args: Vec<Expression>,
    },

    // Box expression: `box value` or `Box::new(value)`
    Box(Box<Expression>),

    // Dereference with auto-deref: `*value`
    Deref {
        value: Box<Expression>,
    },

    // Return expression: `return value`
    Return(Option<Box<Expression>>),

    // Break with value: `break value` or just `break`
    Break(Option<Box<Expression>>),

    // Continue expression
    
    // Macro invocation: `name!(args)` or `name!(a, b, c)`
    MacroInvocation {
        name: String,
        args: Vec<Expression>,
    },
    Continue,
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
    /// Mutable binding: `mut x`
    MutableBinding(String),
    /// Reference pattern: `&x` or `&mut x`
    Reference {
        mutable: bool,
        pattern: Box<Pattern>,
    },
    /// Tuple pattern: `(x, y)`
    Tuple(Vec<Pattern>),
    /// Struct pattern: `Point { x, y }`
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
    },
    /// Or pattern: `pattern1 | pattern2`
    Or(Vec<Pattern>),
    /// Range pattern: `1..5` or `1..=5`
    Range {
        start: Box<Expression>,
        end: Box<Expression>,
        inclusive: bool,
    },
    /// Slice pattern: `[a, b, ..rest]`
    /// Elements before the rest pattern, rest pattern (if any), elements after
    Slice {
        patterns: Vec<Pattern>,
        rest: Option<Box<Pattern>>,
    },
    /// Box pattern: `box value`
    Box(Box<Pattern>),
    /// Enum variant pattern: `Some(x)` or `Result::Ok(value)`
    EnumVariant {
        path: Vec<String>,
        data: Option<Box<Pattern>>,
    },
}

/// Types in Rust
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Basic types: i32, i64, f64, bool, char, str
    Named(String),
    
    /// Generic type: `Vec<T>`, `HashMap<K, V>`, etc.
    Generic {
        name: String,
        type_args: Vec<Type>,
    },
    
    /// Reference: `&T` or `&mut T` with optional lifetime `&'a T`
    Reference {
        lifetime: Option<String>,
        mutable: bool,
        inner: Box<Type>,
    },
    
    /// Pointer: `*const T` or `*mut T` (for unsafe)
    Pointer {
        mutable: bool,
        inner: Box<Type>,
    },
    
    /// Array: `[T; size]`
    Array {
        element: Box<Type>,
        size: Option<Box<Expression>>, // None means slice
    },
    
    /// Function type: `fn(T1, T2) -> R` or `unsafe fn()` or `extern "C" fn()`
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
        is_unsafe: bool,
        abi: Option<String>, // "C", "Rust", etc.
    },
    
    /// Tuple type: `(T1, T2)`
    Tuple(Vec<Type>),
    
    /// Trait object: `dyn Trait` or `dyn Trait + 'a`
    TraitObject {
        bounds: Vec<String>,
        lifetime: Option<String>,
    },
    
    /// Impl trait: `impl Trait` for return types
    ImplTrait {
        bounds: Vec<String>,
    },
    
    /// Associated type: `T::AssocType`
    AssociatedType {
        ty: Box<Type>,
        name: String,
    },
    
    /// Qualified path type: `<T as Trait>::Type`
    QualifiedPath {
        ty: Box<Type>,
        trait_name: String,
        name: String,
    },
    
    /// Closure type (simplified)
    Closure {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    
    /// Type parameter/variable: `T`, `U`, etc.
    TypeVar(String),
    
    /// Never type: `!`
    Never,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Named(name) => write!(f, "{}", name),
            Type::Generic { name, type_args } => {
                write!(f, "{}<", name)?;
                for (i, arg) in type_args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            }
            Type::Reference { lifetime, mutable, inner } => {
                write!(f, "&")?;
                if let Some(lt) = lifetime {
                    write!(f, "{} ", lt)?;
                }
                if *mutable { write!(f, "mut ")?; }
                write!(f, "{}", inner)
            }
            Type::Pointer { mutable, inner } => {
                write!(f, "*{}{}", if *mutable { "mut " } else { "const " }, inner)
            }
            Type::Array { element, size } => {
                if let Some(_sz) = size {
                    write!(f, "[{}; ...]", element)
                } else {
                    write!(f, "[{}]", element)
                }
            }
            Type::Function { params, return_type, is_unsafe, abi } => {
                if *is_unsafe { write!(f, "unsafe ")?; }
                if let Some(a) = abi {
                    write!(f, "extern \"{}\" ", a)?;
                }
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
            Type::TraitObject { bounds, lifetime } => {
                write!(f, "dyn")?;
                if let Some(lt) = lifetime {
                    write!(f, " {} +", lt)?;
                } else {
                    write!(f, " ")?;
                }
                for (i, bound) in bounds.iter().enumerate() {
                    if i > 0 { write!(f, " +")?; }
                    write!(f, " {}", bound)?;
                }
                Ok(())
            }
            Type::ImplTrait { bounds } => {
                write!(f, "impl")?;
                for (i, bound) in bounds.iter().enumerate() {
                    write!(f, "{} {}", if i == 0 { " " } else { " + " }, bound)?;
                }
                Ok(())
            }
            Type::AssociatedType { ty, name } => {
                write!(f, "{}::{}", ty, name)
            }
            Type::QualifiedPath { ty, trait_name, name } => {
                write!(f, "<{} as {}>::{}", ty, trait_name, name)
            }
            Type::Closure { params, return_type } => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", return_type)
            }
            Type::TypeVar(name) => write!(f, "{}", name),
            Type::Never => write!(f, "!"),
        }
    }
}

/// Attribute: `#[derive(Debug)]`, `#[cfg(test)]`, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<String>,
    pub is_macro: bool, // true for #[...], false for ///
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
    Negate,           // -
    Not,              // !
    BitwiseNot,       // ~
    Dereference,      // *
    Reference,        // & (immutable borrow)
    MutableReference, // &mut (mutable borrow)
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnaryOp::Negate => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
            UnaryOp::BitwiseNot => write!(f, "~"),
            UnaryOp::Dereference => write!(f, "*"),
            UnaryOp::Reference => write!(f, "&"),
            UnaryOp::MutableReference => write!(f, "&mut"),
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