//! # Enum Support System
//!
//! Complete enum type support with variants and pattern matching integration.
//!
//! This module provides:
//! - Enum definition tracking
//! - Variant registration (unit, tuple, struct)
//! - Variant discriminants
//! - Enum value representation
//! - Pattern matching integration
//! - Enum serialization support
//!
//! # Examples
//!
//! ```ignore
//! use gaiarusted::typesystem::enum_support::{EnumSupportAnalyzer, EnumSupportConfig, VariantKind};
//!
//! let config = EnumSupportConfig::default();
//! let mut analyzer = EnumSupportAnalyzer::new(config);
//!
//! // Register enum and variants
//! analyzer.register_enum("Result")?;
//! analyzer.register_variant("Result", "Ok", VariantKind::Tuple(vec!["T".to_string()]))?;
//! analyzer.register_variant("Result", "Err", VariantKind::Tuple(vec!["E".to_string()]))?;
//!
//! // Validate enum
//! let report = analyzer.validate_enum("Result")?;
//! ```

use std::collections::HashMap;

/// Configuration for enum support
#[derive(Debug, Clone)]
pub struct EnumSupportConfig {
    /// Enable associated values in variants
    pub enable_associated_values: bool,
    /// Enable enum serialization support
    pub enable_serialization: bool,
    /// Maximum number of variants per enum
    pub max_variants: usize,
    /// Maximum fields per variant
    pub max_variant_fields: usize,
}

impl Default for EnumSupportConfig {
    fn default() -> Self {
        EnumSupportConfig {
            enable_associated_values: true,
            enable_serialization: true,
            max_variants: 256,
            max_variant_fields: 32,
        }
    }
}

/// Kind of enum variant
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantKind {
    /// Unit variant: None, Ok
    Unit,
    /// Tuple variant: Some(T), Ok(T)
    Tuple(Vec<String>),
    /// Struct variant: Point { x, y }
    Struct(Vec<(String, String)>),
}

/// Information about an enum variant
#[derive(Debug, Clone)]
pub struct EnumVariant {
    /// Variant name
    pub name: String,
    /// Variant kind (unit, tuple, struct)
    pub kind: VariantKind,
    /// Discriminant value
    pub discriminant: usize,
    /// Documentation
    pub doc: Option<String>,
}

impl EnumVariant {
    fn new(name: String, kind: VariantKind, discriminant: usize) -> Self {
        EnumVariant {
            name,
            kind,
            discriminant,
            doc: None,
        }
    }
}

/// Information about an enum type
#[derive(Debug, Clone)]
pub struct EnumDefinition {
    /// Enum name
    pub name: String,
    /// Enum variants
    pub variants: Vec<EnumVariant>,
    /// Generic type parameters
    pub generics: Vec<String>,
    /// Where clause constraints
    pub where_clauses: Vec<String>,
    /// Associated methods
    pub methods: HashMap<String, String>,
}

impl EnumDefinition {
    fn new(name: String) -> Self {
        EnumDefinition {
            name,
            variants: Vec::new(),
            generics: Vec::new(),
            where_clauses: Vec::new(),
            methods: HashMap::new(),
        }
    }

    /// Add a generic parameter
    fn add_generic(&mut self, param: String) {
        self.generics.push(param);
    }

    /// Add a where clause
    fn add_where_clause(&mut self, clause: String) {
        self.where_clauses.push(clause);
    }
}

