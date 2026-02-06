//! String Literal Method Binding
//!
//! Handles direct method calls on string literals and string values.
//! Enables patterns like: "hello".len(), "world".to_upper(), etc.

use std::collections::HashMap;

/// String method definitions for builtin string operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringMethod {
    pub name: String,
    pub parameters: Vec<String>,
    pub return_type: String,
    pub builtin_name: String,
}

impl StringMethod {
    pub fn new(
        name: &str,
        parameters: Vec<String>,
        return_type: &str,
        builtin_name: &str,
    ) -> Self {
        StringMethod {
            name: name.to_string(),
            parameters,
            return_type: return_type.to_string(),
            builtin_name: builtin_name.to_string(),
        }
    }
}

/// Registry for string methods
pub struct StringMethodRegistry {
    methods: HashMap<String, StringMethod>,
}

impl StringMethodRegistry {
    pub fn new() -> Self {
        let mut registry = StringMethodRegistry {
            methods: HashMap::new(),
        };
        registry.init_builtin_methods();
        registry
    }

    /// Initialize builtin string methods
    fn init_builtin_methods(&mut self) {
        self.register_method(StringMethod::new(
            "len",
            vec![],
            "usize",
            "builtin_str_len",
        ));

        self.register_method(StringMethod::new(
            "to_upper",
            vec![],
            "String",
            "builtin_str_to_upper",
        ));

        self.register_method(StringMethod::new(
            "to_lowercase",
            vec![],
            "String",
            "builtin_str_to_lower",
        ));

        self.register_method(StringMethod::new(
            "to_lower",
            vec![],
            "String",
            "builtin_str_to_lower",
        ));

        self.register_method(StringMethod::new(
            "trim",
            vec![],
            "String",
            "builtin_str_trim",
        ));

        self.register_method(StringMethod::new(
            "is_empty",
            vec![],
            "bool",
            "builtin_str_is_empty",
        ));

        self.register_method(StringMethod::new(
            "contains",
            vec!["&str".to_string()],
            "bool",
            "builtin_str_contains",
        ));

        self.register_method(StringMethod::new(
            "starts_with",
            vec!["&str".to_string()],
            "bool",
            "builtin_str_starts_with",
        ));

        self.register_method(StringMethod::new(
            "ends_with",
            vec!["&str".to_string()],
            "bool",
            "builtin_str_ends_with",
        ));

        self.register_method(StringMethod::new(
            "split",
            vec!["char".to_string()],
            "Vec<String>",
            "builtin_str_split",
        ));

        self.register_method(StringMethod::new(
            "replace",
            vec!["&str".to_string(), "&str".to_string()],
            "String",
            "builtin_str_replace",
        ));

        self.register_method(StringMethod::new(
            "repeat",
            vec!["usize".to_string()],
            "String",
            "builtin_str_repeat",
        ));

        self.register_method(StringMethod::new(
            "chars",
            vec![],
            "Vec<char>",
            "builtin_str_chars",
        ));

        self.register_method(StringMethod::new(
            "parse",
            vec![],
            "Option<i64>",
            "builtin_str_parse",
        ));

        self.register_method(StringMethod::new(
            "strip_prefix",
            vec!["&str".to_string()],
            "Option<String>",
            "builtin_str_strip_prefix",
        ));

        self.register_method(StringMethod::new(
            "strip_suffix",
            vec!["&str".to_string()],
            "Option<String>",
            "builtin_str_strip_suffix",
        ));

        self.register_method(StringMethod::new(
            "split_whitespace",
            vec![],
            "Vec<String>",
            "builtin_str_split_whitespace",
        ));

        self.register_method(StringMethod::new(
            "lines",
            vec![],
            "Vec<String>",
            "builtin_str_lines",
        ));

        self.register_method(StringMethod::new(
            "as_bytes",
            vec![],
            "&[u8]",
            "builtin_str_as_bytes",
        ));

        self.register_method(StringMethod::new(
            "reverse",
            vec![],
            "String",
            "builtin_str_reverse",
        ));
    }

    /// Register a string method
    pub fn register_method(&mut self, method: StringMethod) {
        self.methods.insert(method.name.clone(), method);
    }

    /// Look up a string method
    pub fn get_method(&self, name: &str) -> Option<StringMethod> {
        self.methods.get(name).cloned()
    }

    /// Check if a method exists
    pub fn has_method(&self, name: &str) -> bool {
        self.methods.contains_key(name)
    }

    /// Get all method names
    pub fn method_names(&self) -> Vec<String> {
        self.methods.keys().cloned().collect()
    }

    /// Get builtin name for a method
    pub fn get_builtin_name(&self, method_name: &str) -> Option<String> {
        self.get_method(method_name).map(|m| m.builtin_name)
    }

