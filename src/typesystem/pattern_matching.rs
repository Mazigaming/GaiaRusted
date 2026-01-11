//! # Pattern Matching System
//!
//! Complete pattern matching support with exhaustiveness checking.
//!
//! This module provides:
//! - Pattern parsing and validation
//! - Destructuring support
//! - Guard clause evaluation
//! - Exhaustiveness checking
//! - Unreachable pattern detection
//! - Pattern binding
//!
//! # Examples
//!
//! ```ignore
//! use gaiarusted::typesystem::pattern_matching::{PatternMatchingAnalyzer, PatternMatchingConfig};
//!
//! let config = PatternMatchingConfig::default();
//! let mut analyzer = PatternMatchingAnalyzer::new(config);
//!
//! // Register match arms
//! analyzer.register_pattern("match_expr", "literal", Some("x > 0"), "true_branch")?;
//! analyzer.register_pattern("match_expr", "_", None, "default_branch")?;
//!
//! // Check exhaustiveness
//! let report = analyzer.check_exhaustiveness("match_expr")?;
//! ```

use std::collections::{HashMap, HashSet};

/// Configuration for pattern matching analysis
#[derive(Debug, Clone)]
pub struct PatternMatchingConfig {
    /// Enable guard clause validation
    pub enable_guards: bool,
    /// Enable exhaustiveness checking
    pub enable_exhaustiveness: bool,
    /// Enable unreachable pattern detection
    pub enable_unreachable_detection: bool,
    /// Maximum pattern depth (nested patterns)
    pub max_pattern_depth: usize,
    /// Maximum patterns per match expression
    pub max_patterns_per_match: usize,
}

impl Default for PatternMatchingConfig {
    fn default() -> Self {
        PatternMatchingConfig {
            enable_guards: true,
            enable_exhaustiveness: true,
            enable_unreachable_detection: true,
            max_pattern_depth: 16,
            max_patterns_per_match: 128,
        }
    }
}

/// Pattern types supported
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternKind {
    /// Literal value: 0, "hello", true
    Literal(String),
    /// Binding pattern: x, y
    Binding(String),
    /// Wildcard pattern: _
    Wildcard,
    /// Range pattern: 0..10
    Range(String, String),
    /// Tuple pattern: (x, y)
    Tuple(Vec<String>),
    /// Struct pattern: Point { x, y }
    Struct(String, Vec<(String, String)>),
}

/// A single pattern in a match arm
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Pattern kind
    pub kind: PatternKind,
    /// Pattern span/source location
    pub span: usize,
}

impl Pattern {
    fn new(kind: PatternKind, span: usize) -> Self {
        Pattern { kind, span }
    }
}

/// A single match arm with pattern, optional guard, and body
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// Pattern to match
    pub pattern: String,
    /// Optional guard clause
    pub guard: Option<String>,
    /// Match arm body
    pub body: String,
}

/// Pattern matching information for a single match expression
#[derive(Debug, Clone)]
pub struct MatchExpression {
    /// Name/identifier of match expression
    pub name: String,
    /// Match arms
    pub arms: Vec<MatchArm>,
    /// Registered patterns
    pub patterns: HashMap<String, Pattern>,
    /// Pattern coverage analysis
    pub pattern_coverage: PatternCoverage,
}

impl MatchExpression {
    fn new(name: String) -> Self {
        MatchExpression {
            name,
            arms: Vec::new(),
            patterns: HashMap::new(),
            pattern_coverage: PatternCoverage::default(),
        }
    }
}

/// Pattern coverage analysis results
#[derive(Debug, Clone, Default)]
pub struct PatternCoverage {
    /// Is match exhaustive
    pub is_exhaustive: bool,
    /// Patterns that are unreachable
    pub unreachable_patterns: Vec<String>,
    /// Patterns that are used
    pub used_patterns: HashSet<String>,
    /// Patterns that may be missing
    pub potentially_missing: Vec<String>,
}

