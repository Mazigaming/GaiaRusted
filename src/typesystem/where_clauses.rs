//! # Where Clause Support
//!
//! Support for complex trait bounds and constraints.
//!
//! This module provides:
//! - Where clause parsing and validation
//! - Trait bound constraints
//! - Associated type constraints
//! - Lifetime constraints
//! - Generic parameter validation
//!
//! # Examples
//!
//! ```ignore
//! use gaiarusted::typesystem::where_clauses::{WhereClauseAnalyzer, WhereClauseConfig};
//!
//! let config = WhereClauseConfig::default();
//! let mut analyzer = WhereClauseAnalyzer::new(config);
//!
//! // Register a where clause
//! analyzer.register_constraint("T", "Clone")?;
//! analyzer.register_constraint("U", "Debug")?;
//!
//! // Validate constraints
//! let report = analyzer.validate_constraints()?;
//! ```

use std::collections::HashMap;

/// Configuration for where clause analysis
#[derive(Debug, Clone)]
pub struct WhereClauseConfig {
    /// Maximum number of constraints per type
    pub max_constraints_per_type: usize,
    /// Maximum nesting depth for complex bounds
    pub max_nesting_depth: usize,
    /// Enable associated type constraints
    pub enable_associated_types: bool,
    /// Enable lifetime constraints
    pub enable_lifetime_constraints: bool,
}

impl Default for WhereClauseConfig {
    fn default() -> Self {
        WhereClauseConfig {
            max_constraints_per_type: 16,
            max_nesting_depth: 8,
            enable_associated_types: true,
            enable_lifetime_constraints: true,
        }
    }
}

/// A single trait bound constraint
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitBound {
    /// Type being constrained
    pub type_name: String,
    /// Trait name
    pub trait_name: String,
    /// Is this bound required
    pub required: bool,
}

impl TraitBound {
    fn new(type_name: String, trait_name: String) -> Self {
        TraitBound {
            type_name,
            trait_name,
            required: true,
        }
    }
}

/// An associated type constraint
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociatedTypeConstraint {
    /// Type being constrained
    pub type_name: String,
    /// Associated type name
    pub assoc_type: String,
    /// Bound type
    pub bound_to: String,
}

/// A lifetime constraint
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifetimeConstraint {
    /// Lifetime being constrained
    pub lifetime: String,
    /// Lower bound lifetime
    pub lower_bound: Option<String>,
    /// Upper bound lifetime
    pub upper_bound: Option<String>,
}

/// Where clause information
#[derive(Debug, Clone)]
pub struct WhereClause {
    /// Trait bounds
    pub trait_bounds: Vec<TraitBound>,
    /// Associated type constraints
    pub assoc_constraints: Vec<AssociatedTypeConstraint>,
    /// Lifetime constraints
    pub lifetime_constraints: Vec<LifetimeConstraint>,
}

impl WhereClause {
    fn new() -> Self {
        WhereClause {
            trait_bounds: Vec::new(),
            assoc_constraints: Vec::new(),
            lifetime_constraints: Vec::new(),
        }
    }
}

