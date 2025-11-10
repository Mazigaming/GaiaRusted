//! Tests for error reporting system

#[cfg(test)]
mod error_reporting_tests {
    use gaiarusted::error_reporting::{Diagnostic, Severity, SourceLocation, ErrorReporter};
    use std::path::PathBuf;

    #[test]
    fn test_source_location() {
        let loc = SourceLocation::new(5, 12, 50);
        assert_eq!(loc.line, 5);
        assert_eq!(loc.column, 12);
        assert_eq!(loc.to_string(), "5:12");
    }

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic::error("Parser", "Unexpected token");
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.phase, "Parser");
        assert_eq!(diag.message, "Unexpected token");
    }

    #[test]
    fn test_diagnostic_builder() {
        let loc = SourceLocation::new(3, 8, 0);
        let diag = Diagnostic::error("Lexer", "Invalid character")
            .with_location(loc)
            .with_file(PathBuf::from("test.rs"))
            .with_suggestion("Remove invalid character")
            .with_help("Valid characters are: a-z, 0-9, _");

        assert_eq!(diag.location.unwrap().line, 3);
        assert!(diag.file.is_some());
        assert!(diag.suggestion.is_some());
        assert!(diag.help.is_some());
    }

    #[test]
    fn test_warning_creation() {
        let warn = Diagnostic::warning("Typechecker", "Unused variable");
        assert_eq!(warn.severity, Severity::Warning);
    }

    #[test]
    fn test_error_reporter() {
        let mut reporter = ErrorReporter::new();
        reporter.error("Lexer", "Invalid syntax");
        reporter.warning("Parser", "Unused variable");

        assert!(reporter.has_errors());
        assert_eq!(reporter.diagnostics().len(), 2);
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = Diagnostic::error("Parser", "Expected semicolon")
            .with_location(SourceLocation::new(10, 5, 0));
        
        let display = diag.to_string();
        assert!(display.contains("error"));
        assert!(display.contains("Parser"));
        assert!(display.contains("Expected semicolon"));
    }

    #[test]
    fn test_diagnostic_with_source_context() {
        let source = "let x = 42\nlet y = x + 1;";
        let loc = SourceLocation::new(1, 10, 10);
        let diag = Diagnostic::error("Parser", "Missing semicolon")
            .with_location(loc);
        
        let formatted = diag.format_detailed(Some(source));
        assert!(formatted.contains("let x = 42"));
        assert!(formatted.contains("^"));
    }

    #[test]
    fn test_error_reporter_formatting() {
        let source = "fn main() {\n    let x = 42\n}".to_string();
        let mut reporter = ErrorReporter::new()
            .with_source(source);
        
        {
            let diag = reporter.error("Parser", "Expected semicolon");
            diag.location = Some(SourceLocation::new(2, 18, 20));
        }

        let formatted = reporter.format_all();
        assert!(!formatted.is_empty());
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Note.to_string(), "note");
    }
}