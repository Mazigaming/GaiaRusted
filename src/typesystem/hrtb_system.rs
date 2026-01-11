//! # Higher-Ranked Trait Bounds (HRTBs)
//!
//! Support for `for<'a>` syntax and lifetime quantification.
//!
//! This module provides:
//! - HRTB parsing and validation
//! - Lifetime quantification
//! - Bound variable tracking
//! - HRTB in function pointers
//! - Variance checking
//!
//! # Examples
//!
//! ```ignore
//! use gaiarusted::typesystem::hrtb_system::{HRTBAnalyzer, HRTBConfig};
//!
//! let config = HRTBConfig::default();
//! let mut analyzer = HRTBAnalyzer::new(config);
//!
//! // Register HRTB
//! analyzer.register_hrtb("fn_ptr", vec!["'a"], "Fn(&'a T)")?;
//!
//! // Validate HRTB
//! let report = analyzer.validate_hrtb("fn_ptr")?;
//! ```

use std::collections::HashMap;

/// Configuration for HRTB analysis
#[derive(Debug, Clone)]
pub struct HRTBConfig {
    /// Maximum bound variables per HRTB
    pub max_bound_variables: usize,
    /// Maximum nesting depth
    pub max_nesting_depth: usize,
    /// Enable function pointer HRTBs
    pub enable_function_pointers: bool,
    /// Enable trait object HRTBs
    pub enable_trait_objects: bool,
}

impl Default for HRTBConfig {
    fn default() -> Self {
        HRTBConfig {
            max_bound_variables: 8,
            max_nesting_depth: 4,
            enable_function_pointers: true,
            enable_trait_objects: true,
        }
    }
}

/// A bound variable (lifetime)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundVariable {
    /// Variable name
    pub name: String,
    /// Is this a lifetime variable
    pub is_lifetime: bool,
    /// Scope where this is bound
    pub scope: usize,
}

impl BoundVariable {
    fn new(name: String, is_lifetime: bool) -> Self {
        BoundVariable {
            name,
            is_lifetime,
            scope: 0,
        }
    }
}

/// HRTB information
#[derive(Debug, Clone)]
pub struct HigherRankedBound {
    /// Name/identifier
    pub name: String,
    /// Bound variables
    pub bound_variables: Vec<BoundVariable>,
    /// The trait bound
    pub trait_bound: String,
    /// Is this a function pointer HRTB
    pub is_function_pointer: bool,
    /// Variance information
    pub variance: HashMap<String, Variance>,
}

/// Variance of a type parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variance {
    /// Type parameter is covariant
    Covariant,
    /// Type parameter is contravariant
    Contravariant,
    /// Type parameter is invariant
    Invariant,
}

impl HigherRankedBound {
    fn new(name: String, trait_bound: String) -> Self {
        HigherRankedBound {
            name,
            bound_variables: Vec::new(),
            trait_bound,
            is_function_pointer: false,
            variance: HashMap::new(),
        }
    }
}

