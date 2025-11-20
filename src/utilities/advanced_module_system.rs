//! # Advanced Module System with Constants and Re-exports
//!
//! Enhancements to module system:
//! - Module-level constants
//! - Re-export capability
//! - Namespace aliasing
//! - Advanced import resolution
//! - Circular import detection

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

/// Module-level constant definition
#[derive(Debug, Clone)]
pub struct ModuleConstant {
    /// Constant name
    pub name: String,
    /// Constant type
    pub ty: String,
    /// Constant value (as string representation)
    pub value: String,
    /// Visibility of the constant
    pub visibility: ModuleVisibility,
}

impl ModuleConstant {
    /// Create a new module constant
    pub fn new(name: String, ty: String, value: String) -> Self {
        ModuleConstant {
            name,
            ty,
            value,
            visibility: ModuleVisibility::Private,
        }
    }

    /// Make constant public
    pub fn make_public(&mut self) {
        self.visibility = ModuleVisibility::Public;
    }
}

/// Module visibility levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleVisibility {
    /// Private to module
    Private,
    /// Public to all
    Public,
    /// Public within crate
    Crate,
    /// Public to parent module
    Super,
}

impl fmt::Display for ModuleVisibility {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ModuleVisibility::Private => write!(f, "private"),
            ModuleVisibility::Public => write!(f, "pub"),
            ModuleVisibility::Crate => write!(f, "pub(crate)"),
            ModuleVisibility::Super => write!(f, "pub(super)"),
        }
    }
}

/// Module re-export information
#[derive(Debug, Clone)]
pub struct ModuleReexport {
    /// Source module path
    pub source: Vec<String>,
    /// Items being re-exported
    pub items: Vec<String>,
    /// Alias for re-export
    pub alias: Option<String>,
}

impl ModuleReexport {
    /// Create a new re-export
    pub fn new(source: Vec<String>, items: Vec<String>) -> Self {
        ModuleReexport {
            source,
            items,
            alias: None,
        }
    }

    /// Set an alias for the re-export
    pub fn with_alias(mut self, alias: String) -> Self {
        self.alias = Some(alias);
        self
    }
}

/// Namespace aliasing for imports
#[derive(Debug, Clone)]
pub struct NamespaceAlias {
    /// Original name
    pub original: String,
    /// Aliased name
    pub alias: String,
    /// Full module path
    pub module_path: String,
}

impl NamespaceAlias {
    /// Create a new namespace alias
    pub fn new(original: String, alias: String, module_path: String) -> Self {
        NamespaceAlias {
            original,
            alias,
            module_path,
        }
    }
}

/// Import resolution result
#[derive(Debug, Clone)]
pub struct ImportResolution {
    /// Resolved module path
    pub module_path: String,
    /// Resolved item name
    pub item_name: String,
    /// Visibility of imported item
    pub visibility: ModuleVisibility,
}

/// Advanced module system
pub struct AdvancedModuleSystem {
    /// Module hierarchy: module -> parent
    module_hierarchy: HashMap<String, String>,
    /// Module contents
    module_contents: HashMap<String, ModuleContents>,
    /// Module-level constants
    module_constants: HashMap<String, Vec<ModuleConstant>>,
    /// Re-exports
    reexports: HashMap<String, Vec<ModuleReexport>>,
    /// Namespace aliases
    aliases: HashMap<String, NamespaceAlias>,
    /// Import graph for cycle detection
    import_graph: HashMap<String, HashSet<String>>,
    /// Visited modules (for cycle detection)
    visited: HashSet<String>,
}

/// Contents of a module
#[derive(Debug, Clone)]
pub struct ModuleContents {
    /// Functions
    pub functions: Vec<String>,
    /// Structs
    pub structs: Vec<String>,
    /// Enums
    pub enums: Vec<String>,
    /// Traits
    pub traits: Vec<String>,
    /// Re-exports
    pub reexports: Vec<String>,
}

impl ModuleContents {
    /// Create new module contents
    pub fn new() -> Self {
        ModuleContents {
            functions: Vec::new(),
            structs: Vec::new(),
            enums: Vec::new(),
            traits: Vec::new(),
            reexports: Vec::new(),
        }
    }

    /// Get all items in module
    pub fn all_items(&self) -> Vec<String> {
        let mut items = Vec::new();
        items.extend(self.functions.clone());
        items.extend(self.structs.clone());
        items.extend(self.enums.clone());
        items.extend(self.traits.clone());
        items.extend(self.reexports.clone());
        items
    }
}

