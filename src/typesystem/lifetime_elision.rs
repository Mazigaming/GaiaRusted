//! # Lifetime Elision (RFC 130)
//!
//! Support for automatic lifetime inference based on RFC 130 rules.
//!
//! This module provides:
//! - Input lifetime position tracking
//! - Output lifetime position tracking
//! - Implicit lifetime inference
//! - Elision error detection
//! - Three basic elision rules
//!
//! # Examples
//!
//! ```ignore
//! use gaiarusted::typesystem::lifetime_elision::{LifetimeElisionAnalyzer, LifetimeElisionConfig};
//!
//! let config = LifetimeElisionConfig::default();
//! let mut analyzer = LifetimeElisionAnalyzer::new(config);
//!
//! // Register function signature
//! analyzer.register_function("foo", vec!["&T"], "&U")?;
//!
//! // Infer lifetimes
//! let report = analyzer.infer_lifetimes("foo")?;
//! ```

use std::collections::HashMap;

/// Configuration for lifetime elision analysis
#[derive(Debug, Clone)]
pub struct LifetimeElisionConfig {
    /// Enable rule 1 (each elided lifetime in input becomes distinct)
    pub enable_rule1: bool,
    /// Enable rule 2 (if exactly one input lifetime, it's used for all elided outputs)
    pub enable_rule2: bool,
    /// Enable rule 3 (if &self or &mut self, use self's lifetime for all elided outputs)
    pub enable_rule3: bool,
    /// Maximum input positions to track
    pub max_input_positions: usize,
    /// Maximum output positions to track
    pub max_output_positions: usize,
}

impl Default for LifetimeElisionConfig {
    fn default() -> Self {
        LifetimeElisionConfig {
            enable_rule1: true,
            enable_rule2: true,
            enable_rule3: true,
            max_input_positions: 128,
            max_output_positions: 64,
        }
    }
}

/// Information about a lifetime position
#[derive(Debug, Clone, PartialEq)]
pub struct LifetimePosition {
    /// Parameter index
    pub param_index: usize,
    /// Reference nesting depth
    pub depth: usize,
    /// Is this position in `&mut` reference
    pub is_mutable: bool,
    /// Associated lifetime name (if explicit)
    pub explicit_lifetime: Option<String>,
}

impl LifetimePosition {
    fn new(param_index: usize, depth: usize, is_mutable: bool) -> Self {
        LifetimePosition {
            param_index,
            depth,
            is_mutable,
            explicit_lifetime: None,
        }
    }

    fn with_lifetime(mut self, lifetime: String) -> Self {
        self.explicit_lifetime = Some(lifetime);
        self
    }
}

/// Function signature information
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    /// Input parameter types
    pub inputs: Vec<String>,
    /// Output type
    pub output: String,
    /// Input lifetime positions
    pub input_positions: Vec<LifetimePosition>,
    /// Output lifetime positions
    pub output_positions: Vec<LifetimePosition>,
    /// Inferred lifetimes
    pub inferred_lifetimes: HashMap<usize, String>,
}

impl FunctionSignature {
    fn new(inputs: Vec<String>, output: String) -> Self {
        FunctionSignature {
            inputs,
            output,
            input_positions: Vec::new(),
            output_positions: Vec::new(),
            inferred_lifetimes: HashMap::new(),
        }
    }
}

