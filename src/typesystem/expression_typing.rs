//! # Expression Typing Module
//!
//! Walks AST expressions and generates constraints for type inference.
//! This is the critical bridge between the parser's AST and the constraint solver.
//!
//! ## Design:
//! 1. Each expression gets assigned a type variable
//! 2. Walk expression tree recursively
//! 3. Generate constraints from operations
//! 4. Solve constraints to get concrete types
//! 5. Return expression with inferred types

use super::types::{Type, TypeVar};
use super::constraints::{ConstraintGenerator, BinaryOp, UnaryOp};
use super::substitution::Substitution;
use std::collections::HashMap;

/// Result of expression typing
pub type ExprTypingResult<T> = Result<T, ExprTypingError>;

/// Error during expression typing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprTypingError {
    pub message: String,
}

impl ExprTypingError {
    pub fn new(msg: impl Into<String>) -> Self {
        ExprTypingError {
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for ExprTypingError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Type Error: {}", self.message)
    }
}

/// A simplified AST expression for typing (from parser)
#[derive(Debug, Clone, PartialEq)]
pub enum AstExpr {
    Integer(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Variable(String),
    BinaryOp {
        left: Box<AstExpr>,
        op: AstBinaryOp,
        right: Box<AstExpr>,
    },
    UnaryOp {
        op: AstUnaryOp,
        operand: Box<AstExpr>,
    },
    FunctionCall {
        name: String,
        args: Vec<AstExpr>,
    },
    MethodCall {
        object: Box<AstExpr>,
        method: String,
        args: Vec<AstExpr>,
    },
    FieldAccess {
        object: Box<AstExpr>,
        field: String,
    },
    Array(Vec<AstExpr>),
    Tuple(Vec<AstExpr>),
    Index {
        array: Box<AstExpr>,
        index: Box<AstExpr>,
    },
    Assign {
        target: Box<AstExpr>,
        value: Box<AstExpr>,
    },
}

/// Binary operators from AST (need to map to constraint system)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
}

impl AstBinaryOp {
    /// Convert AST operator to constraint system operator
    pub fn to_constraint_op(self) -> BinaryOp {
        match self {
            AstBinaryOp::Add => BinaryOp::Add,
            AstBinaryOp::Sub => BinaryOp::Subtract,
            AstBinaryOp::Mul => BinaryOp::Multiply,
            AstBinaryOp::Div => BinaryOp::Divide,
            AstBinaryOp::Mod => BinaryOp::Modulo,
            AstBinaryOp::Eq => BinaryOp::Equal,
            AstBinaryOp::Ne => BinaryOp::NotEqual,
            AstBinaryOp::Lt => BinaryOp::Less,
            AstBinaryOp::Le => BinaryOp::LessEq,
            AstBinaryOp::Gt => BinaryOp::Greater,
            AstBinaryOp::Ge => BinaryOp::GreaterEq,
            AstBinaryOp::And => BinaryOp::And,
            AstBinaryOp::Or => BinaryOp::Or,
            AstBinaryOp::BitwiseAnd => BinaryOp::BitwiseAnd,
            AstBinaryOp::BitwiseOr => BinaryOp::BitwiseOr,
            AstBinaryOp::BitwiseXor => BinaryOp::BitwiseXor,
            AstBinaryOp::LeftShift => BinaryOp::LeftShift,
            AstBinaryOp::RightShift => BinaryOp::RightShift,
        }
    }
}

/// Unary operators from AST
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstUnaryOp {
    Negate,
    Not,
    BitwiseNot,
    Deref,
    Reference,
    MutableReference,
}

impl AstUnaryOp {
    /// Convert AST operator to constraint system operator
    pub fn to_constraint_op(self) -> UnaryOp {
        match self {
            AstUnaryOp::Negate => UnaryOp::Negate,
            AstUnaryOp::Not => UnaryOp::Not,
            AstUnaryOp::BitwiseNot => UnaryOp::BitwiseNot,
            AstUnaryOp::Deref => UnaryOp::Dereference,
            AstUnaryOp::Reference => UnaryOp::Reference,
            AstUnaryOp::MutableReference => UnaryOp::MutableReference,
        }
    }
}

/// Typed expression: expression with inferred type
#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    pub expr: AstExpr,
    pub ty: Type,
}

