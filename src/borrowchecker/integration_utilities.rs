//! # Phase 4 Integration Utilities
//!
//! Practical helper functions and builders for using all Phase 4 modules together
//! in real code analysis scenarios.

use crate::borrowchecker::{
    TypeSystemBridge, UnionTypeInfo, BorrowEnv, UnionTypeDetector,
    IteratorAnalyzer, NLLBindingTracker, BindingLocation,
    BorrowErrorKind,
};
use crate::lowering::{HirExpression, HirType};
use std::collections::HashMap;

/// Builder for setting up a complete Phase 4 analysis environment
#[derive(Debug)]
pub struct Phase4AnalysisBuilder {
    bridge: TypeSystemBridge,
    union_types: Vec<UnionTypeInfo>,
    iterator_infos: Vec<(String, HirType, bool)>,
    generic_bindings: HashMap<String, HirType>,
}

impl Phase4AnalysisBuilder {
    /// Create a new Phase 4 analysis builder
    pub fn new() -> Self {
        Phase4AnalysisBuilder {
            bridge: TypeSystemBridge::new(),
            union_types: Vec::new(),
            iterator_infos: Vec::new(),
            generic_bindings: HashMap::new(),
        }
    }

    /// Add a union type to the analysis
    pub fn add_union_type(mut self, name: &str, is_union: bool, variants: Vec<&str>) -> Self {
        let union_info = UnionTypeInfo {
            name: name.to_string(),
            is_union,
            variants: variants.iter().map(|v| v.to_string()).collect(),
        };
        self.union_types.push(union_info);
        self
    }

    /// Add iterator information for a collection type
    pub fn add_iterator_type(
        mut self,
        collection: &str,
        item_type: HirType,
        is_consuming: bool,
    ) -> Self {
        self.iterator_infos.push((collection.to_string(), item_type, is_consuming));
        self
    }

    /// Add a generic type parameter binding
    pub fn bind_generic(mut self, param: &str, concrete_type: HirType) -> Self {
        self.generic_bindings.insert(param.to_string(), concrete_type);
        self
    }

    /// Build the complete analysis environment
    pub fn build(mut self) -> BorrowEnv {
        // Register all union types
        for union_info in self.union_types {
            self.bridge.register_union_type(union_info);
        }

        // Register all iterator information
        for (collection, item_type, is_consuming) in self.iterator_infos {
            self.bridge.register_iterator_info(&collection, item_type, is_consuming);
        }

        // Bind all generics
        for (param, concrete_type) in self.generic_bindings {
            self.bridge.bind_generic(&param, concrete_type);
        }

        // Create and return environment with bridge
        BorrowEnv::with_type_bridge(self.bridge)
    }

    /// Build with standard library types pre-configured
    pub fn with_stdlib_types(self) -> Self {
        self
            .add_iterator_type("Vec", HirType::Unknown, true)
            .add_iterator_type("String", HirType::Char, true)
            .add_iterator_type("HashMap", HirType::Unknown, true)
            .add_iterator_type("HashSet", HirType::Unknown, true)
    }
}

/// Analysis result with all safety information
#[derive(Debug, Clone)]
pub struct SafetyAnalysisResult {
    /// Union types detected in expression
    pub unions_detected: Vec<String>,
    /// Iterator consumption detected
    pub is_consuming: bool,
    /// Inferred type for loop variables
    pub loop_var_type: Option<HirType>,
    /// Any safety violations found
    pub violations: Vec<String>,
}

impl SafetyAnalysisResult {
    /// Create a new empty result
    pub fn new() -> Self {
        SafetyAnalysisResult {
            unions_detected: Vec::new(),
            is_consuming: false,
            loop_var_type: None,
            violations: Vec::new(),
        }
    }

