//! # Phase 5A: Trait Bound Extraction - Comprehensive Testing
//!
//! Tests to find bugs and edge cases in trait bound extraction

use crate::borrowchecker::trait_bound_extractor::{TraitBound, TraitBoundExtractor, TraitHierarchy};
use crate::lowering::HirType;

#[cfg(test)]
mod phase5a_tests {
    use super::*;

    // ============================================================================
    // BASIC FUNCTIONALITY TESTS
    // ============================================================================

    #[test]
    fn test_empty_extractor_has_no_variables() {
        let extractor = TraitBoundExtractor::new();
        assert!(extractor.all_variables().is_empty());
    }

    #[test]
    fn test_add_single_bound_to_type_var() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));

        let vars = extractor.all_variables();
        assert_eq!(vars.len(), 1);
        assert!(vars.contains(&"T"));
    }

    #[test]
    fn test_get_direct_bounds_returns_ordered_list() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));
        extractor.add_bound("T", TraitBound::simple("Debug"));
        extractor.add_bound("T", TraitBound::simple("Display"));

        let bounds = extractor.get_direct_bounds("T");
        assert_eq!(bounds.len(), 3);

        let trait_names: Vec<_> = bounds.iter().map(|b| b.trait_name.as_str()).collect();
        assert_eq!(trait_names, vec!["Clone", "Debug", "Display"]);
    }

    // ============================================================================
    // EDGE CASE TESTS
    // ============================================================================

    #[test]
    fn test_adding_same_bound_twice() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));
        extractor.add_bound("T", TraitBound::simple("Clone")); // Add again

        let bounds = extractor.get_direct_bounds("T");
        // Both should be stored (duplicates not removed at add time)
        assert_eq!(bounds.len(), 2);
    }

    #[test]
    fn test_multiple_type_variables() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));
        extractor.add_bound("U", TraitBound::simple("Debug"));
        extractor.add_bound("V", TraitBound::simple("Display"));

        let vars = extractor.all_variables();
        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&"T"));
        assert!(vars.contains(&"U"));
        assert!(vars.contains(&"V"));
    }

    #[test]
    fn test_bounds_summary_for_empty_variable() {
        let extractor = TraitBoundExtractor::new();
        let summary = extractor.get_bounds_summary("NonExistent");
        assert_eq!(summary, "no bounds");
    }

    #[test]
    fn test_bounds_summary_with_single_bound() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));

        let summary = extractor.get_bounds_summary("T");
        assert!(summary.contains("Clone"));
    }

    #[test]
    fn test_bounds_summary_with_multiple_bounds() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));
        extractor.add_bound("T", TraitBound::simple("Debug"));

        let summary = extractor.get_bounds_summary("T");
        assert!(summary.contains("Clone"));
        assert!(summary.contains("Debug"));
        assert!(summary.contains("+"));
    }

    // ============================================================================
    // HIERARCHY TESTS
    // ============================================================================

    #[test]
    fn test_empty_hierarchy() {
        let hierarchy = TraitHierarchy::new();
        let supertraits = hierarchy.get_supertraits("Foo");
        assert!(supertraits.is_empty());
    }

    #[test]
    fn test_single_level_hierarchy() {
        let mut hierarchy = TraitHierarchy::new();
        hierarchy.add_supertrait("Clone", TraitBound::simple("CloneBase"));

        let supertraits = hierarchy.get_supertraits("Clone");
        assert_eq!(supertraits.len(), 1);
        assert_eq!(supertraits[0].trait_name, "CloneBase");
    }

    #[test]
    fn test_multi_level_hierarchy() {
        let mut hierarchy = TraitHierarchy::new();
        hierarchy.add_supertrait("Foo", TraitBound::simple("Bar"));
        hierarchy.add_supertrait("Bar", TraitBound::simple("Baz"));

        let supertraits = hierarchy.get_supertraits("Foo");
        assert_eq!(supertraits.len(), 2); // Bar and Baz

        let trait_names: Vec<_> = supertraits.iter().map(|b| b.trait_name.as_str()).collect();
        assert!(trait_names.contains(&"Bar"));
        assert!(trait_names.contains(&"Baz"));
    }

    #[test]
    fn test_circular_hierarchy_prevented() {
        let mut hierarchy = TraitHierarchy::new();
        hierarchy.add_supertrait("A", TraitBound::simple("B"));
        hierarchy.add_supertrait("B", TraitBound::simple("C"));
        hierarchy.add_supertrait("C", TraitBound::simple("A")); // Circular!

        // This should not cause infinite loop
        let supertraits = hierarchy.get_supertraits("A");
        assert!(supertraits.len() > 0);
        assert!(supertraits.len() < 10); // Should be bounded
    }

    // ============================================================================
    // DEDUPLICATION TESTS
    // ============================================================================

    #[test]
    fn test_all_bounds_deduplicates_by_trait_name() {
        let mut extractor = TraitBoundExtractor::new();

        // Add same trait via different paths
        extractor.add_supertrait("Foo", TraitBound::simple("Common"));
        extractor.add_supertrait("Bar", TraitBound::simple("Common"));

        // Add both Foo and Bar
        extractor.add_bound("T", TraitBound::simple("Foo"));
        extractor.add_bound("T", TraitBound::simple("Bar"));

        let all_bounds = extractor.get_all_bounds("T");
        let trait_names: Vec<_> = all_bounds.iter().map(|b| b.trait_name.as_str()).collect();

        // Common should appear only once
        let common_count = trait_names.iter().filter(|&&n| n == "Common").count();
        assert_eq!(common_count, 1, "Common trait should be deduplicated");
    }

    // ============================================================================
    // CONFLICT DETECTION TESTS
    // ============================================================================

    #[test]
    fn test_no_conflicts_for_normal_bounds() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));
        extractor.add_bound("T", TraitBound::simple("Debug"));

        let conflicts = extractor.check_conflicts("T");
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_copy_drop_conflict_detected() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Copy"));
        extractor.add_bound("T", TraitBound::simple("Drop"));

        let conflicts = extractor.check_conflicts("T");
        assert!(!conflicts.is_empty());
        assert!(conflicts[0].contains("Copy"));
        assert!(conflicts[0].contains("Drop"));
    }

    #[test]
    fn test_sync_without_send_detected() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Sync"));

        let conflicts = extractor.check_conflicts("T");
        assert!(!conflicts.is_empty());
        assert!(conflicts[0].contains("Sync"));
        assert!(conflicts[0].contains("Send"));
    }

    #[test]
    fn test_no_conflict_when_both_sync_and_send() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Sync"));
        extractor.add_bound("T", TraitBound::simple("Send"));

        let conflicts = extractor.check_conflicts("T");
        // Should not report Sync/Send conflict since Send is present
        let sync_send_conflicts: Vec<_> = conflicts
            .iter()
            .filter(|c| c.contains("Sync") && c.contains("Send"))
            .collect();
        assert!(sync_send_conflicts.is_empty());
    }

    // ============================================================================
    // COMPLIANCE VALIDATION TESTS
    // ============================================================================

    #[test]
    fn test_validate_with_no_bounds_always_succeeds() {
        let extractor = TraitBoundExtractor::new();

        assert!(extractor.validate_compliance("T", &HirType::Int32).is_ok());
        assert!(extractor.validate_compliance("T", &HirType::String).is_ok());
        assert!(extractor.validate_compliance("T", &HirType::Bool).is_ok());
    }

    #[test]
    fn test_validate_with_bounds_rejects_unknown_type() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));

        assert!(extractor.validate_compliance("T", &HirType::Unknown).is_err());
    }

    #[test]
    fn test_validate_with_bounds_accepts_concrete_types() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));

        // Concrete types should be accepted
        // (In a full implementation, we'd verify they implement Clone)
        assert!(extractor.validate_compliance("T", &HirType::Int32).is_ok());
        assert!(extractor.validate_compliance("T", &HirType::String).is_ok());
    }

    // ============================================================================
    // TRAIT BOUND STRUCTURE TESTS
    // ============================================================================

    #[test]
    fn test_simple_trait_bound_creation() {
        let bound = TraitBound::simple("Clone");

        assert_eq!(bound.trait_name, "Clone");
        assert!(bound.trait_params.is_empty());
        assert!(bound.lifetime_bounds.is_empty());
    }

    #[test]
    fn test_trait_bound_with_parameters() {
        let bound = TraitBound::with_params(
            "Iterator",
            vec![("Item".to_string(), HirType::String)],
        );

        assert_eq!(bound.trait_name, "Iterator");
        assert_eq!(bound.trait_params.len(), 1);
        assert_eq!(bound.trait_params[0].0, "Item");
    }

    #[test]
    fn test_trait_bound_with_lifetime() {
        let bound = TraitBound::simple("Clone").with_lifetime_bound("'a".to_string());

        assert_eq!(bound.trait_name, "Clone");
        assert_eq!(bound.lifetime_bounds.len(), 1);
        assert_eq!(bound.lifetime_bounds[0], "'a");
    }

    #[test]
    fn test_trait_bound_chaining() {
        let bound = TraitBound::simple("Clone")
            .with_lifetime_bound("'a".to_string())
            .with_lifetime_bound("'b".to_string());

        assert_eq!(bound.lifetime_bounds.len(), 2);
        assert_eq!(bound.lifetime_bounds[0], "'a");
        assert_eq!(bound.lifetime_bounds[1], "'b");
    }

    // ============================================================================
    // INTEGRATION TESTS
    // ============================================================================

    #[test]
    fn test_complex_generic_signature() {
        // Simulate: fn foo<T: Clone + Debug, U: Display + Send>(x: T, y: U)
        let mut extractor = TraitBoundExtractor::new();

        // T bounds
        extractor.add_bound("T", TraitBound::simple("Clone"));
        extractor.add_bound("T", TraitBound::simple("Debug"));

        // U bounds
        extractor.add_bound("U", TraitBound::simple("Display"));
        extractor.add_bound("U", TraitBound::simple("Send"));

        // Verify T
        assert!(extractor.has_bound("T", "Clone"));
        assert!(extractor.has_bound("T", "Debug"));
        assert!(!extractor.has_bound("T", "Display"));

        // Verify U
        assert!(extractor.has_bound("U", "Display"));
        assert!(extractor.has_bound("U", "Send"));
        assert!(!extractor.has_bound("U", "Clone"));
    }

    #[test]
    fn test_generic_bounds_with_supertrait_hierarchy() {
        // Simulate: Iterator : Foo, Foo : Bar
        // T : Iterator should imply T : Foo and T : Bar
        let mut extractor = TraitBoundExtractor::new();

        extractor.add_supertrait("Iterator", TraitBound::simple("IteratorBase"));
        extractor.add_supertrait("IteratorBase", TraitBound::simple("Marker"));

        extractor.add_bound("T", TraitBound::simple("Iterator"));

        let all_bounds = extractor.get_all_bounds("T");
        let trait_names: Vec<_> = all_bounds.iter().map(|b| b.trait_name.as_str()).collect();

        assert!(trait_names.contains(&"Iterator"));
        assert!(trait_names.contains(&"IteratorBase"));
        assert!(trait_names.contains(&"Marker"));
    }

    #[test]
    fn test_standard_trait_bounds_clone_debug_display() {
        let mut extractor = TraitBoundExtractor::new();

        // Register standard trait hierarchy
        extractor.add_supertrait("Clone", TraitBound::simple("Copy"));

        // T : Clone + Debug + Display
        extractor.add_bound("T", TraitBound::simple("Clone"));
        extractor.add_bound("T", TraitBound::simple("Debug"));
        extractor.add_bound("T", TraitBound::simple("Display"));

        // Verify all bounds present
        let summary = extractor.get_bounds_summary("T");
        assert!(summary.contains("Clone"));
        assert!(summary.contains("Debug"));
        assert!(summary.contains("Display"));

        // No conflicts expected
        let conflicts = extractor.check_conflicts("T");
        assert!(conflicts.is_empty());
    }

    // ============================================================================
    // STRESS TESTS
    // ============================================================================

    #[test]
    fn test_large_number_of_bounds() {
        let mut extractor = TraitBoundExtractor::new();

        // Add 100 bounds to same type var
        for i in 0..100 {
            let trait_name = format!("Trait{}", i);
            extractor.add_bound("T", TraitBound::simple(&trait_name));
        }

        let bounds = extractor.get_direct_bounds("T");
        assert_eq!(bounds.len(), 100);
    }

    #[test]
    fn test_many_type_variables() {
        let mut extractor = TraitBoundExtractor::new();

        // Add 50 type variables
        for i in 0..50 {
            let var_name = format!("T{}", i);
            extractor.add_bound(&var_name, TraitBound::simple("Clone"));
        }

        let vars = extractor.all_variables();
        assert_eq!(vars.len(), 50);
    }

    #[test]
    fn test_deep_supertrait_chain() {
        let mut extractor = TraitBoundExtractor::new();

        // Create chain: A -> B -> C -> ... -> Z (26 levels)
        let traits = [
            "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P",
            "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
        ];

        for i in 0..traits.len() - 1 {
            extractor.add_supertrait(traits[i], TraitBound::simple(traits[i + 1]));
        }

        extractor.add_bound("T", TraitBound::simple(traits[0]));

        let all_bounds = extractor.get_all_bounds("T");
        // Should have all traits in the chain
        assert!(all_bounds.len() > 20, "Should have found many bounds in chain");
    }
}
