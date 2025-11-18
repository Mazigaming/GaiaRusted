//! # Enhanced Unsafe Code Validation
//!
//! Provides sophisticated validation of unsafe code with detailed error messages,
//! transmute support, and lint rules for common unsafe patterns.

use crate::lowering::HirType;
use std::fmt;

/// Enhanced unsafe error with location context
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsafeErrorEnhanced {
    pub message: String,
    pub context: Option<String>,  // e.g., "in function foo()"
    pub suggestion: Option<String>, // e.g., "wrap in unsafe {}"
}

impl fmt::Display for UnsafeErrorEnhanced {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(ctx) = &self.context {
            write!(f, " ({})", ctx)?;
        }
        if let Some(sug) = &self.suggestion {
            write!(f, "\nSuggestion: {}", sug)?;
        }
        Ok(())
    }
}

pub type UnsafeCheckResultEnhanced<T> = Result<T, UnsafeErrorEnhanced>;

/// Type transmutation validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransmuteValidity {
    /// Safe - types have same size
    Safe,
    /// Requires unsafe - different sizes or incompatible layouts
    RequiresUnsafe,
    /// Invalid - types cannot be transmuted at all
    Invalid,
}

/// Enhanced unsafe checker with better diagnostics
#[derive(Debug)]
pub struct UnsafeCheckerEnhanced {
    unsafe_depth: usize,
    unsafe_functions: std::collections::HashSet<String>,
    errors: Vec<UnsafeErrorEnhanced>,
    current_context: Option<String>,
}

impl UnsafeCheckerEnhanced {
    pub fn new() -> Self {
        UnsafeCheckerEnhanced {
            unsafe_depth: 0,
            unsafe_functions: std::collections::HashSet::new(),
            errors: Vec::new(),
            current_context: None,
        }
    }
    
    /// Set the current context (e.g., function name)
    pub fn set_context(&mut self, context: impl Into<String>) {
        self.current_context = Some(context.into());
    }
    
    /// Clear the current context
    pub fn clear_context(&mut self) {
        self.current_context = None;
    }
    
    /// Register an unsafe function
    pub fn register_unsafe_function(&mut self, name: &str) {
        self.unsafe_functions.insert(name.to_string());
    }
    
    /// Check pointer dereference with enhanced error
    pub fn check_pointer_deref_enhanced(&mut self) -> UnsafeCheckResultEnhanced<()> {
        if !self.is_in_unsafe_context() {
            let error = UnsafeErrorEnhanced {
                message: "cannot dereference raw pointer outside of unsafe block".to_string(),
                context: self.current_context.clone(),
                suggestion: Some("wrap dereference in unsafe { ... }".to_string()),
            };
            self.errors.push(error.clone());
            return Err(error);
        }
        Ok(())
    }
    
    /// Check unsafe function call with enhanced error
    pub fn check_unsafe_function_call_enhanced(
        &mut self,
        func_name: &str,
    ) -> UnsafeCheckResultEnhanced<()> {
        if !self.is_in_unsafe_context() {
            let error = UnsafeErrorEnhanced {
                message: format!("cannot call unsafe function '{}' outside of unsafe block", func_name),
                context: self.current_context.clone(),
                suggestion: Some("wrap call in unsafe { ... }".to_string()),
            };
            self.errors.push(error.clone());
            return Err(error);
        }
        Ok(())
    }
    
    /// Validate transmutation between two types
    pub fn validate_transmute(&mut self, from: &HirType, to: &HirType) -> UnsafeCheckResultEnhanced<TransmuteValidity> {
        let validity = self.check_transmute_validity(from, to);
        
        match validity {
            TransmuteValidity::Safe => Ok(validity),
            TransmuteValidity::RequiresUnsafe => {
                if !self.is_in_unsafe_context() {
                    let error = UnsafeErrorEnhanced {
                        message: format!("transmute from {} to {} requires unsafe block", 
                            self.type_name(from), self.type_name(to)),
                        context: self.current_context.clone(),
                        suggestion: Some("wrap transmute in unsafe { ... }".to_string()),
                    };
                    self.errors.push(error.clone());
                    return Err(error);
                }
                Ok(validity)
            }
            TransmuteValidity::Invalid => {
                let error = UnsafeErrorEnhanced {
                    message: format!("transmute from {} to {} is invalid", 
                        self.type_name(from), self.type_name(to)),
                    context: self.current_context.clone(),
                    suggestion: Some(format!("use a safe cast or conversion instead of transmute")),
                };
                self.errors.push(error.clone());
                Err(error)
            }
        }
    }
    
    /// Check transmute validity (internal)
    fn check_transmute_validity(&self, from: &HirType, to: &HirType) -> TransmuteValidity {
        // Check if types are exactly equal
        if self.types_equal(from, to) {
            return TransmuteValidity::Safe;
        }
        
        // Pointer-to-pointer = requires unsafe but valid
        if matches!(from, HirType::Pointer(_)) && matches!(to, HirType::Pointer(_)) {
            return TransmuteValidity::RequiresUnsafe;
        }
        
        // Numeric types of same size = requires unsafe
        match (from, to) {
            (HirType::Int32, HirType::Float64) |
            (HirType::Float64, HirType::Int32) => TransmuteValidity::RequiresUnsafe,
            
            (HirType::Int64, HirType::Int64) => TransmuteValidity::Safe,
            (HirType::Float64, HirType::Float64) => TransmuteValidity::Safe,
            
            _ => TransmuteValidity::Invalid,
        }
    }
    
