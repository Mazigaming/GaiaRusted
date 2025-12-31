//! # Phase 3: AST LOWERING (Syntactic Sugar Removal)
//!
//! Converts AST into HIR (Higher-Level IR) by removing syntactic sugar
//! and normalizing constructs.
//!
//! ## What we do:
//! - Remove syntactic sugar (for loops → while loops)
//! - Normalize patterns
//! - Expand basic macros
//! - Add implicit type annotations where possible
//!
//! ## Algorithm:
//! Single recursive pass over the AST, transforming nodes as we go.

use crate::parser::{self, Expression, Statement, Item, Type, Block, Parameter, StructField, Pattern, EnumVariant, GenericParam};
use std::fmt;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

thread_local! {
    static ENUM_REGISTRY: RefCell<HashMap<String, HashMap<String, i64>>> = RefCell::new(HashMap::new());
    static SCOPE_TRACKER: RefCell<ScopeTracker> = RefCell::new(ScopeTracker::new());
    static STRUCT_REGISTRY: RefCell<HashMap<String, Vec<(String, HirType)>>> = RefCell::new(HashMap::new());
}

/// Tracks available variables in the current scope
#[derive(Debug, Clone)]
pub struct ScopeTracker {
    scopes: Vec<HashMap<String, HirType>>,
}

impl ScopeTracker {
    pub fn new() -> Self {
        ScopeTracker {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn add_binding(&mut self, name: String, ty: HirType) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    pub fn is_in_scope(&self, name: &str) -> bool {
        self.scopes.iter().any(|scope| scope.contains_key(name))
    }

    pub fn get_type(&self, name: &str) -> Option<HirType> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    pub fn get_all_bindings(&self) -> HashMap<String, HirType> {
        let mut result = HashMap::new();
        for scope in &self.scopes {
            for (name, ty) in scope {
                result.insert(name.clone(), ty.clone());
            }
        }
        result
    }
}

fn get_enum_variant(enum_name: &str, variant_name: &str) -> Option<i64> {
    ENUM_REGISTRY.with(|registry| {
        registry.borrow().get(enum_name).and_then(|variants| {
            variants.get(variant_name).copied()
        })
    })
}

fn register_enum_variants(enum_name: String, variants: Vec<String>) {
    ENUM_REGISTRY.with(|registry| {
        let mut reg = registry.borrow_mut();
        let mut variant_map = HashMap::new();
        for (idx, variant_name) in variants.iter().enumerate() {
            variant_map.insert(variant_name.clone(), idx as i64);
        }
        reg.insert(enum_name, variant_map);
    });
}

fn clear_enum_registry() {
    ENUM_REGISTRY.with(|registry| {
        registry.borrow_mut().clear();
    });
}

fn register_struct_fields(struct_name: String, fields: Vec<(String, HirType)>) {
    STRUCT_REGISTRY.with(|registry| {
        registry.borrow_mut().insert(struct_name, fields);
    });
}

fn get_struct_field_type(struct_name: &str, field_name: &str) -> Option<HirType> {
    STRUCT_REGISTRY.with(|registry| {
        registry.borrow().get(struct_name).and_then(|fields| {
            fields.iter()
                .find(|(fname, _)| fname == field_name)
                .map(|(_, ty)| ty.clone())
        })
    })
}

pub fn get_struct_field_index(struct_name: &str, field_name: &str) -> Option<usize> {
    STRUCT_REGISTRY.with(|registry| {
        registry.borrow().get(struct_name).and_then(|fields| {
            fields.iter()
                .position(|(fname, _)| fname == field_name)
        })
    })
}

fn clear_struct_registry() {
    STRUCT_REGISTRY.with(|registry| {
        registry.borrow_mut().clear();
    });
}

fn push_scope() {
    SCOPE_TRACKER.with(|tracker| {
        tracker.borrow_mut().push_scope();
    });
}

fn pop_scope() {
    SCOPE_TRACKER.with(|tracker| {
        tracker.borrow_mut().pop_scope();
    });
}

fn add_binding(name: String, ty: HirType) {
    SCOPE_TRACKER.with(|tracker| {
        tracker.borrow_mut().add_binding(name, ty);
    });
}

fn get_available_bindings() -> HashMap<String, HirType> {
    SCOPE_TRACKER.with(|tracker| {
        tracker.borrow().get_all_bindings()
    })
}

fn clear_scope_tracker() {
    SCOPE_TRACKER.with(|tracker| {
        *tracker.borrow_mut() = ScopeTracker::new();
    });
}

fn collect_variables_from_expr(expr: &HirExpression, vars: &mut HashSet<String>) {
    match expr {
        HirExpression::Variable(name) => {
            vars.insert(name.clone());
        }
        HirExpression::BinaryOp { left, right, .. } => {
            collect_variables_from_expr(left, vars);
            collect_variables_from_expr(right, vars);
        }
        HirExpression::UnaryOp { operand, .. } => {
            collect_variables_from_expr(operand, vars);
        }
        HirExpression::Assign { target, value } => {
            collect_variables_from_expr(target, vars);
            collect_variables_from_expr(value, vars);
        }
        HirExpression::If { condition, then_body, else_body } => {
            collect_variables_from_expr(condition, vars);
            for stmt in then_body {
                collect_variables_from_stmt(stmt, vars);
            }
            if let Some(else_stmts) = else_body {
                for stmt in else_stmts {
                    collect_variables_from_stmt(stmt, vars);
                }
            }
        }
        HirExpression::While { condition, body } => {
            collect_variables_from_expr(condition, vars);
            for stmt in body {
                collect_variables_from_stmt(stmt, vars);
            }
        }
        HirExpression::Match { scrutinee, arms } => {
            collect_variables_from_expr(scrutinee, vars);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_variables_from_expr(guard, vars);
                }
                for stmt in &arm.body {
                    collect_variables_from_stmt(stmt, vars);
                }
            }
        }
        HirExpression::Call { func, args } => {
            collect_variables_from_expr(func, vars);
            for arg in args {
                collect_variables_from_expr(arg, vars);
            }
        }
        HirExpression::FieldAccess { object, .. } => {
            collect_variables_from_expr(object, vars);
        }
        HirExpression::Index { array, index } => {
            collect_variables_from_expr(array, vars);
            collect_variables_from_expr(index, vars);
        }
        HirExpression::StructLiteral { fields, .. } => {
            for (_, field_expr) in fields {
                collect_variables_from_expr(field_expr, vars);
            }
        }
        HirExpression::EnumStructVariant { fields, .. } => {
            for (_, field_expr) in fields {
                collect_variables_from_expr(field_expr, vars);
            }
        }
        HirExpression::ArrayLiteral(exprs) => {
            for expr in exprs {
                collect_variables_from_expr(expr, vars);
            }
        }
        HirExpression::Tuple(exprs) => {
            for expr in exprs {
                collect_variables_from_expr(expr, vars);
            }
        }
        HirExpression::Block(stmts, final_expr) => {
            for stmt in stmts {
                collect_variables_from_stmt(stmt, vars);
            }
            if let Some(expr) = final_expr {
                collect_variables_from_expr(expr, vars);
            }
        }
        _ => {}
    }
}

fn collect_variables_from_stmt(stmt: &HirStatement, vars: &mut HashSet<String>) {
    match stmt {
        HirStatement::Expression(expr) => {
            collect_variables_from_expr(expr, vars);
        }
        HirStatement::Return(Some(expr)) => {
            collect_variables_from_expr(expr, vars);
        }
        HirStatement::Let { init, .. } => {
            collect_variables_from_expr(init, vars);
        }
        _ => {}
    }
}

