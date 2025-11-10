//! # Enhanced Module Visibility System
//!
//! Advanced module access control and re-export tracking:
//! - Visibility modifier enforcement (pub, pub(crate), pub(super))
//! - Re-export tracking and cycle detection
//! - Public API surface definition
//! - Access control validation
//! - Module hierarchy visibility rules

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Crate,
    Super,
    Private,
}

#[derive(Debug, Clone)]
pub struct ModuleItem {
    pub name: String,
    pub visibility: Visibility,
    pub module_path: String,
    pub item_type: ItemType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemType {
    Function,
    Struct,
    Enum,
    Trait,
    Constant,
    Module,
}

#[derive(Debug, Clone)]
pub struct ModuleReexport {
    pub original_module: String,
    pub reexport_as: String,
    pub items: Vec<String>,
}

pub struct ModuleVisibilityEngine {
    modules: HashMap<String, Vec<ModuleItem>>,
    reexports: HashMap<String, Vec<ModuleReexport>>,
    module_hierarchy: HashMap<String, String>,
    access_cache: HashMap<(String, String), bool>,
}

impl ModuleVisibilityEngine {
    pub fn new() -> Self {
        ModuleVisibilityEngine {
            modules: HashMap::new(),
            reexports: HashMap::new(),
            module_hierarchy: HashMap::new(),
            access_cache: HashMap::new(),
        }
    }

    pub fn register_module(&mut self, name: String) {
        self.modules.insert(name, Vec::new());
    }

    pub fn add_item(&mut self, module: &str, item: ModuleItem) -> Result<(), String> {
        let items = self.modules.get_mut(module)
            .ok_or(format!("Module {} not found", module))?;

        items.push(item);
        Ok(())
    }

    pub fn set_module_hierarchy(&mut self, child: String, parent: String) {
        self.module_hierarchy.insert(child, parent);
    }

    pub fn check_access(
        &mut self,
        from_module: &str,
        to_module: &str,
        item_name: &str,
    ) -> Result<bool, String> {
        let cache_key = (
            format!("{}.{}", to_module, item_name),
            from_module.to_string(),
        );

        if let Some(&cached) = self.access_cache.get(&cache_key) {
            return Ok(cached);
        }

        let items = self.modules.get(to_module)
            .ok_or(format!("Module {} not found", to_module))?;

        let item = items.iter()
            .find(|i| i.name == item_name)
            .ok_or(format!("Item {} not found", item_name))?;

        let accessible = self.is_accessible(from_module, to_module, &item.visibility)?;
        self.access_cache.insert(cache_key, accessible);

        Ok(accessible)
    }

    fn is_accessible(&self, from_module: &str, to_module: &str, visibility: &Visibility) -> Result<bool, String> {
        match visibility {
            Visibility::Public => Ok(true),
            Visibility::Private => Ok(from_module == to_module),
            Visibility::Crate => Ok(!from_module.is_empty()),
            Visibility::Super => {
                let from_parent = self.module_hierarchy.get(from_module);
                let to_parent = self.module_hierarchy.get(to_module);

                Ok(from_parent == to_parent)
            }
        }
    }

    pub fn register_reexport(&mut self, module: String, reexport: ModuleReexport) {
        self.reexports
            .entry(module)
            .or_insert_with(Vec::new)
            .push(reexport);
    }

