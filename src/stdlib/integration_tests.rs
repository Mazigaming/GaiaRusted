//! # Standard Library Integration Tests
//!
//! Tests that verify String, Vec, and other stdlib types integrate
//! properly with the compiler's type system.

#[cfg(test)]
mod tests {
    use crate::typesystem::types::Type;
    use crate::stdlib::method_resolution::StdlibMethodResolver;

    #[test]
    fn test_string_type_creation() {
        let string_type = Type::String;
        assert!(string_type.is_string());
        assert!(string_type.is_collection());
        assert!(!string_type.is_vec());
    }

    #[test]
    fn test_vec_type_creation() {
        let vec_i32 = Type::Vec(Box::new(Type::I32));
        assert!(vec_i32.is_vec());
        assert!(vec_i32.is_collection());
        assert!(!vec_i32.is_string());
    }

    #[test]
    fn test_vec_element_type_extraction() {
        let vec_str = Type::Vec(Box::new(Type::Str));
        assert_eq!(vec_str.vec_element_type(), Some(&Type::Str));
    }

    #[test]
    fn test_string_display() {
        let string_type = Type::String;
        assert_eq!(string_type.to_string(), "String");
    }

    #[test]
    fn test_vec_display() {
        let vec_i32 = Type::Vec(Box::new(Type::I32));
        assert_eq!(vec_i32.to_string(), "Vec<i32>");

        let vec_bool = Type::Vec(Box::new(Type::Bool));
        assert_eq!(vec_bool.to_string(), "Vec<bool>");
    }

    #[test]
    fn test_method_resolution_on_string() {
        let string_type = Type::String;

        // Test various String methods
        let len_method = StdlibMethodResolver::resolve_method(&string_type, "len");
        assert!(len_method.is_some());
        assert_eq!(len_method.unwrap().return_type, Type::Usize);

        let push_method = StdlibMethodResolver::resolve_method(&string_type, "push");
        assert!(push_method.is_some());
        assert!(push_method.unwrap().is_mutable);

        let to_upper = StdlibMethodResolver::resolve_method(&string_type, "to_uppercase");
        assert!(to_upper.is_some());
        assert_eq!(to_upper.unwrap().return_type, Type::String);
    }

    #[test]
    fn test_method_resolution_on_vec() {
        let vec_i32 = Type::Vec(Box::new(Type::I32));

        // Test Vec methods
        let len_method = StdlibMethodResolver::resolve_method(&vec_i32, "len");
        assert!(len_method.is_some());
        assert_eq!(len_method.unwrap().return_type, Type::Usize);

        let push_method = StdlibMethodResolver::resolve_method(&vec_i32, "push");
        assert!(push_method.is_some());
        assert!(push_method.unwrap().is_mutable);

        let pop_method = StdlibMethodResolver::resolve_method(&vec_i32, "pop");
        assert!(pop_method.is_some());
    }

    #[test]
    fn test_nested_vec_type() {
        let vec_vec_i32 = Type::Vec(Box::new(Type::Vec(Box::new(Type::I32))));
        assert_eq!(vec_vec_i32.to_string(), "Vec<Vec<i32>>");
        assert!(vec_vec_i32.is_vec());
    }

    #[test]
    fn test_vec_of_string() {
        let vec_string = Type::Vec(Box::new(Type::String));
        assert_eq!(vec_string.to_string(), "Vec<String>");
        assert!(vec_string.is_collection());
    }

    #[test]
    fn test_type_equality_for_strings() {
        let s1 = Type::String;
        let s2 = Type::String;
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_type_inequality_string_vs_str() {
        let string = Type::String;
        let str_type = Type::Str;
        assert_ne!(string, str_type);
    }

    #[test]
    fn test_type_equality_for_vecs() {
        let v1 = Type::Vec(Box::new(Type::I32));
        let v2 = Type::Vec(Box::new(Type::I32));
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_type_inequality_different_vec_elements() {
        let vec_i32 = Type::Vec(Box::new(Type::I32));
        let vec_i64 = Type::Vec(Box::new(Type::I64));
        assert_ne!(vec_i32, vec_i64);
    }

    #[test]
    fn test_collection_predicate() {
        assert!(Type::String.is_collection());
        assert!(Type::Vec(Box::new(Type::I32)).is_collection());
        assert!(!Type::I32.is_collection());
        assert!(!Type::Bool.is_collection());
        assert!(!Type::Str.is_collection()); // &str is not owned collection
    }

    #[test]
    fn test_string_methods_are_resolvable() {
        let string_type = Type::String;
        
        // Test that all common methods are resolvable
        let methods = vec!["new", "from", "len", "is_empty", "push", "pop", "clear",
                          "contains", "starts_with", "ends_with", "find",
                          "to_uppercase", "to_lowercase", "trim"];
        
        for method_name in methods {
            let result = StdlibMethodResolver::resolve_method(&string_type, method_name);
            assert!(result.is_some(), "Method {} should be resolvable", method_name);
        }
    }

    #[test]
    fn test_vec_methods_are_resolvable() {
        let vec_i32 = Type::Vec(Box::new(Type::I32));
        
        // Test that all common methods are resolvable
        let methods = vec!["new", "len", "is_empty", "push", "pop", "clear",
                          "get", "first", "last", "sort", "reverse"];
        
        for method_name in methods {
            let result = StdlibMethodResolver::resolve_method(&vec_i32, method_name);
            assert!(result.is_some(), "Method {} should be resolvable on Vec<i32>", method_name);
        }
    }

    #[test]
    fn test_string_method_mutability() {
        let string_type = Type::String;
        
        // Methods that don't modify
        let len = StdlibMethodResolver::resolve_method(&string_type, "len").unwrap();
        assert!(!len.is_mutable);
        
        let contains = StdlibMethodResolver::resolve_method(&string_type, "contains").unwrap();
        assert!(!contains.is_mutable);
        
        // Methods that do modify
        let push = StdlibMethodResolver::resolve_method(&string_type, "push").unwrap();
        assert!(push.is_mutable);
        
        let clear = StdlibMethodResolver::resolve_method(&string_type, "clear").unwrap();
        assert!(clear.is_mutable);
    }

    #[test]
    fn test_vec_method_mutability() {
        let vec_i32 = Type::Vec(Box::new(Type::I32));
        
        // Methods that don't modify
        let len = StdlibMethodResolver::resolve_method(&vec_i32, "len").unwrap();
        assert!(!len.is_mutable);
        
        let first = StdlibMethodResolver::resolve_method(&vec_i32, "first").unwrap();
        assert!(!first.is_mutable);
        
        // Methods that do modify
        let push = StdlibMethodResolver::resolve_method(&vec_i32, "push").unwrap();
        assert!(push.is_mutable);
        
        let sort = StdlibMethodResolver::resolve_method(&vec_i32, "sort").unwrap();
        assert!(sort.is_mutable);
    }
}
