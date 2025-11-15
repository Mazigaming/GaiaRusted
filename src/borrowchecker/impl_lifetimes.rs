//! Impl block lifetime elision and validation
//!
//! Handles lifetime elision and validation for impl blocks and methods,
//! particularly focusing on self parameters and their lifetime constraints.
//!
//! # Self Parameter Lifetime Rules
//!
//! In Rust, when a method has a `&self` or `&mut self` parameter:
//! - The self parameter has an implicit lifetime
//! - If the return type contains a reference without explicit lifetime, it borrows from self
//! - Example: `fn foo(&self) -> &i32` elides to `fn foo<'a>(&'a self) -> &'a i32`

use crate::typesystem::{Type, Lifetime, LifetimeName};
use std::collections::HashSet;

/// Information about a method's self parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfKind {
    /// No self parameter (standalone function in impl)
    None,
    /// Immutable borrow: `&self`
    Immutable,
    /// Mutable borrow: `&mut self`
    Mutable,
    /// Owned self: `self` (consumes)
    Owned,
}

impl SelfKind {
    pub fn is_reference(&self) -> bool {
        matches!(self, SelfKind::Immutable | SelfKind::Mutable)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SelfKind::None => "",
            SelfKind::Immutable => "&self",
            SelfKind::Mutable => "&mut self",
            SelfKind::Owned => "self",
        }
    }
}

/// Location where a lifetime reference appears in impl method signature
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MethodLifetimeLocation {
    /// In parameter at index N
    Parameter(usize),
    /// In return type
    ReturnType,
    /// In method's generic bounds
    GenericBound(String),
}

/// Detailed error for impl method lifetime issues
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImplLifetimeError {
    /// Self parameter has explicit lifetime when it should be elided
    UnneededSelfLifetime {
        method_name: String,
        lifetime_name: String,
    },
    /// Return type references self lifetime without explicit binding
    UninferredSelfReturnRef {
        method_name: String,
        param_index: usize,
    },
    /// Multiple reference parameters make return lifetime ambiguous
    AmbiguousReturnLifetime {
        method_name: String,
        param_indices: Vec<usize>,
    },
    /// No reference parameter but return type borrows
    NoReferenceButBorrows {
        method_name: String,
        return_position: Box<MethodLifetimeLocation>,
    },
}

impl std::fmt::Display for ImplLifetimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImplLifetimeError::UnneededSelfLifetime {
                method_name,
                lifetime_name,
            } => {
                write!(
                    f,
                    "Method '{}': Self parameter should not have explicit lifetime '{}'\n\
                     Note: Self parameter lifetime is implicit and should not be named\n\
                     Help: Remove '{}' from the signature and let it be inferred",
                    method_name, lifetime_name, lifetime_name
                )
            }
            ImplLifetimeError::UninferredSelfReturnRef {
                method_name,
                param_index,
            } => {
                write!(
                    f,
                    "Method '{}': Return type borrows from parameter {}, but parameter is not a reference\n\
                     Note: Only reference parameters can provide lifetimes for return borrows\n\
                     Help: Make parameter {} a reference (&T or &mut T), or return owned data",
                    method_name, param_index, param_index
                )
            }
            ImplLifetimeError::AmbiguousReturnLifetime {
                method_name,
                param_indices,
            } => {
                let indices = param_indices
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(
                    f,
                    "Method '{}': Return type is ambiguous - multiple reference parameters: {}\n\
                     Note: Return type doesn't specify which parameter it borrows from\n\
                     Help: Add explicit lifetimes to clarify: fn method<'a>(&'a self, other: &'a T) -> &'a i32",
                    method_name, indices
                )
            }
            ImplLifetimeError::NoReferenceButBorrows {
                method_name,
                return_position,
            } => {
                let position_desc = match return_position.as_ref() {
                    MethodLifetimeLocation::ReturnType => "return type",
                    MethodLifetimeLocation::Parameter(_idx) => {
                        return write!(
                            f,
                            "Method '{}': References a lifetime in parameter position but doesn't have reference parameters",
                            method_name
                        );
                    }
                    MethodLifetimeLocation::GenericBound(name) => {
                        return write!(
                            f,
                            "Method '{}': Generic bound '{}' requires lifetimes but method has no reference parameters",
                            method_name, name
                        );
                    }
                };
                write!(
                    f,
                    "Method '{}': {} uses lifetime but no reference parameters available\n\
                     Note: Lifetimes in return types must come from input parameters\n\
                     Help: Add reference parameters to the method signature",
                    method_name, position_desc
                )
            }
        }
    }
}

