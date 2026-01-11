//! # Guard Clause Type Checking
//!
//! Validates that guard clauses in match expressions are bool type.
//! This module enforces Rust's requirement that guards must evaluate to boolean values.
//!
//! # Examples
//!
//! ```ignore
//! use gaiarusted::typesystem::guard_checker::GuardChecker;
//!
//! let mut checker = GuardChecker::new();
//! // Register guard: match x { n if n > 0 => ... }
//! checker.register_guard("match_1", "n > 0", "n", "i32")?;
//! let report = checker.validate_guards("match_1")?;
//! ```

use std::collections::{HashMap, HashSet};

/// Configuration for guard checking
#[derive(Debug, Clone)]
pub struct GuardCheckerConfig {
    /// Enable strict guard type checking
    pub enable_strict: bool,
    /// Maximum nested operations in guard
    pub max_guard_depth: usize,
    /// Allow implicit bool conversions (truthy values)
    pub allow_implicit_bool: bool,
}

impl Default for GuardCheckerConfig {
    fn default() -> Self {
        GuardCheckerConfig {
            enable_strict: true,
            max_guard_depth: 16,
            allow_implicit_bool: false,
        }
    }
}

/// Represents a guard clause in a match arm
#[derive(Debug, Clone)]
pub struct Guard {
    /// Guard expression code
    pub expression: String,
    /// Inferred type of guard expression
    pub inferred_type: String,
    /// Is the type valid (bool)
    pub is_valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
}

impl Guard {
    fn new(expression: String) -> Self {
        Guard {
            expression,
            inferred_type: "unknown".to_string(),
            is_valid: false,
            error: None,
        }
    }
}

/// Information about a match arm with guard
#[derive(Debug, Clone)]
pub struct GuardedArm {
    /// Pattern being matched
    pub pattern: String,
    /// Guard clause (if present)
    pub guard: Option<Guard>,
    /// Type of scrutinee
    pub scrutinee_type: String,
}

/// Match expression with guards
#[derive(Debug, Clone)]
pub struct MatchWithGuards {
    /// Name/ID of match expression
    pub name: String,
    /// Type of the value being matched
    pub scrutinee_type: String,
    /// Arms with guards
    pub arms: Vec<GuardedArm>,
}

/// Report of guard validation
#[derive(Debug, Clone)]
pub struct GuardValidationReport {
    /// Match expression name
    pub match_name: String,
    /// Total guards found
    pub total_guards: usize,
    /// Valid guards
    pub valid_guards: usize,
    /// Invalid guards
    pub invalid_guards: usize,
    /// Error details
    pub errors: Vec<GuardError>,
}

impl GuardValidationReport {
    /// Check if all guards are valid
    pub fn all_valid(&self) -> bool {
        self.invalid_guards == 0 && self.errors.is_empty()
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        format!(
            "Guard validation: {} total, {} valid, {} invalid",
            self.total_guards, self.valid_guards, self.invalid_guards
        )
    }
}

/// Detailed error for invalid guard
#[derive(Debug, Clone)]
pub struct GuardError {
    /// Pattern this guard belongs to
    pub pattern: String,
    /// Guard expression
    pub guard_expr: String,
    /// Inferred type
    pub inferred_type: String,
    /// Error message
    pub message: String,
}

/// Main guard checker
pub struct GuardChecker {
    config: GuardCheckerConfig,
    pub matches: HashMap<String, MatchWithGuards>,
    builtin_types: HashSet<String>,
}

