//! Comprehensive error reporting test suite
//! Tests error codes, messages, suggestions for all error phases
//! Phase 10: Error Reporting Enhancement

#[cfg(test)]
mod error_reporting_tests {
    use crate::utilities::ErrorCode;
    use crate::utilities::Span;
    use crate::typechecker::TypeCheckError;
    use crate::borrowchecker::BorrowCheckError;



    // ============================================================================
    // TYPE CHECKER ERROR TESTS
    // ============================================================================

    #[test]
    fn test_type_mismatch_error_code() {
        let error = TypeCheckError::type_mismatch("i32", "f64");
        
        assert_eq!(error.code, Some(ErrorCode::E0308));
        assert!(error.message.contains("Type mismatch"));
        assert_eq!(error.expected_type, Some("i32".to_string()));
        assert_eq!(error.found_type, Some("f64".to_string()));
    }

    #[test]
    fn test_type_mismatch_has_suggestion() {
        let error = TypeCheckError::type_mismatch("i32", "String");
        
        assert!(error.suggestion.is_none());
        
        // With suggestion builder
        let error_with_suggestion = error.with_suggestion("consider converting String to i32");
        assert!(error_with_suggestion.suggestion.is_some());
        assert!(error_with_suggestion.suggestion.unwrap().contains("converting"));
    }

    #[test]
    fn test_undefined_variable_error_code() {
        let error = TypeCheckError::undefined_variable("unknown_var");
        
        assert_eq!(error.code, Some(ErrorCode::E0425));
        assert!(error.message.contains("Undefined variable"));
        assert!(error.suggestion.is_some());
        assert!(error.suggestion.unwrap().contains("not defined"));
    }

    #[test]
    fn test_type_error_with_span() {
        let span = Span::new(10, 15, 100, 10);
        
        let error = TypeCheckError::type_mismatch("i32", "i64")
            .with_span(span);
        
        assert!(error.span.is_some());
        let s = error.span.unwrap();
        assert_eq!(s.line, 10);
        assert_eq!(s.column, 15);
    }

    #[test]
    fn test_type_error_generic_creation() {
        let error = TypeCheckError::new("Custom type error");
        
        assert_eq!(error.message, "Custom type error");
        assert_eq!(error.code, None);
        assert_eq!(error.span, None);
    }

    #[test]
    fn test_type_error_from_message() {
        let error = TypeCheckError::from_message("Some error message".to_string());
        
        assert_eq!(error.message, "Some error message");
        assert_eq!(error.code, Some(ErrorCode::E0001)); // Generic code
        assert!(error.suggestion.is_none());
    }

    #[test]
    fn test_type_error_builder_pattern() {
        let span = Span::new(5, 10, 50, 10);
        let error = TypeCheckError::new("Base error")
            .with_code(ErrorCode::E0308)
            .with_suggestion("Fix this by doing X")
            .with_span(span);
        
        assert_eq!(error.code, Some(ErrorCode::E0308));
        assert!(error.suggestion.is_some());
        assert!(error.span.is_some());
    }

    // ============================================================================
    // BORROW CHECKER ERROR TESTS
    // ============================================================================

    #[test]
    fn test_value_moved_error() {
        let error = BorrowCheckError::value_moved("x");
        
        assert_eq!(error.code, Some(ErrorCode::E0382));
        assert!(error.message.contains("value used after move"));
        assert!(error.message.contains("x"));
        assert!(error.suggestion.is_some());
        assert!(error.suggestion.unwrap().contains("moved"));
    }

