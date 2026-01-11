//! # Constraint-Based Type Solver
//!
//! Advanced type inference using the constraint system.
//! This module provides a complete type solving pipeline:
//! 1. Generate constraints from expressions
//! 2. Collect all constraints
//! 3. Solve via unification
//! 4. Apply substitution to get concrete types
//!
//! This is more powerful than simple type checking because it can:
//! - Infer types for variables with incomplete information
//! - Detect type conflicts at the constraint level
//! - Support generic type resolution
//! - Generate informative error messages

use super::types::Type;
use super::expression_typing::AstExpr;
use crate::typesystem::ExprTyper;
use super::substitution::Substitution;
use std::collections::HashMap;

/// Error from constraint solving
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintError {
    pub message: String,
}

impl ConstraintError {
    pub fn new(msg: impl Into<String>) -> Self {
        ConstraintError {
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for ConstraintError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Constraint Error: {}", self.message)
    }
}

pub type ConstraintResult<T> = Result<T, ConstraintError>;

/// Type solution: mapping from expression/variable names to resolved types
#[derive(Debug, Clone)]
pub struct TypeSolution {
    /// Mapping from variable names to their inferred types
    pub bindings: HashMap<String, Type>,
    /// The substitution used to solve constraints
    pub substitution: Substitution,
}

impl TypeSolution {
    /// Create a new empty solution
    pub fn new() -> Self {
        TypeSolution {
            bindings: HashMap::new(),
            substitution: Substitution::new(),
        }
    }

    /// Look up a variable's type
    pub fn lookup(&self, name: &str) -> Option<&Type> {
        self.bindings.get(name)
    }

    /// Add a binding
    pub fn insert(&mut self, name: String, ty: Type) {
        self.bindings.insert(name, ty);
    }

    /// Resolve a type using the substitution
    pub fn resolve(&self, ty: &Type) -> Type {
        self.substitution.apply(ty)
    }
}

/// Constraint-based type solver
pub struct ConstraintSolver {
    typer: ExprTyper,
}

impl ConstraintSolver {
    /// Create a new constraint solver
    pub fn new() -> Self {
        ConstraintSolver {
            typer: ExprTyper::new(),
        }
    }

    /// Register a variable with a known type
    pub fn register_variable(&mut self, name: String, ty: Type) -> ConstraintResult<()> {
        self.typer
            .register_variable(name, ty)
            .map_err(|e| ConstraintError::new(e.to_string()))
    }

    /// Register a function signature
    pub fn register_function(
        &mut self,
        name: String,
        param_types: Vec<Type>,
        return_type: Type,
    ) -> ConstraintResult<()> {
        self.typer
            .register_function(name, param_types, return_type)
            .map_err(|e| ConstraintError::new(e.to_string()))
    }

    /// Type and solve a single expression
    pub fn solve_expr(&mut self, expr: &AstExpr) -> ConstraintResult<Type> {
        let typed = self.typer
            .type_expr(expr)
            .map_err(|e| ConstraintError::new(e.to_string()))?;

        let subst = self.typer
            .solve()
            .map_err(|e| ConstraintError::new(e.to_string()))?;

        Ok(subst.apply(&typed.ty))
    }

    /// Type and solve multiple expressions in sequence
    pub fn solve_exprs(&mut self, exprs: &[AstExpr]) -> ConstraintResult<Vec<Type>> {
        let mut results = Vec::new();

        for expr in exprs {
            let ty = self.solve_expr(expr)?;
            results.push(ty);
        }

        Ok(results)
    }

    /// Solve and return complete solution with all bindings
    pub fn get_solution(&mut self) -> ConstraintResult<TypeSolution> {
        let subst = self.typer
            .solve()
            .map_err(|e| ConstraintError::new(e.to_string()))?;

        // Build bindings from symbol table
        let mut bindings = HashMap::new();
        for (name, var) in &self.typer.generator.symbols {
            let ty = subst.apply(&Type::Variable(*var));
            bindings.insert(name.clone(), ty);
        }

        Ok(TypeSolution {
            bindings,
            substitution: subst,
        })
    }

    /// Check if all constraints can be satisfied
    pub fn validate(&mut self) -> ConstraintResult<()> {
        self.typer
            .solve()
            .map_err(|e| ConstraintError::new(e.to_string()))?;
        Ok(())
    }
}

/// Multi-expression type checker using constraints
pub struct MultiExprTypeChecker {
    solver: ConstraintSolver,
}

impl MultiExprTypeChecker {
    /// Create a new multi-expression type checker
    pub fn new() -> Self {
        MultiExprTypeChecker {
            solver: ConstraintSolver::new(),
        }
    }

    /// Add a variable binding
    pub fn add_variable(&mut self, name: String, ty: Type) -> ConstraintResult<()> {
        self.solver.register_variable(name, ty)
    }

    /// Add a function signature
    pub fn add_function(
        &mut self,
        name: String,
        param_types: Vec<Type>,
        return_type: Type,
    ) -> ConstraintResult<()> {
        self.solver.register_function(name, param_types, return_type)
    }

