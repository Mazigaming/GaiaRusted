//! # Impl Block Support System
//!
//! Complete support for impl blocks with methods, trait implementations,
//! and generic impl blocks.
//!
//! Features:
//! - Impl block registration (inherent and trait)
//! - Method tracking and validation
//! - Trait implementation verification
//! - Generic impl block support
//! - Associated type integration
//! - Method dispatch information
//! - Comprehensive error detection

use std::collections::{HashMap, HashSet};
use crate::typesystem::types::{Type, StructId, TraitId};

/// Configuration for impl block analysis
#[derive(Debug, Clone)]
pub struct ImplBlockConfig {
    /// Maximum methods per impl block
    pub max_methods_per_impl: usize,
    /// Maximum generic parameters
    pub max_generic_params: usize,
    /// Whether to allow trait implementations
    pub allow_trait_impls: bool,
    /// Whether to validate method names uniqueness
    pub check_method_uniqueness: bool,
}

impl Default for ImplBlockConfig {
    fn default() -> Self {
        ImplBlockConfig {
            max_methods_per_impl: 100,
            max_generic_params: 16,
            allow_trait_impls: true,
            check_method_uniqueness: true,
        }
    }
}

/// Information about a method in an impl block
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Option<Type>,
    pub is_static: bool,
    pub is_public: bool,
}