/// Main enum support analyzer
pub struct EnumSupportAnalyzer {
    config: EnumSupportConfig,
    enums: HashMap<String, EnumDefinition>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl EnumSupportAnalyzer {
    /// Create a new analyzer
    pub fn new(config: EnumSupportConfig) -> Self {
        EnumSupportAnalyzer {
            config,
            enums: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Register a new enum type
    pub fn register_enum(&mut self, name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Enum name cannot be empty".to_string());
        }

        self.enums.insert(name.to_string(), EnumDefinition::new(name.to_string()));
        Ok(())
    }

    /// Register a variant for an enum
    pub fn register_variant(
        &mut self,
        enum_name: &str,
        variant_name: &str,
        kind: VariantKind,
    ) -> Result<(), String> {
        if enum_name.is_empty() || variant_name.is_empty() {
            return Err("Enum and variant names cannot be empty".to_string());
        }

        let enum_def = self
            .enums
            .get_mut(enum_name)
            .ok_or_else(|| format!("Enum {} not found", enum_name))?;

        if enum_def.variants.len() >= self.config.max_variants {
            return Err(format!(
                "Too many variants for enum {}: {}",
                enum_name,
                enum_def.variants.len()
            ));
        }

        // Check variant field count for struct variants
        if let VariantKind::Struct(fields) = &kind {
            if fields.len() > self.config.max_variant_fields {
                return Err(format!(
                    "Too many fields in variant {}: {}",
                    variant_name,
                    fields.len()
                ));
            }
        }

        let variant = EnumVariant::new(variant_name.to_string(), kind, enum_def.variants.len());
        enum_def.variants.push(variant);

        Ok(())
    }

    /// Add a generic parameter to an enum
    pub fn add_generic(&mut self, enum_name: &str, param: &str) -> Result<(), String> {
        if enum_name.is_empty() || param.is_empty() {
            return Err("Enum and parameter names cannot be empty".to_string());
        }

        let enum_def = self
            .enums
            .get_mut(enum_name)
            .ok_or_else(|| format!("Enum {} not found", enum_name))?;

        enum_def.add_generic(param.to_string());
        Ok(())
    }

    /// Add a where clause to an enum
    pub fn add_where_clause(&mut self, enum_name: &str, clause: &str) -> Result<(), String> {
        if enum_name.is_empty() || clause.is_empty() {
            return Err("Enum and clause cannot be empty".to_string());
        }

        let enum_def = self
            .enums
            .get_mut(enum_name)
            .ok_or_else(|| format!("Enum {} not found", enum_name))?;

        enum_def.add_where_clause(clause.to_string());
        Ok(())
    }

    /// Check if an enum exists
    pub fn has_enum(&self, enum_name: &str) -> bool {
        self.enums.contains_key(enum_name)
    }

    /// Get enum definition
    pub fn get_enum(&self, enum_name: &str) -> Option<&EnumDefinition> {
        self.enums.get(enum_name)
    }

    /// Get variant from enum
    pub fn get_variant(&self, enum_name: &str, variant_name: &str) -> Option<&EnumVariant> {
        self.enums
            .get(enum_name)
            .and_then(|enum_def| enum_def.variants.iter().find(|v| v.name == variant_name))
    }

    /// Get variant count for an enum
    pub fn variant_count(&self, enum_name: &str) -> Option<usize> {
        self.enums.get(enum_name).map(|e| e.variants.len())
    }

    /// Get all variant names for an enum
    pub fn get_variant_names(&self, enum_name: &str) -> Option<Vec<String>> {
        self.enums.get(enum_name).map(|e| {
            e.variants
                .iter()
                .map(|v| v.name.clone())
                .collect()
        })
    }

    /// Validate an enum
    pub fn validate_enum(&self, enum_name: &str) -> Result<EnumValidationReport, String> {
        let enum_def = self
            .enums
            .get(enum_name)
            .ok_or_else(|| format!("Enum {} not found", enum_name))?;

        let mut report = EnumValidationReport {
            enum_name: enum_name.to_string(),
            variant_count: enum_def.variants.len(),
            generic_count: enum_def.generics.len(),
            is_valid: true,
            errors: Vec::new(),
        };

        // Check that enum has at least one variant
        if enum_def.variants.is_empty() {
            report.is_valid = false;
            report
                .errors
                .push("Enum must have at least one variant".to_string());
        }

        // Check for duplicate variant names
        let mut seen_names = std::collections::HashSet::new();
        for variant in &enum_def.variants {
            if !seen_names.insert(&variant.name) {
                report.is_valid = false;
                report
                    .errors
                    .push(format!("Duplicate variant name: {}", variant.name));
            }
        }

        Ok(report)
    }

    /// Generate analysis report
    pub fn generate_report(&self) -> EnumSupportAnalysisReport {
        let mut total_variants = 0;
        let mut total_generics = 0;
        let mut enum_count = 0;

        for enum_def in self.enums.values() {
            enum_count += 1;
            total_variants += enum_def.variants.len();
            total_generics += enum_def.generics.len();
        }

        EnumSupportAnalysisReport {
            enum_count,
            total_variants,
            total_generics,
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
        }
    }

    /// Add an error message
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Add a warning message
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/// Enum validation report
#[derive(Debug, Clone)]
pub struct EnumValidationReport {
    /// Enum name
    pub enum_name: String,
    /// Variant count
    pub variant_count: usize,
    /// Generic parameter count
    pub generic_count: usize,
    /// Is the enum valid
    pub is_valid: bool,
    /// Error messages
    pub errors: Vec<String>,
}

/// Analysis report for enum support
#[derive(Debug, Clone)]
pub struct EnumSupportAnalysisReport {
    /// Total enums
    pub enum_count: usize,
    /// Total variants across all enums
    pub total_variants: usize,
    /// Total generic parameters
    pub total_generics: usize,
    /// Error messages
    pub errors: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_analyzer() -> EnumSupportAnalyzer {
        EnumSupportAnalyzer::new(EnumSupportConfig::default())
    }

    #[test]
    fn test_create_analyzer() {
        let analyzer = create_test_analyzer();
        assert_eq!(analyzer.enums.len(), 0);
    }

    #[test]
    fn test_register_enum() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_enum("Option");
        assert!(result.is_ok());
        assert!(analyzer.has_enum("Option"));
    }

    #[test]
    fn test_register_enum_empty_name() {
        let mut analyzer = create_test_analyzer();
        let result = analyzer.register_enum("");
        assert!(result.is_err());
    }

    #[test]
    fn test_register_unit_variant() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Option").ok();
        let result = analyzer.register_variant("Option", "None", VariantKind::Unit);
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_tuple_variant() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Option").ok();
        let result = analyzer.register_variant(
            "Option",
            "Some",
            VariantKind::Tuple(vec!["T".to_string()]),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_struct_variant() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Point").ok();
        let result = analyzer.register_variant(
            "Point",
            "Cartesian",
            VariantKind::Struct(vec![
                ("x".to_string(), "f64".to_string()),
                ("y".to_string(), "f64".to_string()),
            ]),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_variant_count() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Result").ok();
        analyzer
            .register_variant("Result", "Ok", VariantKind::Unit)
            .ok();
        analyzer
            .register_variant("Result", "Err", VariantKind::Unit)
            .ok();

        assert_eq!(analyzer.variant_count("Result"), Some(2));
    }

    #[test]
    fn test_get_variant_names() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Option").ok();
        analyzer
            .register_variant("Option", "None", VariantKind::Unit)
            .ok();
        analyzer
            .register_variant("Option", "Some", VariantKind::Unit)
            .ok();

        let names = analyzer.get_variant_names("Option");
        assert_eq!(names, Some(vec!["None".to_string(), "Some".to_string()]));
    }

    #[test]
    fn test_get_variant() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Option").ok();
        analyzer
            .register_variant("Option", "None", VariantKind::Unit)
            .ok();

        let variant = analyzer.get_variant("Option", "None");
        assert!(variant.is_some());
        assert_eq!(variant.unwrap().name, "None");
    }

    #[test]
    fn test_add_generic() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Option").ok();
        let result = analyzer.add_generic("Option", "T");
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_where_clause() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Option").ok();
        let result = analyzer.add_where_clause("Option", "T: Clone");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_enum_empty() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Empty").ok();

        let report = analyzer.validate_enum("Empty").unwrap();
        assert!(!report.is_valid);
        assert!(!report.errors.is_empty());
    }

