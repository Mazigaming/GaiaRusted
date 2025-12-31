//! # Trait Bound Extraction (Phase 5A)
//!
//! Extracts trait bounds from type declarations and validates type compliance.
//!
//! This module analyzes generic type parameter bounds declared in:
//! - Generic function parameters: `fn foo<T: Clone + Debug>(x: T)`
//! - Struct/enum definitions: `struct Foo<T: Copy>`
//! - Impl blocks: `impl<T: Display> ToString for T`
//! - Where clauses: `fn foo<T>(x: T) where T: Clone`
//!
//! For each type variable, we build a list of trait requirements that concrete types
//! must satisfy. This enables:
//! - Type constraint verification
//! - Method availability analysis
//! - Associated type resolution
//! - Compiler error generation

use crate::lowering::HirType;
use std::collections::HashMap;

/// A trait bound constraint on a type variable
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitBound {
    /// The trait name (e.g., "Clone", "Display")
    pub trait_name: String,
    /// Generic parameters of the trait itself (e.g., Iterator<Item=T>)
    pub trait_params: Vec<(String, HirType)>,
    /// Lifetime bounds (e.g., 'a for &'a T: Trait)
    pub lifetime_bounds: Vec<String>,
}

impl TraitBound {
    /// Create a simple trait bound with no parameters
    pub fn simple(trait_name: &str) -> Self {
        TraitBound {
            trait_name: trait_name.to_string(),
            trait_params: Vec::new(),
            lifetime_bounds: Vec::new(),
        }
    }

    /// Create a trait bound with generic parameters
    pub fn with_params(
        trait_name: &str,
        trait_params: Vec<(String, HirType)>,
    ) -> Self {
        TraitBound {
            trait_name: trait_name.to_string(),
            trait_params,
            lifetime_bounds: Vec::new(),
        }
    }

    /// Add lifetime bound
    pub fn with_lifetime_bound(mut self, lifetime: String) -> Self {
        self.lifetime_bounds.push(lifetime);
        self
    }
}

/// A trait requirement chain for type inheritance
///
/// For example: if Foo : Bar and Bar : Baz, then Foo implies Baz
#[derive(Debug, Clone)]
pub struct TraitHierarchy {
    /// Map from trait to its supertrait requirements
    supertraits: HashMap<String, Vec<TraitBound>>,
}

impl TraitHierarchy {
    pub fn new() -> Self {
        TraitHierarchy {
            supertraits: HashMap::new(),
        }
    }

    /// Add supertrait relationship: `subtrait` extends `supertrait`
    pub fn add_supertrait(&mut self, subtrait: &str, supertrait: TraitBound) {
        self.supertraits
            .entry(subtrait.to_string())
            .or_default()
            .push(supertrait);
    }

    /// Get all supertraits of a trait (including transitive)
    pub fn get_supertraits(&self, trait_name: &str) -> Vec<TraitBound> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        self.collect_supertraits(trait_name, &mut result, &mut visited);
        result
    }

    fn collect_supertraits(
        &self,
        trait_name: &str,
        result: &mut Vec<TraitBound>,
        visited: &mut std::collections::HashSet<String>,
    ) {
        if visited.contains(trait_name) {
            return; // Prevent infinite loops
        }
        visited.insert(trait_name.to_string());

        if let Some(supers) = self.supertraits.get(trait_name) {
            for supertrait in supers {
                result.push(supertrait.clone());
                self.collect_supertraits(&supertrait.trait_name, result, visited);
            }
        }
    }
}

impl Default for TraitHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracts and manages trait bounds for type variables
#[derive(Debug)]
pub struct TraitBoundExtractor {
    /// Map from type variable name to its direct trait bounds
    /// Example: "T" -> [Clone, Debug, Display]
    direct_bounds: HashMap<String, Vec<TraitBound>>,
    /// Trait inheritance hierarchy for supertrait expansion
    hierarchy: TraitHierarchy,
}

impl TraitBoundExtractor {
    /// Create a new trait bound extractor
    pub fn new() -> Self {
        TraitBoundExtractor {
            direct_bounds: HashMap::new(),
            hierarchy: TraitHierarchy::new(),
        }
    }