    /// Type check an expression sequence (like a program block)
    pub fn check_block(&mut self, exprs: &[AstExpr]) -> ConstraintResult<TypeSolution> {
        // Type all expressions (generates constraints)
        for expr in exprs {
            self.solver
                .solve_expr(expr)?;
        }

        // Get final solution
        self.solver.get_solution()
    }

    /// Check a sequence of let bindings and expressions
    pub fn check_let_bindings(
        &mut self,
        bindings: &[(String, AstExpr)],
        final_expr: Option<&AstExpr>,
    ) -> ConstraintResult<TypeSolution> {
        // Process bindings
        for (name, init_expr) in bindings {
            let ty = self.solver.solve_expr(init_expr)?;
            self.solver.register_variable(name.clone(), ty)?;
        }

        // Process final expression if present
        if let Some(expr) = final_expr {
            self.solver.solve_expr(expr)?;
        }

        self.solver.get_solution()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typesystem::expression_typing::{AstBinaryOp, AstUnaryOp};

    #[test]
    fn test_simple_expression_solving() {
        let mut solver = ConstraintSolver::new();

        // 5 + 3
        let expr = AstExpr::BinaryOp {
            left: Box::new(AstExpr::Integer(5)),
            op: AstBinaryOp::Add,
            right: Box::new(AstExpr::Integer(3)),
        };

        let ty = solver.solve_expr(&expr)
            .expect("Failed to solve binary expression type");
        assert_eq!(ty, Type::I32, "Binary operation should resolve to I32");
    }

    #[test]
    fn test_variable_resolution() {
        let mut solver = ConstraintSolver::new();

        // Register x: i32
        solver.register_variable("x".to_string(), Type::I32)
            .expect("Failed to register variable x");

        // x + 5
        let expr = AstExpr::BinaryOp {
            left: Box::new(AstExpr::Variable("x".to_string())),
            op: AstBinaryOp::Add,
            right: Box::new(AstExpr::Integer(5)),
        };

        let ty = solver.solve_expr(&expr)
            .expect("Failed to solve variable expression type");
        assert_eq!(ty, Type::I32, "Variable expression should resolve to I32");
    }

    #[test]
    fn test_function_call_resolution() {
        let mut solver = ConstraintSolver::new();

        // Register: add(x: i32, y: i32) -> i32
        solver
            .register_function(
                "add".to_string(),
                vec![Type::I32, Type::I32],
                Type::I32,
            )
            .expect("Failed to register add function");

        // add(5, 3)
        let expr = AstExpr::FunctionCall {
            name: "add".to_string(),
            args: vec![AstExpr::Integer(5), AstExpr::Integer(3)],
        };

        let ty = solver.solve_expr(&expr)
            .expect("Failed to solve function call expression type");
        assert_eq!(ty, Type::I32, "Function call should resolve to I32");
    }

    #[test]
    fn test_function_call_type_error() {
        let mut solver = ConstraintSolver::new();

        // Register: add(x: i32, y: i32) -> i32
        solver
            .register_function(
                "add".to_string(),
                vec![Type::I32, Type::I32],
                Type::I32,
            )
            .expect("Failed to register add function");

        // add(5) - wrong arity!
        let expr = AstExpr::FunctionCall {
            name: "add".to_string(),
            args: vec![AstExpr::Integer(5)],
        };

        let result = solver.solve_expr(&expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_expressions() {
        let mut solver = ConstraintSolver::new();

        let exprs = vec![
            AstExpr::Integer(42),
            AstExpr::BinaryOp {
                left: Box::new(AstExpr::Integer(10)),
                op: AstBinaryOp::Add,
                right: Box::new(AstExpr::Integer(5)),
            },
            AstExpr::Bool(true),
        ];

        let types = solver.solve_exprs(&exprs)
            .expect("Failed to solve multiple expressions");
        assert_eq!(types.len(), 3, "Should solve 3 expressions");
        assert_eq!(types[0], Type::I32, "First expression should be I32");
        assert_eq!(types[1], Type::I32, "Second expression should be I32");
        assert_eq!(types[2], Type::Bool, "Third expression should be Bool");
    }

    #[test]
    fn test_type_solution() {
        let mut solver = ConstraintSolver::new();

        solver.register_variable("x".to_string(), Type::I32)
            .expect("Failed to register variable x");
        solver.register_variable("y".to_string(), Type::F64)
            .expect("Failed to register variable y");

        // Solve (generates constraints from variables)
        solver.register_function(
            "f".to_string(),
            vec![Type::I32],
            Type::Bool,
        ).expect("Failed to register function f");

        let solution = solver.get_solution()
            .expect("Failed to get type solution");

        // Check bindings
        assert_eq!(solution.lookup("x"), Some(&Type::I32));
        assert_eq!(solution.lookup("y"), Some(&Type::F64));
    }

    #[test]
    fn test_comparison_expression() {
        let mut solver = ConstraintSolver::new();

        // 5 < 10
        let expr = AstExpr::BinaryOp {
            left: Box::new(AstExpr::Integer(5)),
            op: AstBinaryOp::Lt,
            right: Box::new(AstExpr::Integer(10)),
        };

        let ty = solver.solve_expr(&expr).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_reference_expression() {
        let mut solver = ConstraintSolver::new();

        // &42
        let expr = AstExpr::UnaryOp {
            op: AstUnaryOp::Reference,
            operand: Box::new(AstExpr::Integer(42)),
        };

        let ty = solver.solve_expr(&expr).unwrap();
        assert!(matches!(ty, Type::Reference { .. }));
    }

    #[test]
    fn test_nested_operations() {
        let mut solver = ConstraintSolver::new();

        // 5 + 3 < 10
        let expr = AstExpr::BinaryOp {
            left: Box::new(AstExpr::BinaryOp {
                left: Box::new(AstExpr::Integer(5)),
                op: AstBinaryOp::Add,
                right: Box::new(AstExpr::Integer(3)),
            }),
            op: AstBinaryOp::Lt,
            right: Box::new(AstExpr::Integer(10)),
        };

        let ty = solver.solve_expr(&expr).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_array_typing() {
        let mut solver = ConstraintSolver::new();

        // [1, 2, 3]
        let expr = AstExpr::Array(vec![
            AstExpr::Integer(1),
            AstExpr::Integer(2),
            AstExpr::Integer(3),
        ]);

        let ty = solver.solve_expr(&expr).unwrap();
        assert!(matches!(ty, Type::Array { .. }));
    }

    #[test]
    fn test_tuple_typing() {
        let mut solver = ConstraintSolver::new();

        // (1, true, "hello")
        let expr = AstExpr::Tuple(vec![
            AstExpr::Integer(1),
            AstExpr::Bool(true),
            AstExpr::String("hello".to_string()),
        ]);

        let ty = solver.solve_expr(&expr).unwrap();
        assert!(matches!(ty, Type::Tuple(_)));
    }

    #[test]
    fn test_multi_expr_checker_block() {
        let mut checker = MultiExprTypeChecker::new();

        let exprs = vec![
            AstExpr::Integer(42),
            AstExpr::Float(3.14),
        ];

        let solution = checker.check_block(&exprs).unwrap();
        assert_eq!(solution.bindings.len(), 0); // No named bindings yet
    }

    #[test]
    fn test_multi_expr_checker_let_bindings() {
        let mut checker = MultiExprTypeChecker::new();

        let bindings = vec![
            ("x".to_string(), AstExpr::Integer(42)),
            ("y".to_string(), AstExpr::Bool(true)),
        ];

        let solution = checker.check_let_bindings(&bindings, None).unwrap();

        assert_eq!(solution.lookup("x"), Some(&Type::I32));
        assert_eq!(solution.lookup("y"), Some(&Type::Bool));
    }

    #[test]
    fn test_constraint_validation() {
        let mut solver = ConstraintSolver::new();

        solver.register_variable("x".to_string(), Type::I32).unwrap();
        solver.register_function(
            "f".to_string(),
            vec![Type::I32],
            Type::Bool,
        ).unwrap();

        let result = solver.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_logical_operations() {
        let mut solver = ConstraintSolver::new();

        // true && false
        let expr = AstExpr::BinaryOp {
            left: Box::new(AstExpr::Bool(true)),
            op: AstBinaryOp::And,
            right: Box::new(AstExpr::Bool(false)),
        };

        let ty = solver.solve_expr(&expr).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_bitwise_operations() {
        let mut solver = ConstraintSolver::new();

        // 5 & 3
        let expr = AstExpr::BinaryOp {
            left: Box::new(AstExpr::Integer(5)),
            op: AstBinaryOp::BitwiseAnd,
            right: Box::new(AstExpr::Integer(3)),
        };

        let ty = solver.solve_expr(&expr).unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_shift_operations() {
        let mut solver = ConstraintSolver::new();

        // 5 << 2
        let expr = AstExpr::BinaryOp {
            left: Box::new(AstExpr::Integer(5)),
            op: AstBinaryOp::LeftShift,
            right: Box::new(AstExpr::Integer(2)),
        };

        let ty = solver.solve_expr(&expr).unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_arithmetic_operations() {
        let tests = vec![
            (AstBinaryOp::Add, "addition"),
            (AstBinaryOp::Sub, "subtraction"),
            (AstBinaryOp::Mul, "multiplication"),
            (AstBinaryOp::Div, "division"),
            (AstBinaryOp::Mod, "modulo"),
        ];

        for (op, _name) in tests {
            let mut solver = ConstraintSolver::new();
            let expr = AstExpr::BinaryOp {
                left: Box::new(AstExpr::Integer(10)),
                op,
                right: Box::new(AstExpr::Integer(3)),
            };

            let ty = solver.solve_expr(&expr).unwrap();
            assert_eq!(ty, Type::I32);
        }
    }
}