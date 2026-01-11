//! # Type Constraint Validation System
//!
//! Validates and verifies type constraints across the type system.
//!
//! Features:
//! - Generic constraint verification
//! - Lifetime constraint validation  
//! - Trait bound checking
//! - Associated type constraint resolution
//! - Constraint satisfaction reporting

use std::collections::HashMap;

/// Type of constraint to validate
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintType {
    /// Generic type parameter constraint: T: Display
    TraitBound,
    /// Lifetime constraint: 'a: 'b
    LifetimeOutlives,
    /// Associated type constraint: T::Item = i32
    AssociatedType,
    /// Equality constraint: T = i32
    TypeEquality,
}

/// A single constraint to be validated
#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub constraint: String,
    pub satisfied: bool,
    pub location: Option<usize>,
}

impl TypeConstraint {
    pub fn new(name: String, constraint_type: ConstraintType, constraint: String) -> Self {
        TypeConstraint {
            name,
            constraint_type,
            constraint,
            satisfied: false,
            location: None,
        }
    }

    pub fn with_location(mut self, location: usize) -> Self {
        self.location = Some(location);
        self
    }
}

/// Configuration for constraint validation
#[derive(Debug, Clone)]
pub struct ConstraintValidationConfig {
    /// Maximum constraints to track
    pub max_constraints: usize,
    /// Whether to check transitive constraints
    pub check_transitive: bool,
    /// Whether to perform deep equality checks
    pub deep_equality_check: bool,
    /// Maximum constraint depth
    pub max_depth: usize,
}

impl Default for ConstraintValidationConfig {
    fn default() -> Self {
        ConstraintValidationConfig {
            max_constraints: 1000,
            check_transitive: true,
            deep_equality_check: true,
            max_depth: 16,
        }
    }
}

/// Constraint validation engine
pub struct ConstraintValidator {
    config: ConstraintValidationConfig,
    constraints: HashMap<String, Vec<TypeConstraint>>,
    satisfied_count: usize,
    failed_count: usize,
}

impl ConstraintValidator {
    pub fn new(config: ConstraintValidationConfig) -> Self {
        ConstraintValidator {
            config,
            constraints: HashMap::new(),
            satisfied_count: 0,
            failed_count: 0,
        }
    }

    /// Register a constraint for validation
    pub fn register_constraint(&mut self, constraint: TypeConstraint) -> Result<(), String> {
        if self.constraints.values().flatten().count() >= self.config.max_constraints {
            return Err("maximum constraints reached".to_string());
        }

        self.constraints
            .entry(constraint.name.clone())
            .or_insert_with(Vec::new)
            .push(constraint);

        Ok(())
    }

    /// Validate a trait bound constraint
    pub fn validate_trait_bound(&mut self, param: &str, trait_name: &str) -> Result<bool, String> {
        let constraint = TypeConstraint::new(
            param.to_string(),
            ConstraintType::TraitBound,
            trait_name.to_string(),
        );

        self.register_constraint(constraint.clone())?;

        // Simple validation: check if trait is known
        let is_valid = matches!(
            trait_name,
            "Clone" | "Debug" | "Display" | "PartialEq" | "Eq" | "Hash" | "Default"
                | "Iterator" | "IntoIterator" | "From" | "Into" | "AsRef" | "Borrow"
                | "Drop" | "Sized" | "Send" | "Sync" | "Unpin"
        );

        if is_valid {
            self.satisfied_count += 1;
        } else {
            self.failed_count += 1;
        }

        Ok(is_valid)
    }

    /// Validate a lifetime constraint
    pub fn validate_lifetime_constraint(&mut self, lifetime1: &str, lifetime2: &str) -> Result<bool, String> {
        let constraint = TypeConstraint::new(
            lifetime1.to_string(),
            ConstraintType::LifetimeOutlives,
            format!("outlives {}", lifetime2),
        );

        self.register_constraint(constraint)?;

        // Lifetimes are valid if they follow naming conventions
        let is_valid = lifetime1.starts_with('\'') && lifetime2.starts_with('\'');

        if is_valid {
            self.satisfied_count += 1;
        } else {
            self.failed_count += 1;
        }

        Ok(is_valid)
    }

    /// Validate an associated type constraint
    pub fn validate_associated_type(
        &mut self,
        trait_name: &str,
        assoc_type: &str,
        concrete_type: &str,
    ) -> Result<bool, String> {
        let constraint = TypeConstraint::new(
            format!("{}::{}", trait_name, assoc_type),
            ConstraintType::AssociatedType,
            concrete_type.to_string(),
        );

        self.register_constraint(constraint)?;

        // Associated types are valid if the trait is known and type is non-empty
        let is_valid = !concrete_type.is_empty()
            && matches!(
                trait_name,
                "Iterator" | "IntoIterator" | "From" | "Add" | "Mul" | "Deref"
            );

        if is_valid {
            self.satisfied_count += 1;
        } else {
            self.failed_count += 1;
        }

        Ok(is_valid)
    }

    /// Get all constraints for a specific type
    pub fn get_constraints(&self, name: &str) -> Option<&[TypeConstraint]> {
        self.constraints.get(name).map(|v| v.as_slice())
    }

    /// Check if all constraints are satisfied
    pub fn all_satisfied(&self) -> bool {
        self.failed_count == 0 && self.satisfied_count > 0
    }