    pub fn get_reexported_items(&self, module: &str) -> Vec<String> {
        self.reexports.get(module)
            .map(|reexports| {
                reexports.iter()
                    .flat_map(|r| r.items.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn detect_reexport_cycles(&self) -> Result<(), String> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for module in self.modules.keys() {
            if !visited.contains(module) {
                self.detect_cycle_dfs(module, &mut visited, &mut rec_stack)?;
            }
        }

        Ok(())
    }

    fn detect_cycle_dfs(
        &self,
        module: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> Result<(), String> {
        visited.insert(module.to_string());
        rec_stack.insert(module.to_string());

        if let Some(reexports) = self.reexports.get(module) {
            for reexport in reexports {
                let re_module = &reexport.original_module;
                if !visited.contains(re_module) {
                    self.detect_cycle_dfs(re_module, visited, rec_stack)?;
                } else if rec_stack.contains(re_module) {
                    return Err(format!(
                        "Re-export cycle detected: {} -> {}",
                        module, re_module
                    ));
                }
            }
        }

        rec_stack.remove(module);
        Ok(())
    }

    pub fn get_public_api(&self, module: &str) -> Result<Vec<String>, String> {
        let items = self.modules.get(module)
            .ok_or(format!("Module {} not found", module))?;

        Ok(items.iter()
            .filter(|item| item.visibility == Visibility::Public)
            .map(|item| item.name.clone())
            .collect())
    }

    pub fn validate_visibility_rules(&self, module: &str) -> Result<(), String> {
        let items = self.modules.get(module)
            .ok_or(format!("Module {} not found", module))?;

        for item in items {
            if item.visibility == Visibility::Super {
                if !self.module_hierarchy.contains_key(module) {
                    return Err(format!(
                        "Item {} uses pub(super) but {} has no parent module",
                        item.name, module
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn get_module_items(&self, module: &str) -> Option<Vec<ModuleItem>> {
        self.modules.get(module).cloned()
    }

    pub fn is_item_accessible_from_outside(&self, module: &str, item_name: &str) -> Result<bool, String> {
        let items = self.modules.get(module)
            .ok_or(format!("Module {} not found", module))?;

        let item = items.iter()
            .find(|i| i.name == item_name)
            .ok_or(format!("Item {} not found", item_name))?;

        Ok(item.visibility == Visibility::Public || item.visibility == Visibility::Crate)
    }

    pub fn collect_public_api(&self) -> HashMap<String, Vec<String>> {
        let mut api = HashMap::new();

        for (module, items) in &self.modules {
            let public_items: Vec<_> = items.iter()
                .filter(|item| item.visibility == Visibility::Public)
                .map(|item| item.name.clone())
                .collect();

            if !public_items.is_empty() {
                api.insert(module.clone(), public_items);
            }
        }

        api
    }

    pub fn clear_cache(&mut self) {
        self.access_cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_engine() {
        let _engine = ModuleVisibilityEngine::new();
        assert!(true);
    }

    #[test]
    fn test_register_module() {
        let mut engine = ModuleVisibilityEngine::new();
        engine.register_module("math".to_string());

        assert!(engine.modules.contains_key("math"));
    }

    #[test]
    fn test_add_item() {
        let mut engine = ModuleVisibilityEngine::new();
        engine.register_module("math".to_string());

        let item = ModuleItem {
            name: "add".to_string(),
            visibility: Visibility::Public,
            module_path: "math".to_string(),
            item_type: ItemType::Function,
        };

        assert!(engine.add_item("math", item).is_ok());
    }

    #[test]
    fn test_set_module_hierarchy() {
        let mut engine = ModuleVisibilityEngine::new();
        engine.set_module_hierarchy("child".to_string(), "parent".to_string());

        assert!(engine.module_hierarchy.contains_key("child"));
    }

    #[test]
    fn test_check_access_public() {
        let mut engine = ModuleVisibilityEngine::new();
        engine.register_module("math".to_string());

        let item = ModuleItem {
            name: "add".to_string(),
            visibility: Visibility::Public,
            module_path: "math".to_string(),
            item_type: ItemType::Function,
        };

        engine.add_item("math", item).unwrap();

        let access = engine.check_access("other", "math", "add").unwrap();
        assert!(access);
    }

    #[test]
    fn test_check_access_private() {
        let mut engine = ModuleVisibilityEngine::new();
        engine.register_module("math".to_string());

        let item = ModuleItem {
            name: "helper".to_string(),
            visibility: Visibility::Private,
            module_path: "math".to_string(),
            item_type: ItemType::Function,
        };

        engine.add_item("math", item).unwrap();

        let access = engine.check_access("other", "math", "helper");
        assert!(access.is_ok());
        assert!(!access.unwrap());
    }

    #[test]
    fn test_register_reexport() {
        let mut engine = ModuleVisibilityEngine::new();
        let reexport = ModuleReexport {
            original_module: "math".to_string(),
            reexport_as: "core::math".to_string(),
            items: vec!["add".to_string()],
        };

        engine.register_reexport("core".to_string(), reexport);
        assert!(!engine.reexports.is_empty());
    }

    #[test]
    fn test_get_reexported_items() {
        let mut engine = ModuleVisibilityEngine::new();
        let reexport = ModuleReexport {
            original_module: "math".to_string(),
            reexport_as: "core::math".to_string(),
            items: vec!["add".to_string(), "sub".to_string()],
        };

        engine.register_reexport("core".to_string(), reexport);
        let items = engine.get_reexported_items("core");
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_detect_reexport_cycles() {
        let mut engine = ModuleVisibilityEngine::new();
        engine.register_module("a".to_string());
        engine.register_module("b".to_string());

        assert!(engine.detect_reexport_cycles().is_ok());
    }

    #[test]
    fn test_get_public_api() {
        let mut engine = ModuleVisibilityEngine::new();
        engine.register_module("math".to_string());

        let item = ModuleItem {
            name: "add".to_string(),
            visibility: Visibility::Public,
            module_path: "math".to_string(),
            item_type: ItemType::Function,
        };

        engine.add_item("math", item).unwrap();

        let api = engine.get_public_api("math").unwrap();
        assert_eq!(api.len(), 1);
    }

    #[test]
    fn test_validate_visibility_rules() {
        let mut engine = ModuleVisibilityEngine::new();
        engine.register_module("math".to_string());

        assert!(engine.validate_visibility_rules("math").is_ok());
    }

    #[test]
    fn test_get_module_items() {
        let mut engine = ModuleVisibilityEngine::new();
        engine.register_module("math".to_string());

        let item = ModuleItem {
            name: "add".to_string(),
            visibility: Visibility::Public,
            module_path: "math".to_string(),
            item_type: ItemType::Function,
        };

        engine.add_item("math", item).unwrap();

        let items = engine.get_module_items("math");
        assert!(items.is_some());
        assert_eq!(items.unwrap().len(), 1);
    }

    #[test]
    fn test_is_item_accessible_from_outside() {
        let mut engine = ModuleVisibilityEngine::new();
        engine.register_module("math".to_string());

        let item = ModuleItem {
            name: "add".to_string(),
            visibility: Visibility::Public,
            module_path: "math".to_string(),
            item_type: ItemType::Function,
        };

        engine.add_item("math", item).unwrap();

        let accessible = engine.is_item_accessible_from_outside("math", "add").unwrap();
        assert!(accessible);
    }

    #[test]
    fn test_collect_public_api() {
        let mut engine = ModuleVisibilityEngine::new();
        engine.register_module("math".to_string());

        let item = ModuleItem {
            name: "add".to_string(),
            visibility: Visibility::Public,
            module_path: "math".to_string(),
            item_type: ItemType::Function,
        };

        engine.add_item("math", item).unwrap();

        let api = engine.collect_public_api();
        assert!(!api.is_empty());
    }

    #[test]
    fn test_clear_cache() {
        let mut engine = ModuleVisibilityEngine::new();
        engine.access_cache.insert(
            ("test".to_string(), "module".to_string()),
            true,
        );

        assert!(!engine.access_cache.is_empty());
        engine.clear_cache();
        assert!(engine.access_cache.is_empty());
    }
}
