//! Standard Library Bindings
//!
//! Bindings to Rust standard library types and functions

use std::collections::HashMap;

/// Standard library module definition
#[derive(Debug, Clone)]
pub struct StdlibModule {
    pub name: String,
    pub items: Vec<StdlibItem>,
}

/// Stdlib item (type, trait, function)
#[derive(Debug, Clone)]
pub struct StdlibItem {
    pub name: String,
    pub kind: StdlibItemKind,
    pub signature: String,
}

/// Kind of stdlib item
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdlibItemKind {
    Type,
    Trait,
    Function,
    Macro,
    Constant,
    Module,
}

/// Stdlib binding definition
#[derive(Debug, Clone)]
pub struct StdlibBinding {
    pub modules: HashMap<String, StdlibModule>,
    pub prelude_items: Vec<String>,
}

impl StdlibBinding {
    /// Create new stdlib binding
    pub fn new() -> Self {
        let mut binding = StdlibBinding {
            modules: HashMap::new(),
            prelude_items: Vec::new(),
        };

        // Initialize standard modules
        binding.register_core_modules();
        binding.setup_prelude();

        binding
    }

    /// Register core standard library modules
    fn register_core_modules(&mut self) {
        // Collections module
        let collections_module = StdlibModule {
            name: "collections".to_string(),
            items: vec![
                StdlibItem {
                    name: "Vec".to_string(),
                    kind: StdlibItemKind::Type,
                    signature: "pub struct Vec<T>".to_string(),
                },
                StdlibItem {
                    name: "HashMap".to_string(),
                    kind: StdlibItemKind::Type,
                    signature: "pub struct HashMap<K, V>".to_string(),
                },
                StdlibItem {
                    name: "HashSet".to_string(),
                    kind: StdlibItemKind::Type,
                    signature: "pub struct HashSet<T>".to_string(),
                },
            ],
        };

        // Option and Result module
        let option_result_module = StdlibModule {
            name: "option".to_string(),
            items: vec![
                StdlibItem {
                    name: "Option".to_string(),
                    kind: StdlibItemKind::Type,
                    signature: "pub enum Option<T> { Some(T), None }".to_string(),
                },
            ],
        };

        let result_module = StdlibModule {
            name: "result".to_string(),
            items: vec![
                StdlibItem {
                    name: "Result".to_string(),
                    kind: StdlibItemKind::Type,
                    signature: "pub enum Result<T, E> { Ok(T), Err(E) }".to_string(),
                },
            ],
        };

        // String module
        let string_module = StdlibModule {
            name: "string".to_string(),
            items: vec![
                StdlibItem {
                    name: "String".to_string(),
                    kind: StdlibItemKind::Type,
                    signature: "pub struct String".to_string(),
                },
            ],
        };

        // Iterator traits
        let iter_module = StdlibModule {
            name: "iter".to_string(),
            items: vec![
                StdlibItem {
                    name: "Iterator".to_string(),
                    kind: StdlibItemKind::Trait,
                    signature: "pub trait Iterator { type Item; fn next(&mut self) -> Option<Self::Item>; }".to_string(),
                },
                StdlibItem {
                    name: "IntoIterator".to_string(),
                    kind: StdlibItemKind::Trait,
                    signature: "pub trait IntoIterator".to_string(),
                },
            ],
        };

        // Trait module
        let traits_module = StdlibModule {
            name: "traits".to_string(),
            items: vec![
                StdlibItem {
                    name: "Clone".to_string(),
                    kind: StdlibItemKind::Trait,
                    signature: "pub trait Clone { fn clone(&self) -> Self; }".to_string(),
                },
                StdlibItem {
                    name: "Copy".to_string(),
                    kind: StdlibItemKind::Trait,
                    signature: "pub trait Copy: Clone".to_string(),
                },
                StdlibItem {
                    name: "Debug".to_string(),
                    kind: StdlibItemKind::Trait,
                    signature: "pub trait Debug".to_string(),
                },
                StdlibItem {
                    name: "Default".to_string(),
                    kind: StdlibItemKind::Trait,
                    signature: "pub trait Default { fn default() -> Self; }".to_string(),
                },
            ],
        };

        self.modules
            .insert("collections".to_string(), collections_module);
        self.modules
            .insert("option".to_string(), option_result_module);
        self.modules
            .insert("result".to_string(), result_module);
        self.modules
            .insert("string".to_string(), string_module);
        self.modules.insert("iter".to_string(), iter_module);
        self.modules.insert("traits".to_string(), traits_module);
    }

