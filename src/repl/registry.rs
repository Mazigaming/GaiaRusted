//! Function and variable registry for REPL

use std::collections::HashMap;

/// Represents a stored function definition
#[derive(Debug, Clone)]
pub struct FunctionEntry {
    pub definition: String,
}

/// Represents a stored variable binding
#[derive(Debug, Clone)]
pub struct VariableEntry {
    pub definition: String,
}

/// Registry for storing function definitions in the REPL
#[derive(Debug, Clone)]
pub struct Registry {
    functions: HashMap<String, FunctionEntry>,
}

impl Registry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Registry {
            functions: HashMap::new(),
        }
    }

    /// Insert a function into the registry
    pub fn insert(&mut self, name: String, entry: FunctionEntry) {
        self.functions.insert(name, entry);
    }

    /// Check if a function exists
    pub fn contains(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Get a function definition
    pub fn get(&self, name: &str) -> Option<&FunctionEntry> {
        self.functions.get(name)
    }

    /// Remove a function
    pub fn remove(&mut self, name: &str) -> Option<FunctionEntry> {
        self.functions.remove(name)
    }

    /// Get all function names
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.functions.keys()
    }

    /// Get number of functions
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    /// Clear all functions
    pub fn clear(&mut self) {
        self.functions.clear();
    }

    /// Get all functions as a vec of tuples
    pub fn all(&self) -> Vec<(String, String)> {
        self.functions
            .iter()
            .map(|(name, entry)| (name.clone(), entry.definition.clone()))
            .collect()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = Registry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_insert_function() {
        let mut registry = Registry::new();
        registry.insert(
            "add".to_string(),
            FunctionEntry {
                definition: "fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
            },
        );
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("add"));
    }

    #[test]
    fn test_get_function() {
        let mut registry = Registry::new();
        let def = "fn square(x: i32) -> i32 { x * x }";
        registry.insert(
            "square".to_string(),
            FunctionEntry {
                definition: def.to_string(),
            },
        );
        let entry = registry.get("square");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().definition, def);
    }

    #[test]
    fn test_remove_function() {
        let mut registry = Registry::new();
        registry.insert(
            "test".to_string(),
            FunctionEntry {
                definition: "fn test() {}".to_string(),
            },
        );
        assert_eq!(registry.len(), 1);
        registry.remove("test");
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_clear_registry() {
        let mut registry = Registry::new();
        registry.insert(
            "f1".to_string(),
            FunctionEntry {
                definition: "fn f1() {}".to_string(),
            },
        );
        registry.insert(
            "f2".to_string(),
            FunctionEntry {
                definition: "fn f2() {}".to_string(),
            },
        );
        assert_eq!(registry.len(), 2);
        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_keys() {
        let mut registry = Registry::new();
        registry.insert(
            "add".to_string(),
            FunctionEntry {
                definition: "fn add() {}".to_string(),
            },
        );
        registry.insert(
            "mul".to_string(),
            FunctionEntry {
                definition: "fn mul() {}".to_string(),
            },
        );
        let keys: Vec<_> = registry.keys().collect();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_registry_all() {
        let mut registry = Registry::new();
        registry.insert(
            "add".to_string(),
            FunctionEntry {
                definition: "fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
            },
        );
        let all = registry.all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, "add");
    }
}
