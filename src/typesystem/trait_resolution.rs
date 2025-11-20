//! # Trait Resolution System
//!
//! Implements trait solving and resolution for type checking.
//! Handles:
//! - Trait bound checking
//! - Associated type resolution
//! - Method lookup
//! - Implicit trait object creation

use super::advanced_types::{
    TypeBound, TraitDefinition, TraitMethod, TypePredicate, AssociatedType,
};
use super::types::{Type, TraitId, TypeVar};
use std::collections::HashMap;

/// A resolved trait implementation
#[derive(Debug, Clone)]
pub struct TraitImpl {
    /// The trait being implemented
    pub trait_id: TraitId,
    /// The type implementing the trait
    pub impl_type: Box<Type>,
    /// Associated type assignments
    pub associated_types: HashMap<String, Type>,
    /// Methods implementation mapping
    pub methods: HashMap<String, String>,
}

impl TraitImpl {
    /// Create a new trait implementation
    pub fn new(trait_id: TraitId, impl_type: Type) -> Self {
        TraitImpl {
            trait_id,
            impl_type: Box::new(impl_type),
            associated_types: HashMap::new(),
            methods: HashMap::new(),
        }
    }

    /// Set an associated type
    pub fn set_associated_type(&mut self, name: String, ty: Type) {
        self.associated_types.insert(name, ty);
    }

    /// Map a method name to implementation
    pub fn map_method(&mut self, trait_method: String, impl_method: String) {
        self.methods.insert(trait_method, impl_method);
    }
}

/// Trait resolution context
pub struct TraitResolver {
    /// Known trait implementations
    impls: Vec<TraitImpl>,
    /// Known trait definitions
    trait_defs: HashMap<TraitId, TraitDefinition>,
    /// Cache for resolution results
    cache: HashMap<(TraitId, String), Option<TraitImpl>>,
}

