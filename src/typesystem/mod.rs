//! # Type System Module
//!
//! Complete type inference and checking for the Rust compiler.
//!
//! # Components:
//! 1. **types**: Core type representation
//! 2. **substitution**: Type variable bindings
//! 3. **unification**: Robinson's unification algorithm
//!
//! # Usage Example:
//! ```ignore
//! use typesystem::{Type, TypeVar, Substitution, UnificationEngine};
//!
//! let mut engine = UnificationEngine::new();
//! let mut subst = Substitution::new();
//!
//! // Unify X with i32
//! engine.unify(
//!     &Type::Variable(TypeVar(0)),
//!     &Type::I32,
//!     &mut subst
//! ).unwrap();
//!
//! // X is now bound to i32
//! assert_eq!(subst.apply(&Type::Variable(TypeVar(0))), Type::I32);
//! ```

pub mod types;
pub mod substitution;
pub mod unification;
pub mod constraints;
pub mod expression_typing;
pub mod constraint_solver;
pub mod ast_bridge;
pub mod integrated_checker;

// Re-export main types for convenient access
pub use types::{
    Type, TypeVar, Lifetime, LifetimeName, LifetimeVar,
    StructId, EnumId, TraitId, GenericId,
    TypeVarGenerator, LifetimeVarGenerator,
};
pub use substitution::Substitution;
pub use unification::UnificationEngine;
pub use constraints::{
    Constraint, ConstraintGenerator, StructDef, FunctionDef, GenericParam,
    BinaryOp, UnaryOp, ExprTypeMap,
};
pub use expression_typing::{
    ExprTyper, AstExpr, AstBinaryOp, AstUnaryOp, TypedExpr, ExprTypingError,
};
pub use constraint_solver::{
    ConstraintSolver, ConstraintError, TypeSolution, MultiExprTypeChecker,
};
pub use ast_bridge::{
    convert_type, convert_expression, convert_binary_op, convert_unary_op,
    convert_type_with_context, extract_function_signature, extract_struct_definition,
    extract_methods_from_impl,
    BridgeError, TypeRegistry, StructTypeInfo, FunctionTypeInfo, ConversionContext,
};
pub use integrated_checker::{IntegratedTypeChecker, DetailedTypeError, TypeCheckReport};

