//! # Trait System Defaults Implementation
//!
//! Features:
//! - Default trait implementations
//! - Default method bodies
//! - Supertrait defaults
//! - Generic defaults with specialization
//! - Default trait objects
//! - Method resolution with defaults

use std::collections::{HashMap, HashSet};
use std::fmt;

/// Represents a default method implementation
#[derive(Debug, Clone)]
pub struct DefaultMethod {
    pub name: String,
    pub body: String,
    pub return_type: String,
    pub params: Vec<(String, String)>,
    pub is_generic: bool,
    pub specializations: Vec<String>,
}

impl DefaultMethod {
    pub fn new(
        name: String,
        body: String,
        return_type: String,
        params: Vec<(String, String)>,
    ) -> Self {
        DefaultMethod {
            name,
            body,
            return_type,
            params,
            is_generic: false,
            specializations: Vec::new(),
        }
    }

    pub fn with_generics(mut self, is_generic: bool) -> Self {
        self.is_generic = is_generic;
        self
    }

    pub fn add_specialization(mut self, spec: String) -> Self {
        self.specializations.push(spec);
        self
    }

    pub fn add_specialization_ref(&mut self, spec: String) {
        self.specializations.push(spec);
    }
}

/// Trait definition with defaults
#[derive(Debug, Clone)]
pub struct TraitWithDefaults {
    pub name: String,
    pub supertraits: Vec<String>,
    pub methods: HashMap<String, DefaultMethod>,
    pub associated_types: HashMap<String, String>,
    pub generic_params: Vec<String>,
    pub required_methods: HashSet<String>,
}

impl TraitWithDefaults {
    pub fn new(name: String) -> Self {
        TraitWithDefaults {
            name,
            supertraits: Vec::new(),
            methods: HashMap::new(),
            associated_types: HashMap::new(),
            generic_params: Vec::new(),
            required_methods: HashSet::new(),
        }
    }

    pub fn add_supertrait(mut self, supertrait: String) -> Self {
        self.supertraits.push(supertrait);
        self
    }

    pub fn add_method(mut self, method: DefaultMethod) -> Self {
        self.methods.insert(method.name.clone(), method);
        self
    }

    pub fn add_required_method(mut self, name: String) -> Self {
        self.required_methods.insert(name);
        self
    }

    pub fn add_associated_type(mut self, name: String, default: String) -> Self {
        self.associated_types.insert(name, default);
        self
    }

    pub fn add_generic_param(mut self, param: String) -> Self {
        self.generic_params.push(param);
        self
    }

    pub fn get_method(&self, name: &str) -> Option<&DefaultMethod> {
        self.methods.get(name)
    }

    pub fn has_default_for(&self, method_name: &str) -> bool {
        self.methods.contains_key(method_name)
    }

    pub fn is_method_required(&self, method_name: &str) -> bool {
        self.required_methods.contains(method_name)
    }
}

/// Trait implementation with defaults
#[derive(Debug, Clone)]
pub struct ImplWithDefaults {
    pub trait_name: String,
    pub impl_type: String,
    pub methods: HashMap<String, String>,
    pub uses_defaults: HashSet<String>,
    pub overrides: HashMap<String, String>,
}

impl ImplWithDefaults {
    pub fn new(trait_name: String, impl_type: String) -> Self {
        ImplWithDefaults {
            trait_name,
            impl_type,
            methods: HashMap::new(),
            uses_defaults: HashSet::new(),
            overrides: HashMap::new(),
        }
    }

    pub fn add_method(mut self, name: String, body: String) -> Self {
        self.methods.insert(name, body);
        self
    }

    pub fn use_default(mut self, method_name: String) -> Self {
        self.uses_defaults.insert(method_name);
        self
    }

    pub fn override_default(mut self, method_name: String, new_body: String) -> Self {
        self.overrides.insert(method_name, new_body);
        self
    }
}

/// Trait system resolver with defaults
pub struct TraitDefaultResolver {
    traits: HashMap<String, TraitWithDefaults>,
    impls: Vec<ImplWithDefaults>,
    method_cache: HashMap<String, HashMap<String, String>>,
    specialization_cache: HashMap<String, Vec<String>>,
}

impl TraitDefaultResolver {
    pub fn new() -> Self {
        TraitDefaultResolver {
            traits: HashMap::new(),
            impls: Vec::new(),
            method_cache: HashMap::new(),
            specialization_cache: HashMap::new(),
        }
    }

    /// Register a trait with defaults
    pub fn register_trait(&mut self, trait_def: TraitWithDefaults) {
        self.traits.insert(trait_def.name.clone(), trait_def);
        self.method_cache.clear();
    }