impl MethodInfo {
    /// Get method signature for display
    pub fn signature(&self) -> String {
        let params = self
            .params
            .iter()
            .map(|(name, _ty)| name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        format!("fn {}({})", self.name, params)
    }
}

/// Information about a generic parameter
#[derive(Debug, Clone)]
pub struct GenericParamInfo {
    pub name: String,
    pub bounds: Vec<String>,
}

/// Information about an impl block
#[derive(Debug, Clone)]
pub struct ImplBlockInfo {
    pub name: String,
    pub struct_name: String,
    pub struct_id: Option<StructId>,
    pub trait_name: Option<String>,
    pub trait_id: Option<TraitId>,
    pub methods: Vec<MethodInfo>,
    pub generic_params: Vec<GenericParamInfo>,
    pub is_trait_impl: bool,
    pub total_methods: usize,
}

/// Method dispatch information
#[derive(Debug, Clone)]
pub struct MethodDispatchInfo {
    pub impl_name: String,
    pub method_name: String,
    pub struct_name: String,
    pub trait_name: Option<String>,
    pub method_info: MethodInfo,
}

/// Analysis result for impl blocks
#[derive(Debug, Clone)]
pub struct ImplBlockAnalysisReport {
    pub impl_blocks: HashMap<String, ImplBlockInfo>,
    pub method_dispatch: Vec<MethodDispatchInfo>,
    pub unimplemented_trait_methods: Vec<(String, String)>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Main impl block analyzer
pub struct ImplBlockAnalyzer {
    config: ImplBlockConfig,
    impl_blocks: HashMap<String, ImplBlockInfo>,
    method_index: HashMap<String, Vec<MethodDispatchInfo>>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl ImplBlockAnalyzer {
    /// Create a new impl block analyzer
    pub fn new(config: ImplBlockConfig) -> Self {
        ImplBlockAnalyzer {
            config,
            impl_blocks: HashMap::new(),
            method_index: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Register an impl block
    pub fn register_impl_block(
        &mut self,
        name: String,
        struct_name: String,
        trait_name: Option<String>,
        methods: Vec<MethodInfo>,
        generic_params: Vec<GenericParamInfo>,
    ) -> Result<(), String> {
        // Check if already registered
        if self.impl_blocks.contains_key(&name) {
            return Err(format!("Impl block '{}' already registered", name));
        }

        // Validate method count
        if methods.len() > self.config.max_methods_per_impl {
            return Err(format!(
                "Impl block '{}' has too many methods ({} > {})",
                name,
                methods.len(),
                self.config.max_methods_per_impl
            ));
        }

        // Validate generic params count
        if generic_params.len() > self.config.max_generic_params {
            return Err(format!(
                "Impl block '{}' has too many generic params ({} > {})",
                name,
                generic_params.len(),
                self.config.max_generic_params
            ));
        }

        // Check for trait impls if disabled
        if trait_name.is_some() && !self.config.allow_trait_impls {
            return Err(format!(
                "Trait implementations are disabled, but impl block '{}' implements trait",
                name
            ));
        }

        // Check for duplicate method names
        if self.config.check_method_uniqueness {
            let mut seen = HashSet::new();
            for method in &methods {
                if !seen.insert(&method.name) {
                    return Err(format!(
                        "Impl block '{}' has duplicate method '{}'",
                        name, method.name
                    ));
                }
            }
        }

        let impl_info = ImplBlockInfo {
            name: name.clone(),
            struct_name: struct_name.clone(),
            struct_id: None,
            trait_name: trait_name.clone(),
            trait_id: None,
            methods: methods.clone(),
            generic_params,
            is_trait_impl: trait_name.is_some(),
            total_methods: methods.len(),
        };

        // Index methods for dispatch lookup
        for method in &methods {
            let dispatch_info = MethodDispatchInfo {
                impl_name: name.clone(),
                method_name: method.name.clone(),
                struct_name: struct_name.clone(),
                trait_name: trait_name.clone(),
                method_info: method.clone(),
            };

            self.method_index
                .entry(method.name.clone())
                .or_insert_with(Vec::new)
                .push(dispatch_info);
        }

        self.impl_blocks.insert(name, impl_info);
        Ok(())
    }

    /// Get impl block info by name
    pub fn get_impl_block(&self, name: &str) -> Option<&ImplBlockInfo> {
        self.impl_blocks.get(name)
    }

    /// Get all impl blocks
    pub fn impl_blocks(&self) -> &HashMap<String, ImplBlockInfo> {
        &self.impl_blocks
    }

    /// Get impl blocks for a specific struct
    pub fn get_impl_blocks_for_struct(&self, struct_name: &str) -> Vec<&ImplBlockInfo> {
        self.impl_blocks
            .values()
            .filter(|impl_info| impl_info.struct_name == struct_name)
            .collect()
    }

    /// Get trait impl blocks for a struct
    pub fn get_trait_impls_for_struct(
        &self,
        struct_name: &str,
    ) -> Vec<(&ImplBlockInfo, &str)> {
        self.impl_blocks
            .values()
            .filter(|impl_info| {
                impl_info.struct_name == struct_name && impl_info.trait_name.is_some()
            })
            .map(|impl_info| {
                (
                    impl_info,
                    impl_info.trait_name.as_deref().unwrap_or(""),
                )
            })
            .collect()
    }

    /// Find method by name and struct
    pub fn find_method(
        &self,
        struct_name: &str,
        method_name: &str,
    ) -> Option<MethodDispatchInfo> {
        self.method_index
            .get(method_name)
            .and_then(|methods| {
                methods
                    .iter()
                    .find(|m| m.struct_name == struct_name)
                    .cloned()
            })
    }

    /// Get all methods for a struct
    pub fn get_methods_for_struct(&self, struct_name: &str) -> Vec<&MethodInfo> {
        self.impl_blocks
            .values()
            .filter(|impl_info| impl_info.struct_name == struct_name)
            .flat_map(|impl_info| &impl_info.methods)
            .collect()
    }

    /// Validate trait implementation completeness
    pub fn validate_trait_impl(
        &mut self,
        impl_name: &str,
        required_methods: Vec<&str>,
    ) -> Result<(), String> {
        if let Some(impl_info) = self.impl_blocks.get(impl_name) {
            if impl_info.trait_name.is_none() {
                return Err(format!(
                    "Impl block '{}' is not a trait implementation",
                    impl_name
                ));
            }

            let provided: HashSet<_> = impl_info
                .methods
                .iter()
                .map(|m| m.name.as_str())
                .collect();
            let required: HashSet<_> = required_methods.into_iter().collect();

            // Check for missing methods
            let missing: Vec<_> = required.difference(&provided).map(|s| *s).collect();
            if !missing.is_empty() {
                let missing_str = missing.join(", ");
                return Err(format!(
                    "Impl block '{}' missing required methods: {}",
                    impl_name, missing_str
                ));
            }

            // Check for extra methods
            let extra: Vec<_> = provided.difference(&required).map(|s| *s).collect();
            if !extra.is_empty() {
                let extra_str = extra.join(", ");
                self.warnings.push(format!(
                    "Impl block '{}' has extra methods not in trait: {}",
                    impl_name, extra_str
                ));
            }

            Ok(())
        } else {
            Err(format!("Impl block '{}' not found", impl_name))
        }
    }

    /// Check if struct has impl block
    pub fn has_impl_block(&self, struct_name: &str) -> bool {
        self.impl_blocks
            .values()
            .any(|impl_info| impl_info.struct_name == struct_name)
    }

    /// Check if method exists for struct
    pub fn has_method(&self, struct_name: &str, method_name: &str) -> bool {
        self.impl_blocks
            .values()
            .filter(|impl_info| impl_info.struct_name == struct_name)
            .any(|impl_info| impl_info.methods.iter().any(|m| m.name == method_name))
    }

    /// Get generic impl blocks (those with generic parameters)
    pub fn get_generic_impl_blocks(&self) -> Vec<&ImplBlockInfo> {
        self.impl_blocks
            .values()
            .filter(|impl_info| !impl_info.generic_params.is_empty())
            .collect()
    }

    /// Add an error message
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Add a warning message
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get all errors
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Generate the analysis report
    pub fn generate_report(self) -> ImplBlockAnalysisReport {
        let method_dispatch = self
            .method_index
            .values()
            .flat_map(|methods| methods.clone())
            .collect();

        ImplBlockAnalysisReport {
            impl_blocks: self.impl_blocks,
            method_dispatch,
            unimplemented_trait_methods: Vec::new(),
            errors: self.errors,
            warnings: self.warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_analyzer() -> ImplBlockAnalyzer {
        ImplBlockAnalyzer::new(ImplBlockConfig::default())
    }

    fn create_simple_method() -> MethodInfo {
        MethodInfo {
            name: "method".to_string(),
            params: vec![("self".to_string(), Type::I64)],
            return_type: Some(Type::I64),
            is_static: false,
            is_public: true,
        }
    }

    #[test]
    fn test_simple_impl_block_registration() {
        let mut analyzer = create_test_analyzer();

        let result = analyzer.register_impl_block(
            "impl_Point".to_string(),
            "Point".to_string(),
            None,
            vec![create_simple_method()],
            vec![],
        );

        assert!(result.is_ok());
        assert!(analyzer.get_impl_block("impl_Point").is_some());
    }

    #[test]
    fn test_inherent_impl_block() {
        let mut analyzer = create_test_analyzer();

        let result = analyzer.register_impl_block(
            "impl_String".to_string(),
            "String".to_string(),
            None,
            vec![MethodInfo {
                name: "len".to_string(),
                params: vec![("self".to_string(), Type::Str)],
                return_type: Some(Type::Usize),
                is_static: false,
                is_public: true,
            }],
            vec![],
        );

        assert!(result.is_ok());
        let impl_info = analyzer.get_impl_block("impl_String").unwrap();
        assert!(!impl_info.is_trait_impl);
        assert!(impl_info.trait_name.is_none());
    }

    #[test]
    fn test_trait_impl_block() {
        let mut analyzer = create_test_analyzer();

        let result = analyzer.register_impl_block(
            "impl_Iterator_for_Range".to_string(),
            "Range".to_string(),
            Some("Iterator".to_string()),
            vec![MethodInfo {
                name: "next".to_string(),
                params: vec![("self".to_string(), Type::I64)],
                return_type: Some(Type::Unit),
                is_static: false,
                is_public: true,
            }],
            vec![],
        );

        assert!(result.is_ok());
        let impl_info = analyzer.get_impl_block("impl_Iterator_for_Range").unwrap();
        assert!(impl_info.is_trait_impl);
        assert_eq!(impl_info.trait_name, Some("Iterator".to_string()));
    }

    #[test]
    fn test_multiple_methods() {
        let mut analyzer = create_test_analyzer();

        let methods = vec![
            MethodInfo {
                name: "new".to_string(),
                params: vec![],
                return_type: Some(Type::Struct(StructId(1))),
                is_static: true,
                is_public: true,
            },
            MethodInfo {
                name: "process".to_string(),
                params: vec![("self".to_string(), Type::I64)],
                return_type: None,
                is_static: false,
                is_public: true,
            },
            MethodInfo {
                name: "result".to_string(),
                params: vec![("self".to_string(), Type::I64)],
                return_type: Some(Type::I64),
                is_static: false,
                is_public: true,
            },
        ];

        let result = analyzer.register_impl_block(
            "impl_Processor".to_string(),
            "Processor".to_string(),
            None,
            methods,
            vec![],
        );

        assert!(result.is_ok());
        let impl_info = analyzer.get_impl_block("impl_Processor").unwrap();
        assert_eq!(impl_info.total_methods, 3);
    }

    #[test]
    fn test_duplicate_method_names_error() {
        let mut analyzer = create_test_analyzer();

        let methods = vec![
            MethodInfo {
                name: "process".to_string(),
                params: vec![],
                return_type: None,
                is_static: false,
                is_public: true,
            },
            MethodInfo {
                name: "process".to_string(),
                params: vec![],
                return_type: None,
                is_static: false,
                is_public: true,
            },
        ];

        let result = analyzer.register_impl_block(
            "impl_Duplicate".to_string(),
            "Dummy".to_string(),
            None,
            methods,
            vec![],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_generic_impl_block() {
        let mut analyzer = create_test_analyzer();

        let generic_params = vec![GenericParamInfo {
            name: "T".to_string(),
            bounds: vec!["Clone".to_string()],
        }];

        let result = analyzer.register_impl_block(
            "impl_Vec_T".to_string(),
            "Vec".to_string(),
            None,
            vec![create_simple_method()],
            generic_params,
        );

        assert!(result.is_ok());
        let impl_info = analyzer.get_impl_block("impl_Vec_T").unwrap();
        assert_eq!(impl_info.generic_params.len(), 1);
        assert_eq!(impl_info.generic_params[0].name, "T");
    }

    #[test]
    fn test_method_finding() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_impl_block(
                "impl_Point".to_string(),
                "Point".to_string(),
                None,
                vec![MethodInfo {
                    name: "distance".to_string(),
                    params: vec![("self".to_string(), Type::I64)],
                    return_type: Some(Type::F64),
                    is_static: false,
                    is_public: true,
                }],
                vec![],
            )
            .unwrap();

        let method = analyzer.find_method("Point", "distance");
        assert!(method.is_some());
        assert_eq!(method.unwrap().method_name, "distance");
    }

    #[test]
    fn test_get_impl_blocks_for_struct() {
        let mut analyzer = create_test_analyzer();

        // First inherent impl
        analyzer
            .register_impl_block(
                "impl_Point_1".to_string(),
                "Point".to_string(),
                None,
                vec![create_simple_method()],
                vec![],
            )
            .ok();

        // Second inherent impl
        analyzer
            .register_impl_block(
                "impl_Point_2".to_string(),
                "Point".to_string(),
                None,
                vec![create_simple_method()],
                vec![],
            )
            .ok();

        let impl_blocks = analyzer.get_impl_blocks_for_struct("Point");
        assert_eq!(impl_blocks.len(), 2);
    }

    #[test]
    fn test_trait_impl_validation() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_impl_block(
                "impl_Iterator_for_Custom".to_string(),
                "Custom".to_string(),
                Some("Iterator".to_string()),
                vec![
                    MethodInfo {
                        name: "next".to_string(),
                        params: vec![],
                        return_type: Some(Type::Unit),
                        is_static: false,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "size_hint".to_string(),
                        params: vec![],
                        return_type: Some(Type::Tuple(vec![])),
                        is_static: false,
                        is_public: true,
                    },
                ],
                vec![],
            )
            .unwrap();

        let result = analyzer.validate_trait_impl(
            "impl_Iterator_for_Custom",
            vec!["next", "size_hint"],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_trait_impl_missing_methods() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_impl_block(
                "impl_Iterator_for_Custom".to_string(),
                "Custom".to_string(),
                Some("Iterator".to_string()),
                vec![MethodInfo {
                    name: "next".to_string(),
                    params: vec![],
                    return_type: Some(Type::Unit),
                    is_static: false,
                    is_public: true,
                }],
                vec![],
            )
            .unwrap();

        let result = analyzer.validate_trait_impl(
            "impl_Iterator_for_Custom",
            vec!["next", "size_hint"],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_has_method() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_impl_block(
                "impl_String".to_string(),
                "String".to_string(),
                None,
                vec![MethodInfo {
                    name: "len".to_string(),
                    params: vec![],
                    return_type: Some(Type::Usize),
                    is_static: false,
                    is_public: true,
                }],
                vec![],
            )
            .ok();

        assert!(analyzer.has_method("String", "len"));
        assert!(!analyzer.has_method("String", "push"));
    }

    #[test]
    fn test_report_generation() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_impl_block(
                "impl_Point".to_string(),
                "Point".to_string(),
                None,
                vec![create_simple_method()],
                vec![],
            )
            .ok();

        let report = analyzer.generate_report();
        assert_eq!(report.impl_blocks.len(), 1);
        assert!(report.impl_blocks.contains_key("impl_Point"));
    }

    #[test]
    fn test_max_methods_validation() {
        let mut config = ImplBlockConfig::default();
        config.max_methods_per_impl = 2;
        let mut analyzer = ImplBlockAnalyzer::new(config);

        let methods = vec![
            create_simple_method(),
            create_simple_method(),
            create_simple_method(),
        ];

        let result = analyzer.register_impl_block(
            "impl_TooMany".to_string(),
            "Dummy".to_string(),
            None,
            methods,
            vec![],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_max_generics_validation() {
        let mut config = ImplBlockConfig::default();
        config.max_generic_params = 2;
        let mut analyzer = ImplBlockAnalyzer::new(config);

        let generics = vec![
            GenericParamInfo {
                name: "A".to_string(),
                bounds: vec![],
            },
            GenericParamInfo {
                name: "B".to_string(),
                bounds: vec![],
            },
            GenericParamInfo {
                name: "C".to_string(),
                bounds: vec![],
            },
        ];

        let result = analyzer.register_impl_block(
            "impl_TooManyGenerics".to_string(),
            "Dummy".to_string(),
            None,
            vec![create_simple_method()],
            generics,
        );

        assert!(result.is_err());
    }
}