impl Default for ModuleContents {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedModuleSystem {
    /// Create a new advanced module system
    pub fn new() -> Self {
        let mut system = AdvancedModuleSystem {
            module_hierarchy: HashMap::new(),
            module_contents: HashMap::new(),
            module_constants: HashMap::new(),
            reexports: HashMap::new(),
            aliases: HashMap::new(),
            import_graph: HashMap::new(),
            visited: HashSet::new(),
        };

        system.module_hierarchy.insert("crate".to_string(), String::new());
        system.module_contents.insert("crate".to_string(), ModuleContents::new());
        system.module_constants.insert("crate".to_string(), Vec::new());

        system
    }

    /// Create a new module
    pub fn create_module(&mut self, path: Vec<String>) -> Result<(), String> {
        let module_path = path.join("::");
        if self.module_contents.contains_key(&module_path) {
            return Err(format!("Module {} already exists", module_path));
        }

        let parent = if path.len() > 1 {
            Some(path[..path.len() - 1].join("::"))
        } else {
            Some("crate".to_string())
        };

        if let Some(parent_path) = parent {
            self.module_hierarchy.insert(module_path.clone(), parent_path);
        }

        self.module_contents.insert(module_path.clone(), ModuleContents::new());
        self.module_constants.insert(module_path.clone(), Vec::new());

        Ok(())
    }

    /// Add a module-level constant
    pub fn add_constant(
        &mut self,
        module: &str,
        constant: ModuleConstant,
    ) -> Result<(), String> {
        self.module_constants
            .get_mut(module)
            .ok_or_else(|| format!("Module {} not found", module))?
            .push(constant);
        Ok(())
    }

    /// Get module constants
    pub fn get_constants(&self, module: &str) -> Result<Vec<ModuleConstant>, String> {
        self.module_constants
            .get(module)
            .ok_or_else(|| format!("Module {} not found", module))
            .map(|v| v.clone())
    }

    /// Add a re-export
    pub fn add_reexport(&mut self, module: &str, reexport: ModuleReexport) -> Result<(), String> {
        self.reexports
            .entry(module.to_string())
            .or_insert_with(Vec::new)
            .push(reexport.clone());

        self.module_contents
            .get_mut(module)
            .ok_or_else(|| format!("Module {} not found", module))?
            .reexports
            .extend(reexport.items);

        Ok(())
    }

    /// Create a namespace alias
    pub fn create_alias(
        &mut self,
        alias: NamespaceAlias,
    ) -> Result<(), String> {
        self.aliases.insert(alias.alias.clone(), alias);
        Ok(())
    }

    /// Resolve an import path
    pub fn resolve_import(
        &mut self,
        from_module: &str,
        path: &[String],
    ) -> Result<ImportResolution, String> {
        if path.is_empty() {
            return Err("Empty import path".to_string());
        }

        self.check_circular_import(from_module, &path.join("::"))?;

        let resolved_path = path.join("::");

        let visibility = if self.is_accessible(from_module, &resolved_path)? {
            ModuleVisibility::Public
        } else {
            ModuleVisibility::Private
        };

        Ok(ImportResolution {
            module_path: resolved_path.clone(),
            item_name: path.last().unwrap_or(&"unknown".to_string()).clone(),
            visibility,
        })
    }

    /// Check if item is accessible from another module
    pub fn is_accessible(&self, from_module: &str, to_item: &str) -> Result<bool, String> {
        if from_module == to_item {
            return Ok(true);
        }

        let from_parts: Vec<&str> = from_module.split("::").collect();
        let to_parts: Vec<&str> = to_item.split("::").collect();

        if to_parts.is_empty() {
            return Ok(false);
        }

        let is_same_crate = from_parts.get(0).map(|p| *p) == to_parts.get(0).map(|p| *p);

        Ok(is_same_crate)
    }

    /// Check for circular imports (BFS)
    pub fn check_circular_import(&mut self, from: &str, to: &str) -> Result<(), String> {
        self.visited.clear();
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(from.to_string());

        while let Some(current) = queue.pop_front() {
            if self.visited.contains(&current) {
                continue;
            }
            self.visited.insert(current.clone());

            if let Some(deps) = self.import_graph.get(&current) {
                for dep in deps {
                    if dep == to {
                        return Err(format!(
                            "Circular import detected: {} -> {} -> {}",
                            from, current, to
                        ));
                    }
                    queue.push_back(dep.clone());
                }
            }
        }

        Ok(())
    }

    /// Get parent module
    pub fn get_parent(&self, module: &str) -> Option<String> {
        self.module_hierarchy.get(module).cloned()
    }

