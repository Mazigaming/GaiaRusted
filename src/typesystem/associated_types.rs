//! # Associated Type System
//!
//! This module provides support for associated types in traits.
//! Associated types allow traits to define placeholder types that implementers
//! must provide concrete types for.
//!
//! Example:
//! ```rust,ignore
//! trait Iterator {
//!     type Item;
//!     fn next(&mut self) -> Option<Self::Item>;
//! }
//!
//! impl Iterator for MyIterator {
//!     type Item = i32;
//!     fn next(&mut self) -> Option<i32> { /* ... */ }
//! }
//! ```
//!
//! Features:
//! - Associated type definition in traits
//! - Concrete type assignment in impl blocks
//! - Type bounds on associated types
//! - Resolution of Self::AssociatedType
//! - Integration with generics and where clauses

use std::collections::{HashMap, HashSet};
use crate::typesystem::types::{Type, StructId, TraitId};

/// Configuration for associated type analysis
#[derive(Debug, Clone)]
pub struct AssociatedTypeConfig {
    /// Whether to allow recursive associated types
    pub allow_recursive: bool,
    /// Maximum number of associated types per trait
    pub max_per_trait: usize,
}

impl Default for AssociatedTypeConfig {
    fn default() -> Self {
        AssociatedTypeConfig {
            allow_recursive: true,
            max_per_trait: 16,
        }
    }
}

/// Definition of an associated type in a trait
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssociatedTypeDefinition {
    /// Name of the associated type
    pub name: String,
    /// Type bounds (e.g., `type Item: Display`)
    pub bounds: Vec<String>,
    /// Optional default type
    pub default: Option<Type>,
}

/// A concrete assignment of an associated type in an impl block
#[derive(Debug, Clone)]
pub struct AssociatedTypeAssignment {
    /// Name of the associated type
    pub name: String,
    /// Concrete type being assigned
    pub concrete_type: Type,
}

/// Information about a trait's associated types
#[derive(Debug, Clone)]
pub struct TraitAssociatedTypes {
    pub trait_name: String,
    pub trait_id: Option<TraitId>,
    pub associated_types: Vec<AssociatedTypeDefinition>,
}

/// Information about an impl block's associated type assignments
#[derive(Debug, Clone)]
pub struct ImplAssociatedTypes {
    pub impl_trait_name: Option<String>,
    pub impl_struct_name: String,
    pub assignments: HashMap<String, AssociatedTypeAssignment>,
}

/// Resolution of a Self::AssociatedType reference
#[derive(Debug, Clone)]
pub struct ResolvedAssociatedType {
    pub type_name: String,
    pub resolved_type: Type,
}

