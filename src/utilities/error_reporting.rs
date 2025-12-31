//! Enhanced error reporting with source code context
//!
//! This module provides a comprehensive error reporting system that displays
//! source code context, line numbers, helpful suggestions, and colored output
//! for compilation errors, warnings, and notes. Includes detailed error
//! categorization and multi-phase error accumulation.

use std::fmt;
use std::path::PathBuf;
use std::collections::HashMap;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Note,
    Warning,
    Error,
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

/// Error category for better classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    SyntaxError,
    TypeMismatch,
    BorrowingViolation,
    UndefinedSymbol,
    TraitResolution,
    PatternMatching,
    LifetimeViolation,
    UnknownMethod,
    InvalidArgument,
    CompilerLimitation,
    Other,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            ErrorCategory::SyntaxError => "syntax error",
            ErrorCategory::TypeMismatch => "type mismatch",
            ErrorCategory::BorrowingViolation => "borrowing violation",
            ErrorCategory::UndefinedSymbol => "undefined symbol",
            ErrorCategory::TraitResolution => "trait resolution",
            ErrorCategory::PatternMatching => "pattern matching",
            ErrorCategory::LifetimeViolation => "lifetime violation",
            ErrorCategory::UnknownMethod => "unknown method",
            ErrorCategory::InvalidArgument => "invalid argument",
            ErrorCategory::CompilerLimitation => "compiler limitation",
            ErrorCategory::Other => "other",
        };
        write!(f, "{}", s)
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
    pub category: ErrorCategory,
    pub context: Option<String>,
    pub related_items: Vec<String>,
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
            category: ErrorCategory::Other,
            context: None,
            related_items: Vec::new(),
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
            category: ErrorCategory::Other,
            context: None,
            related_items: Vec::new(),
        }
    }

    /// Create a new note diagnostic
    pub fn note(phase: &str, message: &str) -> Self {
        Diagnostic {
            severity: Severity::Note,
            phase: phase.to_string(),
            message: message.to_string(),
            location: None,
            file: None,
            suggestion: None,
            help: None,
            category: ErrorCategory::Other,
            context: None,
            related_items: Vec::new(),
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

    /// Set the error category
    pub fn with_category(mut self, category: ErrorCategory) -> Self {
        self.category = category;
        self
    }

    /// Set context information
    pub fn with_context(mut self, context: &str) -> Self {
        self.context = Some(context.to_string());
        self
    }

    /// Add a related item
    pub fn add_related(mut self, item: String) -> Self {
        self.related_items.push(item);
        self
    }

    /// Format diagnostic with source context and colored output (v0.0.3+)
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
            Severity::Error => format!("\x1b[31m{}\x1b[0m", self.severity),
            Severity::Warning => format!("\x1b[33m{}\x1b[0m", self.severity),
            Severity::Note => format!("\x1b[36m{}\x1b[0m", self.severity),
        };

        // Include category in header
        let category_str = if self.category != ErrorCategory::Other {
            format!(" [{}]", self.category)
        } else {
            String::new()
        };

        output.push_str(&format!(
            "{}: {}: {}{}{}\n",
            severity_label, self.phase, self.message, location_str, category_str
        ));
        output.push_str(&format!("  --> {}:{}\n", file_str, 
            self.location.map(|l| l.to_string()).unwrap_or_default()));

        // Source context
        if let (Some(source), Some(loc)) = (source, self.location) {
            output.push('\n');
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
                        output.push('\n');
                    } else {
                        output.push_str(&format!("  {} | {}\n", 
                            format!("{:4}", line_num), line));
                    }
                }
            }
            output.push('\n');
        }

        // Context information
        if let Some(context) = &self.context {
            output.push_str(&format!("  \x1b[35mcontext:\x1b[0m {}\n", context));
        }

        // Suggestion with yellow highlight
        if let Some(suggestion) = &self.suggestion {
            output.push_str(&format!("  \x1b[33msuggestion:\x1b[0m {}\n", suggestion));
        }

        // Help text with cyan highlight
        if let Some(help) = &self.help {
            output.push_str(&format!("  \x1b[36mhelp:\x1b[0m {}\n", help));
        }

        // Related items
        if !self.related_items.is_empty() {
            output.push_str("  \x1b[36mrelated items:\x1b[0m\n");
            for item in &self.related_items {
                output.push_str(&format!("    - {}\n", item));
            }
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

    /// Add a note
    pub fn note(&mut self, phase: &str, message: &str) -> &mut Diagnostic {
        self.diagnostics.push(Diagnostic::note(phase, message));
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

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Error).count()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Warning).count()
    }

    /// Group diagnostics by phase
    pub fn diagnostics_by_phase(&self) -> HashMap<String, Vec<&Diagnostic>> {
        let mut grouped = HashMap::new();
        for diag in &self.diagnostics {
            grouped.entry(diag.phase.clone())
                .or_insert_with(Vec::new)
                .push(diag);
        }
        grouped
    }

    /// Group diagnostics by category
    pub fn diagnostics_by_category(&self) -> HashMap<ErrorCategory, Vec<&Diagnostic>> {
        let mut grouped = HashMap::new();
        for diag in &self.diagnostics {
            grouped.entry(diag.category)
                .or_insert_with(Vec::new)
                .push(diag);
        }
        grouped
    }

    /// Format all diagnostics with summary (v0.0.3+ multi-error batching)
    pub fn format_all(&self) -> String {
        let mut output = String::new();
        
        if self.diagnostics.is_empty() {
            return output;
        }

        // Sort diagnostics by severity (errors first, then warnings, then notes)
        let mut sorted = self.diagnostics.clone();
        sorted.sort_by_key(|d| std::cmp::Reverse(d.severity));
        
        // Group by phase for better organization
        let mut by_phase: HashMap<String, Vec<&Diagnostic>> = HashMap::new();
        for diag in &sorted {
            by_phase.entry(diag.phase.clone())
                .or_insert_with(Vec::new)
                .push(diag);
        }

        // Output grouped diagnostics
        let mut phases: Vec<_> = by_phase.keys().collect();
        phases.sort();
        
        for phase in phases {
            if let Some(diags) = by_phase.get(phase) {
                output.push_str(&format!("\n\x1b[1m[{}]\x1b[0m\n", phase));
                for (idx, diagnostic) in diags.iter().enumerate() {
                    output.push_str(&diagnostic.format_detailed(self.source.as_deref()));
                    if idx < diags.len() - 1 {
                        output.push('\n');
                    }
                }
            }
        }

        // Add comprehensive summary
        let error_count = self.error_count();
        let warning_count = self.warning_count();
        let note_count = self.diagnostics.iter()
            .filter(|d| d.severity == Severity::Note).count();
        
        output.push('\n');
        output.push_str("═══════════════════════════════════════════════\n");
        
        let mut summary_parts = Vec::new();
        
        if error_count > 0 {
            summary_parts.push(format!("\x1b[31m✗ {} error{}\x1b[0m", 
                error_count, if error_count == 1 { "" } else { "s" }));
        }
        
        if warning_count > 0 {
            summary_parts.push(format!("\x1b[33m⚠ {} warning{}\x1b[0m", 
                warning_count, if warning_count == 1 { "" } else { "s" }));
        }
        
        if note_count > 0 {
            summary_parts.push(format!("\x1b[36mℹ {} note{}\x1b[0m", 
                note_count, if note_count == 1 { "" } else { "s" }));
        }
        
        output.push_str(&summary_parts.join(", "));
        output.push('\n');

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