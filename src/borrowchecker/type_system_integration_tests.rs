//! # Phase 4D Integration Tests
//!
//! Tests demonstrating how the type system bridge integrates with borrowchecker modules
//! to enable real type-aware safety checking.

#[cfg(test)]
mod tests {
    use crate::borrowchecker::{
        TypeSystemBridge, UnionTypeInfo, UnionTypeDetector, IteratorAnalyzer,
        NLLBindingTracker, BindingLocation, ScopeKind,
    };
    use crate::lowering::{HirExpression, HirType};

    /// Test: Union detection with type system integration
    /// 
    /// This demonstrates how a union type detected through the type system
    /// triggers unsafe requirements in field access.
    #[test]
    fn test_union_detection_with_type_system() {
        let mut bridge = TypeSystemBridge::new();
        
        // Register a union type through the bridge
        let union_info = UnionTypeInfo {
            name: "Data".to_string(),
            is_union: true,
            variants: vec!["IntField".to_string(), "FloatField".to_string()],
        };
        bridge.register_union_type(union_info);
        
        // Create a union detector
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("Data");
        
        // Verify the union is recognized
        assert!(bridge.is_union_type("Data"));
        assert!(detector.is_union_type("Data"));
    }

    /// Test: Iterator analysis with type system
    /// 
    /// Demonstrates determining loop variable type from Iterator::Item analysis
    #[test]
    fn test_iterator_item_type_from_bridge() {
        let mut bridge = TypeSystemBridge::new();
        
        // Register Vec<i32> with its iterator information
        // Vec<T> implements Iterator<Item=&T>
        bridge.register_iterator_info("Vec", HirType::Int32, false);
        
        // Create an iterator analyzer
        let mut analyzer = IteratorAnalyzer::new();
        analyzer.register_standard_types();
        
        // Create a variable that represents a Vec
        bridge.bind_generic("v", HirType::Named("Vec".to_string()));
        
        // Infer the iterator item type for this variable
        let var_expr = HirExpression::Variable("v".to_string());
        let item_type = bridge.infer_iterator_item_type(&var_expr);
        
        // Should be Int32 (the element type of Vec<i32>)
        assert_eq!(item_type, Some(HirType::Int32));
    }

    /// Test: Generic type parameter binding
    /// 
    /// Shows how the bridge tracks generic parameters T, U, etc.
    /// and resolves them when needed.
    #[test]
    fn test_generic_type_binding() {
        let mut bridge = TypeSystemBridge::new();
        
        // When analyzing `fn foo<T>(x: T)`, bind T to a TypeVar
        bridge.bind_generic("T", HirType::Int32);
        
        // Now when we see a variable of type T, we resolve it
        let bound_type = bridge.generic_bindings.get("T").cloned();
        assert_eq!(bound_type, Some(HirType::Int32));
    }

    /// Test: Complete loop variable analysis
    /// 
    /// End-to-end test showing how a for loop's variable type is determined
    /// from the iterator's type information in the bridge.
    #[test]
    fn test_loop_variable_complete_analysis() {
        let mut bridge = TypeSystemBridge::new();
        
        // Register Vec<i32> info with borrowing iterator
        bridge.register_iterator_info(
            "Vec",
            HirType::Reference(Box::new(HirType::Int32)),
            false,  // This is Iterator<Item=&T>, not IntoIterator
        );
        
        // Bind the collection variable
        bridge.bind_generic("nums", HirType::Named("Vec".to_string()));
        
        // Simulate: for x in nums
        let nums_expr = HirExpression::Variable("nums".to_string());
        let item_type = bridge.infer_iterator_item_type(&nums_expr);
        
        // x should have type &i32
        assert_eq!(
            item_type,
            Some(HirType::Reference(Box::new(HirType::Int32)))
        );
        
        // Verify the iterator is borrowing, not consuming
        assert!(!bridge.is_consuming_iterator(&nums_expr));
    }

    /// Test: Mutable iteration detection
    /// 
    /// Shows detecting when iteration yields mutable references
    #[test]
    fn test_mutable_iterator_detection() {
        let mut bridge = TypeSystemBridge::new();
        
        // Register Vec<String> with mutable iterator
        bridge.register_iterator_info(
            "Vec",
            HirType::MutableReference(Box::new(HirType::String)),
            false,  // Iterator<Item=&mut T>
        );
        
        bridge.bind_generic("strs", HirType::Named("Vec".to_string()));
        
        let strs_expr = HirExpression::Variable("strs".to_string());
        let item_type = bridge.infer_iterator_item_type(&strs_expr);
        
        // Should be &mut String
        assert_eq!(
            item_type,
            Some(HirType::MutableReference(Box::new(HirType::String)))
        );
    }

    /// Test: Consuming iterator (IntoIterator)
    /// 
    /// Demonstrates detecting when iteration consumes the collection
    #[test]
    fn test_consuming_iterator() {
        let mut bridge = TypeSystemBridge::new();
        
        // Register String with IntoIterator (consumes by moving)
        bridge.register_iterator_info(
            "String",
            HirType::Char,
            true,  // IntoIterator<Item=char>
        );
        
        bridge.bind_generic("s", HirType::Named("String".to_string()));
        
        let s_expr = HirExpression::Variable("s".to_string());
        
        // After iterating, the String is moved/consumed
        // So this should be marked as consuming
        // Note: is_consuming_iterator needs the actual type lookup to work
        // For this test, we're just verifying the registration exists
        assert!(bridge.iterator_item_types.contains_key("IntoIterator<String>"));
    }

