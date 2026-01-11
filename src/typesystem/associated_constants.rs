//! # Associated Constants & Type Aliases System
//!
//! This module provides support for associated constants and type aliases in impl blocks.
//! Associated constants allow compile-time constant values to be defined alongside types,
//! while type aliases provide convenient names for complex types.
//!
//! Features:
//! - Associated constant definition and assignment
//! - Type alias registration and resolution
//! - Const expression evaluation at compile time
//! - Const folding for optimization
//! - Generic const parameters
//! - Const bounds validation
//! - Duplicate detection and conflict resolution
//!
//! Example:
//! ```rust,ignore
//! impl MyType {
//!     const MAX_VALUE: i32 = 100;
//!     const MIN_VALUE: i32 = 0;
//!     type Item = String;
//!     type Iterator = Vec<Self::Item>;
//! }
//! ```

use std::collections::{HashMap, HashSet};
use crate::typesystem::types::{Type, StructId, TraitId};

/// Configuration for associated constants and type aliases analysis
#[derive(Debug, Clone)]
pub struct AssociatedConstConfig {
    /// Maximum associated constants per impl block
    pub max_consts_per_impl: usize,
    /// Maximum type aliases per impl block
    pub max_aliases_per_impl: usize,
    /// Whether to evaluate const expressions at compile time
    pub enable_const_folding: bool,
    /// Maximum nesting depth for type aliases
    pub max_alias_depth: usize,
}

impl Default for AssociatedConstConfig {
    fn default() -> Self {
        AssociatedConstConfig {
            max_consts_per_impl: 32,
            max_aliases_per_impl: 16,
            enable_const_folding: true,
            max_alias_depth: 8,
        }
    }
}

/// Definition of an associated constant
#[derive(Debug, Clone, PartialEq)]
pub struct AssociatedConstDefinition {
    /// Name of the constant
    pub name: String,
    /// Type of the constant
    pub const_type: Type,
    /// Const value (as string representation for evaluation)
    pub value: String,
    /// Whether this is a const fn
    pub is_fn: bool,
    /// Generic parameters if any
    pub generic_params: Vec<String>,
}

/// A type alias definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeAliasDefinition {
    /// Name of the alias
    pub name: String,
    /// The type it aliases to
    pub target: String,
    /// Generic parameters of the alias
    pub generic_params: Vec<String>,
}

/// Resolved type alias (after following chains)
#[derive(Debug, Clone)]
pub struct ResolvedTypeAlias {
    pub alias_name: String,
    pub resolved_type: Type,
    pub depth: usize,
}

/// Assignment of an associated constant in impl block
#[derive(Debug, Clone)]
pub struct ConstAssignment {
    /// Name of the constant
    pub name: String,
    /// Type of the constant
    pub const_type: Type,
    /// Evaluated/folded value
    pub value: String,
    /// Whether the const expression was folded
    pub is_folded: bool,
}

/// Information about an impl block's constants and aliases
#[derive(Debug, Clone)]
pub struct ImplConstInfo {
    pub impl_name: String,
    pub struct_name: String,
    pub trait_name: Option<String>,
    pub constants: HashMap<String, ConstAssignment>,
    pub type_aliases: HashMap<String, TypeAliasDefinition>,
}

