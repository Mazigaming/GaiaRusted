//! # Standard Library Integration into Type Checker
//!
//! Hooks stdlib method resolution into the type checking pipeline.
//! Handles method calls on String, Vec, and other stdlib types.

use crate::lowering::HirType;
use crate::stdlib::method_resolution::{StdlibMethodResolver, MethodInfo};
use crate::typesystem::types::Type;
use std::collections::HashMap;

/// Converts HirType to Type for stdlib method resolution
pub fn hirtype_to_type(hir_ty: &HirType) -> Option<Type> {
    match hir_ty {
        HirType::Int32 => Some(Type::I32),
        HirType::Int64 => Some(Type::I64),
        HirType::UInt32 => Some(Type::U32),
        HirType::UInt64 => Some(Type::U64),
        HirType::USize => Some(Type::Usize),
        HirType::ISize => Some(Type::Isize),
        HirType::Float64 => Some(Type::F64),
        HirType::Bool => Some(Type::Bool),
        HirType::Char => Some(Type::Char),
        HirType::String => Some(Type::String),
        HirType::Vec(element) => {
            hirtype_to_type(element).map(|elem_type| Type::Vec(Box::new(elem_type)))
        }
        _ => None,
    }
}

/// Converts Type to HirType for type checking
pub fn type_to_hirtype(ty: &Type) -> Option<HirType> {
    match ty {
        Type::I32 => Some(HirType::Int32),
        Type::I64 => Some(HirType::Int64),
        Type::U32 => Some(HirType::UInt32),
        Type::U64 => Some(HirType::UInt64),
        Type::Usize => Some(HirType::USize),
        Type::Isize => Some(HirType::ISize),
        Type::F64 => Some(HirType::Float64),
        Type::Bool => Some(HirType::Bool),
        Type::Char => Some(HirType::Char),
        Type::String => Some(HirType::String),
        Type::Unit => Some(HirType::Tuple(vec![])), // Unit = empty tuple
        Type::Vec(inner) => {
            type_to_hirtype(inner).map(|elem_type| HirType::Vec(Box::new(elem_type)))
        }
        Type::Reference { lifetime: _, mutable, inner } => {
            type_to_hirtype(inner).map(|elem_type| {
                if *mutable {
                    HirType::MutableReference(Box::new(elem_type))
                } else {
                    HirType::Reference(Box::new(elem_type))
                }
            })
        }
        _ => None,
    }
}

/// Resolves a method call on a stdlib type
pub fn resolve_stdlib_method(
    object_type: &HirType,
    method_name: &str,
) -> Option<(HirType, bool)> {
    // Convert HirType to Type for resolution
    let ty = hirtype_to_type(object_type)?;

    // Resolve method using stdlib resolver
    let method_info = StdlibMethodResolver::resolve_method(&ty, method_name)?;

    // Convert return type back to HirType
    let return_hir_type = type_to_hirtype(&method_info.return_type)?;

    // Return (return_type, is_mutable)
    Some((return_hir_type, method_info.is_mutable))
}

/// Validates a method call is valid
pub fn validate_method_call(
    object_type: &HirType,
    method_name: &str,
    is_mutable_context: bool,
) -> Result<HirType, String> {
    // Try to resolve the method
    let (return_type, requires_mut) = resolve_stdlib_method(object_type, method_name)
        .ok_or_else(|| {
            format!(
                "method `{}` not found for type `{}`",
                method_name, object_type
            )
        })?;

    // Check mutability if needed
    if requires_mut && !is_mutable_context {
        return Err(format!(
            "cannot call mutable method `{}` on immutable binding",
            method_name
        ));
    }

    Ok(return_type)
}

