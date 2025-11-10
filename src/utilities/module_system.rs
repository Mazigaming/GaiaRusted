//! # Advanced Module System (Phase 12+)
//!
//! Full module support with:
//! - Module declarations and nesting
//! - Use statements and imports
//! - Path resolution
//! - Visibility control
//! - Module namespacing

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleError {
    ModuleNotFound(String),
    ItemNotFound(String),
    VisibilityViolation(String),
    CyclicImport(String),
    AmbiguousImport(String),
    InvalidPath(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Public,
    PubCrate,
    PubSuper,
}

#[derive(Debug, Clone)]
pub struct ModuleItem {
    pub name: String,
    pub item_type: String,
    pub visibility: Visibility,
}

#[derive(Debug, Clone)]
pub struct ModuleDef {
    pub name: String,
    pub parent: Option<String>,
    pub items: HashMap<String, ModuleItem>,
    pub submodules: HashMap<String, String>,
    pub imports: Vec<(Vec<String>, Option<String>)>,
    pub visibility: Visibility,
}

pub struct ModuleSystem {
    modules: HashMap<String, ModuleDef>,
    current_module: String,
    import_graph: HashMap<String, HashSet<String>>,
    visibility_cache: HashMap<(String, String), bool>,
}

impl ModuleSystem {
    pub fn new() -> Self {
        let mut system = ModuleSystem {
            modules: HashMap::new(),
            current_module: "crate".to_string(),
            import_graph: HashMap::new(),
            visibility_cache: HashMap::new(),
        };

        system.modules.insert(
            "crate".to_string(),
            ModuleDef {
                name: "crate".to_string(),
                parent: None,
                items: HashMap::new(),
                submodules: HashMap::new(),
                imports: Vec::new(),
                visibility: Visibility::Public,
            },
        );

        system
    }

    pub fn create_module(&mut self, path: Vec<String>) -> Result<(), ModuleError> {
        let module_name = path.join("::");
        if self.modules.contains_key(&module_name) {
            return Err(ModuleError::ModuleNotFound(format!(
                "Module {} already exists",
                module_name
            )));
        }

        let parent = if path.len() > 1 {
            Some(path[..path.len() - 1].join("::"))
        } else {
            None
        };

        self.modules.insert(
            module_name,
            ModuleDef {
                name: path.last().unwrap_or(&"crate".to_string()).clone(),
                parent,
                items: HashMap::new(),
                submodules: HashMap::new(),
                imports: Vec::new(),
                visibility: Visibility::Private,
            },
        );

        Ok(())
    }

    pub fn add_item(
        &mut self,
        module_path: &str,
        item: ModuleItem,
    ) -> Result<(), ModuleError> {
        let module = self
            .modules
            .get_mut(module_path)
            .ok_or_else(|| ModuleError::ModuleNotFound(module_path.to_string()))?;

        module.items.insert(item.name.clone(), item);
        Ok(())
    }

    pub fn add_submodule(
        &mut self,
        parent_path: &str,
        submodule_name: String,
        submodule_full_path: String,
    ) -> Result<(), ModuleError> {
        let module = self
            .modules
            .get_mut(parent_path)
            .ok_or_else(|| ModuleError::ModuleNotFound(parent_path.to_string()))?;

        module
            .submodules
            .insert(submodule_name, submodule_full_path);
        Ok(())
    }

    pub fn add_import(
        &mut self,
        module_path: &str,
        import_path: Vec<String>,
        alias: Option<String>,
    ) -> Result<(), ModuleError> {
        self.check_cyclic_import(&import_path)?;

        let module = self
            .modules
            .get_mut(module_path)
            .ok_or_else(|| ModuleError::ModuleNotFound(module_path.to_string()))?;

        module.imports.push((import_path.clone(), alias));

        self.import_graph
            .entry(module_path.to_string())
            .or_insert_with(HashSet::new)
            .insert(import_path.join("::"));

        Ok(())
    }

    fn check_cyclic_import(&self, path: &[String]) -> Result<(), ModuleError> {
        let import_str = path.join("::");
        let mut visited = HashSet::new();
        self.check_cycle_recursive(&self.current_module, &import_str, &mut visited)
    }

    fn check_cycle_recursive(
        &self,
        current: &str,
        target: &str,
        visited: &mut HashSet<String>,
    ) -> Result<(), ModuleError> {
        if visited.contains(current) {
            return Err(ModuleError::CyclicImport(format!(
                "Cyclic import detected: {} -> {}",
                current, target
            )));
        }

        visited.insert(current.to_string());

        if let Some(imports) = self.import_graph.get(current) {
            for import in imports {
                if import == target {
                    return Err(ModuleError::CyclicImport(format!(
                        "Cyclic import: {} -> {}",
                        current, target
                    )));
                }
                self.check_cycle_recursive(import, target, visited)?;
            }
        }

        Ok(())
    }

    pub fn resolve_path(
        &self,
        from_module: &str,
        path: &[String],
    ) -> Result<String, ModuleError> {
        if path.is_empty() {
            return Err(ModuleError::InvalidPath("Empty path".to_string()));
        }

        if path[0] == "crate" {
            return Ok(path.join("::"));
        }

        if path[0] == "super" {
            let current_parts: Vec<&str> = from_module.split("::").collect();
            if current_parts.len() > 1 {
                let parent = current_parts[..current_parts.len() - 1].join("::");
                let mut resolved_path = parent.split("::").map(|s| s.to_string()).collect::<Vec<_>>();
                resolved_path.extend_from_slice(&path[1..]);
                return Ok(resolved_path.join("::"));
            }
        }

        let mut resolved_path = vec![from_module.to_string()];
        resolved_path.extend_from_slice(path);
        Ok(resolved_path.join("::"))
    }

    pub fn resolve_item(
        &self,
        from_module: &str,
        item_name: &str,
    ) -> Result<(String, &ModuleItem), ModuleError> {
        let module = self
            .modules
            .get(from_module)
            .ok_or_else(|| ModuleError::ModuleNotFound(from_module.to_string()))?;

        if let Some(item) = module.items.get(item_name) {
            if self.is_visible(from_module, item)? {
                return Ok((from_module.to_string(), item));
            } else {
                return Err(ModuleError::VisibilityViolation(format!(
                    "Item {} is not visible",
                    item_name
                )));
            }
        }

        for (import_path, alias) in &module.imports {
            let lookup_name = alias.as_ref().unwrap_or(
                import_path.last().ok_or(ModuleError::InvalidPath(
                    "Empty import path".to_string(),
                ))?,
            );

            if lookup_name == item_name {
                let target_module = import_path.join("::");
                return self.resolve_item(&target_module, item_name);
            }
        }

        Err(ModuleError::ItemNotFound(item_name.to_string()))
    }

    fn is_visible(
        &self,
        _from_module: &str,
        item: &ModuleItem,
    ) -> Result<bool, ModuleError> {
        Ok(matches!(
            item.visibility,
            Visibility::Public | Visibility::PubCrate
        ))
    }

    pub fn get_module(&self, path: &str) -> Option<&ModuleDef> {
        self.modules.get(path)
    }

    pub fn list_items(&self, module_path: &str) -> Result<Vec<String>, ModuleError> {
        let module = self
            .modules
            .get(module_path)
            .ok_or_else(|| ModuleError::ModuleNotFound(module_path.to_string()))?;

        Ok(module.items.keys().cloned().collect())
    }

    pub fn set_current_module(&mut self, path: String) -> Result<(), ModuleError> {
        if !self.modules.contains_key(&path) {
            return Err(ModuleError::ModuleNotFound(path));
        }
        self.current_module = path;
        Ok(())
    }

    pub fn get_current_module(&self) -> &str {
        &self.current_module
    }

    pub fn resolve_use_glob(
        &self,
        module_path: &str,
    ) -> Result<Vec<String>, ModuleError> {
        let module = self
            .modules
            .get(module_path)
            .ok_or_else(|| ModuleError::ModuleNotFound(module_path.to_string()))?;

        Ok(module.items.keys().cloned().collect())
    }

    pub fn build_full_path(&self, module_path: &str, item_name: &str) -> String {
        format!("{}::{}", module_path, item_name)
    }

    pub fn get_all_modules(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }
}

impl Default for ModuleSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_system() {
        let system = ModuleSystem::new();
        assert!(system.modules.contains_key("crate"));
    }

    #[test]
    fn test_create_module() {
        let mut system = ModuleSystem::new();
        assert!(system.create_module(vec!["mymod".to_string()]).is_ok());
        assert!(system.modules.contains_key("mymod"));
    }

    #[test]
    fn test_add_item() {
        let mut system = ModuleSystem::new();
        let item = ModuleItem {
            name: "func".to_string(),
            item_type: "function".to_string(),
            visibility: Visibility::Public,
        };
        assert!(system.add_item("crate", item).is_ok());
    }

    #[test]
    fn test_add_item_to_nonexistent_module() {
        let mut system = ModuleSystem::new();
        let item = ModuleItem {
            name: "func".to_string(),
            item_type: "function".to_string(),
            visibility: Visibility::Public,
        };
        assert!(system.add_item("missing", item).is_err());
    }

    #[test]
    fn test_add_submodule() {
        let mut system = ModuleSystem::new();
        system.create_module(vec!["parent".to_string()]).ok();
        system
            .create_module(vec!["parent".to_string(), "child".to_string()])
            .ok();
        assert!(system
            .add_submodule("parent", "child".to_string(), "parent::child".to_string())
            .is_ok());
    }

    #[test]
    fn test_add_import() {
        let mut system = ModuleSystem::new();
        assert!(system
            .add_import("crate", vec!["std".to_string(), "io".to_string()], None)
            .is_ok());
    }

    #[test]
    fn test_resolve_crate_path() {
        let system = ModuleSystem::new();
        let resolved = system.resolve_path("crate", &["foo".to_string()]).unwrap();
        assert_eq!(resolved, "crate::foo");
    }

    #[test]
    fn test_resolve_relative_path() {
        let mut system = ModuleSystem::new();
        system.create_module(vec!["parent".to_string()]).ok();
        let resolved = system
            .resolve_path("parent", &["child".to_string()])
            .unwrap();
        assert_eq!(resolved, "parent::child");
    }

    #[test]
    fn test_resolve_item_in_crate() {
        let mut system = ModuleSystem::new();
        let item = ModuleItem {
            name: "my_func".to_string(),
            item_type: "function".to_string(),
            visibility: Visibility::Public,
        };
        system.add_item("crate", item).ok();
        let result = system.resolve_item("crate", "my_func");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_missing_item() {
        let system = ModuleSystem::new();
        let result = system.resolve_item("crate", "missing");
        assert!(matches!(result, Err(ModuleError::ItemNotFound(_))));
    }

    #[test]
    fn test_list_items() {
        let mut system = ModuleSystem::new();
        let item1 = ModuleItem {
            name: "func1".to_string(),
            item_type: "function".to_string(),
            visibility: Visibility::Public,
        };
        let item2 = ModuleItem {
            name: "func2".to_string(),
            item_type: "function".to_string(),
            visibility: Visibility::Public,
        };
        system.add_item("crate", item1).ok();
        system.add_item("crate", item2).ok();
        let items = system.list_items("crate").unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_set_current_module() {
        let mut system = ModuleSystem::new();
        system.create_module(vec!["mymod".to_string()]).ok();
        assert!(system.set_current_module("mymod".to_string()).is_ok());
        assert_eq!(system.get_current_module(), "mymod");
    }

    #[test]
    fn test_set_invalid_current_module() {
        let mut system = ModuleSystem::new();
        assert!(system.set_current_module("missing".to_string()).is_err());
    }

    #[test]
    fn test_resolve_use_glob() {
        let mut system = ModuleSystem::new();
        let item = ModuleItem {
            name: "test".to_string(),
            item_type: "function".to_string(),
            visibility: Visibility::Public,
        };
        system.add_item("crate", item).ok();
        let items = system.resolve_use_glob("crate").unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_build_full_path() {
        let system = ModuleSystem::new();
        let path = system.build_full_path("crate", "function");
        assert_eq!(path, "crate::function");
    }

    #[test]
    fn test_get_all_modules() {
        let mut system = ModuleSystem::new();
        system.create_module(vec!["mod1".to_string()]).ok();
        system.create_module(vec!["mod2".to_string()]).ok();
        let modules = system.get_all_modules();
        assert!(modules.contains(&"crate".to_string()));
        assert!(modules.contains(&"mod1".to_string()));
        assert!(modules.contains(&"mod2".to_string()));
    }

    #[test]
    fn test_nested_modules() {
        let mut system = ModuleSystem::new();
        system
            .create_module(vec!["parent".to_string(), "child".to_string()])
            .ok();
        assert!(system.modules.contains_key("parent::child"));
    }

    #[test]
    fn test_visibility_public() {
        let item = ModuleItem {
            name: "public_item".to_string(),
            item_type: "function".to_string(),
            visibility: Visibility::Public,
        };
        let system = ModuleSystem::new();
        assert!(system.is_visible("crate", &item).unwrap());
    }

    #[test]
    fn test_visibility_private() {
        let item = ModuleItem {
            name: "private_item".to_string(),
            item_type: "function".to_string(),
            visibility: Visibility::Private,
        };
        let system = ModuleSystem::new();
        assert!(!system.is_visible("crate", &item).unwrap());
    }

    #[test]
    fn test_import_with_alias() {
        let mut system = ModuleSystem::new();
        system.create_module(vec!["external".to_string()]).ok();
        let item = ModuleItem {
            name: "data".to_string(),
            item_type: "struct".to_string(),
            visibility: Visibility::Public,
        };
        system.add_item("external", item).ok();
        system
            .add_import(
                "crate",
                vec!["external".to_string(), "data".to_string()],
                Some("ext_data".to_string()),
            )
            .ok();
        assert_eq!(system.get_module("crate").unwrap().imports.len(), 1);
    }

    #[test]
    fn test_get_module() {
        let system = ModuleSystem::new();
        assert!(system.get_module("crate").is_some());
        assert!(system.get_module("missing").is_none());
    }

    #[test]
    fn test_empty_path_resolution() {
        let system = ModuleSystem::new();
        let result = system.resolve_path("crate", &[]);
        assert!(matches!(result, Err(ModuleError::InvalidPath(_))));
    }
}
