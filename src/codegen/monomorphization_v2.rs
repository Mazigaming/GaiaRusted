//! Monomorphization Integration into Compiler Pipeline
//!
//! This module integrates the generics instantiation system into the main
//! compilation flow, handling function calls with generic parameters.
//!
//! Integration Points:
//! 1. Type Checker Phase: Detect generic function calls
//! 2. MIR Generation: Register generics and instantiate concrete versions
//! 3. Codegen: Generate code for monomorphized functions

use std::collections::HashMap;
use crate::parser::ast::{GenericParam, Type, Item};

/// A monomorphized function instantiation
#[derive(Debug, Clone)]
pub struct MonomorphicInstance {
    pub original_name: String,
    pub concrete_name: String,
    pub type_args: Vec<Type>,
}

/// Registry for tracking generic function instantiations
pub struct MonomorphizationRegistry {
    /// Maps (generic_name, type_arg_signature) to concrete instantiation
    instances: HashMap<String, MonomorphicInstance>,
    /// Set of function names that have generics
    generic_functions: HashMap<String, Vec<GenericParam>>,
}

impl MonomorphizationRegistry {
    pub fn new() -> Self {
        MonomorphizationRegistry {
            instances: HashMap::new(),
            generic_functions: HashMap::new(),
        }
    }

    /// Register a generic function and its parameters
    pub fn register_generic(&mut self, name: String, generics: Vec<GenericParam>) {
        if !generics.is_empty() {
            self.generic_functions.insert(name, generics);
        }
    }

    /// Create a monomorphic instance of a generic function
    pub fn instantiate(&mut self, func_name: &str, type_args: Vec<Type>) -> Result<String, String> {
        if !self.generic_functions.contains_key(func_name) {
            return Err(format!("Generic function '{}' not registered", func_name));
        }

        let generics = &self.generic_functions[func_name];
        if type_args.len() != generics.len() {
            return Err(format!(
                "Generic function '{}' expects {} type arguments, got {}",
                func_name,
                generics.len(),
                type_args.len()
            ));
        }

        let concrete_name = self.generate_concrete_name(func_name, &type_args);
        let signature = format!("{}::{}", func_name, concrete_name);

        if !self.instances.contains_key(&signature) {
            self.instances.insert(
                signature,
                MonomorphicInstance {
                    original_name: func_name.to_string(),
                    concrete_name: concrete_name.clone(),
                    type_args,
                },
            );
        }

        Ok(concrete_name)
    }

    /// Generate a unique name for a concrete instantiation
    fn generate_concrete_name(&self, func_name: &str, type_args: &[Type]) -> String {
        let mut name = func_name.to_string();
        name.push('_');

        let type_parts: Vec<String> = type_args.iter().map(|t| self.type_to_mangled_name(t)).collect();
        name.push_str(&type_parts.join("_"));
        name
    }

    /// Convert a Type to a mangled name component
    fn type_to_mangled_name(&self, ty: &Type) -> String {
        match ty {
            Type::Named(name) => {
                name.replace(" ", "_")
            }
            Type::Generic { name, type_args } => {
                let mut result = name.clone();
                result.push('_');
                let parts: Vec<String> = type_args.iter().map(|t| self.type_to_mangled_name(t)).collect();
                result.push_str(&parts.join("_"));
                result
            }
            Type::Reference { inner, mutable, .. } => {
                let mut_str = if *mutable { "mut_" } else { "" };
                format!("ref_{}{}", mut_str, self.type_to_mangled_name(inner))
            }
            Type::Pointer { inner, mutable } => {
                let mut_str = if *mutable { "mut_" } else { "" };
                format!("ptr_{}{}", mut_str, self.type_to_mangled_name(inner))
            }
            Type::Array { element, size } => {
                let size_str = size.as_ref().map(|_| "_unknown").unwrap_or_default();
                format!("array_{}{}", self.type_to_mangled_name(element), size_str)
            }
            Type::Tuple(types) => {
                let parts: Vec<String> = types.iter().map(|t| self.type_to_mangled_name(t)).collect();
                format!("tuple_{}", parts.join("_"))
            }
            Type::Function { params, return_type, .. } => {
                let param_parts: Vec<String> = params.iter().map(|t| self.type_to_mangled_name(t)).collect();
                let ret = self.type_to_mangled_name(return_type);
                format!("fn_{}_{}", param_parts.join("_"), ret)
            }
            Type::TypeVar(name) => format!("var_{}", name),
            Type::Never => "never".to_string(),
            Type::TraitObject { .. } => "dyn".to_string(),
            Type::ImplTrait { .. } => "impl_trait".to_string(),
            Type::AssociatedType { name, .. } => format!("assoc_{}", name),
            Type::QualifiedPath { name, .. } => format!("qpath_{}", name),
            Type::Closure { .. } => "closure".to_string(),
        }
    }