fn convert_rust_format_to_printf(rust_fmt: &str) -> String {
    let mut result = String::new();
    let mut chars = rust_fmt.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '{' {
            if chars.peek() == Some(&'}') {
                chars.next();
                result.push_str("%ld");
            } else {
                result.push(ch);
            }
        } else if ch == '\\' {
            if let Some(next_ch) = chars.next() {
                match next_ch {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    c => {
                        result.push('\\');
                        result.push(c);
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    result.push('\n');
    result
}

fn convert_rust_format_to_printf_with_types(rust_fmt: &str, arg_types: &[HirType]) -> String {
    let mut result = String::new();
    let mut chars = rust_fmt.chars().peekable();
    let mut arg_index = 0;
    
    while let Some(ch) = chars.next() {
        if ch == '{' {
            if chars.peek() == Some(&'}') {
                chars.next();
                if arg_index < arg_types.len() {
                    let ty = &arg_types[arg_index];
                    let fmt_spec = get_printf_format_spec(ty);
                    result.push_str(fmt_spec);
                    arg_index += 1;
                } else {
                    result.push_str("%ld");
                }
            } else {
                result.push(ch);
            }
        } else if ch == '\\' {
            if let Some(next_ch) = chars.next() {
                match next_ch {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    c => {
                        result.push('\\');
                        result.push(c);
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    result.push('\n');
    result
}

fn get_printf_format_spec(ty: &HirType) -> &'static str {
    match ty {
        HirType::Float64 => "%f",
        HirType::String => "%s",
        HirType::Bool => "%d",
        HirType::Int32 => "%d",
        HirType::Int64 => "%ld",
        HirType::USize => "%lu",
        HirType::ISize => "%ld",
        HirType::Reference(inner) => get_printf_format_spec(inner),
        HirType::Pointer(inner) => {
            match &**inner {
                HirType::String => "%s",
                _ => "%p",
            }
        }
        _ => "%ld",
    }
}

/// High-Level Intermediate Representation (HIR)
/// Similar to AST but with syntactic sugar removed
#[derive(Debug, Clone)]
pub enum HirItem {
    /// Function definition
    Function {
        name: String,
        generics: Vec<GenericParam>,
        params: Vec<(String, HirType)>,
        return_type: Option<HirType>,
        body: Vec<HirStatement>,
    },
    /// Struct definition
    Struct {
        name: String,
        fields: Vec<(String, HirType)>,
    },
    /// Module definition
    Module {
        name: String,
        items: Vec<HirItem>,
        is_public: bool,
    },
    /// Associated type in trait
    AssociatedType {
        name: String,
        ty: Option<HirType>,
    },
    /// Use statement / re-export: `use path::to::item;` or `pub use path::to::item;`
    Use {
        path: Vec<String>,
        is_glob: bool,
        is_public: bool,
    },
    /// Const item: `const NAME: Type = value;`
    Const {
        name: String,
        ty: HirType,
        is_public: bool,
        generics: Vec<GenericParam>,
    },
    /// Static item: `static NAME: Type = value;`
    Static {
        name: String,
        ty: HirType,
        is_mutable: bool,
        is_public: bool,
        generics: Vec<GenericParam>,
    },
    /// Impl block: `impl Trait for Struct { methods }` or `impl Struct { methods }`
    Impl {
        trait_name: Option<String>,
        struct_name: String,
        methods: Vec<HirItem>,
        generics: Vec<String>,
        is_unsafe: bool,
    },
    /// Enum definition
    Enum {
        name: String,
        variants: Vec<(String, Option<HirType>)>,
    },
    /// Trait definition
    Trait {
        name: String,
        methods: Vec<HirItem>,
        generics: Vec<String>,
    },
}

/// HIR statements (simplified from parser statements)
#[derive(Debug, Clone)]
pub enum HirStatement {
    /// Variable binding: let x: i32 = 42; or let mut x: i32 = 42;
    Let {
        name: String,
        mutable: bool,
        ty: HirType,
        init: HirExpression,
    },
    /// Expression statement
    Expression(HirExpression),
    /// Return statement
    Return(Option<HirExpression>),
    /// Break statement
    Break,
    /// Continue statement
    Continue,
    /// For loop statement: for var in iter { body }
    For {
        var: String,
        iter: Box<HirExpression>,
        body: Vec<HirStatement>,
    },
    /// While loop statement: while condition { body }
    While {
        condition: Box<HirExpression>,
        body: Vec<HirStatement>,
    },
    /// If statement: if condition { then_body } else { else_body }
    If {
        condition: Box<HirExpression>,
        then_body: Vec<HirStatement>,
        else_body: Option<Vec<HirStatement>>,
    },
    /// Unsafe block: unsafe { ... }
    UnsafeBlock(Vec<HirStatement>),
    /// Item definition (nested functions, structs, etc.)
    Item(Box<HirItem>),
}

/// HIR expressions (simplified from parser expressions)
#[derive(Debug, Clone)]
pub enum HirExpression {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),

    // Variables and identifiers
    Variable(String),

    // Binary operations
    BinaryOp {
        op: BinaryOp,
        left: Box<HirExpression>,
        right: Box<HirExpression>,
    },

    // Unary operations
    UnaryOp {
        op: UnaryOp,
        operand: Box<HirExpression>,
    },

    // Assignment
    Assign {
        target: Box<HirExpression>,
        value: Box<HirExpression>,
    },

    // Control flow
    If {
        condition: Box<HirExpression>,
        then_body: Vec<HirStatement>,
        else_body: Option<Vec<HirStatement>>,
    },

    /// While loop (for loops desugared to while)
    While {
        condition: Box<HirExpression>,
        body: Vec<HirStatement>,
    },

    /// Match expression (simplified pattern support)
    Match {
        scrutinee: Box<HirExpression>,
        arms: Vec<MatchArm>,
    },

    /// Function call
    Call {
        func: Box<HirExpression>,
        args: Vec<HirExpression>,
    },

    /// Method call: obj.method(args)
    MethodCall {
        receiver: Box<HirExpression>,
        method: String,
        args: Vec<HirExpression>,
    },

    /// Field access: obj.field
    FieldAccess {
        object: Box<HirExpression>,
        field: String,
    },

    /// Tuple access: tuple.0, tuple.1, etc.
    TupleAccess {
        object: Box<HirExpression>,
        index: u32,
    },

    /// Array indexing: arr[idx]
    Index {
        array: Box<HirExpression>,
        index: Box<HirExpression>,
    },

    /// Struct literal: Point { x: 1, y: 2 }
    StructLiteral {
        name: String,
        fields: Vec<(String, HirExpression)>,
    },

    /// Array literal: [1, 2, 3]
    ArrayLiteral(Vec<HirExpression>),

    /// Tuple literal: (1, 2, 3)
    Tuple(Vec<HirExpression>),

    /// Enum variant: Status::Active or Result::Ok(42) or Message::Text(x, y)
    EnumVariant {
        enum_name: String,
        variant_name: String,
        args: Vec<HirExpression>,
    },

    /// Enum struct variant: Message::Text { content: "hello", id: 5 }
    EnumStructVariant {
        enum_name: String,
        variant_name: String,
        fields: Vec<(String, HirExpression)>,
    },

    /// Range literal: 0..10, 1..=5
    Range {
        start: Option<Box<HirExpression>>,
        end: Option<Box<HirExpression>>,
        inclusive: bool,
    },

    /// Block: { statements... ; expression }
    Block(Vec<HirStatement>, Option<Box<HirExpression>>),

    /// Closure: |x, y| x + y
    Closure {
        params: Vec<(String, HirType)>,
        body: Vec<HirStatement>,
        return_type: Box<HirType>,
        is_move: bool,
        captures: Vec<(String, HirType)>,
    },

    /// Try operator: `value?` - unwrap Result/Option or propagate error
    Try {
        value: Box<HirExpression>,
    },
}

/// Match arm for match expressions
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// Pattern to match (simplified: just identifiers and literals for now)
    pub pattern: String,
    /// Guard condition (optional)
    pub guard: Option<HirExpression>,
    /// Body of the arm
    pub body: Vec<HirStatement>,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    // Logical
    And,
    Or,

    // Bitwise
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,           // -x
    Not,              // !x
    BitwiseNot,       // ~x
    Dereference,      // *x
    Reference,        // &x
    MutableReference, // &mut x
}

/// Closure trait kind: Fn, FnMut, or FnOnce
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum ClosureTrait {
    /// Fn: immutable reference, can be called multiple times
    Fn,
    /// FnMut: mutable reference, can be called multiple times but may mutate
    FnMut,
    /// FnOnce: takes self by value, can only be called once
    FnOnce,
}

impl fmt::Display for ClosureTrait {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClosureTrait::Fn => write!(f, "Fn"),
            ClosureTrait::FnMut => write!(f, "FnMut"),
            ClosureTrait::FnOnce => write!(f, "FnOnce"),
        }
    }
}

/// HIR Types (simplified from parser types)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirType {
    /// Primitive types
    Int32,
    Int64,
    UInt32,
    UInt64,
    USize,
    ISize,
    Float64,
    Bool,
    Char,
    String,

    /// User-defined type
    Named(String),

    /// Reference type
    Reference(Box<HirType>),

    /// Mutable reference
    MutableReference(Box<HirType>),

    /// Pointer
    Pointer(Box<HirType>),

    /// Array type
    Array {
        element_type: Box<HirType>,
        size: Option<usize>,
    },

    /// Function type
    Function {
        params: Vec<HirType>,
        return_type: Box<HirType>,
    },

    /// Tuple type
    Tuple(Vec<HirType>),

    /// Closure type
    Closure {
        params: Vec<HirType>,
        return_type: Box<HirType>,
        trait_kind: ClosureTrait,
    },

    /// Range type (for slicing)
    Range,

    /// Unknown type (will be inferred later)
    Unknown,
}

impl fmt::Display for HirType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HirType::Int32 => write!(f, "i32"),
            HirType::Int64 => write!(f, "i64"),
            HirType::UInt32 => write!(f, "u32"),
            HirType::UInt64 => write!(f, "u64"),
            HirType::USize => write!(f, "usize"),
            HirType::ISize => write!(f, "isize"),
            HirType::Float64 => write!(f, "f64"),
            HirType::Bool => write!(f, "bool"),
            HirType::Char => write!(f, "char"),
            HirType::String => write!(f, "str"),
            HirType::Named(name) => write!(f, "{}", name),
            HirType::Reference(ty) => write!(f, "&{}", ty),
            HirType::MutableReference(ty) => write!(f, "&mut {}", ty),
            HirType::Pointer(ty) => write!(f, "*{}", ty),
            HirType::Array {
                element_type,
                size,
            } => {
                if let Some(sz) = size {
                    write!(f, "[{}; {}]", element_type, sz)
                } else {
                    write!(f, "[{}]", element_type)
                }
            }
            HirType::Function { params, return_type } => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", return_type)
            }
            HirType::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            HirType::Closure { params, return_type, trait_kind } => {
                write!(f, "{}(", trait_kind)?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", return_type)
            }
            HirType::Range => write!(f, "Range"),
            HirType::Unknown => write!(f, "?"),
        }
    }
}

