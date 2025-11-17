//! # Unsafe Code Validation
//!
//! Validates that unsafe operations only occur within unsafe blocks.
//! Unsafe operations include:
//! - Dereferencing raw pointers (*const T, *mut T)
//! - Calling unsafe functions
//! - Accessing mutable statics
//! - Accessing union fields
//!
//! Track whether we're currently in an unsafe context as we traverse the AST.

use crate::lowering::{HirExpression, HirStatement, HirType, HirItem};
use std::fmt;

/// Unsafe checking error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsafeError {
    pub message: String,
}

impl fmt::Display for UnsafeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub type UnsafeCheckResult<T> = Result<T, UnsafeError>;

/// Tracks unsafe context while traversing the program
#[derive(Debug)]
pub struct UnsafeChecker {
    /// Nesting level of unsafe blocks (0 = not in unsafe)
    unsafe_depth: usize,
    
    /// Names of functions marked as unsafe
    unsafe_functions: std::collections::HashSet<String>,
    
    /// Errors found during checking
    errors: Vec<UnsafeError>,
}

impl UnsafeChecker {
    pub fn new() -> Self {
        UnsafeChecker {
            unsafe_depth: 0,
            unsafe_functions: std::collections::HashSet::new(),
            errors: Vec::new(),
        }
    }
    
    /// Mark a function as unsafe
    pub fn register_unsafe_function(&mut self, name: &str) {
        self.unsafe_functions.insert(name.to_string());
    }
    
    /// Check if a function is marked as unsafe
    pub fn is_unsafe_function(&self, name: &str) -> bool {
        self.unsafe_functions.contains(name)
    }
    
    /// Check a raw pointer dereference
    pub fn check_pointer_deref(&mut self) -> UnsafeCheckResult<()> {
        if !self.is_in_unsafe_context() {
            let error = UnsafeError {
                message: "cannot dereference raw pointer outside of unsafe block".to_string(),
            };
            self.errors.push(error.clone());
            return Err(error);
        }
        Ok(())
    }
    
    /// Check an unsafe function call
    pub fn check_unsafe_function_call(&mut self, func_name: &str) -> UnsafeCheckResult<()> {
        if !self.is_in_unsafe_context() {
            let error = UnsafeError {
                message: format!("cannot call unsafe function '{}' outside of unsafe block", func_name),
            };
            self.errors.push(error.clone());
            return Err(error);
        }
        Ok(())
    }
    
    /// Check mutable static access
    pub fn check_mutable_static_access(&mut self, var_name: &str) -> UnsafeCheckResult<()> {
        if !self.is_in_unsafe_context() {
            let error = UnsafeError {
                message: format!("cannot access mutable static '{}' outside of unsafe block", var_name),
            };
            self.errors.push(error.clone());
            return Err(error);
        }
        Ok(())
    }
    
    /// Check union field access
    pub fn check_union_field_access(&mut self) -> UnsafeCheckResult<()> {
        if !self.is_in_unsafe_context() {
            let error = UnsafeError {
                message: "cannot access union field outside of unsafe block".to_string(),
            };
            self.errors.push(error.clone());
            return Err(error);
        }
        Ok(())
    }
    
    /// Enter an unsafe block
    pub fn enter_unsafe_block(&mut self) {
        self.unsafe_depth += 1;
    }
    
    /// Exit an unsafe block
    pub fn exit_unsafe_block(&mut self) {
        if self.unsafe_depth > 0 {
            self.unsafe_depth -= 1;
        }
    }
    
    /// Check if currently in unsafe context
    pub fn is_in_unsafe_context(&self) -> bool {
        self.unsafe_depth > 0
    }
    
    /// Get all errors found
    pub fn errors(&self) -> &[UnsafeError] {
        &self.errors
    }
    