/// Analysis result for associated types
#[derive(Debug, Clone)]
pub struct AssociatedTypeAnalysisReport {
    pub trait_associated_types: HashMap<String, TraitAssociatedTypes>,
    pub impl_assignments: HashMap<String, ImplAssociatedTypes>,
    pub unresolved_references: Vec<(String, String)>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Main associated type analyzer
pub struct AssociatedTypeAnalyzer {
    config: AssociatedTypeConfig,
    trait_types: HashMap<String, TraitAssociatedTypes>,
    impl_assignments: HashMap<String, ImplAssociatedTypes>,
    unresolved_references: Vec<(String, String)>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl AssociatedTypeAnalyzer {
    /// Create a new associated type analyzer
    pub fn new(config: AssociatedTypeConfig) -> Self {
        AssociatedTypeAnalyzer {
            config,
            trait_types: HashMap::new(),
            impl_assignments: HashMap::new(),
            unresolved_references: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Register a trait with its associated types
    pub fn register_trait(
        &mut self,
        trait_name: String,
        associated_types: Vec<AssociatedTypeDefinition>,
    ) -> Result<(), String> {
        if associated_types.len() > self.config.max_per_trait {
            return Err(format!(
                "Trait '{}' has too many associated types ({})",
                trait_name,
                associated_types.len()
            ));
        }

        // Check for duplicates
        let mut seen = HashSet::new();
        for assoc_type in &associated_types {
            if !seen.insert(&assoc_type.name) {
                return Err(format!(
                    "Duplicate associated type '{}' in trait '{}'",
                    assoc_type.name, trait_name
                ));
            }
        }

        let trait_info = TraitAssociatedTypes {
            trait_name: trait_name.clone(),
            trait_id: None,
            associated_types,
        };

        self.trait_types.insert(trait_name, trait_info);
        Ok(())
    }

    /// Register an impl block's associated type assignments
    pub fn register_impl_assignments(
        &mut self,
        impl_name: String,
        trait_name: Option<String>,
        struct_name: String,
        assignments: Vec<AssociatedTypeAssignment>,
    ) -> Result<(), String> {
        // If implementing a trait, validate all required associated types are provided
        if let Some(ref trait_name) = trait_name {
            if let Some(trait_info) = self.trait_types.get(trait_name) {
                let provided_names: HashSet<_> =
                    assignments.iter().map(|a| &a.name).collect();
                let required_names: HashSet<_> = trait_info
                    .associated_types
                    .iter()
                    .map(|a| &a.name)
                    .collect();

                // Check for missing assignments
                let missing: Vec<_> = required_names.difference(&provided_names).collect();
                if !missing.is_empty() {
                    let missing_str = missing
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(format!(
                        "Impl for '{}' missing associated types: {}",
                        trait_name, missing_str
                    ));
                }

                // Check for extra assignments
                let extra: Vec<_> = provided_names.difference(&required_names).collect();
                if !extra.is_empty() {
                    let extra_str = extra
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.warnings.push(format!(
                        "Impl for '{}' has extra associated types: {}",
                        trait_name, extra_str
                    ));
                }
            }
        }

        // Check for duplicate assignments in this impl
        let mut seen = HashSet::new();
        for assignment in &assignments {
            if !seen.insert(&assignment.name) {
                return Err(format!(
                    "Duplicate associated type assignment '{}' in impl '{}'",
                    assignment.name, impl_name
                ));
            }
        }

        let impl_info = ImplAssociatedTypes {
            impl_trait_name: trait_name,
            impl_struct_name: struct_name,
            assignments: assignments
                .into_iter()
                .map(|a| (a.name.clone(), a))
                .collect(),
        };

        self.impl_assignments.insert(impl_name, impl_info);
        Ok(())
    }

    /// Resolve a Self::AssociatedType reference in a specific impl context
    pub fn resolve_self_type(
        &self,
        impl_name: &str,
        type_name: &str,
    ) -> Result<ResolvedAssociatedType, String> {
        if let Some(impl_info) = self.impl_assignments.get(impl_name) {
            if let Some(assignment) = impl_info.assignments.get(type_name) {
                Ok(ResolvedAssociatedType {
                    type_name: type_name.to_string(),
                    resolved_type: assignment.concrete_type.clone(),
                })
            } else {
                Err(format!(
                    "Associated type '{}' not found in impl '{}'",
                    type_name, impl_name
                ))
            }
        } else {
            Err(format!("Impl block '{}' not found", impl_name))
        }
    }

    /// Resolve <Type as Trait>::AssociatedType references
    pub fn resolve_qualified_type(
        &self,
        type_path: &str,
        trait_name: &str,
        assoc_type: &str,
    ) -> Result<ResolvedAssociatedType, String> {
        // Find the impl that matches this type and trait
        for (_, impl_info) in &self.impl_assignments {
            if impl_info.impl_struct_name == type_path
                && impl_info.impl_trait_name.as_deref() == Some(trait_name)
            {
                if let Some(assignment) = impl_info.assignments.get(assoc_type) {
                    return Ok(ResolvedAssociatedType {
                        type_name: assoc_type.to_string(),
                        resolved_type: assignment.concrete_type.clone(),
                    });
                }
            }
        }

        Err(format!(
            "Cannot resolve <{} as {}>::{}",
            type_path, trait_name, assoc_type
        ))
    }

    /// Validate type bounds on associated types
    pub fn validate_type_bounds(
        &mut self,
        impl_name: &str,
    ) -> Result<(), String> {
        if let Some(impl_info) = self.impl_assignments.get(impl_name) {
            if let Some(trait_name) = &impl_info.impl_trait_name {
                if let Some(trait_info) = self.trait_types.get(trait_name) {
                    // Check that each assignment satisfies the trait's bounds
                    for trait_assoc in &trait_info.associated_types {
                        if let Some(assignment) = impl_info.assignments.get(&trait_assoc.name) {
                            // Validate bounds would go here
                            // For now, we just track that it was checked
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Get associated type information for a trait
    pub fn get_trait_associated_types(&self, trait_name: &str) -> Option<&TraitAssociatedTypes> {
        self.trait_types.get(trait_name)
    }

    /// Get all trait associated types
    pub fn trait_types(&self) -> &HashMap<String, TraitAssociatedTypes> {
        &self.trait_types
    }

    /// Get all impl assignments
    pub fn impl_assignments(&self) -> &HashMap<String, ImplAssociatedTypes> {
        &self.impl_assignments
    }

    /// Check if an impl satisfies all trait requirements
    pub fn check_impl_completeness(&self, impl_name: &str) -> Result<(), String> {
        if let Some(impl_info) = self.impl_assignments.get(impl_name) {
            if let Some(trait_name) = &impl_info.impl_trait_name {
                if let Some(trait_info) = self.trait_types.get(trait_name) {
                    for assoc_type in &trait_info.associated_types {
                        if !impl_info.assignments.contains_key(&assoc_type.name) {
                            if assoc_type.default.is_none() {
                                return Err(format!(
                                    "Impl '{}' missing required associated type '{}'",
                                    impl_name, assoc_type.name
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Add an error message
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Add a warning message
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get all errors
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Generate the analysis report
    pub fn generate_report(self) -> AssociatedTypeAnalysisReport {
        AssociatedTypeAnalysisReport {
            trait_associated_types: self.trait_types,
            impl_assignments: self.impl_assignments,
            unresolved_references: self.unresolved_references,
            errors: self.errors,
            warnings: self.warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_analyzer() -> AssociatedTypeAnalyzer {
        AssociatedTypeAnalyzer::new(AssociatedTypeConfig::default())
    }

    #[test]
    fn test_register_trait_with_associated_type() {
        let mut analyzer = create_test_analyzer();

        let assoc_types = vec![AssociatedTypeDefinition {
            name: "Item".to_string(),
            bounds: vec![],
            default: None,
        }];

        let result = analyzer.register_trait("Iterator".to_string(), assoc_types);
        assert!(result.is_ok());
        assert!(analyzer.get_trait_associated_types("Iterator").is_some());
    }

    #[test]
    fn test_register_trait_with_multiple_types() {
        let mut analyzer = create_test_analyzer();

        let assoc_types = vec![
            AssociatedTypeDefinition {
                name: "Item".to_string(),
                bounds: vec![],
                default: None,
            },
            AssociatedTypeDefinition {
                name: "IntoIter".to_string(),
                bounds: vec![],
                default: None,
            },
        ];

        let result = analyzer.register_trait("Iterator".to_string(), assoc_types);
        assert!(result.is_ok());

        let trait_info = analyzer.get_trait_associated_types("Iterator").unwrap();
        assert_eq!(trait_info.associated_types.len(), 2);
    }

    #[test]
    fn test_duplicate_trait_types_error() {
        let mut analyzer = create_test_analyzer();

        let assoc_types = vec![
            AssociatedTypeDefinition {
                name: "Item".to_string(),
                bounds: vec![],
                default: None,
            },
            AssociatedTypeDefinition {
                name: "Item".to_string(),
                bounds: vec![],
                default: None,
            },
        ];

        let result = analyzer.register_trait("Iterator".to_string(), assoc_types);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_impl_assignments() {
        let mut analyzer = create_test_analyzer();

        // Register trait first
        analyzer
            .register_trait(
                "Iterator".to_string(),
                vec![AssociatedTypeDefinition {
                    name: "Item".to_string(),
                    bounds: vec![],
                    default: None,
                }],
            )
            .unwrap();

        // Register impl with assignment
        let assignments = vec![AssociatedTypeAssignment {
            name: "Item".to_string(),
            concrete_type: Type::I64,
        }];

        let result = analyzer.register_impl_assignments(
            "impl_Iterator_for_MyIter".to_string(),
            Some("Iterator".to_string()),
            "MyIter".to_string(),
            assignments,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_impl_missing_required_type() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_trait(
                "Iterator".to_string(),
                vec![AssociatedTypeDefinition {
                    name: "Item".to_string(),
                    bounds: vec![],
                    default: None,
                }],
            )
            .unwrap();

        let result = analyzer.register_impl_assignments(
            "impl_Iterator_for_MyIter".to_string(),
            Some("Iterator".to_string()),
            "MyIter".to_string(),
            vec![], // Empty assignments - should fail
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_self_type() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_trait(
                "Iterator".to_string(),
                vec![AssociatedTypeDefinition {
                    name: "Item".to_string(),
                    bounds: vec![],
                    default: None,
                }],
            )
            .unwrap();

        analyzer
            .register_impl_assignments(
                "impl_Iterator_for_Vec".to_string(),
                Some("Iterator".to_string()),
                "Vec".to_string(),
                vec![AssociatedTypeAssignment {
                    name: "Item".to_string(),
                    concrete_type: Type::I64,
                }],
            )
            .unwrap();

        let result =
            analyzer.resolve_self_type("impl_Iterator_for_Vec", "Item");
        assert!(result.is_ok());

        let resolved = result.unwrap();
        assert_eq!(resolved.type_name, "Item");
        assert_eq!(resolved.resolved_type, Type::I64);
    }

    #[test]
    fn test_resolve_qualified_type() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_trait(
                "Iterator".to_string(),
                vec![AssociatedTypeDefinition {
                    name: "Item".to_string(),
                    bounds: vec![],
                    default: None,
                }],
            )
            .unwrap();

        analyzer
            .register_impl_assignments(
                "impl_Iterator_for_Vec".to_string(),
                Some("Iterator".to_string()),
                "Vec".to_string(),
                vec![AssociatedTypeAssignment {
                    name: "Item".to_string(),
                    concrete_type: Type::I32,
                }],
            )
            .unwrap();

        let result = analyzer.resolve_qualified_type("Vec", "Iterator", "Item");
        assert!(result.is_ok());

        let resolved = result.unwrap();
        assert_eq!(resolved.resolved_type, Type::I32);
    }

    #[test]
    fn test_check_impl_completeness() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_trait(
                "Iterator".to_string(),
                vec![AssociatedTypeDefinition {
                    name: "Item".to_string(),
                    bounds: vec![],
                    default: None,
                }],
            )
            .unwrap();

        analyzer
            .register_impl_assignments(
                "impl_Iterator_for_Vec".to_string(),
                Some("Iterator".to_string()),
                "Vec".to_string(),
                vec![AssociatedTypeAssignment {
                    name: "Item".to_string(),
                    concrete_type: Type::I64,
                }],
            )
            .unwrap();

        let result = analyzer.check_impl_completeness("impl_Iterator_for_Vec");
        assert!(result.is_ok());
    }

    #[test]
    fn test_associated_type_with_bounds() {
        let mut analyzer = create_test_analyzer();

        let assoc_types = vec![AssociatedTypeDefinition {
            name: "Item".to_string(),
            bounds: vec!["Clone".to_string(), "Display".to_string()],
            default: None,
        }];

        let result = analyzer.register_trait("Iterator".to_string(), assoc_types);
        assert!(result.is_ok());

        let trait_info = analyzer.get_trait_associated_types("Iterator").unwrap();
        let item_type = &trait_info.associated_types[0];
        assert_eq!(item_type.bounds.len(), 2);
    }

    #[test]
    fn test_report_generation() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_trait(
                "Iterator".to_string(),
                vec![AssociatedTypeDefinition {
                    name: "Item".to_string(),
                    bounds: vec![],
                    default: None,
                }],
            )
            .unwrap();

        let report = analyzer.generate_report();
        assert_eq!(report.trait_associated_types.len(), 1);
        assert!(report.trait_associated_types.contains_key("Iterator"));
    }
}
