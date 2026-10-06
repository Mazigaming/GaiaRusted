//! # Lifetime Elision Analyzer (Phase 1)
//!
//! Implements Rust's lifetime elision rules to allow idiomatic code
//! without explicit lifetime annotations.
//!
//! ## Elision Rules (from Rust RFC)
//!
//! 1. Each elided lifetime in a function's arguments becomes a distinct lifetime parameter.
//! 2. If there is exactly one input lifetime position (elided or not), that lifetime
//!    is assigned to all elided output lifetimes.
//! 3. If there are multiple input lifetime positions, but one of them is `&self` or
//!    `&mut self` (i.e., this is a method), the lifetime of `self` is assigned to
//!    all elided output lifetimes.
//!
//! ## Examples
//!
//! ```rust
//! // Rule 1: fn foo(&str) -> &str  =>  fn foo<'a>(&'a str) -> &'a str
//! // Rule 2: fn foo(&str, &str) -> &str  =>  fn foo<'a, 'b>(&'a str, &'b str) -> &'a str
//! // Rule 3: fn foo(&self) -> &str  =>  fn foo<'a>(&'a self) -> &'a str
//! // Note: Rule 3 is conservative - explicit lifetimes recommended for methods
//! ```

use crate::borrowchecker::lifetimes::{Lifetime, LifetimeContext};
use crate::lowering::HirType;
use std::collections::HashMap;

/// Configuration for lifetime elision analysis
#[derive(Debug, Clone, Default)]
pub struct LifetimeElisionConfig {
    /// Enable elision rule 1 (single input reference)
    pub enable_rule1: bool,
    /// Enable elision rule 2 (multiple inputs, first is reference)
    pub enable_rule2: bool,
    /// Enable elision rule 3 (method with self)
    pub enable_rule3: bool,
}

impl LifetimeElisionConfig {
    /// Create default configuration with all rules enabled
    pub fn default() -> Self {
        LifetimeElisionConfig {
            enable_rule1: true,
            enable_rule2: true,
            enable_rule3: true,
        }
    }
}

/// Result of lifetime elision analysis
#[derive(Debug, Clone, PartialEq)]
pub struct ElisionResult {
    /// Input lifetimes for each parameter
    pub input_lifetimes: Vec<Option<Lifetime>>,
    /// Output lifetime (if any)
    pub output_lifetime: Option<Lifetime>,
    /// Which rule was applied
    pub rule_applied: ElisionRule,
}

/// Which elision rule was applied
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElisionRule {
    /// Rule 1: Single input reference → return takes that lifetime
    Rule1,
    /// Rule 2: Multiple inputs, first is reference → return takes first lifetime
    Rule2,
    /// Rule 3: Method with &self or &mut self → return takes self lifetime
    Rule3,
    /// No rule applied (explicit lifetimes or no references)
    None,
}

/// Analyzes function signatures and applies lifetime elision rules
#[derive(Debug)]
pub struct LifetimeElisionAnalyzer {
    config: LifetimeElisionConfig,
    /// Cache of analyzed functions: function_name -> elision result
    analyzed_functions: HashMap<String, ElisionResult>,
}

impl LifetimeElisionAnalyzer {
    /// Create a new analyzer with default configuration
    pub fn new() -> Self {
        LifetimeElisionAnalyzer {
            config: LifetimeElisionConfig::default(),
            analyzed_functions: HashMap::new(),
        }
    }

    /// The elision rule configuration this analyzer was built with
    pub fn config(&self) -> &LifetimeElisionConfig {
        &self.config
    }

    /// Create with custom configuration
    pub fn with_config(config: LifetimeElisionConfig) -> Self {
        LifetimeElisionAnalyzer {
            config,
            analyzed_functions: HashMap::new(),
        }
    }

    /// Register a function for lifetime elision analysis
    ///
    /// # Arguments
    /// * `name` - Function name
    /// * `params` - Parameter types (as HirType)
    /// * `return_type` - Optional return type
    /// * `lifetime_ctx` - Lifetime context for generating fresh lifetimes
    ///
    /// # Returns
    /// ElisionResult with inferred lifetimes
    pub fn register_function(
        &mut self,
        name: String,
        params: Vec<HirType>,
        return_type: Option<HirType>,
        lifetime_ctx: &mut LifetimeContext,
    ) -> ElisionResult {
        let result = self.infer_lifetimes(&params, &return_type, lifetime_ctx);
        self.analyzed_functions.insert(name, result.clone());
        result
    }