    /// Register a trait implementation
    pub fn register_impl(&mut self, impl_def: ImplWithDefaults) {
        self.impls.push(impl_def);
        self.method_cache.clear();
    }

    /// Resolve all methods for a type implementing a trait
    pub fn resolve_methods(
        &mut self,
        trait_name: &str,
        impl_type: &str,
    ) -> Result<HashMap<String, String>, String> {
        let cache_key = format!("{}@{}", trait_name, impl_type);

        if let Some(cached) = self.method_cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        // Clone the trait_def to avoid borrow checker issues
        let trait_def = self
            .traits
            .get(trait_name)
            .ok_or_else(|| format!("Unknown trait: {}", trait_name))?
            .clone();

        let mut all_methods = HashMap::new();

        // Add methods from supertraits
        for supertrait in &trait_def.supertraits {
            let supertrait_methods = self.resolve_methods(supertrait, impl_type)?;
            all_methods.extend(supertrait_methods);
        }

        // Find matching implementation
        let impl_def = self
            .impls
            .iter()
            .find(|i| i.trait_name == trait_name && i.impl_type == impl_type)
            .cloned();

        // Resolve each method
        for (method_name, default_method) in &trait_def.methods {
            if let Some(impl_def) = &impl_def {
                // Check if overridden
                if let Some(override_body) = impl_def.overrides.get(method_name) {
                    all_methods.insert(method_name.clone(), override_body.clone());
                    continue;
                }

                // Check if provided
                if let Some(body) = impl_def.methods.get(method_name) {
                    all_methods.insert(method_name.clone(), body.clone());
                    continue;
                }

                // Check if using default
                if impl_def.uses_defaults.contains(method_name) {
                    all_methods.insert(method_name.clone(), default_method.body.clone());
                    continue;
                }
            }

            // Use default if available
            if !trait_def.is_method_required(method_name) {
                all_methods.insert(method_name.clone(), default_method.body.clone());
            }
        }

        // Validate all required methods are provided
        for required in &trait_def.required_methods {
            if !all_methods.contains_key(required) {
                return Err(format!(
                    "Missing required method: {} for {} in trait {}",
                    required, impl_type, trait_name
                ));
            }
        }

        self.method_cache.insert(cache_key, all_methods.clone());
        Ok(all_methods)
    }

    /// Get default for a method
    pub fn get_default_method(&self, trait_name: &str, method_name: &str) -> Option<&DefaultMethod> {
        self.traits
            .get(trait_name)
            .and_then(|t| t.get_method(method_name))
    }

    /// Check if type implements trait with all defaults
    pub fn can_use_default_impl(&self, trait_name: &str, impl_type: &str) -> bool {
        if let Some(trait_def) = self.traits.get(trait_name) {
            trait_def
                .required_methods
                .iter()
                .all(|method| trait_def.methods.contains_key(method))
        } else {
            false
        }
    }

    /// Find all types implementing a trait
    pub fn find_implementations(&self, trait_name: &str) -> Vec<String> {
        self.impls
            .iter()
            .filter(|i| i.trait_name == trait_name)
            .map(|i| i.impl_type.clone())
            .collect()
    }

    /// Get supertrait chain
    pub fn get_supertrait_chain(&self, trait_name: &str) -> Vec<String> {
        let mut chain = vec![trait_name.to_string()];
        let mut queue = vec![trait_name.to_string()];

        while let Some(current) = queue.pop() {
            if let Some(trait_def) = self.traits.get(&current) {
                for supertrait in &trait_def.supertraits {
                    if !chain.contains(supertrait) {
                        chain.push(supertrait.clone());
                        queue.push(supertrait.clone());
                    }
                }
            }
        }

        chain
    }

    /// Specialize a generic method
    pub fn specialize_method(
        &mut self,
        trait_name: &str,
        method_name: &str,
        type_param: &str,
        specialized_impl: String,
    ) -> Result<(), String> {
        let trait_def = self
            .traits
            .get_mut(trait_name)
            .ok_or_else(|| format!("Unknown trait: {}", trait_name))?;

        let method = trait_def
            .methods
            .get_mut(method_name)
            .ok_or_else(|| format!("Unknown method: {}", method_name))?;

        method.add_specialization_ref(format!("{}: {}", type_param, specialized_impl));

        let spec_key = format!("{}:{}:{}", trait_name, method_name, type_param);
        self.specialization_cache.insert(
            spec_key,
            vec![type_param.to_string(), specialized_impl],
        );

        Ok(())
    }

    /// Get method specializations
    pub fn get_specializations(&self, trait_name: &str, method_name: &str) -> Vec<String> {
        self.traits
            .get(trait_name)
            .and_then(|t| t.methods.get(method_name))
            .map(|m| m.specializations.clone())
            .unwrap_or_default()
    }