    /// Get all instantiations for a specific generic function
    pub fn get_instantiations(&self, func_name: &str) -> Vec<MonomorphicInstance> {
        self.instances
            .values()
            .filter(|inst| inst.original_name == func_name)
            .cloned()
            .collect()
    }

    /// Get all registered generic functions
    pub fn get_generic_functions(&self) -> Vec<String> {
        self.generic_functions.keys().cloned().collect()
    }
}

/// Extract generics from AST items for registration
pub fn collect_generics(items: &[Item]) -> HashMap<String, Vec<GenericParam>> {
    let mut generics_map = HashMap::new();

    for item in items {
        match item {
            Item::Function {
                name,
                generics,
                ..
            } => {
                if !generics.is_empty() {
                    generics_map.insert(name.clone(), generics.clone());
                }
            }
            Item::Struct {
                name,
                generics,
                ..
            } => {
                if !generics.is_empty() {
                    generics_map.insert(name.clone(), generics.clone());
                }
            }
            Item::Enum {
                name,
                generics,
                ..
            } => {
                if !generics.is_empty() {
                    generics_map.insert(name.clone(), generics.clone());
                }
            }
            Item::Trait {
                name,
                generics,
                ..
            } => {
                if !generics.is_empty() {
                    generics_map.insert(name.clone(), generics.clone());
                }
            }
            Item::Impl { generics, struct_name, .. } => {
                if !generics.is_empty() {
                    generics_map.insert(struct_name.clone(), generics.clone());
                }
            }
            _ => {}
        }
    }

    generics_map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = MonomorphizationRegistry::new();
        assert_eq!(registry.generic_functions.len(), 0);
        assert_eq!(registry.instances.len(), 0);
    }

    #[test]
    fn test_register_generic_function() {
        let mut registry = MonomorphizationRegistry::new();
        let generics = vec![GenericParam::Type {
            name: "T".to_string(),
            bounds: vec![],
            default: None,
        }];

        registry.register_generic("add".to_string(), generics);
        assert_eq!(registry.generic_functions.len(), 1);
        assert!(registry.generic_functions.contains_key("add"));
    }

    #[test]
    fn test_type_mangling_named_types() {
        let registry = MonomorphizationRegistry::new();

        let i32_type = Type::Named("i32".to_string());
        assert_eq!(registry.type_to_mangled_name(&i32_type), "i32");

        let bool_type = Type::Named("bool".to_string());
        assert_eq!(registry.type_to_mangled_name(&bool_type), "bool");

        let string_type = Type::Named("String".to_string());
        assert_eq!(registry.type_to_mangled_name(&string_type), "String");
    }

    #[test]
    fn test_type_mangling_generic_types() {
        let registry = MonomorphizationRegistry::new();

        let vec_i32 = Type::Generic {
            name: "Vec".to_string(),
            type_args: vec![Type::Named("i32".to_string())],
        };

        let mangled = registry.type_to_mangled_name(&vec_i32);
        assert!(mangled.contains("Vec"));
        assert!(mangled.contains("i32"));
    }

    #[test]
    fn test_type_mangling_reference_types() {
        let registry = MonomorphizationRegistry::new();

        let ref_i32 = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::Named("i32".to_string())),
        };

        let mangled = registry.type_to_mangled_name(&ref_i32);
        assert!(mangled.contains("ref"));
        assert!(mangled.contains("i32"));
    }

    #[test]
    fn test_type_mangling_mutable_reference() {
        let registry = MonomorphizationRegistry::new();

        let ref_mut_i32 = Type::Reference {
            lifetime: None,
            mutable: true,
            inner: Box::new(Type::Named("i32".to_string())),
        };

        let mangled = registry.type_to_mangled_name(&ref_mut_i32);
        assert!(mangled.contains("ref_mut"));
    }

    #[test]
    fn test_type_mangling_pointer_types() {
        let registry = MonomorphizationRegistry::new();

        let ptr_i32 = Type::Pointer {
            mutable: false,
            inner: Box::new(Type::Named("i32".to_string())),
        };

        let mangled = registry.type_to_mangled_name(&ptr_i32);
        assert!(mangled.contains("ptr"));
        assert!(mangled.contains("i32"));
    }

    #[test]
    fn test_type_mangling_array_types() {
        let registry = MonomorphizationRegistry::new();

        let array = Type::Array {
            element: Box::new(Type::Named("i32".to_string())),
            size: None,
        };

        let mangled = registry.type_to_mangled_name(&array);
        assert!(mangled.contains("array"));
        assert!(mangled.contains("i32"));
    }

    #[test]
    fn test_type_mangling_tuple_types() {
        let registry = MonomorphizationRegistry::new();

        let tuple = Type::Tuple(vec![
            Type::Named("i32".to_string()),
            Type::Named("bool".to_string()),
        ]);

        let mangled = registry.type_to_mangled_name(&tuple);
        assert!(mangled.contains("tuple"));
        assert!(mangled.contains("i32"));
        assert!(mangled.contains("bool"));
    }

    #[test]
    fn test_instantiate_function() {
        let mut registry = MonomorphizationRegistry::new();
        let generics = vec![GenericParam::Type {
            name: "T".to_string(),
            bounds: vec![],
            default: None,
        }];

        registry.register_generic("identity".to_string(), generics);

        let type_args = vec![Type::Named("i32".to_string())];
        let result = registry.instantiate("identity", type_args);

        assert!(result.is_ok());
        let concrete_name = result.unwrap();
        assert!(concrete_name.contains("identity"));
        assert!(concrete_name.contains("i32"));
    }

    #[test]
    fn test_instantiate_deduplication() {
        let mut registry = MonomorphizationRegistry::new();
        let generics = vec![GenericParam::Type {
            name: "T".to_string(),
            bounds: vec![],
            default: None,
        }];

        registry.register_generic("identity".to_string(), generics);

        let type_args = vec![Type::Named("i32".to_string())];

        let result1 = registry.instantiate("identity", type_args.clone());
        let result2 = registry.instantiate("identity", type_args.clone());

        assert_eq!(result1.unwrap(), result2.unwrap());
        assert_eq!(registry.instances.len(), 1);
    }

    #[test]
    fn test_multiple_instantiations() {
        let mut registry = MonomorphizationRegistry::new();
        let generics = vec![GenericParam::Type {
            name: "T".to_string(),
            bounds: vec![],
            default: None,
        }];

        registry.register_generic("wrap".to_string(), generics);

        let result1 = registry.instantiate("wrap", vec![Type::Named("i32".to_string())]);
        let result2 = registry.instantiate("wrap", vec![Type::Named("bool".to_string())]);

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_ne!(result1.unwrap(), result2.unwrap());
        assert_eq!(registry.instances.len(), 2);
    }

    #[test]
    fn test_error_on_missing_generic() {
        let mut registry = MonomorphizationRegistry::new();
        let result = registry.instantiate("nonexistent", vec![Type::Named("i32".to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_on_wrong_arity() {
        let mut registry = MonomorphizationRegistry::new();
        let generics = vec![
            GenericParam::Type {
                name: "T".to_string(),
                bounds: vec![],
                default: None,
            },
            GenericParam::Type {
                name: "U".to_string(),
                bounds: vec![],
                default: None,
            },
        ];

        registry.register_generic("pair".to_string(), generics);

        let result = registry.instantiate("pair", vec![Type::Named("i32".to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn test_collect_generics_from_items() {
        let block = crate::parser::ast::Block {
            statements: vec![],
            expression: None,
        };
        let items = vec![
            Item::Function {
                name: "generic_add".to_string(),
                generics: vec![GenericParam::Type {
                    name: "T".to_string(),
                    bounds: vec![],
                    default: None,
                }],
                params: vec![],
                return_type: None,
                body: block.clone(),
                is_unsafe: false,
                is_async: false,
                is_pub: false,
                attributes: vec![],
            },
            Item::Function {
                name: "normal_func".to_string(),
                generics: vec![],
                params: vec![],
                return_type: None,
                body: block,
                is_unsafe: false,
                is_async: false,
                is_pub: false,
                attributes: vec![],
            },
        ];

        let generics_map = collect_generics(&items);
        assert_eq!(generics_map.len(), 1);
        assert!(generics_map.contains_key("generic_add"));
        assert!(!generics_map.contains_key("normal_func"));
    }

    #[test]
    fn test_get_instantiations() {
        let mut registry = MonomorphizationRegistry::new();
        let generics = vec![GenericParam::Type {
            name: "T".to_string(),
            bounds: vec![],
            default: None,
        }];

        registry.register_generic("wrapper".to_string(), generics);

        let type_args1 = vec![Type::Named("i32".to_string())];
        let type_args2 = vec![Type::Named("f64".to_string())];

        let _ = registry.instantiate("wrapper", type_args1);
        let _ = registry.instantiate("wrapper", type_args2);

        let instantiations = registry.get_instantiations("wrapper");
        assert_eq!(instantiations.len(), 2);

        for inst in &instantiations {
            assert_eq!(inst.original_name, "wrapper");
        }
    }
}
