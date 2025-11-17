//! # Module System for v0.0.3
//!
//! Provides modular code organization, namespace management, and library integration.
//!
//! ## Features
//! - Module definitions and nested modules
//! - Visibility control (public/private)
//! - Use statements and imports
//! - Library linking and FFI support
//! - Dependency management

use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a compiled module
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub items: HashMap<String, ModuleItem>,
    pub visibility: Visibility,
    pub dependencies: Vec<String>,
}

/// Items that can be exported from a module
#[derive(Debug, Clone)]
pub enum ModuleItem {
    Function {
        name: String,
        signature: String,
        visibility: Visibility,
    },
    Struct {
        name: String,
        visibility: Visibility,
    },
    Enum {
        name: String,
        visibility: Visibility,
    },
    Trait {
        name: String,
        visibility: Visibility,
    },
    Constant {
        name: String,
        value: String,
        visibility: Visibility,
    },
    Module(Box<Module>),
}

/// Visibility levels for items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Private - only accessible within the module
    Private,
    /// Public - accessible from anywhere
    Public,
    /// Pub(crate) - accessible within the crate
    PubCrate,
    /// Pub(super) - accessible from parent module
    PubSuper,
}

impl Module {
    /// Create a new module
    pub fn new(name: String, path: PathBuf) -> Self {
        Module {
            name,
            path,
            items: HashMap::new(),
            visibility: Visibility::Private,
            dependencies: Vec::new(),
        }
    }

    /// Add an item to the module
    pub fn add_item(&mut self, name: String, item: ModuleItem) {
        self.items.insert(name, item);
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dep: String) {
        if !self.dependencies.contains(&dep) {
            self.dependencies.push(dep);
        }
    }

    /// Get an item by name (respects visibility)
    pub fn get_item(&self, name: &str, is_external: bool) -> Option<ModuleItem> {
        self.items.get(name).and_then(|item| {
            if is_external && !self.is_accessible(item) {
                None
            } else {
                Some(item.clone())
            }
        })
    }

    /// Check if an item is accessible
    fn is_accessible(&self, item: &ModuleItem) -> bool {
        match self.get_visibility(item) {
            Visibility::Public => true,
            Visibility::PubCrate => true, // Simplified for now
            _ => false,
        }
    }

    /// Get visibility of an item
    fn get_visibility(&self, item: &ModuleItem) -> Visibility {
        match item {
            ModuleItem::Function { visibility, .. } => *visibility,
            ModuleItem::Struct { visibility, .. } => *visibility,
            ModuleItem::Enum { visibility, .. } => *visibility,
            ModuleItem::Trait { visibility, .. } => *visibility,
            ModuleItem::Constant { visibility, .. } => *visibility,
            ModuleItem::Module(m) => m.visibility,
        }
    }

    /// List all public exports
    pub fn list_exports(&self) -> Vec<String> {
        self.items
            .iter()
            .filter(|(_, item)| self.is_accessible(item))
            .map(|(name, _)| name.clone())
            .collect()
    }
}

/// Module cache for faster lookups
pub struct ModuleCache {
    modules: HashMap<String, Module>,
}

impl ModuleCache {
    pub fn new() -> Self {
        ModuleCache {
            modules: HashMap::new(),
        }
    }

    /// Register a module in the cache
    pub fn register(&mut self, module: Module) {
        self.modules.insert(module.name.clone(), module);
    }

    /// Retrieve a module
    pub fn get(&self, name: &str) -> Option<&Module> {
        self.modules.get(name)
    }

    /// Check if a module exists
    pub fn exists(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }

    /// List all registered modules
    pub fn list_modules(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }
}

impl Default for ModuleCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creation() {
        let module = Module::new("test".to_string(), PathBuf::from("test.rs"));
        assert_eq!(module.name, "test");
    }

    #[test]
    fn test_module_cache() {
        let mut cache = ModuleCache::new();
        let module = Module::new("test".to_string(), PathBuf::from("test.rs"));
        cache.register(module);

        assert!(cache.exists("test"));
        assert!(cache.get("test").is_some());
    }

    #[test]
    fn test_visibility_filtering() {
        let mut module = Module::new("test".to_string(), PathBuf::from("test.rs"));
        module.add_item(
            "public_fn".to_string(),
            ModuleItem::Function {
                name: "public_fn".to_string(),
                signature: "fn() -> i32".to_string(),
                visibility: Visibility::Public,
            },
        );
        module.add_item(
            "private_fn".to_string(),
            ModuleItem::Function {
                name: "private_fn".to_string(),
                signature: "fn() -> i32".to_string(),
                visibility: Visibility::Private,
            },
        );

        let exports = module.list_exports();
        assert_eq!(exports.len(), 1);
        assert!(exports.contains(&"public_fn".to_string()));
    }
}