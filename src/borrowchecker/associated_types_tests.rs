//! # Phase 5B: Associated Type Resolution - Integration Tests
//!
//! Comprehensive integration tests for associated type resolution.

#[cfg(test)]
mod phase5b_tests {
    use crate::borrowchecker::associated_type_resolver::{
        AssociatedTypeResolver, IteratorTypeRegistry, AssociatedTypeMapping,
    };
    use crate::lowering::HirType;

    // ============================================================================
    // BASIC RESOLVER TESTS
    // ============================================================================

    #[test]
    fn test_resolver_create_empty() {
        let resolver = AssociatedTypeResolver::new();
        assert_eq!(resolver.total_associations(), 0);
    }

    #[test]
    fn test_resolver_with_standard_types() {
        let resolver = AssociatedTypeResolver::with_standard_types();
        // Should have at least the 3 standard types
        assert!(resolver.total_associations() >= 3);
    }

    #[test]
    fn test_register_single_association() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);

        assert_eq!(resolver.total_associations(), 1);
        assert!(resolver.has_assoc_type("Iterator", "Item"));
    }

    #[test]
    fn test_register_multiple_associations() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);
        resolver.register_assoc_type("Iterator", "Size", HirType::USize);
        resolver.register_assoc_type("Default", "Output", HirType::Bool);

        assert_eq!(resolver.total_associations(), 3);
    }

    // ============================================================================
    // RESOLUTION TESTS
    // ============================================================================

    #[test]
    fn test_resolve_exists() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::String);

        let result = resolver.resolve("Iterator", "Item");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), HirType::String);
    }

    #[test]
    fn test_resolve_not_exists() {
        let resolver = AssociatedTypeResolver::new();
        
        let result = resolver.resolve("NonExistent", "Item");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_wrong_type_name() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);

        let result = resolver.resolve("Iterator", "WrongName");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_case_sensitive() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);

        let result_correct = resolver.resolve("Iterator", "Item");
        let result_wrong_trait = resolver.resolve("iterator", "Item");
        let result_wrong_type = resolver.resolve("Iterator", "item");

        assert!(result_correct.is_some());
        assert!(result_wrong_trait.is_none());
        assert!(result_wrong_type.is_none());
    }

    // ============================================================================
    // ITERATOR RESOLUTION TESTS
    // ============================================================================

    #[test]
    fn test_resolve_iterator_item_vec() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_iterator_type("Vec", HirType::Int32);

        let item = resolver.resolve_iterator_item(&HirType::Named("Vec".to_string()));
        assert!(item.is_some());
        assert_eq!(item.unwrap(), HirType::Int32);
    }

    #[test]
    fn test_resolve_iterator_item_string() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_iterator_type("String", HirType::Char);

        let item = resolver.resolve_iterator_item(&HirType::Named("String".to_string()));
        assert!(item.is_some());
        assert_eq!(item.unwrap(), HirType::Char);
    }

    #[test]
    fn test_resolve_iterator_item_through_reference() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_iterator_type("Vec", HirType::Int32);

        let vec_ref = HirType::Reference(Box::new(HirType::Named("Vec".to_string())));
        let item = resolver.resolve_iterator_item(&vec_ref);
        
        assert!(item.is_some());
        assert_eq!(item.unwrap(), HirType::Int32);
    }

    #[test]
    fn test_resolve_iterator_item_through_mut_reference() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_iterator_type("Vec", HirType::String);

        let vec_mut_ref = HirType::MutableReference(Box::new(HirType::Named("Vec".to_string())));
        let item = resolver.resolve_iterator_item(&vec_mut_ref);
        
        assert!(item.is_some());
        assert_eq!(item.unwrap(), HirType::String);
    }

    #[test]
    fn test_resolve_iterator_item_deep_references() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_iterator_type("Vec", HirType::Bool);

        let deep_ref = HirType::Reference(Box::new(HirType::MutableReference(Box::new(
            HirType::Named("Vec".to_string()),
        ))));
        
        let item = resolver.resolve_iterator_item(&deep_ref);
        assert!(item.is_some());
        assert_eq!(item.unwrap(), HirType::Bool);
    }

    #[test]
    fn test_resolve_iterator_item_not_found() {
        let resolver = AssociatedTypeResolver::new();
        
        let item = resolver.resolve_iterator_item(&HirType::Named("Unknown".to_string()));
        assert!(item.is_none());
    }

    #[test]
    fn test_resolve_for_loop_iterator() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_iterator_type("Array", HirType::Int64);

        let result = resolver.resolve_for_loop_iterator(&HirType::Named("Array".to_string()));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), HirType::Int64);
    }

    // ============================================================================
    // REGISTRY TESTS
    // ============================================================================

    #[test]
    fn test_registry_new() {
        let registry = IteratorTypeRegistry::new();
        assert!(registry.get_item_type("Vec").is_none());
    }

    #[test]
    fn test_registry_register() {
        let mut registry = IteratorTypeRegistry::new();
        registry.register_collection("MyCollection", HirType::Float64);

        assert!(registry.get_item_type("MyCollection").is_some());
        assert_eq!(registry.get_item_type("MyCollection").unwrap(), HirType::Float64);
    }

    #[test]
    fn test_registry_standard_types() {
        let registry = IteratorTypeRegistry::with_standard_types();

        // Check some standard collections
        assert!(registry.get_item_type("Vec").is_some());
        assert!(registry.get_item_type("String").is_some());
        assert!(registry.get_item_type("HashMap").is_some());
        assert!(registry.get_item_type("HashSet").is_some());
    }

    #[test]
    fn test_registry_multiple_registrations() {
        let mut registry = IteratorTypeRegistry::new();
        registry.register_collection("List", HirType::Int32);
        registry.register_collection("Set", HirType::String);
        registry.register_collection("Map", HirType::Bool);

        assert!(registry.get_item_type("List").is_some());
        assert!(registry.get_item_type("Set").is_some());
        assert!(registry.get_item_type("Map").is_some());
    }

    #[test]
    fn test_registry_overwrite() {
        let mut registry = IteratorTypeRegistry::new();
        registry.register_collection("Collection", HirType::Int32);
        registry.register_collection("Collection", HirType::String);

        // Last registration should win
        assert_eq!(registry.get_item_type("Collection").unwrap(), HirType::String);
    }

    // ============================================================================
    // TRAIT ASSOCIATION TESTS
    // ============================================================================

    #[test]
    fn test_get_trait_associations_empty() {
        let resolver = AssociatedTypeResolver::new();
        let assocs = resolver.get_trait_associations("Iterator");
        assert!(assocs.is_empty());
    }

    #[test]
    fn test_get_trait_associations_single() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);

        let assocs = resolver.get_trait_associations("Iterator");
        assert_eq!(assocs.len(), 1);
        assert_eq!(assocs[0].0, "Item");
    }

    #[test]
    fn test_get_trait_associations_multiple() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);
        resolver.register_assoc_type("Iterator", "Size", HirType::USize);
        resolver.register_assoc_type("Iterator", "Index", HirType::USize);

        let assocs = resolver.get_trait_associations("Iterator");
        assert_eq!(assocs.len(), 3);
    }

    #[test]
    fn test_get_all_associations() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);
        resolver.register_assoc_type("Default", "Output", HirType::Bool);

        let all = resolver.get_all_associations();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_has_assoc_type() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);

        assert!(resolver.has_assoc_type("Iterator", "Item"));
        assert!(!resolver.has_assoc_type("Iterator", "Other"));
        assert!(!resolver.has_assoc_type("Other", "Item"));
    }

    // ============================================================================
    // GENERICS TESTS
    // ============================================================================

    #[test]
    fn test_register_trait_generics() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_trait_generics("Iterator", vec!["T".to_string()]);

        // The generics should be stored (accessed via get_all_associations)
        let all = resolver.get_all_associations();
        // No associations yet, so empty
        assert!(all.is_empty());
    }

    #[test]
    fn test_register_trait_generics_multiple() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_trait_generics("Iterator", vec![
            "T".to_string(),
            "U".to_string(),
            "V".to_string(),
        ]);

        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);
        let all = resolver.get_all_associations();
        
        // One association with three generic params
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].generic_params.len(), 3);
    }

    // ============================================================================
    // MAPPING TESTS
    // ============================================================================

    #[test]
    fn test_associated_type_mapping_creation() {
        let mapping = AssociatedTypeMapping::new("Iterator", "Item", HirType::Int32);
        
        assert_eq!(mapping.trait_name, "Iterator");
        assert_eq!(mapping.type_name, "Item");
        assert_eq!(mapping.resolved_type, HirType::Int32);
        assert!(mapping.generic_params.is_empty());
    }

    #[test]
    fn test_associated_type_mapping_with_generics() {
        let params = vec!["T".to_string(), "U".to_string()];
        let mapping = AssociatedTypeMapping::with_generics(
            "Iterator",
            "Item",
            HirType::String,
            params.clone(),
        );

        assert_eq!(mapping.generic_params, params);
    }

    // ============================================================================
    // SUMMARY TESTS
    // ============================================================================

    #[test]
    fn test_summary_empty() {
        let resolver = AssociatedTypeResolver::new();
        let summary = resolver.get_summary();
        assert_eq!(summary, "No associated types registered");
    }

    #[test]
    fn test_summary_single() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);

        let summary = resolver.get_summary();
        assert!(!summary.is_empty());
        assert!(summary.contains("Iterator"));
    }

    #[test]
    fn test_summary_multiple_traits() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);
        resolver.register_assoc_type("Default", "Output", HirType::Bool);

        let summary = resolver.get_summary();
        // Should mention at least one of the traits
        assert!(summary.contains("Iterator") || summary.contains("Default"));
    }

    // ============================================================================
    // STRESS TESTS
    // ============================================================================

    #[test]
    fn test_many_associations_single_trait() {
        let mut resolver = AssociatedTypeResolver::new();

        // Register 50 different type names for Iterator
        for i in 0..50 {
            let type_name = format!("Type{}", i);
            resolver.register_assoc_type("Iterator", &type_name, HirType::Int32);
        }

        assert_eq!(resolver.total_associations(), 50);
        assert!(resolver.has_assoc_type("Iterator", "Type0"));
        assert!(resolver.has_assoc_type("Iterator", "Type49"));
    }

    #[test]
    fn test_many_traits_many_associations() {
        let mut resolver = AssociatedTypeResolver::new();

        // Register 30 traits, each with 5 types
        for i in 0..30 {
            let trait_name = format!("Trait{}", i);
            for j in 0..5 {
                let type_name = format!("Type{}", j);
                resolver.register_assoc_type(&trait_name, &type_name, HirType::Int32);
            }
        }

        assert_eq!(resolver.total_associations(), 150);
    }

    #[test]
    fn test_many_iterator_registrations() {
        let mut resolver = AssociatedTypeResolver::new();

        // Register 100 collection types
        for i in 0..100 {
            let name = format!("Collection{}", i);
            resolver.register_iterator_type(&name, HirType::Int32);
        }

        // Test a few random ones
        let item1 = resolver.resolve_iterator_item(&HirType::Named("Collection0".to_string()));
        let item50 = resolver.resolve_iterator_item(&HirType::Named("Collection50".to_string()));
        let item99 = resolver.resolve_iterator_item(&HirType::Named("Collection99".to_string()));

        assert!(item1.is_some());
        assert!(item50.is_some());
        assert!(item99.is_some());
    }

    // ============================================================================
    // INTEGRATION TESTS
    // ============================================================================

    #[test]
    fn test_standard_library_simulation() {
        let resolver = AssociatedTypeResolver::with_standard_types();

        // Vec should have Iterator::Item
        assert!(resolver.has_assoc_type("Iterator", "Item"));
        
        // IntoIterator should have Item
        assert!(resolver.has_assoc_type("IntoIterator", "Item"));
    }

    #[test]
    fn test_custom_types_with_standard() {
        let mut resolver = AssociatedTypeResolver::with_standard_types();
        
        // Add custom collection
        resolver.register_iterator_type("MyVec", HirType::Float64);

        // Both standard and custom should work
        assert!(resolver.resolve("Iterator", "Item").is_some());
        let custom_item = resolver.resolve_iterator_item(&HirType::Named("MyVec".to_string()));
        assert!(custom_item.is_some());
    }

    #[test]
    fn test_multiple_references_resolution() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_iterator_type("Vec", HirType::String);

        // Test: &Vec, &&Vec, &mut Vec, &mut &Vec
        let single_ref = HirType::Reference(Box::new(HirType::Named("Vec".to_string())));
        let double_ref = HirType::Reference(Box::new(single_ref.clone()));
        let mut_ref = HirType::MutableReference(Box::new(HirType::Named("Vec".to_string())));
        let mut_ref_ref = HirType::MutableReference(Box::new(single_ref.clone()));

        assert!(resolver.resolve_iterator_item(&single_ref).is_some());
        assert!(resolver.resolve_iterator_item(&double_ref).is_some());
        assert!(resolver.resolve_iterator_item(&mut_ref).is_some());
        assert!(resolver.resolve_iterator_item(&mut_ref_ref).is_some());
    }

    #[test]
    fn test_compatibility_with_phase5a() {
        // This test verifies Phase 5B can be used alongside Phase 5A
        use crate::borrowchecker::trait_bound_extractor::{TraitBound, TraitBoundExtractor};

        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Iterator"));

        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);

        // Both should work together
        assert!(extractor.has_bound("T", "Iterator"));
        assert!(resolver.resolve("Iterator", "Item").is_some());
    }

    // ============================================================================
    // EDGE CASE TESTS
    // ============================================================================

    #[test]
    fn test_empty_trait_name() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("", "Item", HirType::Int32);

        assert!(resolver.has_assoc_type("", "Item"));
        assert_eq!(resolver.resolve("", "Item").unwrap(), HirType::Int32);
    }

    #[test]
    fn test_empty_type_name() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "", HirType::Int32);

        assert!(resolver.has_assoc_type("Iterator", ""));
        assert_eq!(resolver.resolve("Iterator", "").unwrap(), HirType::Int32);
    }

    #[test]
    fn test_empty_collection_name() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_iterator_type("", HirType::Int32);

        let item = resolver.resolve_iterator_item(&HirType::Named("".to_string()));
        assert!(item.is_some());
    }

    #[test]
    fn test_unicode_trait_names() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Trait🎯", "Item🎯", HirType::Int32);

        assert!(resolver.has_assoc_type("Trait🎯", "Item🎯"));
    }

    #[test]
    fn test_large_trait_name() {
        let mut resolver = AssociatedTypeResolver::new();
        let large_name = "T".repeat(1000);
        
        resolver.register_assoc_type(&large_name, "Item", HirType::Int32);
        assert!(resolver.has_assoc_type(&large_name, "Item"));
    }

    #[test]
    fn test_clone_and_equality() {
        let mapping1 = AssociatedTypeMapping::new("Iterator", "Item", HirType::Int32);
        let mapping2 = mapping1.clone();

        assert_eq!(mapping1, mapping2);
    }
}