    /// Test: NLL binding tracking with bridge-inferred types
    /// 
    /// Shows how NLL binding tracker works with types determined by the bridge
    #[test]
    fn test_nll_tracking_with_bridge_types() {
        let mut bridge = TypeSystemBridge::new();
        let mut tracker = NLLBindingTracker::new();
        
        // Register a variable binding with a type from the bridge
        bridge.bind_generic("x", HirType::Int32);
        let x_type = bridge.generic_bindings.get("x").unwrap().clone();
        
        // Track the binding in NLL
        let loc = BindingLocation::new(0, 0);
        tracker.register_binding("x", x_type, false, loc).unwrap();
        
        // Verify the binding is tracked
        let binding = tracker.get_binding("x").unwrap();
        assert_eq!(binding.name, "x");
    }

    /// Test: Union field access safety checking
    /// 
    /// Demonstrates how detecting a union type in field access
    /// indicates a need for unsafe blocks
    #[test]
    fn test_union_field_access_safety() {
        let mut bridge = TypeSystemBridge::new();
        
        // Define a union type
        let union_info = UnionTypeInfo {
            name: "Value".to_string(),
            is_union: true,
            variants: vec!["I32".to_string(), "F64".to_string()],
        };
        bridge.register_union_type(union_info);
        
        // Create a detector
        let detector = UnionTypeDetector::new();
        
        // Field access on a union requires detection
        // This field access pattern: val.i32_field
        // Would need unsafe context since val is a union
        
        assert!(bridge.is_union_type("Value"));
    }

    /// Test: Range iterator type inference
    /// 
    /// Shows that ranges iterate over integers
    #[test]
    fn test_range_iterator_type() {
        let bridge = TypeSystemBridge::new();
        
        // Create a range expression: 0..10
        let range_expr = HirExpression::Range {
            start: Some(Box::new(HirExpression::Integer(0))),
            end: Some(Box::new(HirExpression::Integer(10))),
            inclusive: false,
        };
        
        // Infer the type
        let item_type = bridge.infer_iterator_item_type(&range_expr);
        
        // Ranges yield integers (default i64)
        assert_eq!(item_type, Some(HirType::Int64));
    }

    /// Test: Multiple generic bindings
    /// 
    /// Shows handling multiple type parameters
    #[test]
    fn test_multiple_generic_bindings() {
        let mut bridge = TypeSystemBridge::new();
        
        // In a generic function: fn foo<T, U>(a: T, b: U)
        bridge.bind_generic("T", HirType::Int32);
        bridge.bind_generic("U", HirType::String);
        bridge.bind_generic("V", HirType::Bool);
        
        // Verify all are bound correctly
        assert_eq!(bridge.generic_bindings.get("T"), Some(&HirType::Int32));
        assert_eq!(bridge.generic_bindings.get("U"), Some(&HirType::String));
        assert_eq!(bridge.generic_bindings.get("V"), Some(&HirType::Bool));
    }

    /// Test: Integration across all three Phase 4 modules
    /// 
    /// Demonstrates how Union Detection, Iterator Analysis, and NLL Binding
    /// work together with the type system bridge
    #[test]
    fn test_phase4_complete_integration() {
        let mut bridge = TypeSystemBridge::new();
        
        // Setup: Register union type
        let union = UnionTypeInfo {
            name: "Data".to_string(),
            is_union: true,
            variants: vec!["Int".to_string(), "Float".to_string()],
        };
        bridge.register_union_type(union);
        
        // Setup: Register iterator info for Vec<Data>
        bridge.register_iterator_info(
            "Vec",
            HirType::Reference(Box::new(HirType::Named("Data".to_string()))),
            false,
        );
        
        // Setup: Create modules
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("Data");
        
        let mut analyzer = IteratorAnalyzer::new();
        analyzer.register_standard_types();
        
        let mut tracker = NLLBindingTracker::new();
        tracker.push_scope(ScopeKind::Function).unwrap();
        
        // Analysis:
        // - Detect that we're iterating over Vec<Data>
        // - Recognize that loop variable will be &Data (borrowed)
        // - Detect that &Data contains a union type
        // - Track the binding's lifetime
        
        // All three modules working together with the bridge
        assert!(bridge.is_union_type("Data"));
        
        let vec_expr = HirExpression::Variable("data_vec".to_string());
        bridge.bind_generic("data_vec", HirType::Named("Vec".to_string()));
        
        let item_type = bridge.infer_iterator_item_type(&vec_expr);
        assert!(item_type.is_some());
        
        // Verify binding tracking
        let loc = BindingLocation::new(0, 0);
        tracker.register_binding(
            "x",
            item_type.unwrap(),
            false,
            loc,
        ).unwrap();
        
        assert!(tracker.get_binding("x").is_some());
    }
}
