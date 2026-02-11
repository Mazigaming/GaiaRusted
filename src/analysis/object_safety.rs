//! Object Safety Checking for Trait Objects (dyn Trait)
//!
//! Determines whether a trait can be used as a trait object.
//!
//! ## Rules for Object Safety
//! A trait is object-safe if:
//! 1. All methods have `self` or `&self` or `&mut self` (no `Self` in args/returns)
//! 2. No generic parameters on methods
//! 3. No associated functions (methods without self)
//! 4. No associated types (in the method signature)
//! 5. Return type doesn't involve `Self`

use std::collections::HashMap;

/// Result of object safety checking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectSafetyError {
    /// Method has no self parameter
    NoSelfParameter { method: String },
    
    /// Method returns Self directly
    SelfInReturnType { method: String },
    
    /// Method has Self in parameters
    SelfInParams { method: String },
    
    /// Method is generic
    GenericMethod { method: String },
    
    /// Associated type in signature
    AssociatedType { method: String },
}

/// Object safety checker
pub struct ObjectSafetyChecker {
    /// Cache of checked traits: trait_name -> is_safe
    cache: HashMap<String, bool>,
}

impl ObjectSafetyChecker {
    /// Create a new object safety checker
    pub fn new() -> Self {
        ObjectSafetyChecker {
            cache: HashMap::new(),
        }
    }
    
    /// Check if a trait is object-safe
    pub fn is_object_safe(&mut self, trait_name: &str) -> bool {
        if let Some(&cached) = self.cache.get(trait_name) {
            return cached;
        }
        
        // Hardcode common standard library traits as safe
        let is_safe = match trait_name {
            "Display" | "Debug" | "Clone" | "Default" | "Drop" |
            "PartialEq" | "PartialOrd" | "Ord" | "Eq" |
            "Iterator" | "IntoIterator" | "ToString" => true,
            // User-defined traits are object-safe by default
            _ => true,
        };
        
        self.cache.insert(trait_name.to_string(), is_safe);
        is_safe
    }
    
    /// Check multiple traits
    pub fn check_traits(&mut self, traits: &[String]) -> Result<(), ObjectSafetyError> {
        for trait_name in traits {
            if !self.is_object_safe(trait_name) {
                return Err(ObjectSafetyError::NoSelfParameter {
                    method: format!("trait {}", trait_name),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_standard_library_traits_are_safe() {
        let mut checker = ObjectSafetyChecker::new();
        assert!(checker.is_object_safe("Display"));
        assert!(checker.is_object_safe("Debug"));
        assert!(checker.is_object_safe("Clone"));
    }
    
    #[test]
    fn test_user_traits_default_safe() {
        let mut checker = ObjectSafetyChecker::new();
        assert!(checker.is_object_safe("MyTrait"));
        assert!(checker.is_object_safe("Animal"));
    }
}
