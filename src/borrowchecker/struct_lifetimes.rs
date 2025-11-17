//! Struct field lifetime validation
//!
//! Handles lifetime parameters in struct definitions and validates
//! that field lifetimes are properly constrained by struct lifetimes.

use crate::parser::ast::{Type, GenericParam, StructField};
use super::lifetimes::{Lifetime, LifetimeContext};
use super::lifetime_validation::StructLifetimeValidator;

/// Information about a struct's lifetime parameters and field constraints
#[derive(Debug, Clone)]
pub struct StructLifetimes {
    /// Named lifetime parameters: 'a, 'b, etc.
    pub lifetime_params: Vec<String>,
    /// Lifetime for each field (by index)
    pub field_lifetimes: Vec<Option<Lifetime>>,
}

impl StructLifetimes {
    /// Create new struct lifetime info
    pub fn new() -> Self {
        StructLifetimes {
            lifetime_params: Vec::new(),
            field_lifetimes: Vec::new(),
        }
    }
}

/// Extract lifetime information from a struct signature
pub fn extract_struct_lifetimes(
    lifetime_ctx: &mut LifetimeContext,
    generics: &[GenericParam],
    fields: &[StructField],
) -> StructLifetimes {
    let mut info = StructLifetimes::new();

    info.lifetime_params = extract_lifetime_params(generics);

    for param_lifetime in &info.lifetime_params {
        lifetime_ctx.register_named_lifetime(param_lifetime.clone());
    }

    let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let field_lifetimes = extract_field_lifetimes(lifetime_ctx, &field_types);
    info.field_lifetimes = field_lifetimes;

    info
}

/// Extract lifetime parameters from generics
fn extract_lifetime_params(generics: &[GenericParam]) -> Vec<String> {
    let mut lifetimes = Vec::new();
    for param in generics {
        if let GenericParam::Lifetime(name) = param {
            lifetimes.push(name.clone());
        }
    }
    lifetimes
}