    /// Register a trait bound for a type variable
    ///
    /// Example: Extract that `T: Clone` means T must satisfy Clone bound
    pub fn add_bound(&mut self, type_var: &str, bound: TraitBound) {
        self.direct_bounds
            .entry(type_var.to_string())
            .or_default()
            .push(bound);
    }

    /// Get all direct bounds for a type variable
    pub fn get_direct_bounds(&self, type_var: &str) -> Vec<&TraitBound> {
        self.direct_bounds
            .get(type_var)
            .map(|bounds| bounds.iter().collect())
            .unwrap_or_default()
    }

    /// Get all bounds including supertraits (transitive closure)
    pub fn get_all_bounds(&self, type_var: &str) -> Vec<TraitBound> {
        let mut result = Vec::new();

        // Add direct bounds
        if let Some(bounds) = self.direct_bounds.get(type_var) {
            result.extend(bounds.clone());

            // Add supertraits for each direct bound
            for bound in bounds {
                let supertraits = self.hierarchy.get_supertraits(&bound.trait_name);
                result.extend(supertraits);
            }
        }

        // Remove duplicates while preserving order (compare by trait_name)
        let mut seen_traits = std::collections::HashSet::new();
        result.retain(|bound| seen_traits.insert(bound.trait_name.clone()));

        result
    }

    /// Check if a type variable has a specific trait bound
    pub fn has_bound(&self, type_var: &str, trait_name: &str) -> bool {
        self.get_all_bounds(type_var)
            .iter()
            .any(|b| b.trait_name == trait_name)
    }

    /// Get all type variables with bounds
    pub fn all_variables(&self) -> Vec<&str> {
        self.direct_bounds.keys().map(|s| s.as_str()).collect()
    }

    /// Register supertrait relationship for hierarchy
    pub fn add_supertrait(&mut self, subtrait: &str, supertrait: TraitBound) {
        self.hierarchy.add_supertrait(subtrait, supertrait);
    }

    /// Validate that a concrete type satisfies the bounds of a type variable
    ///
    /// This is the core compliance check: does the concrete type implement
    /// all required traits?
    pub fn validate_compliance(&self, type_var: &str, concrete_type: &HirType) -> Result<(), String> {
        let bounds = self.get_all_bounds(type_var);

        if bounds.is_empty() {
            // No bounds means any type is valid
            return Ok(());
        }

        // For now, we accept all concrete types since we don't have access to
        // the actual type system to verify trait implementations.
        // In a full implementation, we would:
        // 1. Look up the concrete type in the type system
        // 2. Check if it implements all required traits
        // 3. Report specific trait violations

        // Placeholder validation
        match concrete_type {
            HirType::Unknown => {
                Err(format!(
                    "Cannot validate bounds for {} against Unknown type",
                    type_var
                ))
            }
            _ => Ok(()), // Accept concrete types (validation would require full type system)
        }
    }

    /// Get a summary of all bounds for error reporting
    pub fn get_bounds_summary(&self, type_var: &str) -> String {
        let bounds = self.get_all_bounds(type_var);
        if bounds.is_empty() {
            "no bounds".to_string()
        } else {
            bounds
                .iter()
                .map(|b| b.trait_name.clone())
                .collect::<Vec<_>>()
                .join(" + ")
        }
    }

    /// Check for conflicting bounds
    ///
    /// Some trait combinations are incompatible or unusual and might indicate
    /// a programming error.
    pub fn check_conflicts(&self, type_var: &str) -> Vec<String> {
        let bounds = self.get_all_bounds(type_var);
        let mut conflicts = Vec::new();

        // Check for Copy + non-Copy conflicts
        let has_copy = bounds.iter().any(|b| b.trait_name == "Copy");
        let has_drop = bounds.iter().any(|b| b.trait_name == "Drop");

        if has_copy && has_drop {
            conflicts.push(format!(
                "Type variable {} cannot be both Copy and Drop",
                type_var
            ));
        }

        // Check for Sync without Send
        let has_sync = bounds.iter().any(|b| b.trait_name == "Sync");
        let has_send = bounds.iter().any(|b| b.trait_name == "Send");

        if has_sync && !has_send {
            conflicts.push(format!(
                "Type variable {} is Sync but not Send (unusual pattern)",
                type_var
            ));
        }

        conflicts
    }
}

