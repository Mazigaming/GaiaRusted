//! # Standard Library Method Resolution
//!
//! Integrates stdlib method calls with the type system.
//! Provides method lookup and signature resolution for String, Vec, and other stdlib types.

use crate::typesystem::types::Type;
use std::collections::HashMap;

/// Method information for resolution
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// Method name
    pub name: String,
    /// Method signature as string
    pub signature: String,
    /// Parameter types
    pub params: Vec<Type>,
    /// Return type
    pub return_type: Type,
    /// Whether method takes &mut self
    pub is_mutable: bool,
    /// Whether method consumes self (takes ownership)
    pub consumes_self: bool,
}

/// Resolves methods on stdlib types
pub struct StdlibMethodResolver;

impl StdlibMethodResolver {
    /// Resolve a method call on a type
    /// Returns MethodInfo if method exists
    pub fn resolve_method(type_: &Type, method_name: &str) -> Option<MethodInfo> {
        match type_ {
            Type::String => Self::resolve_string_method(method_name),
            Type::Vec(inner) => Self::resolve_vec_method(method_name, inner),
            _ => None,
        }
    }

    /// Resolve methods on String type
    fn resolve_string_method(method_name: &str) -> Option<MethodInfo> {
        match method_name {
            // Creation methods
            "new" => Some(MethodInfo {
                name: "new".to_string(),
                signature: "String::new() -> String".to_string(),
                params: vec![],
                return_type: Type::String,
                is_mutable: false,
                consumes_self: false,
            }),
            "from" => Some(MethodInfo {
                name: "from".to_string(),
                signature: "String::from(&str) -> String".to_string(),
                params: vec![Type::Str],
                return_type: Type::String,
                is_mutable: false,
                consumes_self: false,
            }),

            // Query methods
            "len" => Some(MethodInfo {
                name: "len".to_string(),
                signature: "String::len(&self) -> usize".to_string(),
                params: vec![],
                return_type: Type::Usize,
                is_mutable: false,
                consumes_self: false,
            }),
            "is_empty" => Some(MethodInfo {
                name: "is_empty".to_string(),
                signature: "String::is_empty(&self) -> bool".to_string(),
                params: vec![],
                return_type: Type::Bool,
                is_mutable: false,
                consumes_self: false,
            }),

            // Mutation methods
            "push" => Some(MethodInfo {
                name: "push".to_string(),
                signature: "String::push(&mut self, char)".to_string(),
                params: vec![Type::Char],
                return_type: Type::Unit,
                is_mutable: true,
                consumes_self: false,
            }),
            "push_str" => Some(MethodInfo {
                name: "push_str".to_string(),
                signature: "String::push_str(&mut self, &str)".to_string(),
                params: vec![Type::Str],
                return_type: Type::Unit,
                is_mutable: true,
                consumes_self: false,
            }),
            "pop" => Some(MethodInfo {
                name: "pop".to_string(),
                signature: "String::pop(&mut self) -> Option<char>".to_string(),
                params: vec![],
                return_type: Type::Char, // Simplified - should be Option<char>
                is_mutable: true,
                consumes_self: false,
            }),
            "clear" => Some(MethodInfo {
                name: "clear".to_string(),
                signature: "String::clear(&mut self)".to_string(),
                params: vec![],
                return_type: Type::Unit,
                is_mutable: true,
                consumes_self: false,
            }),

            // Search methods
            "contains" => Some(MethodInfo {
                name: "contains".to_string(),
                signature: "String::contains(&self, &str) -> bool".to_string(),
                params: vec![Type::Str],
                return_type: Type::Bool,
                is_mutable: false,
                consumes_self: false,
            }),
            "starts_with" => Some(MethodInfo {
                name: "starts_with".to_string(),
                signature: "String::starts_with(&self, &str) -> bool".to_string(),
                params: vec![Type::Str],
                return_type: Type::Bool,
                is_mutable: false,
                consumes_self: false,
            }),
            "ends_with" => Some(MethodInfo {
                name: "ends_with".to_string(),
                signature: "String::ends_with(&self, &str) -> bool".to_string(),
                params: vec![Type::Str],
                return_type: Type::Bool,
                is_mutable: false,
                consumes_self: false,
            }),
            "find" => Some(MethodInfo {
                name: "find".to_string(),
                signature: "String::find(&self, &str) -> Option<usize>".to_string(),
                params: vec![Type::Str],
                return_type: Type::Usize, // Simplified
                is_mutable: false,
                consumes_self: false,
            }),

            // Transform methods
            "to_uppercase" => Some(MethodInfo {
                name: "to_uppercase".to_string(),
                signature: "String::to_uppercase(&self) -> String".to_string(),
                params: vec![],
                return_type: Type::String,
                is_mutable: false,
                consumes_self: false,
            }),
            "to_lowercase" => Some(MethodInfo {
                name: "to_lowercase".to_string(),
                signature: "String::to_lowercase(&self) -> String".to_string(),
                params: vec![],
                return_type: Type::String,
                is_mutable: false,
                consumes_self: false,
            }),
            "trim" => Some(MethodInfo {
                name: "trim".to_string(),
                signature: "String::trim(&self) -> String".to_string(),
                params: vec![],
                return_type: Type::String,
                is_mutable: false,
                consumes_self: false,
            }),

            _ => None,
        }
    }