    /// Validate method call parameters
    pub fn validate_call(
        &self,
        method_name: &str,
        arg_types: &[String],
    ) -> Result<String, String> {
        let method = self
            .get_method(method_name)
            .ok_or_else(|| format!("Unknown string method: {}", method_name))?;

        if arg_types.len() != method.parameters.len() {
            return Err(format!(
                "Method {} expects {} arguments, got {}",
                method_name,
                method.parameters.len(),
                arg_types.len()
            ));
        }

        for (i, (expected, actual)) in method.parameters.iter().zip(arg_types.iter()).enumerate() {
            if !self.types_compatible(expected, actual) {
                return Err(format!(
                    "Method {} argument {} type mismatch: expected {}, got {}",
                    method_name, i, expected, actual
                ));
            }
        }

        Ok(method.return_type)
    }

    /// Check type compatibility
    fn types_compatible(&self, expected: &str, actual: &str) -> bool {
        expected == actual || (expected == "&str" && actual == "String")
    }
}

impl Default for StringMethodRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Slice/Array method registry for completeness
pub struct SliceMethodRegistry {
    methods: HashMap<String, StringMethod>,
}

impl SliceMethodRegistry {
    pub fn new() -> Self {
        let mut registry = SliceMethodRegistry {
            methods: HashMap::new(),
        };
        registry.init_builtin_methods();
        registry
    }

    fn init_builtin_methods(&mut self) {
        self.register_method(StringMethod::new(
            "len",
            vec![],
            "usize",
            "builtin_slice_len",
        ));

        self.register_method(StringMethod::new(
            "is_empty",
            vec![],
            "bool",
            "builtin_slice_is_empty",
        ));

        self.register_method(StringMethod::new(
            "first",
            vec![],
            "Option<&T>",
            "builtin_slice_first",
        ));

        self.register_method(StringMethod::new(
            "last",
            vec![],
            "Option<&T>",
            "builtin_slice_last",
        ));

        self.register_method(StringMethod::new(
            "get",
            vec!["usize".to_string()],
            "Option<&T>",
            "builtin_slice_get",
        ));

        self.register_method(StringMethod::new(
            "contains",
            vec!["&T".to_string()],
            "bool",
            "builtin_slice_contains",
        ));

        self.register_method(StringMethod::new(
            "starts_with",
            vec!["&[T]".to_string()],
            "bool",
            "builtin_slice_starts_with",
        ));

        self.register_method(StringMethod::new(
            "ends_with",
            vec!["&[T]".to_string()],
            "bool",
            "builtin_slice_ends_with",
        ));
    }

    pub fn register_method(&mut self, method: StringMethod) {
        self.methods.insert(method.name.clone(), method);
    }

    pub fn get_method(&self, name: &str) -> Option<StringMethod> {
        self.methods.get(name).cloned()
    }

    pub fn has_method(&self, name: &str) -> bool {
        self.methods.contains_key(name)
    }
}

impl Default for SliceMethodRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_method_registry_creation() {
        let registry = StringMethodRegistry::new();
        assert!(!registry.methods.is_empty());
    }

    #[test]
    fn test_get_string_method() {
        let registry = StringMethodRegistry::new();
        assert!(registry.get_method("len").is_some());
        assert!(registry.get_method("to_upper").is_some());
    }

    #[test]
    fn test_unknown_method() {
        let registry = StringMethodRegistry::new();
        assert!(registry.get_method("unknown").is_none());
    }

    #[test]
    fn test_has_method() {
        let registry = StringMethodRegistry::new();
        assert!(registry.has_method("len"));
        assert!(!registry.has_method("nonexistent"));
    }

    #[test]
    fn test_validate_call_success() {
        let registry = StringMethodRegistry::new();
        let result = registry.validate_call("len", &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "usize");
    }

    #[test]
    fn test_validate_call_wrong_arity() {
        let registry = StringMethodRegistry::new();
        let result = registry.validate_call("len", &["str".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_method_names() {
        let registry = StringMethodRegistry::new();
        let names = registry.method_names();
        assert!(names.contains(&"len".to_string()));
        assert!(names.contains(&"to_upper".to_string()));
    }

    #[test]
    fn test_get_builtin_name() {
        let registry = StringMethodRegistry::new();
        let builtin = registry.get_builtin_name("len");
        assert_eq!(builtin, Some("builtin_str_len".to_string()));
    }

    #[test]
    fn test_slice_method_registry() {
        let registry = SliceMethodRegistry::new();
        assert!(registry.has_method("len"));
        assert!(registry.has_method("is_empty"));
        assert!(registry.has_method("first"));
    }
}
