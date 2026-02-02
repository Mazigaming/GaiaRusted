//! Type System Support for Trait Objects (dyn Trait)
//! Handles object safety checking and trait object type inference

use std::collections::HashSet;

/// Trait object type: dyn Trait
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DynTraitObject {
    pub trait_name: String,
    pub lifetime: Option<String>, // e.g., 'a in dyn Trait + 'a
}

impl DynTraitObject {
    pub fn new(trait_name: String) -> Self {
        DynTraitObject {
            trait_name,
            lifetime: None,
        }
    }

    pub fn with_lifetime(trait_name: String, lifetime: String) -> Self {
        DynTraitObject {
            trait_name,
            lifetime: Some(lifetime),
        }
    }

    /// Check if trait is object-safe
    pub fn is_object_safe(&self, methods: &[(String, bool)]) -> bool {
        // A trait is object-safe if:
        // 1. No methods have Self in parameters (except self)
        // 2. No generic parameters
        // 3. No Self type constraints
        for (method_name, _has_self_param) in methods {
            // For now, allow all methods
            // In full implementation, check Self constraints
        }
        true
    }
}

/// Fat pointer for trait objects: [data_ptr: *const T, vtable_ptr: *const VTable]
#[derive(Debug, Clone)]
pub struct FatPointer {
    pub data_type: String,      // The concrete type
    pub trait_obj: DynTraitObject,
    pub is_mutable: bool,
}

impl FatPointer {
    pub fn new(data_type: String, trait_obj: DynTraitObject) -> Self {
        FatPointer {
            data_type,
            trait_obj,
            is_mutable: false,
        }
    }

    pub fn mutable(data_type: String, trait_obj: DynTraitObject) -> Self {
        FatPointer {
            data_type,
            trait_obj,
            is_mutable: true,
        }
    }

    /// Fat pointer size: always 16 bytes (2 x 8-byte pointers)
    pub fn size() -> usize {
        16
    }

    /// Alignment requirement for fat pointers
    pub fn alignment() -> usize {
        8
    }
}

/// Validator for object-safe traits
pub struct ObjectSafetyValidator {
    object_safe_traits: HashSet<String>,
    non_object_safe: HashSet<String>,
}

impl ObjectSafetyValidator {
    pub fn new() -> Self {
        ObjectSafetyValidator {
            object_safe_traits: HashSet::new(),
            non_object_safe: HashSet::new(),
        }
    }

    /// Check if a trait can be used as a trait object
    pub fn validate_trait(&mut self, trait_name: &str) -> bool {
        if self.object_safe_traits.contains(trait_name) {
            return true;
        }
        if self.non_object_safe.contains(trait_name) {
            return false;
        }

        // Mark as safe (in full impl, do actual checking)
        self.object_safe_traits.insert(trait_name.to_string());
        true
    }

    /// Get all object-safe traits
    pub fn get_object_safe_traits(&self) -> Vec<String> {
        self.object_safe_traits.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trait_object_creation() {
        let obj = DynTraitObject::new("Display".to_string());
        assert_eq!(obj.trait_name, "Display");
        assert_eq!(obj.lifetime, None);
    }

    #[test]
    fn test_trait_object_with_lifetime() {
        let obj = DynTraitObject::with_lifetime("Iterator".to_string(), "'a".to_string());
        assert_eq!(obj.trait_name, "Iterator");
        assert_eq!(obj.lifetime, Some("'a".to_string()));
    }

    #[test]
    fn test_fat_pointer_size() {
        assert_eq!(FatPointer::size(), 16);
        assert_eq!(FatPointer::alignment(), 8);
    }

    #[test]
    fn test_object_safety_validator() {
        let mut validator = ObjectSafetyValidator::new();
        assert!(validator.validate_trait("Display"));
        assert!(validator.validate_trait("Display")); // Should use cache
        assert_eq!(validator.get_object_safe_traits().len(), 1);
    }
}