impl TypedExpr {
    pub fn new(expr: AstExpr, ty: Type) -> Self {
        TypedExpr { expr, ty }
    }
}

/// Expression typer: main engine for type inference
pub struct ExprTyper {
    pub generator: ConstraintGenerator,
    expr_types: HashMap<String, TypeVar>,
    next_expr_id: usize,
}

impl ExprTyper {
    /// Create a new expression typer
    pub fn new() -> Self {
        ExprTyper {
            generator: ConstraintGenerator::new(),
            expr_types: HashMap::new(),
            next_expr_id: 0,
        }
    }

    /// Register a variable with a known type
    pub fn register_variable(&mut self, name: String, ty: Type) -> ExprTypingResult<()> {
        let var = self.generator.fresh_var();
        self.generator.symbols.insert(name, var);
        self.generator.add_constraint(Type::Variable(var), ty);
        Ok(())
    }

    /// Register a function signature
    pub fn register_function(
        &mut self,
        name: String,
        param_types: Vec<Type>,
        return_type: Type,
    ) -> ExprTypingResult<()> {
        self.generator.register_function(name, param_types, return_type);
        Ok(())
    }

    /// Get a fresh expression ID for internal tracking
    fn next_expr_id(&mut self) -> usize {
        let id = self.next_expr_id;
        self.next_expr_id += 1;
        id
    }

    /// Type an expression and return its inferred type
    /// This is the main entry point
    pub fn type_expr(&mut self, expr: &AstExpr) -> ExprTypingResult<TypedExpr> {
        let ty = self.infer_expr(expr)?;
        Ok(TypedExpr::new(expr.clone(), ty))
    }

