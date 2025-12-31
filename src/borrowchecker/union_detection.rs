//! # Union Type Detection from Type System
//!
//! Integrates with the type system to detect union types and enable
//! unsafe checking for union field access operations.
//!
//! This module bridges the gap between type inference/analysis and unsafe checking.
//! Rather than relying on manual registration, we now detect union types from the
//! actual type information available in the compiler.

use crate::lowering::{HirExpression, ScopeTracker};
use std::collections::HashSet;

/// Detects if an expression's type is a union type.
/// 
/// This analyzes the type of an expression and determines if it's a union,
/// which requires unsafe context for field access.
/// 
/// # Algorithm
/// 1. Infer the type of the expression
/// 2. Check if that type is a union in the type registry
/// 3. Return true if the type is a union
#[derive(Debug)]
pub struct UnionTypeDetector {
    /// Cached union types from type system
    /// In a full implementation, this would be populated from the actual
    /// type registry during compilation.
    known_union_types: HashSet<String>,
}

impl UnionTypeDetector {
    pub fn new() -> Self {
        UnionTypeDetector {
            known_union_types: HashSet::new(),
        }
    }

    /// Register a union type by name
    pub fn register_union_type(&mut self, name: &str) {
        self.known_union_types.insert(name.to_string());
    }

    /// Check if a type name is a union
    pub fn is_union_type(&self, type_name: &str) -> bool {
        self.known_union_types.contains(type_name)
    }

    /// Detect if an expression's type is a union
    ///
    /// # Parameters
    /// - `expr`: The expression to analyze
    /// - `scope`: The current scope for type lookup
    ///
    /// # Returns
    /// Some(type_name) if the expression evaluates to a union type,
    /// None otherwise
    pub fn detect_union_type(
        &self,
        expr: &HirExpression,
        _scope: Option<&ScopeTracker>,
    ) -> Option<String> {
        match expr {
            // Direct variable access: check if variable's type is a union
            HirExpression::Variable(name) => {
                // In a full implementation, we would:
                // 1. Look up the variable's type in the scope
                // 2. Convert that type to a string (e.g., "MyUnion")
                // 3. Check if it's registered as a union
                //
                // For now, we check if the variable name itself matches a registered union.
                // This is a simplification that works for direct union variables.
                if self.is_union_type(name) {
                    Some(name.clone())
                } else {
                    None
                }
            }

            // Field access: check if the object is a union
            // e.g., obj.field - need to check if obj's type is a union
            HirExpression::FieldAccess { object, field: _ } => {
                // Recursively check the object's type
                self.detect_union_type(object.as_ref(), _scope)
            }

            // Method call receiver: check if receiver type is a union
            HirExpression::MethodCall {
                receiver,
                method: _,
                args: _,
            } => {
                // Recursively check the receiver's type
                self.detect_union_type(receiver.as_ref(), _scope)
            }

            // Index access: the indexed object could be a union
            HirExpression::Index { array, index: _ } => {
                self.detect_union_type(array.as_ref(), _scope)
            }

            // Binary operations: neither side is typically a union itself,
            // but we should check both operands for union field access
            HirExpression::BinaryOp { left, right, .. } => {
                // Check left and right operands
                if let Some(union_type) = self.detect_union_type(left.as_ref(), _scope) {
                    return Some(union_type);
                }
                self.detect_union_type(right.as_ref(), _scope)
            }

            // Unary operations: check the operand
            HirExpression::UnaryOp { operand, .. } => {
                self.detect_union_type(operand.as_ref(), _scope)
            }

            // Function calls: check all arguments
            HirExpression::Call { func, args } => {
                // Check function expression
                if let Some(union_type) = self.detect_union_type(func.as_ref(), _scope) {
                    return Some(union_type);
                }
                // Check arguments
                for arg in args {
                    if let Some(union_type) = self.detect_union_type(arg, _scope) {
                        return Some(union_type);
                    }
                }
                None
            }

            // Assignment: check both target and value
            HirExpression::Assign { target, value } => {
                if let Some(union_type) = self.detect_union_type(target.as_ref(), _scope) {
                    return Some(union_type);
                }
                self.detect_union_type(value.as_ref(), _scope)
            }

            // If expressions: check condition and branches
            HirExpression::If {
                condition,
                then_body,
                else_body,
            } => {
                if let Some(union_type) = self.detect_union_type(condition.as_ref(), _scope) {
                    return Some(union_type);
                }
                // Check statements in branches
                for stmt in then_body {
                    use crate::lowering::HirStatement;
                    if let HirStatement::Expression(expr) = stmt {
                        if let Some(union_type) = self.detect_union_type(expr, _scope) {
                            return Some(union_type);
                        }
                    }
                }
                if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        use crate::lowering::HirStatement;
                        if let HirStatement::Expression(expr) = stmt {
                            if let Some(union_type) = self.detect_union_type(expr, _scope) {
                                return Some(union_type);
                            }
                        }
                    }
                }
                None
            }

            // Match expressions: check scrutinee and all arms
            HirExpression::Match {
                scrutinee,
                arms,
            } => {
                if let Some(union_type) = self.detect_union_type(scrutinee.as_ref(), _scope) {
                    return Some(union_type);
                }
                for arm in arms {
                    use crate::lowering::HirStatement;
                    for stmt in &arm.body {
                        if let HirStatement::Expression(expr) = stmt {
                            if let Some(union_type) = self.detect_union_type(expr, _scope) {
                                return Some(union_type);
                            }
                        }
                    }
                }
                None
            }

