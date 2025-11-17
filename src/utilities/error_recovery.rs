//! # Error Recovery System
//!
//! Advanced compiler error recovery for better diagnostics:
//! - Error context collection
//! - Suggestion generation
//! - Recovery point tracking
//! - Error aggregation
//! - Diagnostic reporting

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CompileError {
    pub code: String,
    pub message: String,
    pub location: ErrorLocation,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ErrorLocation {
    pub line: usize,
    pub column: usize,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub source_code: String,
    pub error_code: String,
    pub surrounding_lines: Vec<String>,
}

pub struct ErrorRecoveryEngine {
    errors: Vec<CompileError>,
    error_contexts: HashMap<String, ErrorContext>,
    recovery_points: Vec<usize>,
}

impl ErrorRecoveryEngine {
    pub fn new() -> Self {
        ErrorRecoveryEngine {
            errors: Vec::new(),
            error_contexts: HashMap::new(),
            recovery_points: Vec::new(),
        }
    }

    pub fn report_error(&mut self, error: CompileError) {
        self.errors.push(error);
    }

    pub fn store_context(&mut self, error_code: String, context: ErrorContext) {
        self.error_contexts.insert(error_code, context);
    }

    pub fn record_recovery_point(&mut self, line: usize) {
        self.recovery_points.push(line);
    }

    pub fn suggest_fix(&mut self, error_code: &str) -> Option<String> {
        match error_code {
            "E0001" => Some("Missing semicolon".to_string()),
            "E0002" => Some("Variable not found in scope".to_string()),
            "E0003" => Some("Type mismatch in assignment".to_string()),
            "E0004" => Some("Function not declared".to_string()),
            "E0005" => Some("Invalid return type".to_string()),
            _ => None,
        }
    }

    pub fn get_error_context(&self, error_code: &str) -> Option<ErrorContext> {
        self.error_contexts.get(error_code).cloned()
    }

    pub fn format_error(&self, error: &CompileError) -> String {
        let mut formatted = format!(
            "{}:{}:{}: error[{}]: {}",
            error.location.file,
            error.location.line,
            error.location.column,
            error.code,
            error.message
        );

        if !error.suggestions.is_empty() {
            formatted.push_str("\n  suggestions:");
            for suggestion in &error.suggestions {
                formatted.push_str(&format!("\n    - {}", suggestion));
            }
        }

        formatted
    }

    pub fn get_all_errors(&self) -> Vec<CompileError> {
        self.errors.clone()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn find_recovery_point(&self, failed_line: usize) -> Option<usize> {
        self.recovery_points.iter()
            .filter(|&&point| point > failed_line)
            .min()
            .copied()
    }

    pub fn get_nearest_context(&self, error_code: &str) -> Option<ErrorContext> {
        self.error_contexts.values()
            .find(|ctx| ctx.error_code == error_code)
            .cloned()
    }

    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    pub fn aggregate_errors(&self) -> String {
        let mut result = String::new();

        for error in &self.errors {
            result.push_str(&self.format_error(error));
            result.push_str("\n\n");
        }

        result
    }

    pub fn get_errors_at_location(&self, line: usize) -> Vec<CompileError> {
        self.errors.iter()
            .filter(|e| e.location.line == line)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_engine() {
        let _engine = ErrorRecoveryEngine::new();
        assert!(true);
    }

    #[test]
    fn test_report_error() {
        let mut engine = ErrorRecoveryEngine::new();
        let error = CompileError {
            code: "E0001".to_string(),
            message: "Missing semicolon".to_string(),
            location: ErrorLocation {
                line: 1,
                column: 5,
                file: "test.rs".to_string(),
            },
            suggestions: vec![],
        };

        engine.report_error(error);
        assert_eq!(engine.error_count(), 1);
    }

    #[test]
    fn test_store_context() {
        let mut engine = ErrorRecoveryEngine::new();
        let context = ErrorContext {
            source_code: "let x = ".to_string(),
            error_code: "E0001".to_string(),
            surrounding_lines: vec!["line1".to_string()],
        };

        engine.store_context("E0001".to_string(), context);
        assert!(engine.get_error_context("E0001").is_some());
    }

    #[test]
    fn test_record_recovery_point() {
        let mut engine = ErrorRecoveryEngine::new();
        engine.record_recovery_point(10);

        assert!(!engine.recovery_points.is_empty());
    }

    #[test]
    fn test_suggest_fix() {
        let mut engine = ErrorRecoveryEngine::new();
        let suggestion = engine.suggest_fix("E0001");

        assert!(suggestion.is_some());
    }

    #[test]
    fn test_format_error() {
        let engine = ErrorRecoveryEngine::new();
        let error = CompileError {
            code: "E0001".to_string(),
            message: "Test error".to_string(),
            location: ErrorLocation {
                line: 1,
                column: 1,
                file: "test.rs".to_string(),
            },
            suggestions: vec!["Fix it".to_string()],
        };

        let formatted = engine.format_error(&error);
        assert!(formatted.contains("test.rs:1:1"));
    }

    #[test]
    fn test_get_all_errors() {
        let mut engine = ErrorRecoveryEngine::new();
        let error = CompileError {
            code: "E0001".to_string(),
            message: "Error".to_string(),
            location: ErrorLocation {
                line: 1,
                column: 1,
                file: "test.rs".to_string(),
            },
            suggestions: vec![],
        };

        engine.report_error(error);
        assert_eq!(engine.get_all_errors().len(), 1);
    }

    #[test]
    fn test_has_errors() {
        let mut engine = ErrorRecoveryEngine::new();
        assert!(!engine.has_errors());

        let error = CompileError {
            code: "E0001".to_string(),
            message: "Error".to_string(),
            location: ErrorLocation {
                line: 1,
                column: 1,
                file: "test.rs".to_string(),
            },
            suggestions: vec![],
        };

        engine.report_error(error);
        assert!(engine.has_errors());
    }

    #[test]
    fn test_find_recovery_point() {
        let mut engine = ErrorRecoveryEngine::new();
        engine.record_recovery_point(10);
        engine.record_recovery_point(20);

        let recovery = engine.find_recovery_point(15);
        assert_eq!(recovery, Some(20));
    }

    #[test]
    fn test_aggregate_errors() {
        let mut engine = ErrorRecoveryEngine::new();
        let error = CompileError {
            code: "E0001".to_string(),
            message: "Error".to_string(),
            location: ErrorLocation {
                line: 1,
                column: 1,
                file: "test.rs".to_string(),
            },
            suggestions: vec![],
        };

        engine.report_error(error);
        let aggregated = engine.aggregate_errors();
        assert!(aggregated.contains("Error"));
    }

    #[test]
    fn test_get_errors_at_location() {
        let mut engine = ErrorRecoveryEngine::new();
        let error = CompileError {
            code: "E0001".to_string(),
            message: "Error".to_string(),
            location: ErrorLocation {
                line: 5,
                column: 1,
                file: "test.rs".to_string(),
            },
            suggestions: vec![],
        };

        engine.report_error(error);
        let errors_at_5 = engine.get_errors_at_location(5);
        assert_eq!(errors_at_5.len(), 1);
    }

    #[test]
    fn test_clear_errors() {
        let mut engine = ErrorRecoveryEngine::new();
        let error = CompileError {
            code: "E0001".to_string(),
            message: "Error".to_string(),
            location: ErrorLocation {
                line: 1,
                column: 1,
                file: "test.rs".to_string(),
            },
            suggestions: vec![],
        };

        engine.report_error(error);
        assert!(engine.has_errors());

        engine.clear_errors();
        assert!(!engine.has_errors());
    }

    #[test]
    fn test_multiple_suggestions() {
        let engine = ErrorRecoveryEngine::new();
        let error = CompileError {
            code: "E0003".to_string(),
            message: "Type mismatch".to_string(),
            location: ErrorLocation {
                line: 1,
                column: 1,
                file: "test.rs".to_string(),
            },
            suggestions: vec![
                "Cast to correct type".to_string(),
                "Check variable declaration".to_string(),
            ],
        };

        let formatted = engine.format_error(&error);
        assert!(formatted.contains("Cast to correct type"));
        assert!(formatted.contains("Check variable declaration"));
    }

    #[test]
    fn test_get_nearest_context() {
        let mut engine = ErrorRecoveryEngine::new();
        let context = ErrorContext {
            source_code: "code".to_string(),
            error_code: "E0001".to_string(),
            surrounding_lines: vec![],
        };

        engine.store_context("E0001".to_string(), context);
        assert!(engine.get_nearest_context("E0001").is_some());
    }
}
