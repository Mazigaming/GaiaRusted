//! # String Type Implementation
//!
//! Implements the `String` type - a growable, heap-allocated string that owns its data.
//! Unlike `&str` (string slice), String can be modified and extended.

use std::collections::HashMap;

/// Represents the String type in the type system
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringType;

/// Represents a String value at runtime
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringValue {
    /// The actual string data
    data: String,
}

impl StringValue {
    /// Create a new empty String
    pub fn new() -> Self {
        StringValue {
            data: String::new(),
        }
    }

    /// Create a String from a string slice
    pub fn from(s: &str) -> Self {
        StringValue {
            data: s.to_string(),
        }
    }

    /// Create a String with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        StringValue {
            data: String::with_capacity(capacity),
        }
    }

    /// Get the length of the string in bytes
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the string is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the capacity of the string
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Push a character onto the end of the string
    pub fn push(&mut self, ch: char) {
        self.data.push(ch);
    }

    /// Push a string slice onto the end of the string
    pub fn push_str(&mut self, s: &str) {
        self.data.push_str(s);
    }

    /// Remove and return the last character if it exists
    pub fn pop(&mut self) -> Option<char> {
        self.data.pop()
    }

    /// Clear the string, removing all characters
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Check if the string contains the given substring
    pub fn contains(&self, substring: &str) -> bool {
        self.data.contains(substring)
    }

    /// Check if the string starts with the given prefix
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.data.starts_with(prefix)
    }

    /// Check if the string ends with the given suffix
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.data.ends_with(suffix)
    }

    /// Convert the string to uppercase
    pub fn to_uppercase(&self) -> StringValue {
        StringValue {
            data: self.data.to_uppercase(),
        }
    }

    /// Convert the string to lowercase
    pub fn to_lowercase(&self) -> StringValue {
        StringValue {
            data: self.data.to_lowercase(),
        }
    }

    /// Remove leading and trailing whitespace
    pub fn trim(&self) -> StringValue {
        StringValue {
            data: self.data.trim().to_string(),
        }
    }

    /// Get the string as a string slice
    pub fn as_str(&self) -> &str {
        &self.data
    }

    /// Concatenate with another string (implements + operator)
    pub fn concat(&self, other: &str) -> StringValue {
        let mut result = self.clone();
        result.push_str(other);
        result
    }

    /// Get the number of characters (not bytes)
    pub fn char_count(&self) -> usize {
        self.data.chars().count()
    }

    /// Check if string contains only whitespace
    pub fn is_whitespace(&self) -> bool {
        self.data.chars().all(char::is_whitespace)
    }

    /// Repeat the string n times
    pub fn repeat(&self, n: usize) -> StringValue {
        StringValue {
            data: self.data.repeat(n),
        }
    }

    /// Split the string by delimiter and return parts
    pub fn split(&self, delimiter: char) -> Vec<String> {
        self.data
            .split(delimiter)
            .map(|s| s.to_string())
            .collect()
    }

    /// Replace all occurrences of a pattern with replacement
    pub fn replace(&self, from: &str, to: &str) -> StringValue {
        StringValue {
            data: self.data.replace(from, to),
        }
    }

    /// Get a substring starting at index with given length
    pub fn substring(&self, start: usize, length: usize) -> Option<StringValue> {
        if start + length > self.data.len() {
            return None;
        }
        Some(StringValue {
            data: self.data[start..start + length].to_string(),
        })
    }

    /// Find the index of a substring (first occurrence)
    pub fn find(&self, substring: &str) -> Option<usize> {
        self.data.find(substring)
    }

    /// Reverse the string
    pub fn reverse(&self) -> StringValue {
        StringValue {
            data: self.data.chars().rev().collect(),
        }
    }
}

impl Default for StringValue {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for StringValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
    }
}

/// String method registry for the type system
pub struct StringMethodRegistry;

impl StringMethodRegistry {
    /// Get all available methods for String type
    pub fn get_methods() -> HashMap<String, String> {
        let mut methods = HashMap::new();

        // Constructors
        methods.insert("new".to_string(), "() -> String".to_string());
        methods.insert("from".to_string(), "(&str) -> String".to_string());
        methods.insert("with_capacity".to_string(), "(usize) -> String".to_string());

        // Query methods
        methods.insert("len".to_string(), "(&self) -> usize".to_string());
        methods.insert("is_empty".to_string(), "(&self) -> bool".to_string());
        methods.insert("capacity".to_string(), "(&self) -> usize".to_string());
        methods.insert("char_count".to_string(), "(&self) -> usize".to_string());

        // Modification methods
        methods.insert("push".to_string(), "(&mut self, char)".to_string());
        methods.insert("push_str".to_string(), "(&mut self, &str)".to_string());
        methods.insert("pop".to_string(), "(&mut self) -> Option<char>".to_string());
        methods.insert("clear".to_string(), "(&mut self)".to_string());
        methods.insert("repeat".to_string(), "(&self, usize) -> String".to_string());

        // Search methods
        methods.insert("contains".to_string(), "(&self, &str) -> bool".to_string());
        methods.insert("starts_with".to_string(), "(&self, &str) -> bool".to_string());
        methods.insert("ends_with".to_string(), "(&self, &str) -> bool".to_string());
        methods.insert("find".to_string(), "(&self, &str) -> Option<usize>".to_string());

        // Transformation methods
        methods.insert("to_uppercase".to_string(), "(&self) -> String".to_string());
        methods.insert("to_lowercase".to_string(), "(&self) -> String".to_string());
        methods.insert("trim".to_string(), "(&self) -> String".to_string());
        methods.insert("reverse".to_string(), "(&self) -> String".to_string());
        methods.insert("replace".to_string(), "(&self, &str, &str) -> String".to_string());

        // Splitting methods
        methods.insert("split".to_string(), "(&self, char) -> Vec<String>".to_string());

        // Utility methods
        methods.insert("as_str".to_string(), "(&self) -> &str".to_string());
        methods.insert("is_whitespace".to_string(), "(&self) -> bool".to_string());
        methods.insert("substring".to_string(), "(&self, usize, usize) -> Option<String>".to_string());

        methods
    }