impl TraitResolver {
    /// Create a new trait resolver
    pub fn new() -> Self {
        TraitResolver {
            impls: Vec::new(),
            trait_defs: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    /// Register a trait definition
    pub fn register_trait(&mut self, def: TraitDefinition) {
        self.trait_defs.insert(def.trait_id, def);
    }

    /// Register a trait implementation
    pub fn register_impl(&mut self, impl_: TraitImpl) {
        self.impls.push(impl_);
    }

    /// Resolve a type against a trait bound
    pub fn resolve_bound(&mut self, bound: &TypeBound) -> Result<TraitImpl, String> {
        let key = (bound.trait_id, format!("{}", bound.subject));
        if let Some(cached) = self.cache.get(&key) {
            return if let Some(impl_) = cached.clone() {
                Ok(impl_)
            } else {
                Err("Cached resolution failed".to_string())
            };
        }

        for impl_ in &self.impls {
            if impl_.trait_id == bound.trait_id && impl_.impl_type == bound.subject {
                let result = impl_.clone();
                self.cache.insert(key, Some(result.clone()));
                return Ok(result);
            }
        }

        self.cache.insert(key.clone(), None);
        Err(format!(
            "No implementation of trait {} for type {}",
            bound.trait_id.0, bound.subject
        ))
    }

    /// Resolve an associated type
    pub fn resolve_associated_type(
        &mut self,
        assoc: &AssociatedType,
    ) -> Result<Type, String> {
        let impl_ = self.resolve_bound(&TypeBound::new(
            *assoc.self_type.clone(),
            assoc.trait_id,
            vec![],
        ))?;

        impl_
            .associated_types
            .get(&assoc.name)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Associated type {} not found in implementation",
                    assoc.name
                )
            })
    }

    /// Find a method implementation
    pub fn resolve_method(
        &mut self,
        trait_id: TraitId,
        ty: &Type,
        method_name: &str,
    ) -> Result<TraitMethod, String> {
        let bound = TypeBound::new(ty.clone(), trait_id, vec![]);
        let impl_ = self.resolve_bound(&bound)?;

        let trait_def = self
            .trait_defs
            .get(&trait_id)
            .ok_or_else(|| format!("Trait {} not found", trait_id.0))?;

        trait_def
            .methods
            .get(method_name)
            .cloned()
            .ok_or_else(|| format!("Method {} not found in trait", method_name))
    }

    /// Check if a type satisfies multiple bounds
    pub fn check_bounds(&mut self, ty: &Type, bounds: &[TypeBound]) -> Result<(), String> {
        for bound in bounds {
            self.resolve_bound(bound)?;
        }
        Ok(())
    }

    /// Check predicates in a constraint
    pub fn check_predicates(&mut self, predicates: &[TypePredicate]) -> Result<(), String> {
        for predicate in predicates {
            match predicate {
                TypePredicate::TraitBound(bound) => {
                    self.resolve_bound(bound)?;
                }
                TypePredicate::Equality { left, right } => {
                    if left != right {
                        return Err(format!("Type mismatch: {} != {}", left, right));
                    }
                }
                TypePredicate::LifetimeBound { longer, shorter } => {
                    if longer.0 >= shorter.0 {
                        return Err(format!(
                            "Lifetime bound violation: 't{} should outlive 't{}",
                            longer.0, shorter.0
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for TraitResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait object type representation
#[derive(Debug, Clone)]
pub struct TraitObject {
    /// The trait ID
    pub trait_id: TraitId,
    /// Concrete type implementing the trait
    pub concrete_type: Box<Type>,
    /// Method vtable pointers
    pub vtable: HashMap<String, String>,
}

impl TraitObject {
    /// Create a new trait object
    pub fn new(trait_id: TraitId, concrete_type: Type) -> Self {
        TraitObject {
            trait_id,
            concrete_type: Box::new(concrete_type),
            vtable: HashMap::new(),
        }
    }

    /// Add method to vtable
    pub fn add_method(&mut self, method_name: String, vtable_entry: String) {
        self.vtable.insert(method_name, vtable_entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typesystem::advanced_types::TraitDefinition;

    #[test]
    fn test_trait_impl_creation() {
        let impl_ = TraitImpl::new(TraitId(0), Type::I32);
        assert_eq!(impl_.trait_id, TraitId(0));
        assert!(impl_.associated_types.is_empty());
    }

    #[test]
    fn test_trait_impl_associated_type() {
        let mut impl_ = TraitImpl::new(TraitId(0), Type::I32);
        impl_.set_associated_type("Item".to_string(), Type::Bool);
        assert_eq!(impl_.associated_types.len(), 1);
        assert_eq!(impl_.associated_types.get("Item"), Some(&Type::Bool));
    }

    #[test]
    fn test_trait_resolver_creation() {
        let resolver = TraitResolver::new();
        assert_eq!(resolver.impls.len(), 0);
    }

    #[test]
    fn test_trait_resolver_register_impl() {
        let mut resolver = TraitResolver::new();
        let impl_ = TraitImpl::new(TraitId(0), Type::I32);
        resolver.register_impl(impl_);
        assert_eq!(resolver.impls.len(), 1);
    }

    #[test]
    fn test_trait_object_creation() {
        let obj = TraitObject::new(TraitId(0), Type::I32);
        assert_eq!(obj.trait_id, TraitId(0));
        assert!(obj.vtable.is_empty());
    }

    #[test]
    fn test_trait_object_add_method() {
        let mut obj = TraitObject::new(TraitId(0), Type::I32);
        obj.add_method("clone".to_string(), "clone_i32".to_string());
        assert_eq!(obj.vtable.len(), 1);
    }

    #[test]
    fn test_trait_resolver_resolve_bound_success() {
        let mut resolver = TraitResolver::new();
        let impl_ = TraitImpl::new(TraitId(0), Type::I32);
        resolver.register_impl(impl_);

        let bound = TypeBound::new(Type::I32, TraitId(0), vec![]);
        assert!(resolver.resolve_bound(&bound).is_ok());
    }

    #[test]
    fn test_trait_resolver_resolve_bound_failure() {
        let mut resolver = TraitResolver::new();
        let bound = TypeBound::new(Type::I32, TraitId(0), vec![]);
        assert!(resolver.resolve_bound(&bound).is_err());
    }

    #[test]
    fn test_trait_resolver_caching() {
        let mut resolver = TraitResolver::new();
        let impl_ = TraitImpl::new(TraitId(0), Type::I32);
        resolver.register_impl(impl_);

        let bound = TypeBound::new(Type::I32, TraitId(0), vec![]);
        let _result1 = resolver.resolve_bound(&bound);
        let cache_size_after_first = resolver.cache.len();
        let _result2 = resolver.resolve_bound(&bound);
        assert_eq!(cache_size_after_first, resolver.cache.len());
    }

    #[test]
    fn test_trait_resolver_check_bounds() {
        let mut resolver = TraitResolver::new();
        let impl_ = TraitImpl::new(TraitId(0), Type::I32);
        resolver.register_impl(impl_);

        let bounds = vec![
            TypeBound::new(Type::I32, TraitId(0), vec![]),
        ];
        assert!(resolver.check_bounds(&Type::I32, &bounds).is_ok());
    }

    #[test]
    fn test_trait_resolver_check_predicates() {
        let mut resolver = TraitResolver::new();
        let predicates = vec![
            TypePredicate::Equality {
                left: Box::new(Type::I32),
                right: Box::new(Type::I32),
            },
        ];
        assert!(resolver.check_predicates(&predicates).is_ok());
    }

    #[test]
    fn test_trait_resolver_clear_cache() {
        let mut resolver = TraitResolver::new();
        let impl_ = TraitImpl::new(TraitId(0), Type::I32);
        resolver.register_impl(impl_);

        let bound = TypeBound::new(Type::I32, TraitId(0), vec![]);
        let _result = resolver.resolve_bound(&bound);
        assert!(!resolver.cache.is_empty());

        resolver.clear_cache();
        assert!(resolver.cache.is_empty());
    }
}