/// Extract lifetime from a type
fn extract_lifetime_from_type(lifetime_ctx: &mut LifetimeContext, ty: &Type) -> Option<Lifetime> {
    match ty {
        Type::Reference { lifetime, .. } => {
            if let Some(lt) = lifetime {
                Some(lifetime_ctx.register_named_lifetime(lt.clone()))
            } else {
                Some(lifetime_ctx.fresh_lifetime())
            }
        }
        Type::Generic { type_args, .. } => {
            for arg in type_args {
                if let Some(lt) = extract_lifetime_from_type(lifetime_ctx, arg) {
                    return Some(lt);
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract lifetimes from field types
fn extract_field_lifetimes(
    lifetime_ctx: &mut LifetimeContext,
    field_types: &[&Type],
) -> Vec<Option<Lifetime>> {
    field_types
        .iter()
        .map(|ty| extract_lifetime_from_type(lifetime_ctx, ty))
        .collect()
}

/// Generate constraints from struct signature
pub fn generate_struct_constraints(
    lifetime_ctx: &mut LifetimeContext,
    struct_lifetime_params: &[String],
    field_lifetimes: &[Option<Lifetime>],
) {
    for field_lt in field_lifetimes {
        if let Some(field_lt) = field_lt {
            for struct_param in struct_lifetime_params {
                let struct_lt = Lifetime::Named(struct_param.clone());
                lifetime_ctx.add_constraint(
                    struct_lt,
                    field_lt.clone(),
                    format!("struct lifetime '{}' must outlive field lifetime", struct_param),
                );
            }
        }
    }
}

/// Validate that a struct's lifetimes are used correctly
pub fn validate_struct_lifetimes(
    info: &StructLifetimes,
    named_lifetime_params: &[String],
) -> Result<(), String> {
    let mut used_lifetime_params: std::collections::HashSet<String> = std::collections::HashSet::new();

    for opt_lt in &info.field_lifetimes {
        if let Some(Lifetime::Named(name)) = opt_lt {
            used_lifetime_params.insert(name.clone());
        }
    }

    for param in named_lifetime_params {
        if !used_lifetime_params.contains(param) {
            return Err(format!("Unused lifetime parameter in struct: '{}", param));
        }
    }

    Ok(())
}

/// Enhanced validation with detailed error messages
/// Validates:
/// 1. All declared lifetimes are used
/// 2. All referenced lifetimes are declared
/// 3. Provides helpful error context
pub fn validate_struct_lifetimes_detailed(
    generics: &[GenericParam],
    fields: &[StructField],
) -> Result<(), Vec<String>> {
    let mut validator = StructLifetimeValidator::new(generics);
    
    // Collect field lifetimes
    for (_i, field) in fields.iter().enumerate() {
        validator.add_field(field.name.clone(), &field.ty);
    }
    
    // Perform validation
    validator.validate()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_lifetime_params() {
        let generics = vec![
            GenericParam::Lifetime("a".to_string()),
            GenericParam::Lifetime("b".to_string()),
        ];
        let lifetimes = extract_lifetime_params(&generics);
        assert_eq!(lifetimes, vec!["a", "b"]);
    }

    #[test]
    fn test_extract_lifetime_from_reference() {
        let mut ctx = LifetimeContext::new();
        let ty = Type::Reference {
            lifetime: Some("a".to_string()),
            mutable: false,
            inner: Box::new(Type::Named("i32".to_string())),
        };

        let lt = extract_lifetime_from_type(&mut ctx, &ty);
        assert!(lt.is_some());
        assert_eq!(lt.unwrap(), Lifetime::Named("a".to_string()));
    }

    #[test]
    fn test_extract_lifetime_inferred() {
        let mut ctx = LifetimeContext::new();
        let ty = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::Named("i32".to_string())),
        };

        let lt = extract_lifetime_from_type(&mut ctx, &ty);
        assert!(lt.is_some());
        match lt.unwrap() {
            Lifetime::Inferred(_) => {}
            _ => panic!("Expected inferred lifetime"),
        }
    }

    #[test]
    fn test_validate_all_field_lifetime_params_used() {
        let info = StructLifetimes {
            lifetime_params: vec!["a".to_string()],
            field_lifetimes: vec![Some(Lifetime::Named("a".to_string()))],
        };

        let result = validate_struct_lifetimes(&info, &["a".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_unused_field_lifetime_param() {
        let info = StructLifetimes {
            lifetime_params: vec!["a".to_string()],
            field_lifetimes: vec![],
        };

        let result = validate_struct_lifetimes(&info, &["a".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unused lifetime"));
    }

    #[test]
    fn test_struct_constraints_generation() {
        let mut ctx = LifetimeContext::new();
        let field_lt = Some(Lifetime::Named("a".to_string()));

        generate_struct_constraints(&mut ctx, &["b".to_string()], &[field_lt]);

        let constraints = ctx.constraints();
        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].lhs, Lifetime::Named("b".to_string()));
        assert_eq!(constraints[0].rhs, Lifetime::Named("a".to_string()));
    }

    #[test]
    fn test_validate_struct_lifetimes_valid() {
        let generics = vec![GenericParam::Lifetime("a".to_string())];
        let fields = vec![StructField {
            name: "data".to_string(),
            ty: Type::Reference {
                lifetime: Some("a".to_string()),
                mutable: false,
                inner: Box::new(Type::Named("i32".to_string())),
            },
            attributes: vec![],
        }];

        let result = validate_struct_lifetimes_detailed(&generics, &fields);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_struct_lifetimes_unused() {
        let generics = vec![GenericParam::Lifetime("a".to_string())];
        let fields = vec![]; // No fields using 'a

        let result = validate_struct_lifetimes_detailed(&generics, &fields);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains("Unused lifetime parameter 'a'"));
    }

    #[test]
    fn test_validate_struct_lifetimes_undeclared() {
        let generics = vec![]; // No lifetime declarations
        let fields = vec![StructField {
            name: "data".to_string(),
            ty: Type::Reference {
                lifetime: Some("a".to_string()), // Uses undeclared 'a
                mutable: false,
                inner: Box::new(Type::Named("i32".to_string())),
            },
            attributes: vec![],
        }];

        let result = validate_struct_lifetimes_detailed(&generics, &fields);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains("Undeclared lifetime 'a'"));
    }

    #[test]
    fn test_validate_struct_lifetimes_multiple_fields() {
        let generics = vec![
            GenericParam::Lifetime("a".to_string()),
            GenericParam::Lifetime("b".to_string()),
        ];
        let fields = vec![
            StructField {
                name: "first".to_string(),
                ty: Type::Reference {
                    lifetime: Some("a".to_string()),
                    mutable: false,
                    inner: Box::new(Type::Named("i32".to_string())),
                },
                attributes: vec![],
            },
            StructField {
                name: "second".to_string(),
                ty: Type::Reference {
                    lifetime: Some("b".to_string()),
                    mutable: false,
                    inner: Box::new(Type::Named("str".to_string())),
                },
                attributes: vec![],
            },
        ];

        let result = validate_struct_lifetimes_detailed(&generics, &fields);
        assert!(result.is_ok());
    }
}
