//! # Generic Type Parameter Substitutor (Phase 5C)
//!
//! Substitutes type variables with concrete types consistently across
//! type expressions, function signatures, and method calls.

use std::collections::HashMap;
use crate::lowering::HirType;

/// Represents a single type variable binding to a concrete type
#[derive(Debug, Clone, PartialEq)]
pub struct TypeBinding {
    pub param_name: String,
    pub concrete_type: HirType,
}

impl TypeBinding {
    /// Create a new type binding
    pub fn new(param_name: impl Into<String>, concrete_type: HirType) -> Self {
        TypeBinding {
            param_name: param_name.into(),
            concrete_type,
        }
    }
}

/// Substitution context for tracking multiple type bindings
#[derive(Debug, Clone)]
pub struct SubstitutionContext {
    // type_var_name -> concrete_type
    bindings: HashMap<String, HirType>,
    // Track depth to prevent infinite recursion
    max_depth: usize,
    // Current recursion depth
    current_depth: usize,
}

impl SubstitutionContext {
    /// Create a new substitution context
    pub fn new() -> Self {
        SubstitutionContext {
            bindings: HashMap::new(),
            max_depth: 100,
            current_depth: 0,
        }
    }

    /// Create with custom max depth
    pub fn with_max_depth(max_depth: usize) -> Self {
        SubstitutionContext {
            bindings: HashMap::new(),
            max_depth,
            current_depth: 0,
        }
    }

    /// Add a type binding
    pub fn bind(&mut self, param_name: impl Into<String>, concrete_type: HirType) {
        self.bindings.insert(param_name.into(), concrete_type);
    }

    /// Get a binding
    pub fn get_binding(&self, param_name: &str) -> Option<&HirType> {
        self.bindings.get(param_name)
    }

    /// Check if a type variable is bound
    pub fn is_bound(&self, param_name: &str) -> bool {
        self.bindings.contains_key(param_name)
    }

    /// Get all bindings
    pub fn get_all_bindings(&self) -> Vec<TypeBinding> {
        self.bindings
            .iter()
            .map(|(name, ty)| TypeBinding::new(name.clone(), ty.clone()))
            .collect()
    }

    /// Increment recursion depth (returns true if within limit)
    fn increment_depth(&mut self) -> bool {
        if self.current_depth >= self.max_depth {
            false
        } else {
            self.current_depth += 1;
            true
        }
    }

    /// Decrement recursion depth
    fn decrement_depth(&mut self) {
        if self.current_depth > 0 {
            self.current_depth -= 1;
        }
    }

    /// Get current recursion depth
    pub fn current_depth(&self) -> usize {
        self.current_depth
    }

    /// Total number of bindings
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }
}

impl Default for SubstitutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Substitutes type variables with concrete types
#[derive(Debug)]
pub struct GenericSubstitutor;

impl GenericSubstitutor {
    /// Substitute type variables in a type
    pub fn substitute(ty: &HirType, context: &mut SubstitutionContext) -> HirType {
        // Check recursion depth
        if !context.increment_depth() {
            context.decrement_depth();
            return ty.clone();
        }

        let result = match ty {
            // Named types might be type variables - check if bound
            HirType::Named(name) => {
                if let Some(concrete) = context.get_binding(name).cloned() {
                    // Recursively substitute in the bound type
                    GenericSubstitutor::substitute(&concrete, context)
                } else {
                    ty.clone()
                }
            }

            // Recursively substitute in references
            HirType::Reference(inner) => {
                let substituted = GenericSubstitutor::substitute(inner, context);
                HirType::Reference(Box::new(substituted))
            }

            // Recursively substitute in mutable references
            HirType::MutableReference(inner) => {
                let substituted = GenericSubstitutor::substitute(inner, context);
                HirType::MutableReference(Box::new(substituted))
            }

            // Recursively substitute in pointers
            HirType::Pointer(inner) => {
                let substituted = GenericSubstitutor::substitute(inner, context);
                HirType::Pointer(Box::new(substituted))
            }

            // Recursively substitute in arrays
            HirType::Array { element_type, size } => {
                let substituted = GenericSubstitutor::substitute(element_type, context);
                HirType::Array {
                    element_type: Box::new(substituted),
                    size: *size,
                }
            }

            // Recursively substitute in function types
            HirType::Function { params, return_type } => {
                let substituted_params = params
                    .iter()
                    .map(|p| GenericSubstitutor::substitute(p, context))
                    .collect();
                let substituted_return = GenericSubstitutor::substitute(return_type, context);

                HirType::Function {
                    params: substituted_params,
                    return_type: Box::new(substituted_return),
                }
            }

            // Recursively substitute in tuples
            HirType::Tuple(types) => {
                let substituted = types
                    .iter()
                    .map(|t| GenericSubstitutor::substitute(t, context))
                    .collect();
                HirType::Tuple(substituted)
            }

            // Recursively substitute in closures
            HirType::Closure { params, return_type, trait_kind } => {
                let substituted_params = params
                    .iter()
                    .map(|p| GenericSubstitutor::substitute(p, context))
                    .collect();
                let substituted_return = GenericSubstitutor::substitute(return_type, context);

                HirType::Closure {
                    params: substituted_params,
                    return_type: Box::new(substituted_return),
                    trait_kind: trait_kind.clone(),
                }
            }

            // Primitive types don't have type variables
            other => other.clone(),
        };

        context.decrement_depth();
        result
    }

