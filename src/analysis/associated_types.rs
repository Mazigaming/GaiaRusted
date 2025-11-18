//! Associated Types and Where Clauses
//!
//! Implements associated types (e.g., trait Item = T) and generic where bounds

use std::collections::HashMap;

/// Associated type
#[derive(Debug, Clone)]
pub struct AssociatedType {
    pub name: String,
    pub default: Option<String>,
    pub bounds: Vec<String>,
}

/// Where clause
#[derive(Debug, Clone)]
pub struct WhereClause {
    pub generic_param: String,
    pub bounds: Vec<String>,
}

/// Trait with associated types
#[derive(Debug, Clone)]
pub struct TraitWithAssoc {
    pub name: String,
    pub generic_params: Vec<String>,
    pub associated_types: Vec<AssociatedType>,
    pub where_clauses: Vec<WhereClause>,
}

/// Type binding for associated type
#[derive(Debug, Clone)]
pub struct TypeBinding {
    pub associated_type: String,
    pub concrete_type: String,
}

/// Generic impl with where bounds
#[derive(Debug, Clone)]
pub struct GenericImpl {
    pub trait_name: String,
    pub type_name: String,
    pub generic_params: Vec<String>,
    pub type_bindings: Vec<TypeBinding>,
    pub where_clauses: Vec<WhereClause>,
}

/// Associated type resolver
pub struct AssociatedTypeResolver {
    trait_assoc: HashMap<String, Vec<AssociatedType>>,
    type_bindings: HashMap<(String, String), String>,  // (impl, assoc_type) -> concrete_type
}

impl AssociatedTypeResolver {
    /// Create new resolver
    pub fn new() -> Self {
        AssociatedTypeResolver {
            trait_assoc: HashMap::new(),
            type_bindings: HashMap::new(),
        }
    }

    /// Register associated types for trait
    pub fn register_assoc_types(&mut self, trait_name: String, assoc_types: Vec<AssociatedType>) {
        self.trait_assoc.insert(trait_name, assoc_types);
    }

    /// Bind associated type to concrete type
    pub fn bind_type(&mut self, impl_id: String, assoc_name: String, concrete_type: String) {
        self.type_bindings.insert((impl_id, assoc_name), concrete_type);
    }

    /// Resolve associated type
    pub fn resolve_assoc_type(&self, impl_id: &str, assoc_name: &str) -> Option<String> {
        self.type_bindings.get(&(impl_id.to_string(), assoc_name.to_string())).cloned()
    }

    /// Get all associated types for trait
    pub fn get_assoc_types(&self, trait_name: &str) -> Option<&Vec<AssociatedType>> {
        self.trait_assoc.get(trait_name)
    }

    /// Generate trait with assoc types code
    pub fn generate_trait_code(&self, trait_def: &TraitWithAssoc) -> String {
        let mut code = String::new();

        code.push_str(&format!("pub trait {}", trait_def.name));

        if !trait_def.generic_params.is_empty() {
            code.push_str(&format!("<{}>", trait_def.generic_params.join(", ")));
        }

        code.push_str(" {\n");

        // Associated types
        for assoc in &trait_def.associated_types {
            code.push_str(&format!("    type {} {};",
                assoc.name,
                if let Some(default) = &assoc.default {
                    format!("= {}", default)
                } else {
                    ";".to_string()
                }
            ));
        }

        code.push_str("}\n");

        // Where clauses
        if !trait_def.where_clauses.is_empty() {
            code.push_str("where\n");
            for clause in &trait_def.where_clauses {
                code.push_str(&format!("    {}: {},\n", clause.generic_param, clause.bounds.join(" + ")));
            }
        }

        code
    }

    /// Generate impl with where bounds
    pub fn generate_impl_code(&self, impl_def: &GenericImpl) -> String {
        let mut code = String::new();

        code.push_str(&format!("impl"));

        if !impl_def.generic_params.is_empty() {
            code.push_str(&format!("<{}>", impl_def.generic_params.join(", ")));
        }

        code.push_str(&format!(" {} for {}", impl_def.trait_name, impl_def.type_name));

        if !impl_def.where_clauses.is_empty() {
            code.push_str("\nwhere\n");
            for clause in &impl_def.where_clauses {
                code.push_str(&format!("    {}: {},\n", clause.generic_param, clause.bounds.join(" + ")));
            }
        }

        code.push_str(" {\n");

        // Type bindings
        for binding in &impl_def.type_bindings {
            code.push_str(&format!("    type {} = {};\n", binding.associated_type, binding.concrete_type));
        }

        code.push_str("}\n");

        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_associated_type_creation() {
        let assoc = AssociatedType {
            name: "Item".to_string(),
            default: None,
            bounds: vec!["Clone".to_string()],
        };
        assert_eq!(assoc.name, "Item");
    }

    #[test]
    fn test_where_clause_creation() {
        let where_clause = WhereClause {
            generic_param: "T".to_string(),
            bounds: vec!["Clone".to_string(), "Debug".to_string()],
        };
        assert_eq!(where_clause.generic_param, "T");
        assert_eq!(where_clause.bounds.len(), 2);
    }

    #[test]
    fn test_resolver_creation() {
        let resolver = AssociatedTypeResolver::new();
        assert_eq!(resolver.trait_assoc.len(), 0);
    }

    #[test]
    fn test_register_assoc_types() {
        let mut resolver = AssociatedTypeResolver::new();
        let assoc_types = vec![
            AssociatedType {
                name: "Item".to_string(),
                default: None,
                bounds: vec![],
            },
        ];
        resolver.register_assoc_types("Iterator".to_string(), assoc_types);
        assert!(resolver.trait_assoc.contains_key("Iterator"));
    }

    #[test]
    fn test_type_binding() {
        let mut resolver = AssociatedTypeResolver::new();
        resolver.bind_type(
            "VecImpl".to_string(),
            "Item".to_string(),
            "i32".to_string(),
        );

        let resolved = resolver.resolve_assoc_type("VecImpl", "Item");
        assert_eq!(resolved, Some("i32".to_string()));
    }

    #[test]
    fn test_generate_trait_code() {
        let trait_def = TraitWithAssoc {
            name: "Iterator".to_string(),
            generic_params: vec![],
            associated_types: vec![
                AssociatedType {
                    name: "Item".to_string(),
                    default: None,
                    bounds: vec![],
                },
            ],
            where_clauses: vec![],
        };

        let resolver = AssociatedTypeResolver::new();
        let code = resolver.generate_trait_code(&trait_def);
        assert!(code.contains("pub trait Iterator"));
        assert!(code.contains("type Item"));
    }

    #[test]
    fn test_generate_impl_with_where() {
        let impl_def = GenericImpl {
            trait_name: "Clone".to_string(),
            type_name: "MyType".to_string(),
            generic_params: vec!["T".to_string()],
            type_bindings: vec![],
            where_clauses: vec![
                WhereClause {
                    generic_param: "T".to_string(),
                    bounds: vec!["Clone".to_string()],
                },
            ],
        };

        let resolver = AssociatedTypeResolver::new();
        let code = resolver.generate_impl_code(&impl_def);
        assert!(code.contains("impl<T>"));
        assert!(code.contains("where"));
    }
}
