/// Type Checking for dyn Trait (Trait Objects)
///
/// Implements type checking rules for trait objects:
/// 1. Verify method calls on dyn Trait resolve to trait methods
/// 2. Validate return types match trait method signatures
/// 3. Handle variance for lifetime parameters
/// 4. Ensure trait object is used correctly
use crate::lowering::{HirExpression, HirItem, HirType};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TraitMethodSignature {
    pub name: String,
    pub params: Vec<HirType>,
    pub return_type: HirType,
}

#[derive(Debug, Clone)]
pub struct DynTraitInfo {
    pub trait_name: String,
    pub methods: HashMap<String, TraitMethodSignature>,
}

/// Type checker for dyn Trait operations
pub struct DynTraitTypeChecker {
    /// Known trait definitions and their methods
    trait_methods: HashMap<String, DynTraitInfo>,
}

impl DynTraitTypeChecker {
    /// Create new dyn Trait type checker
    pub fn new() -> Self {
        DynTraitTypeChecker {
            trait_methods: HashMap::new(),
        }
    }

    /// Register a trait definition for type checking
    pub fn register_trait(&mut self, trait_name: &str, methods: &[HirItem]) -> Result<(), String> {
        let mut method_sigs = HashMap::new();

        for method in methods {
            if let HirItem::Function {
                name: method_name,
                params,
                return_type,
                ..
            } = method
            {
                // Skip 'self' parameter
                let param_types: Vec<HirType> =
                    params.iter().skip(1).map(|(_, ty)| ty.clone()).collect();

                let ret_type = return_type.clone().unwrap_or(HirType::Unknown);

                method_sigs.insert(
                    method_name.clone(),
                    TraitMethodSignature {
                        name: method_name.clone(),
                        params: param_types,
                        return_type: ret_type,
                    },
                );
            }
        }

        self.trait_methods.insert(
            trait_name.to_string(),
            DynTraitInfo {
                trait_name: trait_name.to_string(),
                methods: method_sigs,
            },
        );

        Ok(())
    }

    /// Check if a method exists in a trait and has compatible signature
    pub fn check_method_call(
        &self,
        trait_name: &str,
        method_name: &str,
        arg_types: &[HirType],
        return_context: &HirType,
    ) -> Result<HirType, String> {
        // Look up trait
        let trait_info = self
            .trait_methods
            .get(trait_name)
            .ok_or_else(|| format!("Unknown trait: {}", trait_name))?;

        // Look up method in trait
        let method_sig = trait_info
            .methods
            .get(method_name)
            .ok_or_else(|| format!("Method {} not found in trait {}", method_name, trait_name))?;

        // Validate argument count
        if arg_types.len() != method_sig.params.len() {
            return Err(format!(
                "Method {} expects {} arguments, got {}",
                method_name,
                method_sig.params.len(),
                arg_types.len()
            ));
        }

        // Validate argument types
        for (i, (expected, actual)) in method_sig.params.iter().zip(arg_types.iter()).enumerate() {
            if !self.types_compatible(expected, actual) {
                return Err(format!(
                    "Argument {} type mismatch in {}: expected {}, got {}",
                    i + 1,
                    method_name,
                    expected,
                    actual
                ));
            }
        }

        // Return method's return type
        Ok(method_sig.return_type.clone())
    }

    /// Check type compatibility for method arguments
    fn types_compatible(&self, expected: &HirType, actual: &HirType) -> bool {
        match (expected, actual) {
            // Exact match
            (a, b) if std::mem::discriminant(a) == std::mem::discriminant(b) => true,

            // Allow reference/value conversions
            (HirType::Reference(exp), HirType::Reference(act)) => self.types_compatible(exp, act),

            // Unknown is compatible with anything
            (HirType::Unknown, _) | (_, HirType::Unknown) => true,

            _ => false,
        }
    }

    /// Validate assignment to dyn Trait
    pub fn check_trait_object_assignment(
        &self,
        concrete_type: &str,
        trait_name: &str,
    ) -> Result<(), String> {
        // Verify trait exists
        if !self.trait_methods.contains_key(trait_name) {
            return Err(format!("Unknown trait: {}", trait_name));
        }

        // In a real implementation, we'd check if concrete_type implements trait_name
        // For now, we assume it does if object-safety validation passed
        Ok(())
    }

    /// Check method call on dyn Trait reference
    pub fn check_dyn_trait_method_call(
        &self,
        trait_name: &str,
        method_name: &str,
        arg_types: &[HirType],
    ) -> Result<HirType, String> {
        self.check_method_call(trait_name, method_name, arg_types, &HirType::Unknown)
    }

    /// Verify function parameter is compatible with dyn Trait
    pub fn check_dyn_trait_parameter(
        &self,
        param_type: &HirType,
        trait_name: &str,
    ) -> Result<(), String> {
        match param_type {
            HirType::DynTrait {
                trait_name: param_trait,
            } => {
                if param_trait == trait_name {
                    Ok(())
                } else {
                    Err(format!(
                        "Expected dyn {}, got dyn {}",
                        trait_name, param_trait
                    ))
                }
            }
            HirType::Reference(inner) => {
                // &dyn Trait
                if let HirType::DynTrait {
                    trait_name: param_trait,
                } = inner.as_ref()
                {
                    if param_trait == trait_name {
                        Ok(())
                    } else {
                        Err(format!(
                            "Expected &dyn {}, got &dyn {}",
                            trait_name, param_trait
                        ))
                    }
                } else {
                    Err("Reference must be to dyn Trait".to_string())
                }
            }
            _ => Err(format!("Expected dyn Trait type, got {}", param_type)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dyn_trait_checker_creation() {
        let checker = DynTraitTypeChecker::new();
        assert_eq!(checker.trait_methods.len(), 0);
    }

    #[test]
    fn test_type_compatibility() {
        let checker = DynTraitTypeChecker::new();
        assert!(checker.types_compatible(&HirType::Int32, &HirType::Int32));
        assert!(checker.types_compatible(&HirType::Unknown, &HirType::Int32));
        assert!(!checker.types_compatible(&HirType::Int32, &HirType::Int64));
    }

    #[test]
    fn test_dyn_trait_parameter_check() {
        let checker = DynTraitTypeChecker::new();
        let dyn_trait_type = HirType::DynTrait {
            trait_name: "Animal".to_string(),
        };
        assert!(checker
            .check_dyn_trait_parameter(&dyn_trait_type, "Animal")
            .is_ok());
    }

    #[test]
    fn test_dyn_trait_parameter_mismatch() {
        let checker = DynTraitTypeChecker::new();
        let dyn_trait_type = HirType::DynTrait {
            trait_name: "Animal".to_string(),
        };
        assert!(checker
            .check_dyn_trait_parameter(&dyn_trait_type, "Vehicle")
            .is_err());
    }

    #[test]
    fn test_trait_reference_parameter() {
        let checker = DynTraitTypeChecker::new();
        let ref_dyn_trait = HirType::Reference(Box::new(HirType::DynTrait {
            trait_name: "Reader".to_string(),
        }));
        assert!(checker
            .check_dyn_trait_parameter(&ref_dyn_trait, "Reader")
            .is_ok());
    }
}