    /// Check if two types are exactly equal
    fn types_equal(&self, a: &HirType, b: &HirType) -> bool {
        match (a, b) {
            (HirType::Int32, HirType::Int32) => true,
            (HirType::Int64, HirType::Int64) => true,
            (HirType::Float64, HirType::Float64) => true,
            (HirType::Bool, HirType::Bool) => true,
            (HirType::String, HirType::String) => true,
            (HirType::Named(a), HirType::Named(b)) => a == b,
            (HirType::Reference(a), HirType::Reference(b)) => self.types_equal(a, b),
            (HirType::MutableReference(a), HirType::MutableReference(b)) => self.types_equal(a, b),
            (HirType::Pointer(a), HirType::Pointer(b)) => self.types_equal(a, b),
            (HirType::Tuple(a), HirType::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| self.types_equal(x, y))
            }
            (HirType::Unknown, HirType::Unknown) => true,
            _ => false,
        }
    }
    
    /// Get human-readable type name
    fn type_name(&self, ty: &HirType) -> String {
        match ty {
            HirType::Int32 => "i32".to_string(),
            HirType::Int64 => "i64".to_string(),
            HirType::UInt32 => "u32".to_string(),
            HirType::UInt64 => "u64".to_string(),
            HirType::USize => "usize".to_string(),
            HirType::ISize => "isize".to_string(),
            HirType::Float64 => "f64".to_string(),
            HirType::Bool => "bool".to_string(),
            HirType::Char => "char".to_string(),
            HirType::String => "str".to_string(),
            HirType::Named(n) => n.clone(),
            HirType::Reference(inner) => format!("&{}", self.type_name(inner)),
            HirType::MutableReference(inner) => format!("&mut {}", self.type_name(inner)),
            HirType::Pointer(inner) => format!("*{}", self.type_name(inner)),
            HirType::Array { element_type, size } => {
                let size_str = size.map(|s| s.to_string()).unwrap_or_else(|| "?".to_string());
                format!("[{}; {}]", self.type_name(element_type), size_str)
            }
            HirType::Function { .. } => "fn(...)".to_string(),
            HirType::Closure { .. } => "closure".to_string(),
            HirType::Tuple(types) => {
                let type_strs: Vec<_> = types.iter().map(|t| self.type_name(t)).collect();
                format!("({})", type_strs.join(", "))
            }
            HirType::Range => "Range".to_string(),
            HirType::Unknown => "?".to_string(),
        }
    }
    
    /// Enter unsafe context
    pub fn enter_unsafe_context(&mut self) {
        self.unsafe_depth += 1;
    }
    
    /// Exit unsafe context
    pub fn exit_unsafe_context(&mut self) {
        if self.unsafe_depth > 0 {
            self.unsafe_depth -= 1;
        }
    }
    
    /// Check if in unsafe context
    pub fn is_in_unsafe_context(&self) -> bool {
        self.unsafe_depth > 0
    }
    
    /// Get all errors
    pub fn errors(&self) -> &[UnsafeErrorEnhanced] {
        &self.errors
    }
    
    /// Report detailed diagnostics
    pub fn report_diagnostics(&self) -> String {
        if self.errors.is_empty() {
            "No unsafe violations found.".to_string()
        } else {
            let mut report = format!("Found {} unsafe violation(s):\n\n", self.errors.len());
            for (i, error) in self.errors.iter().enumerate() {
                report.push_str(&format!("{}. {}\n", i + 1, error));
                report.push('\n');
            }
            report
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_error_with_context() {
        let mut checker = UnsafeCheckerEnhanced::new();
        checker.set_context("in function main()");
        
        let result = checker.check_pointer_deref_enhanced();
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.context.is_some());
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_transmute_same_type_safe() {
        let mut checker = UnsafeCheckerEnhanced::new();
        let i32_type = HirType::Int32;
        
        let result = checker.validate_transmute(&i32_type, &i32_type);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TransmuteValidity::Safe);
    }

    #[test]
    fn test_transmute_pointer_types_requires_unsafe() {
        let mut checker = UnsafeCheckerEnhanced::new();
        let ptr1 = HirType::Pointer(Box::new(HirType::Int32));
        let ptr2 = HirType::Pointer(Box::new(HirType::Float64));
        
        let result = checker.validate_transmute(&ptr1, &ptr2);
        assert!(result.is_err()); // Outside unsafe - should fail
        
        checker.enter_unsafe_context();
        let result = checker.validate_transmute(&ptr1, &ptr2);
        assert!(result.is_ok()); // Inside unsafe - should succeed
        assert_eq!(result.unwrap(), TransmuteValidity::RequiresUnsafe);
    }

    #[test]
    fn test_transmute_invalid_types() {
        let mut checker = UnsafeCheckerEnhanced::new();
        checker.enter_unsafe_context();
        
        let int_type = HirType::Int32;
        let str_type = HirType::String;
        
        let result = checker.validate_transmute(&int_type, &str_type);
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert!(e.message.contains("invalid"));
        }
    }

    #[test]
    fn test_diagnostics_report() {
        let mut checker = UnsafeCheckerEnhanced::new();
        
        // Generate some errors
        let _ = checker.check_pointer_deref_enhanced();
        let _ = checker.check_unsafe_function_call_enhanced("bad_func");
        
        let report = checker.report_diagnostics();
        assert!(report.contains("2 unsafe violation"));
    }

    #[test]
    fn test_type_name_display() {
        let checker = UnsafeCheckerEnhanced::new();
        assert_eq!(checker.type_name(&HirType::Int32), "i32");
        assert_eq!(checker.type_name(&HirType::String), "str");
    }
}
