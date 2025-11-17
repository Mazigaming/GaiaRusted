//! # Type Aliases Support (Phase 12+)
//!
//! Type alias declaration and resolution:
//! - Simple type aliases
//! - Generic type aliases
//! - Recursive type aliases
//! - Alias expansion and normalization

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AliasError {
    AliasNotFound(String),
    CyclicAlias(String),
    TypeMismatch(String),
    InvalidAlias(String),
    RecursionDepthExceeded,
}

#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    pub generics: Vec<String>,
    pub target: String,
    pub is_generic: bool,
}

pub struct TypeAliasRegistry {
    aliases: HashMap<String, TypeAlias>,
    resolution_cache: HashMap<String, String>,
    recursion_depth: usize,
    max_recursion: usize,
}

impl TypeAliasRegistry {
    pub fn new() -> Self {
        TypeAliasRegistry {
            aliases: HashMap::new(),
            resolution_cache: HashMap::new(),
            recursion_depth: 0,
            max_recursion: 100,
        }
    }

    pub fn register_alias(&mut self, alias: TypeAlias) -> Result<(), AliasError> {
        self.validate_alias(&alias)?;
        self.aliases.insert(alias.name.clone(), alias);
        Ok(())
    }

    pub fn get_alias(&self, name: &str) -> Option<&TypeAlias> {
        self.aliases.get(name)
    }

    pub fn resolve_alias(&mut self, name: &str) -> Result<String, AliasError> {
        if let Some(cached) = self.resolution_cache.get(name) {
            return Ok(cached.clone());
        }

        self.recursion_depth = 0;
        let resolved = self.resolve_alias_recursive(name)?;
        self.resolution_cache.insert(name.to_string(), resolved.clone());
        Ok(resolved)
    }

    fn resolve_alias_recursive(&mut self, name: &str) -> Result<String, AliasError> {
        if self.recursion_depth > self.max_recursion {
            return Err(AliasError::RecursionDepthExceeded);
        }

        let alias = self
            .get_alias(name)
            .ok_or_else(|| AliasError::AliasNotFound(name.to_string()))?
            .clone();

        self.recursion_depth += 1;
        let target = &alias.target;

        if self.get_alias(target).is_some() {
            let resolved = self.resolve_alias_recursive(target)?;
            self.recursion_depth -= 1;
            Ok(resolved)
        } else {
            self.recursion_depth -= 1;
            Ok(target.clone())
        }
    }

    pub fn resolve_with_generics(
        &mut self,
        alias_name: &str,
        type_args: &[String],
    ) -> Result<String, AliasError> {
        let alias = self
            .get_alias(alias_name)
            .ok_or_else(|| AliasError::AliasNotFound(alias_name.to_string()))?
            .clone();

        if alias.generics.len() != type_args.len() {
            return Err(AliasError::TypeMismatch(format!(
                "Expected {} type arguments, got {}",
                alias.generics.len(),
                type_args.len()
            )));
        }

        let mut target = alias.target.clone();
        for (generic, arg) in alias.generics.iter().zip(type_args.iter()) {
            target = target.replace(&format!("{{{}}}", generic), arg);
        }

        self.resolve_alias_recursive_with_resolution(&target)
    }

    fn resolve_alias_recursive_with_resolution(
        &mut self,
        target: &str,
    ) -> Result<String, AliasError> {
        if self.recursion_depth > self.max_recursion {
            return Err(AliasError::RecursionDepthExceeded);
        }

        if self.get_alias(target).is_some() {
            self.recursion_depth += 1;
            let resolved = self.resolve_alias_recursive(target)?;
            self.recursion_depth -= 1;
            Ok(resolved)
        } else {
            Ok(target.to_string())
        }
    }

    pub fn expand_type(&mut self, ty: &str) -> Result<String, AliasError> {
        if ty.contains('<') && ty.contains('>') {
            let alias_name = ty.split('<').next().unwrap_or(ty);
            if self.get_alias(alias_name).is_some() {
                let type_args: Vec<String> = ty
                    .split('<')
                    .nth(1)
                    .unwrap_or("")
                    .trim_end_matches('>')
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();

                return self.resolve_with_generics(alias_name, &type_args);
            }
        }

        self.resolve_alias(ty)
    }

    fn validate_alias(&self, alias: &TypeAlias) -> Result<(), AliasError> {
        if alias.name.is_empty() {
            return Err(AliasError::InvalidAlias("Empty alias name".to_string()));
        }

        if alias.target.is_empty() {
            return Err(AliasError::InvalidAlias(
                format!("Empty target for alias '{}'", alias.name),
            ));
        }

        Ok(())
    }

    pub fn list_aliases(&self) -> Vec<String> {
        self.aliases.keys().cloned().collect()
    }

    pub fn normalize_type(&mut self, ty: &str) -> Result<String, AliasError> {
        self.expand_type(ty)
    }

    pub fn is_alias(&self, name: &str) -> bool {
        self.aliases.contains_key(name)
    }

    pub fn clear_cache(&mut self) {
        self.resolution_cache.clear();
        self.recursion_depth = 0;
    }
}

impl Default for TypeAliasRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> TypeAliasRegistry {
        let mut registry = TypeAliasRegistry::new();

        registry
            .register_alias(TypeAlias {
                name: "StringAlias".to_string(),
                generics: vec![],
                target: "String".to_string(),
                is_generic: false,
            })
            .unwrap();

        registry
            .register_alias(TypeAlias {
                name: "IntVec".to_string(),
                generics: vec![],
                target: "Vec<i32>".to_string(),
                is_generic: false,
            })
            .unwrap();