/// Main where clause analyzer
pub struct WhereClauseAnalyzer {
    config: WhereClauseConfig,
    constraints: HashMap<String, WhereClause>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl WhereClauseAnalyzer {
    /// Create a new analyzer
    pub fn new(config: WhereClauseConfig) -> Self {
        WhereClauseAnalyzer {
            config,
            constraints: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Register a trait bound constraint
    pub fn register_constraint(&mut self, type_name: &str, trait_name: &str) -> Result<(), String> {
        if type_name.is_empty() || trait_name.is_empty() {
            return Err("Type and trait names cannot be empty".to_string());
        }

        let clause = self
            .constraints
            .entry(type_name.to_string())
            .or_insert_with(WhereClause::new);

        if clause.trait_bounds.len() >= self.config.max_constraints_per_type {
            return Err(format!(
                "Too many constraints for type {}: {}",
                type_name,
                clause.trait_bounds.len()
            ));
        }

        let bound = TraitBound::new(type_name.to_string(), trait_name.to_string());
        clause.trait_bounds.push(bound);

        Ok(())
    }

    /// Register an associated type constraint
    pub fn register_associated_type(
        &mut self,
        type_name: &str,
        assoc_type: &str,
        bound_to: &str,
    ) -> Result<(), String> {
        // Associated type constraints are now enabled (Fix #4)
        if !self.config.enable_associated_types {
            return Err("Associated type constraints are disabled in configuration".to_string());
        }

        if type_name.is_empty() || assoc_type.is_empty() || bound_to.is_empty() {
            return Err("All names must be non-empty".to_string());
        }

        let clause = self
            .constraints
            .entry(type_name.to_string())
            .or_insert_with(WhereClause::new);

        let constraint = AssociatedTypeConstraint {
            type_name: type_name.to_string(),
            assoc_type: assoc_type.to_string(),
            bound_to: bound_to.to_string(),
        };

        clause.assoc_constraints.push(constraint);

        Ok(())
    }

    /// Register a lifetime constraint
    pub fn register_lifetime_constraint(
        &mut self,
        lifetime: &str,
        lower_bound: Option<&str>,
        upper_bound: Option<&str>,
    ) -> Result<(), String> {
        // Lifetime constraints are now enabled (Fix #4)
        if !self.config.enable_lifetime_constraints {
            return Err("Lifetime constraints are disabled in configuration".to_string());
        }

        if lifetime.is_empty() {
            return Err("Lifetime name cannot be empty".to_string());
        }

        let constraint = LifetimeConstraint {
            lifetime: lifetime.to_string(),
            lower_bound: lower_bound.map(|s| s.to_string()),
            upper_bound: upper_bound.map(|s| s.to_string()),
        };

        // Store in a generic constraint for tracking
        self.constraints
            .entry(lifetime.to_string())
            .or_insert_with(WhereClause::new)
            .lifetime_constraints
            .push(constraint);

        Ok(())
    }

    /// Check if a type has constraints
    pub fn has_constraints(&self, type_name: &str) -> bool {
        self.constraints.contains_key(type_name)
    }

    /// Get trait bounds for a type
    pub fn get_trait_bounds(&self, type_name: &str) -> Option<Vec<String>> {
        self.constraints.get(type_name).map(|clause| {
            clause
                .trait_bounds
                .iter()
                .map(|b| b.trait_name.clone())
                .collect()
        })
    }

    /// Get associated type constraints for a type
    pub fn get_assoc_constraints(
        &self,
        type_name: &str,
    ) -> Option<Vec<(String, String)>> {
        self.constraints.get(type_name).map(|clause| {
            clause
                .assoc_constraints
                .iter()
                .map(|c| (c.assoc_type.clone(), c.bound_to.clone()))
                .collect()
        })
    }
    
    /// Check if an associated type name is valid (Fix #4)
    fn is_valid_associated_type(&self, assoc_type: &str) -> bool {
        // Common standard library associated types
        let valid_assoc_types = &[
            "Item",      // Iterator, etc.
            "Output",    // Fn
            "Target",    // Deref
            "Err",       // Try
            "Ok",        // Try (alias)
            "IntoIter",  // IntoIterator
            "Error",     // FromStr, TryFrom
        ];
        
        // Allow any uppercase-starting identifier as a potential associated type
        // This is permissive for v0.13.0; can be made stricter later
        valid_assoc_types.contains(&assoc_type) || 
        assoc_type.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
    }

    /// Validate all constraints for consistency (Fix #4)
    pub fn validate_constraints(&self) -> Result<WhereClauseReport, String> {
        let mut report = WhereClauseReport {
            constraint_count: self.constraints.len(),
            trait_bound_count: 0,
            assoc_constraint_count: 0,
            lifetime_constraint_count: 0,
            errors: Vec::new(),
        };

        for (type_name, clause) in &self.constraints {
            // Validate trait bounds
            for bound in &clause.trait_bounds {
                report.trait_bound_count += 1;
                // Check that trait names are valid (non-empty, not reserved)
                if bound.trait_name.is_empty() {
                    report.errors.push(
                        format!("Type {} has trait bound with empty name", type_name)
                    );
                } else {
                    // Validate that the trait name is well-formed (not a reserved keyword)
                    if self.is_reserved_keyword(&bound.trait_name) {
                        report.errors.push(
                            format!("Trait name '{}' is a reserved keyword", bound.trait_name)
                        );
                    }
                }
            }
            
            // Validate associated type constraints (NOW ENABLED - Fix #4)
            for assoc in &clause.assoc_constraints {
                report.assoc_constraint_count += 1;
                
                // Verify associated type constraint is well-formed
                if assoc.assoc_type.is_empty() {
                    report.errors.push(
                        format!("Type {} has associated type constraint with empty type name", type_name)
                    );
                }
                if assoc.bound_to.is_empty() {
                    report.errors.push(
                        format!("Associated type constraint for {} has empty bound", assoc.assoc_type)
                    );
                }
                
                // Validate that associated type projections are valid
                // For T::Item: Clone, we check that Item is a valid associated type name
                if !self.is_valid_associated_type(&assoc.assoc_type) {
                    report.errors.push(
                        format!("Associated type '{}' is not a recognized associated type", assoc.assoc_type)
                    );
                }
                
                // Validate the projection structure (T::Item where T should be a type parameter or path)
                if !self.is_valid_type_parameter(&assoc.type_name) {
                    report.errors.push(
                        format!("Associated type projection for '{}' uses invalid base type", assoc.type_name)
                    );
                }
            }
            
            // Validate lifetime constraints (NOW ENABLED - Fix #4)
            for lifetime in &clause.lifetime_constraints {
                report.lifetime_constraint_count += 1;
                
                // Check that lifetime names are valid
                if lifetime.lifetime.is_empty() {
                    report.errors.push("Lifetime constraint with empty lifetime name".to_string());
                } else if !lifetime.lifetime.starts_with('\'') {
                    report.errors.push(
                        format!("Lifetime '{}' should start with apostrophe", lifetime.lifetime)
                    );
                }
                
                // Validate lifetime outlives relationships
                // 'a: 'b means 'a must outlive 'b (or be equal)
                if let (Some(lower), Some(upper)) = (&lifetime.lower_bound, &lifetime.upper_bound) {
                    // Validate that we have reasonable lifetime names
                    if lower.is_empty() || upper.is_empty() {
                        report.errors.push(
                            format!("Lifetime outlives constraint for {} has empty bounds", lifetime.lifetime)
                        );
                    } else if !lower.starts_with('\'') || !upper.starts_with('\'') {
                        report.errors.push(
                            format!("Lifetime bounds for {} should start with apostrophe", lifetime.lifetime)
                        );
                    } else {
                        // Validate that the lifetime ordering makes sense
                        // (lower_bound outlives upper_bound, so lower_bound >= upper_bound in terms of scope)
                        if lower == upper {
                            report.errors.push(
                                format!("Lifetime constraint {} should have different bounds", lifetime.lifetime)
                            );
                        }
                    }
                }
            }
        }

        Ok(report)
    }
    
    /// Check if a keyword is reserved in the trait namespace
    fn is_reserved_keyword(&self, name: &str) -> bool {
        matches!(name, "Self" | "type" | "impl" | "trait" | "fn" | "let" | "mut" | "const")
    }
    
    /// Check if a type parameter name is valid
    fn is_valid_type_parameter(&self, name: &str) -> bool {
        // Type parameters are typically single uppercase letters or identifiers starting with uppercase
        // or they could be concrete type names
        if name.is_empty() {
            return false;
        }
        
        let first_char = name.chars().next().unwrap();
        // Allow uppercase letters (type params), lowercase (in contexts like trait bounds), or numbers for special cases
        first_char.is_alphanumeric() && !first_char.is_numeric()
    }

    /// Generate analysis report
    pub fn generate_report(&self) -> WhereClauseAnalysisReport {
        WhereClauseAnalysisReport {
            total_constraints: self.constraints.len(),
            total_bounds: self.constraints.values().map(|c| c.trait_bounds.len()).sum(),
            total_assoc: self
                .constraints
                .values()
                .map(|c| c.assoc_constraints.len())
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

/// Report from where clause validation
#[derive(Debug, Clone)]
pub struct WhereClauseReport {
    /// Number of constrained types
    pub constraint_count: usize,
    /// Number of trait bounds
    pub trait_bound_count: usize,
    /// Number of associated type constraints
    pub assoc_constraint_count: usize,
    /// Number of lifetime constraints
    pub lifetime_constraint_count: usize,
    /// Error messages
    pub errors: Vec<String>,
}

/// Analysis report
#[derive(Debug, Clone)]
pub struct WhereClauseAnalysisReport {
    /// Total number of constrained types
    pub total_constraints: usize,
    /// Total trait bounds
    pub total_bounds: usize,
    /// Total associated constraints
    pub total_assoc: usize,
    /// Error messages
    pub errors: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_analyzer() -> WhereClauseAnalyzer {
        WhereClauseAnalyzer::new(WhereClauseConfig::default())
    }

    #[test]
    fn test_create_analyzer() {
        let analyzer = create_test_analyzer();
        assert_eq!(analyzer.constraints.len(), 0);
    }

    #[test]
    fn test_register_constraint() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_constraint("T", "Clone");
        assert!(result.is_ok());
        assert!(analyzer.has_constraints("T"));
    }

    #[test]
    fn test_register_empty_constraint() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_constraint("", "Clone");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_bounds_same_type() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_constraint("T", "Clone").ok();
        analyzer.register_constraint("T", "Debug").ok();
        assert_eq!(analyzer.get_trait_bounds("T"), Some(vec!["Clone".to_string(), "Debug".to_string()]));
    }

    #[test]
    fn test_register_associated_type() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_associated_type("T", "Item", "String");
        assert!(result.is_ok());
    }

    #[test]
    fn test_associated_type_disabled() {
        let config = WhereClauseConfig {
            enable_associated_types: false,
            ..Default::default()
        };
        let mut analyzer = WhereClauseAnalyzer::new(config);
        let result = analyzer.register_associated_type("T", "Item", "String");
        assert!(result.is_err());
    }

    #[test]
    fn test_register_lifetime_constraint() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_lifetime_constraint("'a", None, Some("'b"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_lifetime_constraint_disabled() {
        let config = WhereClauseConfig {
            enable_lifetime_constraints: false,
            ..Default::default()
        };
        let mut analyzer = WhereClauseAnalyzer::new(config);
        let result = analyzer.register_lifetime_constraint("'a", None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_constraints() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_constraint("T", "Clone").ok();
        analyzer.register_constraint("U", "Debug").ok();
        let report = analyzer.validate_constraints().unwrap();
        assert_eq!(report.constraint_count, 2);
    }

    #[test]
    fn test_generate_report() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_constraint("T", "Clone").ok();
        let report = analyzer.generate_report();
        assert_eq!(report.total_constraints, 1);
    }

    #[test]
    fn test_add_error() {
        let mut analyzer = create_test_analyzer();
        analyzer.add_error("test error".to_string());
        assert_eq!(analyzer.errors.len(), 1);
    }

    #[test]
    fn test_has_constraints() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_constraint("T", "Clone").ok();
        assert!(analyzer.has_constraints("T"));
        assert!(!analyzer.has_constraints("U"));
    }

    #[test]
    fn test_max_constraints() {
       let config = WhereClauseConfig {
           max_constraints_per_type: 2,
           ..Default::default()
       };
       let mut analyzer = WhereClauseAnalyzer::new(config);
       analyzer.register_constraint("T", "Clone").ok();
       analyzer.register_constraint("T", "Debug").ok();
       let result = analyzer.register_constraint("T", "Default");
       assert!(result.is_err());
    }
    
    #[test]
    fn test_validate_associated_type_constraint() {
       let mut analyzer = create_test_analyzer();
       let result = analyzer.register_associated_type("T", "Item", "i32");
       assert!(result.is_ok());
       
       let report = analyzer.validate_constraints().unwrap();
       assert_eq!(report.assoc_constraint_count, 1);
       assert!(report.errors.is_empty());
    }
    
    #[test]
    fn test_validate_empty_associated_type() {
       let mut analyzer = create_test_analyzer();
       let result = analyzer.register_associated_type("T", "", "i32");
       assert!(result.is_err());
    }
    
    #[test]
    fn test_validate_lifetime_constraint_valid() {
       let mut analyzer = create_test_analyzer();
       let result = analyzer.register_lifetime_constraint("'a", None, Some("'b"));
       assert!(result.is_ok());
       
       let report = analyzer.validate_constraints().unwrap();
       assert_eq!(report.lifetime_constraint_count, 1);
    }
    
    #[test]
    fn test_validate_lifetime_constraint_invalid_format() {
       let mut analyzer = create_test_analyzer();
       let result = analyzer.register_lifetime_constraint("a", None, Some("b"));
       assert!(result.is_ok());  // Registration succeeds, but validation will catch it
       
       let report = analyzer.validate_constraints().unwrap();
       assert!(!report.errors.is_empty());  // Should have validation errors
       assert!(report.errors.iter().any(|e| e.contains("apostrophe")));
    }
    
    #[test]
    fn test_validate_lifetime_outlives_relationship() {
       let mut analyzer = create_test_analyzer();
       // Register with same bounds - should trigger error
       analyzer.register_lifetime_constraint("'a", Some("'b"), Some("'b")).ok();
       
       let report = analyzer.validate_constraints().unwrap();
       // Should error because lower_bound == upper_bound (both 'b)
       assert!(report.errors.iter().any(|e| e.contains("different bounds")));
    }
    
    #[test]
    fn test_validate_trait_bound_reserved_keyword() {
       let mut analyzer = create_test_analyzer();
       analyzer.register_constraint("T", "Clone").ok();
       
       // Try to add a reserved keyword as trait
       analyzer.register_constraint("U", "impl").ok();
       
       let report = analyzer.validate_constraints().unwrap();
       // Should have error for reserved keyword
       assert!(report.errors.iter().any(|e| e.contains("reserved")));
    }
    
    #[test]
    fn test_validate_associated_type_recognized() {
       let mut analyzer = create_test_analyzer();
       
       // Register valid associated types
       analyzer.register_associated_type("T", "Item", "i32").ok();
       analyzer.register_associated_type("T", "Output", "bool").ok();
       analyzer.register_associated_type("T", "Target", "String").ok();
       
       let report = analyzer.validate_constraints().unwrap();
       assert_eq!(report.assoc_constraint_count, 3);
       assert!(report.errors.is_empty());
    }
    
    #[test]
    fn test_validate_complex_where_clause() {
       let mut analyzer = create_test_analyzer();
       
       // Register multiple constraint types
       analyzer.register_constraint("T", "Clone").ok();
       analyzer.register_constraint("T", "Debug").ok();
       analyzer.register_associated_type("T", "Item", "Display").ok();
       analyzer.register_lifetime_constraint("'a", Some("'a"), Some("'b")).ok();
       
       let report = analyzer.validate_constraints().unwrap();
       assert_eq!(report.constraint_count, 2);  // T and 'a
       assert_eq!(report.trait_bound_count, 2);
       assert_eq!(report.assoc_constraint_count, 1);
       assert_eq!(report.lifetime_constraint_count, 1);
    }
    
    #[test]
    fn test_is_valid_associated_type() {
        let analyzer = create_test_analyzer();
        
        // Standard library associated types should be recognized
        assert!(analyzer.is_valid_associated_type("Item"));
        assert!(analyzer.is_valid_associated_type("Output"));
        assert!(analyzer.is_valid_associated_type("Target"));
        assert!(analyzer.is_valid_associated_type("Error"));
        
        // Uppercase custom types should also be allowed
        assert!(analyzer.is_valid_associated_type("CustomType"));
        
        // Lowercase shouldn't be allowed
        assert!(!analyzer.is_valid_associated_type("item"));
    }
    }
