//! Generic Trait Method Monomorphization
//!
//! Handles the instantiation of generic trait methods into concrete implementations.
//! Ensures proper linking of trait method calls with concrete type instantiations.

use std::collections::{HashMap, HashSet};
use std::fmt;

/// Generic trait method specification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericTraitMethod {
    pub trait_name: String,
    pub method_name: String,
    pub generic_params: Vec<String>,
    pub signature: String,
}

/// Monomorphized trait method instance
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConcreteTraitMethod {
    pub trait_name: String,
    pub impl_type: String,
    pub method_name: String,
    pub concrete_symbol: String,
    pub type_args: Vec<String>,
}

impl fmt::Display for ConcreteTraitMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}::<{}>::{}",
            self.trait_name,
            self.type_args.join(", "),
            self.method_name
        )
    }
}

/// Registry for tracking generic trait method instantiations
pub struct TraitMonomorphizationRegistry {
    /// Maps (trait, method, type_args) to concrete implementation
    instances: HashMap<String, ConcreteTraitMethod>,
    /// Set of generic trait methods
    generic_methods: HashMap<String, GenericTraitMethod>,
    /// Maps trait methods to their implementations per type
    impl_map: HashMap<String, HashMap<String, String>>,
    /// Unresolved method calls (for error reporting)
    unresolved: Vec<(String, Vec<String>)>,
}

impl TraitMonomorphizationRegistry {
    pub fn new() -> Self {
        TraitMonomorphizationRegistry {
            instances: HashMap::new(),
            generic_methods: HashMap::new(),
            impl_map: HashMap::new(),
            unresolved: Vec::new(),
        }
    }

    /// Register a generic trait method
    pub fn register_generic_method(
        &mut self,
        trait_name: String,
        method_name: String,
        generic_params: Vec<String>,
        signature: String,
    ) {
        let key = format!("{}::{}", trait_name, method_name);
        self.generic_methods.insert(
            key,
            GenericTraitMethod {
                trait_name,
                method_name,
                generic_params,
                signature,
            },
        );
    }

    /// Instantiate a generic trait method for a concrete type
    pub fn instantiate_trait_method(
        &mut self,
        trait_name: &str,
        method_name: &str,
        impl_type: &str,
        type_args: Vec<String>,
    ) -> Result<String, String> {
        let key = format!("{}::{}", trait_name, method_name);
        
        if !self.generic_methods.contains_key(&key) {
            return Err(format!(
                "Generic trait method '{}::{}' not registered",
                trait_name, method_name
            ));
        }

        let concrete_symbol = self.generate_concrete_symbol(
            trait_name,
            method_name,
            impl_type,
            &type_args,
        );

        let instance = ConcreteTraitMethod {
            trait_name: trait_name.to_string(),
            impl_type: impl_type.to_string(),
            method_name: method_name.to_string(),
            concrete_symbol: concrete_symbol.clone(),
            type_args,
        };

        let instance_key = format!(
            "{}::<{}>::{}",
            trait_name,
            instance.type_args.join("_"),
            method_name
        );

        self.instances.insert(instance_key, instance);
        Ok(concrete_symbol)
    }

    /// Generate a unique symbol for a concrete trait method
    fn generate_concrete_symbol(
        &self,
        trait_name: &str,
        method_name: &str,
        impl_type: &str,
        type_args: &[String],
    ) -> String {
        let type_part = type_args
            .iter()
            .map(|t| t.replace(" ", "_").replace("<", "_").replace(">", "_"))
            .collect::<Vec<_>>()
            .join("_");

        let impl_part = impl_type.replace(" ", "_").replace("<", "_").replace(">", "_");

        if type_part.is_empty() {
            format!("{}__{}__{}", trait_name, impl_part, method_name)
        } else {
            format!("{}__{}__{}_{}", trait_name, impl_part, type_part, method_name)
        }
    }

    /// Register an implementation mapping
    pub fn register_impl_mapping(&mut self, trait_name: String, impl_type: String, method_mapping: HashMap<String, String>) {
        let key = format!("{}::{}", trait_name, impl_type);
        self.impl_map.insert(key, method_mapping);
    }

    /// Resolve a trait method call to its implementation
    pub fn resolve_method_call(
        &self,
        trait_name: &str,
        impl_type: &str,
        method_name: &str,
    ) -> Option<String> {
        let key = format!("{}::{}", trait_name, impl_type);
        self.impl_map
            .get(&key)
            .and_then(|mapping| mapping.get(method_name).cloned())
    }

