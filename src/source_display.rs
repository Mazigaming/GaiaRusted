//! Display source code context for errors (rustc style)
//!
//! Shows the actual source line with error pointers, matching rustc's error format:
//! ```
//! error[E0308]: mismatched types
//!   |
//! 5 | let x: i32 = "hello";
//!   |         ^^^ expected `i32`, found `&str`
//! ```

use std::fs;
use std::path::Path;

/// Display error with source code context
pub struct SourceError {
    pub error_code: String,
    pub message: String,
    pub file_path: String,
    pub line_number: usize,
    pub column_number: usize,
    pub error_length: usize,
    pub expected: String,
    pub found: String,
    pub suggestions: Vec<String>,
}

impl SourceError {
    pub fn new(
        error_code: &str,
        message: &str,
        file_path: &str,
        line_number: usize,
        column_number: usize,
        error_length: usize,
        expected: &str,
        found: &str,
    ) -> Self {
        SourceError {
            error_code: error_code.to_string(),
            message: message.to_string(),
            file_path: file_path.to_string(),
            line_number,
            column_number,
            error_length,
            expected: expected.to_string(),
            found: found.to_string(),
            suggestions: vec![],
        }
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Display error in rustc format with source code
    pub fn display(&self) -> String {
        let mut output = String::new();

        // Header: error[CODE]: message
        output.push_str(&format!(
            "{}error[{}]{}: {}\n",
            crate::formatter::Colors::RED,
            self.error_code,
            crate::formatter::Colors::RESET,
            self.message
        ));

        // Try to read and display source line
        if let Ok(source_line) = self.read_source_line() {
            output.push_str(&self.format_source_context(&source_line));
        }

        // Type mismatch details
        output.push_str(&format!(
            "{}  expected `{}`, found `{}`{}\n",
            crate::formatter::Colors::CYAN,
            self.expected,
            self.found,
            crate::formatter::Colors::RESET
        ));

        // Suggestions
        if !self.suggestions.is_empty() {
            output.push_str("\n");
            output.push_str(&format!(
                "{}help:{} Consider these options:\n",
                crate::formatter::Colors::GREEN,
                crate::formatter::Colors::RESET
            ));
            for (idx, suggestion) in self.suggestions.iter().enumerate() {
                output.push_str(&format!("  {}. {}\n", idx + 1, suggestion));
            }
        }

        output.push('\n');
        output
    }

    /// Read the source line from file
    fn read_source_line(&self) -> Result<String, std::io::Error> {
        let contents = fs::read_to_string(&self.file_path)?;
        let lines: Vec<&str> = contents.lines().collect();

        if self.line_number > 0 && self.line_number <= lines.len() {
            Ok(lines[self.line_number - 1].to_string())
        } else {
            // Try to find the line by searching for a pattern
            // This is a fallback when line number isn't exact
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "line not found",
            ))
        }
    }

    /// Try to find source line by searching for a pattern (variable declaration)
    pub fn find_source_line_by_pattern(&mut self, var_name: &str) -> Result<(), std::io::Error> {
        let contents = fs::read_to_string(&self.file_path)?;
        let lines: Vec<&str> = contents.lines().collect();

        // Search for "let VAR_NAME:" pattern specifically for variable declarations
        // This avoids false matches like "let greeting:" matching in function names
        let patterns = vec![
            format!("let {}: ", var_name),  // with type annotation
            format!("let {} =", var_name),  // without type annotation
            format!("let {}", var_name),    // fallback
        ];

        for pattern in patterns {
            // Search all lines looking for variable declarations
            for (idx, line) in lines.iter().enumerate() {
                // Check if this line has the pattern AND looks like a variable declaration
                if line.contains(&pattern) {
                    // Make sure it's not in a comment or something
                    let trimmed = line.trim_start();
                    if trimmed.starts_with("let ") || trimmed.starts_with("let\t") {
                        self.line_number = idx + 1;
                        // Set column to where "let" starts
                        self.column_number = line.find("let").unwrap_or(0);
                        return Ok(());
                    }
                }
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("variable '{}' not found in source", var_name),
        ))
    }

    /// Format source context with error pointer
    fn format_source_context(&self, source_line: &str) -> String {
        let mut output = String::new();

        // Line numbers and context
        let line_num_width = self.line_number.to_string().len().max(3);
        let padding = " ".repeat(line_num_width);

        output.push_str(&format!("{}  |{}\n", padding, crate::formatter::Colors::DIM));

        // Source code line
        output.push_str(&format!(
            "{}{}|{} {}\n",
            crate::formatter::Colors::DIM,
            self.line_number,
            crate::formatter::Colors::RESET,
            source_line
        ));

        // Error pointer line
        output.push_str(&format!("{}  |{} ", padding, crate::formatter::Colors::DIM));

        // Spacing to error location
        let mut pointer_line = String::new();
        for _ in 0..self.column_number {
            pointer_line.push(' ');
        }

        // Add pointer (^^^)
        for _ in 0..self.error_length.max(1) {
            pointer_line.push('^');
        }

        output.push_str(&format!(
            "{}{}{}",
            crate::formatter::Colors::RED,
            pointer_line,
            crate::formatter::Colors::RESET
        ));
        output.push('\n');

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_error_creation() {
        let error = SourceError::new("E0308", "mismatched types", "test.rs", 5, 10, 3, "i32", "&str");
        assert_eq!(error.error_code, "E0308");
        assert_eq!(error.line_number, 5);
        assert_eq!(error.column_number, 10);
    }

    #[test]
    fn test_source_error_with_suggestions() {
        let error = SourceError::new("E0308", "mismatched types", "test.rs", 5, 10, 3, "i32", "&str")
            .with_suggestion("let x: i32 = \"hello\".parse().unwrap();".to_string())
            .with_suggestion("let x: &str = \"hello\";".to_string());

        assert_eq!(error.suggestions.len(), 2);
    }
}