impl Default for TraitBoundExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_extractor() {
        let extractor = TraitBoundExtractor::new();
        assert_eq!(extractor.all_variables().len(), 0);
    }

    #[test]
    fn test_add_simple_bound() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));

        assert!(extractor.has_bound("T", "Clone"));
        assert!(!extractor.has_bound("T", "Debug"));
    }

    #[test]
    fn test_multiple_bounds() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));
        extractor.add_bound("T", TraitBound::simple("Debug"));
        extractor.add_bound("T", TraitBound::simple("Display"));

        assert_eq!(extractor.get_direct_bounds("T").len(), 3);
        assert!(extractor.has_bound("T", "Clone"));
        assert!(extractor.has_bound("T", "Debug"));
        assert!(extractor.has_bound("T", "Display"));
    }

    #[test]
    fn test_bounds_summary() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));
        extractor.add_bound("T", TraitBound::simple("Debug"));

        let summary = extractor.get_bounds_summary("T");
        assert!(summary.contains("Clone"));
        assert!(summary.contains("Debug"));
    }

    #[test]
    fn test_supertrait_hierarchy() {
        let mut extractor = TraitBoundExtractor::new();
        // Set up: Foo is a subtrait of Bar, Bar is a subtrait of Baz
        extractor.add_supertrait("Foo", TraitBound::simple("Bar"));
        extractor.add_supertrait("Bar", TraitBound::simple("Baz"));

        // T : Foo should imply T : Bar and T : Baz
        extractor.add_bound("T", TraitBound::simple("Foo"));

        let bounds = extractor.get_all_bounds("T");
        let trait_names: Vec<_> = bounds.iter().map(|b| b.trait_name.as_str()).collect();

        assert!(trait_names.contains(&"Foo"));
        assert!(trait_names.contains(&"Bar"));
        assert!(trait_names.contains(&"Baz"));
    }

    #[test]
    fn test_copy_drop_conflict() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Copy"));
        extractor.add_bound("T", TraitBound::simple("Drop"));

        let conflicts = extractor.check_conflicts("T");
        assert!(!conflicts.is_empty());
        assert!(conflicts[0].contains("Copy") && conflicts[0].contains("Drop"));
    }

    #[test]
    fn test_sync_without_send_warning() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Sync"));

        let conflicts = extractor.check_conflicts("T");
        assert!(!conflicts.is_empty());
        assert!(conflicts[0].contains("Sync") && conflicts[0].contains("Send"));
    }

    #[test]
    fn test_trait_bound_with_params() {
        let bound = TraitBound::with_params(
            "Iterator",
            vec![("Item".to_string(), HirType::Int32)],
        );

        assert_eq!(bound.trait_name, "Iterator");
        assert_eq!(bound.trait_params.len(), 1);
        assert_eq!(bound.trait_params[0].0, "Item");
    }

    #[test]
    fn test_no_bounds_means_valid() {
        let extractor = TraitBoundExtractor::new();

        // Type with no bounds should accept any concrete type
        assert!(extractor.validate_compliance("T", &HirType::Int32).is_ok());
        assert!(extractor.validate_compliance("T", &HirType::String).is_ok());
    }

    #[test]
    fn test_unknown_type_fails_validation() {
        let mut extractor = TraitBoundExtractor::new();
        extractor.add_bound("T", TraitBound::simple("Clone"));

        // Unknown type should fail validation
        assert!(extractor.validate_compliance("T", &HirType::Unknown).is_err());
    }

    #[test]
    fn test_nonexistent_variable() {
        let extractor = TraitBoundExtractor::new();
        assert_eq!(extractor.get_direct_bounds("NonExistent").len(), 0);
        assert!(!extractor.has_bound("NonExistent", "Clone"));
    }
}