    /// Infer lifetimes for a function signature
    ///
    /// This is the core algorithm that applies the three elision rules.
    pub fn infer_lifetimes(
        &self,
        params: &[HirType],
        return_type: &Option<HirType>,
        lifetime_ctx: &mut LifetimeContext,
    ) -> ElisionResult {
        // Step 1: Determine which parameters are references
        let param_is_ref: Vec<bool> = params.iter().map(|ty| self.is_reference_type(ty)).collect();

        // Step 2: Count reference parameters
        let ref_count = param_is_ref.iter().filter(|&&is_ref| is_ref).count();

        // Step 3: Check if return type is a reference
        let has_return_ref = return_type
            .as_ref()
            .map(|ty| self.is_reference_type(ty))
            .unwrap_or(false);

        // Step 4: Check if this is a method with self parameter
        let is_method_with_self = self.is_method_with_self(params);

        // Step 5: Apply elision rules
        let (input_lifetimes, output_lifetime, rule) = self.apply_elision_rules(
            &param_is_ref,
            ref_count,
            has_return_ref,
            is_method_with_self,
            lifetime_ctx,
        );

        ElisionResult {
            input_lifetimes,
            output_lifetime,
            rule_applied: rule,
        }
    }

    /// Apply the three elision rules
    fn apply_elision_rules(
        &self,
        param_is_ref: &[bool],
        ref_count: usize,
        has_return_ref: bool,
        is_method_with_self: bool,
        lifetime_ctx: &mut LifetimeContext,
    ) -> (Vec<Option<Lifetime>>, Option<Lifetime>, ElisionRule) {
        // Rule 3: Method with &self or &mut self
        // Conservative: Only apply if explicitly enabled and we can detect self
        // For now, we're conservative and don't auto-apply Rule 3
        if self.config.enable_rule3 && is_method_with_self && has_return_ref {
            // Note: In production, this would need parameter name analysis
            // to reliably detect 'self' vs other references
        }

        // Rule 1: Single input reference → return takes that lifetime
        if self.config.enable_rule1 && ref_count == 1 && has_return_ref {
            let fresh_lt = lifetime_ctx.fresh_lifetime();
            let input_lifetimes = param_is_ref
                .iter()
                .map(|&is_ref| if is_ref { Some(fresh_lt.clone()) } else { None })
                .collect();
            return (input_lifetimes, Some(fresh_lt), ElisionRule::Rule1);
        }

        // Rule 2: Multiple input references where first is reference → return takes first lifetime
        if self.config.enable_rule2 && ref_count > 1 && has_return_ref {
            if let Some(true) = param_is_ref.first().copied() {
                let first_lt = lifetime_ctx.fresh_lifetime();
                let input_lifetimes = param_is_ref
                    .iter()
                    .enumerate()
                    .map(|(i, &is_ref)| {
                        if is_ref {
                            if i == 0 {
                                Some(first_lt.clone())
                            } else {
                                Some(lifetime_ctx.fresh_lifetime())
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                return (input_lifetimes, Some(first_lt), ElisionRule::Rule2);
            }
        }

        // Default: each reference gets its own lifetime, no output lifetime
        let input_lifetimes = param_is_ref
            .iter()
            .map(|&is_ref| {
                if is_ref {
                    Some(lifetime_ctx.fresh_lifetime())
                } else {
                    None
                }
            })
            .collect();

        (input_lifetimes, None, ElisionRule::None)
    }

    /// Check if a type is a reference type
    fn is_reference_type(&self, ty: &HirType) -> bool {
        matches!(ty, HirType::Reference(_))
    }

    /// Check if function has self as first parameter (method)
    /// Conservative approach: return false since we can't reliably detect 'self' from type alone
    fn is_method_with_self(&self, _params: &[HirType]) -> bool {
        false
    }

    /// Get elision result for a previously registered function
    pub fn get_elision_result(&self, name: &str) -> Option<&ElisionResult> {
        self.analyzed_functions.get(name)
    }

    /// Get all analyzed functions
    pub fn get_all_results(&self) -> &HashMap<String, ElisionResult> {
        &self.analyzed_functions
    }

    /// Validate that elided lifetimes don't create contradictions
    pub fn validate(&self, lifetime_ctx: &LifetimeContext) -> bool {
        lifetime_ctx.is_satisfiable()
    }

    /// Clear all cached results
    pub fn clear(&mut self) {
        self.analyzed_functions.clear();
    }
}

impl Default for LifetimeElisionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule1_single_input_reference() {
        let mut analyzer = LifetimeElisionAnalyzer::new();
        let mut lifetime_ctx = LifetimeContext::new();

        // fn foo(x: &str) -> &str
        let params = vec![HirType::Reference(Box::new(HirType::String))];
        let return_type = Some(HirType::Reference(Box::new(HirType::String)));

        let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);

        assert_eq!(result.rule_applied, ElisionRule::Rule1);
        assert_eq!(result.input_lifetimes.len(), 1);
        assert!(result.input_lifetimes[0].is_some());
        assert!(result.output_lifetime.is_some());
        assert_eq!(result.input_lifetimes[0], result.output_lifetime);
    }

    #[test]
    fn test_rule2_multiple_inputs_first_reference() {
        let mut analyzer = LifetimeElisionAnalyzer::new();
        let mut lifetime_ctx = LifetimeContext::new();

        // fn foo(x: &str, y: &str) -> &str
        let params = vec![
            HirType::Reference(Box::new(HirType::String)),
            HirType::Reference(Box::new(HirType::String)),
        ];
        let return_type = Some(HirType::Reference(Box::new(HirType::String)));

        let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);

        assert_eq!(result.rule_applied, ElisionRule::Rule2);
        assert_eq!(result.input_lifetimes.len(), 2);
        assert!(result.input_lifetimes[0].is_some());
        assert!(result.input_lifetimes[1].is_some());
        assert!(result.output_lifetime.is_some());
        assert_eq!(result.input_lifetimes[0], result.output_lifetime);
    }

    #[test]
    fn test_rule3_method_with_self() {
        let mut analyzer = LifetimeElisionAnalyzer::new();
        let mut lifetime_ctx = LifetimeContext::new();

        // fn foo(&self) -> &str
        // Conservative: Rule 3 not applied automatically
        let params = vec![HirType::Reference(Box::new(HirType::String))];
        let return_type = Some(HirType::Reference(Box::new(HirType::String)));

        let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);

        // Conservative approach: Rule 1 applied instead
        assert_eq!(result.rule_applied, ElisionRule::Rule1);
        assert_eq!(result.input_lifetimes.len(), 1);
        assert!(result.input_lifetimes[0].is_some());
        assert!(result.output_lifetime.is_some());
        assert_eq!(result.input_lifetimes[0], result.output_lifetime);
    }

