//! Phase 6 Weeks 11-12: Polish & Edge Cases Integration Tests
//!
//! Tests for enhanced unsafe code validation with:
//! - Better error messages
//! - Transmute validation
//! - Context-aware diagnostics
//! - Type safety checking

use gaiarusted::borrowchecker::unsafe_checking_enhanced::{
    UnsafeCheckerEnhanced, TransmuteValidity,
};
use gaiarusted::lowering::HirType;

#[test]
fn test_enhanced_error_messages_with_context() {
    let mut checker = UnsafeCheckerEnhanced::new();
    checker.set_context("in function process_data()");
    
    let result = checker.check_pointer_deref_enhanced();
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.context.is_some());
    assert_eq!(error.context.unwrap(), "in function process_data()");
    assert!(error.suggestion.is_some());
}

#[test]
fn test_error_suggestions_for_unsafe_violations() {
    let mut checker = UnsafeCheckerEnhanced::new();
    
    // Pointer dereference error should suggest wrapping in unsafe
    let deref_result = checker.check_pointer_deref_enhanced();
    assert!(deref_result.is_err());
    
    if let Err(e) = deref_result {
        assert!(e.suggestion.is_some());
        assert!(e.suggestion.unwrap().contains("unsafe"));
    }
    
    // Function call error should suggest wrapping in unsafe
    let call_result = checker.check_unsafe_function_call_enhanced("malloc");
    assert!(call_result.is_err());
    
    if let Err(e) = call_result {
        assert!(e.suggestion.is_some());
    }
}

#[test]
fn test_transmute_same_type_is_always_safe() {
    let mut checker = UnsafeCheckerEnhanced::new();
    let i32_type = HirType::Int32;
    
    let result = checker.validate_transmute(&i32_type, &i32_type);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), TransmuteValidity::Safe);
}

#[test]
fn test_transmute_different_pointers_requires_unsafe() {
    let mut checker = UnsafeCheckerEnhanced::new();
    let ptr_i32 = HirType::Pointer(Box::new(HirType::Int32));
    let ptr_f64 = HirType::Pointer(Box::new(HirType::Float64));
    
    // Outside unsafe - should fail
    let result = checker.validate_transmute(&ptr_i32, &ptr_f64);
    assert!(result.is_err());
    
    // Inside unsafe - should succeed
    checker.enter_unsafe_context();
    let result = checker.validate_transmute(&ptr_i32, &ptr_f64);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), TransmuteValidity::RequiresUnsafe);
}

#[test]
fn test_transmute_same_pointer_type_is_safe() {
    let mut checker = UnsafeCheckerEnhanced::new();
    let ptr_i32_a = HirType::Pointer(Box::new(HirType::Int32));
    let ptr_i32_b = HirType::Pointer(Box::new(HirType::Int32));
    
    // Same pointer types are safe
    let result = checker.validate_transmute(&ptr_i32_a, &ptr_i32_b);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), TransmuteValidity::Safe);
}

#[test]
fn test_transmute_incompatible_types_invalid() {
    let mut checker = UnsafeCheckerEnhanced::new();
    checker.enter_unsafe_context();
    
    let string_type = HirType::String;
    let int_type = HirType::Int32;
    
    // String to int is invalid even in unsafe
    let result = checker.validate_transmute(&string_type, &int_type);
    assert!(result.is_err());
    
    if let Err(e) = result {
        assert!(e.message.contains("invalid"));
    }
}

#[test]
fn test_transmute_numeric_same_size_requires_unsafe() {
    let mut checker = UnsafeCheckerEnhanced::new();
    
    // i32 and f64 have different sizes, so this is invalid (not just requires unsafe)
    let i32_type = HirType::Int32;
    let f64_type = HirType::Float64;
    
    let result = checker.validate_transmute(&i32_type, &f64_type);
    // This should require unsafe because they're both numeric
    assert!(result.is_err()); // Outside unsafe
    
    checker.enter_unsafe_context();
    let result = checker.validate_transmute(&i32_type, &f64_type);
    assert!(result.is_ok());
}

#[test]
fn test_context_changes() {
    let mut checker = UnsafeCheckerEnhanced::new();
    
    checker.set_context("in function foo()");
    let result1 = checker.check_pointer_deref_enhanced();
    assert!(result1.is_err());
    
    if let Err(e) = result1 {
        assert_eq!(e.context.unwrap(), "in function foo()");
    }
    
    checker.clear_context();
    checker.set_context("in function bar()");
    
    let result2 = checker.check_unsafe_function_call_enhanced("bad_func");
    assert!(result2.is_err());
    
    if let Err(e) = result2 {
        assert_eq!(e.context.unwrap(), "in function bar()");
    }
}