    /// Check if analysis found any violations
    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    /// Get all violations as a formatted string
    pub fn violations_summary(&self) -> String {
        if self.violations.is_empty() {
            "No violations detected".to_string()
        } else {
            format!(
                "Found {} violation(s):\n{}",
                self.violations.len(),
                self.violations
                    .iter()
                    .enumerate()
                    .map(|(i, v)| format!("  {}. {}", i + 1, v))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    }
}

/// High-level analyzer combining all Phase 4 modules
pub struct Phase4Analyzer {
    env: BorrowEnv,
    detector: UnionTypeDetector,
    iterator_analyzer: IteratorAnalyzer,
    binding_tracker: NLLBindingTracker,
}

impl Phase4Analyzer {
    /// Create a new analyzer from a Phase 4 environment
    pub fn new(env: BorrowEnv) -> Self {
        let detector = UnionTypeDetector::new();
        let mut iterator_analyzer = IteratorAnalyzer::new();

        // Register standard types in detectors
        iterator_analyzer.register_standard_types();

        Phase4Analyzer {
            env,
            detector,
            iterator_analyzer,
            binding_tracker: NLLBindingTracker::new(),
        }
    }

    /// Analyze an expression for safety violations
    pub fn analyze_expression(&mut self, expr: &HirExpression) -> SafetyAnalysisResult {
        let mut result = SafetyAnalysisResult::new();

        // Check for union types
        if let Some(union_type) = self.detector.detect_union_type(expr, None) {
            result.unions_detected.push(union_type);
            result
                .violations
                .push("Union field access requires unsafe block".to_string());
        }

        // Check iterator patterns
        if let Some(info) = self.iterator_analyzer.analyze_iterator(expr) {
            result.loop_var_type = Some(info.item_type);
            result.is_consuming = info.is_into_iterator;

            if result.is_consuming {
                result.violations.push(
                    "Collection consumption: collection moves after iteration".to_string(),
                );
            }
        }

        result
    }

    /// Analyze a for-loop for safety
    pub fn analyze_for_loop(
        &mut self,
        var: &str,
        iter: &HirExpression,
    ) -> SafetyAnalysisResult {
        let mut result = SafetyAnalysisResult::new();

        // Infer loop variable type using bridge
        let var_type = if let Some(bridge) = self.env.type_bridge.as_ref() {
            bridge
                .infer_iterator_item_type(iter)
                .unwrap_or(HirType::Unknown)
        } else {
            HirType::Unknown
        };

        result.loop_var_type = Some(var_type.clone());

        // Register binding
        let loc = BindingLocation::new(0, 0);
        let _ = self
            .binding_tracker
            .register_binding(var.to_string(), var_type, false, loc);

        // Check if iterator is consuming
        if let Some(bridge) = self.env.type_bridge.as_ref() {
            if bridge.is_consuming_iterator(iter) {
                result.is_consuming = true;
                result
                    .violations
                    .push("Iterator consumes collection".to_string());
            }
        }

        result
    }

    /// Register a union type for detection
    pub fn register_union(&mut self, name: &str) {
        self.detector.register_union_type(name);
    }

    /// Get the borrow environment
    pub fn env(&self) -> &BorrowEnv {
        &self.env
    }

    /// Get mutable borrow environment
    pub fn env_mut(&mut self) -> &mut BorrowEnv {
        &mut self.env
    }
}

/// Convenience function to create error from violation
pub fn violation_to_error(violation: &str) -> Option<BorrowErrorKind> {
    match violation {
        v if v.contains("Union field") => Some(BorrowErrorKind::UnionFieldAccessNotUnsafe {
            union_type: "Unknown".to_string(),
            field: "unknown".to_string(),
        }),
        v if v.contains("after move") => Some(BorrowErrorKind::ValueUsedAfterMove {
            variable: "unknown".to_string(),
        }),
        v if v.contains("consumed") => Some(BorrowErrorKind::IteratorConsumptionNotTracked {
            variable: "unknown".to_string(),
        }),
        v if v.contains("multiple") => Some(BorrowErrorKind::MultipleMutableBorrows {
            variable: "unknown".to_string(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creates_environment() {
        let env = Phase4AnalysisBuilder::new()
            .add_union_type("Data", true, vec!["A", "B"])
            .build();

        assert!(env.type_bridge.is_some());
    }

    #[test]
    fn test_builder_with_stdlib_types() {
        let env = Phase4AnalysisBuilder::new()
            .with_stdlib_types()
            .add_iterator_type("Vec", HirType::Int32, false)
            .build();

        assert!(env.type_bridge.is_some());
    }

    #[test]
    fn test_analysis_result_no_violations() {
        let result = SafetyAnalysisResult::new();
        assert!(!result.has_violations());
        assert_eq!(result.violations_summary(), "No violations detected");
    }

    #[test]
    fn test_analysis_result_with_violations() {
        let mut result = SafetyAnalysisResult::new();
        result.violations.push("Test violation".to_string());

        assert!(result.has_violations());
        assert!(result.violations_summary().contains("1 violation"));
    }

    #[test]
    fn test_phase4_analyzer_creation() {
        let env = Phase4AnalysisBuilder::new().build();
        let analyzer = Phase4Analyzer::new(env);

        assert!(analyzer.env().type_bridge.is_some());
    }

    #[test]
    fn test_phase4_analyzer_register_union() {
        let env = Phase4AnalysisBuilder::new().build();
        let mut analyzer = Phase4Analyzer::new(env);

        analyzer.register_union("Data");
        // Analyzer now has union registered
    }

    #[test]
    fn test_violation_to_error_union() {
        let error = violation_to_error("Union field access requires unsafe");
        assert!(error.is_some());

        match error.unwrap() {
            BorrowErrorKind::UnionFieldAccessNotUnsafe { .. } => {
                // Correct
            }
            _ => panic!("Wrong error kind"),
        }
    }

    #[test]
    fn test_violation_to_error_move() {
        let error = violation_to_error("Value used after move");
        assert!(error.is_some());

        match error.unwrap() {
            BorrowErrorKind::ValueUsedAfterMove { .. } => {
                // Correct
            }
            _ => panic!("Wrong error kind"),
        }
    }

    #[test]
    fn test_safety_analysis_result_unions() {
        let mut result = SafetyAnalysisResult::new();
        result.unions_detected.push("Data".to_string());

        assert_eq!(result.unions_detected.len(), 1);
    }

    #[test]
    fn test_safety_analysis_result_loop_var_type() {
        let mut result = SafetyAnalysisResult::new();
        result.loop_var_type = Some(HirType::Int32);

        assert_eq!(result.loop_var_type, Some(HirType::Int32));
    }

    #[test]
    fn test_safety_analysis_result_consuming_iterator() {
        let mut result = SafetyAnalysisResult::new();
        result.is_consuming = true;

        assert!(result.is_consuming);
    }

    #[test]
    fn test_analyzer_for_loop_analysis() {
        let env = Phase4AnalysisBuilder::new()
            .with_stdlib_types()
            .build();
        let mut analyzer = Phase4Analyzer::new(env);

        let iter_expr = HirExpression::Variable("v".to_string());
        let result = analyzer.analyze_for_loop("x", &iter_expr);

        // Analysis completed
        assert!(result.loop_var_type.is_some());
    }

    #[test]
    fn test_builder_generic_binding() {
        let env = Phase4AnalysisBuilder::new()
            .bind_generic("T", HirType::Int32)
            .build();

        assert!(env.type_bridge.is_some());
    }

    #[test]
    fn test_multiple_union_types() {
        let env = Phase4AnalysisBuilder::new()
            .add_union_type("Data1", true, vec!["A"])
            .add_union_type("Data2", true, vec!["B"])
            .build();

        assert!(env.type_bridge.is_some());
    }

    #[test]
    fn test_multiple_iterator_types() {
        let env = Phase4AnalysisBuilder::new()
            .add_iterator_type("Vec", HirType::Int32, false)
            .add_iterator_type("String", HirType::Char, true)
            .build();

        assert!(env.type_bridge.is_some());
    }
}