/// Get all available methods for a type
pub fn get_available_methods(hir_ty: &HirType) -> Vec<String> {
    match hir_ty {
        HirType::String => {
            vec![
                "new", "from", "len", "is_empty", "push", "push_str", "pop", "clear",
                "contains", "starts_with", "ends_with", "find", "to_uppercase", "to_lowercase", "trim",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect()
        }
        HirType::Vec(_) => {
            vec![
                "new", "len", "is_empty", "push", "pop", "clear", "get", "first", "last",
                "sort", "reverse",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect()
        }
        _ => vec![],
    }
}

/// Checks if a type is a stdlib collection type
pub fn is_stdlib_collection(hir_ty: &HirType) -> bool {
    matches!(hir_ty, HirType::String | HirType::Vec(_))
}

/// Checks if a type has stdlib methods
pub fn has_stdlib_methods(hir_ty: &HirType) -> bool {
    is_stdlib_collection(hir_ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hirtype_string_to_type() {
        let hir_string = HirType::String;
        let ty = hirtype_to_type(&hir_string);
        assert!(ty.is_some());
        assert_eq!(ty.unwrap(), Type::String);
    }

    #[test]
    fn test_hirtype_vec_to_type() {
        let hir_vec = HirType::Vec(Box::new(HirType::Int32));
        let ty = hirtype_to_type(&hir_vec);
        assert!(ty.is_some());
        let Type::Vec(inner) = ty.unwrap() else {
            assert!(false, "Should be Vec");
        };
        assert_eq!(*inner, Type::I32);
        }

        #[test]
        fn test_type_string_to_hirtype() {
        let ty = Type::String;
        let hir_ty = type_to_hirtype(&ty);
        assert!(hir_ty.is_some());
        assert_eq!(hir_ty.unwrap(), HirType::String);
        }

        #[test]
        fn test_type_vec_to_hirtype() {
        let ty = Type::Vec(Box::new(Type::I32));
        let hir_ty = type_to_hirtype(&ty);
        assert!(hir_ty.is_some());
        match hir_ty.unwrap() {
            HirType::Vec(element) => {
                assert_eq!(*element, HirType::Int32);
            }
            _ => assert!(false, "Should be Vec"),
        }
        }

    #[test]
    fn test_hirtype_string_conversion() {
        // Test that HirType::String converts to Type::String
        let ty = hirtype_to_type(&HirType::String);
        assert!(ty.is_some());
        assert_eq!(ty.unwrap(), Type::String);
    }

    #[test]
    fn test_available_methods_collections() {
        // Test that collections have methods discoverable
        let string_methods = get_available_methods(&HirType::String);
        assert!(!string_methods.is_empty());
        assert!(string_methods.contains(&"push".to_string()));
        
        let vec_methods = get_available_methods(&HirType::Vec(Box::new(HirType::Int32)));
        assert!(!vec_methods.is_empty());
        assert!(vec_methods.contains(&"push".to_string()));
    }

    #[test]
    fn test_validate_method_call_success() {
        let hir_string = HirType::String;
        let result = validate_method_call(&hir_string, "len", false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), HirType::USize);
    }

    #[test]
    fn test_validate_method_call_mutable_fail() {
        let hir_string = HirType::String;
        let result = validate_method_call(&hir_string, "push", false);
        assert!(result.is_err(), "push should require &mut");
    }

    #[test]
    fn test_validate_method_call_mutable_success() {
        let hir_string = HirType::String;
        let result = validate_method_call(&hir_string, "len", true);
        // len doesn't require mut, should work either way
        assert!(result.is_ok(), "len should work on any binding");
    }

    #[test]
    fn test_get_available_methods_string() {
        let hir_string = HirType::String;
        let methods = get_available_methods(&hir_string);
        assert!(methods.contains(&"len".to_string()));
        assert!(methods.contains(&"push".to_string()));
    }

    #[test]
    fn test_get_available_methods_vec() {
        let hir_vec = HirType::Vec(Box::new(HirType::Int32));
        let methods = get_available_methods(&hir_vec);
        assert!(methods.contains(&"push".to_string()));
        assert!(methods.contains(&"len".to_string()));
    }

    #[test]
    fn test_is_stdlib_collection() {
        assert!(is_stdlib_collection(&HirType::String));
        assert!(is_stdlib_collection(&HirType::Vec(
            Box::new(HirType::Int32),
        )));
        assert!(!is_stdlib_collection(&HirType::Int32));
    }
}
