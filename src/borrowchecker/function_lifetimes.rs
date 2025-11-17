//! Function lifetime constraint generation
//!
//! Handles lifetime parameter extraction from function signatures
//! and generates constraints that ensure type safety.

use crate::parser::ast::{Type, GenericParam, Parameter};
use super::lifetimes::{Lifetime, LifetimeContext, LifetimeElision};
use super::lifetime_validation::FunctionLifetimeValidator;

/// Information about a function's lifetime parameters and constraints
#[derive(Debug, Clone)]
pub struct FunctionLifetimes {
    /// Named lifetime parameters: 'a, 'b, etc.
    pub lifetime_params: Vec<String>,
    /// Lifetime for each parameter (by index)
    pub param_lifetimes: Vec<Option<Lifetime>>,
    /// Lifetime of return type
    pub return_lifetime: Option<Lifetime>,
}

impl FunctionLifetimes {
    /// Create new function lifetime info
    pub fn new() -> Self {
        FunctionLifetimes {
            lifetime_params: Vec::new(),
            param_lifetimes: Vec::new(),
            return_lifetime: None,
        }
    }
}

/// Extract lifetime information from a function signature
pub fn extract_function_lifetimes(
    lifetime_ctx: &mut LifetimeContext,
    generics: &[GenericParam],
    params: &[Parameter],
    return_type: &Option<Type>,
) -> FunctionLifetimes {
    let mut info = FunctionLifetimes::new();

    info.lifetime_params = extract_lifetime_params(generics);

    for param_lifetime in &info.lifetime_params {
        lifetime_ctx.register_named_lifetime(param_lifetime.clone());
    }

    let param_types: Vec<_> = params.iter().map(|p| &p.ty).collect();
    let param_lifetimes = extract_param_lifetimes(lifetime_ctx, &param_types);
    info.param_lifetimes = param_lifetimes;

    let return_lifetime = extract_return_lifetime(lifetime_ctx, &param_types, return_type);
    info.return_lifetime = return_lifetime;

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

/// Extract lifetime from a reference type
fn extract_lifetime_from_type(lifetime_ctx: &mut LifetimeContext, ty: &Type) -> Option<Lifetime> {
    match ty {
        Type::Reference { lifetime, .. } => {
            if let Some(lt) = lifetime {
                Some(lifetime_ctx.register_named_lifetime(lt.clone()))
            } else {
                Some(lifetime_ctx.fresh_lifetime())
            }
        }
        _ => None,
    }
}

/// Extract lifetimes from parameter types
fn extract_param_lifetimes(
    lifetime_ctx: &mut LifetimeContext,
    param_types: &[&Type],
) -> Vec<Option<Lifetime>> {
    param_types
        .iter()
        .map(|ty| extract_lifetime_from_type(lifetime_ctx, ty))
        .collect()
}

/// Extract return type lifetime
fn extract_return_lifetime(
    lifetime_ctx: &mut LifetimeContext,
    param_types: &[&Type],
    return_type: &Option<Type>,
) -> Option<Lifetime> {
    match return_type {
        Some(ty) => {
            match ty {
                Type::Reference { lifetime, .. } => {
                    if let Some(lt) = lifetime {
                        Some(lifetime_ctx.register_named_lifetime(lt.clone()))
                    } else {
                        let input_refs: Vec<bool> = param_types.iter()
                            .map(|t| matches!(t, Type::Reference { .. }))
                            .collect();
                        let (_, return_lt) = LifetimeElision::elide_function_lifetimes(
                            input_refs,
                            true,
                            lifetime_ctx,
                        );
                        return_lt
                    }
                }
                _ => None,
            }
        }
        None => None,
    }
}

/// Generate constraints from function signature
pub fn generate_function_constraints(
    lifetime_ctx: &mut LifetimeContext,
    param_lifetimes: &[Option<Lifetime>],
    return_lifetime: &Option<Lifetime>,
) {
    if let Some(ret_lt) = return_lifetime {
        for (i, param_lt) in param_lifetimes.iter().enumerate() {
            if let Some(param_lt) = param_lt {
                lifetime_ctx.add_constraint(
                    param_lt.clone(),
                    ret_lt.clone(),
                    format!("parameter {} lifetime must outlive return type", i),
                );
            }
        }
    }
}

/// Validate that a function's lifetimes are used correctly
pub fn validate_function_lifetimes(
    info: &FunctionLifetimes,
    named_lifetime_params: &[String],
) -> Result<(), String> {
    let mut used_lifetime_params: std::collections::HashSet<String> = std::collections::HashSet::new();

    for opt_lt in &info.param_lifetimes {
        if let Some(Lifetime::Named(name)) = opt_lt {
            used_lifetime_params.insert(name.clone());
        }
    }

    if let Some(Lifetime::Named(name)) = &info.return_lifetime {
        used_lifetime_params.insert(name.clone());
    }

    for param in named_lifetime_params {
        if !used_lifetime_params.contains(param) {
            return Err(format!("Unused lifetime parameter: '{}", param));
        }
    }

    Ok(())
}

/// Enhanced validation with detailed error messages
/// Validates:
/// 1. All declared lifetimes are used
/// 2. All referenced lifetimes are declared
/// 3. Provides helpful error context
pub fn validate_function_lifetimes_detailed(
    generics: &[GenericParam],
    params: &[Parameter],
    return_type: &Option<Type>,
) -> Result<(), Vec<String>> {
    let mut validator = FunctionLifetimeValidator::new(generics);
    
    // Collect parameter lifetimes
    validator.add_parameters(params);
    
    // Collect return type lifetimes
    validator.add_return_type(return_type);
    
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
    fn test_validate_all_lifetime_params_used() {
        let info = FunctionLifetimes {
            lifetime_params: vec!["a".to_string()],
            param_lifetimes: vec![Some(Lifetime::Named("a".to_string()))],
            return_lifetime: None,
        };

        let result = validate_function_lifetimes(&info, &["a".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_unused_lifetime_param() {
        let info = FunctionLifetimes {
            lifetime_params: vec!["a".to_string()],
            param_lifetimes: vec![],
            return_lifetime: None,
        };

        let result = validate_function_lifetimes(&info, &["a".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unused lifetime"));
    }

    #[test]
    fn test_function_constraints_generation() {
        let mut ctx = LifetimeContext::new();
        let param_lt = Some(Lifetime::Named("a".to_string()));
        let return_lt = Some(Lifetime::Named("b".to_string()));

        generate_function_constraints(&mut ctx, &[param_lt], &return_lt);

        let constraints = ctx.constraints();
        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].lhs, Lifetime::Named("a".to_string()));
        assert_eq!(constraints[0].rhs, Lifetime::Named("b".to_string()));
    }

    #[test]
    fn test_validate_function_lifetimes_valid_signature() {
        let generics = vec![GenericParam::Lifetime("a".to_string())];
        let params = vec![Parameter {
            name: "x".to_string(),
            mutable: false,
            ty: Type::Reference {
                lifetime: Some("a".to_string()),
                mutable: false,
                inner: Box::new(Type::Named("i32".to_string())),
            },
        }];
        let return_type = None;

        let result = validate_function_lifetimes_detailed(&generics, &params, &return_type);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_function_lifetimes_unused() {
        let generics = vec![GenericParam::Lifetime("a".to_string())];
        let params = vec![]; // No parameters using 'a
        let return_type = None;

        let result = validate_function_lifetimes_detailed(&generics, &params, &return_type);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains("Unused lifetime parameter 'a'"));
    }

    #[test]
    fn test_validate_function_lifetimes_undeclared() {
        let generics = vec![]; // No lifetime declarations
        let params = vec![Parameter {
            name: "x".to_string(),
            mutable: false,
            ty: Type::Reference {
                lifetime: Some("a".to_string()), // Uses undeclared 'a
                mutable: false,
                inner: Box::new(Type::Named("i32".to_string())),
            },
        }];
        let return_type = None;

        let result = validate_function_lifetimes_detailed(&generics, &params, &return_type);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains("Undeclared lifetime 'a'"));
    }

    #[test]
    fn test_validate_function_lifetimes_return_type() {
        let generics = vec![GenericParam::Lifetime("a".to_string())];
        let params = vec![];
        let return_type = Some(Type::Reference {
            lifetime: Some("a".to_string()),
            mutable: false,
            inner: Box::new(Type::Named("i32".to_string())),
        });

        let result = validate_function_lifetimes_detailed(&generics, &params, &return_type);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_function_lifetimes_multiple() {
        let generics = vec![
            GenericParam::Lifetime("a".to_string()),
            GenericParam::Lifetime("b".to_string()),
        ];
        let params = vec![
            Parameter {
                name: "x".to_string(),
                mutable: false,
                ty: Type::Reference {
                    lifetime: Some("a".to_string()),
                    mutable: false,
                    inner: Box::new(Type::Named("i32".to_string())),
                },
            },
            Parameter {
                name: "y".to_string(),
                mutable: false,
                ty: Type::Reference {
                    lifetime: Some("b".to_string()),
                    mutable: false,
                    inner: Box::new(Type::Named("str".to_string())),
                },
            },
        ];
        let return_type = None;

        let result = validate_function_lifetimes_detailed(&generics, &params, &return_type);
        assert!(result.is_ok());
    }
}