    /// Setup prelude items
    fn setup_prelude(&mut self) {
        self.prelude_items = vec![
            "Vec".to_string(),
            "String".to_string(),
            "Option".to_string(),
            "Result".to_string(),
            "Iterator".to_string(),
            "Clone".to_string(),
            "Debug".to_string(),
            "Default".to_string(),
        ];
    }

    /// Get module
    pub fn get_module(&self, name: &str) -> Option<&StdlibModule> {
        self.modules.get(name)
    }

    /// Check if item is in prelude
    pub fn is_in_prelude(&self, item: &str) -> bool {
        self.prelude_items.contains(&item.to_string())
    }

    /// Get all available items in a module
    pub fn get_module_items(&self, module: &str) -> Option<Vec<&StdlibItem>> {
        self.modules.get(module).map(|m| m.items.iter().collect())
    }

    /// Resolve item from module path
    pub fn resolve_item(&self, path: &str) -> Option<&StdlibItem> {
        let parts: Vec<&str> = path.split("::").collect();
        if parts.len() != 2 {
            return None;
        }

        let module_name = parts[0];
        let item_name = parts[1];

        self.modules.get(module_name).and_then(|module| {
            module
                .items
                .iter()
                .find(|item| item.name == item_name)
        })
    }
}

impl Default for StdlibBinding {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdlib_binding_creation() {
        let binding = StdlibBinding::new();
        assert!(!binding.modules.is_empty());
        assert!(!binding.prelude_items.is_empty());
    }

    #[test]
    fn test_get_module() {
        let binding = StdlibBinding::new();
        let collections = binding.get_module("collections");
        assert!(collections.is_some());
        assert_eq!(collections.unwrap().name, "collections");
    }

    #[test]
    fn test_is_in_prelude() {
        let binding = StdlibBinding::new();
        assert!(binding.is_in_prelude("Vec"));
        assert!(binding.is_in_prelude("String"));
        assert!(!binding.is_in_prelude("NonExistent"));
    }

    #[test]
    fn test_get_module_items() {
        let binding = StdlibBinding::new();
        let items = binding.get_module_items("collections");
        assert!(items.is_some());
        let items = items.unwrap();
        assert!(!items.is_empty());
    }

    #[test]
    fn test_resolve_item() {
        let binding = StdlibBinding::new();
        let item = binding.resolve_item("collections::Vec");
        assert!(item.is_some());
        assert_eq!(item.unwrap().name, "Vec");
    }

    #[test]
    fn test_resolve_nonexistent_item() {
        let binding = StdlibBinding::new();
        let item = binding.resolve_item("nonexistent::Item");
        assert!(item.is_none());
    }

    #[test]
    fn test_stdlib_item_kinds() {
        let item = StdlibItem {
            name: "Vec".to_string(),
            kind: StdlibItemKind::Type,
            signature: "pub struct Vec<T>".to_string(),
        };
        assert_eq!(item.kind, StdlibItemKind::Type);
    }

    #[test]
    fn test_multiple_modules() {
        let binding = StdlibBinding::new();
        assert!(binding.get_module("collections").is_some());
        assert!(binding.get_module("option").is_some());
        assert!(binding.get_module("result").is_some());
        assert!(binding.get_module("traits").is_some());
    }
}