    /// Check a type for unsafe operations
    pub fn check_type(&mut self, ty: &HirType) -> UnsafeCheckResult<()> {
        match ty {
            HirType::Pointer(_) => {
                // Pointer types themselves are OK, dereferencing them is what's unsafe
                Ok(())
            }
            HirType::Array { element_type, .. } => {
                self.check_type(element_type)
            }
            HirType::Reference(inner) | HirType::MutableReference(inner) => {
                self.check_type(inner)
            }
            HirType::Tuple(types) => {
                for ty in types {
                    self.check_type(ty)?;
                }
                Ok(())
            }
            HirType::Function { params, return_type } => {
                // Function types are OK, calling them is what's unsafe
                for param in params {
                    self.check_type(param)?;
                }
                self.check_type(return_type)
            }
            HirType::Closure { params, return_type, .. } => {
                // Closure types are OK, calling them is what's unsafe
                for param in params {
                    self.check_type(param)?;
                }
                self.check_type(return_type)
            }
            HirType::Named(_) |
            HirType::Int32 |
            HirType::Int64 |
            HirType::UInt32 |
            HirType::UInt64 |
            HirType::USize |
            HirType::ISize |
            HirType::Float64 |
            HirType::Bool |
            HirType::Char |
            HirType::String |
            HirType::Unknown => Ok(()),
        }
    }
    
