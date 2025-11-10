//! Enhanced error reporting with source code context
//!
//! This module provides a comprehensive error reporting system that displays
//! source code context, line numbers, helpful suggestions, and colored output
//! for compilation errors, warnings, and notes.

use std::fmt;
use std::path::PathBuf;

// Note: Color support is now always available in v0.0.3

/// Location of an error in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub byte_pos: usize,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize, byte_pos: usize) -> Self {
        SourceLocation { line, column, byte_pos }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Severity level of an error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Note => write!(f, "note"),
        }
    }
}

/// Detailed diagnostic with source context
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub phase: String,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub file: Option<PathBuf>,
    pub suggestion: Option<String>,
    pub help: Option<String>,
}

impl Diagnostic {
    /// Create a new error diagnostic
    pub fn error(phase: &str, message: &str) -> Self {
        Diagnostic {
            severity: Severity::Error,
            phase: phase.to_string(),
            message: message.to_string(),
            location: None,
            file: None,
            suggestion: None,
            help: None,
        }
    }

    /// Create a new warning diagnostic
    pub fn warning(phase: &str, message: &str) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            phase: phase.to_string(),
            message: message.to_string(),
            location: None,
            file: None,
            suggestion: None,
            help: None,
        }
    }

    /// Set the source location
    pub fn with_location(mut self, loc: SourceLocation) -> Self {
        self.location = Some(loc);
        self
    }

    /// Set the file path
    pub fn with_file(mut self, file: PathBuf) -> Self {
        self.file = Some(file);
        self
    }

    /// Set a suggestion for fixing the error
    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }

    /// Set additional help text
    pub fn with_help(mut self, help: &str) -> Self {
        self.help = Some(help.to_string());
        self
    }

    /// Format diagnostic with source context and colored output (v0.0.3)
    pub fn format_detailed(&self, source: Option<&str>) -> String {
        let mut output = String::new();

        // Header line with color
        let location_str = self.location
            .map(|l| format!(" at {}", l))
            .unwrap_or_default();
        let file_str = self.file.as_ref()
            .map(|f| format!("{}", f.display()))
            .unwrap_or_else(|| "<stdin>".to_string());

        // Color the severity label (v0.0.3 enhancement)
        let severity_label = match self.severity {
            Severity::Error => format!("\x1b[31m{}\x1b[0m", self.severity), // Red for errors
            Severity::Warning => format!("\x1b[33m{}\x1b[0m", self.severity), // Yellow for warnings
            Severity::Note => format!("\x1b[36m{}\x1b[0m", self.severity), // Cyan for notes
        };

        output.push_str(&format!(
            "{}: {}: {}{}\n",
            severity_label, self.phase, self.message, location_str
        ));
        output.push_str(&format!("  --> {}:{}\n", file_str, 
            self.location.map(|l| l.to_string()).unwrap_or_default()));

        // Source context
        if let (Some(source), Some(loc)) = (source, self.location) {
            output.push_str("\n");
            let lines: Vec<&str> = source.lines().collect();
            
            // Display context lines
            let start_line = loc.line.saturating_sub(2).max(1);
            let end_line = (loc.line + 2).min(lines.len());

            for line_num in start_line..=end_line {
                let line_idx = line_num - 1;
                if line_idx < lines.len() {
                    let line = lines[line_idx];
                    let is_error_line = line_num == loc.line;
                    
                    if is_error_line {
                        output.push_str(&format!("  {} | {}\n", 
                            format!("{:4}", line_num), line));
                        // Highlight error location with red caret
                        let caret_line = format!("    | {}\x1b[31m^\x1b[0m", 
                            " ".repeat(loc.column.saturating_sub(1)));
                        output.push_str(&caret_line);
                        output.push_str("\n");
                    } else {
                        output.push_str(&format!("  {} | {}\n", 
                            format!("{:4}", line_num), line));
                    }
                }
            }
            output.push_str("\n");
        }

        // Suggestion with yellow highlight
        if let Some(suggestion) = &self.suggestion {
            output.push_str(&format!("  \x1b[33msuggestion:\x1b[0m {}\n", suggestion));
        }

        // Help text with cyan highlight
        if let Some(help) = &self.help {
            output.push_str(&format!("  \x1b[36mhelp:\x1b[0m {}\n", help));
        }

        output
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let location_str = self.location
            .map(|l| format!(" at {}", l))
            .unwrap_or_default();
        
        write!(f, "{}: {}: {}{}",
            self.severity,
            self.phase,
            self.message,
            location_str
        )
    }
}

impl std::error::Error for Diagnostic {}

/// Error reporter that accumulates and formats diagnostics
pub struct ErrorReporter {
    diagnostics: Vec<Diagnostic>,
    source: Option<String>,
}

impl ErrorReporter {
    /// Create a new error reporter
    pub fn new() -> Self {
        ErrorReporter {
            diagnostics: Vec::new(),
            source: None,
        }
    }

    /// Set the source code for context display
    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    /// Add a diagnostic
    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Add an error
    pub fn error(&mut self, phase: &str, message: &str) -> &mut Diagnostic {
        self.diagnostics.push(Diagnostic::error(phase, message));
        self.diagnostics.last_mut().unwrap()
    }

    /// Add a warning
    pub fn warning(&mut self, phase: &str, message: &str) -> &mut Diagnostic {
        self.diagnostics.push(Diagnostic::warning(phase, message));
        self.diagnostics.last_mut().unwrap()
    }

    /// Get all diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }

    /// Format all diagnostics with summary (v0.0.3 multi-error batching)
    pub fn format_all(&self) -> String {
        let mut output = String::new();
        
        for (idx, diagnostic) in self.diagnostics.iter().enumerate() {
            output.push_str(&diagnostic.format_detailed(self.source.as_deref()));
            if idx < self.diagnostics.len() - 1 {
                output.push_str("\n");
            }
        }

        // Add summary line if there are multiple errors/warnings (v0.0.3 feature)
        if self.diagnostics.len() > 1 {
            let error_count = self.diagnostics.iter()
                .filter(|d| d.severity == Severity::Error).count();
            let warning_count = self.diagnostics.iter()
                .filter(|d| d.severity == Severity::Warning).count();
            
            output.push_str("\n");
            output.push_str("────────────────────────────────────\n");
            
            if error_count > 0 {
                output.push_str(&format!("\x1b[31m✗ {} error{}\x1b[0m", 
                    error_count, if error_count == 1 { "" } else { "s" }));
            }
            
            if error_count > 0 && warning_count > 0 {
                output.push_str(", ");
            }
            
            if warning_count > 0 {
                output.push_str(&format!("\x1b[33m⚠ {} warning{}\x1b[0m", 
                    warning_count, if warning_count == 1 { "" } else { "s" }));
            }
            
            output.push_str("\n");
        }

        output
    }

    /// Print all diagnostics to stderr
    pub fn print_all(&self) {
        eprint!("{}", self.format_all());
    }
}

impl Default for ErrorReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic::error("Parser", "Unexpected token")
            .with_location(SourceLocation::new(5, 12, 0))
            .with_suggestion("Did you mean `;`?");
        
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.location.unwrap().line, 5);
    }

    #[test]
    fn test_error_reporter() {
        let mut reporter = ErrorReporter::new();
        reporter.error("Lexer", "Invalid character");
        reporter.warning("Parser", "Unused variable");
        
        assert!(reporter.has_errors());
        assert_eq!(reporter.diagnostics().len(), 2);
    }
}