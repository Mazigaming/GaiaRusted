//! # For-Loop Desugaring
//!
//! Converts `for item in collection { body }` into explicit iterator protocol:
//!
//! ```ignore
//! {
//!     let mut iter = collection.into_iter();
//!     loop {
//!         match iter.next() {
//!             Some(item) => { body }
//!             None => break
//!         }
//!     }
//! }
//! ```

use crate::lowering::{HirStatement, HirExpression, HirType};
use std::fmt;

/// Error during for-loop desugaring
#[derive(Debug, Clone)]
pub struct DesugarError {
    pub message: String,
}

impl fmt::Display for DesugarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

type DesugarResult<T> = Result<T, DesugarError>;

/// Counter for generating unique temporary variable names
thread_local! {
    static TEMP_COUNTER: std::cell::RefCell<usize> = std::cell::RefCell::new(0);
}

/// Generate a unique temporary variable name
fn gen_temp_var() -> String {
    TEMP_COUNTER.with(|counter| {
        let mut c = counter.borrow_mut();
        let name = format!("__iter_{}", c);
        *c += 1;
        name
    })
}

/// Reset the temporary counter (for testing)
#[allow(dead_code)]
pub fn reset_temp_counter() {
    TEMP_COUNTER.with(|counter| {
        *counter.borrow_mut() = 0;
    });
}

use crate::lowering::MatchArm;