/// Lowering error
#[derive(Debug, Clone)]
pub struct LowerError {
    pub message: String,
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

type LowerResult<T> = Result<T, LowerError>;

/// Convert a parser Type to HirType (used in some contexts)
fn convert_type(ty: &Type) -> HirType {
    match lower_type(ty) {
        Ok(hir_type) => hir_type,
        Err(_) => HirType::Unknown,
    }
}

/// Convert parsed types to HIR types
fn lower_type(ty: &Type) -> LowerResult<HirType> {
    match ty {
        Type::Named(name) => match name.as_str() {
            "i32" => Ok(HirType::Int32),
            "i64" => Ok(HirType::Int64),
            "u32" => Ok(HirType::UInt32),
            "u64" => Ok(HirType::UInt64),
            "usize" => Ok(HirType::USize),
            "isize" => Ok(HirType::ISize),
            "f64" => Ok(HirType::Float64),
            "bool" => Ok(HirType::Bool),
            "str" => Ok(HirType::String),
            _ => Ok(HirType::Named(name.clone())),
        },
        Type::Reference { lifetime: _, mutable: _, inner } => {
            let inner_hir = lower_type(inner)?;
            Ok(HirType::Reference(Box::new(inner_hir)))
        }
        Type::Pointer { mutable: _, inner } => {
            let inner_hir = lower_type(inner)?;
            Ok(HirType::Pointer(Box::new(inner_hir)))
        }
        Type::Array { element, size } => {
            let elem_hir = lower_type(element)?;
            // Extract the size from the expression if present
            let size_value = if let Some(size_expr) = size {
                if let Expression::Integer(n) = size_expr.as_ref() {
                    Some(*n as usize)
                } else if let Expression::Variable(_name) = size_expr.as_ref() {
                    // For const generics, we'll treat as unknown for now
                    None
                } else {
                    None
                }
            } else {
                None
            };
            
            Ok(HirType::Array {
                element_type: Box::new(elem_hir),
                size: size_value,
            })
        }
        Type::Function {
            params,
            return_type,
            is_unsafe: _,
            abi: _,
        } => {
            let params_hir: Result<Vec<_>, _> =
                params.iter().map(|p| lower_type(p)).collect();
            let ret_hir = lower_type(return_type)?;
            Ok(HirType::Function {
                params: params_hir?,
                return_type: Box::new(ret_hir),
            })
        }
        Type::Tuple(types) => {
            let types_hir: Result<Vec<_>, _> =
                types.iter().map(|t| lower_type(t)).collect();
            Ok(HirType::Tuple(types_hir?))
        }
        Type::Generic { .. } => {
            // Generic types with type parameters - treat as a named type for now
            Ok(HirType::Named("Generic".to_string()))
        }
        Type::TypeVar(_name) => {
            // Type variable - treat as unknown for now
            Ok(HirType::Unknown)
        }
        Type::ImplTrait { .. } => {
            // impl Trait - treat as unknown for now
            Ok(HirType::Unknown)
        }
        Type::TraitObject { .. } => {
            // dyn Trait - treat as a named type for now
            Ok(HirType::Named("TraitObject".to_string()))
        }
        Type::AssociatedType { .. } => {
            // Associated types like T::Item - treat as unknown for now
            Ok(HirType::Unknown)
        }
        Type::QualifiedPath { .. } => {
            // Qualified paths like <T as Trait>::Item - treat as unknown for now
            Ok(HirType::Unknown)
        }
        Type::Closure { params, return_type } => {
            let param_types: Result<Vec<_>, _> = params
                .iter()
                .map(|ty| lower_type(ty))
                .collect();
            
            match param_types {
                Ok(pts) => {
                    let ret_ty = lower_type(return_type)?;
                    Ok(HirType::Closure {
                        params: pts,
                        return_type: Box::new(ret_ty),
                        trait_kind: ClosureTrait::Fn,
                    })
                }
                Err(e) => Err(e),
            }
        }
        Type::Never => {
            // Never type (!)
            Ok(HirType::Named("!".to_string()))
        }
    }
}

/// Infer the type of a HirExpression for println argument type-awareness
fn infer_hir_type(expr: &HirExpression) -> HirType {
    match expr {
        HirExpression::Integer(_) => HirType::Int32,
        HirExpression::Float(_) => HirType::Float64,
        HirExpression::String(_) => HirType::String,
        HirExpression::Bool(_) => HirType::Bool,
        HirExpression::Variable(_name) => {
            // Try to look up the variable type from scope tracker
            SCOPE_TRACKER.with(|tracker| {
                let scopes = &tracker.borrow().scopes;
                for scope in scopes.iter().rev() {
                    if let Some(ty) = scope.get(_name) {
                        return ty.clone();
                    }
                }
                HirType::Unknown
            })
        }
        HirExpression::BinaryOp { left, right, .. } => {
            // Infer from operands
            let left_ty = infer_hir_type(left);
            let right_ty = infer_hir_type(right);
            
            // If either operand is a float, result is float
            if left_ty == HirType::Float64 || right_ty == HirType::Float64 {
                HirType::Float64
            } else if left_ty != HirType::Unknown {
                left_ty
            } else if right_ty != HirType::Unknown {
                right_ty
            } else {
                HirType::Unknown
            }
        }
        HirExpression::Call { func, args } => {
            // For method calls, try to infer from the method name
            if let HirExpression::Variable(func_name) = &**func {
                // Methods that return i64/usize
                if func_name == "len" || func_name.contains("::len") {
                    return HirType::Int64;
                }
                // Methods that return bool
                if func_name == "contains" || func_name.contains("::contains") {
                    return HirType::Bool;
                }
                if func_name == "is_empty" || func_name.contains("::is_empty") {
                    return HirType::Bool;
                }
                // Methods that return the element type (we infer from context)
                // pop, get, etc. should have their types inferred from the collection type
                if (func_name == "pop" || func_name.contains("::pop")) && !args.is_empty() {
                    // Try to infer from the receiver (first argument)
                    let receiver_type = infer_hir_type(&args[0]);
                    // For Vec::pop, return i64 (simplified assumption)
                    if let HirType::Named(name) = receiver_type {
                        if name.contains("Vec") {
                            return HirType::Int64;
                        }
                    }
                    return HirType::Int64; // Default assumption
                }
                if (func_name == "get" || func_name.contains("::get")) && !args.is_empty() {
                    // Similar to pop, return i64 for now
                    return HirType::Int64;
                }
            }
            HirType::Unknown
        }
        HirExpression::FieldAccess { object, field } => {
            // Try to infer the field type from struct definition
            let obj_ty = infer_hir_type(object);
            
            // If the object is a named type (struct), look up the field type
            if let HirType::Named(struct_name) = obj_ty {
                if let Some(field_ty) = get_struct_field_type(&struct_name, field) {
                    return field_ty;
                }
            }
            
            // Otherwise, try to infer from the field name (heuristic fallback)
            // Common patterns: name/str/text/message fields are strings
            if field.to_lowercase().contains("name")
                || field.to_lowercase().contains("str")
                || field.to_lowercase().contains("text")
                || field.to_lowercase().contains("message")
            {
                return HirType::String;
            }
            
            HirType::Unknown
        }
        _ => HirType::Unknown,
    }
}

/// Lower an expression from AST to HIR
fn lower_expression(expr: &Expression) -> LowerResult<HirExpression> {
    match expr {
        Expression::Integer(n) => Ok(HirExpression::Integer(*n)),
        Expression::Float(f) => Ok(HirExpression::Float(*f)),
        Expression::String(s) => Ok(HirExpression::String(s.clone())),
        Expression::Bool(b) => Ok(HirExpression::Bool(*b)),
        Expression::Char(c) => {
            // Treat char as an integer representing its Unicode code point
            // This is a simplified representation
            let code_point = *c as u32 as i64;
            Ok(HirExpression::Integer(code_point))
        }

        Expression::Variable(name) => Ok(HirExpression::Variable(name.clone())),

        Expression::Binary { left, op, right } => {
            let left_hir = lower_expression(left)?;
            let right_hir = lower_expression(right)?;
            let op_hir = match op {
                parser::BinaryOp::Add => BinaryOp::Add,
                parser::BinaryOp::Subtract => BinaryOp::Subtract,
                parser::BinaryOp::Multiply => BinaryOp::Multiply,
                parser::BinaryOp::Divide => BinaryOp::Divide,
                parser::BinaryOp::Modulo => BinaryOp::Modulo,
                parser::BinaryOp::Equal => BinaryOp::Equal,
                parser::BinaryOp::NotEqual => BinaryOp::NotEqual,
                parser::BinaryOp::Less => BinaryOp::Less,
                parser::BinaryOp::LessEq => BinaryOp::LessEqual,
                parser::BinaryOp::Greater => BinaryOp::Greater,
                parser::BinaryOp::GreaterEq => BinaryOp::GreaterEqual,
                parser::BinaryOp::And => BinaryOp::And,
                parser::BinaryOp::Or => BinaryOp::Or,
                parser::BinaryOp::BitwiseAnd => BinaryOp::BitwiseAnd,
                parser::BinaryOp::BitwiseOr => BinaryOp::BitwiseOr,
                parser::BinaryOp::BitwiseXor => BinaryOp::BitwiseXor,
                parser::BinaryOp::LeftShift => BinaryOp::LeftShift,
                parser::BinaryOp::RightShift => BinaryOp::RightShift,
            };
            Ok(HirExpression::BinaryOp {
                op: op_hir,
                left: Box::new(left_hir),
                right: Box::new(right_hir),
            })
        }

        Expression::Unary { op, operand } => {
            let operand_hir = lower_expression(operand)?;
            let op_hir = match op {
                parser::UnaryOp::Negate => UnaryOp::Negate,
                parser::UnaryOp::Not => UnaryOp::Not,
                parser::UnaryOp::BitwiseNot => UnaryOp::BitwiseNot,
                parser::UnaryOp::Dereference => UnaryOp::Dereference,
                parser::UnaryOp::Reference => UnaryOp::Reference,
                parser::UnaryOp::MutableReference => UnaryOp::MutableReference,
            };
            Ok(HirExpression::UnaryOp {
                op: op_hir,
                operand: Box::new(operand_hir),
            })
        }

        Expression::Assign { target, value } => {
            let target_hir = lower_expression(target)?;
            let value_hir = lower_expression(value)?;
            Ok(HirExpression::Assign {
                target: Box::new(target_hir),
                value: Box::new(value_hir),
            })
        }

        Expression::CompoundAssign { target, op: _, value } => {
            // For now, desugar compound assignments as regular assignments
            let target_hir = lower_expression(target)?;
            let value_hir = lower_expression(value)?;
            Ok(HirExpression::Assign {
                target: Box::new(target_hir),
                value: Box::new(value_hir),
            })
        }

        Expression::If {
            condition,
            then_body,
            else_body,
        } => {
            let cond_hir = lower_expression(condition)?;
            let then_hir = lower_block(then_body)?;
            let else_hir = if let Some(else_expr) = else_body {
                // else_body is an Expression, could be another If or Block
                match &**else_expr {
                    Expression::Block(block) => Some(lower_block(block)?),
                    _ => return Err(LowerError {
                        message: "Else body must be a block".to_string(),
                    }),
                }
            } else {
                None
            };
            Ok(HirExpression::If {
                condition: Box::new(cond_hir),
                then_body: then_hir,
                else_body: else_hir,
            })
        }

        Expression::While { condition, body } => {
            let cond_hir = lower_expression(condition)?;
            let body_hir = lower_block(body)?;
            Ok(HirExpression::While {
                condition: Box::new(cond_hir),
                body: body_hir,
            })
        }

        Expression::Loop(body) => {
            // Desugar loop { ... } to while true { ... }
            let body_stmts = lower_block(body)?;
            Ok(HirExpression::While {
                condition: Box::new(HirExpression::Bool(true)),
                body: body_stmts,
            })
        }

        Expression::Match {
            scrutinee,
            arms,
        } => {
            let scrutinee_hir = lower_expression(scrutinee)?;
            
            // Desugar match into nested if-else statements
            // Process arms in reverse to build the else-chain correctly
            let mut result_expr: Option<HirExpression> = None;
            
            for arm in arms.iter().rev() {
                let pattern_condition = match &arm.pattern {
                    Pattern::Literal(lit) => {
                        // Compare scrutinee with literal
                        HirExpression::BinaryOp {
                            op: BinaryOp::Equal,
                            left: Box::new(scrutinee_hir.clone()),
                            right: Box::new(lower_expression(lit)?),
                        }
                    }
                    Pattern::Identifier(_name) => {
                        // Identifiers always match (binding), so use true
                        HirExpression::Bool(true)
                    }
                    Pattern::EnumVariant { .. } => {
                        // Simplified enum patterns
                        HirExpression::Bool(true)
                    }
                    Pattern::Tuple(_) => {
                        // Simplified tuple patterns
                        HirExpression::Bool(true)
                    }
                    Pattern::Struct { .. } => {
                        // Simplified struct patterns
                        HirExpression::Bool(true)
                    }
                    Pattern::Wildcard => {
                        // Wildcard always matches
                        HirExpression::Bool(true)
                    }
                    Pattern::Range { .. } => {
                        // Range patterns for numbers
                        HirExpression::Bool(true)
                    }
                    _ => {
                        // Handle other patterns
                        HirExpression::Bool(true)
                    }
                };
                
                // Extract bindings from the pattern and create let bindings in the arm body
                let mut arm_body = Vec::new();
                
                // Add let bindings for pattern variables
                if let Pattern::EnumVariant { path: _, data: Some(inner_pattern) } = &arm.pattern {
                    // Extract variable name from inner pattern
                    if let Pattern::Identifier(var_name) = &**inner_pattern {
                        // Create a dummy binding for now - ideally we'd extract the actual value
                        // For now, treat the variable as bound to the scrutinee value
                        arm_body.push(HirStatement::Let {
                            name: var_name.clone(),
                            mutable: false,
                            ty: HirType::Unknown,
                            init: scrutinee_hir.clone(),
                        });
                    }
                }
                
                let arm_body_expr = lower_expression(&arm.body)?;
                arm_body.push(HirStatement::Expression(arm_body_expr));
                
                result_expr = Some(HirExpression::If {
                    condition: Box::new(pattern_condition),
                    then_body: arm_body,
                    else_body: result_expr.as_ref().map(|expr| {
                        vec![HirStatement::Expression(expr.clone())]
                    }),
                });
            }
            
            match result_expr {
                Some(expr) => Ok(expr),
                None => Err(LowerError {
                    message: "Match expression with no arms".to_string(),
                }),
            }
        }

        Expression::FunctionCall { name, args } => {
            let args_hir: Result<Vec<_>, _> =
                args.iter().map(|arg| lower_expression(arg)).collect();
            let mut args_final = args_hir?;
            
            if name.contains("::") {
                let parts: Vec<&str> = name.split("::").collect();
                if parts.len() == 2 {
                    let enum_name = parts[0].to_string();
                    let variant_name = parts[1].to_string();
                    
                    if get_enum_variant(&enum_name, &variant_name).is_some() {
                        return Ok(HirExpression::EnumVariant {
                            enum_name,
                            variant_name,
                            args: args_final,
                        });
                    }
                }
            }
            
            let func_name = match name.as_str() {
                "__builtin_println" | "println" => {
                    if args_final.len() > 1 {
                        if let HirExpression::String(fmt_str) = &args_final[0] {
                            // For format strings with arguments, try to use type-aware format specs
                            let arg_types: Vec<HirType> = args_final[1..].iter().map(infer_hir_type).collect();
                            
                            // Check if we have a definite float type or a BinaryOp that produces a float
                            let has_float = arg_types.iter().any(|t| t == &HirType::Float64);
                            let has_binop_float = args_final[1..].iter().any(|expr| {
                                matches!(expr, HirExpression::BinaryOp { .. }) &&
                                infer_hir_type(expr) == HirType::Float64
                            });
                            
                            if (has_float || has_binop_float) && args_final.len() == 2 {
                                // Single float argument with format string
                                // Use gaia_print_f64 instead of printf (which requires XMM0)
                                let func_name = "gaia_print_f64".to_string();
                                return Ok(HirExpression::Call {
                                    func: Box::new(HirExpression::Variable(func_name)),
                                    args: vec![args_final[1].clone()],
                                });
                            }
                            
                            let printf_fmt = convert_rust_format_to_printf_with_types(fmt_str, &arg_types);
                            args_final[0] = HirExpression::String(printf_fmt);
                        }
                        "__builtin_printf".to_string()
                    } else if args_final.len() == 1 {
                        // Type-aware println: check argument type
                        let arg_type = infer_hir_type(&args_final[0]);
                        match arg_type {
                            HirType::Int64 | HirType::Int32 | HirType::USize | HirType::ISize => {
                                "gaia_print_i64".to_string()
                            }
                            HirType::Bool => {
                                "gaia_print_bool".to_string()
                            }
                            HirType::Float64 => {
                                // For floats, use dedicated print function
                                "gaia_print_f64".to_string()
                            }
                            _ => {
                                // Default to string printing
                                "__builtin_println".to_string()
                            }
                        }
                    } else {
                        "__builtin_println".to_string()
                    }
                }
                "print" => {
                    if args_final.len() > 1 {
                        if let HirExpression::String(fmt_str) = &args_final[0] {
                            let printf_fmt_no_newline = {
                                let arg_types: Vec<HirType> = args_final[1..].iter().map(infer_hir_type).collect();
                                let fmt = convert_rust_format_to_printf_with_types(fmt_str, &arg_types);
                                if fmt.ends_with("\n") {
                                    fmt[..fmt.len()-1].to_string()
                                } else {
                                    fmt
                                }
                            };
                            args_final[0] = HirExpression::String(printf_fmt_no_newline);
                        }
                        "__builtin_printf".to_string()
                    } else {
                        "__builtin_print".to_string()
                    }
                }
                "eprintln" => "__builtin_eprintln".to_string(),
                "__builtin_println_args" => {
                    if !args_final.is_empty() {
                        if let HirExpression::String(fmt_str) = &args_final[0] {
                            let arg_types: Vec<HirType> = args_final[1..].iter().map(infer_hir_type).collect();
                            let printf_fmt = convert_rust_format_to_printf_with_types(fmt_str, &arg_types);
                            args_final[0] = HirExpression::String(printf_fmt);
                        }
                    }
                    "__builtin_printf".to_string()
                }
                _ => name.clone(),
            };
            
            Ok(HirExpression::Call {
                func: Box::new(HirExpression::Variable(func_name)),
                args: args_final,
            })
        }

        Expression::FieldAccess { object, field } => {
            let object_hir = lower_expression(object)?;
            Ok(HirExpression::FieldAccess {
                object: Box::new(object_hir),
                field: field.clone(),
            })
        }

        Expression::Index { array, index } => {
            let array_hir = lower_expression(array)?;
            let index_hir = lower_expression(index)?;
            Ok(HirExpression::Index {
                array: Box::new(array_hir),
                index: Box::new(index_hir),
            })
        }

        Expression::StructLiteral {
            struct_name,
            fields,
        } => {
            let fields_hir: Result<Vec<_>, _> = fields
                .iter()
                .map(|(fname, fexpr)| {
                    let expr_hir = lower_expression(fexpr)?;
                    Ok((fname.clone(), expr_hir))
                })
                .collect();
            Ok(HirExpression::StructLiteral {
                name: struct_name.clone(),
                fields: fields_hir?,
            })
        }

        Expression::EnumStructLiteral {
            enum_name,
            variant_name,
            fields,
        } => {
            let fields_hir: Result<Vec<_>, _> = fields
                .iter()
                .map(|(fname, fexpr)| {
                    let expr_hir = lower_expression(fexpr)?;
                    Ok((fname.clone(), expr_hir))
                })
                .collect();
            Ok(HirExpression::EnumStructVariant {
                enum_name: enum_name.clone(),
                variant_name: variant_name.clone(),
                fields: fields_hir?,
            })
        }

        Expression::Array(elements) => {
            let elements_hir: Result<Vec<_>, _> =
                elements.iter().map(|e| lower_expression(e)).collect();
            Ok(HirExpression::ArrayLiteral(elements_hir?))
        }

        Expression::Block(block) => {
            let block_hir = lower_block(block)?;
            let last_expr = if let Some(e) = &block.expression {
                Some(Box::new(lower_expression(e)?))
            } else {
                None
            };
            Ok(HirExpression::Block(block_hir, last_expr))
        }

        Expression::Range { start, end, inclusive } => {
            // Desugar range expressions into a special RangeExpression
            // For codegen, this will be converted to an iterator or loop structure
            let start_expr = if let Some(s) = start {
                Some(Box::new(lower_expression(s)?))
            } else {
                None
            };
            let end_expr = if let Some(e) = end {
                Some(Box::new(lower_expression(e)?))
            } else {
                None
            };
            Ok(HirExpression::Range {
                start: start_expr,
                end: end_expr,
                inclusive: *inclusive,
            })
        }

        Expression::Tuple(elements) => {
            let elements_hir: Result<Vec<_>, _> =
                elements.iter().map(|e| lower_expression(e)).collect();
            Ok(HirExpression::Tuple(elements_hir?))
        }

        Expression::For { var, iter, body } => {
            // Desugar: for var in iter { body } 
            // to: {
            //     let mut __iter = iter;
            //     while ... { var = __iter.next(); body }
            // }
            // For simple ranges like 0..10, we desugar differently:
            // to: { let mut var = 0; while var < 10 { body; var = var + 1; } }
            
            let iter_expr = lower_expression(iter)?;
            let body_stmts = lower_block(body)?;
            
            // Check if iter is a simple range
            if let HirExpression::Range { start: Some(_s), end: Some(e), inclusive } = &iter_expr {
                // Desugar simple range iteration:
                // let mut var = start;
                // while var < end { body; var = var + 1; }
                let var_name = var.clone();
                let condition = HirExpression::BinaryOp {
                    op: if *inclusive { BinaryOp::LessEqual } else { BinaryOp::Less },
                    left: Box::new(HirExpression::Variable(var_name.clone())),
                    right: e.clone(),
                };
                
                let increment = HirExpression::Assign {
                    target: Box::new(HirExpression::Variable(var_name.clone())),
                    value: Box::new(HirExpression::BinaryOp {
                        op: BinaryOp::Add,
                        left: Box::new(HirExpression::Variable(var_name.clone())),
                        right: Box::new(HirExpression::Integer(1)),
                    }),
                };
                
                let mut while_body = body_stmts.clone();
                while_body.push(HirStatement::Expression(increment));
                
                Ok(HirExpression::While {
                    condition: Box::new(condition),
                    body: while_body,
                })
            } else {
                // For general iterators, we need full iterator support
                // For now, simplified: assume iteration over collections
                // Proper implementation would desugar to .into_iter().next() calls
                Ok(HirExpression::While {
                    condition: Box::new(HirExpression::Bool(true)),
                    body: body_stmts,
                })
            }
        }

        Expression::Closure { params, return_type: ret_type_opt, body, is_move } => {
            let mut typed_params = Vec::new();
            let mut param_names = HashSet::new();
            
            for (param_name, param_type_opt) in params {
                let hir_type = match param_type_opt {
                    Some(ty) => lower_type(ty)?,
                    None => HirType::Unknown,
                };
                typed_params.push((param_name.clone(), hir_type));
                param_names.insert(param_name.clone());
            }
            
            let return_type = match ret_type_opt {
                Some(ty) => lower_type(ty)?,
                None => HirType::Unknown,
            };
            
            let lowered_body = lower_expression(body)?;
            
            let mut body_stmts = match lowered_body {
                HirExpression::Block(stmts, final_expr) => {
                    let mut result = stmts;
                    if let Some(expr) = final_expr {
                        result.push(HirStatement::Return(Some(*expr)));
                    }
                    result
                }
                expr => {
                    vec![HirStatement::Return(Some(expr))]
                }
            };
            
            if body_stmts.is_empty() {
                body_stmts.push(HirStatement::Return(None));
            }
            
            let mut used_vars = HashSet::new();
            for stmt in &body_stmts {
                collect_variables_from_stmt(stmt, &mut used_vars);
            }
            
            let available_bindings = get_available_bindings();
            let mut captures = Vec::new();
            
            for var_name in used_vars {
                if !param_names.contains(&var_name) {
                    if let Some(var_type) = available_bindings.get(&var_name) {
                        captures.push((var_name, var_type.clone()));
                    }
                }
            }
            
            Ok(HirExpression::Closure {
                params: typed_params,
                body: body_stmts,
                return_type: Box::new(return_type),
                is_move: *is_move,
                captures,
            })
        }

        // New Expression variants from expanded AST
        Expression::MethodCall { receiver, method, type_args: _, args } => {
            let receiver_hir = lower_expression(receiver)?;
            let args_hir: Result<Vec<_>, _> = args
                .iter()
                .map(lower_expression)
                .collect();
            
            Ok(HirExpression::MethodCall {
                receiver: Box::new(receiver_hir),
                method: method.clone(),
                args: args_hir?,
            })
        }

        Expression::Cast { value: _, ty: _ } => {
            Err(LowerError {
                message: "Type casts not yet fully supported".to_string(),
            })
        }

        Expression::Try { value } => {
            let inner = lower_expression(value)?;
            Ok(HirExpression::Try {
                value: Box::new(inner),
            })
        }

        Expression::UnsafeBlock(block) => {
            let block_hir = lower_block(block)?;
            let last_expr = if let Some(e) = &block.expression {
                Some(Box::new(lower_expression(e)?))
            } else {
                None
            };
            Ok(HirExpression::Block(block_hir, last_expr))
        }

        Expression::AsyncBlock(_) => {
            Err(LowerError {
                message: "Async blocks not yet fully supported".to_string(),
            })
        }

        Expression::Await { value: _ } => {
            Err(LowerError {
                message: "Await expressions not yet fully supported".to_string(),
            })
        }

        Expression::Path { segments, is_absolute: _ } => {
            if segments.len() == 2 {
                let enum_name = &segments[0];
                let variant_name = &segments[1];
                
                if get_enum_variant(enum_name, variant_name).is_some() {
                    Ok(HirExpression::EnumVariant {
                        enum_name: enum_name.clone(),
                        variant_name: variant_name.clone(),
                        args: Vec::new(),
                    })
                } else {
                    Ok(HirExpression::Variable(format!("{}::{}", enum_name, variant_name)))
                }
            } else if segments.len() > 1 {
                let path = segments.join("::");
                Ok(HirExpression::Variable(path))
            } else {
                Ok(HirExpression::Variable(segments[0].clone()))
            }
        }

        Expression::QualifiedPath { .. } => {
            Err(LowerError {
                message: "Qualified path expressions not yet fully supported".to_string(),
            })
        }

        Expression::GenericCall { name, type_args: _, args } => {
            // Handle generic-style calls like `Type::method(args)`
            // Format: "Type::method" from the parser
            let args_hir: Result<Vec<_>, _> =
                args.iter().map(|arg| lower_expression(arg)).collect();
            let args_final = args_hir?;
            
            if name.contains("::") {
                let parts: Vec<&str> = name.split("::").collect();
                if parts.len() == 2 {
                    let enum_name = parts[0].to_string();
                    let variant_name = parts[1].to_string();
                    
                    if get_enum_variant(&enum_name, &variant_name).is_some() {
                        return Ok(HirExpression::EnumVariant {
                            enum_name,
                            variant_name,
                            args: args_final,
                        });
                    }
                }
                
                Ok(HirExpression::Call {
                    func: Box::new(HirExpression::Variable(name.clone())),
                    args: args_final,
                })
            } else {
                // Regular generic function call
                Ok(HirExpression::Call {
                    func: Box::new(HirExpression::Variable(name.clone())),
                    args: args_final,
                })
            }
        }

        Expression::VecMacro { elements: _ } => {
            Err(LowerError {
                message: "Vec! macro not yet fully supported".to_string(),
            })
        }

        Expression::FormatString { parts: _, args: _ } => {
            Err(LowerError {
                message: "Format strings not yet fully supported".to_string(),
            })
        }

        Expression::Box(_) => {
            Err(LowerError {
                message: "Box expressions not yet fully supported".to_string(),
            })
        }

        Expression::Deref { value: _ } => {
            Err(LowerError {
                message: "Dereference expressions not yet fully supported".to_string(),
            })
        }

        Expression::Return(_) => {
            Err(LowerError {
                message: "return expressions should be handled as statements, not expressions".to_string(),
            })
        }

        Expression::Break(_) => {
            Err(LowerError {
                message: "break expressions should be handled as statements, not expressions".to_string(),
            })
        }

        Expression::Continue => {
            Err(LowerError {
                message: "continue should be handled as a statement, not an expression".to_string(),
            })
        }

        Expression::MacroInvocation { name: _, args: _ } => {
            Err(LowerError {
                message: "Macro invocations not yet fully supported".to_string(),
            })
        }
    }
}

/// Lower a block (statements + optional expression)
fn lower_block(block: &Block) -> LowerResult<Vec<HirStatement>> {
    let mut statements = lower_statements(&block.statements)?;
    
    if let Some(expr) = &block.expression {
        let expr_hir = lower_expression(expr)?;
        statements.push(HirStatement::Expression(expr_hir));
    }
    
    Ok(statements)
}

/// Lower a statement from AST to HIR
fn lower_statement(stmt: &Statement) -> LowerResult<HirStatement> {
    match stmt {
        Statement::Let {
            name,
            mutable,
            ty: type_opt,
            initializer,
            attributes: _,
            pattern: _,
        } => {
            let init_hir = lower_expression(initializer)?;
            // Infer or use provided type
            let ty = if let Some(t) = type_opt {
                lower_type(t)?
            } else {
                // Try to infer type from initializer expression
                infer_hir_type(&init_hir)
            };
            add_binding(name.clone(), ty.clone());
            Ok(HirStatement::Let {
                name: name.clone(),
                mutable: *mutable,
                ty,
                init: init_hir,
            })
        }

        Statement::Expression(expr) => {
            let expr_hir = lower_expression(expr)?;
            Ok(HirStatement::Expression(expr_hir))
        }

        Statement::Return(expr_opt) => {
            let expr_hir = if let Some(e) = expr_opt {
                Some(lower_expression(e)?)
            } else {
                None
            };
            Ok(HirStatement::Return(expr_hir))
        }

        Statement::Break(_) => Ok(HirStatement::Break),

        Statement::Continue => Ok(HirStatement::Continue),

        Statement::For {
            var,
            iter,
            body,
        } => {
            let iter_hir = lower_expression(iter)?;
            let body_hir = lower_statements(&body.statements)?;
            Ok(HirStatement::For {
                var: var.clone(),
                iter: Box::new(iter_hir),
                body: body_hir,
            })
        }

        Statement::While {
            condition,
            body,
        } => {
            let cond_hir = lower_expression(condition)?;
            let body_hir = lower_statements(&body.statements)?;
            Ok(HirStatement::While {
                condition: Box::new(cond_hir),
                body: body_hir,
            })
        }

        Statement::If {
            condition,
            then_body,
            else_body,
        } => {
            let cond_hir = lower_expression(condition)?;
            let then_hir = lower_block(then_body)?;
            let else_hir = if let Some(else_stmt) = else_body {
                // else_body is a Statement, which could be another If (for else-if) or a block
                // We need to convert it to Vec<HirStatement>
                Some(vec![lower_statement(else_stmt)?])
            } else {
                None
            };
            Ok(HirStatement::If {
                condition: Box::new(cond_hir),
                then_body: then_hir,
                else_body: else_hir,
            })
        }

        Statement::UnsafeBlock(block) => {
            let body_hir = lower_statements(&block.statements)?;
            Ok(HirStatement::UnsafeBlock(body_hir))
        }

        Statement::Item(item) => {
            let item_hir = lower_item(item)?;
            Ok(HirStatement::Item(Box::new(item_hir)))
        }

        Statement::MacroInvocation { name: _, args: _ } => {
            Err(LowerError {
                message: "Macro invocations not yet fully supported".to_string(),
            })
        }
    }
}

/// Extract variable names and their positions from a pattern
fn extract_pattern_vars(pattern: &Pattern) -> Vec<String> {
    match pattern {
        Pattern::Identifier(name) => vec![name.clone()],
        Pattern::MutableBinding(name) => vec![name.clone()],
        Pattern::Tuple(patterns) => {
            patterns.iter()
                .flat_map(|p| extract_pattern_vars(p))
                .collect()
        }
        _ => vec![],
    }
}

/// Lower a list of statements
fn lower_statements(stmts: &[Statement]) -> LowerResult<Vec<HirStatement>> {
    let mut result = Vec::new();
    
    for stmt in stmts {
        if let Statement::Let {
            name: _,
            mutable,
            ty: _,
            initializer,
            attributes: _,
            pattern: Some(Pattern::Tuple(patterns)),
        } = stmt {
            // Handle tuple destructuring
            let tuple_init = lower_expression(initializer)?;
            
            // If it's a literal tuple, extract elements directly
            if let HirExpression::Tuple(elements) = tuple_init {
                for (idx, pattern) in patterns.iter().enumerate() {
                    let vars = extract_pattern_vars(pattern);
                    if let Some(elem) = elements.get(idx) {
                        for var_name in vars {
                            result.push(HirStatement::Let {
                                name: var_name.clone(),
                                mutable: *mutable,
                                ty: HirType::Unknown,
                                init: elem.clone(),
                            });
                            add_binding(var_name, HirType::Unknown);
                        }
                    }
                }
            } else {
                // For non-literal tuples, use temporary and field access
                let tuple_temp = format!("__tuple_temp_{}", result.len());
                
                // Create a temporary variable to hold the tuple
                result.push(HirStatement::Let {
                    name: tuple_temp.clone(),
                    mutable: false,
                    ty: HirType::Unknown,
                    init: tuple_init,
                });
                
                // For each pattern in the tuple, create a let binding
                for (idx, pattern) in patterns.iter().enumerate() {
                    let vars = extract_pattern_vars(pattern);
                    for var_name in vars {
                        let field_access = HirExpression::TupleAccess {
                            object: Box::new(HirExpression::Variable(tuple_temp.clone())),
                            index: idx as u32,
                        };
                        result.push(HirStatement::Let {
                            name: var_name.clone(),
                            mutable: *mutable,
                            ty: HirType::Unknown,
                            init: field_access,
                        });
                        add_binding(var_name, HirType::Unknown);
                    }
                }
            }
        } else if let Statement::Let {
            name: _,
            mutable,
            ty: _,
            initializer,
            attributes: _,
            pattern: Some(Pattern::Slice { patterns, .. }),
        } = stmt {
            // Handle array/slice destructuring
            let array_init = lower_expression(initializer)?;
            
            // If it's a literal array, extract elements directly
            if let HirExpression::ArrayLiteral(array_elements) = array_init {
                for (idx, pattern) in patterns.iter().enumerate() {
                    let vars = extract_pattern_vars(pattern);
                    if let Some(elem) = array_elements.get(idx) {
                        for var_name in vars {
                            result.push(HirStatement::Let {
                                name: var_name.clone(),
                                mutable: *mutable,
                                ty: HirType::Unknown,
                                init: elem.clone(),
                            });
                            add_binding(var_name, HirType::Unknown);
                        }
                    }
                }
            } else {
                // For non-literal arrays, use temporary and index access
                let array_temp = format!("__array_temp_{}", result.len());
                
                // Create a temporary variable to hold the array
                result.push(HirStatement::Let {
                    name: array_temp.clone(),
                    mutable: false,
                    ty: HirType::Unknown,
                    init: array_init,
                });
                
                // For each pattern in the array, create a let binding with index access
                for (idx, pattern) in patterns.iter().enumerate() {
                    let vars = extract_pattern_vars(pattern);
                    for var_name in vars {
                        let index_access = HirExpression::Index {
                            array: Box::new(HirExpression::Variable(array_temp.clone())),
                            index: Box::new(HirExpression::Integer(idx as i64)),
                        };
                        result.push(HirStatement::Let {
                            name: var_name.clone(),
                            mutable: *mutable,
                            ty: HirType::Unknown,
                            init: index_access,
                        });
                        add_binding(var_name, HirType::Unknown);
                    }
                }
            }
        } else {
            result.push(lower_statement(stmt)?);
        }
    }
    
    Ok(result)
}

/// Lower an item from AST to HIR
fn lower_item(item: &Item) -> LowerResult<HirItem> {
     match item {
         Item::Function {
             name,
             generics,
             params,
             return_type,
             body,
             is_unsafe: _,
             is_async: _,
             is_pub: _,
             attributes: _,
             where_clause: _,
             abi: _,
         } => {
             // Skip enum constructor functions (they have empty bodies with no statements/expression)
             // These will be handled specially during codegen
             if body.statements.is_empty() && body.expression.is_none() && name.contains("::") {
                 // Check if this looks like an enum constructor (EnumName::VariantName)
                 let parts: Vec<&str> = name.split("::").collect();
                 if parts.len() == 2 {
                     // This is likely an enum constructor - skip it
                     // Create a dummy function that won't be called
                     return Ok(HirItem::Function {
                         name: format!("_enum_constructor_{}", name.replace("::", "_impl_")),
                         generics: vec![],
                         params: vec![],
                         return_type: None,
                         body: vec![],
                     });
                 }
             }

             let params_hir: Result<Vec<_>, _> = params
                 .iter()
                 .map(|param: &Parameter| {
                     let ptype_hir = lower_type(&param.ty)?;
                     Ok((param.name.clone(), ptype_hir))
                 })
                 .collect();

             let ret_type_hir = if let Some(rt) = return_type {
                 Some(lower_type(rt)?)
             } else {
                 None
             };

             let body_hir = lower_block(body)?;

             Ok(HirItem::Function {
                 name: name.clone(),
                 generics: generics.clone(),
                 params: params_hir?,
                 return_type: ret_type_hir,
                 body: body_hir,
             })
         }

        Item::Struct { name, generics: _, fields, is_pub: _, attributes: _, where_clause: _ } => {
            let fields_hir: Result<Vec<_>, _> = fields
                .iter()
                .map(|field: &StructField| {
                    let ftype_hir = lower_type(&field.ty)?;
                    Ok((field.name.clone(), ftype_hir))
                })
                .collect();

            Ok(HirItem::Struct {
                name: name.clone(),
                fields: fields_hir?,
            })
        }

        Item::Enum { name, generics: _, variants, is_pub: _, attributes: _, where_clause: _ } => {
            // Register enum variants for later resolution (important for inner enums)
            let variant_names_list: Vec<String> = variants
                .iter()
                .map(|v| match v {
                    EnumVariant::Unit(n) => n.clone(),
                    EnumVariant::Tuple(n, _) => n.clone(),
                    EnumVariant::Struct(n, _) => n.clone(),
                })
                .collect();
            register_enum_variants(name.clone(), variant_names_list.clone());
            
            // Properly lower enum variants
            let variant_names: Vec<(String, HirType)> = variant_names_list
                .iter()
                .map(|variant_name| {
                    (variant_name.clone(), HirType::Named(name.clone()))
                })
                .collect();
            
            Ok(HirItem::Struct {
                name: name.clone(),
                fields: variant_names,
            })
        }

        Item::Trait { name, generics: _, supertraits: _, methods, is_pub: _, attributes: _, where_clause: _ } => {
            let mut fields = Vec::new();
            
            for item in methods {
                match item {
                    Item::Function { name: fn_name, .. } => {
                        fields.push((fn_name.clone(), HirType::Unknown));
                    }
                    Item::AssociatedType { name: assoc_name, .. } => {
                        fields.push((format!("_assoc_{}", assoc_name), HirType::Unknown));
                    }
                    _ => {}
                }
            }
            
            Ok(HirItem::Struct {
                name: format!("trait_{}", name),
                fields,
            })
        }

        Item::Impl {
            generics: _,
            trait_name: _,
            struct_name,
            methods,
            is_unsafe: _,
            attributes: _,
            where_clause: _,
        } => {
            // Lower impl block methods with qualified names
            // This allows them to be called as Type::method(args)
            let methods_hir: Result<Vec<_>, _> = methods
                .iter()
                .filter_map(|item| {
                    if let Item::Function { name, params, return_type, body, .. } = item {
                        let qualified_name = format!("{}::{}", struct_name, name);
                        
                        let params_hir: Result<Vec<_>, _> = params
                            .iter()
                            .map(|param: &Parameter| {
                                let ptype_hir = lower_type(&param.ty);
                                match ptype_hir {
                                    Ok(t) => Ok((param.name.clone(), t)),
                                    Err(e) => Err(e),
                                }
                            })
                            .collect();

                        let ret_type_hir = if let Some(rt) = return_type {
                            Some(lower_type(rt).ok())
                        } else {
                            None
                        };

                        let body_hir = lower_block(body);

                        return Some(match (params_hir, ret_type_hir, body_hir) {
                            (Ok(p), Some(Some(r)), Ok(b)) => {
                                Ok(HirItem::Function {
                                    name: qualified_name,
                                    generics: vec![],
                                    params: p,
                                    return_type: Some(r),
                                    body: b,
                                })
                            }
                            (Ok(p), None, Ok(b)) => {
                                Ok(HirItem::Function {
                                    name: qualified_name,
                                    generics: vec![],
                                    params: p,
                                    return_type: None,
                                    body: b,
                                })
                            }
                            _ => Err(LowerError {
                                message: format!("Failed to lower impl method {}", name),
                            }),
                        });
                    }
                    None
                })
                .collect();
            
            // Return a marker struct that impl was processed
            // The actual method functions will be collected by the compiler
            Ok(HirItem::Struct {
                name: format!("impl_{}", struct_name),
                fields: vec![],
            })
        }

        Item::Module { name, items: module_items, is_inline: _, is_pub, attributes: _ } => {
            // Recursively lower all items in the module
            let mut lowered_items = Vec::new();
            for item in module_items {
                lowered_items.push(lower_item(item)?);
            }
            Ok(HirItem::Module {
                name: name.clone(),
                items: lowered_items,
                is_public: *is_pub,
            })
        }

        Item::Use { path, is_glob, is_public, attributes: _ } => {
            Ok(HirItem::Use {
                path: path.clone(),
                is_glob: *is_glob,
                is_public: *is_public,
            })
        }

        Item::TypeAlias { name, generics: _, ty, is_pub: _, attributes: _ } => {
            Ok(HirItem::Struct {
                name: name.clone(),
                fields: vec![(format!("_alias"), convert_type(ty))],
            })
        }

        Item::Const { name, ty, value: _, is_pub, attributes: _ } => {
            Ok(HirItem::Const {
                name: name.clone(),
                ty: convert_type(ty),
                is_public: *is_pub,
                generics: Vec::new(),
            })
        }

        Item::Static { name, ty, value: _, is_mutable, is_pub, attributes: _ } => {
            Ok(HirItem::Static {
                name: name.clone(),
                ty: convert_type(ty),
                is_mutable: *is_mutable,
                is_public: *is_pub,
                generics: Vec::new(),
            })
        }

        Item::ExternBlock { abi: _, items: _, attributes: _ } => {
            Ok(HirItem::Struct {
                name: "extern".to_string(),
                fields: vec![(format!("_extern_marker"), HirType::Tuple(vec![]))],
            })
        }

        Item::MacroDefinition { name, rules: _, attributes: _ } => {
            Ok(HirItem::Struct {
                name: format!("macro_{}", name),
                fields: Vec::new(),
            })
        }

        Item::AssociatedType { name, bounds: _, ty, attributes: _ } => {
            let ty_hir = if let Some(t) = ty {
                Some(lower_type(t)?)
            } else {
                None
            };

            Ok(HirItem::AssociatedType {
                name: name.clone(),
                ty: ty_hir,
            })
        }
    }
}

/// Lower the entire AST to HIR
pub fn lower(ast: &[Item]) -> LowerResult<Vec<HirItem>> {
    clear_enum_registry();
    clear_struct_registry();
    
    // Helper function to replace Self with actual struct name in types
    fn replace_self_in_type(ty: &Type, struct_name: &str) -> Type {
        match ty {
            Type::Named(n) if n == "Self" => Type::Named(struct_name.to_string()),
            Type::Reference { lifetime, mutable, inner } => {
                Type::Reference {
                    lifetime: lifetime.clone(),
                    mutable: *mutable,
                    inner: Box::new(replace_self_in_type(inner, struct_name)),
                }
            }
            Type::Pointer { mutable, inner } => {
                Type::Pointer {
                    mutable: *mutable,
                    inner: Box::new(replace_self_in_type(inner, struct_name)),
                }
            }
            Type::Array { element, size } => {
                Type::Array {
                    element: Box::new(replace_self_in_type(element, struct_name)),
                    size: size.clone(),
                }
            }
            Type::Generic { name, type_args } => {
                Type::Generic {
                    name: name.clone(),
                    type_args: type_args.iter().map(|p| replace_self_in_type(p, struct_name)).collect(),
                }
            }
            other => other.clone(),
        }
    }
    
    // Helper function to replace Self in parameters
    fn replace_self_in_param(param: &Parameter, struct_name: &str) -> Parameter {
        Parameter {
            name: param.name.clone(),
            ty: replace_self_in_type(&param.ty, struct_name),
            mutable: param.mutable,
        }
    }
    
    // First pass: register enums and structs, and collect impl methods
    let mut all_items = ast.to_vec();
    
    for item in ast {
        if let Item::Enum { name, variants, .. } = item {
            let variant_names: Vec<String> = variants
                .iter()
                .map(|v| match v {
                    EnumVariant::Unit(n) => n.clone(),
                    EnumVariant::Tuple(n, _) => n.clone(),
                    EnumVariant::Struct(n, _) => n.clone(),
                })
                .collect();
            register_enum_variants(name.clone(), variant_names);
        } else if let Item::Struct { name, fields, .. } = item {
            // Register struct fields with their types for later type inference
            let field_types: Vec<(String, HirType)> = fields
                .iter()
                .map(|f| (f.name.clone(), lower_type(&f.ty).unwrap_or(HirType::Unknown)))
                .collect();
            register_struct_fields(name.clone(), field_types);
        }
    }
    
    // Extract impl block methods and add them as regular functions with qualified names
    let mut expanded_items = Vec::new();
    for item in &all_items {
        if let Item::Impl { struct_name, methods, .. } = item {
            for method in methods {
                if let Item::Function {
                    name,
                    generics,
                    params,
                    return_type,
                    body,
                    is_unsafe,
                    is_async,
                    is_pub,
                    attributes,
                    where_clause,
                    abi,
                } = method
                {
                    let qualified_name = format!("{}::{}", struct_name, name);
                    // Replace Self in return type and parameters
                    let new_return_type = return_type.as_ref().map(|rt| replace_self_in_type(rt, struct_name));
                    let new_params: Vec<Parameter> = params.iter().map(|p| replace_self_in_param(p, struct_name)).collect();
                    
                    expanded_items.push(Item::Function {
                        name: qualified_name,
                        generics: generics.clone(),
                        params: new_params,
                        return_type: new_return_type,
                        body: body.clone(),
                        is_unsafe: *is_unsafe,
                        is_async: *is_async,
                        is_pub: *is_pub,
                        attributes: attributes.clone(),
                        where_clause: where_clause.clone(),
                        abi: abi.clone(),
                    });
                }
            }
        }
    }
    
    // Generate constructors for enum variants
    for item in &all_items {
        if let Item::Enum { name: enum_name, variants, generics, .. } = item {
            for variant in variants {
                match variant {
                    EnumVariant::Unit(variant_name) => {
                        // Unit variants don't need constructors, they're constants
                    }
                    EnumVariant::Tuple(variant_name, types) => {
                        // Create a function: Variant(T1, T2, ...) -> EnumName
                        let params: Vec<Parameter> = types
                            .iter()
                            .enumerate()
                            .map(|(i, ty)| Parameter {
                                name: format!("arg{}", i),
                                ty: ty.clone(),
                                mutable: false,
                            })
                            .collect();
                        
                        let return_type = Type::Named(enum_name.clone());
                        
                        expanded_items.push(Item::Function {
                            name: format!("{}::{}", enum_name, variant_name),
                            generics: generics.clone(),
                            params,
                            return_type: Some(return_type),
                            body: Block { statements: vec![], expression: None }, // Empty body - lowering will handle
                            is_unsafe: false,
                            is_async: false,
                            is_pub: true,
                            attributes: vec![],
                            where_clause: vec![],
                            abi: None,
                        });
                    }
                    EnumVariant::Struct(variant_name, fields) => {
                        // Create a function with named parameters
                        let params: Vec<Parameter> = fields
                            .iter()
                            .map(|field| Parameter {
                                name: field.name.clone(),
                                ty: field.ty.clone(),
                                mutable: false,
                            })
                            .collect();
                        
                        let return_type = Type::Named(enum_name.clone());
                        
                        expanded_items.push(Item::Function {
                            name: format!("{}::{}", enum_name, variant_name),
                            generics: generics.clone(),
                            params,
                            return_type: Some(return_type),
                            body: Block { statements: vec![], expression: None }, // Empty body - lowering will handle
                            is_unsafe: false,
                            is_async: false,
                            is_pub: true,
                            attributes: vec![],
                            where_clause: vec![],
                            abi: None,
                        });
                    }
                }
            }
        }
    }
    
    // Add regular items (non-impl)
    for item in &all_items {
        if !matches!(item, Item::Impl { .. }) {
            expanded_items.push(item.clone());
        }
    }
    
    expanded_items.iter().map(lower_item).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lower_type_primitives() {
        assert_eq!(
            lower_type(&Type::Named("i32".to_string())).unwrap(),
            HirType::Int32
        );
        assert_eq!(
            lower_type(&Type::Named("f64".to_string())).unwrap(),
            HirType::Float64
        );
    }

    #[test]
    fn test_lower_expression_literal() {
        let expr = Expression::Integer(42);
        match lower_expression(&expr) {
            Ok(HirExpression::Integer(42)) => {},
            Ok(other) => panic!("Expected Integer(42), got {:?}", other),
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }
}