    /// Check an expression for unsafe operations
    pub fn check_expression(&mut self, expr: &HirExpression) -> UnsafeCheckResult<()> {
        use crate::lowering::UnaryOp;
        
        match expr {
            HirExpression::UnaryOp { op, operand } => {
                // Check for pointer dereference: *ptr
                if matches!(op, UnaryOp::Dereference) {
                    self.check_pointer_deref()?;
                }
                self.check_expression(operand)
            }
            HirExpression::Call { func, args } => {
                // Check if calling an unsafe function
                if let HirExpression::Variable(name) = func.as_ref() {
                    if self.is_unsafe_function(name) {
                        self.check_unsafe_function_call(name)?;
                    }
                }
                self.check_expression(func)?;
                for arg in args {
                    self.check_expression(arg)?;
                }
                Ok(())
            }
            HirExpression::FieldAccess { object, .. } => {
                // TODO: Check if accessing a union field
                self.check_expression(object)
            }
            HirExpression::BinaryOp { left, right, .. } => {
                self.check_expression(left)?;
                self.check_expression(right)
            }
            HirExpression::Variable(_) => {
                // TODO: Check if it's a mutable static
                Ok(())
            }
            HirExpression::Assign { target, value } => {
                self.check_expression(target)?;
                self.check_expression(value)
            }
            HirExpression::If { condition, then_body, else_body } => {
                self.check_expression(condition)?;
                for stmt in then_body {
                    self.check_statement(stmt)?;
                }
                if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        self.check_statement(stmt)?;
                    }
                }
                Ok(())
            }
            HirExpression::Index { array, index } => {
                self.check_expression(array)?;
                self.check_expression(index)
            }
            HirExpression::Match { scrutinee, arms } => {
                self.check_expression(scrutinee)?;
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.check_expression(guard)?;
                    }
                    for stmt in &arm.body {
                        self.check_statement(stmt)?;
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    
    /// Check a statement for unsafe operations
    pub fn check_statement(&mut self, stmt: &HirStatement) -> UnsafeCheckResult<()> {
        match stmt {
            HirStatement::Expression(expr) => {
                self.check_expression(expr)
            }
            HirStatement::Let { init, .. } => {
                self.check_expression(init)
            }
            HirStatement::UnsafeBlock(stmts) => {
                // Enter unsafe context for this block
                self.enter_unsafe_block();
                
                for stmt in stmts {
                    self.check_statement(stmt)?;
                }
                
                // Exit unsafe context
                self.exit_unsafe_block();
                Ok(())
            }
            HirStatement::If { condition, then_body, else_body } => {
                self.check_expression(condition)?;
                for stmt in then_body {
                    self.check_statement(stmt)?;
                }
                if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        self.check_statement(stmt)?;
                    }
                }
                Ok(())
            }
            HirStatement::While { condition, body } => {
                self.check_expression(condition)?;
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                Ok(())
            }
            HirStatement::For { iter, body, .. } => {
                self.check_expression(iter)?;
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                Ok(())
            }
            HirStatement::Return(Some(expr)) => {
                self.check_expression(expr)
            }
            HirStatement::Item(item) => {
                self.check_item(item)
            }
            _ => Ok(()),
        }
    }
    
    /// Check an item for unsafe definitions
    pub fn check_item(&mut self, item: &HirItem) -> UnsafeCheckResult<()> {
        match item {
            HirItem::Function { body, .. } => {
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                Ok(())
            }
            HirItem::Struct { .. } => Ok(()),
            HirItem::AssociatedType { .. } => Ok(()),
            HirItem::Use { .. } => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_unsafe_checker() {
        let checker = UnsafeChecker::new();
        assert!(!checker.is_in_unsafe_context());
        assert_eq!(checker.errors().len(), 0);
    }

    #[test]
    fn test_pointer_deref_in_unsafe_allowed() {
        let mut checker = UnsafeChecker::new();
        checker.enter_unsafe_block();
        assert!(checker.check_pointer_deref().is_ok());
    }

    #[test]
    fn test_pointer_deref_outside_unsafe_error() {
        let mut checker = UnsafeChecker::new();
        assert!(checker.check_pointer_deref().is_err());
        assert_eq!(checker.errors().len(), 1);
    }

    #[test]
    fn test_unsafe_function_call_in_unsafe_allowed() {
        let mut checker = UnsafeChecker::new();
        checker.register_unsafe_function("dangerous");
        checker.enter_unsafe_block();
        assert!(checker.check_unsafe_function_call("dangerous").is_ok());
    }

    #[test]
    fn test_unsafe_function_call_outside_unsafe_error() {
        let mut checker = UnsafeChecker::new();
        checker.register_unsafe_function("dangerous");
        let result = checker.check_unsafe_function_call("dangerous");
        assert!(result.is_err());
        assert_eq!(checker.errors().len(), 1);
        assert_eq!(
            checker.errors()[0].message,
            "cannot call unsafe function 'dangerous' outside of unsafe block"
        );
    }

    #[test]
    fn test_mutable_static_in_unsafe_allowed() {
        let mut checker = UnsafeChecker::new();
        checker.enter_unsafe_block();
        assert!(checker.check_mutable_static_access("GLOBAL").is_ok());
    }

    #[test]
    fn test_mutable_static_outside_unsafe_error() {
        let mut checker = UnsafeChecker::new();
        let result = checker.check_mutable_static_access("GLOBAL");
        assert!(result.is_err());
        assert_eq!(checker.errors().len(), 1);
    }

    #[test]
    fn test_union_access_in_unsafe_allowed() {
        let mut checker = UnsafeChecker::new();
        checker.enter_unsafe_block();
        assert!(checker.check_union_field_access().is_ok());
    }

    #[test]
    fn test_union_access_outside_unsafe_error() {
        let mut checker = UnsafeChecker::new();
        let result = checker.check_union_field_access();
        assert!(result.is_err());
        assert_eq!(checker.errors().len(), 1);
    }

    #[test]
    fn test_nested_unsafe_blocks() {
        let mut checker = UnsafeChecker::new();
        assert!(!checker.is_in_unsafe_context());
        
        checker.enter_unsafe_block();
        assert!(checker.is_in_unsafe_context());
        
        checker.enter_unsafe_block();
        assert!(checker.is_in_unsafe_context());
        
        checker.exit_unsafe_block();
        assert!(checker.is_in_unsafe_context());
        
        checker.exit_unsafe_block();
        assert!(!checker.is_in_unsafe_context());
    }

    #[test]
    fn test_multiple_unsafe_errors() {
        let mut checker = UnsafeChecker::new();
        
        let _ = checker.check_pointer_deref();
        let _ = checker.check_unsafe_function_call("bad");
        let _ = checker.check_mutable_static_access("STATIC");
        
        assert_eq!(checker.errors().len(), 3);
    }
}