    /// Recursively infer the type of an expression
    fn infer_expr(&mut self, expr: &AstExpr) -> ExprTypingResult<Type> {
        match expr {
            AstExpr::Integer(_) => {
                // Integers default to i32
                Ok(Type::I32)
            }

            AstExpr::Float(_) => {
                // Floats default to f64
                Ok(Type::F64)
            }

            AstExpr::Bool(_) => {
                // Booleans are always bool
                Ok(Type::Bool)
            }

            AstExpr::String(_) => {
                // Strings are always str
                Ok(Type::Str)
            }

            AstExpr::Variable(name) => {
                // Look up variable in symbol table
                if let Some(var) = self.generator.symbols.get(name) {
                    Ok(Type::Variable(*var))
                } else {
                    Err(ExprTypingError::new(format!(
                        "Unknown variable: {}",
                        name
                    )))
                }
            }

            AstExpr::BinaryOp { left, op, right } => {
                let left_ty = self.infer_expr(left)?;
                let right_ty = self.infer_expr(right)?;

                let constraint_op = op.to_constraint_op();
                self.generator
                    .constrain_binary_op(constraint_op, left_ty, right_ty)
                    .map_err(|e| ExprTypingError::new(e))
            }

            AstExpr::UnaryOp { op, operand } => {
                let operand_ty = self.infer_expr(operand)?;
                let constraint_op = op.to_constraint_op();

                self.generator
                    .constrain_unary_op(constraint_op, operand_ty)
                    .map_err(|e| ExprTypingError::new(e))
            }

            AstExpr::FunctionCall { name, args } => {
                let arg_types: Result<Vec<_>, _> =
                    args.iter().map(|arg| self.infer_expr(arg)).collect();

                self.generator
                    .constrain_function_call(name, arg_types?)
                    .map_err(|e| ExprTypingError::new(e))
            }

            AstExpr::MethodCall { object, method, args } => {
                let receiver_ty = self.infer_expr(object)?;
                let arg_types: Result<Vec<_>, _> =
                    args.iter().map(|arg| self.infer_expr(arg)).collect();

                self.generator
                    .constrain_method_call(&receiver_ty, method, arg_types?)
                    .map_err(|e| ExprTypingError::new(e))
            }

            AstExpr::Array(elements) => {
                if elements.is_empty() {
                    // Empty array - type variable for element type
                    let elem_var = self.generator.fresh_var();
                    Ok(Type::Array {
                        element: Box::new(Type::Variable(elem_var)),
                        size: 0,
                    })
                } else {
                    let first_ty = self.infer_expr(&elements[0])?;

                    // All elements must have the same type
                    for elem in &elements[1..] {
                        let elem_ty = self.infer_expr(elem)?;
                        self.generator.add_constraint(first_ty.clone(), elem_ty);
                    }

                    Ok(Type::Array {
                        element: Box::new(first_ty),
                        size: elements.len(),
                    })
                }
            }

            AstExpr::Tuple(elements) => {
                let elem_types: Result<Vec<_>, _> =
                    elements.iter().map(|e| self.infer_expr(e)).collect();
                Ok(Type::Tuple(elem_types?))
            }

            AstExpr::Index { array, index } => {
                let array_ty = self.infer_expr(array)?;
                let _index_ty = self.infer_expr(index)?;

                // Indexing returns the element type of the array
                match array_ty {
                    Type::Array { element, .. } => Ok(*element),
                    Type::Tuple(ref elems) if elems.len() == 1 => Ok(elems[0].clone()),
                    _ => Err(ExprTypingError::new(
                        "Cannot index non-array type".to_string(),
                    )),
                }
            }

            AstExpr::FieldAccess { object, field } => {
                let obj_ty = self.infer_expr(object)?;
                // Look up the field type from struct definition
                self.generator
                    .constrain_field_access(&obj_ty, field)
                    .map_err(|e| ExprTypingError::new(e))
            }

            AstExpr::Assign { target, value } => {
                let value_ty = self.infer_expr(value)?;
                let target_ty = self.infer_expr(target)?;

                // Assignment: target type must match value type
                self.generator.add_constraint(target_ty, value_ty.clone());
                Ok(value_ty)
            }
        }
    }

    /// Type check a list of expressions in sequence
    pub fn type_exprs(&mut self, exprs: &[AstExpr]) -> ExprTypingResult<Vec<TypedExpr>> {
        exprs
            .iter()
            .map(|e| self.type_expr(e))
            .collect()
    }

    /// Solve all collected constraints and return substitution
    pub fn solve(&mut self) -> ExprTypingResult<Substitution> {
        self.generator.solve().map_err(|e| ExprTypingError::new(e))
    }

    /// Get the final substitution
    pub fn get_solution(&mut self) -> ExprTypingResult<Substitution> {
        self.solve()
    }