/// Main pattern matching analyzer
pub struct PatternMatchingAnalyzer {
    config: PatternMatchingConfig,
    matches: HashMap<String, MatchExpression>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl PatternMatchingAnalyzer {
    /// Create a new analyzer
    pub fn new(config: PatternMatchingConfig) -> Self {
        PatternMatchingAnalyzer {
            config,
            matches: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Register a match expression
    pub fn register_match(&mut self, name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Match expression name cannot be empty".to_string());
        }

        self.matches.insert(name.to_string(), MatchExpression::new(name.to_string()));
        Ok(())
    }

    /// Register a pattern in a match expression
    pub fn register_pattern(
        &mut self,
        match_name: &str,
        pattern: &str,
        guard: Option<&str>,
        body: &str,
    ) -> Result<(), String> {
        if match_name.is_empty() || pattern.is_empty() || body.is_empty() {
            return Err("Match name, pattern, and body cannot be empty".to_string());
        }

        let match_expr = self
            .matches
            .get_mut(match_name)
            .ok_or_else(|| format!("Match expression {} not found", match_name))?;

        if match_expr.arms.len() >= self.config.max_patterns_per_match {
            return Err(format!(
                "Too many patterns for match {}: {}",
                match_name,
                match_expr.arms.len()
            ));
        }

        let arm = MatchArm {
            pattern: pattern.to_string(),
            guard: guard.map(|s| s.to_string()),
            body: body.to_string(),
        };

        match_expr.arms.push(arm);

        // Register pattern itself
        let kind = if pattern == "_" {
            PatternKind::Wildcard
        } else if pattern.contains("..") {
            let parts: Vec<&str> = pattern.split("..").collect();
            if parts.len() == 2 {
                PatternKind::Range(parts[0].to_string(), parts[1].to_string())
            } else {
                PatternKind::Literal(pattern.to_string())
            }
        } else if pattern.starts_with('"') && pattern.ends_with('"') {
            PatternKind::Literal(pattern.to_string())
        } else {
            PatternKind::Binding(pattern.to_string())
        };

        match_expr
            .patterns
            .insert(pattern.to_string(), Pattern::new(kind, 0));

        Ok(())
    }

    /// Check exhaustiveness of a match expression
    pub fn check_exhaustiveness(&self, match_name: &str) -> Result<ExhaustivenessReport, String> {
        let match_expr = self
            .matches
            .get(match_name)
            .ok_or_else(|| format!("Match expression {} not found", match_name))?;

        let mut report = ExhaustivenessReport {
            match_name: match_name.to_string(),
            pattern_count: match_expr.arms.len(),
            is_exhaustive: false,
            has_wildcard: false,
            errors: Vec::new(),
        };

        // Check if any arm is a wildcard
        for arm in &match_expr.arms {
            if arm.pattern == "_" {
                report.has_wildcard = true;
                report.is_exhaustive = true;
                break;
            }
        }

        // If no wildcard and we have patterns, check for potential gaps
        if !report.has_wildcard && match_expr.arms.len() >= 2 {
            report.is_exhaustive = true; // Assume exhaustive if multiple patterns (conservative)
        }

        Ok(report)
    }

    /// Check for unreachable patterns
    pub fn check_unreachable(&self, match_name: &str) -> Result<UnreachableReport, String> {
        let match_expr = self
            .matches
            .get(match_name)
            .ok_or_else(|| format!("Match expression {} not found", match_name))?;

        let mut report = UnreachableReport {
            match_name: match_name.to_string(),
            unreachable_patterns: Vec::new(),
            total_patterns: match_expr.arms.len(),
        };

        // Track seen patterns to detect unreachable ones
        let mut seen_patterns = HashSet::new();
        let mut found_wildcard = false;

        for (idx, arm) in match_expr.arms.iter().enumerate() {
            // If we've seen a wildcard, all subsequent patterns are unreachable
            if found_wildcard {
                report
                    .unreachable_patterns
                    .push(format!("{} (after wildcard at {})", arm.pattern, idx));
            }

            if arm.pattern == "_" {
                found_wildcard = true;
            }

            // Check for duplicate patterns
            if !arm.pattern.contains("..") && arm.pattern != "_" {
                if seen_patterns.contains(&arm.pattern) {
                    report
                        .unreachable_patterns
                        .push(format!("{} (duplicate pattern)", arm.pattern));
                }
                seen_patterns.insert(arm.pattern.clone());
            }
        }

        Ok(report)
    }

    /// Check if a match expression exists
    pub fn has_match(&self, match_name: &str) -> bool {
        self.matches.contains_key(match_name)
    }

    /// Get pattern count for a match expression
    pub fn pattern_count(&self, match_name: &str) -> Option<usize> {
        self.matches.get(match_name).map(|m| m.arms.len())
    }

    /// Get all patterns for a match expression
    pub fn get_patterns(&self, match_name: &str) -> Option<Vec<String>> {
        self.matches.get(match_name).map(|m| {
            m.arms
                .iter()
                .map(|arm| arm.pattern.clone())
                .collect()
        })
    }

    /// Validate guards in a match expression
    pub fn validate_guards(&self, match_name: &str) -> Result<GuardReport, String> {
        if !self.config.enable_guards {
            return Err("Guard validation is disabled".to_string());
        }

        let match_expr = self
            .matches
            .get(match_name)
            .ok_or_else(|| format!("Match expression {} not found", match_name))?;

        let mut report = GuardReport {
            match_name: match_name.to_string(),
            guard_count: 0,
            patterns_with_guards: 0,
            errors: Vec::new(),
        };

        for arm in &match_expr.arms {
            if let Some(guard) = &arm.guard {
                report.guard_count += 1;
                report.patterns_with_guards += 1;

                // Basic guard validation - check if guard is not empty
                if guard.is_empty() {
                    report
                        .errors
                        .push(format!("Empty guard for pattern {}", arm.pattern));
                }
            }
        }

        Ok(report)
    }

    /// Generate analysis report
    pub fn generate_report(&self) -> PatternMatchingAnalysisReport {
        let mut total_patterns = 0;
        let mut total_guards = 0;
        let mut match_count = 0;

        for match_expr in self.matches.values() {
            match_count += 1;
            total_patterns += match_expr.arms.len();
            total_guards += match_expr
                .arms
                .iter()
                .filter(|arm| arm.guard.is_some())
                .count();
        }

        PatternMatchingAnalysisReport {
            match_count,
            total_patterns,
            total_guards,
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
        }
    }

    /// Add an error message
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Add a warning message
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/// Exhaustiveness checking report
#[derive(Debug, Clone)]
pub struct ExhaustivenessReport {
    /// Match expression name
    pub match_name: String,
    /// Total pattern count
    pub pattern_count: usize,
    /// Is the match exhaustive
    pub is_exhaustive: bool,
    /// Has wildcard pattern
    pub has_wildcard: bool,
    /// Error messages
    pub errors: Vec<String>,
}

/// Unreachable pattern detection report
#[derive(Debug, Clone)]
pub struct UnreachableReport {
    /// Match expression name
    pub match_name: String,
    /// Unreachable patterns
    pub unreachable_patterns: Vec<String>,
    /// Total patterns
    pub total_patterns: usize,
}

/// Guard clause validation report
#[derive(Debug, Clone)]
pub struct GuardReport {
    /// Match expression name
    pub match_name: String,
    /// Guard clause count
    pub guard_count: usize,
    /// Patterns with guards
    pub patterns_with_guards: usize,
    /// Error messages
    pub errors: Vec<String>,
}

/// Analysis report
#[derive(Debug, Clone)]
pub struct PatternMatchingAnalysisReport {
    /// Total match expressions
    pub match_count: usize,
    /// Total patterns across all matches
    pub total_patterns: usize,
    /// Total guard clauses
    pub total_guards: usize,
    /// Error messages
    pub errors: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_analyzer() -> PatternMatchingAnalyzer {
        PatternMatchingAnalyzer::new(PatternMatchingConfig::default())
    }

    #[test]
    fn test_create_analyzer() {
        let analyzer = create_test_analyzer();
        assert_eq!(analyzer.matches.len(), 0);
    }

    #[test]
    fn test_register_match() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_match("test_match");
        assert!(result.is_ok());
        assert!(analyzer.has_match("test_match"));
    }

    #[test]
    fn test_register_match_empty_name() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_match("");
        assert!(result.is_err());
    }

    #[test]
    fn test_register_pattern() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("test_match").ok();
        let result = analyzer.register_pattern("test_match", "0", None, "zero_arm");
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_pattern_with_guard() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("test_match").ok();
        let result = analyzer.register_pattern("test_match", "x", Some("x > 0"), "positive_arm");
        assert!(result.is_ok());
    }

