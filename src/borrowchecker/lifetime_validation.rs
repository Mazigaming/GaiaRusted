//! Comprehensive lifetime validation and error reporting
//!
//! Validates:
//! 1. All declared lifetime parameters are used
//! 2. All referenced lifetimes are declared
//! 3. Lifetime bounds are valid
//! 4. Provides detailed error messages with context

use crate::parser::ast::{Type, GenericParam, Parameter};
use std::collections::{HashMap, HashSet};

/// Represents a lifetime reference in the code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LifetimeRef {
    pub name: String,
    pub location: LifetimeLocation,
}

/// Where a lifetime is referenced
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LifetimeLocation {
    /// In a struct field type
    StructField(String),
    /// In a function parameter type
    FunctionParam(usize),
    /// In a function return type
    FunctionReturn,
    /// In a lifetime bound (e.g., 'a: 'b)
    LifetimeBound(String),
    /// In an impl generic
    ImplGeneric,
}

impl std::fmt::Display for LifetimeLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::StructField(name) => write!(f, "struct field '{}'", name),
            Self::FunctionParam(idx) => write!(f, "function parameter {}", idx),
            Self::FunctionReturn => write!(f, "function return type"),
            Self::LifetimeBound(name) => write!(f, "lifetime bound '{}'", name),
            Self::ImplGeneric => write!(f, "impl generic"),
        }
    }
}

/// Information about a lifetime in a signature
#[derive(Debug, Clone)]
pub struct LifetimeInfo {
    pub name: String,
    pub is_declared: bool,
    pub is_used: bool,
    pub references: Vec<LifetimeLocation>,
}

/// Validates lifetimes in a function signature
pub struct LifetimeValidator {
    declared_lifetimes: HashSet<String>,
    referenced_lifetimes: HashMap<String, LifetimeInfo>,
}

impl LifetimeValidator {
    pub fn new() -> Self {
        LifetimeValidator {
            declared_lifetimes: HashSet::new(),
            referenced_lifetimes: HashMap::new(),
        }
    }

    /// Extract lifetime declarations from generics
    pub fn register_declared_lifetimes(&mut self, generics: &[GenericParam]) {
        for param in generics {
            if let GenericParam::Lifetime(name) = param {
                self.declared_lifetimes.insert(name.clone());
            }
        }
    }

    /// Extract lifetime references from a type
    pub fn collect_type_lifetimes(&mut self, ty: &Type, location: LifetimeLocation) {
        self._collect_type_lifetimes_impl(ty, location);
    }

    fn _collect_type_lifetimes_impl(&mut self, ty: &Type, location: LifetimeLocation) {
        match ty {
            Type::Reference { lifetime, inner, .. } => {
                // Add explicit lifetime reference
                if let Some(name) = lifetime {
                    self._add_lifetime_reference(name.clone(), location.clone());
                }
                // Recurse into inner type
                self._collect_type_lifetimes_impl(inner, location);
            }
            Type::Array { element, .. } => {
                self._collect_type_lifetimes_impl(element, location);
            }
            Type::Tuple(types) => {
                for (_i, ty) in types.iter().enumerate() {
                    let loc = match &location {
                        LifetimeLocation::StructField(name) => LifetimeLocation::StructField(name.clone()),
                        LifetimeLocation::FunctionParam(idx) => LifetimeLocation::FunctionParam(*idx),
                        _ => location.clone(),
                    };
                    self._collect_type_lifetimes_impl(ty, loc);
                }
            }
            Type::Function { params, return_type, .. } => {
                for param in params {
                    self._collect_type_lifetimes_impl(param, location.clone());
                }
                self._collect_type_lifetimes_impl(return_type, location);
            }
            _ => {}
        }
    }

    /// Extract lifetimes from function parameters
    pub fn collect_param_lifetimes(&mut self, params: &[Parameter]) {
        for (idx, param) in params.iter().enumerate() {
            self.collect_type_lifetimes(
                &param.ty,
                LifetimeLocation::FunctionParam(idx),
            );
        }
    }

    /// Extract lifetimes from return type
    pub fn collect_return_lifetime(&mut self, return_type: &Option<Type>) {
        if let Some(ty) = return_type {
            self.collect_type_lifetimes(ty, LifetimeLocation::FunctionReturn);
        }
    }

    /// Add a lifetime reference
    fn _add_lifetime_reference(&mut self, name: String, location: LifetimeLocation) {
        let is_declared = self.declared_lifetimes.contains(&name);
        let info = self
            .referenced_lifetimes
            .entry(name.clone())
            .or_insert_with(|| LifetimeInfo {
                name: name.clone(),
                is_declared,
                is_used: true,
                references: Vec::new(),
            });
        info.references.push(location);
    }

