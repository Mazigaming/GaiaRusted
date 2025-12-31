//! # Associated Type Resolver (Phase 5B)
//!
//! Resolves associated types (like Iterator::Item) from trait definitions.
//! Maps trait/type pairs to concrete types for use in type inference and checking.

use std::collections::HashMap;
use crate::lowering::HirType;

/// Represents an associated type mapping from a trait to a concrete type
#[derive(Debug, Clone, PartialEq)]
pub struct AssociatedTypeMapping {
    pub trait_name: String,
    pub type_name: String,
    pub resolved_type: HirType,
    pub generic_params: Vec<String>,
}

impl AssociatedTypeMapping {
    /// Create a simple associated type mapping
    pub fn new(trait_name: impl Into<String>, type_name: impl Into<String>, resolved_type: HirType) -> Self {
        AssociatedTypeMapping {
            trait_name: trait_name.into(),
            type_name: type_name.into(),
            resolved_type,
            generic_params: Vec::new(),
        }
    }

    /// Create with generic parameters
    pub fn with_generics(
        trait_name: impl Into<String>,
        type_name: impl Into<String>,
        resolved_type: HirType,
        generic_params: Vec<String>,
    ) -> Self {
        AssociatedTypeMapping {
            trait_name: trait_name.into(),
            type_name: type_name.into(),
            resolved_type,
            generic_params,
        }
    }
}

/// Registry of standard library iterator types
#[derive(Debug, Clone)]
pub struct IteratorTypeRegistry {
    // Collection type -> Item type mapping
    collections: HashMap<String, HirType>,
}

impl IteratorTypeRegistry {
    /// Create a new iterator type registry
    pub fn new() -> Self {
        IteratorTypeRegistry {
            collections: HashMap::new(),
        }
    }

    /// Register a standard collection type's Item type
    pub fn register_collection(&mut self, collection_name: impl Into<String>, item_type: HirType) {
        self.collections.insert(collection_name.into(), item_type);
    }

    /// Get the Item type for a collection
    pub fn get_item_type(&self, collection_name: &str) -> Option<HirType> {
        self.collections.get(collection_name).cloned()
    }

    /// Initialize with standard Rust types
    pub fn with_standard_types() -> Self {
        let mut registry = IteratorTypeRegistry::new();

        // Vec<T> -> Iterator<Item=T>
        registry.register_collection("Vec", HirType::Named("T".to_string()));

        // Array [T; n] -> Iterator<Item=T>
        registry.register_collection("Array", HirType::Named("T".to_string()));

        // Slice [T] -> Iterator<Item=T>
        registry.register_collection("Slice", HirType::Named("T".to_string()));

        // String -> Iterator<Item=char>
        registry.register_collection("String", HirType::Char);

        // HashMap<K, V> -> Iterator<Item=(K, V)>
        registry.register_collection("HashMap", HirType::Tuple(vec![
            HirType::Named("K".to_string()),
            HirType::Named("V".to_string()),
        ]));

        // HashSet<T> -> Iterator<Item=T>
        registry.register_collection("HashSet", HirType::Named("T".to_string()));

        // BTreeMap<K, V> -> Iterator<Item=(K, V)>
        registry.register_collection("BTreeMap", HirType::Tuple(vec![
            HirType::Named("K".to_string()),
            HirType::Named("V".to_string()),
        ]));

        // BTreeSet<T> -> Iterator<Item=T>
        registry.register_collection("BTreeSet", HirType::Named("T".to_string()));

        // Range<T> -> Iterator<Item=T>
        registry.register_collection("Range", HirType::Named("T".to_string()));

        registry
    }
}

impl Default for IteratorTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolves associated types from trait definitions
#[derive(Debug)]
pub struct AssociatedTypeResolver {
    // (trait_name, type_name) -> resolved type
    associations: HashMap<(String, String), HirType>,
    // trait_name -> generic parameters
    trait_generics: HashMap<String, Vec<String>>,
    // Iterator-specific registry
    iterator_registry: IteratorTypeRegistry,
}

impl AssociatedTypeResolver {
    /// Create a new associated type resolver
    pub fn new() -> Self {
        AssociatedTypeResolver {
            associations: HashMap::new(),
            trait_generics: HashMap::new(),
            iterator_registry: IteratorTypeRegistry::new(),
        }
    }

