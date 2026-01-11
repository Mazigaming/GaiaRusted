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
pub mod advanced_types;
pub mod trait_resolution;
pub mod trait_defaults;
pub mod nested_structs;
pub mod associated_types;
pub mod impl_blocks;
pub mod associated_constants;
pub mod const_eval;
pub mod lifetime_elision;
pub mod where_clauses;
pub mod hrtb_system;
pub mod pattern_matching;
pub mod enum_support;
pub mod error_diagnostics;
pub mod constraint_validation;
pub mod guard_checker;

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
pub use advanced_types::{
    HigherRankedType, AssociatedType, TypeBound, TypePredicate, TraitDefinition,
    TraitMethod, TypeConstraintSet, TypeConstraintChecker,
};
pub use trait_resolution::{
    TraitImpl, TraitResolver, TraitObject,
};
pub use trait_defaults::{
    DefaultMethod, TraitWithDefaults, ImplWithDefaults, TraitDefaultResolver,
};
pub use nested_structs::{
    NestedStructAnalyzer, NestedStructConfig, StructInfo, FieldInfo, FieldAccessPath,
    FieldOffset, NestedStructAnalysisReport,
};
pub use associated_types::{
    AssociatedTypeAnalyzer, AssociatedTypeConfig, AssociatedTypeDefinition,
    AssociatedTypeAssignment, TraitAssociatedTypes, ImplAssociatedTypes,
    ResolvedAssociatedType, AssociatedTypeAnalysisReport,
};
pub use impl_blocks::{
    ImplBlockAnalyzer, ImplBlockConfig, MethodInfo, GenericParamInfo,
    ImplBlockInfo, MethodDispatchInfo, ImplBlockAnalysisReport,
};
pub use associated_constants::{
    AssociatedConstAnalyzer, AssociatedConstConfig, AssociatedConstDefinition,
    ConstAssignment, TypeAliasDefinition, ResolvedTypeAlias, ImplConstInfo,
    AssociatedConstAnalysisReport,
};
pub use const_eval::{
    ConstEvaluator, ConstEvalConfig, ConstValue, ConstFunction, ConstFoldResult,
    ConstEvalAnalysisReport,
};
pub use lifetime_elision::{
    LifetimeElisionAnalyzer, LifetimeElisionConfig, LifetimePosition, FunctionSignature,
    LifetimeElisionReport, LifetimeElisionAnalysisReport,
};
pub use where_clauses::{
    WhereClauseAnalyzer, WhereClauseConfig, TraitBound, AssociatedTypeConstraint,
    LifetimeConstraint, WhereClause, WhereClauseReport, WhereClauseAnalysisReport,
};
pub use hrtb_system::{
    HRTBAnalyzer, HRTBConfig, BoundVariable, HigherRankedBound, Variance,
    HRTBValidationReport, HRTBAnalysisReport,
};
pub use pattern_matching::{
    PatternMatchingAnalyzer, PatternMatchingConfig, Pattern, PatternKind, MatchArm,
    ExhaustivenessReport, UnreachableReport, GuardReport, PatternMatchingAnalysisReport,
};
pub use enum_support::{
    EnumSupportAnalyzer, EnumSupportConfig, VariantKind, EnumVariant, EnumDefinition,
    EnumValidationReport, EnumSupportAnalysisReport,
};
pub use error_diagnostics::{
    DiagnosticsEngine, DiagnosticConfig, Diagnostic, Severity, ErrorCode, SourceLocation,
};
pub use constraint_validation::{
    ConstraintValidator, ConstraintValidationConfig, TypeConstraint, ConstraintType,
    ConstraintValidationReport,
};

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

    // ========================================
    // Phase 3: Advanced Type Features Tests
    // ========================================

    // Lifetime Elision Tests
    #[test]
    fn test_lifetime_elision_rule2_single_input() {
        use super::lifetime_elision::{LifetimeElisionAnalyzer, LifetimeElisionConfig};

        let config = LifetimeElisionConfig::default();
        let mut analyzer = LifetimeElisionAnalyzer::new(config);

        // fn foo(&T) -> &U where single input lifetime applies to output
        analyzer.register_function("foo", vec!["&str"], "&str").ok();
        let report = analyzer.infer_lifetimes("foo").unwrap();

        assert_eq!(report.inferred_count, 1);
        assert!(!report.ambiguous);
    }

    #[test]
    fn test_lifetime_elision_rule1_multiple_inputs() {
        use super::lifetime_elision::{LifetimeElisionAnalyzer, LifetimeElisionConfig};

        let config = LifetimeElisionConfig::default();
        let mut analyzer = LifetimeElisionAnalyzer::new(config);

        // fn foo(&T, &U) -> &V where each input gets distinct lifetime
        // Multiple input lifetimes with output reference = ambiguous
        analyzer.register_function("foo", vec!["&str", "&str"], "&str").ok();
        let report = analyzer.infer_lifetimes("foo").unwrap();

        assert_eq!(report.input_count, 2);
        assert!(report.ambiguous); // Multiple input lifetimes with output reference = ambiguous
    }

    #[test]
    fn test_lifetime_elision_self_parameter() {
        use super::lifetime_elision::{LifetimeElisionAnalyzer, LifetimeElisionConfig};

        let config = LifetimeElisionConfig::default();
        let mut analyzer = LifetimeElisionAnalyzer::new(config);

        // fn foo(&self) -> &T where self's lifetime applies to output
        analyzer.register_function("foo", vec!["&self"], "&str").ok();
        let report = analyzer.infer_lifetimes("foo").unwrap();

        assert_eq!(report.inferred_count, 1);
    }

    #[test]
    fn test_lifetime_elision_disabled_rules() {
        use super::lifetime_elision::{LifetimeElisionAnalyzer, LifetimeElisionConfig};

        let config = LifetimeElisionConfig {
            enable_rule1: false,
            enable_rule2: false,
            enable_rule3: false,
            ..Default::default()
        };
        let mut analyzer = LifetimeElisionAnalyzer::new(config);

        analyzer.register_function("foo", vec!["&str"], "&str").ok();
        let report = analyzer.infer_lifetimes("foo").unwrap();

        assert_eq!(report.inferred_count, 0);
    }

    // Where Clause Tests
    #[test]
    fn test_where_clause_basic_trait_bound() {
        use super::where_clauses::{WhereClauseAnalyzer, WhereClauseConfig};

        let config = WhereClauseConfig::default();
        let mut analyzer = WhereClauseAnalyzer::new(config);

        analyzer.register_constraint("T", "Clone").ok();
        analyzer.register_constraint("T", "Debug").ok();

        let bounds = analyzer.get_trait_bounds("T");
        assert_eq!(bounds, Some(vec!["Clone".to_string(), "Debug".to_string()]));
    }

    #[test]
    fn test_where_clause_associated_type_constraints() {
        use super::where_clauses::{WhereClauseAnalyzer, WhereClauseConfig};

        let config = WhereClauseConfig::default();
        let mut analyzer = WhereClauseAnalyzer::new(config);

        analyzer.register_associated_type("T", "Item", "String").ok();
        analyzer.register_associated_type("T", "Output", "i32").ok();

        let constraints = analyzer.get_assoc_constraints("T");
        assert!(constraints.is_some());
        assert_eq!(constraints.unwrap().len(), 2);
    }

    #[test]
    fn test_where_clause_lifetime_constraints() {
        use super::where_clauses::{WhereClauseAnalyzer, WhereClauseConfig};

        let config = WhereClauseConfig::default();
        let mut analyzer = WhereClauseAnalyzer::new(config);

        analyzer.register_lifetime_constraint("'a", None, Some("'b")).ok();
        let report = analyzer.validate_constraints().unwrap();

        assert_eq!(report.lifetime_constraint_count, 1);
    }

    #[test]
    fn test_where_clause_validation() {
        use super::where_clauses::{WhereClauseAnalyzer, WhereClauseConfig};

        let config = WhereClauseConfig::default();
        let mut analyzer = WhereClauseAnalyzer::new(config);

        analyzer.register_constraint("T", "Clone").ok();
        analyzer.register_constraint("U", "Debug").ok();
        analyzer.register_associated_type("T", "Item", "String").ok();

        let report = analyzer.validate_constraints().unwrap();
        assert_eq!(report.constraint_count, 2);
        assert_eq!(report.trait_bound_count, 2);
        assert_eq!(report.assoc_constraint_count, 1);
    }

    #[test]
    fn test_where_clause_disabled_features() {
        use super::where_clauses::{WhereClauseAnalyzer, WhereClauseConfig};

        let config = WhereClauseConfig {
            enable_associated_types: false,
            enable_lifetime_constraints: false,
            ..Default::default()
        };
        let mut analyzer = WhereClauseAnalyzer::new(config);

        // Associated types should fail
        let assoc_result = analyzer.register_associated_type("T", "Item", "String");
        assert!(assoc_result.is_err());

        // Lifetime constraints should fail
        let lifetime_result = analyzer.register_lifetime_constraint("'a", None, Some("'b"));
        assert!(lifetime_result.is_err());

        // Trait bounds should still work
        let bound_result = analyzer.register_constraint("T", "Clone");
        assert!(bound_result.is_ok());
    }

    // HRTB Tests
    #[test]
    fn test_hrtb_basic_registration() {
        use super::hrtb_system::{HRTBAnalyzer, HRTBConfig};

        let config = HRTBConfig::default();
        let mut analyzer = HRTBAnalyzer::new(config);

        let result = analyzer.register_hrtb("fn_ptr", vec!["'a"], "Fn(&'a T)");
        assert!(result.is_ok());
        assert!(analyzer.has_hrtb("fn_ptr"));
    }

    #[test]
    fn test_hrtb_multiple_bound_variables() {
        use super::hrtb_system::{HRTBAnalyzer, HRTBConfig};

        let config = HRTBConfig::default();
        let mut analyzer = HRTBAnalyzer::new(config);

        analyzer.register_hrtb("complex", vec!["'a", "'b", "'c"], "Fn(&'a T, &'b U) -> &'c V").ok();

        let vars = analyzer.get_bound_variables("complex");
        assert_eq!(vars, Some(vec!["'a".to_string(), "'b".to_string(), "'c".to_string()]));
    }

    #[test]
    fn test_hrtb_variance_tracking() {
        use super::hrtb_system::{HRTBAnalyzer, HRTBConfig, Variance};

        let config = HRTBConfig::default();
        let mut analyzer = HRTBAnalyzer::new(config);

        analyzer.register_hrtb("generic", vec!["'a"], "Fn(&'a T)").ok();
        analyzer.register_variance("generic", "T", Variance::Covariant).ok();
        analyzer.register_variance("generic", "U", Variance::Contravariant).ok();

        assert_eq!(analyzer.get_variance("generic", "T"), Some(Variance::Covariant));
        assert_eq!(analyzer.get_variance("generic", "U"), Some(Variance::Contravariant));
    }

    #[test]
    fn test_hrtb_function_pointer_detection() {
        use super::hrtb_system::{HRTBAnalyzer, HRTBConfig};

        let config = HRTBConfig::default();
        let mut analyzer = HRTBAnalyzer::new(config);

        // Fn variant
        analyzer.register_hrtb("fn_trait", vec!["'a"], "Fn(&'a T)").ok();
        // FnMut variant
        analyzer.register_hrtb("fnmut_trait", vec!["'a"], "FnMut(&'a T)").ok();
        // FnOnce variant
        analyzer.register_hrtb("fnonce_trait", vec!["'a"], "FnOnce(&'a T)").ok();

        assert!(analyzer.is_function_pointer("fn_trait"));
        assert!(analyzer.is_function_pointer("fnmut_trait"));
        assert!(analyzer.is_function_pointer("fnonce_trait"));
    }

    #[test]
    fn test_hrtb_validation() {
        use super::hrtb_system::{HRTBAnalyzer, HRTBConfig};

        let config = HRTBConfig::default();
        let mut analyzer = HRTBAnalyzer::new(config);

        analyzer.register_hrtb("valid_hrtb", vec!["'a", "'b"], "Fn(&'a T, &'b U)").ok();
        let report = analyzer.validate_hrtb("valid_hrtb").unwrap();

        assert!(report.is_valid);
        assert_eq!(report.bound_variable_count, 2);
    }

    #[test]
    fn test_hrtb_max_bound_variables() {
        use super::hrtb_system::{HRTBAnalyzer, HRTBConfig};

        let config = HRTBConfig {
            max_bound_variables: 2,
            ..Default::default()
        };
        let mut analyzer = HRTBAnalyzer::new(config);

        // Should succeed with 2 variables
        let result = analyzer.register_hrtb("limited", vec!["'a", "'b"], "Fn(&'a T)");
        assert!(result.is_ok());

        // Should fail with 3 variables
        let result = analyzer.register_hrtb("too_many", vec!["'a", "'b", "'c"], "Fn(&'a T)");
        assert!(result.is_err());
    }

    // Integration between Phase 3 modules
    #[test]
    fn test_lifetime_elision_with_where_clauses() {
        use super::lifetime_elision::{LifetimeElisionAnalyzer, LifetimeElisionConfig};
        use super::where_clauses::{WhereClauseAnalyzer, WhereClauseConfig};

        let le_config = LifetimeElisionConfig::default();
        let mut le_analyzer = LifetimeElisionAnalyzer::new(le_config);
        le_analyzer.register_function("foo", vec!["&str"], "&str").ok();

        let wc_config = WhereClauseConfig::default();
        let mut wc_analyzer = WhereClauseAnalyzer::new(wc_config);
        wc_analyzer.register_constraint("T", "Clone").ok();

        // Both should work together
        let le_report = le_analyzer.infer_lifetimes("foo").ok();
        let wc_report = wc_analyzer.validate_constraints().ok();

        assert!(le_report.is_some());
        assert!(wc_report.is_some());
    }

    #[test]
    fn test_hrtb_with_where_clauses() {
        use super::hrtb_system::{HRTBAnalyzer, HRTBConfig};
        use super::where_clauses::{WhereClauseAnalyzer, WhereClauseConfig};

        let hrtb_config = HRTBConfig::default();
        let mut hrtb_analyzer = HRTBAnalyzer::new(hrtb_config);
        hrtb_analyzer.register_hrtb("complex_fn", vec!["'a"], "Fn(&'a T) -> &'a U").ok();

        let wc_config = WhereClauseConfig::default();
        let mut wc_analyzer = WhereClauseAnalyzer::new(wc_config);
        wc_analyzer.register_constraint("T", "Clone").ok();
        wc_analyzer.register_constraint("U", "Debug").ok();

        // Both should coexist
        let hrtb_report = hrtb_analyzer.generate_report();
        let wc_report = wc_analyzer.generate_report();

        assert_eq!(hrtb_report.hrtb_count, 1);
        assert_eq!(wc_report.total_constraints, 2);
    }

    #[test]
    fn test_phase3_complete_integration() {
        use super::lifetime_elision::{LifetimeElisionAnalyzer, LifetimeElisionConfig};
        use super::where_clauses::{WhereClauseAnalyzer, WhereClauseConfig};
        use super::hrtb_system::{HRTBAnalyzer, HRTBConfig, Variance};

        // Set up all three Phase 3 analyzers
        let mut le_analyzer = LifetimeElisionAnalyzer::new(LifetimeElisionConfig::default());
        let mut wc_analyzer = WhereClauseAnalyzer::new(WhereClauseConfig::default());
        let mut hrtb_analyzer = HRTBAnalyzer::new(HRTBConfig::default());

        // Lifetime elision for a complex function
        le_analyzer.register_function(
            "process",
            vec!["&str", "&str"],
            "&str"
        ).ok();

        // Where clauses on the types
        wc_analyzer.register_constraint("T", "Clone").ok();
        wc_analyzer.register_constraint("T", "Debug").ok();
        wc_analyzer.register_associated_type("T", "Output", "String").ok();

        // HRTB for function pointers
        hrtb_analyzer.register_hrtb(
            "callback",
            vec!["'a"],
            "Fn(&'a T) -> &'a U"
        ).ok();
        hrtb_analyzer.register_variance("callback", "T", Variance::Covariant).ok();

        // All should complete without errors
        let le_report = le_analyzer.infer_lifetimes("process").ok();
        let wc_report = wc_analyzer.validate_constraints().ok();
        let hrtb_report = hrtb_analyzer.validate_hrtb("callback").ok();

        assert!(le_report.is_some());
        assert!(wc_report.is_some());
        assert!(hrtb_report.is_some());
        assert!(hrtb_report.unwrap().is_valid);
    }

    // ========================================
    // Phase 4: Pattern Matching & Enum Tests
    // ========================================

    #[test]
    fn test_pattern_matching_basic_literal() {
        use super::pattern_matching::{PatternMatchingAnalyzer, PatternMatchingConfig};

        let config = PatternMatchingConfig::default();
        let mut analyzer = PatternMatchingAnalyzer::new(config);

        analyzer.register_match("value_match").ok();
        analyzer.register_pattern("value_match", "0", None, "zero").ok();
        analyzer.register_pattern("value_match", "1", None, "one").ok();
        analyzer.register_pattern("value_match", "_", None, "other").ok();

        let report = analyzer.check_exhaustiveness("value_match").unwrap();
        assert!(report.is_exhaustive);
    }

    #[test]
    fn test_pattern_matching_with_guards() {
        use super::pattern_matching::{PatternMatchingAnalyzer, PatternMatchingConfig};

        let config = PatternMatchingConfig::default();
        let mut analyzer = PatternMatchingAnalyzer::new(config);

        analyzer.register_match("guarded_match").ok();
        analyzer.register_pattern("guarded_match", "x", Some("x > 0"), "positive").ok();
        analyzer.register_pattern("guarded_match", "x", Some("x < 0"), "negative").ok();
        analyzer.register_pattern("guarded_match", "_", None, "zero").ok();

        let report = analyzer.validate_guards("guarded_match").unwrap();
        assert_eq!(report.guard_count, 2);
    }

    #[test]
    fn test_pattern_matching_unreachable_detection() {
        use super::pattern_matching::{PatternMatchingAnalyzer, PatternMatchingConfig};

        let config = PatternMatchingConfig::default();
        let mut analyzer = PatternMatchingAnalyzer::new(config);

        analyzer.register_match("reach_test").ok();
        analyzer.register_pattern("reach_test", "_", None, "catch_all").ok();
        analyzer.register_pattern("reach_test", "0", None, "zero").ok(); // Unreachable

        let report = analyzer.check_unreachable("reach_test").unwrap();
        assert!(!report.unreachable_patterns.is_empty());
    }

    #[test]
    fn test_enum_support_unit_variant() {
        use super::enum_support::{EnumSupportAnalyzer, EnumSupportConfig};
        use super::enum_support::VariantKind;

        let config = EnumSupportConfig::default();
        let mut analyzer = EnumSupportAnalyzer::new(config);

        analyzer.register_enum("Option").ok();
        analyzer.register_variant("Option", "None", VariantKind::Unit).ok();
        analyzer.register_variant("Option", "Some", VariantKind::Tuple(vec!["T".to_string()])).ok();

        let report = analyzer.validate_enum("Option").unwrap();
        assert_eq!(report.variant_count, 2);
    }

    #[test]
    fn test_enum_support_struct_variant() {
        use super::enum_support::{EnumSupportAnalyzer, EnumSupportConfig};
        use super::enum_support::VariantKind;

        let config = EnumSupportConfig::default();
        let mut analyzer = EnumSupportAnalyzer::new(config);

        analyzer.register_enum("Event").ok();
        let button_fields = vec![("x".to_string(), "i32".to_string()), ("y".to_string(), "i32".to_string())];
        analyzer.register_variant("Event", "Button", VariantKind::Struct(button_fields)).ok();

        let report = analyzer.validate_enum("Event").unwrap();
        assert!(report.is_valid);
    }

    #[test]
    fn test_enum_with_generics() {
        use super::enum_support::{EnumSupportAnalyzer, EnumSupportConfig};
        use super::enum_support::VariantKind;

        let config = EnumSupportConfig::default();
        let mut analyzer = EnumSupportAnalyzer::new(config);

        analyzer.register_enum("Result").ok();
        analyzer.add_generic("Result", "T").ok();
        analyzer.add_generic("Result", "E").ok();
        analyzer.register_variant("Result", "Ok", VariantKind::Tuple(vec!["T".to_string()])).ok();
        analyzer.register_variant("Result", "Err", VariantKind::Tuple(vec!["E".to_string()])).ok();

        let report = analyzer.validate_enum("Result").unwrap();
        assert_eq!(report.variant_count, 2);
        assert_eq!(report.generic_count, 2);
    }

    #[test]
    fn test_enum_with_where_clauses() {
        use super::enum_support::{EnumSupportAnalyzer, EnumSupportConfig};
        use super::enum_support::VariantKind;

        let config = EnumSupportConfig::default();
        let mut analyzer = EnumSupportAnalyzer::new(config);

        analyzer.register_enum("Container").ok();
        analyzer.add_generic("Container", "T").ok();
        analyzer.add_where_clause("Container", "T: Clone").ok();
        analyzer.add_where_clause("Container", "T: Debug").ok();
        // Add a variant so the enum isn't empty
        analyzer.register_variant("Container", "Some", VariantKind::Tuple(vec!["T".to_string()])).ok();

        let report = analyzer.validate_enum("Container").unwrap();
        assert_eq!(report.variant_count, 1);
    }

    #[test]
    fn test_pattern_matching_with_enum() {
        use super::pattern_matching::{PatternMatchingAnalyzer, PatternMatchingConfig};
        use super::enum_support::{EnumSupportAnalyzer, EnumSupportConfig};
        use super::enum_support::VariantKind;

        // Set up enum
        let enum_config = EnumSupportConfig::default();
        let mut enum_analyzer = EnumSupportAnalyzer::new(enum_config);
        enum_analyzer.register_enum("Status").ok();
        enum_analyzer.register_variant("Status", "Active", VariantKind::Unit).ok();
        enum_analyzer.register_variant("Status", "Inactive", VariantKind::Unit).ok();
        let enum_report = enum_analyzer.validate_enum("Status").ok();
        assert!(enum_report.is_some());

        // Set up pattern matching on enum
        let pm_config = PatternMatchingConfig::default();
        let mut pm_analyzer = PatternMatchingAnalyzer::new(pm_config);
        pm_analyzer.register_match("status_check").ok();
        pm_analyzer.register_pattern("status_check", "Active", None, "is_active").ok();
        pm_analyzer.register_pattern("status_check", "Inactive", None, "is_inactive").ok();

        let report = pm_analyzer.check_exhaustiveness("status_check").unwrap();
        assert!(report.is_exhaustive);
    }

    #[test]
    fn test_phase3_and_phase4_integration_with_enum_where_clauses() {
        use super::enum_support::{EnumSupportAnalyzer, EnumSupportConfig, VariantKind};
        use super::where_clauses::{WhereClauseAnalyzer, WhereClauseConfig};
        use super::lifetime_elision::{LifetimeElisionAnalyzer, LifetimeElisionConfig};

        // Phase 3: Set up where clauses
        let wc_config = WhereClauseConfig::default();
        let mut wc_analyzer = WhereClauseAnalyzer::new(wc_config);
        wc_analyzer.register_constraint("T", "Clone").ok();
        wc_analyzer.register_constraint("T", "PartialEq").ok();

        // Phase 3: Set up lifetime elision
        let le_config = LifetimeElisionConfig::default();
        let mut le_analyzer = LifetimeElisionAnalyzer::new(le_config);
        le_analyzer.register_function("compare", vec!["&T", "&T"], "bool").ok();

        // Phase 4: Set up enum with generics and bounds
        let enum_config = EnumSupportConfig::default();
        let mut enum_analyzer = EnumSupportAnalyzer::new(enum_config);
        enum_analyzer.register_enum("Comparable").ok();
        enum_analyzer.add_generic("Comparable", "T").ok();
        enum_analyzer.add_where_clause("Comparable", "T: Clone").ok();
        enum_analyzer.add_where_clause("Comparable", "T: PartialEq").ok();

        let wc_report = wc_analyzer.validate_constraints().ok();
        let le_report = le_analyzer.infer_lifetimes("compare").ok();
        let enum_report = enum_analyzer.validate_enum("Comparable").ok();

        assert!(wc_report.is_some());
        assert!(le_report.is_some());
        assert!(enum_report.is_some());
    }

    #[test]
    fn test_complex_pattern_matching_with_guards_and_enums() {
        use super::pattern_matching::{PatternMatchingAnalyzer, PatternMatchingConfig};
        use super::enum_support::{EnumSupportAnalyzer, EnumSupportConfig};
        use super::enum_support::VariantKind;

        // Set up enum
        let enum_config = EnumSupportConfig::default();
        let mut enum_analyzer = EnumSupportAnalyzer::new(enum_config);
        enum_analyzer.register_enum("Result").ok();
        enum_analyzer.add_generic("Result", "T").ok();
        enum_analyzer.add_generic("Result", "E").ok();
        enum_analyzer.register_variant("Result", "Ok", VariantKind::Tuple(vec!["T".to_string()])).ok();
        enum_analyzer.register_variant("Result", "Err", VariantKind::Tuple(vec!["E".to_string()])).ok();
        let enum_report = enum_analyzer.validate_enum("Result").unwrap();
        assert_eq!(enum_report.variant_count, 2);

        // Set up pattern matching
        let pm_config = PatternMatchingConfig::default();
        let mut pm_analyzer = PatternMatchingAnalyzer::new(pm_config);
        pm_analyzer.register_match("result_handler").ok();
        pm_analyzer.register_pattern("result_handler", "Ok", Some("value > 0"), "positive_ok").ok();
        pm_analyzer.register_pattern("result_handler", "Ok", Some("value <= 0"), "non_positive_ok").ok();
        pm_analyzer.register_pattern("result_handler", "Err", Some("error.is_critical()"), "critical_error").ok();
        pm_analyzer.register_pattern("result_handler", "Err", None, "other_error").ok();

        let guard_report = pm_analyzer.validate_guards("result_handler").unwrap();
        assert_eq!(guard_report.guard_count, 3);
    }

    #[test]
    fn test_pattern_matching_config_limits() {
        use super::pattern_matching::{PatternMatchingAnalyzer, PatternMatchingConfig};

        let config = PatternMatchingConfig {
            max_patterns_per_match: 3,
            ..Default::default()
        };
        let mut analyzer = PatternMatchingAnalyzer::new(config);

        analyzer.register_match("limited").ok();
        analyzer.register_pattern("limited", "0", None, "zero").ok();
        analyzer.register_pattern("limited", "1", None, "one").ok();
        analyzer.register_pattern("limited", "2", None, "two").ok();

        // Fourth pattern should fail due to limit
        let result = analyzer.register_pattern("limited", "_", None, "other");
        assert!(result.is_err());
    }

    #[test]
    fn test_enum_variant_discriminant_assignment() {
        use super::enum_support::{EnumSupportAnalyzer, EnumSupportConfig};
        use super::enum_support::VariantKind;

        let config = EnumSupportConfig::default();
        let mut analyzer = EnumSupportAnalyzer::new(config);

        analyzer.register_enum("Color").ok();
        analyzer.register_variant("Color", "Red", VariantKind::Unit).ok();
        analyzer.register_variant("Color", "Green", VariantKind::Unit).ok();
        analyzer.register_variant("Color", "Blue", VariantKind::Unit).ok();

        let report = analyzer.validate_enum("Color").unwrap();
        assert_eq!(report.variant_count, 3);
        // Discriminants should be sequential (0, 1, 2)
        assert!(report.is_valid);
    }
}