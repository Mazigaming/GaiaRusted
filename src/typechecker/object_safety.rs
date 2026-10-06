/// Object-Safety Validation for Trait Objects (dyn Trait)
///
/// This module implements validation rules to ensure trait methods are
/// object-safe, i.e., can be called through a dynamic trait object.
///
/// Rules for object-safety:
/// 1. Method must have &self or &mut self (not Self)
/// 2. Method cannot return Self
/// 3. Method cannot use Self in parameter positions (except first param)
/// 4. Method cannot have generic type parameters (besides Self)
use crate::lowering::{HirExpression, HirItem, HirStatement, HirType};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ObjectSafetyError {
    pub trait_name: String,
    pub method_name: String,
    pub reason: String,
}

/// Check if a method is object-safe
pub fn is_method_object_safe(method: &HirItem, trait_name: &str) -> Result<(), ObjectSafetyError> {
    match method {
        HirItem::Function {
            name: method_name,
            params,
            return_type,
            generics,
            ..
        } => {
            // Rule 1: Check self parameter
            if params.is_empty() {
                return Err(ObjectSafetyError {
                    trait_name: trait_name.to_string(),
                    method_name: method_name.clone(),
                    reason: "trait method must take &self or &mut self as first parameter"
                        .to_string(),
                });
            }

            let (self_param_name, self_param_type) = &params[0];

            // First parameter must be named "self"
            if self_param_name != "self" {
                return Err(ObjectSafetyError {
                    trait_name: trait_name.to_string(),
                    method_name: method_name.clone(),
                    reason: "first parameter must be 'self'".to_string(),
                });
            }

            // First parameter must be &self or &mut self (not Self, not owned)
            match self_param_type {
                HirType::Reference(inner) => {
                    // Good: &self is reference to some type
                    // We'll accept this - it's &T for some T
                }
                HirType::MutableReference(inner) => {
                    // Good: &mut self is mutable reference
                }
                _ => {
                    return Err(ObjectSafetyError {
                        trait_name: trait_name.to_string(),
                        method_name: method_name.clone(),
                        reason: "method receiver must be &self or &mut self (not owned Self)"
                            .to_string(),
                    });
                }
            }

            // Rule 2: Check return type - cannot be Self or any variant of Self
            if let Some(ret_type) = return_type {
                if contains_self_type(ret_type) {
                    return Err(ObjectSafetyError {
                        trait_name: trait_name.to_string(),
                        method_name: method_name.clone(),
                        reason: "method cannot return Self or a type containing Self".to_string(),
                    });
                }
            }

            // Rule 3: Check parameter types - cannot use Self except in first param
            for (i, (_param_name, param_type)) in params.iter().enumerate().skip(1) {
                if contains_self_type(param_type) {
                    return Err(ObjectSafetyError {
                        trait_name: trait_name.to_string(),
                        method_name: method_name.clone(),
                        reason: format!(
                            "method parameter {} cannot use Self (except first parameter)",
                            i + 1
                        ),
                    });
                }
            }

            // Rule 4: Check generic type parameters
            // Note: Methods can have generic parameters only in certain contexts
            // For now, we reject methods with explicit generic type parameters
            // (lifetimes are OK, but type parameters are not)
            if !generics.is_empty() {
                for generic in generics {
                    match generic {
                        // Type parameters are not object-safe
                        crate::parser::ast::GenericParam::Type { name, .. } => {
                            // Allow Self-bound generics in special cases,
                            // but for simplicity, reject all explicit type generics
                            return Err(ObjectSafetyError {
                                trait_name: trait_name.to_string(),
                                method_name: method_name.clone(),
                                reason: format!(
                                    "method cannot have generic type parameter '{}' (use concrete types instead)",
                                    name
                                ),
                            });
                        }
                        // Lifetime parameters are OK
                        crate::parser::ast::GenericParam::Lifetime(_) => {}
                        // Const parameters are not object-safe
                        crate::parser::ast::GenericParam::Const { name, .. } => {
                            return Err(ObjectSafetyError {
                                trait_name: trait_name.to_string(),
                                method_name: method_name.clone(),
                                reason: format!(
                                    "method cannot have const generic parameter '{}'",
                                    name
                                ),
                            });
                        }
                    }
                }
            }

            Ok(())
        }
        _ => Err(ObjectSafetyError {
            trait_name: trait_name.to_string(),
            method_name: "unknown".to_string(),
            reason: "expected a function item".to_string(),
        }),
    }
}

/// Check if a type contains Self
fn contains_self_type(ty: &HirType) -> bool {
    match ty {
        // Check for explicit Self references
        HirType::Named(name) => name == "Self",

        // Recursively check containers
        HirType::Reference(inner) => contains_self_type(inner),
        HirType::MutableReference(inner) => contains_self_type(inner),
        HirType::Pointer(inner) => contains_self_type(inner),
        HirType::Box(inner) => contains_self_type(inner),
        HirType::Vec(inner) => contains_self_type(inner),
        HirType::Option(inner) => contains_self_type(inner),

        HirType::Array { element_type, .. } => contains_self_type(element_type),

        HirType::Function {
            params,
            return_type,
        } => params.iter().any(|p| contains_self_type(p)) || contains_self_type(return_type),

        HirType::Tuple(types) => types.iter().any(|t| contains_self_type(t)),

        HirType::Result { ok_type, err_type } => {
            contains_self_type(ok_type) || contains_self_type(err_type)
        }

        // These don't contain Self
        HirType::Int32
        | HirType::Int64
        | HirType::UInt32
        | HirType::UInt64
        | HirType::USize
        | HirType::ISize
        | HirType::Float64
        | HirType::Bool
        | HirType::Char
        | HirType::String
        | HirType::Range
        | HirType::Unknown
        | HirType::DynTrait { .. }
        | HirType::Closure { .. } => false,
    }
}

/// Validate all methods in a trait are object-safe
pub fn validate_trait_object_safety(
    trait_name: &str,
    methods: &[HirItem],
) -> Result<(), Vec<ObjectSafetyError>> {
    let mut errors = Vec::new();

    for method in methods {
        match method {
            HirItem::Function {
                name: method_name, ..
            } => {
                if let Err(err) = is_method_object_safe(method, trait_name) {
                    errors.push(err);
                }
            }
            // Skip other item types (associated types, etc.)
            _ => {}
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_type_detection() {
        assert!(contains_self_type(&HirType::Named("Self".to_string())));
        assert!(!contains_self_type(&HirType::Named("String".to_string())));
        assert!(contains_self_type(&HirType::Reference(Box::new(
            HirType::Named("Self".to_string())
        ))));
    }
}
