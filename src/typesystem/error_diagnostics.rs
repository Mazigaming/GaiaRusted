//! # Error Diagnostics System
//!
//! Comprehensive error reporting and diagnostics for the type system.
//!
//! Provides detailed error messages with:
//! - Source location tracking
//! - Error codes for precise identification
//! - Suggested fixes and hints
//! - Error grouping and deduplication
//! - Severity levels

use std::collections::HashMap;

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Informational message
    Info,
    /// Warning (compilation continues)
    Warning,
    /// Error (compilation fails)
    Error,
    /// Fatal error (stops immediately)
    Fatal,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
            Severity::Fatal => write!(f, "fatal"),
        }
    }
}

/// Error code for precise identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // Type mismatch errors
    E001,
    E002,
    E003,
    // Lifetime errors
    E101,
    E102,
    E103,
    // Generic errors
    E201,
    E202,
    E203,
    // Pattern errors
    E301,
    E302,
    E303,
    // Enum errors
    E401,
    E402,
    E403,
    // Trait errors
    E501,
    E502,
    E503,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::E001 => write!(f, "E001"),
            ErrorCode::E002 => write!(f, "E002"),
            ErrorCode::E003 => write!(f, "E003"),
            ErrorCode::E101 => write!(f, "E101"),
            ErrorCode::E102 => write!(f, "E102"),
            ErrorCode::E103 => write!(f, "E103"),
            ErrorCode::E201 => write!(f, "E201"),
            ErrorCode::E202 => write!(f, "E202"),
            ErrorCode::E203 => write!(f, "E203"),
            ErrorCode::E301 => write!(f, "E301"),
            ErrorCode::E302 => write!(f, "E302"),
            ErrorCode::E303 => write!(f, "E303"),
            ErrorCode::E401 => write!(f, "E401"),
            ErrorCode::E402 => write!(f, "E402"),
            ErrorCode::E403 => write!(f, "E403"),
            ErrorCode::E501 => write!(f, "E501"),
            ErrorCode::E502 => write!(f, "E502"),
            ErrorCode::E503 => write!(f, "E503"),
        }
    }
}

/// Source location information
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub fn new(file: String, line: usize, column: usize) -> Self {
        SourceLocation { file, line, column }
    }
}

/// Diagnostic message
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: ErrorCode,
    pub severity: Severity,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub hint: Option<String>,
    pub fix_suggestion: Option<String>,
}

impl Diagnostic {
    pub fn new(code: ErrorCode, severity: Severity, message: String) -> Self {
        Diagnostic {
            code,
            severity,
            message,
            location: None,
            hint: None,
            fix_suggestion: None,
        }
    }

    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_hint(mut self, hint: String) -> Self {
        self.hint = Some(hint);
        self
    }

    pub fn with_fix(mut self, fix: String) -> Self {
        self.fix_suggestion = Some(fix);
        self
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(loc) = &self.location {
            write!(
                f,
                "{}:{}:{}: {}: {}: {}",
                loc.file, loc.line, loc.column, self.severity, self.code, self.message
            )?;
        } else {
            write!(f, "{}: {}: {}", self.severity, self.code, self.message)?;
        }

        if let Some(hint) = &self.hint {
            write!(f, "\n  hint: {}", hint)?;
        }

        if let Some(fix) = &self.fix_suggestion {
            write!(f, "\n  fix: {}", fix)?;
        }

        Ok(())
    }
}

/// Configuration for error diagnostics
#[derive(Debug, Clone)]
pub struct DiagnosticConfig {
    /// Maximum errors to report before stopping
    pub max_errors: usize,
    /// Whether to deduplicate similar errors
    pub deduplicate: bool,
    /// Whether to include hints
    pub include_hints: bool,
    /// Whether to include fix suggestions
    pub include_fixes: bool,
    /// Minimum severity level to report
    pub min_severity: Severity,
}

impl Default for DiagnosticConfig {
    fn default() -> Self {
        DiagnosticConfig {
            max_errors: 100,
            deduplicate: true,
            include_hints: true,
            include_fixes: true,
            min_severity: Severity::Warning,
        }
    }
}

/// Error diagnostics analyzer
pub struct DiagnosticsEngine {
    config: DiagnosticConfig,
    diagnostics: Vec<Diagnostic>,
    dedup_map: HashMap<String, usize>,
}

impl DiagnosticsEngine {
    pub fn new(config: DiagnosticConfig) -> Self {
        DiagnosticsEngine {
            config,
            diagnostics: Vec::new(),
            dedup_map: HashMap::new(),
        }
    }