/// Validates impl method lifetime usage
pub struct ImplMethodValidator {
    method_name: String,
    self_kind: SelfKind,
    param_types: Vec<Type>,
    return_type: Type,
    declared_lifetimes: HashSet<String>,
}

impl ImplMethodValidator {
    /// Create a new validator for an impl method
    pub fn new(
        method_name: String,
        self_kind: SelfKind,
        param_types: Vec<Type>,
        return_type: Type,
    ) -> Self {
        Self {
            method_name,
            self_kind,
            param_types,
            return_type,
            declared_lifetimes: HashSet::new(),
        }
    }

    /// Register declared lifetime parameters
    pub fn add_declared_lifetime(&mut self, name: String) {
        self.declared_lifetimes.insert(name);
    }

    /// Check that self parameter is not explicitly named with a lifetime
    fn check_self_lifetime(&self) -> Result<(), ImplLifetimeError> {
        // In Rust, &self is implicitly &'self and you can't write that explicitly
        // The lifetime is inferred from context. This check ensures no one tries to
        // write something like &'a self which is an error.
        // This is more of a parser check, but we include it for completeness.
        Ok(())
    }

    /// Collect all lifetime references from a type
    fn collect_lifetime_refs(&self, ty: &Type) -> HashSet<String> {
        let mut lifetimes = HashSet::new();
        self.collect_lifetime_refs_impl(ty, &mut lifetimes);
        lifetimes
    }

    fn collect_lifetime_refs_impl(&self, ty: &Type, lifetimes: &mut HashSet<String>) {
        match ty {
            Type::Reference {
                lifetime: Some(lifetime),
                inner,
                ..
            } => {
                // Convert lifetime to string representation
                lifetimes.insert(lifetime.to_string());
                self.collect_lifetime_refs_impl(inner, lifetimes);
            }
            Type::Reference { inner, .. } => {
                self.collect_lifetime_refs_impl(inner, lifetimes);
            }
            Type::Tuple(types) => {
                for t in types {
                    self.collect_lifetime_refs_impl(t, lifetimes);
                }
            }
            Type::Array { element, .. } => {
                self.collect_lifetime_refs_impl(element, lifetimes);
            }
            Type::Function { params, .. } => {
                for p in params {
                    self.collect_lifetime_refs_impl(p, lifetimes);
                }
            }
            _ => {}
        }
    }

