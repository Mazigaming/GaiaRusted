//! # Unification Engine
//!
//! Implements Robinson's unification algorithm for type inference.
//!
//! The algorithm finds a substitution that makes two types equal.
//! Key property: unification is idempotent and confluent.
//!
//! Examples:
//! - unify(X, i32) → X = i32
//! - unify([X, X], [i32, i32]) → X = i32
//! - unify(X, i32) + unify(X, f64) → Error (contradiction)
//! - unify(X, [X]) → Error (occurs check prevents infinite types)

use super::types::{Type, TypeVar};
use super::substitution::Substitution;

/// Performs unification of types using Robinson's algorithm
pub struct UnificationEngine {
    /// Counter for generating fresh variables if needed
    #[allow(dead_code)]
    var_counter: usize,
}

impl UnificationEngine {
    /// Create a new unification engine
    pub fn new() -> Self {
        Self { var_counter: 0 }
    }

    /// Unify two types
    ///
    /// # Arguments
    /// * `ty1`: First type to unify
    /// * `ty2`: Second type to unify
    /// * `subst`: Current substitution (will be modified)
    ///
    /// # Returns
    /// * Ok(()) if unification succeeds
    /// * Err(message) if unification fails
    ///
    /// # Example
    /// ```ignore
    /// let mut engine = UnificationEngine::new();
    /// let mut subst = Substitution::new();
    ///
    /// engine.unify(&Type::Variable(TypeVar(0)), &Type::I32, &mut subst)?;
    /// assert_eq!(subst.apply(&Type::Variable(TypeVar(0))), Type::I32);
    /// ```
    pub fn unify(
        &mut self,
        ty1: &Type,
        ty2: &Type,
        subst: &mut Substitution,
    ) -> Result<(), String> {
        // Apply current substitution to both types
        let ty1 = subst.apply(ty1);
        let ty2 = subst.apply(ty2);

        // Unify the resolved types
        self.unify_resolved(&ty1, &ty2, subst)
    }

    /// Internal unification for already-resolved types
    fn unify_resolved(
        &mut self,
        ty1: &Type,
        ty2: &Type,
        subst: &mut Substitution,
    ) -> Result<(), String> {
        match (ty1, ty2) {
            // Two identical concrete types: success
            (t1, t2) if t1 == t2 => Ok(()),

            // Variable unification: bind variable to type
            (Type::Variable(v1), Type::Variable(v2)) if v1 == v2 => Ok(()),
            (Type::Variable(v), t) | (t, Type::Variable(v)) => {
                subst.bind(*v, t.clone())
            }

            // Tuple unification: must have same length and unify elements
            (Type::Tuple(tys1), Type::Tuple(tys2)) => {
                if tys1.len() != tys2.len() {
                    return Err(format!(
                        "Tuple length mismatch: expected {}, got {}",
                        tys1.len(),
                        tys2.len()
                    ));
                }
                // Unify each pair of elements
                for (t1, t2) in tys1.iter().zip(tys2.iter()) {
                    self.unify(t1, t2, subst)?;
                }
                Ok(())
            }

            // Array unification: element types and size must match
            (
                Type::Array {
                    element: e1,
                    size: s1,
                },
                Type::Array {
                    element: e2,
                    size: s2,
                },
            ) => {
                if s1 != s2 {
                    return Err(format!(
                        "Array size mismatch: expected {}, got {}",
                        s1, s2
                    ));
                }
                self.unify(e1, e2, subst)
            }

            // Reference unification: mutability and inner type must match
            (
                Type::Reference {
                    mutable: m1,
                    inner: i1,
                    lifetime: _lt1,
                },
                Type::Reference {
                    mutable: m2,
                    inner: i2,
                    lifetime: _lt2,
                },
            ) => {
                if m1 != m2 {
                    return Err(format!(
                        "Mutability mismatch: {} vs {}",
                        if *m1 { "mut" } else { "const" },
                        if *m2 { "mut" } else { "const" }
                    ));
                }
                // Lifetime check: for now, accept any lifetime combination
                // (this is simplified; real Rust has more complex lifetime rules)
                self.unify(i1, i2, subst)
            }

            // Raw pointer unification: mutability and inner type must match
            (
                Type::RawPointer {
                    mutable: m1,
                    inner: i1,
                },
                Type::RawPointer {
                    mutable: m2,
                    inner: i2,
                },
            ) => {
                if m1 != m2 {
                    return Err(format!(
                        "Pointer mutability mismatch: {} vs {}",
                        if *m1 { "mut" } else { "const" },
                        if *m2 { "mut" } else { "const" }
                    ));
                }
                self.unify(i1, i2, subst)
            }

            // Function type unification: parameter count and types must match
            (
                Type::Function { params: p1, ret: r1 },
                Type::Function { params: p2, ret: r2 },
            ) => {
                if p1.len() != p2.len() {
                    return Err(format!(
                        "Function parameter count mismatch: expected {}, got {}",
                        p1.len(),
                        p2.len()
                    ));
                }
                // Unify each parameter pair
                for (p1, p2) in p1.iter().zip(p2.iter()) {
                    self.unify(p1, p2, subst)?;
                }
                // Unify return types
                self.unify(r1, r2, subst)
            }

            // No unification possible: type mismatch
            _ => Err(format!("Type mismatch: cannot unify {} and {}", ty1, ty2)),
        }
    }