/// Main lifetime elision analyzer
pub struct LifetimeElisionAnalyzer {
    config: LifetimeElisionConfig,
    functions: HashMap<String, FunctionSignature>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl LifetimeElisionAnalyzer {
    /// Create a new analyzer
    pub fn new(config: LifetimeElisionConfig) -> Self {
        LifetimeElisionAnalyzer {
            config,
            functions: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Register a function signature
    pub fn register_function(
        &mut self,
        name: &str,
        inputs: Vec<&str>,
        output: &str,
    ) -> Result<(), String> {
        if name.is_empty() {
            return Err("Function name cannot be empty".to_string());
        }

        let inputs: Vec<String> = inputs.iter().map(|s| s.to_string()).collect();

        if inputs.len() > self.config.max_input_positions {
            return Err(format!(
                "Too many input parameters: {} > {}",
                inputs.len(),
                self.config.max_input_positions
            ));
        }

        let mut sig = FunctionSignature::new(inputs.clone(), output.to_string());

        // Parse input positions
        for (idx, input) in inputs.iter().enumerate() {
            if input.starts_with('&') {
                let is_mutable = input.starts_with("&mut");
                let pos = LifetimePosition::new(idx, 1, is_mutable);
                sig.input_positions.push(pos);
            }
        }

        // Parse output positions
        if output.starts_with('&') {
            let is_mutable = output.starts_with("&mut");
            let pos = LifetimePosition::new(0, 1, is_mutable);
            sig.output_positions.push(pos);
        }

        self.functions.insert(name.to_string(), sig);
        Ok(())
    }

    /// Infer lifetimes for a function using RFC 130 rules
    pub fn infer_lifetimes(&mut self, func_name: &str) -> Result<LifetimeElisionReport, String> {
        let sig = self
            .functions
            .get(func_name)
            .ok_or_else(|| format!("Function {} not found", func_name))?
            .clone();

        let mut report = LifetimeElisionReport {
            function: func_name.to_string(),
            input_count: sig.inputs.len(),
            output_count: sig.output_positions.len(),
            inferred_count: 0,
            ambiguous: false,
            errors: Vec::new(),
        };

        // Rule 1: Each elided lifetime in input becomes distinct
        if self.config.enable_rule1 && sig.input_positions.len() > 1 {
            for (idx, _pos) in sig.input_positions.iter().enumerate() {
                report.inferred_count += 1;
            }
        }

        // Rule 2: If exactly one input lifetime, it's used for all elided outputs
        if self.config.enable_rule2 && sig.input_positions.len() == 1 {
            report.inferred_count = sig.output_positions.len();
        }

        // Rule 3: If &self or &mut self, self's lifetime applies to all elided outputs
        if self.config.enable_rule3 {
            let has_self = sig.inputs.iter().any(|i| i.contains("self"));
            if has_self && sig.output_positions.len() > 0 {
                report.inferred_count = sig.output_positions.len();
            }
        }

        // Check for ambiguity
        if sig.input_positions.len() > 1 && sig.output_positions.len() > 0 {
            report.ambiguous = true;
            report
                .errors
                .push("Ambiguous lifetimes in output position".to_string());
        }

        Ok(report)
    }

    /// Check if a lifetime is explicit
    pub fn has_explicit_lifetime(&self, func_name: &str, param_index: usize) -> bool {
        self.functions
            .get(func_name)
            .map(|sig| {
                sig.input_positions
                    .iter()
                    .any(|p| p.param_index == param_index && p.explicit_lifetime.is_some())
            })
            .unwrap_or(false)
    }

    /// Get input lifetime count
    pub fn input_lifetime_count(&self, func_name: &str) -> Option<usize> {
        self.functions
            .get(func_name)
            .map(|sig| sig.input_positions.len())
    }

    /// Get output lifetime count
    pub fn output_lifetime_count(&self, func_name: &str) -> Option<usize> {
        self.functions
            .get(func_name)
            .map(|sig| sig.output_positions.len())
    }

    /// Generate analysis report
    pub fn generate_report(&self) -> LifetimeElisionAnalysisReport {
        LifetimeElisionAnalysisReport {
            function_count: self.functions.len(),
            total_input_positions: self.functions.values().map(|s| s.input_positions.len()).sum(),
            total_output_positions: self
                .functions
                .values()
                .map(|s| s.output_positions.len())
                .sum(),
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

/// Report from lifetime elision inference
#[derive(Debug, Clone)]
pub struct LifetimeElisionReport {
    /// Function name
    pub function: String,
    /// Number of input parameters
    pub input_count: usize,
    /// Number of output positions
    pub output_count: usize,
    /// Number of inferred lifetimes
    pub inferred_count: usize,
    /// Is the inference ambiguous
    pub ambiguous: bool,
    /// Error messages
    pub errors: Vec<String>,
}

/// Analysis report from lifetime elision
#[derive(Debug, Clone)]
pub struct LifetimeElisionAnalysisReport {
    /// Total functions analyzed
    pub function_count: usize,
    /// Total input lifetime positions
    pub total_input_positions: usize,
    /// Total output lifetime positions
    pub total_output_positions: usize,
    /// Error messages
    pub errors: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_analyzer() -> LifetimeElisionAnalyzer {
        LifetimeElisionAnalyzer::new(LifetimeElisionConfig::default())
    }

    #[test]
    fn test_create_analyzer() {
        let analyzer = create_test_analyzer();
        assert_eq!(analyzer.functions.len(), 0);
    }

    #[test]
    fn test_register_function() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_function("foo", vec!["&str"], "&str");
        assert!(result.is_ok());
        assert_eq!(analyzer.functions.len(), 1);
    }

    #[test]
    fn test_register_function_empty_name() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_function("", vec!["&str"], "&str");
        assert!(result.is_err());
    }

    #[test]
    fn test_input_lifetime_count() {
        let mut analyzer = create_test_analyzer();
        analyzer
            .register_function("foo", vec!["&str", "&str"], "&str")
            .ok();
        assert_eq!(analyzer.input_lifetime_count("foo"), Some(2));
    }

    #[test]
    fn test_output_lifetime_count() {
        let mut analyzer = create_test_analyzer();
        analyzer
            .register_function("foo", vec!["&str"], "&str")
            .ok();
        assert_eq!(analyzer.output_lifetime_count("foo"), Some(1));
    }

    #[test]
    fn test_infer_lifetimes_rule2() {
        let mut analyzer = create_test_analyzer();
        analyzer
            .register_function("foo", vec!["&str"], "&str")
            .ok();
        let report = analyzer.infer_lifetimes("foo").unwrap();
        assert_eq!(report.inferred_count, 1);
    }

    #[test]
    fn test_has_explicit_lifetime() {
        let analyzer = create_test_analyzer();
        assert!(!analyzer.has_explicit_lifetime("foo", 0));
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
    fn test_generate_report() {
        let mut analyzer = create_test_analyzer();
        analyzer
            .register_function("foo", vec!["&str", "&str"], "&str")
            .ok();
        let report = analyzer.generate_report();
        assert_eq!(report.function_count, 1);
        assert_eq!(report.total_input_positions, 2);
    }

    #[test]
    fn test_multiple_functions() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_function("foo", vec!["&str"], "&str").ok();
        analyzer.register_function("bar", vec!["&str", "&str"], "&str").ok();
        assert_eq!(analyzer.functions.len(), 2);
    }

    #[test]
    fn test_non_reference_input() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_function("foo", vec!["i32"], "i32").ok();
        assert_eq!(analyzer.input_lifetime_count("foo"), Some(0));
    }

    #[test]
    fn test_mutable_reference() {
        let mut analyzer = create_test_analyzer();
        analyzer
            .register_function("foo", vec!["&mut str"], "&str")
            .ok();
        let count = analyzer.input_lifetime_count("foo");
        assert_eq!(count, Some(1));
    }

    #[test]
    fn test_ambiguous_inference() {
        let mut analyzer = create_test_analyzer();
        analyzer
            .register_function("foo", vec!["&str", "&str"], "&str")
            .ok();
        let report = analyzer.infer_lifetimes("foo").unwrap();
        assert!(report.ambiguous);
    }

    #[test]
    fn test_unknown_function() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.infer_lifetimes("unknown");
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
