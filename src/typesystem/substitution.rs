//! # Substitution System
//!
//! Manages type variable bindings during unification.
//! A substitution is a mapping from type variables to concrete types.
//!
//! Key operations:
//! - `apply`: Replace all type variables in a type using the substitution
//! - `bind`: Add a new binding (with occurs check to prevent infinite types)
//! - `occurs_check`: Ensure we don't create circular types like X = Box<X>
//! - `compose`: Merge two substitutions

use super::types::{Type, TypeVar};
use std::collections::HashMap;

/// A substitution maps type variables to types
#[derive(Debug, Clone)]
pub struct Substitution {
    /// Maps TypeVar to what it should be replaced with
    mapping: HashMap<TypeVar, Type>,
}

impl Substitution {
    /// Create an empty substitution
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
        }
    }

    /// Get the raw mapping (for testing and debugging)
    pub fn mapping(&self) -> &HashMap<TypeVar, Type> {
        &self.mapping
    }

    /// Apply substitution to a type
    /// Recursively replaces all type variables with their bindings
    pub fn apply(&self, ty: &Type) -> Type {
        match ty {
            // Check if variable is bound
            Type::Variable(var) => {
                if let Some(bound_ty) = self.mapping.get(var) {
                    // Follow the chain (in case bound type contains variables)
                    self.apply(bound_ty)
                } else {
                    // Variable is unbound
                    Type::Variable(*var)
                }
            }

            // Recursively apply to tuple elements
            Type::Tuple(tys) => {
                let applied: Vec<Type> = tys.iter().map(|t| self.apply(t)).collect();
                Type::Tuple(applied)
            }

            // Recursively apply to array element type
            Type::Array { element, size } => {
                Type::Array {
                    element: Box::new(self.apply(element)),
                    size: *size,
                }
            }

            // Recursively apply to reference inner type
            Type::Reference {
                lifetime,
                mutable,
                inner,
            } => {
                Type::Reference {
                    lifetime: *lifetime,
                    mutable: *mutable,
                    inner: Box::new(self.apply(inner)),
                }
            }

            // Recursively apply to raw pointer inner type
            Type::RawPointer { mutable, inner } => {
                Type::RawPointer {
                    mutable: *mutable,
                    inner: Box::new(self.apply(inner)),
                }
            }

            // Recursively apply to function parameter and return types
            Type::Function { params, ret } => {
                Type::Function {
                    params: params.iter().map(|t| self.apply(t)).collect(),
                    ret: Box::new(self.apply(ret)),
                }
            }

            // Other types are left unchanged
            other => other.clone(),
        }
    }

    /// Bind a type variable to a type
    ///
    /// # Arguments
    /// * `var`: The type variable to bind
    /// * `ty`: The type to bind it to
    ///
    /// # Returns
    /// Ok(()) on success
    /// Err(message) if occurs check fails
    ///
    /// # Example
    /// ```ignore
    /// let mut subst = Substitution::new();
    /// subst.bind(TypeVar(0), Type::I32)?;  // X = i32
    /// ```
    pub fn bind(&mut self, var: TypeVar, ty: Type) -> Result<(), String> {
        // Prevent infinite types like X = Box<X>
        if self.occurs_check(&var, &ty) {
            return Err(format!(
                "Infinite type: type variable {} occurs in {}",
                var.0, ty
            ));
        }

        self.mapping.insert(var, ty);
        Ok(())
    }

    /// Occurs check: prevent X = Box<X>
    ///
    /// Returns true if the variable appears (possibly nested) in the type
    fn occurs_check(&self, var: &TypeVar, ty: &Type) -> bool {
        let applied = self.apply(ty);

        match &applied {
            Type::Variable(v) => v == var,

            Type::Tuple(tys) => tys.iter().any(|t| self.occurs_check(var, t)),

            Type::Array { element, .. } => self.occurs_check(var, element),

            Type::Reference { inner, .. } => self.occurs_check(var, inner),

            Type::RawPointer { inner, .. } => self.occurs_check(var, inner),

            Type::Function { params, ret } => {
                params.iter().any(|t| self.occurs_check(var, t))
                    || self.occurs_check(var, ret)
            }

            _ => false,
        }
    }

    /// Compose two substitutions: apply `other` to all bindings in `self`
    ///
    /// This is useful when you want to combine the results of multiple unification steps.
    ///
    /// # Example
    /// ```ignore
    /// let mut subst1 = Substitution::new();
    /// subst1.bind(TypeVar(0), Type::I32);  // X = i32
    ///
    /// let mut subst2 = Substitution::new();
    /// subst2.bind(TypeVar(1), Type::Variable(TypeVar(0)));  // Y = X
    ///
    /// subst1.compose(&subst2);
    /// // Now: X = i32, Y = X becomes X = i32, Y = i32
    /// ```
    pub fn compose(&mut self, other: &Substitution) {
        // Apply other's substitution to all our bindings
        for (_var, ty) in self.mapping.iter_mut() {
            *ty = other.apply(ty);
        }

        // Add any new bindings from other
        for (var, ty) in other.mapping.iter() {
            if !self.mapping.contains_key(var) {
                self.mapping.insert(*var, ty.clone());
            }
        }
    }

    /// Get the binding for a specific type variable
    pub fn get(&self, var: TypeVar) -> Option<Type> {
        self.mapping.get(&var).cloned()
    }

    /// Check if a type variable is bound
    pub fn is_bound(&self, var: TypeVar) -> bool {
        self.mapping.contains_key(&var)
    }

    /// Get the number of bindings
    pub fn len(&self) -> usize {
        self.mapping.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.mapping.is_empty()
    }

    /// Clear all bindings
    pub fn clear(&mut self) {
        self.mapping.clear();
    }
}

