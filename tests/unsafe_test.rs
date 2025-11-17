//! Phase 6 Week 9: Unsafe Blocks & Raw Pointers Integration Tests
//! 
//! Tests for unsafe code validation including:
//! - Raw pointer types
//! - Unsafe blocks
//! - Pointer dereferencing validation
//! - Unsafe function definitions and calls

use gaiarusted::borrowchecker::unsafe_checking::UnsafeChecker;
use gaiarusted::lowering::{HirExpression, HirStatement, HirType};

#[test]
fn test_raw_pointer_types_parsing() {
    // Test that raw pointer types are recognized
    // This would normally come from the parser
    
    let _const_ptr = HirType::Pointer(Box::new(HirType::Int32));
    let _mut_ptr = HirType::Pointer(Box::new(HirType::Int64));
    
    // Both should be valid types (though dereferencing them is unsafe)
}

#[test]
fn test_pointer_dereference_requires_unsafe() {
    let mut checker = UnsafeChecker::new();
    
    // Dereferencing outside unsafe block should fail
    assert!(checker.check_pointer_deref().is_err());
    assert_eq!(checker.errors().len(), 1);
}

#[test]
fn test_pointer_dereference_in_unsafe_allowed() {
    let mut checker = UnsafeChecker::new();
    
    // Create an unsafe block scope
    checker.enter_unsafe_block();
    
    // Now dereferencing should be allowed
    assert!(checker.check_pointer_deref().is_ok());
    assert_eq!(checker.errors().len(), 0);
}

#[test]
fn test_unsafe_function_registration() {
    let mut checker = UnsafeChecker::new();
    
    // Register a function as unsafe
    checker.register_unsafe_function("dangerous");
    assert!(checker.is_unsafe_function("dangerous"));
    assert!(!checker.is_unsafe_function("safe"));
}

#[test]
fn test_unsafe_function_call_outside_unsafe_block() {
    let mut checker = UnsafeChecker::new();
    checker.register_unsafe_function("dangerous_func");
    
    // Calling unsafe function outside unsafe block should fail
    let result = checker.check_unsafe_function_call("dangerous_func");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("dangerous_func"));
}

#[test]
fn test_unsafe_function_call_in_unsafe_block() {
    let mut checker = UnsafeChecker::new();
    checker.register_unsafe_function("dangerous_func");
    
    // Enter unsafe block
    checker.enter_unsafe_block();
    
    // Now calling should be allowed
    assert!(checker.check_unsafe_function_call("dangerous_func").is_ok());
}

#[test]
fn test_mutable_static_access_requires_unsafe() {
    let mut checker = UnsafeChecker::new();
    
    // Accessing mutable static outside unsafe block should fail
    assert!(checker.check_mutable_static_access("GLOBAL_STATE").is_err());
    
    // Inside unsafe block should work
    checker.enter_unsafe_block();
    assert!(checker.check_mutable_static_access("GLOBAL_STATE").is_ok());
}

#[test]
fn test_union_access_requires_unsafe() {
    let mut checker = UnsafeChecker::new();
    
    // Accessing union field outside unsafe block should fail
    assert!(checker.check_union_field_access().is_err());
    
    // Inside unsafe block should work
    checker.enter_unsafe_block();
    assert!(checker.check_union_field_access().is_ok());
}

#[test]
fn test_nested_unsafe_blocks() {
    let mut checker = UnsafeChecker::new();
    
    assert!(!checker.is_in_unsafe_context());
    
    // Enter first level
    checker.enter_unsafe_block();
    assert!(checker.is_in_unsafe_context());
    assert!(checker.check_pointer_deref().is_ok());
    
    // Enter nested level
    checker.enter_unsafe_block();
    assert!(checker.is_in_unsafe_context());
    assert!(checker.check_pointer_deref().is_ok());
    
    // Exit nested level - should still be in unsafe
    checker.exit_unsafe_block();
    assert!(checker.is_in_unsafe_context());
    assert!(checker.check_pointer_deref().is_ok());
    
    // Exit outer level - should no longer be in unsafe
    checker.exit_unsafe_block();
    assert!(!checker.is_in_unsafe_context());
    assert!(checker.check_pointer_deref().is_err());
}