    /// Create with standard types pre-registered
    pub fn with_standard_types() -> Self {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.iterator_registry = IteratorTypeRegistry::with_standard_types();
        resolver.register_standard_associated_types();
        resolver
    }

    /// Register a standard associated type
    fn register_standard_associated_types(&mut self) {
        // Iterator trait's Item type
        self.register_assoc_type("Iterator", "Item", HirType::Named("Item".to_string()));

        // IntoIterator trait's Item type
        self.register_assoc_type("IntoIterator", "Item", HirType::Named("Item".to_string()));

        // Default trait's Output
        self.register_assoc_type("Default", "Output", HirType::Named("Self".to_string()));
    }

    /// Register an associated type from a trait
    pub fn register_assoc_type(&mut self, trait_name: &str, type_name: &str, resolved_type: HirType) {
        self.associations.insert((trait_name.to_string(), type_name.to_string()), resolved_type);
    }

    /// Register generic parameters for a trait
    pub fn register_trait_generics(&mut self, trait_name: &str, params: Vec<String>) {
        self.trait_generics.insert(trait_name.to_string(), params);
    }

    /// Register an iterator type
    pub fn register_iterator_type(&mut self, collection_name: &str, item_type: HirType) {
        self.iterator_registry.register_collection(collection_name, item_type);
    }

    /// Resolve an associated type
    pub fn resolve(&self, trait_name: &str, type_name: &str) -> Option<HirType> {
        self.associations
            .get(&(trait_name.to_string(), type_name.to_string()))
            .cloned()
    }

    /// Resolve Iterator::Item for a given collection type
    pub fn resolve_iterator_item(&self, collection_type: &HirType) -> Option<HirType> {
        match collection_type {
            HirType::Named(name) => {
                // Try to get from iterator registry
                self.iterator_registry.get_item_type(name)
            }
            HirType::Reference(inner) | HirType::MutableReference(inner) => {
                // References to iterables are also iterable
                self.resolve_iterator_item(inner)
            }
            _ => None,
        }
    }

    /// Resolve a for-loop iterator type
    pub fn resolve_for_loop_iterator(&self, collection_type: &HirType) -> Option<HirType> {
        // For a collection type, return its Iterator::Item type
        self.resolve_iterator_item(collection_type)
    }

    /// Get all registered associated types for a trait
    pub fn get_trait_associations(&self, trait_name: &str) -> Vec<(String, HirType)> {
        self.associations
            .iter()
            .filter(|((t, _), _)| t == trait_name)
            .map(|((_, type_name), resolved_type)| (type_name.clone(), resolved_type.clone()))
            .collect()
    }

    /// Get all known associated types
    pub fn get_all_associations(&self) -> Vec<AssociatedTypeMapping> {
        self.associations
            .iter()
            .map(|((trait_name, type_name), resolved_type)| {
                let generic_params = self
                    .trait_generics
                    .get(trait_name)
                    .cloned()
                    .unwrap_or_default();
                AssociatedTypeMapping::with_generics(
                    trait_name.clone(),
                    type_name.clone(),
                    resolved_type.clone(),
                    generic_params,
                )
            })
            .collect()
    }

    /// Check if a trait has an associated type
    pub fn has_assoc_type(&self, trait_name: &str, type_name: &str) -> bool {
        self.associations.contains_key(&(trait_name.to_string(), type_name.to_string()))
    }

    /// Count total registered associations
    pub fn total_associations(&self) -> usize {
        self.associations.len()
    }