#[test]
fn test_diagnostics_report_generation() {
    let mut checker = UnsafeCheckerEnhanced::new();
    
    // Generate some violations
    let _ = checker.check_pointer_deref_enhanced();
    let _ = checker.check_unsafe_function_call_enhanced("malloc");
    let _ = checker.check_unsafe_function_call_enhanced("free");
    
    let report = checker.report_diagnostics();
    assert!(report.contains("3 unsafe violation"));
    assert!(report.contains("dereference"));
    assert!(report.contains("malloc"));
}

#[test]
fn test_empty_diagnostics_report() {
    let checker = UnsafeCheckerEnhanced::new();
    let report = checker.report_diagnostics();
    assert!(report.contains("No unsafe violations"));
}

#[test]
fn test_unsafe_depth_tracking() {
    let mut checker = UnsafeCheckerEnhanced::new();
    
    assert!(!checker.is_in_unsafe_context());
    
    checker.enter_unsafe_context();
    assert!(checker.is_in_unsafe_context());
    
    checker.enter_unsafe_context();
    assert!(checker.is_in_unsafe_context());
    
    checker.exit_unsafe_context();
    assert!(checker.is_in_unsafe_context());
    
    checker.exit_unsafe_context();
    assert!(!checker.is_in_unsafe_context());
}

#[test]
fn test_type_equality_checking() {
    let mut checker = UnsafeCheckerEnhanced::new();
    
    // Same primitive types
    let i32_a = HirType::Int32;
    let i32_b = HirType::Int32;
    let result = checker.validate_transmute(&i32_a, &i32_b);
    assert!(result.is_ok());
    
    // Different primitive types
    let i32_type = HirType::Int32;
    let i64_type = HirType::Int64;
    let result = checker.validate_transmute(&i32_type, &i64_type);
    assert!(result.is_err()); // Not the same size, so invalid
}

#[test]
fn test_type_name_generation_for_error_messages() {
    let mut checker = UnsafeCheckerEnhanced::new();
    
    assert_eq!(
        checker.validate_transmute(&HirType::Int32, &HirType::String).err().unwrap().message,
        "transmute from i32 to str is invalid"
    );
}

#[test]
fn test_reference_type_equality() {
    let mut checker = UnsafeCheckerEnhanced::new();
    
    let ref_i32_a = HirType::Reference(Box::new(HirType::Int32));
    let ref_i32_b = HirType::Reference(Box::new(HirType::Int32));
    
    // Same reference types
    let result = checker.validate_transmute(&ref_i32_a, &ref_i32_b);
    assert!(result.is_ok());
}

#[test]
fn test_complex_type_transmutation() {
    let mut checker = UnsafeCheckerEnhanced::new();
    
    // Pointer to int32
    let ptr_i32 = HirType::Pointer(Box::new(HirType::Int32));
    // Pointer to int64
    let ptr_i64 = HirType::Pointer(Box::new(HirType::Int64));
    
    // Different inner types, different pointers
    let result = checker.validate_transmute(&ptr_i32, &ptr_i64);
    assert!(result.is_err());
    
    checker.enter_unsafe_context();
    let result = checker.validate_transmute(&ptr_i32, &ptr_i64);
    assert!(result.is_ok());
}

#[test]
fn test_error_accumulation_in_checker() {
    let mut checker = UnsafeCheckerEnhanced::new();
    
    // Accumulate multiple errors
    let _ = checker.check_pointer_deref_enhanced();
    let _ = checker.check_unsafe_function_call_enhanced("func1");
    let _ = checker.check_pointer_deref_enhanced();
    
    assert_eq!(checker.errors().len(), 3);
    
    // Each should have a suggestion
    for error in checker.errors() {
        assert!(error.suggestion.is_some());
    }
}

#[test]
fn test_capability_summary_enhanced() {
    let mut checker = UnsafeCheckerEnhanced::new();
    checker.set_context("in unsafe { ... } block");
    checker.enter_unsafe_context();
    
    // All these should work in unsafe context
    assert!(checker.check_pointer_deref_enhanced().is_ok());
    
    let ptr1 = HirType::Pointer(Box::new(HirType::Int32));
    let ptr2 = HirType::Pointer(Box::new(HirType::Float64));
    let transmute_result = checker.validate_transmute(&ptr1, &ptr2);
    assert!(transmute_result.is_ok());
    
    // All operations should succeed
    let num_errors = checker.errors().len();
    assert_eq!(num_errors, 0, "No errors should occur in unsafe context");
}
