//! Error Suggestion System for Phase 3.2+
//!
//! Provides intelligent suggestions for common type errors, borrow errors, and lifetime errors.
//! Helps developers fix issues without trial-and-error.

use std::fmt;

/// A suggestion for fixing a compiler error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub description: String,
    pub code: String,
    pub confidence: Confidence,
}

/// How confident we are about a suggestion
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Confidence::Low => write!(f, "low confidence"),
            Confidence::Medium => write!(f, "medium confidence"),
            Confidence::High => write!(f, "high confidence"),
        }
    }
}

impl Suggestion {
    pub fn new(description: impl Into<String>, code: impl Into<String>, confidence: Confidence) -> Self {
        Suggestion {
            description: description.into(),
            code: code.into(),
            confidence,
        }
    }
}

/// Type error suggestion engine
pub struct TypeErrorSuggester;

impl TypeErrorSuggester {
    /// Suggest fixes for type mismatches
    pub fn suggest_type_mismatch(expected: &str, found: &str, variable: Option<&str>) -> Vec<Suggestion> {
        let mut suggestions = vec![];

        // String vs &str cases
        if (expected == "String" && found == "&str") || (expected == "String" && found == "str") {
            suggestions.push(Suggestion::new(
                "Convert string slice to owned String",
                format!("let {}: String = \"{}\".to_string();", variable.unwrap_or("x"), "hello"),
                Confidence::High,
            ));
            suggestions.push(Suggestion::new(
                "Use string reference type instead",
                format!("let {}: &str = \"{}\";", variable.unwrap_or("x"), "hello"),
                Confidence::High,
            ));
        }

        if (expected == "&str" && found == "String") {
            suggestions.push(Suggestion::new(
                "Borrow the String as a string slice",
                format!("let {}: &str = &{};", variable.unwrap_or("x"), variable.unwrap_or("s")),
                Confidence::High,
            ));
            suggestions.push(Suggestion::new(
                "Use String type instead",
                format!("let {}: String = {};", variable.unwrap_or("x"), variable.unwrap_or("s")),
                Confidence::Medium,
            ));
        }

        // Integer type conversions
        if (expected == "i32" && found == "i64") || (expected == "i32" && found == "u32") {
            suggestions.push(Suggestion::new(
                "Cast to i32",
                format!("let {}: i32 = {} as i32;", variable.unwrap_or("x"), variable.unwrap_or("y")),
                Confidence::High,
            ));
            suggestions.push(Suggestion::new(
                "Use i64 type instead",
                format!("let {}: i64 = {};", variable.unwrap_or("x"), variable.unwrap_or("y")),
                Confidence::Medium,
            ));
        }

        if expected == "i64" && found == "i32" {
            suggestions.push(Suggestion::new(
                "Automatically coerced (should work)",
                format!("let {}: i64 = {} as i64;", variable.unwrap_or("x"), variable.unwrap_or("y")),
                Confidence::High,
            ));
        }

        // Float vs Integer
        if (expected.contains("f64") || expected.contains("f32")) && (found.contains("i32") || found.contains("i64")) {
            suggestions.push(Suggestion::new(
                "Cast integer to float",
                format!("let {}: f64 = {} as f64;", variable.unwrap_or("x"), variable.unwrap_or("y")),
                Confidence::High,
            ));
            suggestions.push(Suggestion::new(
                "Use integer type instead",
                format!("let {}: i32 = {} as i32;", variable.unwrap_or("x"), variable.unwrap_or("y")),
                Confidence::Medium,
            ));
        }

        // Bool vs Integer
        if expected == "bool" && (found == "i32" || found == "i64") {
            suggestions.push(Suggestion::new(
                "Compare with 0 to get boolean",
                format!("let {}: bool = {} != 0;", variable.unwrap_or("x"), variable.unwrap_or("y")),
                Confidence::High,
            ));
        }

        suggestions
    }

    /// Suggest fixes for borrow errors
    pub fn suggest_borrow_error(error_type: &str, variable: &str) -> Vec<Suggestion> {
        let mut suggestions = vec![];

        if error_type.contains("borrowed") || error_type.contains("used after move") {
            suggestions.push(Suggestion::new(
                "Clone the value to create a copy",
                format!("let {} = {}.clone();", variable, variable),
                Confidence::High,
            ));
            suggestions.push(Suggestion::new(
                "Take a reference instead of moving",
                format!("fn foo(x: &{}) {{ ... }}", variable),
                Confidence::High,
            ));
        }

        if error_type.contains("multiple mutable") {
            suggestions.push(Suggestion::new(
                "Use mutable reference carefully",
                format!("let mut {} = ...;\nlet ref1 = &mut {};\n// use ref1\nlet ref2 = &mut {};\n// use ref2", variable, variable, variable),
                Confidence::High,
            ));
        }

        suggestions
    }

    /// Suggest fixes for lifetime errors
    pub fn suggest_lifetime_error(description: &str) -> Vec<Suggestion> {
        let mut suggestions = vec![];

        if description.contains("lifetime") {
            suggestions.push(Suggestion::new(
                "Make lifetimes explicit",
                "fn foo<'a>(x: &'a str) -> &'a str { x }",
                Confidence::Medium,
            ));
            suggestions.push(Suggestion::new(
                "Return an owned type instead",
                "fn foo(x: &str) -> String { x.to_string() }",
                Confidence::High,
            ));
        }

        suggestions
    }
}

/// Format suggestions into a readable message
pub fn format_suggestions(suggestions: &[Suggestion]) -> String {
    if suggestions.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    output.push_str("\npossible solutions:\n");

    // Sort by confidence (high first)
    let mut sorted = suggestions.to_vec();
    sorted.sort_by(|a, b| b.confidence.cmp(&a.confidence));

    for (i, suggestion) in sorted.iter().enumerate() {
        output.push_str(&format!("  {}. {}\n", i + 1, suggestion.description));
        output.push_str(&format!("     {}\n", suggestion.code));
        if suggestion.confidence != Confidence::High {
            output.push_str(&format!("     ({})\n", suggestion.confidence));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_suggestions() {
        let suggestions = TypeErrorSuggester::suggest_type_mismatch("String", "&str", Some("x"));
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.confidence == Confidence::High));
    }

    #[test]
    fn test_integer_suggestions() {
        let suggestions = TypeErrorSuggester::suggest_type_mismatch("i32", "i64", Some("x"));
        assert!(!suggestions.is_empty());
    }

    #[test]
    fn test_borrow_suggestions() {
        let suggestions = TypeErrorSuggester::suggest_borrow_error("value used after move", "x");
        assert!(!suggestions.is_empty());
    }

    #[test]
    fn test_format_suggestions() {
        let suggestions = vec![
            Suggestion::new("Test 1", "code 1", Confidence::High),
            Suggestion::new("Test 2", "code 2", Confidence::Low),
        ];
        let formatted = format_suggestions(&suggestions);
        assert!(formatted.contains("possible solutions"));
        assert!(formatted.contains("Test 1"));
    }

    #[test]
    fn test_empty_suggestions() {
        let suggestions: Vec<Suggestion> = vec![];
        let formatted = format_suggestions(&suggestions);
        assert_eq!(formatted, "");
    }

    #[test]
    fn test_confidence_ordering() {
        assert!(Confidence::High > Confidence::Medium);
        assert!(Confidence::Medium > Confidence::Low);
    }
}