    /// Resolve methods on Vec<T> type
    fn resolve_vec_method(method_name: &str, element_type: &Type) -> Option<MethodInfo> {
        match method_name {
            // Creation methods
            "new" => Some(MethodInfo {
                name: "new".to_string(),
                signature: "Vec::new() -> Vec<T>".to_string(),
                params: vec![],
                return_type: Type::Vec(Box::new(element_type.clone())),
                is_mutable: false,
                consumes_self: false,
            }),

            // Query methods
            "len" => Some(MethodInfo {
                name: "len".to_string(),
                signature: "Vec::len(&self) -> usize".to_string(),
                params: vec![],
                return_type: Type::Usize,
                is_mutable: false,
                consumes_self: false,
            }),
            "is_empty" => Some(MethodInfo {
                name: "is_empty".to_string(),
                signature: "Vec::is_empty(&self) -> bool".to_string(),
                params: vec![],
                return_type: Type::Bool,
                is_mutable: false,
                consumes_self: false,
            }),

            // Mutation methods
            "push" => Some(MethodInfo {
                name: "push".to_string(),
                signature: "Vec::push(&mut self, T)".to_string(),
                params: vec![element_type.clone()],
                return_type: Type::Unit,
                is_mutable: true,
                consumes_self: false,
            }),
            "pop" => Some(MethodInfo {
                name: "pop".to_string(),
                signature: "Vec::pop(&mut self) -> Option<T>".to_string(),
                params: vec![],
                return_type: element_type.clone(), // Simplified
                is_mutable: true,
                consumes_self: false,
            }),
            "clear" => Some(MethodInfo {
                name: "clear".to_string(),
                signature: "Vec::clear(&mut self)".to_string(),
                params: vec![],
                return_type: Type::Unit,
                is_mutable: true,
                consumes_self: false,
            }),

            // Access methods
            "get" => Some(MethodInfo {
                name: "get".to_string(),
                signature: "Vec::get(&self, usize) -> Option<&T>".to_string(),
                params: vec![Type::Usize],
                return_type: Type::Reference {
                    lifetime: None,
                    mutable: false,
                    inner: Box::new(element_type.clone()),
                },
                is_mutable: false,
                consumes_self: false,
            }),
            "first" => Some(MethodInfo {
                name: "first".to_string(),
                signature: "Vec::first(&self) -> Option<&T>".to_string(),
                params: vec![],
                return_type: Type::Reference {
                    lifetime: None,
                    mutable: false,
                    inner: Box::new(element_type.clone()),
                },
                is_mutable: false,
                consumes_self: false,
            }),
            "last" => Some(MethodInfo {
                name: "last".to_string(),
                signature: "Vec::last(&self) -> Option<&T>".to_string(),
                params: vec![],
                return_type: Type::Reference {
                    lifetime: None,
                    mutable: false,
                    inner: Box::new(element_type.clone()),
                },
                is_mutable: false,
                consumes_self: false,
            }),

            // Utilities
            "sort" => Some(MethodInfo {
                name: "sort".to_string(),
                signature: "Vec::sort(&mut self)".to_string(),
                params: vec![],
                return_type: Type::Unit,
                is_mutable: true,
                consumes_self: false,
            }),
            "reverse" => Some(MethodInfo {
                name: "reverse".to_string(),
                signature: "Vec::reverse(&mut self)".to_string(),
                params: vec![],
                return_type: Type::Unit,
                is_mutable: true,
                consumes_self: false,
            }),

            _ => None,
        }
    }
}

/// Get all available methods for a type
pub fn get_available_methods(type_: &Type) -> Vec<String> {
    match type_ {
        Type::String => vec![
            "new", "from", "len", "is_empty", "push", "push_str", "pop", "clear",
            "contains", "starts_with", "ends_with", "find", "to_uppercase", "to_lowercase", "trim",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        Type::Vec(_) => vec![
            "new", "len", "is_empty", "push", "pop", "clear", "get", "first", "last",
            "sort", "reverse",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_string_len() {
        let method = StdlibMethodResolver::resolve_method(&Type::String, "len");
        assert!(method.is_some());
        let m = method.unwrap();
        assert_eq!(m.name, "len");
        assert_eq!(m.return_type, Type::Usize);
    }

    #[test]
    fn test_resolve_string_push() {
        let method = StdlibMethodResolver::resolve_method(&Type::String, "push");
        assert!(method.is_some());
        let m = method.unwrap();
        assert_eq!(m.name, "push");
        assert!(m.is_mutable);
    }

    #[test]
    fn test_resolve_vec_push() {
        let vec_i32 = Type::Vec(Box::new(Type::I32));
        let method = StdlibMethodResolver::resolve_method(&vec_i32, "push");
        assert!(method.is_some());
        let m = method.unwrap();
        assert_eq!(m.name, "push");
        assert!(m.is_mutable);
    }

    #[test]
    fn test_resolve_nonexistent_method() {
        let method = StdlibMethodResolver::resolve_method(&Type::String, "unknown");
        assert!(method.is_none());
    }

    #[test]
    fn test_string_available_methods() {
        let methods = get_available_methods(&Type::String);
        assert!(methods.contains(&"len".to_string()));
        assert!(methods.contains(&"push".to_string()));
        assert!(methods.contains(&"contains".to_string()));
    }

    #[test]
    fn test_vec_available_methods() {
        let vec_i32 = Type::Vec(Box::new(Type::I32));
        let methods = get_available_methods(&vec_i32);
        assert!(methods.contains(&"push".to_string()));
        assert!(methods.contains(&"pop".to_string()));
        assert!(methods.contains(&"sort".to_string()));
    }
}