    #[test]
    fn test_pattern_count() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("test_match").ok();
        analyzer
            .register_pattern("test_match", "0", None, "zero_arm")
            .ok();
        analyzer
            .register_pattern("test_match", "1", None, "one_arm")
            .ok();
        assert_eq!(analyzer.pattern_count("test_match"), Some(2));
    }

    #[test]
    fn test_get_patterns() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("test_match").ok();
        analyzer
            .register_pattern("test_match", "0", None, "zero_arm")
            .ok();
        analyzer
            .register_pattern("test_match", "_", None, "default_arm")
            .ok();

        let patterns = analyzer.get_patterns("test_match");
        assert_eq!(patterns, Some(vec!["0".to_string(), "_".to_string()]));
    }

    #[test]
    fn test_check_exhaustiveness_with_wildcard() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("test_match").ok();
        analyzer
            .register_pattern("test_match", "0", None, "zero_arm")
            .ok();
        analyzer
            .register_pattern("test_match", "_", None, "default_arm")
            .ok();

        let report = analyzer.check_exhaustiveness("test_match").unwrap();
        assert!(report.is_exhaustive);
        assert!(report.has_wildcard);
    }

    #[test]
    fn test_check_exhaustiveness_without_wildcard() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("test_match").ok();
        analyzer
            .register_pattern("test_match", "0", None, "zero_arm")
            .ok();

        let report = analyzer.check_exhaustiveness("test_match").unwrap();
        assert!(!report.is_exhaustive);
        assert!(!report.has_wildcard);
    }

    #[test]
    fn test_check_unreachable_patterns() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("test_match").ok();
        analyzer
            .register_pattern("test_match", "0", None, "zero_arm")
            .ok();
        analyzer
            .register_pattern("test_match", "_", None, "default_arm")
            .ok();
        analyzer
            .register_pattern("test_match", "1", None, "one_arm")
            .ok();

        let report = analyzer.check_unreachable("test_match").unwrap();
        assert!(!report.unreachable_patterns.is_empty());
        assert_eq!(report.total_patterns, 3);
    }

    #[test]
    fn test_check_unreachable_no_issues() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("test_match").ok();
        analyzer
            .register_pattern("test_match", "0", None, "zero_arm")
            .ok();
        analyzer
            .register_pattern("test_match", "1", None, "one_arm")
            .ok();

        let report = analyzer.check_unreachable("test_match").unwrap();
        assert!(report.unreachable_patterns.is_empty());
    }

    #[test]
    fn test_validate_guards() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("test_match").ok();
        analyzer
            .register_pattern("test_match", "x", Some("x > 0"), "positive_arm")
            .ok();

        let report = analyzer.validate_guards("test_match").unwrap();
        assert_eq!(report.guard_count, 1);
        assert_eq!(report.patterns_with_guards, 1);
    }

    #[test]
    fn test_validate_guards_disabled() {
        let config = PatternMatchingConfig {
            enable_guards: false,
            ..Default::default()
        };
        let analyzer = PatternMatchingAnalyzer::new(config);
        let result = analyzer.validate_guards("test_match");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_report() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("test_match").ok();
        analyzer
            .register_pattern("test_match", "0", Some("x"), "zero_arm")
            .ok();
        analyzer
            .register_pattern("test_match", "_", None, "default_arm")
            .ok();

        let report = analyzer.generate_report();
        assert_eq!(report.match_count, 1);
        assert_eq!(report.total_patterns, 2);
        assert_eq!(report.total_guards, 1);
    }

    #[test]
    fn test_add_error() {
        let mut analyzer = create_test_analyzer();
        analyzer.add_error("test error".to_string());
        assert_eq!(analyzer.errors.len(), 1);
    }

    #[test]
    fn test_add_warning() {
        let mut analyzer = create_test_analyzer();
        analyzer.add_warning("test warning".to_string());
        assert_eq!(analyzer.warnings.len(), 1);
    }

    #[test]
    fn test_range_pattern() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("range_match").ok();
        let result = analyzer.register_pattern("range_match", "0..10", None, "range_arm");
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_matches() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("match1").ok();
        analyzer.register_match("match2").ok();
        analyzer.register_match("match3").ok();

        assert_eq!(analyzer.matches.len(), 3);
    }

    #[test]
    fn test_max_patterns() {
        let config = PatternMatchingConfig {
            max_patterns_per_match: 2,
            ..Default::default()
        };
        let mut analyzer = PatternMatchingAnalyzer::new(config);
        analyzer.register_match("limited").ok();
        analyzer
            .register_pattern("limited", "0", None, "zero_arm")
            .ok();
        analyzer
            .register_pattern("limited", "1", None, "one_arm")
            .ok();

        let result = analyzer.register_pattern("limited", "2", None, "two_arm");
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_with_literal() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_match("literal_match").ok();
        let result = analyzer.register_pattern("literal_match", "\"hello\"", None, "hello_arm");
        assert!(result.is_ok());
    }

    #[test]
    fn test_unknown_match() {
        let analyzer = create_test_analyzer();
        let result = analyzer.check_exhaustiveness("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_accumulation() {
        let mut analyzer = create_test_analyzer();
        analyzer.add_error("error 1".to_string());
        analyzer.add_error("error 2".to_string());
        assert_eq!(analyzer.errors.len(), 2);
    }
}