impl Default for Substitution {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_substitution() {
        let subst = Substitution::new();
        assert!(subst.is_empty());
        assert_eq!(subst.len(), 0);
    }

    #[test]
    fn test_simple_bind() {
        let mut subst = Substitution::new();
        let var = TypeVar(0);

        assert!(subst.bind(var, Type::I32).is_ok());
        assert!(subst.is_bound(var));
        assert_eq!(subst.len(), 1);
    }

    #[test]
    fn test_apply_unbound_variable() {
        let subst = Substitution::new();
        let var = Type::Variable(TypeVar(0));

        let result = subst.apply(&var);
        assert_eq!(result, Type::Variable(TypeVar(0)));
    }

    #[test]
    fn test_apply_bound_variable() {
        let mut subst = Substitution::new();
        subst.bind(TypeVar(0), Type::I32).unwrap();

        let result = subst.apply(&Type::Variable(TypeVar(0)));
        assert_eq!(result, Type::I32);
    }

    #[test]
    fn test_apply_to_tuple() {
        let mut subst = Substitution::new();
        subst.bind(TypeVar(0), Type::I32).unwrap();
        subst.bind(TypeVar(1), Type::Bool).unwrap();

        let tuple = Type::Tuple(vec![
            Type::Variable(TypeVar(0)),
            Type::Variable(TypeVar(1)),
        ]);

        let result = subst.apply(&tuple);
        assert_eq!(result, Type::Tuple(vec![Type::I32, Type::Bool]));
    }

    #[test]
    fn test_apply_to_array() {
        let mut subst = Substitution::new();
        subst.bind(TypeVar(0), Type::I32).unwrap();

        let array = Type::Array {
            element: Box::new(Type::Variable(TypeVar(0))),
            size: 10,
        };

        let result = subst.apply(&array);
        assert_eq!(
            result,
            Type::Array {
                element: Box::new(Type::I32),
                size: 10
            }
        );
    }