        registry
    }

    #[test]
    fn test_register_alias() {
        let mut registry = TypeAliasRegistry::new();
        let alias = TypeAlias {
            name: "MyType".to_string(),
            generics: vec![],
            target: "i32".to_string(),
            is_generic: false,
        };
        assert!(registry.register_alias(alias).is_ok());
    }

    #[test]
    fn test_get_alias() {
        let registry = create_test_registry();
        let alias = registry.get_alias("StringAlias");
        assert!(alias.is_some());
        assert_eq!(alias.unwrap().target, "String");
    }

    #[test]
    fn test_alias_not_found() {
        let registry = create_test_registry();
        assert!(registry.get_alias("NonExistent").is_none());
    }

    #[test]
    fn test_resolve_simple_alias() {
        let mut registry = create_test_registry();
        let resolved = registry.resolve_alias("StringAlias").unwrap();
        assert_eq!(resolved, "String");
    }

    #[test]
    fn test_resolve_alias_not_found() {
        let mut registry = TypeAliasRegistry::new();
        let result = registry.resolve_alias("Missing");
        assert!(matches!(result, Err(AliasError::AliasNotFound(_))));
    }

    #[test]
    fn test_resolve_alias_caching() {
        let mut registry = create_test_registry();
        let _ = registry.resolve_alias("StringAlias").unwrap();
        assert_eq!(registry.resolution_cache.len(), 1);
    }

    #[test]
    fn test_resolve_with_generics() {
        let mut registry = TypeAliasRegistry::new();
        registry
            .register_alias(TypeAlias {
                name: "Vector".to_string(),
                generics: vec!["T".to_string()],
                target: "Vec<{T}>".to_string(),
                is_generic: true,
            })
            .unwrap();

        let resolved = registry
            .resolve_with_generics("Vector", &["i32".to_string()])
            .unwrap();
        assert_eq!(resolved, "Vec<i32>");
    }

    #[test]
    fn test_resolve_with_generics_mismatch() {
        let mut registry = TypeAliasRegistry::new();
        registry
            .register_alias(TypeAlias {
                name: "Pair".to_string(),
                generics: vec!["A".to_string(), "B".to_string()],
                target: "({A}, {B})".to_string(),
                is_generic: true,
            })
            .unwrap();

        let result = registry.resolve_with_generics("Pair", &["i32".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_type_simple() {
        let mut registry = create_test_registry();
        let expanded = registry.expand_type("StringAlias").unwrap();
        assert_eq!(expanded, "String");
    }

    #[test]
    fn test_expand_type_generic() {
        let mut registry = TypeAliasRegistry::new();
        registry
            .register_alias(TypeAlias {
                name: "Vec".to_string(),
                generics: vec!["T".to_string()],
                target: "Vector<{T}>".to_string(),
                is_generic: true,
            })
            .unwrap();

        let expanded = registry.expand_type("Vec<i32>").unwrap();
        assert_eq!(expanded, "Vector<i32>");
    }

    #[test]
    fn test_normalize_type() {
        let mut registry = create_test_registry();
        let normalized = registry.normalize_type("IntVec").unwrap();
        assert_eq!(normalized, "Vec<i32>");
    }

    #[test]
    fn test_is_alias() {
        let registry = create_test_registry();
        assert!(registry.is_alias("StringAlias"));
        assert!(!registry.is_alias("NotAnAlias"));
    }

    #[test]
    fn test_list_aliases() {
        let registry = create_test_registry();
        let aliases = registry.list_aliases();
        assert_eq!(aliases.len(), 2);
        assert!(aliases.contains(&"StringAlias".to_string()));
        assert!(aliases.contains(&"IntVec".to_string()));
    }

    #[test]
    fn test_validate_empty_name() {
        let mut registry = TypeAliasRegistry::new();
        let alias = TypeAlias {
            name: "".to_string(),
            generics: vec![],
            target: "i32".to_string(),
            is_generic: false,
        };
        assert!(registry.register_alias(alias).is_err());
    }

    #[test]
    fn test_validate_empty_target() {
        let mut registry = TypeAliasRegistry::new();
        let alias = TypeAlias {
            name: "MyType".to_string(),
            generics: vec![],
            target: "".to_string(),
            is_generic: false,
        };
        assert!(registry.register_alias(alias).is_err());
    }

    #[test]
    fn test_clear_cache() {
        let mut registry = create_test_registry();
        let _ = registry.resolve_alias("StringAlias").unwrap();
        assert_eq!(registry.resolution_cache.len(), 1);
        registry.clear_cache();
        assert_eq!(registry.resolution_cache.len(), 0);
    }

    #[test]
    fn test_multiple_generic_params() {
        let mut registry = TypeAliasRegistry::new();
        registry
            .register_alias(TypeAlias {
                name: "HashMap".to_string(),
                generics: vec!["K".to_string(), "V".to_string()],
                target: "Map<{K}, {V}>".to_string(),
                is_generic: true,
            })
            .unwrap();

        let resolved = registry
            .resolve_with_generics("HashMap", &["String".to_string(), "i32".to_string()])
            .unwrap();
        assert_eq!(resolved, "Map<String, i32>");
    }

    #[test]
    fn test_chained_alias_resolution() {
        let mut registry = TypeAliasRegistry::new();
        registry
            .register_alias(TypeAlias {
                name: "Alias1".to_string(),
                generics: vec![],
                target: "Alias2".to_string(),
                is_generic: false,
            })
            .unwrap();

        registry
            .register_alias(TypeAlias {
                name: "Alias2".to_string(),
                generics: vec![],
                target: "i32".to_string(),
                is_generic: false,
            })
            .unwrap();

        let resolved = registry.resolve_alias("Alias1").unwrap();
        assert_eq!(resolved, "i32");
    }
}