    /// Unify a list of constraints (type equations)
    ///
    /// # Arguments
    /// * `constraints`: List of (ty1, ty2) pairs to unify
    /// * `subst`: Substitution to accumulate bindings
    ///
    /// # Returns
    /// * Ok(()) if all constraints unify successfully
    /// * Err(Vec<errors>) if any constraint fails
    pub fn unify_constraints(
        &mut self,
        constraints: &[(Type, Type)],
        subst: &mut Substitution,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for (ty1, ty2) in constraints {
            match self.unify(ty1, ty2, subst) {
                Ok(()) => {}
                Err(e) => errors.push(e),
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for UnificationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unify_same_concrete_type() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let result = engine.unify(&Type::I32, &Type::I32, &mut subst);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unify_different_concrete_types() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let result = engine.unify(&Type::I32, &Type::I64, &mut subst);
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_variable_to_concrete() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();
        let var = TypeVar(0);

        engine
            .unify(&Type::Variable(var), &Type::I32, &mut subst)
            .unwrap();

        assert_eq!(
            subst.apply(&Type::Variable(var)),
            Type::I32
        );
    }

    #[test]
    fn test_unify_concrete_to_variable() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();
        let var = TypeVar(0);

        engine
            .unify(&Type::I32, &Type::Variable(var), &mut subst)
            .unwrap();

        assert_eq!(
            subst.apply(&Type::Variable(var)),
            Type::I32
        );
    }

    #[test]
    fn test_unify_two_variables() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();
        let var1 = TypeVar(0);
        let var2 = TypeVar(1);

        engine
            .unify(&Type::Variable(var1), &Type::Variable(var2), &mut subst)
            .unwrap();

        // One should be bound to the other
        let result1 = subst.apply(&Type::Variable(var1));
        let result2 = subst.apply(&Type::Variable(var2));

        // After resolution, they should be equivalent
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_unify_occurs_check_simple() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();
        let var = TypeVar(0);

        // Try to unify X with [X] (infinite type)
        let array = Type::Array {
            element: Box::new(Type::Variable(var)),
            size: 1,
        };

        let result = engine.unify(&Type::Variable(var), &array, &mut subst);
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_occurs_check_nested() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();
        let var = TypeVar(0);

        // Try to unify X with (i32, X) (infinite type)
        let tuple = Type::Tuple(vec![Type::I32, Type::Variable(var)]);

        let result = engine.unify(&Type::Variable(var), &tuple, &mut subst);
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_tuple_same_length() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let tuple1 = Type::Tuple(vec![Type::I32, Type::Bool]);
        let tuple2 = Type::Tuple(vec![Type::I32, Type::Bool]);

        let result = engine.unify(&tuple1, &tuple2, &mut subst);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unify_tuple_different_length() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let tuple1 = Type::Tuple(vec![Type::I32]);
        let tuple2 = Type::Tuple(vec![Type::I32, Type::Bool]);

        let result = engine.unify(&tuple1, &tuple2, &mut subst);
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_tuple_with_variables() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let tuple1 = Type::Tuple(vec![Type::Variable(TypeVar(0)), Type::Bool]);
        let tuple2 = Type::Tuple(vec![Type::I32, Type::Bool]);

        engine.unify(&tuple1, &tuple2, &mut subst).unwrap();

        // X should be bound to i32
        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(0))),
            Type::I32
        );
    }

    #[test]
    fn test_unify_array_same_size() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let array1 = Type::Array {
            element: Box::new(Type::I32),
            size: 10,
        };
        let array2 = Type::Array {
            element: Box::new(Type::I32),
            size: 10,
        };

        let result = engine.unify(&array1, &array2, &mut subst);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unify_array_different_size() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let array1 = Type::Array {
            element: Box::new(Type::I32),
            size: 10,
        };
        let array2 = Type::Array {
            element: Box::new(Type::I32),
            size: 20,
        };

        let result = engine.unify(&array1, &array2, &mut subst);
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_array_variable_element() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let array1 = Type::Array {
            element: Box::new(Type::Variable(TypeVar(0))),
            size: 10,
        };
        let array2 = Type::Array {
            element: Box::new(Type::I32),
            size: 10,
        };

        engine.unify(&array1, &array2, &mut subst).unwrap();

        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(0))),
            Type::I32
        );
    }

    #[test]
    fn test_unify_reference_immutable() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let ref1 = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };
        let ref2 = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let result = engine.unify(&ref1, &ref2, &mut subst);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unify_reference_mutability_mismatch() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let ref1 = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };
        let ref2 = Type::Reference {
            lifetime: None,
            mutable: true,
            inner: Box::new(Type::I32),
        };

        let result = engine.unify(&ref1, &ref2, &mut subst);
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_reference_with_variable() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let ref1 = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::Variable(TypeVar(0))),
        };
        let ref2 = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::Bool),
        };

        engine.unify(&ref1, &ref2, &mut subst).unwrap();

        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(0))),
            Type::Bool
        );
    }

    #[test]
    fn test_unify_raw_pointer_const() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let ptr1 = Type::RawPointer {
            mutable: false,
            inner: Box::new(Type::I32),
        };
        let ptr2 = Type::RawPointer {
            mutable: false,
            inner: Box::new(Type::I32),
        };

        let result = engine.unify(&ptr1, &ptr2, &mut subst);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unify_raw_pointer_mutability_mismatch() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let ptr1 = Type::RawPointer {
            mutable: false,
            inner: Box::new(Type::I32),
        };
        let ptr2 = Type::RawPointer {
            mutable: true,
            inner: Box::new(Type::I32),
        };

        let result = engine.unify(&ptr1, &ptr2, &mut subst);
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_function_same_signature() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let func1 = Type::Function {
            params: vec![Type::I32, Type::Bool],
            ret: Box::new(Type::Str),
        };
        let func2 = Type::Function {
            params: vec![Type::I32, Type::Bool],
            ret: Box::new(Type::Str),
        };

        let result = engine.unify(&func1, &func2, &mut subst);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unify_function_different_param_count() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let func1 = Type::Function {
            params: vec![Type::I32],
            ret: Box::new(Type::Str),
        };
        let func2 = Type::Function {
            params: vec![Type::I32, Type::Bool],
            ret: Box::new(Type::Str),
        };

        let result = engine.unify(&func1, &func2, &mut subst);
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_function_with_variables() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let func1 = Type::Function {
            params: vec![Type::Variable(TypeVar(0)), Type::Bool],
            ret: Box::new(Type::Variable(TypeVar(1))),
        };
        let func2 = Type::Function {
            params: vec![Type::I32, Type::Bool],
            ret: Box::new(Type::Str),
        };

        engine.unify(&func1, &func2, &mut subst).unwrap();

        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(0))),
            Type::I32
        );
        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(1))),
            Type::Str
        );
    }

    #[test]
    fn test_unify_constraints() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let constraints = vec![
            (Type::Variable(TypeVar(0)), Type::I32),
            (Type::Variable(TypeVar(1)), Type::Bool),
        ];

        let result = engine.unify_constraints(&constraints, &mut subst);
        assert!(result.is_ok());

        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(0))),
            Type::I32
        );
        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(1))),
            Type::Bool
        );
    }

    #[test]
    fn test_unify_constraints_with_failure() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let constraints = vec![
            (Type::Variable(TypeVar(0)), Type::I32),
            (Type::Variable(TypeVar(0)), Type::Bool), // Contradiction!
        ];

        let result = engine.unify_constraints(&constraints, &mut subst);
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_nested_unification() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let complex1 = Type::Array {
            element: Box::new(Type::Reference {
                lifetime: None,
                mutable: false,
                inner: Box::new(Type::Tuple(vec![
                    Type::Variable(TypeVar(0)),
                    Type::Bool,
                ])),
            }),
            size: 5,
        };

        let complex2 = Type::Array {
            element: Box::new(Type::Reference {
                lifetime: None,
                mutable: false,
                inner: Box::new(Type::Tuple(vec![Type::I32, Type::Bool])),
            }),
            size: 5,
        };

        engine.unify(&complex1, &complex2, &mut subst).unwrap();

        assert_eq!(
            subst.apply(&Type::Variable(TypeVar(0))),
            Type::I32
        );
    }

    #[test]
    fn test_unify_same_variable_twice() {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        let var = TypeVar(0);
        let tuple1 = Type::Tuple(vec![Type::Variable(var), Type::Variable(var)]);
        let tuple2 = Type::Tuple(vec![Type::I32, Type::I32]);

        engine.unify(&tuple1, &tuple2, &mut subst).unwrap();

        // X should be bound to i32
        assert_eq!(subst.apply(&Type::Variable(var)), Type::I32);
    }
}