    #[test]
    fn test_apply_to_reference() {
        let mut subst = Substitution::new();
        subst.bind(TypeVar(0), Type::I32).unwrap();

        let reference = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::Variable(TypeVar(0))),
        };

        let result = subst.apply(&reference);
        assert_eq!(
            result,
            Type::Reference {
                lifetime: None,
                mutable: false,
                inner: Box::new(Type::I32),
            }
        );
    }

    #[test]
    fn test_apply_to_function_type() {
        let mut subst = Substitution::new();
        subst.bind(TypeVar(0), Type::I32).unwrap();
        subst.bind(TypeVar(1), Type::Bool).unwrap();

        let func = Type::Function {
            params: vec![Type::Variable(TypeVar(0)), Type::Variable(TypeVar(1))],
            ret: Box::new(Type::Str),
        };

        let result = subst.apply(&func);
        assert_eq!(
            result,
            Type::Function {
                params: vec![Type::I32, Type::Bool],
                ret: Box::new(Type::Str),
            }
        );
    }

    #[test]
    fn test_occurs_check_simple() {
        let mut subst = Substitution::new();
        let var = TypeVar(0);

        // Try to bind X = Box<X> (infinite type)
        let infinite = Type::Array {
            element: Box::new(Type::Variable(var)),
            size: 1,
        };

        let result = subst.bind(var, infinite);
        assert!(result.is_err());
    }

    #[test]
    fn test_occurs_check_nested() {
        let mut subst = Substitution::new();
        let var = TypeVar(0);

        // Try to bind X = (i32, X) (infinite type)
        let infinite = Type::Tuple(vec![Type::I32, Type::Variable(var)]);

        let result = subst.bind(var, infinite);
        assert!(result.is_err());
    }

    #[test]
    fn test_occurs_check_deep_nesting() {
        let mut subst = Substitution::new();
        let var = TypeVar(0);

        // Try to bind X = &(i32, X) (infinite type)
        let infinite = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::Tuple(vec![Type::I32, Type::Variable(var)])),
        };

        let result = subst.bind(var, infinite);
        assert!(result.is_err());
    }

    #[test]
    fn test_compose_simple() {
        let mut subst1 = Substitution::new();
        subst1.bind(TypeVar(0), Type::I32).unwrap();

        let mut subst2 = Substitution::new();
        subst2.bind(TypeVar(1), Type::Variable(TypeVar(0))).unwrap();

        subst1.compose(&subst2);

        // After composition:
        // X = i32 stays the same
        // Y = X should become Y = i32 (but Y is now in subst1 mapping)
        assert_eq!(subst1.apply(&Type::Variable(TypeVar(0))), Type::I32);
        assert_eq!(subst1.apply(&Type::Variable(TypeVar(1))), Type::I32);
    }

    #[test]
    fn test_chain_variable_binding() {
        let mut subst = Substitution::new();
        subst.bind(TypeVar(0), Type::I32).unwrap();
        subst.bind(TypeVar(1), Type::Variable(TypeVar(0))).unwrap();

        // Y is bound to X, which is bound to i32
        // Apply should follow the chain
        let result = subst.apply(&Type::Variable(TypeVar(1)));
        assert_eq!(result, Type::I32);
    }

    #[test]
    fn test_apply_preserves_unbound() {
        let mut subst = Substitution::new();
        subst.bind(TypeVar(0), Type::I32).unwrap();

        // Apply should not affect unbound variables
        let result = subst.apply(&Type::Variable(TypeVar(99)));
        assert_eq!(result, Type::Variable(TypeVar(99)));
    }

    #[test]
    fn test_get_binding() {
        let mut subst = Substitution::new();
        subst.bind(TypeVar(0), Type::I32).unwrap();

        assert_eq!(subst.get(TypeVar(0)), Some(Type::I32));
        assert_eq!(subst.get(TypeVar(1)), None);
    }

    #[test]
    fn test_multiple_bindings() {
        let mut subst = Substitution::new();
        subst.bind(TypeVar(0), Type::I32).unwrap();
        subst.bind(TypeVar(1), Type::Bool).unwrap();
        subst.bind(TypeVar(2), Type::F64).unwrap();

        assert_eq!(subst.len(), 3);
        assert_eq!(subst.apply(&Type::Variable(TypeVar(0))), Type::I32);
        assert_eq!(subst.apply(&Type::Variable(TypeVar(1))), Type::Bool);
        assert_eq!(subst.apply(&Type::Variable(TypeVar(2))), Type::F64);
    }

    #[test]
    fn test_clear_substitution() {
        let mut subst = Substitution::new();
        subst.bind(TypeVar(0), Type::I32).unwrap();
        assert_eq!(subst.len(), 1);

        subst.clear();
        assert!(subst.is_empty());
        assert_eq!(subst.len(), 0);
    }

    #[test]
    fn test_apply_raw_pointer() {
        let mut subst = Substitution::new();
        subst.bind(TypeVar(0), Type::I32).unwrap();

        let ptr = Type::RawPointer {
            mutable: true,
            inner: Box::new(Type::Variable(TypeVar(0))),
        };

        let result = subst.apply(&ptr);
        assert_eq!(
            result,
            Type::RawPointer {
                mutable: true,
                inner: Box::new(Type::I32),
            }
        );
    }

    #[test]
    fn test_nested_complex_type() {
        let mut subst = Substitution::new();
        subst.bind(TypeVar(0), Type::I32).unwrap();
        subst.bind(TypeVar(1), Type::Bool).unwrap();

        let complex = Type::Array {
            element: Box::new(Type::Reference {
                lifetime: None,
                mutable: true,
                inner: Box::new(Type::Tuple(vec![
                    Type::Variable(TypeVar(0)),
                    Type::Variable(TypeVar(1)),
                ])),
            }),
            size: 5,
        };

        let result = subst.apply(&complex);
        let expected = Type::Array {
            element: Box::new(Type::Reference {
                lifetime: None,
                mutable: true,
                inner: Box::new(Type::Tuple(vec![Type::I32, Type::Bool])),
            }),
            size: 5,
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn test_substitution_clone() {
        let mut subst1 = Substitution::new();
        subst1.bind(TypeVar(0), Type::I32).unwrap();

        let subst2 = subst1.clone();

        assert_eq!(
            subst2.apply(&Type::Variable(TypeVar(0))),
            Type::I32
        );
    }
}