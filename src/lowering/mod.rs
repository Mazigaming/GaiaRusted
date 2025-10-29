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

use crate::parser::{self, Expression, Statement, Item, Type, Block, Parameter, StructField};
use std::fmt;

/// High-Level Intermediate Representation (HIR)
/// Similar to AST but with syntactic sugar removed
#[derive(Debug, Clone)]
pub enum HirItem {
    /// Function definition
    Function {
        name: String,
        params: Vec<(String, HirType)>,
        return_type: Option<HirType>,
        body: Vec<HirStatement>,
    },
    /// Struct definition
    Struct {
        name: String,
        fields: Vec<(String, HirType)>,
    },
}

/// HIR statements (simplified from parser statements)
#[derive(Debug, Clone)]
pub enum HirStatement {
    /// Variable binding: let x: i32 = 42;
    Let {
        name: String,
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

    /// Field access: obj.field
    FieldAccess {
        object: Box<HirExpression>,
        field: String,
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

    /// Range literal: 0..10, 1..=5
    Range {
        start: Option<Box<HirExpression>>,
        end: Option<Box<HirExpression>>,
        inclusive: bool,
    },

    /// Block: { statements... ; expression }
    Block(Vec<HirStatement>, Option<Box<HirExpression>>),
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
    Negate,      // -x
    Not,         // !x
    BitwiseNot,  // ~x
    Dereference, // *x
    Reference,   // &x
}

/// HIR Types (simplified from parser types)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirType {
    /// Primitive types
    Int32,
    Int64,
    Float64,
    Bool,
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

    /// Unknown type (will be inferred later)
    Unknown,
}

impl fmt::Display for HirType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HirType::Int32 => write!(f, "i32"),
            HirType::Int64 => write!(f, "i64"),
            HirType::Float64 => write!(f, "f64"),
            HirType::Bool => write!(f, "bool"),
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

/// Convert parsed types to HIR types
fn lower_type(ty: &Type) -> LowerResult<HirType> {
    match ty {
        Type::Named(name) => match name.as_str() {
            "i32" => Ok(HirType::Int32),
            "i64" => Ok(HirType::Int64),
            "f64" => Ok(HirType::Float64),
            "bool" => Ok(HirType::Bool),
            "str" => Ok(HirType::String),
            _ => Ok(HirType::Named(name.clone())),
        },
        Type::Reference { mutable: _, inner } => {
            let inner_hir = lower_type(inner)?;
            Ok(HirType::Reference(Box::new(inner_hir)))
        }
        Type::Pointer { mutable: _, inner } => {
            let inner_hir = lower_type(inner)?;
            Ok(HirType::Pointer(Box::new(inner_hir)))
        }
        Type::Array { element, size: _ } => {
            let elem_hir = lower_type(element)?;
            // For now, we ignore the size expression and treat as unknown size
            Ok(HirType::Array {
                element_type: Box::new(elem_hir),
                size: None,
            })
        }
        Type::Function {
            params,
            return_type,
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
    }
}

/// Lower an expression from AST to HIR
fn lower_expression(expr: &Expression) -> LowerResult<HirExpression> {
    match expr {
        Expression::Integer(n) => Ok(HirExpression::Integer(*n)),
        Expression::Float(f) => Ok(HirExpression::Float(*f)),
        Expression::String(s) => Ok(HirExpression::String(s.clone())),
        Expression::Bool(b) => Ok(HirExpression::Bool(*b)),
        Expression::Char(_c) => {
            // For now, treat char as a single-character string
            Err(LowerError {
                message: "Char literals not yet supported".to_string(),
            })
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

        Expression::Loop(_body) => {
            // TODO: Desugar loop to while true
            Err(LowerError {
                message: "Loop expressions not yet desugared (TODO)".to_string(),
            })
        }

        Expression::Match {
            scrutinee,
            arms: _,
        } => {
            let _scrutinee_hir = lower_expression(scrutinee)?;
            // TODO: Desugar match expressions
            Err(LowerError {
                message: "Match expressions not yet desugared (TODO)".to_string(),
            })
        }

        Expression::FunctionCall { name, args } => {
            let args_hir: Result<Vec<_>, _> =
                args.iter().map(|arg| lower_expression(arg)).collect();
            Ok(HirExpression::Call {
                func: Box::new(HirExpression::Variable(name.clone())),
                args: args_hir?,
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
                // For now, just try to iterate directly
                // This is a simplified version - proper implementation would use iterators
                Ok(HirExpression::While {
                    condition: Box::new(HirExpression::Bool(true)), // TODO: proper iterator support
                    body: body_stmts,
                })
            }
        }

        Expression::Closure { params: _, body: _ } => {
            // Closures are more complex - for now, treat them as errors
            // A proper implementation would:
            // 1. Create a synthetic function
            // 2. Capture variables from outer scope
            // 3. Return a function pointer or trait object
            Err(LowerError {
                message: "Closures not yet supported (requires proper capture semantics)".to_string(),
            })
        }
    }
}

/// Lower a block (statements + optional expression)
fn lower_block(block: &Block) -> LowerResult<Vec<HirStatement>> {
    lower_statements(&block.statements)
}

/// Lower a statement from AST to HIR
fn lower_statement(stmt: &Statement) -> LowerResult<HirStatement> {
    match stmt {
        Statement::Let {
            name,
            mutable: _,
            ty: type_opt,
            initializer,
        } => {
            let init_hir = lower_expression(initializer)?;
            // Infer or use provided type
            let ty = if let Some(t) = type_opt {
                lower_type(t)?
            } else {
                HirType::Unknown // Will be inferred in Phase 4
            };
            Ok(HirStatement::Let {
                name: name.clone(),
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

        Statement::Break => Ok(HirStatement::Break),

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
            let then_hir = lower_statements(&then_body.statements)?;
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
    }
}

/// Lower a list of statements
fn lower_statements(stmts: &[Statement]) -> LowerResult<Vec<HirStatement>> {
    stmts.iter().map(lower_statement).collect()
}

/// Lower an item from AST to HIR
fn lower_item(item: &Item) -> LowerResult<HirItem> {
    match item {
        Item::Function {
            name,
            params,
            return_type,
            body,
        } => {
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
                params: params_hir?,
                return_type: ret_type_hir,
                body: body_hir,
            })
        }

        Item::Struct { name, fields } => {
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

        Item::Enum { name, variants: _ } => {
            // TODO: Handle enum lowering
            Ok(HirItem::Struct {
                name: format!("enum_{}", name),
                fields: Vec::new(),
            })
        }

        Item::Trait { name, methods: _ } => {
            // TODO: Handle trait lowering
            Ok(HirItem::Struct {
                name: format!("trait_{}", name),
                fields: Vec::new(),
            })
        }

        Item::Impl {
            trait_name: _,
            struct_name,
            methods: _,
        } => {
            // TODO: Handle impl lowering
            Ok(HirItem::Struct {
                name: format!("impl_{}", struct_name),
                fields: Vec::new(),
            })
        }

        Item::Module { name, items: _ } => {
            // TODO: Handle module lowering
            Ok(HirItem::Struct {
                name: format!("mod_{}", name),
                fields: Vec::new(),
            })
        }

        Item::Use { path } => {
            // TODO: Handle use statements
            Ok(HirItem::Struct {
                name: format!("use_{}", path),
                fields: Vec::new(),
            })
        }
    }
}

/// Lower the entire AST to HIR
pub fn lower(ast: &[Item]) -> LowerResult<Vec<HirItem>> {
    ast.iter().map(lower_item).collect()
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