    /// Get all instantiations for a specific trait method
    pub fn get_instantiations(&self, trait_name: &str, method_name: &str) -> Vec<ConcreteTraitMethod> {
        self.instances
            .values()
            .filter(|inst| inst.trait_name == trait_name && inst.method_name == method_name)
            .cloned()
            .collect()
    }

    /// Get all registered instances
    pub fn get_all_instances(&self) -> Vec<ConcreteTraitMethod> {
        self.instances.values().cloned().collect()
    }

    /// Get unresolved method calls
    pub fn get_unresolved(&self) -> &[(String, Vec<String>)] {
        &self.unresolved
    }

    /// Record an unresolved method call
    pub fn record_unresolved(&mut self, method_name: String, type_args: Vec<String>) {
        self.unresolved.push((method_name, type_args));
    }

    /// Generate linker symbols for all instantiations
    pub fn generate_linker_symbols(&self) -> HashMap<String, String> {
        let mut symbols = HashMap::new();
        for instance in self.instances.values() {
            symbols.insert(
                instance.concrete_symbol.clone(),
                format!("{}::{}", instance, instance.method_name),
            );
        }
        symbols
    }

    /// Verify all registered generic methods have implementations
    pub fn verify_complete(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for (key, _method) in &self.generic_methods {
            if !self.instances.values().any(|inst| {
                format!("{}::{}", inst.trait_name, inst.method_name) == *key
            }) {
                errors.push(format!("No instantiation found for generic method: {}", key));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for TraitMonomorphizationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_generic_method() {
        let mut registry = TraitMonomorphizationRegistry::new();
        registry.register_generic_method(
            "Iterator".to_string(),
            "map".to_string(),
            vec!["F".to_string()],
            "fn(F) -> Self".to_string(),
        );
        
        assert_eq!(registry.generic_methods.len(), 1);
    }

    #[test]
    fn test_instantiate_trait_method() {
        let mut registry = TraitMonomorphizationRegistry::new();
        registry.register_generic_method(
            "Iterator".to_string(),
            "map".to_string(),
            vec!["F".to_string()],
            "fn(F) -> Self".to_string(),
        );

        let result = registry.instantiate_trait_method(
            "Iterator",
            "map",
            "Vec<i32>",
            vec!["i32".to_string()],
        );

        assert!(result.is_ok());
        assert_eq!(registry.instances.len(), 1);
    }

    #[test]
    fn test_unregistered_method_fails() {
        let mut registry = TraitMonomorphizationRegistry::new();
        let result = registry.instantiate_trait_method(
            "Unknown",
            "method",
            "Type",
            vec![],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_concrete_symbol_generation() {
        let registry = TraitMonomorphizationRegistry::new();
        let symbol = registry.generate_concrete_symbol(
            "Display",
            "fmt",
            "String",
            &["Debug".to_string()],
        );

        assert!(symbol.contains("Display"));
        assert!(symbol.contains("String"));
        assert!(symbol.contains("Debug"));
        assert!(symbol.contains("fmt"));
    }

    #[test]
    fn test_resolve_method_call() {
        let mut registry = TraitMonomorphizationRegistry::new();
        let mut mapping = HashMap::new();
        mapping.insert("clone".to_string(), "clone_i32".to_string());
        registry.register_impl_mapping("Clone".to_string(), "i32".to_string(), mapping);

        let result = registry.resolve_method_call("Clone", "i32", "clone");
        assert_eq!(result, Some("clone_i32".to_string()));
    }

    #[test]
    fn test_get_instantiations() {
        let mut registry = TraitMonomorphizationRegistry::new();
        registry.register_generic_method(
            "Iterator".to_string(),
            "map".to_string(),
            vec!["F".to_string()],
            "fn(F) -> Self".to_string(),
        );

        registry.instantiate_trait_method(
            "Iterator",
            "map",
            "Vec<i32>",
            vec!["i32".to_string()],
        ).ok();

        registry.instantiate_trait_method(
            "Iterator",
            "map",
            "Vec<String>",
            vec!["String".to_string()],
        ).ok();

        let instances = registry.get_instantiations("Iterator", "map");
        assert_eq!(instances.len(), 2);
    }

    #[test]
    fn test_generate_linker_symbols() {
        let mut registry = TraitMonomorphizationRegistry::new();
        registry.register_generic_method(
            "Iterator".to_string(),
            "map".to_string(),
            vec!["F".to_string()],
            "fn(F) -> Self".to_string(),
        );

        registry.instantiate_trait_method(
            "Iterator",
            "map",
            "Vec<i32>",
            vec!["i32".to_string()],
        ).ok();

        let symbols = registry.generate_linker_symbols();
        assert!(!symbols.is_empty());
    }
}
