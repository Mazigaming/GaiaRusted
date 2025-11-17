//! # Advanced Trait System (Phase 12+)
//!
//! Extended trait support with:
//! - Associated types
//! - Default implementations
//! - Trait objects with vtables
//! - Super traits
//! - Type bounds

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraitError {
    AssociatedTypeNotFound(String),
    MissingImplementation(String),
    TypeBoundViolation(String),
    ConflictingImplementations(String),
    InvalidTraitObject(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociatedType {
    pub name: String,
    pub bounds: Vec<String>,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub signature: String,
    pub default_impl: Option<String>,
    pub generics: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    pub supertraits: Vec<String>,
    pub associated_types: HashMap<String, AssociatedType>,
    pub methods: Vec<TraitMethod>,
    pub generics: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TraitImpl {
    pub trait_name: String,
    pub impl_type: String,
    pub associated_types: HashMap<String, String>,
    pub methods: HashMap<String, String>,
    pub generics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VTableEntry {
    pub method_name: String,
    pub ptr: String,
    pub signature: String,
}

#[derive(Debug, Clone)]
pub struct TraitObject {
    pub trait_name: String,
    pub data_ptr: String,
    pub vtable: Vec<VTableEntry>,
}

pub struct TraitSystem {
    traits: HashMap<String, TraitDef>,
    impls: HashMap<String, Vec<TraitImpl>>,
    vtable_cache: HashMap<String, Vec<VTableEntry>>,
}

impl TraitSystem {
    pub fn new() -> Self {
        TraitSystem {
            traits: HashMap::new(),
            impls: HashMap::new(),
            vtable_cache: HashMap::new(),
        }
    }

    pub fn register_trait(&mut self, trait_def: TraitDef) {
        self.traits.insert(trait_def.name.clone(), trait_def);
    }

    pub fn register_impl(&mut self, impl_def: TraitImpl) {
        self.impls
            .entry(impl_def.trait_name.clone())
            .or_insert_with(Vec::new)
            .push(impl_def);
    }

    pub fn get_trait(&self, name: &str) -> Option<&TraitDef> {
        self.traits.get(name)
    }

    pub fn get_impl(&self, trait_name: &str, ty: &str) -> Option<&TraitImpl> {
        self.impls
            .get(trait_name)
            .and_then(|impls| impls.iter().find(|i| i.impl_type == ty))
    }

    pub fn resolve_associated_type(
        &self,
        trait_name: &str,
        type_name: &str,
        impl_type: &str,
    ) -> Result<String, TraitError> {
        let impl_def = self.get_impl(trait_name, impl_type).ok_or(
            TraitError::MissingImplementation(format!(
                "No impl of {} for {}",
                trait_name, impl_type
            )),
        )?;

        impl_def
            .associated_types
            .get(type_name)
            .cloned()
            .ok_or(TraitError::AssociatedTypeNotFound(format!(
                "Associated type {} not found",
                type_name
            )))
    }

    pub fn check_trait_bounds(
        &self,
        ty: &str,
        bounds: &[String],
    ) -> Result<(), TraitError> {
        for bound in bounds {
            if !self.traits.contains_key(bound) {
                return Err(TraitError::TypeBoundViolation(format!(
                    "Type {} does not satisfy bound {}",
                    ty, bound
                )));
            }
        }
        Ok(())
    }

    pub fn generate_vtable(
        &mut self,
        trait_name: &str,
        impl_type: &str,
    ) -> Result<Vec<VTableEntry>, TraitError> {
        let cache_key = format!("{}_{}", trait_name, impl_type);
        if let Some(cached) = self.vtable_cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let trait_def = self.get_trait(trait_name).ok_or(
            TraitError::InvalidTraitObject(format!("Trait {} not found", trait_name)),
        )?;

        let impl_def = self.get_impl(trait_name, impl_type).ok_or(
            TraitError::MissingImplementation(format!(
                "No impl of {} for {}",
                trait_name, impl_type
            )),
        )?;

        let mut vtable = Vec::new();
        for method in &trait_def.methods {
            let impl_ptr = impl_def
                .methods
                .get(&method.name)
                .ok_or(TraitError::MissingImplementation(format!(
                    "Method {} not implemented",
                    method.name
                )))?;

            vtable.push(VTableEntry {
                method_name: method.name.clone(),
                ptr: impl_ptr.clone(),
                signature: method.signature.clone(),
            });
        }

        self.vtable_cache.insert(cache_key, vtable.clone());
        Ok(vtable)
    }

    pub fn create_trait_object(
        &mut self,
        trait_name: &str,
        impl_type: &str,
        data_ptr: String,
    ) -> Result<TraitObject, TraitError> {
        let vtable = self.generate_vtable(trait_name, impl_type)?;
        Ok(TraitObject {
            trait_name: trait_name.to_string(),
            data_ptr,
            vtable,
        })
    }

    pub fn check_method_resolution(
        &self,
        trait_name: &str,
        method_name: &str,
    ) -> Result<&TraitMethod, TraitError> {
        let trait_def = self.get_trait(trait_name).ok_or(
            TraitError::InvalidTraitObject(format!("Trait {} not found", trait_name)),
        )?;

        trait_def
            .methods
            .iter()
            .find(|m| m.name == method_name)
            .ok_or(TraitError::MissingImplementation(format!(
                "Method {} not found in trait {}",
                method_name, trait_name
            )))
    }

    pub fn validate_trait_impl(
        &self,
        trait_name: &str,
        impl_type: &str,
    ) -> Result<(), TraitError> {
        let trait_def = self.get_trait(trait_name).ok_or(
            TraitError::InvalidTraitObject(format!("Trait {} not found", trait_name)),
        )?;

        let impl_def = self.get_impl(trait_name, impl_type).ok_or(
            TraitError::MissingImplementation(format!(
                "No impl of {} for {}",
                trait_name, impl_type
            )),
        )?;

        for method in &trait_def.methods {
            if !method.default_impl.is_some() && !impl_def.methods.contains_key(&method.name) {
                return Err(TraitError::MissingImplementation(format!(
                    "Method {} must be implemented",
                    method.name
                )));
            }
        }

        for assoc_ty in trait_def.associated_types.values() {
            if assoc_ty.default.is_none() && !impl_def.associated_types.contains_key(&assoc_ty.name)
            {
                return Err(TraitError::MissingImplementation(format!(
                    "Associated type {} must be defined",
                    assoc_ty.name
                )));
            }
        }

        Ok(())
    }

    pub fn resolve_super_traits(&self, trait_name: &str) -> Result<Vec<String>, TraitError> {
        let _trait_def = self.get_trait(trait_name).ok_or(
            TraitError::InvalidTraitObject(format!("Trait {} not found", trait_name)),
        )?;

        let mut all_supertraits = Vec::new();
        let mut seen = std::collections::HashSet::new();

        fn collect_supertraits(
            trait_name: &str,
            system: &TraitSystem,
            all: &mut Vec<String>,
            seen: &mut std::collections::HashSet<String>,
        ) -> Result<(), TraitError> {
            if seen.contains(trait_name) {
                return Ok(());
            }
            seen.insert(trait_name.to_string());

            let trait_def = system.get_trait(trait_name).ok_or(
                TraitError::InvalidTraitObject(format!("Trait {} not found", trait_name)),
            )?;

            for supertrait in &trait_def.supertraits {
                all.push(supertrait.clone());
                collect_supertraits(supertrait, system, all, seen)?;
            }

            Ok(())
        }

        collect_supertraits(trait_name, self, &mut all_supertraits, &mut seen)?;
        Ok(all_supertraits)
    }
}

impl Default for TraitSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_system() -> TraitSystem {
        let mut system = TraitSystem::new();

        let display_trait = TraitDef {
            name: "Display".to_string(),
            supertraits: vec![],
            associated_types: HashMap::new(),
            methods: vec![TraitMethod {
                name: "fmt".to_string(),
                signature: "fn fmt(&self) -> String".to_string(),
                default_impl: None,
                generics: vec![],
            }],
            generics: vec![],
        };

        system.register_trait(display_trait);

        let impl_display = TraitImpl {
            trait_name: "Display".to_string(),
            impl_type: "i32".to_string(),
            associated_types: HashMap::new(),
            methods: {
                let mut m = HashMap::new();
                m.insert("fmt".to_string(), "i32::fmt".to_string());
                m
            },
            generics: vec![],
        };

        system.register_impl(impl_display);
        system
    }

    #[test]
    fn test_register_trait() {
        let mut system = TraitSystem::new();
        let trait_def = TraitDef {
            name: "Clone".to_string(),
            supertraits: vec![],
            associated_types: HashMap::new(),
            methods: vec![],
            generics: vec![],
        };
        system.register_trait(trait_def);
        assert!(system.get_trait("Clone").is_some());
    }

    #[test]
    fn test_register_impl() {
        let system = create_test_system();
        assert!(system.get_impl("Display", "i32").is_some());
    }

    #[test]
    fn test_missing_impl() {
        let system = create_test_system();
        assert!(system.get_impl("Display", "f64").is_none());
    }

    #[test]
    fn test_check_trait_bounds() {
        let system = create_test_system();
        let bounds = vec!["Display".to_string()];
        assert!(system.check_trait_bounds("i32", &bounds).is_ok());
    }

    #[test]
    fn test_check_invalid_trait_bounds() {
        let system = create_test_system();
        let bounds = vec!["InvalidTrait".to_string()];
        assert!(system.check_trait_bounds("i32", &bounds).is_err());
    }

    #[test]
    fn test_vtable_generation() {
        let mut system = create_test_system();
        let vtable = system.generate_vtable("Display", "i32").unwrap();
        assert_eq!(vtable.len(), 1);
        assert_eq!(vtable[0].method_name, "fmt");
    }

    #[test]
    fn test_vtable_caching() {
        let mut system = create_test_system();
        let vtable1 = system.generate_vtable("Display", "i32").unwrap();
        let vtable2 = system.generate_vtable("Display", "i32").unwrap();
        assert_eq!(vtable1, vtable2);
        assert_eq!(system.vtable_cache.len(), 1);
    }

    #[test]
    fn test_trait_object_creation() {
        let mut system = create_test_system();
        let obj = system
            .create_trait_object("Display", "i32", "0x12345678".to_string())
            .unwrap();
        assert_eq!(obj.trait_name, "Display");
        assert_eq!(obj.data_ptr, "0x12345678");
    }

    #[test]
    fn test_validate_trait_impl_success() {
        let system = create_test_system();
        assert!(system.validate_trait_impl("Display", "i32").is_ok());
    }

    #[test]
    fn test_validate_trait_impl_missing() {
        let system = create_test_system();
        assert!(system.validate_trait_impl("Display", "f64").is_err());
    }

    #[test]
    fn test_method_resolution() {
        let system = create_test_system();
        let method = system.check_method_resolution("Display", "fmt");
        assert!(method.is_ok());
        assert_eq!(method.unwrap().name, "fmt");
    }

    #[test]
    fn test_method_not_found() {
        let system = create_test_system();
        assert!(system.check_method_resolution("Display", "missing").is_err());
    }

    #[test]
    fn test_associated_type_resolution() {
        let mut system = TraitSystem::new();

        let iter_trait = TraitDef {
            name: "Iterator".to_string(),
            supertraits: vec![],
            associated_types: {
                let mut m = HashMap::new();
                m.insert(
                    "Item".to_string(),
                    AssociatedType {
                        name: "Item".to_string(),
                        bounds: vec![],
                        default: None,
                    },
                );
                m
            },
            methods: vec![],
            generics: vec![],
        };

        system.register_trait(iter_trait);

        let impl_iter = TraitImpl {
            trait_name: "Iterator".to_string(),
            impl_type: "VecIter".to_string(),
            associated_types: {
                let mut m = HashMap::new();
                m.insert("Item".to_string(), "i32".to_string());
                m
            },
            methods: HashMap::new(),
            generics: vec![],
        };

        system.register_impl(impl_iter);

        let resolved = system.resolve_associated_type("Iterator", "Item", "VecIter");
        assert_eq!(resolved.unwrap(), "i32");
    }

    #[test]
    fn test_super_traits() {
        let mut system = TraitSystem::new();

        let eq_trait = TraitDef {
            name: "Eq".to_string(),
            supertraits: vec!["PartialEq".to_string()],
            associated_types: HashMap::new(),
            methods: vec![],
            generics: vec![],
        };

        let partial_eq = TraitDef {
            name: "PartialEq".to_string(),
            supertraits: vec![],
            associated_types: HashMap::new(),
            methods: vec![],
            generics: vec![],
        };

        system.register_trait(eq_trait);
        system.register_trait(partial_eq);

        let supertraits = system.resolve_super_traits("Eq").unwrap();
        assert!(supertraits.contains(&"PartialEq".to_string()));
    }

    #[test]
    fn test_default_implementation() {
        let mut system = TraitSystem::new();

        let clone_trait = TraitDef {
            name: "Clone".to_string(),
            supertraits: vec![],
            associated_types: HashMap::new(),
            methods: vec![TraitMethod {
                name: "clone".to_string(),
                signature: "fn clone(&self) -> Self".to_string(),
                default_impl: Some("default_clone_impl".to_string()),
                generics: vec![],
            }],
            generics: vec![],
        };

        system.register_trait(clone_trait);

        let impl_clone = TraitImpl {
            trait_name: "Clone".to_string(),
            impl_type: "i32".to_string(),
            associated_types: HashMap::new(),
            methods: HashMap::new(),
            generics: vec![],
        };

        system.register_impl(impl_clone);

        assert!(system.validate_trait_impl("Clone", "i32").is_ok());
    }

    #[test]
    fn test_trait_not_found() {
        let system = TraitSystem::new();
        assert!(system.get_trait("Missing").is_none());
    }
}