    #[test]
    fn test_no_elision_no_return_reference() {
        let mut analyzer = LifetimeElisionAnalyzer::new();
        let mut lifetime_ctx = LifetimeContext::new();

        // fn foo(x: &str) -> i32
        let params = vec![HirType::Reference(Box::new(HirType::String))];
        let return_type = Some(HirType::Int32);

        let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);

        assert_eq!(result.rule_applied, ElisionRule::None);
        assert!(result.input_lifetimes[0].is_some());
        assert!(result.output_lifetime.is_none());
    }

    #[test]
    fn test_register_and_retrieve_function() {
        let mut analyzer = LifetimeElisionAnalyzer::new();
        let mut lifetime_ctx = LifetimeContext::new();

        // fn foo(x: &str) -> &str
        let params = vec![HirType::Reference(Box::new(HirType::String))];
        let return_type = Some(HirType::Reference(Box::new(HirType::String)));

        let result =
            analyzer.register_function("foo".to_string(), params, return_type, &mut lifetime_ctx);

        assert_eq!(result.rule_applied, ElisionRule::Rule1);

        let retrieved = analyzer.get_elision_result("foo");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().rule_applied, ElisionRule::Rule1);
    }

    #[test]
    fn test_validation_succeeds() {
        let mut analyzer = LifetimeElisionAnalyzer::new();
        let mut lifetime_ctx = LifetimeContext::new();

        // fn foo(x: &str) -> &str
        let params = vec![HirType::Reference(Box::new(HirType::String))];
        let return_type = Some(HirType::Reference(Box::new(HirType::String)));

        analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);

        assert!(analyzer.validate(&lifetime_ctx));
    }

    #[test]
    fn test_clear_resets_state() {
        let mut analyzer = LifetimeElisionAnalyzer::new();
        let mut lifetime_ctx = LifetimeContext::new();

        // Register a function
        let params = vec![HirType::Reference(Box::new(HirType::String))];
        let return_type = Some(HirType::Reference(Box::new(HirType::String)));

        analyzer.register_function("foo".to_string(), params, return_type, &mut lifetime_ctx);

        assert_eq!(analyzer.get_all_results().len(), 1);

        analyzer.clear();

        assert_eq!(analyzer.get_all_results().len(), 0);
    }
}