/// Type checking result
pub type TypeCheckResult<T> = Result<T, String>;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_basic_type_unification() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        engine
            .unify(&Type::Variable(TypeVar(0)), &Type::I32, &mut subst)
            .unwrap();

        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(0))),
            Type::I32
        );
    }

    #[test]
    fn test_type_system_integration() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        // Build a function type: fn(X, bool) -> Y
        let func_type = Type::Function {
            params: vec![Type::Variable(TypeVar(0)), Type::Bool],
            ret: Box::new(Type::Variable(TypeVar(1))),
        };

        // Expected type: fn(i32, bool) -> str
        let expected = Type::Function {
            params: vec![Type::I32, Type::Bool],
            ret: Box::new(Type::Str),
        };

        engine.unify(&func_type, &expected, &mut subst).unwrap();

        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(0))),
            Type::I32
        );
        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(1))),
            Type::Str
        );
    }

    #[test]
    fn test_complex_generic_inference() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        // Type: [X, bool]
        let list_of_x = Type::Tuple(vec![
            Type::Variable(TypeVar(0)),
            Type::Bool,
        ]);

        // Expected: [i32, bool]
        let expected = Type::Tuple(vec![Type::I32, Type::Bool]);

        engine.unify(&list_of_x, &expected, &mut subst).unwrap();

        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(0))),
            Type::I32
        );
    }

    #[test]
    fn test_reference_unification() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let ref_x = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::Variable(TypeVar(0))),
        };

        let ref_i32 = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };

        engine.unify(&ref_x, &ref_i32, &mut subst).unwrap();

        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(0))),
            Type::I32
        );
    }

    #[test]
    fn test_substitution_composition() {
        let mut subst1 = Substitution::new();
        subst1.bind(TypeVar(0), Type::I32).unwrap();
        subst1.bind(TypeVar(1), Type::Variable(TypeVar(0))).unwrap();

        let mut subst2 = Substitution::new();
        subst2.bind(TypeVar(2), Type::Bool).unwrap();

        subst1.compose(&subst2);

        // After composition:
        // - X = i32 (unchanged)
        // - Y = X becomes Y = i32
        // - Z = bool (added)
        assert_eq!(subst1.apply(&Type::Variable(TypeVar(0))), Type::I32);
        assert_eq!(subst1.apply(&Type::Variable(TypeVar(1))), Type::I32);
    }

    #[test]
    fn test_occurs_check_prevents_infinite_types() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let var = TypeVar(0);
        let infinite_type = Type::Array {
            element: Box::new(Type::Variable(var)),
            size: 1,
        };

        let result = engine.unify(&Type::Variable(var), &infinite_type, &mut subst);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_constraints() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let constraints = vec![
            (Type::Variable(TypeVar(0)), Type::I32),
            (Type::Variable(TypeVar(1)), Type::Bool),
            (
                Type::Function {
                    params: vec![Type::Variable(TypeVar(0))],
                    ret: Box::new(Type::Variable(TypeVar(1))),
                },
                Type::Function {
                    params: vec![Type::I32],
                    ret: Box::new(Type::Bool),
                },
            ),
        ];

        engine.unify_constraints(&constraints, &mut subst).unwrap();

        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(0))),
            Type::I32
        );
        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(1))),
            Type::Bool
        );
    }

    #[test]
    fn test_constraint_generation_and_solving() {
        use super::constraints::{ConstraintGenerator, BinaryOp};

        let mut gen = ConstraintGenerator::new();

        // Constrain: (x + 5) where x : i32
        let result = gen.constrain_binary_op(BinaryOp::Add, Type::I32, Type::I32).unwrap();

        // Solve constraints
        let subst = gen.solve().unwrap();

        // The result should be a type variable that can be resolved
        let resolved = subst.apply(&result);
        assert!(matches!(resolved, Type::I32 | Type::Variable(_)));
    }

    #[test]
    fn test_function_constraint_generation() {
        use super::constraints::ConstraintGenerator;

        let mut gen = ConstraintGenerator::new();

        // Register function: fn double(x: i32) -> i32
        gen.register_function(
            "double".to_string(),
            vec![Type::I32],
            Type::I32,
        );

        // Call with correct argument
        let result = gen.constrain_function_call("double", vec![Type::I32]).unwrap();
        assert_eq!(result, Type::I32);

        // Solve - should succeed
        let _subst = gen.solve().unwrap();
    }

    #[test]
    fn test_reference_constraint_generation() {
        use super::constraints::{ConstraintGenerator, UnaryOp};

        let mut gen = ConstraintGenerator::new();

        // Generate constraints for: &x where x : i32
        let result = gen.constrain_unary_op(UnaryOp::Reference, Type::I32).unwrap();

        // Should be a reference type
        assert!(matches!(result, Type::Reference { .. }));
    }

    #[test]
    fn test_complex_constraint_solving() {
        use super::constraints::{ConstraintGenerator, BinaryOp};

        let mut gen = ConstraintGenerator::new();

        // Register function: fn compare(x: i32, y: i32) -> bool
        gen.register_function(
            "compare".to_string(),
            vec![Type::I32, Type::I32],
            Type::Bool,
        );

        // Constrain: let a = 5 + 3
        let sum = gen.constrain_binary_op(BinaryOp::Add, Type::I32, Type::I32).unwrap();

        // Constrain: let b = 10 + 20
        let sum2 = gen.constrain_binary_op(BinaryOp::Add, Type::I32, Type::I32).unwrap();

        // Constrain: compare(a, b)
        let _result = gen.constrain_function_call("compare", vec![sum, sum2]).unwrap();

        // Should solve successfully
        let _subst = gen.solve().unwrap();
    }
}