    /// Check if a method exists
    pub fn has_method(name: &str) -> bool {
        Self::get_methods().contains_key(name)
    }

    /// Get method signature
    pub fn get_signature(name: &str) -> Option<String> {
        Self::get_methods().get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_new() {
        let s = StringValue::new();
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
    }

    #[test]
    fn test_string_from() {
        let s = StringValue::from("hello");
        assert_eq!(s.len(), 5);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_string_push() {
        let mut s = StringValue::new();
        s.push('h');
        s.push('i');
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_string_push_str() {
        let mut s = StringValue::from("Hello");
        s.push_str(" World");
        assert_eq!(s.as_str(), "Hello World");
    }

    #[test]
    fn test_string_pop() {
        let mut s = StringValue::from("hello");
        assert_eq!(s.pop(), Some('o'));
        assert_eq!(s.len(), 4);
    }

    #[test]
    fn test_string_clear() {
        let mut s = StringValue::from("hello");
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_string_contains() {
        let s = StringValue::from("hello world");
        assert!(s.contains("world"));
        assert!(!s.contains("xyz"));
    }

    #[test]
    fn test_string_starts_with() {
        let s = StringValue::from("hello");
        assert!(s.starts_with("he"));
        assert!(!s.starts_with("wo"));
    }

    #[test]
    fn test_string_ends_with() {
        let s = StringValue::from("hello");
        assert!(s.ends_with("lo"));
        assert!(!s.ends_with("he"));
    }

    #[test]
    fn test_string_to_uppercase() {
        let s = StringValue::from("hello");
        let upper = s.to_uppercase();
        assert_eq!(upper.as_str(), "HELLO");
    }

    #[test]
    fn test_string_to_lowercase() {
        let s = StringValue::from("HELLO");
        let lower = s.to_lowercase();
        assert_eq!(lower.as_str(), "hello");
    }

    #[test]
    fn test_string_trim() {
        let s = StringValue::from("  hello  ");
        let trimmed = s.trim();
        assert_eq!(trimmed.as_str(), "hello");
    }

    #[test]
    fn test_string_concat() {
        let s1 = StringValue::from("Hello");
        let s2 = s1.concat(" World");
        assert_eq!(s2.as_str(), "Hello World");
    }

    #[test]
    fn test_string_char_count() {
        let s = StringValue::from("hello");
        assert_eq!(s.char_count(), 5);
    }

    #[test]
    fn test_string_is_whitespace() {
        let s1 = StringValue::from("   ");
        let s2 = StringValue::from("hello");
        assert!(s1.is_whitespace());
        assert!(!s2.is_whitespace());
    }

    #[test]
    fn test_string_repeat() {
        let s = StringValue::from("ab");
        let repeated = s.repeat(3);
        assert_eq!(repeated.as_str(), "ababab");
    }

    #[test]
    fn test_string_split() {
        let s = StringValue::from("a,b,c");
        let parts = s.split(',');
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_string_replace() {
        let s = StringValue::from("hello world");
        let replaced = s.replace("world", "rust");
        assert_eq!(replaced.as_str(), "hello rust");
    }

    #[test]
    fn test_string_find() {
        let s = StringValue::from("hello world");
        assert_eq!(s.find("world"), Some(6));
        assert_eq!(s.find("xyz"), None);
    }

    #[test]
    fn test_string_reverse() {
        let s = StringValue::from("hello");
        let reversed = s.reverse();
        assert_eq!(reversed.as_str(), "olleh");
    }

    #[test]
    fn test_string_with_capacity() {
        let s = StringValue::with_capacity(100);
        assert!(s.capacity() >= 100);
        assert!(s.is_empty());
    }

    #[test]
    fn test_string_method_registry() {
        assert!(StringMethodRegistry::has_method("new"));
        assert!(StringMethodRegistry::has_method("from"));
        assert!(StringMethodRegistry::has_method("push"));
        assert!(!StringMethodRegistry::has_method("unknown"));
    }

    #[test]
    fn test_string_method_signature() {
        let sig = StringMethodRegistry::get_signature("len");
        assert!(sig.is_some());
        assert!(sig.unwrap().contains("usize"));
    }
}