    /// Get all submodules
    pub fn get_submodules(&self, module: &str) -> Vec<String> {
        self.module_hierarchy
            .iter()
            .filter(|(_, parent)| parent.as_str() == module)
            .map(|(child, _)| child.clone())
            .collect()
    }

    /// List all constants in module
    pub fn list_constants(&self, module: &str) -> Result<Vec<String>, String> {
        Ok(self
            .module_constants
            .get(module)
            .ok_or_else(|| format!("Module {} not found", module))?
            .iter()
            .map(|c| c.name.clone())
            .collect())
    }
}

impl Default for AdvancedModuleSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_constant_creation() {
        let constant = ModuleConstant::new("VERSION".to_string(), "i32".to_string(), "1".to_string());
        assert_eq!(constant.name, "VERSION");
        assert_eq!(constant.value, "1");
    }

    #[test]
    fn test_module_constant_visibility() {
        let mut constant = ModuleConstant::new("DEBUG".to_string(), "bool".to_string(), "true".to_string());
        constant.make_public();
        assert_eq!(constant.visibility, ModuleVisibility::Public);
    }

    #[test]
    fn test_reexport_creation() {
        let reexport = ModuleReexport::new(
            vec!["foo".to_string(), "bar".to_string()],
            vec!["item1".to_string(), "item2".to_string()],
        );
        assert_eq!(reexport.items.len(), 2);
    }

    #[test]
    fn test_namespace_alias() {
        let alias = NamespaceAlias::new(
            "OldName".to_string(),
            "NewName".to_string(),
            "my::module".to_string(),
        );
        assert_eq!(alias.alias, "NewName");
    }

    #[test]
    fn test_module_system_creation() {
        let sys = AdvancedModuleSystem::new();
        assert_eq!(sys.module_hierarchy.len(), 1);
    }

    #[test]
    fn test_create_module() {
        let mut sys = AdvancedModuleSystem::new();
        assert!(sys.create_module(vec!["foo".to_string()]).is_ok());
        assert!(sys.module_contents.contains_key("foo"));
    }

    #[test]
    fn test_nested_modules() {
        let mut sys = AdvancedModuleSystem::new();
        assert!(sys.create_module(vec!["foo".to_string()]).is_ok());
        assert!(sys.create_module(vec!["foo".to_string(), "bar".to_string()]).is_ok());
        assert!(sys.module_contents.contains_key("foo::bar"));
    }

    #[test]
    fn test_add_constant() {
        let mut sys = AdvancedModuleSystem::new();
        let constant = ModuleConstant::new("MAX".to_string(), "i32".to_string(), "100".to_string());
        assert!(sys.add_constant("crate", constant).is_ok());
    }

    #[test]
    fn test_get_constants() {
        let mut sys = AdvancedModuleSystem::new();
        let constant = ModuleConstant::new("PI".to_string(), "f64".to_string(), "3.14".to_string());
        sys.add_constant("crate", constant).unwrap();
        let constants = sys.get_constants("crate").unwrap();
        assert_eq!(constants.len(), 1);
    }

    #[test]
    fn test_add_reexport() {
        let mut sys = AdvancedModuleSystem::new();
        let reexport = ModuleReexport::new(
            vec!["std".to_string(), "collections".to_string()],
            vec!["HashMap".to_string()],
        );
        assert!(sys.add_reexport("crate", reexport).is_ok());
    }

    #[test]
    fn test_module_visibility_enum() {
        assert_eq!(ModuleVisibility::Public.to_string(), "pub");
        assert_eq!(ModuleVisibility::Private.to_string(), "private");
        assert_eq!(ModuleVisibility::Crate.to_string(), "pub(crate)");
        assert_eq!(ModuleVisibility::Super.to_string(), "pub(super)");
    }

    #[test]
    fn test_resolve_import() {
        let mut sys = AdvancedModuleSystem::new();
        sys.create_module(vec!["utils".to_string()]).unwrap();
        let result = sys.resolve_import("crate", &["utils".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_parent_module() {
        let mut sys = AdvancedModuleSystem::new();
        sys.create_module(vec!["foo".to_string()]).unwrap();
        let parent = sys.get_parent("foo");
        assert_eq!(parent, Some("crate".to_string()));
    }

    #[test]
    fn test_get_submodules() {
        let mut sys = AdvancedModuleSystem::new();
        sys.create_module(vec!["parent".to_string()]).unwrap();
        sys.create_module(vec!["parent".to_string(), "child1".to_string()]).unwrap();
        sys.create_module(vec!["parent".to_string(), "child2".to_string()]).unwrap();
        let submodules = sys.get_submodules("parent");
        assert_eq!(submodules.len(), 2);
    }
}