    /// Get satisfaction report
    pub fn get_report(&self) -> ConstraintValidationReport {
        let total = self.satisfied_count + self.failed_count;
        let satisfaction_rate = if total > 0 {
            (self.satisfied_count as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        ConstraintValidationReport {
            total_constraints: total,
            satisfied_constraints: self.satisfied_count,
            failed_constraints: self.failed_count,
            satisfaction_rate,
            constraint_count: self.constraints.len(),
        }
    }

    /// Clear all constraints
    pub fn clear(&mut self) {
        self.constraints.clear();
        self.satisfied_count = 0;
        self.failed_count = 0;
    }
}

/// Constraint validation report
#[derive(Debug, Clone)]
pub struct ConstraintValidationReport {
    pub total_constraints: usize,
    pub satisfied_constraints: usize,
    pub failed_constraints: usize,
    pub satisfaction_rate: f64,
    pub constraint_count: usize,
}

impl std::fmt::Display for ConstraintValidationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Constraint Validation Report:\n\
             Total constraints: {}\n\
             Satisfied: {} ({:.1}%)\n\
             Failed: {}\n\
             Unique types constrained: {}",
            self.total_constraints,
            self.satisfied_constraints,
            self.satisfaction_rate,
            self.failed_constraints,
            self.constraint_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_constraint() {
        let constraint = TypeConstraint::new(
            "T".to_string(),
            ConstraintType::TraitBound,
            "Clone".to_string(),
        );

        assert_eq!(constraint.name, "T");
        assert_eq!(constraint.constraint_type, ConstraintType::TraitBound);
        assert!(!constraint.satisfied);
    }

    #[test]
    fn test_constraint_with_location() {
        let constraint = TypeConstraint::new(
            "T".to_string(),
            ConstraintType::TraitBound,
            "Clone".to_string(),
        )
        .with_location(42);

        assert_eq!(constraint.location, Some(42));
    }

    #[test]
    fn test_validate_trait_bound_valid() {
        let config = ConstraintValidationConfig::default();
        let mut validator = ConstraintValidator::new(config);

        let result = validator.validate_trait_bound("T", "Clone");
        assert!(result.unwrap());
    }

    #[test]
    fn test_validate_trait_bound_invalid() {
        let config = ConstraintValidationConfig::default();
        let mut validator = ConstraintValidator::new(config);

        let result = validator.validate_trait_bound("T", "UnknownTrait");
        assert!(!result.unwrap());
    }

    #[test]
    fn test_validate_lifetime_constraint() {
        let config = ConstraintValidationConfig::default();
        let mut validator = ConstraintValidator::new(config);

        let result = validator.validate_lifetime_constraint("'a", "'b");
        assert!(result.unwrap());
    }

    #[test]
    fn test_validate_associated_type() {
        let config = ConstraintValidationConfig::default();
        let mut validator = ConstraintValidator::new(config);

        let result = validator.validate_associated_type("Iterator", "Item", "i32");
        assert!(result.unwrap());
    }

    #[test]
    fn test_multiple_constraints() {
        let config = ConstraintValidationConfig::default();
        let mut validator = ConstraintValidator::new(config);

        validator.validate_trait_bound("T", "Clone").ok();
        validator.validate_trait_bound("T", "Debug").ok();
        validator.validate_trait_bound("U", "Display").ok();

        let report = validator.get_report();
        assert_eq!(report.total_constraints, 3);
        assert_eq!(report.satisfied_constraints, 3);
    }

    #[test]
    fn test_all_satisfied() {
        let config = ConstraintValidationConfig::default();
        let mut validator = ConstraintValidator::new(config);

        validator.validate_trait_bound("T", "Clone").ok();
        validator.validate_trait_bound("T", "Debug").ok();

        assert!(validator.all_satisfied());
    }

    #[test]
    fn test_not_all_satisfied() {
        let config = ConstraintValidationConfig::default();
        let mut validator = ConstraintValidator::new(config);

        validator.validate_trait_bound("T", "Clone").ok();
        validator.validate_trait_bound("T", "UnknownTrait").ok();

        assert!(!validator.all_satisfied());
    }

    #[test]
    fn test_get_report() {
        let config = ConstraintValidationConfig::default();
        let mut validator = ConstraintValidator::new(config);

        validator.validate_trait_bound("T", "Clone").ok();
        validator.validate_trait_bound("U", "Debug").ok();

        let report = validator.get_report();
        assert_eq!(report.total_constraints, 2);
        assert_eq!(report.constraint_count, 2);
        assert_eq!(report.satisfaction_rate, 100.0);
    }

    #[test]
    fn test_clear() {
        let config = ConstraintValidationConfig::default();
        let mut validator = ConstraintValidator::new(config);

        validator.validate_trait_bound("T", "Clone").ok();
        validator.clear();

        let report = validator.get_report();
        assert_eq!(report.total_constraints, 0);
    }

    #[test]
    fn test_max_constraints() {
        let config = ConstraintValidationConfig {
            max_constraints: 2,
            ..Default::default()
        };
        let mut validator = ConstraintValidator::new(config);

        let c1 = TypeConstraint::new("T".to_string(), ConstraintType::TraitBound, "Clone".to_string());
        let c2 = TypeConstraint::new("U".to_string(), ConstraintType::TraitBound, "Debug".to_string());
        let c3 = TypeConstraint::new("V".to_string(), ConstraintType::TraitBound, "Default".to_string());

        assert!(validator.register_constraint(c1).is_ok());
        assert!(validator.register_constraint(c2).is_ok());
        assert!(validator.register_constraint(c3).is_err());
    }
}