    /// Fully resolve a type using the current substitution
    pub fn resolve_type(&mut self, ty: &Type) -> ExprTypingResult<Type> {
        let subst = self.solve()?;
        Ok(subst.apply(ty))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_types() {
        let mut typer = ExprTyper::new();

        let int_expr = AstExpr::Integer(42);
        let int_type = typer.infer_expr(&int_expr).unwrap();
        assert_eq!(int_type, Type::I32);

        let float_expr = AstExpr::Float(3.14);
        let float_type = typer.infer_expr(&float_expr).unwrap();
        assert_eq!(float_type, Type::F64);

        let bool_expr = AstExpr::Bool(true);
        let bool_type = typer.infer_expr(&bool_expr).unwrap();
        assert_eq!(bool_type, Type::Bool);

        let string_expr = AstExpr::String("hello".to_string());
        let string_type = typer.infer_expr(&string_expr).unwrap();
        assert_eq!(string_type, Type::Str);
    }

    #[test]
    fn test_variable_typing() {
        let mut typer = ExprTyper::new();

        // Register variable x: i32
        typer.register_variable("x".to_string(), Type::I32).unwrap();

        let var_expr = AstExpr::Variable("x".to_string());
        let var_type = typer.infer_expr(&var_expr).unwrap();
        
        // Should resolve to i32 after solving
        let subst = typer.solve().unwrap();
        let resolved = subst.apply(&var_type);
        assert_eq!(resolved, Type::I32);
    }

    #[test]
    fn test_binary_operation() {
        let mut typer = ExprTyper::new();

        // 5 + 3
        let expr = AstExpr::BinaryOp {
            left: Box::new(AstExpr::Integer(5)),
            op: AstBinaryOp::Add,
            right: Box::new(AstExpr::Integer(3)),
        };

        let result_type = typer.infer_expr(&expr).unwrap();
        let subst = typer.solve().unwrap();
        let resolved = subst.apply(&result_type);
        assert_eq!(resolved, Type::I32);
    }

    #[test]
    fn test_comparison_operation() {
        let mut typer = ExprTyper::new();

        // 5 < 10
        let expr = AstExpr::BinaryOp {
            left: Box::new(AstExpr::Integer(5)),
            op: AstBinaryOp::Lt,
            right: Box::new(AstExpr::Integer(10)),
        };

        let result_type = typer.infer_expr(&expr).unwrap();
        assert_eq!(result_type, Type::Bool);
    }

    #[test]
    fn test_logical_operation() {
        let mut typer = ExprTyper::new();

        // true && false
        let expr = AstExpr::BinaryOp {
            left: Box::new(AstExpr::Bool(true)),
            op: AstBinaryOp::And,
            right: Box::new(AstExpr::Bool(false)),
        };

        let result_type = typer.infer_expr(&expr).unwrap();
        assert_eq!(result_type, Type::Bool);
    }

    #[test]
    fn test_unary_negation() {
        let mut typer = ExprTyper::new();

        // -42
        let expr = AstExpr::UnaryOp {
            op: AstUnaryOp::Negate,
            operand: Box::new(AstExpr::Integer(42)),
        };

        let result_type = typer.infer_expr(&expr).unwrap();
        assert_eq!(result_type, Type::I32);
    }

    #[test]
    fn test_unary_not() {
        let mut typer = ExprTyper::new();

        // !true
        let expr = AstExpr::UnaryOp {
            op: AstUnaryOp::Not,
            operand: Box::new(AstExpr::Bool(true)),
        };

        let result_type = typer.infer_expr(&expr).unwrap();
        assert_eq!(result_type, Type::Bool);
    }

    #[test]
    fn test_reference_creation() {
        let mut typer = ExprTyper::new();

        // &42
        let expr = AstExpr::UnaryOp {
            op: AstUnaryOp::Reference,
            operand: Box::new(AstExpr::Integer(42)),
        };

        let result_type = typer.infer_expr(&expr).unwrap();
        assert!(matches!(result_type, Type::Reference { mutable: false, .. }));
    }

    #[test]
    fn test_mutable_reference() {
        let mut typer = ExprTyper::new();
        typer.register_variable("x".to_string(), Type::I32).unwrap();

        // &mut x
        let expr = AstExpr::UnaryOp {
            op: AstUnaryOp::MutableReference,
            operand: Box::new(AstExpr::Variable("x".to_string())),
        };

        let result_type = typer.infer_expr(&expr).unwrap();
        assert!(matches!(result_type, Type::Reference { mutable: true, .. }));
    }

    #[test]
    fn test_array_typing() {
        let mut typer = ExprTyper::new();

        // [1, 2, 3]
        let expr = AstExpr::Array(vec![
            AstExpr::Integer(1),
            AstExpr::Integer(2),
            AstExpr::Integer(3),
        ]);

        let result_type = typer.infer_expr(&expr).unwrap();
        assert!(matches!(result_type, Type::Array { .. }));
    }

    #[test]
    fn test_tuple_typing() {
        let mut typer = ExprTyper::new();

        // (1, true, "hello")
        let expr = AstExpr::Tuple(vec![
            AstExpr::Integer(1),
            AstExpr::Bool(true),
            AstExpr::String("hello".to_string()),
        ]);

        let result_type = typer.infer_expr(&expr).unwrap();
        assert!(matches!(result_type, Type::Tuple(_)));
    }

    #[test]
    fn test_function_call_typing() {
        let mut typer = ExprTyper::new();

        // Register: fn add(x: i32, y: i32) -> i32
        typer
            .register_function(
                "add".to_string(),
                vec![Type::I32, Type::I32],
                Type::I32,
            )
            .unwrap();

        // add(5, 3)
        let expr = AstExpr::FunctionCall {
            name: "add".to_string(),
            args: vec![AstExpr::Integer(5), AstExpr::Integer(3)],
        };

        let result_type = typer.infer_expr(&expr).unwrap();
        assert_eq!(result_type, Type::I32);
    }

    #[test]
    fn test_function_call_type_error() {
        let mut typer = ExprTyper::new();

        // Register: fn double(x: i32) -> i32
        typer
            .register_function("double".to_string(), vec![Type::I32], Type::I32)
            .unwrap();

        // double(5, 3) - wrong arity!
        let expr = AstExpr::FunctionCall {
            name: "double".to_string(),
            args: vec![AstExpr::Integer(5), AstExpr::Integer(3)],
        };

        let result = typer.infer_expr(&expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_expression() {
        let mut typer = ExprTyper::new();

        // (5 + 3) * 2
        let expr = AstExpr::BinaryOp {
            left: Box::new(AstExpr::BinaryOp {
                left: Box::new(AstExpr::Integer(5)),
                op: AstBinaryOp::Add,
                right: Box::new(AstExpr::Integer(3)),
            }),
            op: AstBinaryOp::Mul,
            right: Box::new(AstExpr::Integer(2)),
        };

        let result_type = typer.infer_expr(&expr).unwrap();
        let subst = typer.solve().unwrap();
        let resolved = subst.apply(&result_type);
        assert_eq!(resolved, Type::I32);
    }

    #[test]
    fn test_constraint_solving_with_variables() {
        let mut typer = ExprTyper::new();

        // x + 5 where x must be numeric
        typer.register_variable("x".to_string(), Type::I32).unwrap();

        let expr = AstExpr::BinaryOp {
            left: Box::new(AstExpr::Variable("x".to_string())),
            op: AstBinaryOp::Add,
            right: Box::new(AstExpr::Integer(5)),
        };

        let result_type = typer.infer_expr(&expr).unwrap();
        let subst = typer.solve().unwrap();
        let resolved = subst.apply(&result_type);

        // Should resolve to i32
        assert_eq!(resolved, Type::I32);
    }

    #[test]
    fn test_type_expr_wrapper() {
        let mut typer = ExprTyper::new();

        let expr = AstExpr::Integer(42);
        let typed = typer.type_expr(&expr).unwrap();

        assert_eq!(typed.ty, Type::I32);
        assert_eq!(typed.expr, expr);
    }

    #[test]
    fn test_multiple_expressions() {
        let mut typer = ExprTyper::new();

        let exprs = vec![
            AstExpr::Integer(42),
            AstExpr::Float(3.14),
            AstExpr::Bool(true),
        ];

        let typed = typer.type_exprs(&exprs).unwrap();
        assert_eq!(typed.len(), 3);
        assert_eq!(typed[0].ty, Type::I32);
        assert_eq!(typed[1].ty, Type::F64);
        assert_eq!(typed[2].ty, Type::Bool);
    }
}