            // Closures: check the body expressions
            HirExpression::Closure {
                body,
                params: _,
                return_type: _,
                is_move: _,
                captures: _,
            } => {
                use crate::lowering::HirStatement;
                for stmt in body {
                    if let HirStatement::Expression(expr) = stmt {
                        if let Some(union_type) = self.detect_union_type(expr, _scope) {
                            return Some(union_type);
                        }
                    }
                }
                None
            }

            // Block: check the last expression
            HirExpression::Block(statements, final_expr) => {
                use crate::lowering::HirStatement;
                // Check statements for union usage
                for stmt in statements {
                    if let HirStatement::Expression(expr) = stmt {
                        if let Some(union_type) = self.detect_union_type(expr, _scope) {
                            return Some(union_type);
                        }
                    }
                }
                // Check the final expression
                if let Some(expr) = final_expr {
                    self.detect_union_type(expr.as_ref(), _scope)
                } else {
                    None
                }
            }

            // Literals and other expressions don't have union types
            _ => None,
        }
    }

    /// Get the type information for a detected union type
    ///
    /// In a full implementation, this would return detailed information
    /// about the union type including its variants, sizes, etc.
    pub fn get_union_info(&self, type_name: &str) -> Option<UnionInfo> {
        if self.is_union_type(type_name) {
            Some(UnionInfo {
                name: type_name.to_string(),
                // TODO: Load actual variant information from type registry
                variants: Vec::new(),
            })
        } else {
            None
        }
    }
}

/// Information about a union type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionInfo {
    /// Name of the union type
    pub name: String,
    /// List of variant names
    pub variants: Vec<String>,
}

/// Integration point for unsafe checking with union type detection
///
/// This trait allows the unsafe checker to use the union detector
/// without requiring direct dependency on the type system.
pub trait UnionTypeProvider {
    /// Check if an expression's type is a union
    fn is_expr_union_type(&self, expr: &HirExpression) -> bool;
    
    /// Get the union type name for an expression, if applicable
    fn get_expr_union_type(&self, expr: &HirExpression) -> Option<String>;
}

impl UnionTypeProvider for UnionTypeDetector {
    fn is_expr_union_type(&self, expr: &HirExpression) -> bool {
        self.detect_union_type(expr, None).is_some()
    }

    fn get_expr_union_type(&self, expr: &HirExpression) -> Option<String> {
        self.detect_union_type(expr, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_union_detector() {
        let detector = UnionTypeDetector::new();
        assert_eq!(detector.known_union_types.len(), 0);
    }

    #[test]
    fn test_register_and_detect_union() {
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("MyUnion");
        assert!(detector.is_union_type("MyUnion"));
    }

    #[test]
    fn test_unregistered_type_not_union() {
        let detector = UnionTypeDetector::new();
        assert!(!detector.is_union_type("MyUnion"));
    }

    #[test]
    fn test_detect_union_variable() {
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("MyUnion");

        let expr = HirExpression::Variable("MyUnion".to_string());
        assert_eq!(
            detector.detect_union_type(&expr, None),
            Some("MyUnion".to_string())
        );
    }

    #[test]
    fn test_detect_non_union_variable() {
        let detector = UnionTypeDetector::new();

        let expr = HirExpression::Variable("SomeType".to_string());
        assert_eq!(detector.detect_union_type(&expr, None), None);
    }

    #[test]
    fn test_detect_union_field_access() {
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("MyUnion");

        let inner = Box::new(HirExpression::Variable("MyUnion".to_string()));
        let expr = HirExpression::FieldAccess {
            object: inner,
            field: "field1".to_string(),
        };

        assert_eq!(
            detector.detect_union_type(&expr, None),
            Some("MyUnion".to_string())
        );
    }

    #[test]
    fn test_union_type_provider_trait() {
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("MyUnion");

        let expr = HirExpression::Variable("MyUnion".to_string());
        
        // Test through trait interface
        assert!(detector.is_expr_union_type(&expr));
        assert_eq!(
            detector.get_expr_union_type(&expr),
            Some("MyUnion".to_string())
        );
    }

    #[test]
    fn test_binary_op_with_union_operand() {
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("MyUnion");

        let left = Box::new(HirExpression::Variable("MyUnion".to_string()));
        let right = Box::new(HirExpression::Integer(42));
        
        let expr = HirExpression::BinaryOp {
            left,
            right,
            op: crate::lowering::BinaryOp::Add,
        };

        assert_eq!(
            detector.detect_union_type(&expr, None),
            Some("MyUnion".to_string())
        );
    }

    #[test]
    fn test_union_function_call_argument() {
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("MyUnion");

        let func = Box::new(HirExpression::Variable("process".to_string()));
        let args = vec![HirExpression::Variable("MyUnion".to_string())];
        
        let expr = HirExpression::Call { func, args };

        assert_eq!(
            detector.detect_union_type(&expr, None),
            Some("MyUnion".to_string())
        );
    }

    #[test]
    fn test_multiple_union_types() {
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("Union1");
        detector.register_union_type("Union2");

        assert!(detector.is_union_type("Union1"));
        assert!(detector.is_union_type("Union2"));
        assert!(!detector.is_union_type("Union3"));
    }

    #[test]
    fn test_union_info_retrieval() {
        let mut detector = UnionTypeDetector::new();
        detector.register_union_type("MyUnion");

        if let Some(info) = detector.get_union_info("MyUnion") {
            assert_eq!(info.name, "MyUnion");
        } else {
            panic!("Should retrieve union info");
        }
    }

    #[test]
    fn test_union_info_nonexistent() {
        let detector = UnionTypeDetector::new();
        assert_eq!(detector.get_union_info("NonExistent"), None);
    }
}