    /// Count default implementations
    pub fn count_default_impls(&self, trait_name: &str) -> usize {
        if let Some(trait_def) = self.traits.get(trait_name) {
            trait_def.methods.len()
        } else {
            0
        }
    }

    /// Trait count
    pub fn trait_count(&self) -> usize {
        self.traits.len()
    }

    /// Implementation count
    pub fn impl_count(&self) -> usize {
        self.impls.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_method_creation() {
        let method = DefaultMethod::new(
            "display".to_string(),
            "println!(...)".to_string(),
            "()".to_string(),
            vec![("self".to_string(), "&Self".to_string())],
        );

        assert_eq!(method.name, "display");
        assert_eq!(method.return_type, "()");
    }

    #[test]
    fn test_trait_with_defaults() {
        let trait_def = TraitWithDefaults::new("Display".to_string())
            .add_method(DefaultMethod::new(
                "fmt".to_string(),
                "{}".to_string(),
                "()".to_string(),
                vec![],
            ))
            .add_required_method("fmt_impl".to_string());

        assert!(trait_def.has_default_for("fmt"));
        assert!(trait_def.is_method_required("fmt_impl"));
    }

    #[test]
    fn test_trait_with_supertraits() {
        let trait_def = TraitWithDefaults::new("Ord".to_string())
            .add_supertrait("PartialOrd".to_string())
            .add_supertrait("Eq".to_string());

        assert_eq!(trait_def.supertraits.len(), 2);
    }

    #[test]
    fn test_impl_with_defaults() {
        let impl_def = ImplWithDefaults::new("Display".to_string(), "String".to_string())
            .add_method("fmt".to_string(), "self.to_string()".to_string())
            .use_default("debug".to_string());

        assert!(impl_def.methods.contains_key("fmt"));
        assert!(impl_def.uses_defaults.contains("debug"));
    }

    #[test]
    fn test_resolver_register_trait() {
        let mut resolver = TraitDefaultResolver::new();
        let trait_def = TraitWithDefaults::new("Clone".to_string());
        resolver.register_trait(trait_def);

        assert_eq!(resolver.trait_count(), 1);
    }

    #[test]
    fn test_resolver_register_impl() {
        let mut resolver = TraitDefaultResolver::new();
        let impl_def = ImplWithDefaults::new("Clone".to_string(), "i32".to_string());
        resolver.register_impl(impl_def);

        assert_eq!(resolver.impl_count(), 1);
    }

    #[test]
    fn test_resolve_methods_simple() {
        let mut resolver = TraitDefaultResolver::new();

        let trait_def = TraitWithDefaults::new("Clone".to_string())
            .add_method(DefaultMethod::new(
                "clone".to_string(),
                "Self".to_string(),
                "Self".to_string(),
                vec![("self".to_string(), "&Self".to_string())],
            ));

        resolver.register_trait(trait_def);

        let impl_def = ImplWithDefaults::new("Clone".to_string(), "i32".to_string())
            .use_default("clone".to_string());

        resolver.register_impl(impl_def);

        let methods = resolver.resolve_methods("Clone", "i32").unwrap();
        assert!(methods.contains_key("clone"));
    }

    #[test]
    fn test_missing_required_method() {
        let mut resolver = TraitDefaultResolver::new();

        let trait_def = TraitWithDefaults::new("Custom".to_string())
            .add_required_method("required_method".to_string());

        resolver.register_trait(trait_def);

        let impl_def = ImplWithDefaults::new("Custom".to_string(), "MyType".to_string());
        resolver.register_impl(impl_def);

        assert!(resolver.resolve_methods("Custom", "MyType").is_err());
    }

    #[test]
    fn test_method_override() {
        let mut resolver = TraitDefaultResolver::new();

        let trait_def = TraitWithDefaults::new("Display".to_string())
            .add_method(DefaultMethod::new(
                "fmt".to_string(),
                "default implementation".to_string(),
                "()".to_string(),
                vec![],
            ));

        resolver.register_trait(trait_def);

        let impl_def = ImplWithDefaults::new("Display".to_string(), "String".to_string())
            .override_default("fmt".to_string(), "custom implementation".to_string());

        resolver.register_impl(impl_def);

        let methods = resolver.resolve_methods("Display", "String").unwrap();
        assert_eq!(methods.get("fmt"), Some(&"custom implementation".to_string()));
    }

    #[test]
    fn test_supertrait_chain() {
        let mut resolver = TraitDefaultResolver::new();

        let trait_a = TraitWithDefaults::new("A".to_string());
        let trait_b = TraitWithDefaults::new("B".to_string()).add_supertrait("A".to_string());
        let trait_c = TraitWithDefaults::new("C".to_string()).add_supertrait("B".to_string());

        resolver.register_trait(trait_a);
        resolver.register_trait(trait_b);
        resolver.register_trait(trait_c);

        let chain = resolver.get_supertrait_chain("C");
        assert_eq!(chain, vec!["C", "B", "A"]);
    }

    #[test]
    fn test_find_implementations() {
        let mut resolver = TraitDefaultResolver::new();

        resolver.register_impl(ImplWithDefaults::new("Trait1".to_string(), "Type1".to_string()));
        resolver.register_impl(ImplWithDefaults::new("Trait1".to_string(), "Type2".to_string()));
        resolver.register_impl(ImplWithDefaults::new("Trait2".to_string(), "Type1".to_string()));

        let impls = resolver.find_implementations("Trait1");
        assert_eq!(impls.len(), 2);
        assert!(impls.contains(&"Type1".to_string()));
        assert!(impls.contains(&"Type2".to_string()));
    }

    #[test]
    fn test_can_use_default_impl() {
        let mut resolver = TraitDefaultResolver::new();

        let trait_def = TraitWithDefaults::new("Display".to_string())
            .add_method(DefaultMethod::new(
                "fmt".to_string(),
                "{}".to_string(),
                "()".to_string(),
                vec![],
            ))
            .add_required_method("fmt".to_string());

        resolver.register_trait(trait_def);

        assert!(resolver.can_use_default_impl("Display", "String"));
    }

    #[test]
    fn test_specialization() {
        let mut resolver = TraitDefaultResolver::new();

        let trait_def = TraitWithDefaults::new("Iterator".to_string()).add_method(
            DefaultMethod::new(
                "next".to_string(),
                "None".to_string(),
                "Option<T>".to_string(),
                vec![],
            )
            .with_generics(true),
        );

        resolver.register_trait(trait_def);

        resolver
            .specialize_method("Iterator", "next", "T", "some_specific_impl".to_string())
            .unwrap();

        let specs = resolver.get_specializations("Iterator", "next");
        assert_eq!(specs.len(), 1);
    }

    #[test]
    fn test_associated_types() {
        let trait_def = TraitWithDefaults::new("Iterator".to_string())
            .add_associated_type("Item".to_string(), "()".to_string());

        assert!(trait_def.associated_types.contains_key("Item"));
    }

    #[test]
    fn test_generic_params() {
        let trait_def = TraitWithDefaults::new("Container".to_string())
            .add_generic_param("T".to_string())
            .add_generic_param("U".to_string());

        assert_eq!(trait_def.generic_params.len(), 2);
    }

    #[test]
    fn test_count_default_impls() {
        let mut resolver = TraitDefaultResolver::new();

        let trait_def = TraitWithDefaults::new("Trait1".to_string())
            .add_method(DefaultMethod::new(
                "method1".to_string(),
                "{}".to_string(),
                "()".to_string(),
                vec![],
            ))
            .add_method(DefaultMethod::new(
                "method2".to_string(),
                "{}".to_string(),
                "()".to_string(),
                vec![],
            ));

        resolver.register_trait(trait_def);

        assert_eq!(resolver.count_default_impls("Trait1"), 2);
    }

    #[test]
    fn test_method_cache() {
        let mut resolver = TraitDefaultResolver::new();

        let trait_def = TraitWithDefaults::new("Trait1".to_string())
            .add_method(DefaultMethod::new(
                "method".to_string(),
                "impl".to_string(),
                "()".to_string(),
                vec![],
            ));

        resolver.register_trait(trait_def);

        let impl_def =
            ImplWithDefaults::new("Trait1".to_string(), "Type1".to_string());
        resolver.register_impl(impl_def);

        // First call
        let _ = resolver.resolve_methods("Trait1", "Type1");
        assert!(!resolver.method_cache.is_empty());

        // Second call should use cache
        let _ = resolver.resolve_methods("Trait1", "Type1");
    }

    #[test]
    fn test_multiple_supertrait_hierarchy() {
        let mut resolver = TraitDefaultResolver::new();

        let trait_eq = TraitWithDefaults::new("Eq".to_string());
        let trait_partial_eq = TraitWithDefaults::new("PartialEq".to_string());
        let trait_ord =
            TraitWithDefaults::new("Ord".to_string())
                .add_supertrait("Eq".to_string())
                .add_supertrait("PartialEq".to_string());

        resolver.register_trait(trait_eq);
        resolver.register_trait(trait_partial_eq);
        resolver.register_trait(trait_ord);

        let chain = resolver.get_supertrait_chain("Ord");
        assert!(chain.contains(&"Eq".to_string()));
        assert!(chain.contains(&"PartialEq".to_string()));
    }
}