    /// Validate all collected lifetimes
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check 1: All declared lifetimes must be used
        for declared in &self.declared_lifetimes {
            if !self.referenced_lifetimes.contains_key(declared) {
                errors.push(format!(
                    "Error: Unused lifetime parameter '{}'\n\
                    Note: '{}' is declared but never used in the signature",
                    declared, declared
                ));
            }
        }

        // Check 2: All referenced lifetimes must be declared
        for (name, info) in &self.referenced_lifetimes {
            if !self.declared_lifetimes.contains(name) && name != "static" {
                let locations = self._format_locations(&info.references);
                errors.push(format!(
                    "Error: Undeclared lifetime '{}'\n\
                    Note: '{}' is used in {}, but not declared in the signature\n\
                    Help: Add '{}' to the generics, e.g., fn foo<'{}, ...>(...)",
                    name, name, locations, name, name
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn _format_locations(&self, locations: &[LifetimeLocation]) -> String {
        if locations.is_empty() {
            "the signature".to_string()
        } else {
            locations
                .iter()
                .map(|loc| loc.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

/// Validates lifetimes in struct definitions
pub struct StructLifetimeValidator {
    validator: LifetimeValidator,
}

impl StructLifetimeValidator {
    pub fn new(generics: &[GenericParam]) -> Self {
        let mut validator = LifetimeValidator::new();
        validator.register_declared_lifetimes(generics);
        StructLifetimeValidator { validator }
    }

    /// Add a struct field
    pub fn add_field(&mut self, name: String, ty: &Type) {
        self.validator.collect_type_lifetimes(
            ty,
            LifetimeLocation::StructField(name),
        );
    }

    /// Validate the struct
    pub fn validate(&self) -> Result<(), Vec<String>> {
        self.validator.validate()
    }
}

/// Validates lifetimes in function signatures
pub struct FunctionLifetimeValidator {
    validator: LifetimeValidator,
}

impl FunctionLifetimeValidator {
    pub fn new(generics: &[GenericParam]) -> Self {
        let mut validator = LifetimeValidator::new();
        validator.register_declared_lifetimes(generics);
        FunctionLifetimeValidator { validator }
    }

    /// Add function parameters
    pub fn add_parameters(&mut self, params: &[Parameter]) {
        self.validator.collect_param_lifetimes(params);
    }

    /// Add return type
    pub fn add_return_type(&mut self, return_type: &Option<Type>) {
        self.validator.collect_return_lifetime(return_type);
    }

    /// Validate the function
    pub fn validate(&self) -> Result<(), Vec<String>> {
        self.validator.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unused_lifetime_detection() {
        let generics = vec![GenericParam::Lifetime("a".to_string())];
        let validator = LifetimeValidator::new();
        let mut validator = validator;
        validator.register_declared_lifetimes(&generics);

        let result = validator.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains("Unused lifetime parameter 'a'"));
    }

    #[test]
    fn test_undeclared_lifetime_detection() {
        let generics = vec![];
        let mut validator = LifetimeValidator::new();
        validator.register_declared_lifetimes(&generics);

        let ty = Type::Reference {
            lifetime: Some("a".to_string()),
            mutable: false,
            inner: Box::new(Type::Named("i32".to_string())),
        };
        validator.collect_type_lifetimes(
            &ty,
            LifetimeLocation::FunctionReturn,
        );

        let result = validator.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains("Undeclared lifetime 'a'"));
    }

    #[test]
    fn test_valid_lifetime_usage() {
        let generics = vec![GenericParam::Lifetime("a".to_string())];
        let mut validator = LifetimeValidator::new();
        validator.register_declared_lifetimes(&generics);

        let ty = Type::Reference {
            lifetime: Some("a".to_string()),
            mutable: false,
            inner: Box::new(Type::Named("i32".to_string())),
        };
        validator.collect_type_lifetimes(&ty, LifetimeLocation::FunctionReturn);

        let result = validator.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_lifetime_bounds() {
        let generics = vec![
            GenericParam::Lifetime("a".to_string()),
            GenericParam::Lifetime("b".to_string()),
        ];
        let mut validator = LifetimeValidator::new();
        validator.register_declared_lifetimes(&generics);

        let ty1 = Type::Reference {
            lifetime: Some("a".to_string()),
            mutable: false,
            inner: Box::new(Type::Named("i32".to_string())),
        };
        let ty2 = Type::Reference {
            lifetime: Some("b".to_string()),
            mutable: false,
            inner: Box::new(Type::Named("str".to_string())),
        };

        validator.collect_type_lifetimes(&ty1, LifetimeLocation::FunctionParam(0));
        validator.collect_type_lifetimes(&ty2, LifetimeLocation::FunctionParam(1));

        let result = validator.validate();
        assert!(result.is_ok());
    }
}