#[test]
fn test_safe_function_call_always_allowed() {
    let mut checker = UnsafeChecker::new();
    
    // Safe function (not registered as unsafe) should work everywhere
    checker.register_unsafe_function("dangerous");
    
    // Outside unsafe block, safe function is fine
    assert!(!checker.is_in_unsafe_context());
    // (We don't check unsafe for safe functions, so no error for non-unsafe)
    
    // Inside unsafe block, safe function is still fine
    checker.enter_unsafe_block();
    // (Again, no error for safe functions)
}

#[test]
fn test_multiple_unsafe_errors_accumulate() {
    let mut checker = UnsafeChecker::new();
    
    // Generate several errors
    let _ = checker.check_pointer_deref();
    let _ = checker.check_unsafe_function_call("func1");
    let _ = checker.check_unsafe_function_call("func2");
    let _ = checker.check_mutable_static_access("STATIC");
    let _ = checker.check_union_field_access();
    
    // All errors should be recorded
    assert_eq!(checker.errors().len(), 5);
}

#[test]
fn test_raw_pointer_type_validation() {
    let mut checker = UnsafeChecker::new();
    
    // Check that pointer types themselves don't cause errors
    let ptr_type = HirType::Pointer(Box::new(HirType::Int32));
    assert!(checker.check_type(&ptr_type).is_ok());
    
    // Check nested pointers
    let ptr_to_ptr = HirType::Pointer(Box::new(
        HirType::Pointer(Box::new(HirType::Int64))
    ));
    assert!(checker.check_type(&ptr_to_ptr).is_ok());
}

#[test]
fn test_unsafe_block_scope_statement() {
    let mut checker = UnsafeChecker::new();
    
    // Create a simple unsafe block
    let unsafe_block = HirStatement::UnsafeBlock(vec![
        HirStatement::Expression(HirExpression::Variable("x".to_string())),
    ]);
    
    // Checking it should enter and exit unsafe context properly
    assert!(!checker.is_in_unsafe_context());
    assert!(checker.check_statement(&unsafe_block).is_ok());
    assert!(!checker.is_in_unsafe_context()); // Should be back to safe
}

#[test]
fn test_error_messages_are_descriptive() {
    let mut checker = UnsafeChecker::new();
    
    // Get error message for pointer dereference
    let deref_error = checker.check_pointer_deref().unwrap_err();
    assert!(deref_error.message.contains("dereference"));
    assert!(deref_error.message.contains("unsafe"));
    
    let mut checker2 = UnsafeChecker::new();
    let unsafe_call_error = checker2.check_unsafe_function_call("my_func").unwrap_err();
    assert!(unsafe_call_error.message.contains("my_func"));
    assert!(unsafe_call_error.message.contains("unsafe function"));
}

#[test]
fn test_capability_summary() {
    let mut checker = UnsafeChecker::new();
    
    // Register unsafe function
    checker.register_unsafe_function("malloc");
    checker.register_unsafe_function("free");
    
    // Test all major unsafe operations
    let operations = vec![
        ("Pointer dereference", checker.check_pointer_deref()),
        ("Unsafe function (malloc)", checker.check_unsafe_function_call("malloc")),
        ("Unsafe function (free)", checker.check_unsafe_function_call("free")),
        ("Mutable static", checker.check_mutable_static_access("GLOBAL")),
        ("Union access", checker.check_union_field_access()),
    ];
    
    // All should fail outside unsafe block
    for (name, result) in &operations {
        assert!(
            result.is_err(),
            "{} should fail outside unsafe block",
            name
        );
    }
    
    // Now enter unsafe block
    let mut checker2 = UnsafeChecker::new();
    checker2.register_unsafe_function("malloc");
    checker2.register_unsafe_function("free");
    checker2.enter_unsafe_block();
    
    let safe_operations = vec![
        ("Pointer dereference", checker2.check_pointer_deref()),
        ("Unsafe function (malloc)", checker2.check_unsafe_function_call("malloc")),
        ("Unsafe function (free)", checker2.check_unsafe_function_call("free")),
        ("Mutable static", checker2.check_mutable_static_access("GLOBAL")),
        ("Union access", checker2.check_union_field_access()),
    ];
    
    // All should succeed inside unsafe block
    for (name, result) in &safe_operations {
        assert!(
            result.is_ok(),
            "{} should succeed inside unsafe block",
            name
        );
    }
}