impl GuardChecker {
    /// Create new guard checker
    pub fn new() -> Self {
        Self::with_config(GuardCheckerConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: GuardCheckerConfig) -> Self {
        let mut builtin_types = HashSet::new();
        builtin_types.insert("bool".to_string());
        builtin_types.insert("i32".to_string());
        builtin_types.insert("i64".to_string());
        builtin_types.insert("u32".to_string());
        builtin_types.insert("u64".to_string());
        builtin_types.insert("f32".to_string());
        builtin_types.insert("f64".to_string());
        builtin_types.insert("String".to_string());
        builtin_types.insert("&str".to_string());

        GuardChecker {
            config,
            matches: HashMap::new(),
            builtin_types,
        }
    }

    /// Register a match expression
    pub fn register_match(
        &mut self,
        name: &str,
        scrutinee_type: &str,
    ) -> Result<(), String> {
        if name.is_empty() {
            return Err("Match name cannot be empty".to_string());
        }
        if scrutinee_type.is_empty() {
            return Err("Scrutinee type cannot be empty".to_string());
        }

        self.matches.insert(
            name.to_string(),
            MatchWithGuards {
                name: name.to_string(),
                scrutinee_type: scrutinee_type.to_string(),
                arms: Vec::new(),
            },
        );
        Ok(())
    }

    /// Register an arm in a match expression (with optional guard)
    pub fn register_arm(
        &mut self,
        match_name: &str,
        pattern: &str,
        guard: Option<&str>,
    ) -> Result<(), String> {
        let match_expr = self
            .matches
            .get_mut(match_name)
            .ok_or_else(|| format!("Match expression {} not found", match_name))?;

        let guard_obj = guard.map(|g| Guard::new(g.to_string()));

        let arm = GuardedArm {
            pattern: pattern.to_string(),
            guard: guard_obj,
            scrutinee_type: match_expr.scrutinee_type.clone(),
        };

        match_expr.arms.push(arm);
        Ok(())
    }

    /// Validate all guards in a match expression
    pub fn validate_guards(&mut self, match_name: &str) -> Result<GuardValidationReport, String> {
        let match_expr = self
            .matches
            .get_mut(match_name)
            .ok_or_else(|| format!("Match expression {} not found", match_name))?
            .clone();

        let mut report = GuardValidationReport {
            match_name: match_name.to_string(),
            total_guards: 0,
            valid_guards: 0,
            invalid_guards: 0,
            errors: Vec::new(),
        };

        for arm in match_expr.arms {
            if let Some(mut guard) = arm.guard {
                report.total_guards += 1;

                // Type-check the guard
                let inferred_type = self.infer_guard_type(&guard.expression, &arm.scrutinee_type);
                guard.inferred_type = inferred_type.clone();

                if inferred_type == "bool" {
                    guard.is_valid = true;
                    report.valid_guards += 1;
                } else {
                    guard.is_valid = false;
                    guard.error = Some(format!(
                        "Guard must be bool, found {}",
                        inferred_type
                    ));
                    report.invalid_guards += 1;
                    report.errors.push(GuardError {
                        pattern: arm.pattern.clone(),
                        guard_expr: guard.expression.clone(),
                        inferred_type: inferred_type.clone(),
                        message: format!("Guard must be bool, found {}", inferred_type),
                    });
                }

                // Update the stored guard with inferred type
                if let Some(match_expr_mut) = self.matches.get_mut(match_name) {
                    if let Some(arm_mut) = match_expr_mut
                        .arms
                        .iter_mut()
                        .find(|a| a.pattern == arm.pattern)
                    {
                        arm_mut.guard = Some(guard);
                    }
                }
            }
        }

        Ok(report)
    }

    /// Infer the type of a guard expression
    fn infer_guard_type(&self, expression: &str, scrutinee_type: &str) -> String {
        let expr = expression.trim();

        // Direct bool literals
        if expr == "true" || expr == "false" {
            return "bool".to_string();
        }

        // Comparison operators always return bool
        if expr.contains("==")
            || expr.contains("!=")
            || expr.contains("<=")
            || expr.contains(">=")
            || expr.contains("<")
            || expr.contains(">")
        {
            return "bool".to_string();
        }

        // Logical operators return bool
        if expr.contains("&&") || expr.contains("||") || expr.starts_with("!") {
            return "bool".to_string();
        }

        // Method calls that return bool
        if expr.contains(".is_")
            || expr.contains(".ends_with")
            || expr.contains(".starts_with")
            || expr.contains(".contains")
            || expr.contains(".is_empty")
            || expr.contains(".is_some")
            || expr.contains(".is_none")
            || expr.contains(".is_ok")
            || expr.contains(".is_err")
        {
            return "bool".to_string();
        }

        // Single variable might be bool (need context, but we'll assume based on name)
        if !expr.contains(' ')
            && !expr.contains('(')
            && !expr.contains('.')
            && self.looks_like_bool_variable(expr)
        {
            return "bool".to_string();
        }

        // Complex expressions - try to parse for operators
        if self.contains_bool_operation(expr) {
            return "bool".to_string();
        }

        // Unknown type
        "unknown".to_string()
    }

    /// Check if variable name suggests bool type
    fn looks_like_bool_variable(&self, name: &str) -> bool {
        name.starts_with("is_")
            || name.starts_with("has_")
            || name == "true"
            || name == "false"
            || name == "b"  // Common bool variable name
            || name == "ok"
            || name == "valid"
            || name.ends_with("_ok")
    }

    /// Check if expression contains boolean operations
    fn contains_bool_operation(&self, expr: &str) -> bool {
        // Look for comparison or logical operators
        expr.contains("==")
            || expr.contains("!=")
            || expr.contains('<')
            || expr.contains('>')
            || expr.contains("&&")
            || expr.contains("||")
            || expr.contains("!")
    }

    /// Get a match expression by name
    pub fn get_match(&self, match_name: &str) -> Option<&MatchWithGuards> {
        self.matches.get(match_name)
    }

    /// Get guard count for a match
    pub fn guard_count(&self, match_name: &str) -> usize {
        self.matches
            .get(match_name)
            .map(|m| m.arms.iter().filter(|a| a.guard.is_some()).count())
            .unwrap_or(0)
    }

    /// Check if guard is valid
    pub fn is_guard_valid(&self, match_name: &str, pattern: &str) -> Option<bool> {
        self.matches.get(match_name).and_then(|m| {
            m.arms.iter().find(|a| a.pattern == pattern).and_then(|a| {
                a.guard.as_ref().map(|g| g.is_valid)
            })
        })
    }

    /// Generate analysis report
    pub fn generate_report(&self) -> GuardAnalysisReport {
        let mut total_matches = 0;
        let mut total_guards = 0;
        let mut total_valid_guards = 0;
        let mut all_errors = Vec::new();

        for (_, match_expr) in self.matches.iter() {
            total_matches += 1;
            for arm in &match_expr.arms {
                if let Some(guard) = &arm.guard {
                    total_guards += 1;
                    if guard.is_valid {
                        total_valid_guards += 1;
                    } else if let Some(error) = &guard.error {
                        all_errors.push(error.clone());
                    }
                }
            }
        }

        GuardAnalysisReport {
            total_matches,
            total_guards,
            valid_guards: total_valid_guards,
            invalid_guards: total_guards - total_valid_guards,
            errors: all_errors,
        }
    }
}

impl Default for GuardChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Overall analysis report for all guards
#[derive(Debug, Clone)]
pub struct GuardAnalysisReport {
    /// Total match expressions analyzed
    pub total_matches: usize,
    /// Total guard clauses
    pub total_guards: usize,
    /// Valid guard clauses
    pub valid_guards: usize,
    /// Invalid guard clauses
    pub invalid_guards: usize,
    /// All error messages
    pub errors: Vec<String>,
}

impl GuardAnalysisReport {
    /// Check if all guards are valid
    pub fn all_valid(&self) -> bool {
        self.invalid_guards == 0 && self.errors.is_empty()
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        format!(
            "Guard analysis: {} matches, {} guards total ({} valid, {} invalid)",
            self.total_matches, self.total_guards, self.valid_guards, self.invalid_guards
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_checker() -> GuardChecker {
        GuardChecker::new()
    }

    #[test]
    fn test_create_checker() {
        let checker = create_test_checker();
        assert_eq!(checker.matches.len(), 0);
    }

    #[test]
    fn test_register_match() {
        let mut checker = create_test_checker();
        let result = checker.register_match("test_match", "i32");
        assert!(result.is_ok());
        assert!(checker.get_match("test_match").is_some());
    }

    #[test]
    fn test_register_match_empty_name() {
        let mut checker = create_test_checker();
        let result = checker.register_match("", "i32");
        assert!(result.is_err());
    }

    #[test]
    fn test_register_arm() {
        let mut checker = create_test_checker();
        checker.register_match("test_match", "i32").ok();
        let result = checker.register_arm("test_match", "42", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_arm_with_valid_guard() {
        let mut checker = create_test_checker();
        checker.register_match("test_match", "i32").ok();
        let result = checker.register_arm("test_match", "x", Some("x > 0"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_infer_guard_type_comparison() {
        let checker = create_test_checker();
        let result = checker.infer_guard_type("x > 0", "i32");
        assert_eq!(result, "bool");
    }

    #[test]
    fn test_infer_guard_type_equality() {
        let checker = create_test_checker();
        let result = checker.infer_guard_type("x == 5", "i32");
        assert_eq!(result, "bool");
    }

    #[test]
    fn test_infer_guard_type_literal() {
        let checker = create_test_checker();
        let result = checker.infer_guard_type("true", "i32");
        assert_eq!(result, "bool");
    }

    #[test]
    fn test_infer_guard_type_logical_and() {
        let checker = create_test_checker();
        let result = checker.infer_guard_type("x > 0 && x < 10", "i32");
        assert_eq!(result, "bool");
    }

    #[test]
    fn test_infer_guard_type_logical_or() {
        let checker = create_test_checker();
        let result = checker.infer_guard_type("x < 0 || x > 100", "i32");
        assert_eq!(result, "bool");
    }

    #[test]
    fn test_infer_guard_type_negation() {
        let checker = create_test_checker();
        let result = checker.infer_guard_type("!valid", "i32");
        assert_eq!(result, "bool");
    }

    #[test]
    fn test_validate_guards_all_valid() {
        let mut checker = create_test_checker();
        checker.register_match("test_match", "i32").ok();
        checker
            .register_arm("test_match", "x", Some("x > 0"))
            .ok();
        checker
            .register_arm("test_match", "x", Some("x == 5"))
            .ok();

        let report = checker.validate_guards("test_match").unwrap();
        assert_eq!(report.total_guards, 2);
        assert_eq!(report.valid_guards, 2);
        assert_eq!(report.invalid_guards, 0);
        assert!(report.all_valid());
    }

    #[test]
    fn test_validate_guards_with_invalid() {
        let mut checker = create_test_checker();
        checker.register_match("test_match", "i32").ok();
        checker.register_arm("test_match", "x", Some("5")).ok();

        let report = checker.validate_guards("test_match").unwrap();
        assert_eq!(report.total_guards, 1);
        assert_eq!(report.valid_guards, 0);
        assert_eq!(report.invalid_guards, 1);
        assert!(!report.all_valid());
    }

    #[test]
    fn test_guard_count() {
        let mut checker = create_test_checker();
        checker.register_match("test_match", "i32").ok();
        checker.register_arm("test_match", "0", None).ok();
        checker
            .register_arm("test_match", "x", Some("x > 0"))
            .ok();
        checker.register_arm("test_match", "_", None).ok();

        assert_eq!(checker.guard_count("test_match"), 1);
    }

    #[test]
    fn test_is_guard_valid() {
        let mut checker = create_test_checker();
        checker.register_match("test_match", "i32").ok();
        checker
            .register_arm("test_match", "x", Some("x > 0"))
            .ok();
        checker.validate_guards("test_match").ok();

        assert_eq!(
            checker.is_guard_valid("test_match", "x"),
            Some(true)
        );
    }

    #[test]
    fn test_generate_report() {
        let mut checker = create_test_checker();
        checker.register_match("match1", "i32").ok();
        checker
            .register_arm("match1", "x", Some("x > 0"))
            .ok();
        checker.register_match("match2", "bool").ok();
        checker
            .register_arm("match2", "true", Some("true"))
            .ok();

        checker.validate_guards("match1").ok();
        checker.validate_guards("match2").ok();

        let report = checker.generate_report();
        assert_eq!(report.total_matches, 2);
        assert_eq!(report.total_guards, 2);
        assert_eq!(report.valid_guards, 2);
    }

    #[test]
    fn test_looks_like_bool_variable() {
        let checker = create_test_checker();
        assert!(checker.looks_like_bool_variable("is_empty"));
        assert!(checker.looks_like_bool_variable("has_value"));
        assert!(!checker.looks_like_bool_variable("count"));
    }

    #[test]
    fn test_method_call_guard() {
        let checker = create_test_checker();
        let result = checker.infer_guard_type("items.is_empty()", "Vec");
        assert_eq!(result, "bool");
    }

    #[test]
    fn test_complex_guard_expression() {
        let checker = create_test_checker();
        let result = checker.infer_guard_type("(x > 0) && (y < 10) || z == 5", "i32");
        assert_eq!(result, "bool");
    }

    #[test]
    fn test_multiple_matches_report() {
        let mut checker = create_test_checker();
        checker.register_match("match1", "i32").ok();
        checker.register_match("match2", "i32").ok();
        checker.register_match("match3", "i32").ok();

        let report = checker.generate_report();
        assert_eq!(report.total_matches, 3);
    }
}