    #[test]
    fn test_cannot_move_borrowed_error() {
        let error = BorrowCheckError::cannot_move_borrowed("my_var");
        
        assert_eq!(error.code, Some(ErrorCode::E0505));
        assert!(error.message.contains("cannot move out while borrowed"));
        assert!(error.message.contains("my_var"));
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_multiple_mutable_borrows_error() {
        let error = BorrowCheckError::multiple_mutable_borrows("data");
        
        assert_eq!(error.code, Some(ErrorCode::E0499));
        assert!(error.message.contains("cannot create multiple mutable borrows"));
        assert!(error.suggestion.is_some());
        assert!(error.suggestion.unwrap().contains("drop the first"));
    }

    #[test]
    fn test_mutable_immutable_conflict_error() {
        let error = BorrowCheckError::mutable_borrow_immutable_exists("item");
        
        assert_eq!(error.code, Some(ErrorCode::E0502));
        assert!(error.message.contains("cannot mutably borrow"));
        assert!(error.message.contains("borrowed immutably"));
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_cannot_borrow_moved_error() {
        let error = BorrowCheckError::cannot_borrow_moved("obj");
        
        assert_eq!(error.code, Some(ErrorCode::E0382));
        assert!(error.message.contains("cannot borrow moved"));
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_borrow_undefined_variable_error() {
        let error = BorrowCheckError::undefined_variable("undefined");
        
        assert_eq!(error.code, Some(ErrorCode::E0425));
        assert!(error.message.contains("undefined variable"));
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_cannot_mutate_immutable_error() {
        let error = BorrowCheckError::cannot_mutate_immutable("read_only");
        
        assert_eq!(error.code, Some(ErrorCode::E0017));
        assert!(error.message.contains("cannot mutably borrow immutable"));
        assert!(error.suggestion.is_some());
        assert!(error.suggestion.unwrap().contains("mut"));
    }

    #[test]
    fn test_borrow_error_with_span() {
        let span = Span::new(20, 8, 200, 10);
        
        let error = BorrowCheckError::value_moved("value")
            .with_span(span);
        
        assert!(error.span.is_some());
        let s = error.span.unwrap();
        assert_eq!(s.line, 20);
        assert_eq!(s.column, 8);
    }

    #[test]
    fn test_borrow_error_with_custom_suggestion() {
        let error = BorrowCheckError::value_moved("x")
            .with_suggestion("You need to avoid using x after the move");
        
        assert!(error.suggestion.is_some());
        let suggestion = error.suggestion.unwrap();
        assert!(suggestion.contains("You need to avoid"));
    }

    #[test]
    fn test_borrow_error_generic_creation() {
        let error = BorrowCheckError::new("Custom borrow error");
        
        assert_eq!(error.message, "Custom borrow error");
        assert_eq!(error.code, None);
        assert_eq!(error.span, None);
    }

    #[test]
    fn test_borrow_error_with_code_override() {
        let error = BorrowCheckError::new("Generic message")
            .with_code(ErrorCode::E0382);
        
        assert_eq!(error.code, Some(ErrorCode::E0382));
    }

    // ============================================================================
    // ERROR CODE ENUM TESTS
    // ============================================================================

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::E0308.as_str(), "E0308");
        assert_eq!(ErrorCode::E0425.as_str(), "E0425");
        assert_eq!(ErrorCode::E0382.as_str(), "E0382");
        assert_eq!(ErrorCode::E0499.as_str(), "E0499");
        assert_eq!(ErrorCode::E0502.as_str(), "E0502");
        assert_eq!(ErrorCode::E0505.as_str(), "E0505");
        assert_eq!(ErrorCode::E0017.as_str(), "E0017");
    }

    #[test]
    fn test_error_code_description() {
        assert!(ErrorCode::E0308.description().contains("mismatch"));
        assert!(ErrorCode::E0425.description().contains("cannot find"));
        assert!(ErrorCode::E0382.description().contains("used after move"));
        assert!(ErrorCode::E0499.description().contains("multiple"));
    }

    #[test]
    fn test_error_code_equality() {
        assert_eq!(ErrorCode::E0308, ErrorCode::E0308);
        assert_ne!(ErrorCode::E0308, ErrorCode::E0425);
    }

    // ============================================================================
    // CROSS-PHASE ERROR CONSISTENCY TESTS
    // ============================================================================

    #[test]
    fn test_all_errors_have_codes() {
        let type_error = TypeCheckError::type_mismatch("i32", "f64");
        let borrow_error = BorrowCheckError::value_moved("x");
        
        assert!(type_error.code.is_some());
        assert!(borrow_error.code.is_some());
    }

    #[test]
    fn test_all_errors_are_displayable() {
        let type_error = TypeCheckError::type_mismatch("i32", "f64");
        let borrow_error = BorrowCheckError::value_moved("x");
        
        let type_str = format!("{}", type_error);
        let borrow_str = format!("{}", borrow_error);
        
        assert!(!type_str.is_empty());
        assert!(!borrow_str.is_empty());
    }

    #[test]
    fn test_error_builder_chaining() {
        let span = Span::new(1, 1, 0, 10);
        
        let error = TypeCheckError::new("Test error")
            .with_code(ErrorCode::E0308)
            .with_suggestion("Try this fix")
            .with_span(span);
        
        assert!(error.code.is_some());
        assert!(error.suggestion.is_some());
        assert!(error.span.is_some());
    }

    #[test]
    fn test_error_formatting_consistency() {
        let type_errors = vec![
            TypeCheckError::type_mismatch("i32", "f64"),
            TypeCheckError::undefined_variable("x"),
        ];
        
        let borrow_errors = vec![
            BorrowCheckError::value_moved("y"),
            BorrowCheckError::multiple_mutable_borrows("z"),
        ];
        
        for error in type_errors {
            assert!(!error.message.is_empty());
            assert!(error.code.is_some());
        }
        
        for error in borrow_errors {
            assert!(!error.message.is_empty());
            assert!(error.code.is_some());
        }
    }

    // ============================================================================
    // EDGE CASE TESTS
    // ============================================================================

    #[test]
    fn test_error_with_empty_variable_name() {
        let error = BorrowCheckError::value_moved("");
        
        assert!(error.message.contains("value used after move"));
        assert!(error.code == Some(ErrorCode::E0382));
    }

    #[test]
    fn test_error_with_special_characters() {
        let error = BorrowCheckError::value_moved("var_with_123_numbers");
        
        assert!(error.message.contains("var_with_123_numbers"));
    }

    #[test]
    fn test_error_suggestion_override() {
        let mut error = BorrowCheckError::value_moved("x");
        let original_suggestion = error.suggestion.clone();
        
        error = error.with_suggestion("New custom suggestion");
        
        assert_ne!(error.suggestion, original_suggestion);
        assert_eq!(error.suggestion, Some("New custom suggestion".to_string()));
    }

    #[test]
    fn test_error_code_extraction() {
        let error = TypeCheckError::type_mismatch("A", "B");
        
        if let Some(code) = error.code {
            assert_eq!(code, ErrorCode::E0308);
        } else {
            panic!("Error should have a code");
        }
    }

    // ============================================================================
    // DISPLAY AND DEBUG TESTS
    // ============================================================================

    #[test]
    fn test_error_debug_output() {
        let error = TypeCheckError::type_mismatch("i32", "f64");
        let debug_str = format!("{:?}", error);
        
        assert!(debug_str.contains("TypeCheckError"));
    }

    #[test]
    fn test_borrow_error_debug_output() {
        let error = BorrowCheckError::value_moved("x");
        let debug_str = format!("{:?}", error);
        
        assert!(debug_str.contains("BorrowCheckError"));
    }

    #[test]
    fn test_error_code_debug() {
        let code = ErrorCode::E0308;
        let debug_str = format!("{:?}", code);
        
        assert!(debug_str.contains("E0308"));
    }
}