    /// Add a diagnostic
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        if diagnostic.severity < self.config.min_severity {
            return;
        }

        if self.config.deduplicate {
            let key = format!("{}:{}", diagnostic.code, diagnostic.message);
            if self.dedup_map.contains_key(&key) {
                return;
            }
            self.dedup_map.insert(key, self.diagnostics.len());
        }

        if self.diagnostics.len() < self.config.max_errors {
            self.diagnostics.push(diagnostic);
        }
    }

    /// Add type mismatch error
    pub fn add_type_mismatch(&mut self, expected: &str, found: &str, location: Option<SourceLocation>) {
        let mut diag = Diagnostic::new(
            ErrorCode::E001,
            Severity::Error,
            format!("type mismatch: expected `{}`, found `{}`", expected, found),
        );

        if let Some(loc) = location {
            diag = diag.with_location(loc);
        }

        diag = diag.with_hint(format!(
            "the expected type `{}` is different from the found type `{}`",
            expected, found
        ));

        self.add_diagnostic(diag);
    }

    /// Add unknown variable error
    pub fn add_unknown_variable(&mut self, var_name: &str, location: Option<SourceLocation>) {
        let mut diag = Diagnostic::new(
            ErrorCode::E002,
            Severity::Error,
            format!("unknown variable: `{}`", var_name),
        );

        if let Some(loc) = location {
            diag = diag.with_location(loc);
        }

        diag = diag.with_hint(format!(
            "variable `{}` is not defined in this scope",
            var_name
        ));

        self.add_diagnostic(diag);
    }

    /// Add lifetime error
    pub fn add_lifetime_mismatch(&mut self, lifetime1: &str, lifetime2: &str, location: Option<SourceLocation>) {
        let mut diag = Diagnostic::new(
            ErrorCode::E101,
            Severity::Error,
            format!("lifetime mismatch: `{}` vs `{}`", lifetime1, lifetime2),
        );

        if let Some(loc) = location {
            diag = diag.with_location(loc);
        }

        diag = diag.with_hint("lifetimes must match for correct borrowing".to_string());

        self.add_diagnostic(diag);
    }

    /// Add generic constraint error
    pub fn add_generic_constraint_unmet(&mut self, param: &str, bound: &str, location: Option<SourceLocation>) {
        let mut diag = Diagnostic::new(
            ErrorCode::E201,
            Severity::Error,
            format!("generic parameter `{}` does not satisfy bound `{}`", param, bound),
        );

        if let Some(loc) = location {
            diag = diag.with_location(loc);
        }

        diag = diag.with_fix(format!("ensure that `{}` implements `{}`", param, bound));

        self.add_diagnostic(diag);
    }

    /// Add pattern exhaustiveness error
    pub fn add_pattern_not_exhaustive(&mut self, patterns: Vec<String>, location: Option<SourceLocation>) {
        let mut diag = Diagnostic::new(
            ErrorCode::E301,
            Severity::Error,
            format!("pattern matching is not exhaustive, missing: {}", patterns.join(", ")),
        );

        if let Some(loc) = location {
            diag = diag.with_location(loc);
        }

        diag = diag.with_fix("add missing patterns or use a wildcard pattern `_`".to_string());

        self.add_diagnostic(diag);
    }

    /// Add enum variant error
    pub fn add_enum_variant_not_found(&mut self, enum_name: &str, variant: &str, location: Option<SourceLocation>) {
        let mut diag = Diagnostic::new(
            ErrorCode::E401,
            Severity::Error,
            format!("`{}` has no variant named `{}`", enum_name, variant),
        );

        if let Some(loc) = location {
            diag = diag.with_location(loc);
        }

        diag = diag.with_hint(format!(
            "check the spelling of the variant name or ensure `{}` is defined",
            variant
        ));

        self.add_diagnostic(diag);
    }

    /// Get all diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Get diagnostics by severity
    pub fn diagnostics_by_severity(&self, severity: Severity) -> Vec<&Diagnostic> {
        self.diagnostics.iter().filter(|d| d.severity == severity).collect()
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error || d.severity == Severity::Fatal)
    }

    /// Count diagnostics by severity
    pub fn count_by_severity(&self) -> HashMap<Severity, usize> {
        let mut counts = HashMap::new();
        for diag in &self.diagnostics {
            *counts.entry(diag.severity).or_insert(0) += 1;
        }
        counts
    }

    /// Generate a formatted report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Diagnostic Report ===\n");

        for diag in &self.diagnostics {
            report.push_str(&format!("{}\n", diag));
        }

        let counts = self.count_by_severity();
        report.push_str("\n=== Summary ===\n");
        if let Some(count) = counts.get(&Severity::Fatal) {
            report.push_str(&format!("{} fatal\n", count));
        }
        if let Some(count) = counts.get(&Severity::Error) {
            report.push_str(&format!("{} errors\n", count));
        }
        if let Some(count) = counts.get(&Severity::Warning) {
            report.push_str(&format!("{} warnings\n", count));
        }
        if let Some(count) = counts.get(&Severity::Info) {
            report.push_str(&format!("{} notes\n", count));
        }

        report
    }

    /// Clear all diagnostics
    pub fn clear(&mut self) {
        self.diagnostics.clear();
        self.dedup_map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_diagnostic() {
        let diag = Diagnostic::new(ErrorCode::E001, Severity::Error, "test error".to_string());
        assert_eq!(diag.code, ErrorCode::E001);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.message, "test error");
    }

    #[test]
    fn test_diagnostic_with_location() {
        let loc = SourceLocation::new("main.rs".to_string(), 10, 5);
        let diag = Diagnostic::new(ErrorCode::E001, Severity::Error, "test".to_string())
            .with_location(loc.clone());

        assert!(diag.location.is_some());
        assert_eq!(diag.location.unwrap().line, 10);
    }

    #[test]
    fn test_diagnostics_engine_add() {
        let config = DiagnosticConfig::default();
        let mut engine = DiagnosticsEngine::new(config);

        let diag = Diagnostic::new(ErrorCode::E001, Severity::Error, "test".to_string());
        engine.add_diagnostic(diag);

        assert_eq!(engine.diagnostics().len(), 1);
    }

    #[test]
    fn test_diagnostics_engine_dedup() {
        let config = DiagnosticConfig {
            deduplicate: true,
            ..Default::default()
        };
        let mut engine = DiagnosticsEngine::new(config);

        let diag1 = Diagnostic::new(ErrorCode::E001, Severity::Error, "same error".to_string());
        let diag2 = Diagnostic::new(ErrorCode::E001, Severity::Error, "same error".to_string());

        engine.add_diagnostic(diag1);
        engine.add_diagnostic(diag2);

        assert_eq!(engine.diagnostics().len(), 1);
    }

    #[test]
    fn test_diagnostics_engine_max_errors() {
        let config = DiagnosticConfig {
            max_errors: 2,
            ..Default::default()
        };
        let mut engine = DiagnosticsEngine::new(config);

        for i in 0..5 {
            let diag = Diagnostic::new(
                ErrorCode::E001,
                Severity::Error,
                format!("error {}", i),
            );
            engine.add_diagnostic(diag);
        }

        assert_eq!(engine.diagnostics().len(), 2);
    }

    #[test]
    fn test_add_type_mismatch() {
        let config = DiagnosticConfig::default();
        let mut engine = DiagnosticsEngine::new(config);

        engine.add_type_mismatch("i32", "str", None);
        assert!(engine.has_errors());
        assert_eq!(engine.diagnostics().len(), 1);
    }

    #[test]
    fn test_count_by_severity() {
        let config = DiagnosticConfig::default();
        let mut engine = DiagnosticsEngine::new(config);

        engine.add_diagnostic(Diagnostic::new(
            ErrorCode::E001,
            Severity::Error,
            "error".to_string(),
        ));
        engine.add_diagnostic(Diagnostic::new(
            ErrorCode::E001,
            Severity::Warning,
            "warning".to_string(),
        ));

        let counts = engine.count_by_severity();
        assert_eq!(counts.get(&Severity::Error), Some(&1));
        assert_eq!(counts.get(&Severity::Warning), Some(&1));
    }

    #[test]
    fn test_generate_report() {
        let config = DiagnosticConfig::default();
        let mut engine = DiagnosticsEngine::new(config);

        engine.add_type_mismatch("i32", "str", None);
        let report = engine.generate_report();

        assert!(report.contains("Diagnostic Report"));
        assert!(report.contains("Summary"));
        assert!(report.contains("error"));
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = Diagnostic::new(ErrorCode::E001, Severity::Error, "test error".to_string());
        let display = format!("{}", diag);
        assert!(display.contains("error"));
        assert!(display.contains("E001"));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Fatal);
    }
}