/// Desugar a for-loop into iterator protocol
///
/// # Input
/// ```ignore
/// for item in collection {
///     body
/// }
/// ```
///
/// # Output
/// ```ignore
/// {
///     let mut iter = collection.into_iter();
///     {
///         match iter.next() {
///             Some(item) => { body }
///             None => ()
///         }
///     }
/// }
/// ```
pub fn desugar_for_loop(
    pattern: String,
    collection: HirExpression,
    body: Vec<HirStatement>,
) -> DesugarResult<Vec<HirStatement>> {
    let iter_var = gen_temp_var();

    // Step 1: Create binding: let mut iter = collection.into_iter();
    let into_iter_call = HirExpression::MethodCall {
        receiver: Box::new(collection),
        method: "into_iter".to_string(),
        args: vec![],
    };

    let iter_binding = HirStatement::Let {
        name: iter_var.clone(),
        mutable: true,
        ty: HirType::Unknown, // Will be inferred
        init: into_iter_call,
    };

    // Step 2: Create match expression: iter.next()
    let next_call = HirExpression::MethodCall {
        receiver: Box::new(HirExpression::Variable(iter_var.clone())),
        method: "next".to_string(),
        args: vec![],
    };

    // Step 3: Create match arms
    // Arm 1: Some(item) => { body }
    let some_arm = MatchArm {
        pattern: format!("Some({})", pattern),
        guard: None,
        body,
    };

    // Arm 2: None => ()  (do nothing, allow loop to continue/naturally exit)
    let none_arm = MatchArm {
        pattern: "None".to_string(),
        guard: None,
        body: vec![HirStatement::Expression(HirExpression::Tuple(vec![]))],
    };

    let match_expr = HirExpression::Match {
        scrutinee: Box::new(next_call),
        arms: vec![some_arm, none_arm],
    };

    // Step 4: Wrap the match in an expression statement
    let match_stmt = HirStatement::Expression(match_expr);

    // Return sequence: binding, match
    Ok(vec![iter_binding, match_stmt])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desugar_simple_for_loop() {
        reset_temp_counter();
        
        let collection = HirExpression::Variable("v".to_string());
        let body = vec![
            HirStatement::Expression(
                HirExpression::Variable("x".to_string())
            ),
        ];

        let result = desugar_for_loop("item".to_string(), collection, body);
        assert!(result.is_ok());

        let stmts = result.unwrap();
        assert_eq!(stmts.len(), 2, "Should produce 2 statements: binding + match");

        // First statement should be a binding
        match &stmts[0] {
            HirStatement::Let { name, mutable, .. } => {
                assert!(name.contains("__iter"), "Should create temp iterator");
                assert!(*mutable, "Iterator should be mutable");
            }
            _ => assert!(false, "First statement should be Let binding (got a different statement type)"),
        }

        // Second statement should be an expression (the match)
        match &stmts[1] {
            HirStatement::Expression(expr) => {
                match expr {
                    HirExpression::Match { arms, .. } => {
                        assert_eq!(arms.len(), 2, "Should have 2 match arms: Some and None");
                    }
                    _ => assert!(false, "Should be Match expression"),
                }
            }
            _ => assert!(false, "Second statement should be Expression"),
        }
    }

    #[test]
    fn test_desugar_generates_match() {
        reset_temp_counter();
        
        let collection = HirExpression::Variable("v".to_string());
        let body = vec![];

        let result = desugar_for_loop("x".to_string(), collection, body);
        assert!(result.is_ok());

        let stmts = result.unwrap();
        
        // Check the match expression
        match &stmts[1] {
            HirStatement::Expression(expr) => {
                match expr {
                    HirExpression::Match { arms, .. } => {
                        assert_eq!(arms.len(), 2, "Should have 2 match arms: Some and None");
                        
                        assert_eq!(arms[0].pattern, "Some(x)");
                        assert_eq!(arms[1].pattern, "None");
                    }
                    _ => panic!("Should be Match expression"),
                }
            }
            _ => panic!("Should be Expression"),
        }
    }

    #[test]
    fn test_desugar_preserves_body() {
        reset_temp_counter();
        
        let collection = HirExpression::Variable("v".to_string());
        let body_expr = HirExpression::Integer(42);
        let body = vec![
            HirStatement::Expression(body_expr),
        ];

        let result = desugar_for_loop("x".to_string(), collection, body);
        assert!(result.is_ok());

        let stmts = result.unwrap();
        
        // Check the match contains the body
        match &stmts[1] {
            HirStatement::Expression(expr) => {
                match expr {
                    HirExpression::Match { arms, .. } => {
                        // Some arm should contain our original body
                        assert_eq!(arms[0].body.len(), 1, "Some arm should have body");
                    }
                    _ => assert!(false, "Should be Match expression"),
                }
            }
            _ => assert!(false, "Should be Expression"),
        }
    }

    #[test]
    fn test_temp_var_generation() {
        reset_temp_counter();
        
        let var1 = gen_temp_var();
        let var2 = gen_temp_var();
        let var3 = gen_temp_var();
        
        assert_eq!(var1, "__iter_0");
        assert_eq!(var2, "__iter_1");
        assert_eq!(var3, "__iter_2");
    }

    #[test]
    fn test_multiple_desugars_different_temps() {
        reset_temp_counter();
        
        let collection1 = HirExpression::Variable("v1".to_string());
        let collection2 = HirExpression::Variable("v2".to_string());
        
        let result1 = desugar_for_loop("x".to_string(), collection1, vec![]);
        let result2 = desugar_for_loop("y".to_string(), collection2, vec![]);
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        let stmts1 = result1.unwrap();
        let stmts2 = result2.unwrap();
        
        // Extract iter var names
        let iter1 = match &stmts1[0] {
            HirStatement::Let { name, .. } => name.clone(),
            _ => panic!("Expected Let binding for iter1"),
        };
        
        let iter2 = match &stmts2[0] {
            HirStatement::Let { name, .. } => name.clone(),
            _ => panic!("Expected Let binding for iter2"),
        };
        
        // Should be different
        assert_ne!(iter1, iter2, "Different for-loops should have different iter vars");
    }

    #[test]
    fn test_into_iter_method_call() {
        reset_temp_counter();
        
        let collection = HirExpression::Variable("v".to_string());
        let result = desugar_for_loop("x".to_string(), collection, vec![]);
        assert!(result.is_ok());

        let stmts = result.unwrap();
        
        // Check the binding has into_iter call
        match &stmts[0] {
            HirStatement::Let { init: expr, .. } => {
                match expr {
                    HirExpression::MethodCall { method, .. } => {
                        assert_eq!(method, "into_iter", "Should call into_iter method");
                    }
                    _ => assert!(false, "Init should be method call"),
                }
            }
            _ => assert!(false, "Should be Let with init"),
        }
    }

    #[test]
    fn test_next_method_call() {
        reset_temp_counter();
        
        let collection = HirExpression::Variable("v".to_string());
        let result = desugar_for_loop("x".to_string(), collection, vec![]);
        assert!(result.is_ok());

        let stmts = result.unwrap();
        
        // Check the match scrutinee is next() call
        match &stmts[1] {
            HirStatement::Expression(expr) => {
                match expr {
                    HirExpression::Match { scrutinee, .. } => {
                        match scrutinee.as_ref() {
                            HirExpression::MethodCall { method, .. } => {
                                assert_eq!(method, "next", "Should call next method");
                            }
                            _ => panic!("Scrutinee should be method call"),
                        }
                    }
                    _ => panic!("Should be Match"),
                }
            }
            _ => panic!("Should be Expression"),
        }
    }
}