    /// Substitute in a list of types (common for function parameters)
    pub fn substitute_list(types: &[HirType], context: &mut SubstitutionContext) -> Vec<HirType> {
        types
            .iter()
            .map(|t| GenericSubstitutor::substitute(t, context))
            .collect()
    }

    /// Substitute in a function signature
    pub fn substitute_function_signature(
        params: &[HirType],
        return_type: &HirType,
        context: &mut SubstitutionContext,
    ) -> (Vec<HirType>, HirType) {
        let substituted_params = GenericSubstitutor::substitute_list(params, context);
        let substituted_return = GenericSubstitutor::substitute(return_type, context);
        (substituted_params, substituted_return)
    }

    /// Check if a type needs substitution (contains any bound type variables)
    pub fn needs_substitution(ty: &HirType, context: &SubstitutionContext) -> bool {
        match ty {
            HirType::Named(name) => context.is_bound(name),
            HirType::Reference(inner) | HirType::MutableReference(inner)
            | HirType::Pointer(inner) => GenericSubstitutor::needs_substitution(inner, context),
            HirType::Array { element_type, .. } => {
                GenericSubstitutor::needs_substitution(element_type, context)
            }
            HirType::Function { params, return_type } => {
                params
                    .iter()
                    .any(|p| GenericSubstitutor::needs_substitution(p, context))
                    || GenericSubstitutor::needs_substitution(return_type, context)
            }
            HirType::Tuple(types) => {
                types
                    .iter()
                    .any(|t| GenericSubstitutor::needs_substitution(t, context))
            }
            HirType::Closure { params, return_type, .. } => {
                params
                    .iter()
                    .any(|p| GenericSubstitutor::needs_substitution(p, context))
                    || GenericSubstitutor::needs_substitution(return_type, context)
            }
            _ => false,
        }
    }

    /// Get a summary of substitutions for debugging
    pub fn get_substitution_summary(context: &SubstitutionContext) -> String {
        if context.binding_count() == 0 {
            "No substitutions".to_string()
        } else {
            let pairs: Vec<String> = context
                .get_all_bindings()
                .iter()
                .map(|b| format!("{}={:?}", b.param_name, b.concrete_type))
                .collect();
            pairs.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_creation() {
        let binding = TypeBinding::new("T", HirType::Int32);
        assert_eq!(binding.param_name, "T");
        assert_eq!(binding.concrete_type, HirType::Int32);
    }

    #[test]
    fn test_context_new() {
        let ctx = SubstitutionContext::new();
        assert_eq!(ctx.binding_count(), 0);
        assert_eq!(ctx.current_depth(), 0);
    }

    #[test]
    fn test_context_bind() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);

        assert!(ctx.is_bound("T"));
        assert_eq!(ctx.get_binding("T").unwrap(), &HirType::Int32);
    }