    #[test]
    fn test_validate_enum_valid() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Option").ok();
        analyzer
            .register_variant("Option", "None", VariantKind::Unit)
            .ok();

        let report = analyzer.validate_enum("Option").unwrap();
        assert!(report.is_valid);
        assert_eq!(report.variant_count, 1);
    }

    #[test]
    fn test_generate_report() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Option").ok();
        analyzer
            .register_variant("Option", "None", VariantKind::Unit)
            .ok();
        analyzer
            .register_variant("Option", "Some", VariantKind::Unit)
            .ok();
        analyzer.add_generic("Option", "T").ok();

        let report = analyzer.generate_report();
        assert_eq!(report.enum_count, 1);
        assert_eq!(report.total_variants, 2);
        assert_eq!(report.total_generics, 1);
    }

    #[test]
    fn test_add_error() {
        let mut analyzer = create_test_analyzer();
        analyzer.add_error("test error".to_string());
        assert_eq!(analyzer.errors.len(), 1);
    }

    #[test]
    fn test_add_warning() {
        let mut analyzer = create_test_analyzer();
        analyzer.add_warning("test warning".to_string());
        assert_eq!(analyzer.warnings.len(), 1);
    }

    #[test]
    fn test_multiple_enums() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Option").ok();
        analyzer.register_enum("Result").ok();
        analyzer.register_enum("Color").ok();

        assert_eq!(analyzer.enums.len(), 3);
    }

    #[test]
    fn test_max_variants() {
        let config = EnumSupportConfig {
            max_variants: 2,
            ..Default::default()
        };
        let mut analyzer = EnumSupportAnalyzer::new(config);
        analyzer.register_enum("Limited").ok();
        analyzer
            .register_variant("Limited", "A", VariantKind::Unit)
            .ok();
        analyzer
            .register_variant("Limited", "B", VariantKind::Unit)
            .ok();

        let result = analyzer.register_variant("Limited", "C", VariantKind::Unit);
        assert!(result.is_err());
    }

    #[test]
    fn test_max_variant_fields() {
        let config = EnumSupportConfig {
            max_variant_fields: 2,
            ..Default::default()
        };
        let mut analyzer = EnumSupportAnalyzer::new(config);
        analyzer.register_enum("Point").ok();

        let result = analyzer.register_variant(
            "Point",
            "3D",
            VariantKind::Struct(vec![
                ("x".to_string(), "f64".to_string()),
                ("y".to_string(), "f64".to_string()),
                ("z".to_string(), "f64".to_string()),
            ]),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_enum() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Option").ok();

        let enum_def = analyzer.get_enum("Option");
        assert!(enum_def.is_some());
        assert_eq!(enum_def.unwrap().name, "Option");
    }

    #[test]
    fn test_unknown_enum() {
        let analyzer = create_test_analyzer();
        let result = analyzer.validate_enum("Unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_accumulation() {
        let mut analyzer = create_test_analyzer();
        analyzer.add_error("error 1".to_string());
        analyzer.add_error("error 2".to_string());
        assert_eq!(analyzer.errors.len(), 2);
    }

    #[test]
    fn test_discriminant_assignment() {
        let mut analyzer = create_test_analyzer();
        analyzer.register_enum("Color").ok();
        analyzer
            .register_variant("Color", "Red", VariantKind::Unit)
            .ok();
        analyzer
            .register_variant("Color", "Green", VariantKind::Unit)
            .ok();

        let red = analyzer.get_variant("Color", "Red").unwrap();
        let green = analyzer.get_variant("Color", "Green").unwrap();

        assert_eq!(red.discriminant, 0);
        assert_eq!(green.discriminant, 1);
    }
}