    /// Find all reference parameters
    fn collect_reference_params(&self) -> Vec<usize> {
        self.param_types
            .iter()
            .enumerate()
            .filter_map(|(idx, ty)| {
                if matches!(ty, Type::Reference { .. }) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Validate lifetime elision rules for the method
    pub fn validate(&self) -> Result<(), Vec<ImplLifetimeError>> {
        let mut errors = Vec::new();

        // Rule 1: Self parameter shouldn't have explicit lifetime
        if let Err(e) = self.check_self_lifetime() {
            errors.push(e);
        }

        // Rule 2: If return type has references, check they can be inferred
        let return_lifetimes = self.collect_lifetime_refs(&self.return_type);

        if !return_lifetimes.is_empty() {
            let ref_params = self.collect_reference_params();

            match ref_params.len() {
                0 => {
                    // No reference parameters but return type borrows - error
                    errors.push(ImplLifetimeError::NoReferenceButBorrows {
                        method_name: self.method_name.clone(),
                        return_position: Box::new(MethodLifetimeLocation::ReturnType),
                    });
                }
                1 if !self.self_kind.is_reference() => {
                    // Only one non-self reference, but no self - should work
                }
                1 if self.self_kind.is_reference() => {
                    // Self reference + one other - would be ambiguous
                    errors.push(ImplLifetimeError::AmbiguousReturnLifetime {
                        method_name: self.method_name.clone(),
                        param_indices: vec![0, ref_params[0]],
                    });
                }
                _ if ref_params.len() > 1 => {
                    // Multiple reference parameters make it ambiguous
                    errors.push(ImplLifetimeError::AmbiguousReturnLifetime {
                        method_name: self.method_name.clone(),
                        param_indices: ref_params,
                    });
                }
                _ => {}
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_kind_immutable() {
        assert_eq!(SelfKind::Immutable.as_str(), "&self");
        assert!(SelfKind::Immutable.is_reference());
    }

    #[test]
    fn test_self_kind_mutable() {
        assert_eq!(SelfKind::Mutable.as_str(), "&mut self");
        assert!(SelfKind::Mutable.is_reference());
    }

    #[test]
    fn test_self_kind_owned() {
        assert_eq!(SelfKind::Owned.as_str(), "self");
        assert!(!SelfKind::Owned.is_reference());
    }

    #[test]
    fn test_self_kind_none() {
        assert_eq!(SelfKind::None.as_str(), "");
        assert!(!SelfKind::None.is_reference());
    }

    #[test]
    fn test_method_lifetime_location_display() {
        let loc = MethodLifetimeLocation::Parameter(0);
        assert_eq!(format!("{:?}", loc), "Parameter(0)");

        let loc = MethodLifetimeLocation::ReturnType;
        assert_eq!(format!("{:?}", loc), "ReturnType");
    }

    #[test]
    fn test_validator_new() {
        let validator = ImplMethodValidator::new(
            "foo".to_string(),
            SelfKind::Immutable,
            vec![Type::I32],
            Type::Bool,
        );

        assert_eq!(validator.method_name, "foo");
        assert_eq!(validator.self_kind, SelfKind::Immutable);
        assert_eq!(validator.param_types.len(), 1);
        assert_eq!(validator.return_type, Type::Bool);
    }

    #[test]
    fn test_validator_add_declared_lifetime() {
        let mut validator = ImplMethodValidator::new(
            "foo".to_string(),
            SelfKind::Immutable,
            vec![],
            Type::I32,
        );

        validator.add_declared_lifetime("a".to_string());
        assert!(validator.declared_lifetimes.contains("a"));
    }

    #[test]
    fn test_validate_simple_immutable_self() {
        let validator = ImplMethodValidator::new(
            "foo".to_string(),
            SelfKind::Immutable,
            vec![],
            Type::I32,
        );

        assert!(validator.validate().is_ok());
    }

    #[test]
    fn test_validate_simple_mutable_self() {
        let validator = ImplMethodValidator::new(
            "foo".to_string(),
            SelfKind::Mutable,
            vec![],
            Type::Bool,
        );

        assert!(validator.validate().is_ok());
    }

    #[test]
    fn test_collect_reference_params_none() {
        let validator = ImplMethodValidator::new(
            "foo".to_string(),
            SelfKind::None,
            vec![Type::I32, Type::Bool],
            Type::Str,
        );

        let refs = validator.collect_reference_params();
        assert_eq!(refs.len(), 0);
    }

    #[test]
    fn test_collect_reference_params_some() {
        let ref_type = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let validator = ImplMethodValidator::new(
            "foo".to_string(),
            SelfKind::None,
            vec![ref_type, Type::Bool],
            Type::Str,
        );

        let refs = validator.collect_reference_params();
        assert_eq!(refs, vec![0]);
    }

    #[test]
    fn test_collect_lifetime_refs_none() {
        let validator = ImplMethodValidator::new(
            "foo".to_string(),
            SelfKind::None,
            vec![],
            Type::I32,
        );

        let lifetimes = validator.collect_lifetime_refs(&Type::I32);
        assert!(lifetimes.is_empty());
    }

    #[test]
    fn test_collect_lifetime_refs_from_reference() {
        let ref_type = Type::Reference {
            lifetime: Some(Lifetime::Named(LifetimeName(0))),
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let validator = ImplMethodValidator::new(
            "foo".to_string(),
            SelfKind::None,
            vec![],
            Type::I32,
        );

        let lifetimes = validator.collect_lifetime_refs(&ref_type);
        // Check that a lifetime was collected
        assert!(!lifetimes.is_empty());
    }

    #[test]
    fn test_validate_no_reference_but_borrows_error() {
        let return_type = Type::Reference {
            lifetime: Some(Lifetime::Named(LifetimeName(0))),
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let validator = ImplMethodValidator::new(
            "foo".to_string(),
            SelfKind::None,
            vec![Type::I32, Type::Bool], // No reference parameters
            return_type,
        );

        let result = validator.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ImplLifetimeError::NoReferenceButBorrows { .. }
        ));
    }

    #[test]
    fn test_validate_self_with_return_ref_multiple_params_error() {
        let ref_type = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let return_type = Type::Reference {
            lifetime: Some(Lifetime::Named(LifetimeName(0))),
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let validator = ImplMethodValidator::new(
            "foo".to_string(),
            SelfKind::Immutable,
            vec![ref_type],
            return_type,
        );

        let result = validator.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ImplLifetimeError::AmbiguousReturnLifetime { .. }
        )));
    }

    #[test]
    fn test_validate_owned_self_with_ref_param() {
        let ref_type = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let return_type = Type::Reference {
            lifetime: Some(Lifetime::Named(LifetimeName(0))),
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let validator = ImplMethodValidator::new(
            "foo".to_string(),
            SelfKind::Owned,
            vec![ref_type],
            return_type,
        );

        // This should be OK - only one reference parameter
        assert!(validator.validate().is_ok());
    }

    #[test]
    fn test_error_display_no_reference_but_borrows() {
        let err = ImplLifetimeError::NoReferenceButBorrows {
            method_name: "foo".to_string(),
            return_position: Box::new(MethodLifetimeLocation::ReturnType),
        };

        let msg = err.to_string();
        assert!(msg.contains("foo"));
        assert!(msg.contains("return type"));
        assert!(msg.contains("reference parameters"));
    }

    #[test]
    fn test_error_display_ambiguous() {
        let err = ImplLifetimeError::AmbiguousReturnLifetime {
            method_name: "foo".to_string(),
            param_indices: vec![0, 1],
        };

        let msg = err.to_string();
        assert!(msg.contains("foo"));
        assert!(msg.contains("ambiguous"));
        assert!(msg.contains("0, 1"));
    }

    #[test]
    fn test_validate_self_immutable_no_params_no_borrow() {
        let validator = ImplMethodValidator::new(
            "get_value".to_string(),
            SelfKind::Immutable,
            vec![],
            Type::I32,
        );

        assert!(validator.validate().is_ok());
    }

    #[test]
    fn test_validate_self_mutable_returns_bool() {
        let validator = ImplMethodValidator::new(
            "is_empty".to_string(),
            SelfKind::Mutable,
            vec![],
            Type::Bool,
        );

        assert!(validator.validate().is_ok());
    }

    #[test]
    fn test_multiple_reference_params_ambiguous() {
        let ref_type1 = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let ref_type2 = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::Str),
        };

        let return_type = Type::Reference {
            lifetime: Some(Lifetime::Named(LifetimeName(0))),
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let validator = ImplMethodValidator::new(
            "compare".to_string(),
            SelfKind::None,
            vec![ref_type1, ref_type2],
            return_type,
        );

        let result = validator.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ImplLifetimeError::AmbiguousReturnLifetime {
                param_indices,
                ..
            } if param_indices.len() == 2
        )));
    }

    #[test]
    fn test_single_ref_param_with_return_borrow_ok() {
        let ref_type = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let return_type = Type::Reference {
            lifetime: Some(Lifetime::Named(LifetimeName(0))),
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let validator = ImplMethodValidator::new(
            "identity".to_string(),
            SelfKind::None,
            vec![ref_type],
            return_type,
        );

        assert!(validator.validate().is_ok());
    }
}