    #[test]
    fn test_context_multiple_bindings() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);
        ctx.bind("U", HirType::String);
        ctx.bind("V", HirType::Bool);

        assert_eq!(ctx.binding_count(), 3);
        assert!(ctx.is_bound("T"));
        assert!(ctx.is_bound("U"));
        assert!(ctx.is_bound("V"));
    }

    #[test]
    fn test_substitute_primitive() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);

        let result = GenericSubstitutor::substitute(&HirType::String, &mut ctx);
        assert_eq!(result, HirType::String);
    }

    #[test]
    fn test_substitute_type_variable() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);

        let result = GenericSubstitutor::substitute(&HirType::Named("T".to_string()), &mut ctx);
        assert_eq!(result, HirType::Int32);
    }

    #[test]
    fn test_substitute_unbound_type_variable() {
        let mut ctx = SubstitutionContext::new();
        
        let result = GenericSubstitutor::substitute(&HirType::Named("T".to_string()), &mut ctx);
        assert_eq!(result, HirType::Named("T".to_string()));
    }

    #[test]
    fn test_substitute_reference() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);

        let ref_type = HirType::Reference(Box::new(HirType::Named("T".to_string())));
        let result = GenericSubstitutor::substitute(&ref_type, &mut ctx);

        match result {
            HirType::Reference(inner) => assert_eq!(*inner, HirType::Int32),
            _ => panic!("Expected reference"),
        }
    }

    #[test]
    fn test_substitute_mut_reference() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::String);

        let ref_type = HirType::MutableReference(Box::new(HirType::Named("T".to_string())));
        let result = GenericSubstitutor::substitute(&ref_type, &mut ctx);

        match result {
            HirType::MutableReference(inner) => assert_eq!(*inner, HirType::String),
            _ => panic!("Expected mutable reference"),
        }
    }

    #[test]
    fn test_substitute_pointer() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Bool);

        let ptr_type = HirType::Pointer(Box::new(HirType::Named("T".to_string())));
        let result = GenericSubstitutor::substitute(&ptr_type, &mut ctx);

        match result {
            HirType::Pointer(inner) => assert_eq!(*inner, HirType::Bool),
            _ => panic!("Expected pointer"),
        }
    }

    #[test]
    fn test_substitute_array() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);

        let array_type = HirType::Array {
            element_type: Box::new(HirType::Named("T".to_string())),
            size: Some(10),
        };
        let result = GenericSubstitutor::substitute(&array_type, &mut ctx);

        match result {
            HirType::Array { element_type, size } => {
                assert_eq!(*element_type, HirType::Int32);
                assert_eq!(size, Some(10));
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_substitute_tuple() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);
        ctx.bind("U", HirType::String);

        let tuple = HirType::Tuple(vec![
            HirType::Named("T".to_string()),
            HirType::Named("U".to_string()),
        ]);
        let result = GenericSubstitutor::substitute(&tuple, &mut ctx);

        match result {
            HirType::Tuple(types) => {
                assert_eq!(types[0], HirType::Int32);
                assert_eq!(types[1], HirType::String);
            }
            _ => panic!("Expected tuple"),
        }
    }

    #[test]
    fn test_substitute_function_type() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);
        ctx.bind("U", HirType::String);

        let func_type = HirType::Function {
            params: vec![HirType::Named("T".to_string())],
            return_type: Box::new(HirType::Named("U".to_string())),
        };
        let result = GenericSubstitutor::substitute(&func_type, &mut ctx);

        match result {
            HirType::Function { params, return_type } => {
                assert_eq!(params[0], HirType::Int32);
                assert_eq!(*return_type, HirType::String);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_substitute_nested_references() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);

        let nested = HirType::Reference(Box::new(HirType::Reference(Box::new(
            HirType::Named("T".to_string()),
        ))));
        let result = GenericSubstitutor::substitute(&nested, &mut ctx);

        // Should be &&Int32
        match result {
            HirType::Reference(ref1) => match ref1.as_ref() {
                HirType::Reference(ref2) => assert_eq!(ref2.as_ref(), &HirType::Int32),
                _ => panic!("Expected nested reference"),
            },
            _ => panic!("Expected reference"),
        }
    }

    #[test]
    fn test_substitute_list() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);
        ctx.bind("U", HirType::String);

        let types = vec![
            HirType::Named("T".to_string()),
            HirType::Named("U".to_string()),
            HirType::Bool,
        ];
        let result = GenericSubstitutor::substitute_list(&types, &mut ctx);

        assert_eq!(result[0], HirType::Int32);
        assert_eq!(result[1], HirType::String);
        assert_eq!(result[2], HirType::Bool);
    }

    #[test]
    fn test_substitute_function_signature() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);
        ctx.bind("U", HirType::String);

        let params = vec![HirType::Named("T".to_string())];
        let return_type = HirType::Named("U".to_string());

        let (result_params, result_return) =
            GenericSubstitutor::substitute_function_signature(&params, &return_type, &mut ctx);

        assert_eq!(result_params[0], HirType::Int32);
        assert_eq!(result_return, HirType::String);
    }

    #[test]
    fn test_needs_substitution_true() {
        let ctx = SubstitutionContext::new()
            .apply_binding("T", HirType::Int32);

        assert!(GenericSubstitutor::needs_substitution(
            &HirType::Named("T".to_string()),
            &ctx
        ));
    }

    #[test]
    fn test_needs_substitution_false() {
        let ctx = SubstitutionContext::new()
            .apply_binding("T", HirType::Int32);

        assert!(!GenericSubstitutor::needs_substitution(
            &HirType::Named("U".to_string()),
            &ctx
        ));
    }

    #[test]
    fn test_needs_substitution_in_reference() {
        let ctx = SubstitutionContext::new()
            .apply_binding("T", HirType::Int32);

        let ref_type = HirType::Reference(Box::new(HirType::Named("T".to_string())));
        assert!(GenericSubstitutor::needs_substitution(&ref_type, &ctx));
    }

    #[test]
    fn test_context_with_max_depth() {
        let ctx = SubstitutionContext::with_max_depth(10);
        assert_eq!(ctx.max_depth, 10);
    }

    #[test]
    fn test_get_substitution_summary_empty() {
        let ctx = SubstitutionContext::new();
        let summary = GenericSubstitutor::get_substitution_summary(&ctx);
        assert_eq!(summary, "No substitutions");
    }

    #[test]
    fn test_get_substitution_summary_with_bindings() {
        let ctx = SubstitutionContext::new()
            .apply_binding("T", HirType::Int32)
            .apply_binding("U", HirType::String);

        let summary = GenericSubstitutor::get_substitution_summary(&ctx);
        assert!(!summary.is_empty());
        assert!(summary.contains("T") && summary.contains("U"));
    }

    #[test]
    fn test_many_bindings() {
        let mut ctx = SubstitutionContext::new();

        // Add 50 bindings
        for i in 0..50 {
            let name = format!("T{}", i);
            ctx.bind(&name, HirType::Int32);
        }

        assert_eq!(ctx.binding_count(), 50);
    }

    #[test]
    fn test_deep_substitution() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);

        // Create deeply nested structure
        let mut ty = HirType::Named("T".to_string());
        for _ in 0..10 {
            ty = HirType::Reference(Box::new(ty));
        }

        // Substitute - should handle all 10 levels
        let result = GenericSubstitutor::substitute(&ty, &mut ctx);

        // Verify it's still 10 levels deep
        let mut depth = 0;
        let mut current = &result;
        loop {
            match current {
                HirType::Reference(inner) => {
                    depth += 1;
                    current = inner;
                }
                HirType::Int32 => break,
                _ => panic!("Unexpected type"),
            }
        }
        assert_eq!(depth, 10);
    }

    #[test]
    fn test_case_sensitive_bindings() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);
        ctx.bind("t", HirType::String);

        let upper = HirType::Named("T".to_string());
        let lower = HirType::Named("t".to_string());

        let upper_result = GenericSubstitutor::substitute(&upper, &mut ctx);
        let lower_result = GenericSubstitutor::substitute(&lower, &mut ctx);

        assert_eq!(upper_result, HirType::Int32);
        assert_eq!(lower_result, HirType::String);
    }

    #[test]
    fn test_substitution_doesnt_mutate_context() {
        let mut ctx = SubstitutionContext::new();
        ctx.bind("T", HirType::Int32);

        let ty = HirType::Named("T".to_string());
        let _result1 = GenericSubstitutor::substitute(&ty, &mut ctx);
        let _result2 = GenericSubstitutor::substitute(&ty, &mut ctx);

        // Context should still be valid
        assert_eq!(ctx.binding_count(), 1);
    }
}

// Helper trait for fluent API
trait SubstitutionContextBuilder {
    fn apply_binding(self, name: &str, ty: HirType) -> Self;
}

impl SubstitutionContextBuilder for SubstitutionContext {
    fn apply_binding(mut self, name: &str, ty: HirType) -> Self {
        self.bind(name, ty);
        self
    }
}
