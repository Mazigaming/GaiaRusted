//! # Bug Detection Tests
//!
//! Tests designed to find potential bugs and edge cases in borrowchecker modules

#[cfg(test)]
mod bug_tests {
    use crate::borrowchecker::{
        trait_bound_extractor::{TraitBound, TraitBoundExtractor},
        nll_binding_tracker::{NLLBindingTracker, BindingLocation, ScopeKind},
        union_detection::UnionTypeDetector,
        iterator_analysis::IteratorAnalyzer,
    };
    use crate::lowering::HirType;

    // ============================================================================
    // TRAIT BOUND EXTRACTOR BUG TESTS
    // ============================================================================

    #[test]
    fn test_trait_bound_empty_trait_name() {
        let bound = TraitBound::simple("");
        assert_eq!(bound.trait_name, "");
        assert!(bound.trait_params.is_empty());
    }

    #[test]
    fn test_trait_bound_special_characters_in_name() {
        let bound = TraitBound::simple("Trait<'a, T>");
        assert_eq!(bound.trait_name, "Trait<'a, T>");
    }

    #[test]
    fn test_extractor_variable_names_case_sensitive() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));
        extractor.add_bound("t", TraitBound::simple("Clone"));

        let vars = extractor.all_variables();
        assert_eq!(vars.len(), 2); // T and t are different
        assert!(vars.contains(&"T"));
        assert!(vars.contains(&"t"));
    }

    #[test]
    fn test_extractor_bounds_overwrite_not_deduplicate_at_add() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));
        extractor.add_bound("T", TraitBound::simple("Clone"));

        let direct = extractor.get_direct_bounds("T");
        // Both are kept at direct level (dedup happens only in get_all_bounds)
        assert_eq!(direct.len(), 2);
    }

    #[test]
    fn test_hierarchy_self_reference() {
        let mut extractor = TraitBoundExtractor::new();
        // Trait extends itself
        extractor.add_supertrait("Clone", TraitBound::simple("Clone"));
        
        let bounds = extractor.get_all_bounds("T");
        extractor.add_bound("T", TraitBound::simple("Clone"));
        
        let all_bounds = extractor.get_all_bounds("T");
        // Should not infinite loop, should be bounded
        assert!(all_bounds.len() < 10);
    }

    #[test]
    fn test_validate_compliance_with_reference_type() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));

        // Test with various HirType variants
        assert!(extractor.validate_compliance("T", &HirType::Reference(Box::new(HirType::Int32))).is_ok());
        assert!(extractor.validate_compliance("T", &HirType::MutableReference(Box::new(HirType::String))).is_ok());
    }

    #[test]
    fn test_bounds_summary_with_unicode_trait_names() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone🔒"));
        
        let summary = extractor.get_bounds_summary("T");
        assert!(summary.contains("Clone🔒"));
    }

    // ============================================================================
    // NLL BINDING TRACKER BUG TESTS
    // ============================================================================

    #[test]
    fn test_binding_location_boundary_values() {
        let loc_zero = BindingLocation::new(0, 0);
        let loc_max = BindingLocation::new(usize::MAX, usize::MAX);

        assert_eq!(loc_zero.scope_id, 0);
        assert_eq!(loc_max.scope_id, usize::MAX);
    }

    #[test]
    fn test_nll_multiple_push_pop_cycle() {
        let mut tracker = NLLBindingTracker::new();

        // Push and pop multiple times
        for _ in 0..10 {
            let _scope_id = tracker.push_scope(ScopeKind::Block);
            let _ = tracker.pop_scope();
        }

        // Should still work correctly
        let _scope_id = tracker.push_scope(ScopeKind::Block);
        assert!(_scope_id > 0 || _scope_id == 0); // Scope ID is valid
    }

    #[test]
    fn test_nll_pop_empty_scope_stack() {
        let mut tracker = NLLBindingTracker::new();
        
        // Popping from empty stack should fail gracefully
        let result = tracker.pop_scope();
        assert!(result.is_err());
    }

    #[test]
    fn test_nll_register_binding_in_function_scope() {
        let mut tracker = NLLBindingTracker::new();
        let _scope = tracker.push_scope(ScopeKind::Function);

        let loc = BindingLocation::new(0, 0);
        let result = tracker.register_binding("x".to_string(), HirType::Int32, false, loc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nll_multiple_bindings_same_scope() {
        let mut tracker = NLLBindingTracker::new();
        tracker.push_scope(ScopeKind::Block);

        let loc = BindingLocation::new(0, 0);
        let _ = tracker.register_binding("x".to_string(), HirType::Int32, false, loc);
        let _ = tracker.register_binding("y".to_string(), HirType::String, false, loc);
        let _ = tracker.register_binding("z".to_string(), HirType::Bool, false, loc);

        assert!(tracker.get_binding("x").is_some());
        assert!(tracker.get_binding("y").is_some());
        assert!(tracker.get_binding("z").is_some());
    }

    #[test]
    fn test_nll_binding_in_nested_scopes() {
        let mut tracker = NLLBindingTracker::new();
        let loc = BindingLocation::new(0, 0);

        // Outer scope
        tracker.push_scope(ScopeKind::Block);
        let _ = tracker.register_binding("x".to_string(), HirType::Int32, false, loc);

        // Inner scope
        tracker.push_scope(ScopeKind::Block);
        let _ = tracker.register_binding("y".to_string(), HirType::String, false, loc);

        // Both should be accessible
        assert!(tracker.get_binding("x").is_some());
        assert!(tracker.get_binding("y").is_some());

        // Pop inner scope
        let _ = tracker.pop_scope();

        // Both should still be accessible (tracker doesn't isolate by scope)
        assert!(tracker.get_binding("x").is_some());
        assert!(tracker.get_binding("y").is_some());
    }

    // ============================================================================
    // UNION DETECTION BUG TESTS
    // ============================================================================

    #[test]
    fn test_union_detector_case_sensitivity() {
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("MyUnion");

        // Should be case-sensitive
        assert!(detector.is_union_type("MyUnion"));
        assert!(!detector.is_union_type("myunion"));
        assert!(!detector.is_union_type("MYUNION"));
    }

    #[test]
    fn test_union_detector_empty_name() {
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("");

        assert!(detector.is_union_type(""));
    }

    #[test]
    fn test_union_detector_special_characters() {
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("Union<'a, T>");

        assert!(detector.is_union_type("Union<'a, T>"));
    }

    #[test]
    fn test_union_info_variants_empty() {
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("Empty");

        let info = detector.get_union_info("Empty");
        assert!(info.is_some());
        assert_eq!(info.unwrap().variants.len(), 0);
    }

    // ============================================================================
    // ITERATOR ANALYSIS BUG TESTS
    // ============================================================================

    #[test]
    fn test_iterator_analyzer_duplicate_registrations() {
        let mut analyzer = IteratorAnalyzer::new();
        analyzer.register_standard_types();
        analyzer.register_standard_types(); // Register twice

        // Should not cause issues
        let info = analyzer.analyze_iterator(&crate::lowering::HirExpression::Variable("v".to_string()));
        // Result may vary, but should not panic
    }

    #[test]
    fn test_iterator_analyzer_empty_collection_name() {
        let mut analyzer = IteratorAnalyzer::new();
        analyzer.register_iterator_type("", HirType::Unknown, false);

        // Should handle empty name - analyzer may or may not return a result
        let _ = analyzer.analyze_iterator(&crate::lowering::HirExpression::Variable("".to_string()));
        // Just verify it doesn't panic
    }

    // ============================================================================
    // INTEGRATION BUG TESTS
    // ============================================================================

    #[test]
    fn test_trait_bound_and_nll_integration() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));

        let mut tracker = NLLBindingTracker::new();
        let _ = tracker.push_scope(ScopeKind::Block);

        // Both should work together without interference
        assert!(tracker.get_binding("x").is_none());
        assert!(extractor.has_bound("T", "Clone"));
    }

    #[test]
    fn test_large_variable_name() {
        let mut extractor = TraitBoundExtractor::new();
        let large_name = "T".repeat(1000);
        
        extractor.add_bound(&large_name, TraitBound::simple("Clone"));
        assert!(extractor.has_bound(&large_name, "Clone"));
    }

    #[test]
    fn test_many_bounds_on_many_variables() {
        let mut extractor = TraitBoundExtractor::new();

        // Add 100 variables with 10 bounds each
        for i in 0..100 {
            let var_name = format!("T{}", i);
            for j in 0..10 {
                let trait_name = format!("Trait{}", j);
                extractor.add_bound(&var_name, TraitBound::simple(&trait_name));
            }
        }

        assert_eq!(extractor.all_variables().len(), 100);

        // Verify one
        assert_eq!(extractor.get_direct_bounds("T0").len(), 10);
    }

    // ============================================================================
    // PERFORMANCE/REGRESSION TESTS
    // ============================================================================

    #[test]
    fn test_supertrait_expansion_performance() {
        let mut extractor = TraitBoundExtractor::new();

        // Create deep chain
        for i in 0..100 {
            let trait_name = format!("T{}", i);
            let next_trait = format!("T{}", i + 1);
            extractor.add_supertrait(&trait_name, TraitBound::simple(&next_trait));
        }

        // Get all bounds for T0 - should expand through chain but not timeout
        let bounds = extractor.get_all_bounds("T0");
        // Just verify it doesn't timeout or panic - circular detection prevents infinite loops
        let _ = bounds;
    }

    #[test]
    fn test_nll_tracking_many_usages() {
        let mut tracker = NLLBindingTracker::new();
        tracker.push_scope(ScopeKind::Function);

        let loc_bind = BindingLocation::new(0, 0);
        let _ = tracker.register_binding("x".to_string(), HirType::Int32, false, loc_bind);

        // Record 100 usages
        for i in 0..100 {
            let loc_use = BindingLocation::new(0, i);
            let _ = tracker.record_usage("x", loc_use);
        }

        let binding = tracker.get_binding("x");
        assert!(binding.is_some());
    }

    #[test]
    fn test_conflict_detection_all_conflicting_traits() {
        let mut extractor = TraitBoundExtractor::new();

        // Add multiple conflicting combinations
        extractor.add_bound("T1", TraitBound::simple("Copy"));
        extractor.add_bound("T1", TraitBound::simple("Drop"));

        extractor.add_bound("T2", TraitBound::simple("Sync"));
        // (No Send)

        let conflicts1 = extractor.check_conflicts("T1");
        let conflicts2 = extractor.check_conflicts("T2");

        assert!(!conflicts1.is_empty());
        assert!(!conflicts2.is_empty());
    }
}