    /// Get summary of all resolved types for debugging
    pub fn get_summary(&self) -> String {
        let mut parts = Vec::new();
        let mut traits: std::collections::HashSet<_> = std::collections::HashSet::new();

        for (trait_name, _) in self.associations.keys() {
            traits.insert(trait_name.clone());
        }

        for trait_name in traits {
            let assocs = self.get_trait_associations(&trait_name);
            if !assocs.is_empty() {
                parts.push(format!(
                    "{}::{{{}}}",
                    trait_name,
                    assocs
                        .iter()
                        .map(|(tn, _)| tn.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        if parts.is_empty() {
            "No associated types registered".to_string()
        } else {
            parts.join(", ")
        }
    }
}

impl Default for AssociatedTypeResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_associated_type_mapping_creation() {
        let mapping = AssociatedTypeMapping::new("Iterator", "Item", HirType::Int32);
        assert_eq!(mapping.trait_name, "Iterator");
        assert_eq!(mapping.type_name, "Item");
        assert_eq!(mapping.resolved_type, HirType::Int32);
        assert!(mapping.generic_params.is_empty());
    }

    #[test]
    fn test_associated_type_with_generics() {
        let mapping = AssociatedTypeMapping::with_generics(
            "Iterator",
            "Item",
            HirType::Int32,
            vec!["T".to_string()],
        );
        assert_eq!(mapping.generic_params.len(), 1);
        assert_eq!(mapping.generic_params[0], "T");
    }

    #[test]
    fn test_resolver_new() {
        let resolver = AssociatedTypeResolver::new();
        assert_eq!(resolver.total_associations(), 0);
    }

    #[test]
    fn test_register_assoc_type() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);

        let resolved = resolver.resolve("Iterator", "Item");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), HirType::Int32);
    }

    #[test]
    fn test_resolve_nonexistent() {
        let resolver = AssociatedTypeResolver::new();
        assert!(resolver.resolve("NonExistent", "Type").is_none());
    }

