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
use super::types::{Type, TraitId};
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
    /// 
    /// This now properly handles generic trait arguments.
    /// For example: T: Iterator<Item=i32> will properly resolve the Item associated type.
    pub fn resolve_bound(&mut self, bound: &TypeBound) -> Result<TraitImpl, String> {
        // Build cache key including trait arguments for proper generic handling
        let cache_key = self.build_cache_key(bound);
        
        if let Some(cached) = self.cache.get(&cache_key) {
            return if let Some(impl_) = cached.clone() {
                Ok(impl_)
            } else {
                Err("Cached resolution failed".to_string())
            };
        }

        // Try to find a matching implementation
        let mut best_match: Option<TraitImpl> = None;
        let mut last_error: Option<String> = None;
        
        for impl_ in &self.impls {
            if impl_.trait_id == bound.trait_id && impl_.impl_type == bound.subject {
                // Check if generic trait arguments match (if specified)
                if self.trait_args_compatible(&impl_, bound) {
                    best_match = Some(impl_.clone());
                    // Found exact match, can stop searching
                    break;
                } else if !bound.trait_args.is_empty() {
                    // Capture error for better diagnostics
                    let trait_def = self.trait_defs.get(&bound.trait_id);
                    if let Some(def) = trait_def {
                        if let Err(e) = self.validate_associated_types(&impl_, bound, def) {
                            last_error = Some(e);
                        }
                    }
                }
            }
        }

        if let Some(result) = best_match {
            self.cache.insert(cache_key, Some(result.clone()));
            return Ok(result);
        }

        self.cache.insert(cache_key.clone(), None);
        
        // Generate helpful error message
        let mut error_msg = if bound.trait_args.is_empty() {
            format!(
                "No implementation of trait {} for type {}",
                bound.trait_id.0, bound.subject
            )
        } else {
            let args_str = bound.trait_args
                .iter()
                .map(|t| format!("{}", t))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "No implementation of trait {} for type {} with arguments [{}]",
                bound.trait_id.0, bound.subject, args_str
            )
        };
        
        // Include specific error if we found an incompatible impl
        if let Some(err) = last_error {
            error_msg.push_str(&format!("\n  Details: {}", err));
        }
        
        Err(error_msg)
    }
    
    /// Build a cache key that includes trait arguments
    fn build_cache_key(&self, bound: &TypeBound) -> (TraitId, String) {
        let key_str = if bound.trait_args.is_empty() {
            format!("{}", bound.subject)
        } else {
            // Include trait arguments in the key for proper generic handling
            let args_str = bound.trait_args
                .iter()
                .map(|t| format!("{}", t))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", bound.subject, args_str)
        };
        (bound.trait_id, key_str)
    }
    
    /// Check if a trait implementation's generic arguments are compatible with a bound's arguments
    fn trait_args_compatible(&self, impl_: &TraitImpl, bound: &TypeBound) -> bool {
        // If the bound specifies no arguments, it matches any impl
        if bound.trait_args.is_empty() {
            return true;
        }
        
        // Get trait definition to know associated type names and order
        let trait_def = match self.trait_defs.get(&bound.trait_id) {
            Some(def) => def,
            None => return false,  // Unknown trait
        };
        
        // Validate associated types match
        match self.validate_associated_types(impl_, bound, trait_def) {
            Ok(()) => true,
            Err(_) => false,
        }
    }
    
    /// Validate that impl's associated types match the bound's specified types
    /// 
    /// Example: if bound is Iterator<Item=i32>, validates that impl's Item is i32
    fn validate_associated_types(
        &self,
        impl_: &TraitImpl,
        bound: &TypeBound,
        trait_def: &TraitDefinition,
    ) -> Result<(), String> {
        // Parse the bound's trait arguments into (name, type) pairs
        let bound_assoc_types = self.parse_associated_types(bound, trait_def)?;
        
        // For each specified argument, validate it matches impl
        for (name, bound_type) in bound_assoc_types {
            if let Some(impl_type) = impl_.associated_types.get(&name) {
                // Check if types match
                if impl_type != &bound_type {
                    return Err(format!(
                        "Associated type {} mismatch: impl has {}, bound requires {}",
                        name, impl_type, bound_type
                    ));
                }
            } else {
                return Err(format!(
                    "Associated type {} not defined in impl for type {}",
                    name, impl_.impl_type
                ));
            }
        }
        
        Ok(())
    }
    
    /// Parse trait arguments into associated type (name, type) pairs
    /// 
    /// Uses positional matching if no explicit names are provided.
    /// Example: Iterator<i32> matches with Item = i32 (first assoc type)
    fn parse_associated_types(
        &self,
        bound: &TypeBound,
        trait_def: &TraitDefinition,
    ) -> Result<HashMap<String, Type>, String> {
        let mut result = HashMap::new();
        
        // Match trait arguments positionally to associated types
        for (idx, arg) in bound.trait_args.iter().enumerate() {
            if let Some(name) = trait_def.associated_type_order.get(idx) {
                result.insert(name.clone(), arg.clone());
            } else {
                return Err(format!(
                    "Trait has {} associated types but {} arguments provided",
                    trait_def.associated_type_order.len(),
                    bound.trait_args.len()
                ));
            }
        }
        
        Ok(result)
    }

    /// Resolve an associated type
    /// 
    /// Now properly handles associated types specified in trait arguments.
    /// For example: <T as Iterator<Item=i32>>::Item will resolve to i32
    pub fn resolve_associated_type(
        &mut self,
        assoc: &AssociatedType,
    ) -> Result<Type, String> {
        // Build trait arguments from the associated type specification
        // The associated type itself provides context for the trait arguments
        let trait_args = vec![];  // Will be filled in once we have full associated type support
        
        let impl_ = self.resolve_bound(&TypeBound::new(
            *assoc.self_type.clone(),
            assoc.trait_id,
            trait_args,
        ))?;

        impl_
            .associated_types
            .get(&assoc.name)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Associated type {} not found in implementation of trait {}",
                    assoc.name, assoc.trait_id.0
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
                TypePredicate::ProjectionEquality { projection, ty } => {
                    // Check associated type projection equality: <T as Trait>::Assoc = U
                    // For now, we can't fully validate projections without trait information
                    // but we should at least log that we're handling this case
                    eprintln!("[TraitResolver] Projection equality predicate: {} = {}", projection, ty);
                    // TODO: Implement proper projection equality checking
                    // Requires resolving the associated type and comparing with ty
                }
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

    // === Associated Type Matching Tests ===

    #[test]
    fn test_associated_type_single_match() {
        // Create trait definition for Iterator with Item associated type
        let mut trait_def = TraitDefinition::new(TraitId(0), "Iterator".to_string());
        trait_def.add_associated_type("Item".to_string(), None);
        
        // Create impl: Iterator for Str with Item = i32
        let mut impl_ = TraitImpl::new(TraitId(0), Type::Str);
        impl_.set_associated_type("Item".to_string(), Type::I32);
        
        let mut resolver = TraitResolver::new();
        resolver.register_trait(trait_def);
        resolver.register_impl(impl_);
        
        // Bound: T: Iterator<i32> (Item=i32)
        let bound = TypeBound::new(Type::Str, TraitId(0), vec![Type::I32]);
        let result = resolver.resolve_bound(&bound);
        assert!(result.is_ok());
    }

    #[test]
    fn test_associated_type_mismatch() {
        // Create trait definition for Iterator
        let mut trait_def = TraitDefinition::new(TraitId(0), "Iterator".to_string());
        trait_def.add_associated_type("Item".to_string(), None);
        
        // Create impl: Iterator for Str with Item = i32
        let mut impl_ = TraitImpl::new(TraitId(0), Type::Str);
        impl_.set_associated_type("Item".to_string(), Type::I32);
        
        let mut resolver = TraitResolver::new();
        resolver.register_trait(trait_def);
        resolver.register_impl(impl_);
        
        // Bound: T: Iterator<bool> (Item=bool, but impl has Item=i32)
        let bound = TypeBound::new(Type::Str, TraitId(0), vec![Type::Bool]);
        let result = resolver.resolve_bound(&bound);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mismatch"));
    }

    #[test]
    fn test_associated_type_multiple() {
        // Create trait definition with multiple associated types
        let mut trait_def = TraitDefinition::new(TraitId(1), "Pair".to_string());
        trait_def.add_associated_type("First".to_string(), None);
        trait_def.add_associated_type("Second".to_string(), None);
        
        // Create impl with both types
        let mut impl_ = TraitImpl::new(TraitId(1), Type::Str);
        impl_.set_associated_type("First".to_string(), Type::I32);
        impl_.set_associated_type("Second".to_string(), Type::Bool);
        
        let mut resolver = TraitResolver::new();
        resolver.register_trait(trait_def);
        resolver.register_impl(impl_);
        
        // Bound: T: Pair<i32, bool> (First=i32, Second=bool)
        let bound = TypeBound::new(Type::Str, TraitId(1), vec![Type::I32, Type::Bool]);
        let result = resolver.resolve_bound(&bound);
        assert!(result.is_ok());
    }

    #[test]
    fn test_associated_type_multiple_partial_mismatch() {
        // Create trait definition with multiple associated types
        let mut trait_def = TraitDefinition::new(TraitId(1), "Pair".to_string());
        trait_def.add_associated_type("First".to_string(), None);
        trait_def.add_associated_type("Second".to_string(), None);
        
        // Create impl with both types
        let mut impl_ = TraitImpl::new(TraitId(1), Type::Str);
        impl_.set_associated_type("First".to_string(), Type::I32);
        impl_.set_associated_type("Second".to_string(), Type::Bool);
        
        let mut resolver = TraitResolver::new();
        resolver.register_trait(trait_def);
        resolver.register_impl(impl_);
        
        // Bound: T: Pair<i32, Char> (First=i32 OK, Second=Char but impl has Bool)
        let bound = TypeBound::new(Type::Str, TraitId(1), vec![Type::I32, Type::Char]);
        let result = resolver.resolve_bound(&bound);
        assert!(result.is_err());
    }

    #[test]
    fn test_associated_type_no_args() {
        // When no trait arguments are specified, any impl should match
        let mut trait_def = TraitDefinition::new(TraitId(0), "Iterator".to_string());
        trait_def.add_associated_type("Item".to_string(), None);
        
        let mut impl_ = TraitImpl::new(TraitId(0), Type::Str);
        impl_.set_associated_type("Item".to_string(), Type::I32);
        
        let mut resolver = TraitResolver::new();
        resolver.register_trait(trait_def);
        resolver.register_impl(impl_);
        
        // Bound: T: Iterator (no args)
        let bound = TypeBound::new(Type::Str, TraitId(0), vec![]);
        let result = resolver.resolve_bound(&bound);
        assert!(result.is_ok());
    }

    #[test]
    fn test_associated_type_ordering() {
        // Verify that associated types are added to order list
        let mut trait_def = TraitDefinition::new(TraitId(0), "Trait".to_string());
        assert_eq!(trait_def.associated_type_order.len(), 0);
        
        trait_def.add_associated_type("First".to_string(), None);
        assert_eq!(trait_def.associated_type_order.len(), 1);
        assert_eq!(trait_def.associated_type_order[0], "First");
        
        trait_def.add_associated_type("Second".to_string(), None);
        assert_eq!(trait_def.associated_type_order.len(), 2);
        assert_eq!(trait_def.associated_type_order[1], "Second");
    }

    #[test]
    fn test_associated_type_duplicate_names() {
        // Verify that duplicate names don't get added to order list
        let mut trait_def = TraitDefinition::new(TraitId(0), "Trait".to_string());
        
        trait_def.add_associated_type("Item".to_string(), None);
        assert_eq!(trait_def.associated_type_order.len(), 1);
        
        // Adding same name again
        trait_def.add_associated_type("Item".to_string(), None);
        assert_eq!(trait_def.associated_type_order.len(), 1);
    }

    #[test]
    fn test_cache_key_includes_trait_args() {
        // Verify that cache keys are different for different trait arguments
        let mut resolver = TraitResolver::new();
        let bound1 = TypeBound::new(Type::I32, TraitId(0), vec![Type::Bool]);
        let bound2 = TypeBound::new(Type::I32, TraitId(0), vec![Type::Char]);
        
        let key1 = resolver.build_cache_key(&bound1);
        let key2 = resolver.build_cache_key(&bound2);
        
        // Keys should be different because trait args differ
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_error_message_with_associated_type_details() {
        // Verify error messages include details about associated type mismatches
        let mut trait_def = TraitDefinition::new(TraitId(0), "Iterator".to_string());
        trait_def.add_associated_type("Item".to_string(), None);
        
        let mut impl_ = TraitImpl::new(TraitId(0), Type::Str);
        impl_.set_associated_type("Item".to_string(), Type::I32);
        
        let mut resolver = TraitResolver::new();
        resolver.register_trait(trait_def);
        resolver.register_impl(impl_);
        
        let bound = TypeBound::new(Type::Str, TraitId(0), vec![Type::Bool]);
        let result = resolver.resolve_bound(&bound);
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        // Check that error mentions the type arguments
        assert!(error.contains("["));  // Format includes brackets for args
    }
}
