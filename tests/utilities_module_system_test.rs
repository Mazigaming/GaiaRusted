//! Integration tests for module system (v0.0.3)

#[cfg(test)]
mod module_system_tests {
    use gaiarusted::modules::{Module, ModuleItem, Visibility, ModuleCache};
    use std::path::PathBuf;

    #[test]
    fn test_module_creation() {
        let module = Module::new("math".to_string(), PathBuf::from("math.rs"));
        assert_eq!(module.name, "math");
        assert_eq!(module.items.len(), 0);
    }

    #[test]
    fn test_add_item_to_module() {
        let mut module = Module::new("utils".to_string(), PathBuf::from("utils.rs"));

        module.add_item(
            "helper".to_string(),
            ModuleItem::Function {
                name: "helper".to_string(),
                signature: "fn() -> i32".to_string(),
                visibility: Visibility::Public,
            },
        );

        assert_eq!(module.items.len(), 1);
        assert!(module.items.contains_key("helper"));
    }

    #[test]
    fn test_module_cache_registration() {
        let mut cache = ModuleCache::new();
        let module = Module::new("test".to_string(), PathBuf::from("test.rs"));

        cache.register(module);
        assert!(cache.exists("test"));
    }

    #[test]
    fn test_module_cache_retrieval() {
        let mut cache = ModuleCache::new();
        let module = Module::new("math".to_string(), PathBuf::from("math.rs"));

        cache.register(module);
        let retrieved = cache.get("math");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "math");
    }

    #[test]
    fn test_module_visibility_public() {
        let mut module = Module::new("public_mod".to_string(), PathBuf::from("pub.rs"));

        module.add_item(
            "public_fn".to_string(),
            ModuleItem::Function {
                name: "public_fn".to_string(),
                signature: "fn() -> ()".to_string(),
                visibility: Visibility::Public,
            },
        );

        let item = module.get_item("public_fn", true);
        assert!(item.is_some());
    }

    #[test]
    fn test_module_visibility_private() {
        let mut module = Module::new("private_mod".to_string(), PathBuf::from("priv.rs"));

        module.add_item(
            "private_fn".to_string(),
            ModuleItem::Function {
                name: "private_fn".to_string(),
                signature: "fn() -> ()".to_string(),
                visibility: Visibility::Private,
            },
        );

        let item = module.get_item("private_fn", true);
        assert!(item.is_none());
    }

    #[test]
    fn test_module_export_listing() {
        let mut module = Module::new("exports".to_string(), PathBuf::from("exports.rs"));

        module.add_item(
            "public_fn".to_string(),
            ModuleItem::Function {
                name: "public_fn".to_string(),
                signature: "fn() -> ()".to_string(),
                visibility: Visibility::Public,
            },
        );

        module.add_item(
            "private_fn".to_string(),
            ModuleItem::Function {
                name: "private_fn".to_string(),
                signature: "fn() -> ()".to_string(),
                visibility: Visibility::Private,
            },
        );

        let exports = module.list_exports();
        assert_eq!(exports.len(), 1);
        assert!(exports.contains(&"public_fn".to_string()));
    }

    #[test]
    fn test_module_dependency_tracking() {
        let mut module = Module::new("dependent".to_string(), PathBuf::from("dep.rs"));

        module.add_dependency("std".to_string());
        module.add_dependency("math".to_string());
        module.add_dependency("math".to_string()); // Duplicate

        assert_eq!(module.dependencies.len(), 2);
        assert!(module.dependencies.contains(&"std".to_string()));
    }

    #[test]
    fn test_module_cache_list_modules() {
        let mut cache = ModuleCache::new();

        cache.register(Module::new("mod1".to_string(), PathBuf::from("mod1.rs")));
        cache.register(Module::new("mod2".to_string(), PathBuf::from("mod2.rs")));
        cache.register(Module::new("mod3".to_string(), PathBuf::from("mod3.rs")));

        let modules = cache.list_modules();
        assert_eq!(modules.len(), 3);
    }
}