/// Main HRTB analyzer
pub struct HRTBAnalyzer {
    config: HRTBConfig,
    hrtbs: HashMap<String, HigherRankedBound>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl HRTBAnalyzer {
    /// Create a new analyzer
    pub fn new(config: HRTBConfig) -> Self {
        HRTBAnalyzer {
            config,
            hrtbs: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Register an HRTB
    pub fn register_hrtb(
        &mut self,
        name: &str,
        bound_lifetimes: Vec<&str>,
        trait_bound: &str,
    ) -> Result<(), String> {
        if name.is_empty() || trait_bound.is_empty() {
            return Err("Name and trait bound cannot be empty".to_string());
        }

        if bound_lifetimes.len() > self.config.max_bound_variables {
            return Err(format!(
                "Too many bound variables: {} > {}",
                bound_lifetimes.len(),
                self.config.max_bound_variables
            ));
        }

        let mut hrtb = HigherRankedBound::new(name.to_string(), trait_bound.to_string());

        // Add bound variables
        for lifetime in bound_lifetimes {
            if lifetime.is_empty() {
                return Err("Lifetime name cannot be empty".to_string());
            }
            let var = BoundVariable::new(lifetime.to_string(), true);
            hrtb.bound_variables.push(var);
        }

        // Detect if this is a function pointer
        if trait_bound.contains("Fn") || trait_bound.contains("FnMut") || trait_bound.contains("FnOnce") {
            hrtb.is_function_pointer = true;
        }

        self.hrtbs.insert(name.to_string(), hrtb);

        Ok(())
    }

    /// Register variance for a type parameter
    pub fn register_variance(
        &mut self,
        hrtb_name: &str,
        param: &str,
        variance: Variance,
    ) -> Result<(), String> {
        let hrtb = self
            .hrtbs
            .get_mut(hrtb_name)
            .ok_or_else(|| format!("HRTB {} not found", hrtb_name))?;

        hrtb.variance.insert(param.to_string(), variance);

        Ok(())
    }

    /// Validate an HRTB
    pub fn validate_hrtb(&self, hrtb_name: &str) -> Result<HRTBValidationReport, String> {
        let hrtb = self
            .hrtbs
            .get(hrtb_name)
            .ok_or_else(|| format!("HRTB {} not found", hrtb_name))?;

        let mut report = HRTBValidationReport {
            hrtb_name: hrtb_name.to_string(),
            bound_variable_count: hrtb.bound_variables.len(),
            variance_count: hrtb.variance.len(),
            is_valid: true,
            errors: Vec::new(),
        };

        // Validate function pointer HRTBs
        if hrtb.is_function_pointer && !self.config.enable_function_pointers {
            report.is_valid = false;
            report
                .errors
                .push("Function pointer HRTBs are disabled".to_string());
        }

        // Check for duplicate bound variables
        let mut seen = std::collections::HashSet::new();
        for var in &hrtb.bound_variables {
            if !seen.insert(&var.name) {
                report.is_valid = false;
                report
                    .errors
                    .push(format!("Duplicate bound variable: {}", var.name));
            }
        }

        Ok(report)
    }

    /// Check if HRTB exists
    pub fn has_hrtb(&self, hrtb_name: &str) -> bool {
        self.hrtbs.contains_key(hrtb_name)
    }

    /// Check if HRTB is a function pointer type
    pub fn is_function_pointer(&self, hrtb_name: &str) -> bool {
        self.hrtbs
            .get(hrtb_name)
            .map(|hrtb| hrtb.is_function_pointer)
            .unwrap_or(false)
    }

    /// Get bound variables for an HRTB
    pub fn get_bound_variables(&self, hrtb_name: &str) -> Option<Vec<String>> {
        self.hrtbs.get(hrtb_name).map(|hrtb| {
            hrtb.bound_variables
                .iter()
                .map(|v| v.name.clone())
                .collect()
        })
    }

    /// Get variance information
    pub fn get_variance(&self, hrtb_name: &str, param: &str) -> Option<Variance> {
        self.hrtbs
            .get(hrtb_name)
            .and_then(|hrtb| hrtb.variance.get(param).copied())
    }

    /// Generate analysis report
    pub fn generate_report(&self) -> HRTBAnalysisReport {
        let mut total_bound_vars = 0;
        let mut total_variance = 0;

        for hrtb in self.hrtbs.values() {
            total_bound_vars += hrtb.bound_variables.len();
            total_variance += hrtb.variance.len();
        }

        HRTBAnalysisReport {
            hrtb_count: self.hrtbs.len(),
            total_bound_variables: total_bound_vars,
            total_variance_entries: total_variance,
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

/// Validation report for an HRTB
#[derive(Debug, Clone)]
pub struct HRTBValidationReport {
    /// HRTB name
    pub hrtb_name: String,
    /// Number of bound variables
    pub bound_variable_count: usize,
    /// Number of variance entries
    pub variance_count: usize,
    /// Is the HRTB valid
    pub is_valid: bool,
    /// Error messages
    pub errors: Vec<String>,
}

/// Analysis report
#[derive(Debug, Clone)]
pub struct HRTBAnalysisReport {
    /// Total HRTBs analyzed
    pub hrtb_count: usize,
    /// Total bound variables
    pub total_bound_variables: usize,
    /// Total variance entries
    pub total_variance_entries: usize,
    /// Error messages
    pub errors: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_analyzer() -> HRTBAnalyzer {
        HRTBAnalyzer::new(HRTBConfig::default())
    }

    #[test]
    fn test_create_analyzer() {
        let analyzer = create_test_analyzer();
        assert_eq!(analyzer.hrtbs.len(), 0);
    }

    #[test]
    fn test_register_hrtb() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_hrtb("foo", vec!["'a"], "Fn(&'a T)");
        assert!(result.is_ok());
        assert!(analyzer.has_hrtb("foo"));
    }

    #[test]
    fn test_register_empty_name() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_hrtb("", vec!["'a"], "Fn(&'a T)");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_bound_variables() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_hrtb("foo", vec!["'a", "'b"], "Fn(&'a T, &'b U)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_bound_variables() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_hrtb("foo", vec!["'a", "'b"], "Fn(&'a T)").ok();
        let vars = analyzer.get_bound_variables("foo");
        assert_eq!(vars, Some(vec!["'a".to_string(), "'b".to_string()]));
    }

    #[test]
    fn test_register_variance() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_hrtb("foo", vec!["'a"], "Fn(&'a T)").ok();
        let result = analyzer.register_variance("foo", "T", Variance::Covariant);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_variance() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_hrtb("foo", vec!["'a"], "Fn(&'a T)").ok();
        analyzer.register_variance("foo", "T", Variance::Contravariant).ok();
        let var = analyzer.get_variance("foo", "T");
        assert_eq!(var, Some(Variance::Contravariant));
    }

    #[test]
    fn test_validate_hrtb() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_hrtb("foo", vec!["'a"], "Fn(&'a T)").ok();
        let report = analyzer.validate_hrtb("foo").unwrap();
        assert!(report.is_valid);
    }

    #[test]
    fn test_function_pointer_detection() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_hrtb("fn_ptr", vec!["'a"], "Fn(&'a T)").ok();
        assert!(analyzer.hrtbs["fn_ptr"].is_function_pointer);
    }

    #[test]
    fn test_generate_report() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_hrtb("foo", vec!["'a"], "Fn(&'a T)").ok();
        let report = analyzer.generate_report();
        assert_eq!(report.hrtb_count, 1);
    }

    #[test]
    fn test_add_error() {
        let mut analyzer = create_test_analyzer();
        analyzer.add_error("test error".to_string());
        assert_eq!(analyzer.errors.len(), 1);
    }

    #[test]
    fn test_max_bound_variables() {
        let config = HRTBConfig {
            max_bound_variables: 2,
            ..Default::default()
        };
        let mut analyzer = HRTBAnalyzer::new(config);
        let result = analyzer.register_hrtb("foo", vec!["'a", "'b", "'c"], "Fn(&'a T)");
        assert!(result.is_err());
    }
}