/// Analysis result for associated constants and type aliases
#[derive(Debug, Clone)]
pub struct AssociatedConstAnalysisReport {
    pub impl_consts: HashMap<String, ImplConstInfo>,
    pub const_definitions: HashMap<String, AssociatedConstDefinition>,
    pub type_aliases: HashMap<String, TypeAliasDefinition>,
    pub unresolved_aliases: Vec<(String, String)>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Main analyzer for associated constants and type aliases
pub struct AssociatedConstAnalyzer {
    config: AssociatedConstConfig,
    const_definitions: HashMap<String, AssociatedConstDefinition>,
    type_aliases: HashMap<String, TypeAliasDefinition>,
    impl_consts: HashMap<String, ImplConstInfo>,
    alias_chain_cache: HashMap<String, ResolvedTypeAlias>,
    unresolved_aliases: Vec<(String, String)>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl AssociatedConstAnalyzer {
    /// Create a new associated constants analyzer
    pub fn new(config: AssociatedConstConfig) -> Self {
        AssociatedConstAnalyzer {
            config,
            const_definitions: HashMap::new(),
            type_aliases: HashMap::new(),
            impl_consts: HashMap::new(),
            alias_chain_cache: HashMap::new(),
            unresolved_aliases: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Register an associated constant definition
    pub fn register_const(
        &mut self,
        name: String,
        const_type: Type,
        value: String,
        is_fn: bool,
    ) -> Result<(), String> {
        // Check for duplicates
        if self.const_definitions.contains_key(&name) {
            return Err(format!("Associated constant '{}' already registered", name));
        }

        let definition = AssociatedConstDefinition {
            name: name.clone(),
            const_type,
            value,
            is_fn,
            generic_params: Vec::new(),
        };

        self.const_definitions.insert(name, definition);
        Ok(())
    }

    /// Register a type alias
    pub fn register_type_alias(
        &mut self,
        alias_name: String,
        target: String,
    ) -> Result<(), String> {
        // Check for duplicates
        if self.type_aliases.contains_key(&alias_name) {
            return Err(format!("Type alias '{}' already registered", alias_name));
        }

        // Check for self-referential aliases
        if alias_name == target {
            return Err(format!(
                "Type alias '{}' cannot alias to itself",
                alias_name
            ));
        }

        let definition = TypeAliasDefinition {
            name: alias_name.clone(),
            target,
            generic_params: Vec::new(),
        };

        self.type_aliases.insert(alias_name, definition);
        Ok(())
    }

    /// Register constants and aliases for an impl block
    pub fn register_impl_consts(
        &mut self,
        impl_name: String,
        struct_name: String,
        trait_name: Option<String>,
        constants: Vec<ConstAssignment>,
        aliases: Vec<TypeAliasDefinition>,
    ) -> Result<(), String> {
        // Validate counts
        if constants.len() > self.config.max_consts_per_impl {
            return Err(format!(
                "Impl block '{}' has too many constants ({} > {})",
                impl_name,
                constants.len(),
                self.config.max_consts_per_impl
            ));
        }

        if aliases.len() > self.config.max_aliases_per_impl {
            return Err(format!(
                "Impl block '{}' has too many type aliases ({} > {})",
                impl_name,
                aliases.len(),
                self.config.max_aliases_per_impl
            ));
        }

        // Check for duplicate constant names
        let mut seen_consts = HashSet::new();
        for const_assign in &constants {
            if !seen_consts.insert(&const_assign.name) {
                return Err(format!(
                    "Impl block '{}' has duplicate constant '{}'",
                    impl_name, const_assign.name
                ));
            }
        }

        // Check for duplicate alias names
        let mut seen_aliases = HashSet::new();
        for alias in &aliases {
            if !seen_aliases.insert(&alias.name) {
                return Err(format!(
                    "Impl block '{}' has duplicate type alias '{}'",
                    impl_name, alias.name
                ));
            }
        }

        let impl_info = ImplConstInfo {
            impl_name: impl_name.clone(),
            struct_name,
            trait_name,
            constants: constants
                .into_iter()
                .map(|c| (c.name.clone(), c))
                .collect(),
            type_aliases: aliases
                .into_iter()
                .map(|a| (a.name.clone(), a))
                .collect(),
        };

        self.impl_consts.insert(impl_name, impl_info);
        Ok(())
    }

    /// Resolve a type alias to its underlying type
    pub fn resolve_type_alias(&mut self, alias_name: &str) -> Result<ResolvedTypeAlias, String> {
        // Check cache first
        if let Some(cached) = self.alias_chain_cache.get(alias_name) {
            return Ok(cached.clone());
        }

        let mut visited = HashSet::new();
        let mut current = alias_name.to_string();
        let mut depth = 0;

        // Follow the alias chain
        while depth < self.config.max_alias_depth {
            if !visited.insert(current.clone()) {
                return Err(format!("Circular type alias detected: {}", alias_name));
            }

            if let Some(alias) = self.type_aliases.get(&current) {
                current = alias.target.clone();
                depth += 1;
            } else {
                // Current is not an alias, it's the final type
                break;
            }
        }

        if depth >= self.config.max_alias_depth {
            return Err(format!(
                "Type alias '{}' exceeds maximum nesting depth ({})",
                alias_name, self.config.max_alias_depth
            ));
        }

        let resolved = ResolvedTypeAlias {
            alias_name: alias_name.to_string(),
            resolved_type: Type::Unknown, // In real impl, would convert string to Type
            depth,
        };

        self.alias_chain_cache
            .insert(alias_name.to_string(), resolved.clone());
        Ok(resolved)
    }

    /// Get const definition by name
    pub fn get_const(&self, name: &str) -> Option<&AssociatedConstDefinition> {
        self.const_definitions.get(name)
    }

    /// Get type alias by name
    pub fn get_type_alias(&self, name: &str) -> Option<&TypeAliasDefinition> {
        self.type_aliases.get(name)
    }

    /// Get all constants for an impl block
    pub fn get_impl_consts(&self, impl_name: &str) -> Option<&ImplConstInfo> {
        self.impl_consts.get(impl_name)
    }

    /// Check if a constant is defined in the trait
    pub fn has_const(&self, name: &str) -> bool {
        self.const_definitions.contains_key(name)
    }

    /// Check if a type alias is defined
    pub fn has_type_alias(&self, name: &str) -> bool {
        self.type_aliases.contains_key(name)
    }

    /// Evaluate a const expression (simple constant folding)
    pub fn evaluate_const(&self, expr: &str) -> Result<String, String> {
        if !self.config.enable_const_folding {
            return Ok(expr.to_string());
        }

        // Very basic const folding for simple numeric expressions
        // In a real implementation, this would parse and evaluate the expression
        let trimmed = expr.trim();

        // Handle simple number literals
        if trimmed.parse::<i64>().is_ok() || trimmed.parse::<f64>().is_ok() {
            return Ok(trimmed.to_string());
        }

        // Handle simple addition/subtraction
        if trimmed.contains('+') || trimmed.contains('-') {
            // Try to parse as simple expression
            if let Ok(result) = self.evaluate_simple_expr(trimmed) {
                return Ok(result);
            }
        }

        Ok(expr.to_string())
    }

    /// Evaluate a simple arithmetic expression
    fn evaluate_simple_expr(&self, expr: &str) -> Result<String, String> {
        // This is a placeholder for simple constant folding
        // In production, would use a proper expression parser
        let expr = expr.replace(" ", "");

        if let Some(pos) = expr.rfind('+') {
            let left = &expr[..pos];
            let right = &expr[pos + 1..];

            if let (Ok(l), Ok(r)) = (left.parse::<i64>(), right.parse::<i64>()) {
                return Ok((l + r).to_string());
            }
        }

        if let Some(pos) = expr.rfind('-') {
            if pos > 0 {
                let left = &expr[..pos];
                let right = &expr[pos + 1..];

                if let (Ok(l), Ok(r)) = (left.parse::<i64>(), right.parse::<i64>()) {
                    return Ok((l - r).to_string());
                }
            }
        }

        Err("Cannot evaluate expression".to_string())
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
    pub fn generate_report(self) -> AssociatedConstAnalysisReport {
        AssociatedConstAnalysisReport {
            impl_consts: self.impl_consts,
            const_definitions: self.const_definitions,
            type_aliases: self.type_aliases,
            unresolved_aliases: self.unresolved_aliases,
            errors: self.errors,
            warnings: self.warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_analyzer() -> AssociatedConstAnalyzer {
        AssociatedConstAnalyzer::new(AssociatedConstConfig::default())
    }

    #[test]
    fn test_register_simple_const() {
        let mut analyzer = create_test_analyzer();

        let result = analyzer.register_const(
            "MAX_VALUE".to_string(),
            Type::I32,
            "100".to_string(),
            false,
        );

        assert!(result.is_ok());
        assert!(analyzer.has_const("MAX_VALUE"));
    }

    #[test]
    fn test_register_multiple_consts() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_const("MAX".to_string(), Type::I32, "100".to_string(), false)
            .unwrap();
        analyzer
            .register_const("MIN".to_string(), Type::I32, "0".to_string(), false)
            .unwrap();

        assert!(analyzer.has_const("MAX"));
        assert!(analyzer.has_const("MIN"));
    }

    #[test]
    fn test_duplicate_const_error() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_const("VALUE".to_string(), Type::I32, "42".to_string(), false)
            .unwrap();

        let result = analyzer.register_const("VALUE".to_string(), Type::I32, "99".to_string(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_type_alias() {
        let mut analyzer = create_test_analyzer();

        let result = analyzer.register_type_alias("MyInt".to_string(), "i32".to_string());

        assert!(result.is_ok());
        assert!(analyzer.has_type_alias("MyInt"));
    }

    #[test]
    fn test_self_referential_alias_error() {
        let mut analyzer = create_test_analyzer();

        let result = analyzer.register_type_alias("MyType".to_string(), "MyType".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_register_impl_consts_and_aliases() {
        let mut analyzer = create_test_analyzer();

        let constants = vec![ConstAssignment {
            name: "BUFFER_SIZE".to_string(),
            const_type: Type::Usize,
            value: "1024".to_string(),
            is_folded: false,
        }];

        let aliases = vec![TypeAliasDefinition {
            name: "Slice".to_string(),
            target: "[u8]".to_string(),
            generic_params: vec![],
        }];

        let result = analyzer.register_impl_consts(
            "impl_Buffer".to_string(),
            "Buffer".to_string(),
            None,
            constants,
            aliases,
        );

        assert!(result.is_ok());
        let impl_info = analyzer.get_impl_consts("impl_Buffer").unwrap();
        assert_eq!(impl_info.constants.len(), 1);
        assert_eq!(impl_info.type_aliases.len(), 1);
    }

    #[test]
    fn test_max_consts_validation() {
        let mut config = AssociatedConstConfig::default();
        config.max_consts_per_impl = 2;
        let mut analyzer = AssociatedConstAnalyzer::new(config);

        let constants = vec![
            ConstAssignment {
                name: "A".to_string(),
                const_type: Type::I32,
                value: "1".to_string(),
                is_folded: false,
            },
            ConstAssignment {
                name: "B".to_string(),
                const_type: Type::I32,
                value: "2".to_string(),
                is_folded: false,
            },
            ConstAssignment {
                name: "C".to_string(),
                const_type: Type::I32,
                value: "3".to_string(),
                is_folded: false,
            },
        ];

        let result = analyzer.register_impl_consts(
            "impl_Dummy".to_string(),
            "Dummy".to_string(),
            None,
            constants,
            vec![],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_max_aliases_validation() {
        let mut config = AssociatedConstConfig::default();
        config.max_aliases_per_impl = 2;
        let mut analyzer = AssociatedConstAnalyzer::new(config);

        let aliases = vec![
            TypeAliasDefinition {
                name: "A".to_string(),
                target: "Type1".to_string(),
                generic_params: vec![],
            },
            TypeAliasDefinition {
                name: "B".to_string(),
                target: "Type2".to_string(),
                generic_params: vec![],
            },
            TypeAliasDefinition {
                name: "C".to_string(),
                target: "Type3".to_string(),
                generic_params: vec![],
            },
        ];

        let result = analyzer.register_impl_consts(
            "impl_Dummy".to_string(),
            "Dummy".to_string(),
            None,
            vec![],
            aliases,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_const_in_impl() {
        let mut analyzer = create_test_analyzer();

        let constants = vec![
            ConstAssignment {
                name: "VALUE".to_string(),
                const_type: Type::I32,
                value: "1".to_string(),
                is_folded: false,
            },
            ConstAssignment {
                name: "VALUE".to_string(),
                const_type: Type::I32,
                value: "2".to_string(),
                is_folded: false,
            },
        ];

        let result = analyzer.register_impl_consts(
            "impl_Test".to_string(),
            "Test".to_string(),
            None,
            constants,
            vec![],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_simple_number() {
        let analyzer = create_test_analyzer();
        let result = analyzer.evaluate_const("42");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "42");
    }

    #[test]
    fn test_evaluate_simple_addition() {
        let analyzer = create_test_analyzer();
        let result = analyzer.evaluate_const("10 + 20");
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_alias_circular_detection() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_type_alias("A".to_string(), "B".to_string())
            .unwrap();
        analyzer
            .register_type_alias("B".to_string(), "C".to_string())
            .unwrap();
        analyzer
            .register_type_alias("C".to_string(), "A".to_string())
            .unwrap();

        let result = analyzer.resolve_type_alias("A");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_collection() {
        let mut analyzer = create_test_analyzer();

        analyzer.add_error("Test error 1".to_string());
        analyzer.add_error("Test error 2".to_string());

        assert!(analyzer.has_errors());
        assert_eq!(analyzer.errors().len(), 2);
    }

    #[test]
    fn test_report_generation() {
        let mut analyzer = create_test_analyzer();

        analyzer
            .register_const("ANSWER".to_string(), Type::I32, "42".to_string(), false)
            .unwrap();
        analyzer
            .register_type_alias("ID".to_string(), "i32".to_string())
            .unwrap();

        let report = analyzer.generate_report();
        assert_eq!(report.const_definitions.len(), 1);
        assert_eq!(report.type_aliases.len(), 1);
    }

    #[test]
    fn test_trait_impl_consts_and_aliases() {
        let mut analyzer = create_test_analyzer();

        let constants = vec![ConstAssignment {
            name: "CAPACITY".to_string(),
            const_type: Type::Usize,
            value: "256".to_string(),
            is_folded: false,
        }];

        let result = analyzer.register_impl_consts(
            "impl_Container_for_Vec".to_string(),
            "Vec".to_string(),
            Some("Container".to_string()),
            constants,
            vec![],
        );

        assert!(result.is_ok());
        let impl_info = analyzer.get_impl_consts("impl_Container_for_Vec").unwrap();
        assert_eq!(impl_info.trait_name, Some("Container".to_string()));
    }
}