    #[test]
    fn test_has_assoc_type() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);

        assert!(resolver.has_assoc_type("Iterator", "Item"));
        assert!(!resolver.has_assoc_type("Iterator", "Other"));
        assert!(!resolver.has_assoc_type("Other", "Item"));
    }

    #[test]
    fn test_get_trait_associations() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);
        resolver.register_assoc_type("Iterator", "IntoIter", HirType::String);
        resolver.register_assoc_type("Default", "Output", HirType::Bool);

        let assocs = resolver.get_trait_associations("Iterator");
        assert_eq!(assocs.len(), 2);
        assert!(assocs.iter().any(|(name, _)| name == "Item"));
        assert!(assocs.iter().any(|(name, _)| name == "IntoIter"));
    }

    #[test]
    fn test_get_all_associations() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);
        resolver.register_assoc_type("Default", "Output", HirType::Bool);

        let all = resolver.get_all_associations();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_iterator_registry_new() {
        let registry = IteratorTypeRegistry::new();
        assert!(registry.get_item_type("Vec").is_none());
    }

    #[test]
    fn test_register_collection() {
        let mut registry = IteratorTypeRegistry::new();
        registry.register_collection("Vec", HirType::Int32);

        let item_type = registry.get_item_type("Vec");
        assert!(item_type.is_some());
        assert_eq!(item_type.unwrap(), HirType::Int32);
    }

    #[test]
    fn test_with_standard_types() {
        let registry = IteratorTypeRegistry::with_standard_types();

        // Check some standard types
        assert!(registry.get_item_type("Vec").is_some());
        assert!(registry.get_item_type("String").is_some());
        assert!(registry.get_item_type("HashMap").is_some());
    }

    #[test]
    fn test_resolve_iterator_item_named() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_iterator_type("Vec", HirType::Int32);

        let item = resolver.resolve_iterator_item(&HirType::Named("Vec".to_string()));
        assert!(item.is_some());
        assert_eq!(item.unwrap(), HirType::Int32);
    }

    #[test]
    fn test_resolve_iterator_item_reference() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_iterator_type("Vec", HirType::Int32);

        let vec_ref = HirType::Reference(Box::new(HirType::Named("Vec".to_string())));
        let item = resolver.resolve_iterator_item(&vec_ref);
        assert!(item.is_some());
        assert_eq!(item.unwrap(), HirType::Int32);
    }

    #[test]
    fn test_resolve_iterator_item_concrete() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_iterator_type("IntVec", HirType::Int32);

        let item = resolver.resolve_iterator_item(&HirType::Named("IntVec".to_string()));
        assert!(item.is_some());
        assert_eq!(item.unwrap(), HirType::Int32);
    }

    #[test]
    fn test_resolve_for_loop_iterator() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_iterator_type("Vec", HirType::String);

        let item = resolver.resolve_for_loop_iterator(&HirType::Named("Vec".to_string()));
        assert!(item.is_some());
        assert_eq!(item.unwrap(), HirType::String);
    }

    #[test]
    fn test_with_standard_types_resolver() {
        let resolver = AssociatedTypeResolver::with_standard_types();

        // Check that standard associations exist
        assert!(resolver.has_assoc_type("Iterator", "Item"));
        assert!(resolver.has_assoc_type("IntoIterator", "Item"));
        assert!(resolver.resolve("Iterator", "Item").is_some());
    }

    #[test]
    fn test_register_trait_generics() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_trait_generics("Iterator", vec!["T".to_string(), "U".to_string()]);

        let mapping = AssociatedTypeMapping::with_generics(
            "Iterator",
            "Item",
            HirType::Int32,
            resolver
                .trait_generics
                .get("Iterator")
                .cloned()
                .unwrap_or_default(),
        );
        assert_eq!(mapping.generic_params.len(), 2);
    }

    #[test]
    fn test_get_summary() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);
        resolver.register_assoc_type("Default", "Output", HirType::Bool);

        let summary = resolver.get_summary();
        assert!(!summary.is_empty());
        assert!(summary.contains("Iterator") || summary.contains("Default"));
    }

    #[test]
    fn test_get_summary_empty() {
        let resolver = AssociatedTypeResolver::new();
        let summary = resolver.get_summary();
        assert_eq!(summary, "No associated types registered");
    }

    #[test]
    fn test_multiple_associated_types_same_trait() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);
        resolver.register_assoc_type("Iterator", "IntoIter", HirType::String);
        resolver.register_assoc_type("Iterator", "Output", HirType::Bool);

        assert_eq!(resolver.total_associations(), 3);
        let assocs = resolver.get_trait_associations("Iterator");
        assert_eq!(assocs.len(), 3);
    }

    #[test]
    fn test_case_sensitive_trait_names() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "Item", HirType::Int32);
        resolver.register_assoc_type("iterator", "Item", HirType::String);

        // Should be different
        assert_eq!(resolver.resolve("Iterator", "Item").unwrap(), HirType::Int32);
        assert_eq!(resolver.resolve("iterator", "Item").unwrap(), HirType::String);
    }

    #[test]
    fn test_empty_trait_name() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("", "Item", HirType::Int32);

        assert!(resolver.has_assoc_type("", "Item"));
        assert_eq!(resolver.resolve("", "Item").unwrap(), HirType::Int32);
    }

    #[test]
    fn test_empty_type_name() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.register_assoc_type("Iterator", "", HirType::Int32);

        assert!(resolver.has_assoc_type("Iterator", ""));
        assert_eq!(resolver.resolve("Iterator", "").unwrap(), HirType::Int32);
    }

    #[test]
    fn test_many_trait_registrations() {
        let mut resolver = AssociatedTypeResolver::new();

        // Register 100 different traits
        for i in 0..100 {
            let trait_name = format!("Trait{}", i);
            resolver.register_assoc_type(&trait_name, "Item", HirType::Int32);
        }

        assert_eq!(resolver.total_associations(), 100);
        assert!(resolver.has_assoc_type("Trait0", "Item"));
        assert!(resolver.has_assoc_type("Trait99", "Item"));
    }

    #[test]
    fn test_deep_nesting() {
        let mut resolver = AssociatedTypeResolver::new();

        // Deep nesting: reference to reference
        let deep_type = HirType::Reference(Box::new(HirType::Reference(Box::new(
            HirType::Named("Vec".to_string()),
        ))));

        resolver.register_iterator_type("Vec", HirType::Int32);

        // Resolving reference to reference should still work
        let item = resolver.resolve_iterator_item(&deep_type);
        assert!(item.is_some());
        assert_eq!(item.unwrap(), HirType::Int32);
    }

    #[test]
    fn test_standard_collection_resolution() {
        let resolver = AssociatedTypeResolver::with_standard_types();

        // These should all work with standard types
        let string_registry = &resolver.iterator_registry;
        assert!(string_registry.get_item_type("String").is_some());
        assert!(string_registry.get_item_type("Vec").is_some());
        assert!(string_registry.get_item_type("HashMap").is_some());
    }
}
