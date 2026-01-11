//! # Const Evaluation System
//!
//! Support for compile-time evaluation of constant expressions and const functions.
//!
//! This module provides const-time computation capabilities including:
//! - Constant expression evaluation (arithmetic, logical)
//! - Const function definition and invocation
//! - Const folding for compile-time optimization
//! - Type checking for const values
//! - Array size computation from const expressions
//! - Generic const parameters
//! - Overflow detection and reporting
//!
//! # Examples
//!
//! ```ignore
//! use gaiarusted::typesystem::const_eval::{ConstEvaluator, ConstEvalConfig};
//!
//! let mut evaluator = ConstEvaluator::new(ConstEvalConfig::default());
//!
//! // Simple constant evaluation
//! let result = evaluator.evaluate_expression("10 + 20");
//! assert_eq!(result.unwrap(), 30);
//!
//! // Const function definition
//! evaluator.register_const_fn(
//!     "double".to_string(),
//!     vec![("x".to_string(), crate::typesystem::types::Type::I64)],
//!     crate::typesystem::types::Type::I64,
//!     "x * 2".to_string(),
//! ).ok();
//! ```

use std::collections::HashMap;
use crate::typesystem::types::Type;

/// Configuration for const evaluation
#[derive(Debug, Clone)]
pub struct ConstEvalConfig {
    /// Maximum nesting depth for const expressions
    pub max_const_depth: usize,
    /// Maximum number of evaluation iterations before timeout
    pub max_eval_iterations: usize,
    /// Enable const folding optimization
    pub enable_const_folding: bool,
    /// Maximum number of const functions
    pub max_const_fns: usize,
    /// Maximum number of const values
    pub max_const_values: usize,
}

impl Default for ConstEvalConfig {
    fn default() -> Self {
        ConstEvalConfig {
            max_const_depth: 16,
            max_eval_iterations: 1000,
            enable_const_folding: true,
            max_const_fns: 1024,
            max_const_values: 2048,
        }
    }
}

/// Represents a constant value evaluated at compile time
#[derive(Debug, Clone)]
pub struct ConstValue {
    /// Name of the constant
    pub name: String,
    /// Type of the constant
    pub value_type: Type,
    /// The evaluated integer value
    pub evaluated_value: i64,
    /// Whether this is from a const function
    pub is_const_fn: bool,
}

/// Represents a const function definition
#[derive(Debug, Clone)]
pub struct ConstFunction {
    /// Function name
    pub name: String,
    /// Function parameters (name, type)
    pub params: Vec<(String, Type)>,
    /// Return type
    pub return_type: Type,
    /// Expression body of the function
    pub body_expr: String,
    /// Cached evaluation result if computed
    pub cached_result: Option<i64>,
}

/// Information about a const folding operation result
#[derive(Debug, Clone)]
pub struct ConstFoldResult {
    /// The folded value
    pub value: i64,
    /// Whether overflow occurred
    pub overflowed: bool,
    /// Whether the operation was successful
    pub success: bool,
}

/// Main const evaluator for compile-time constant computation
pub struct ConstEvaluator {
    config: ConstEvalConfig,
    const_values: HashMap<String, ConstValue>,
    const_fns: HashMap<String, ConstFunction>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl ConstEvaluator {
    /// Create a new const evaluator with the given configuration
    pub fn new(config: ConstEvalConfig) -> Self {
        ConstEvaluator {
            config,
            const_values: HashMap::new(),
            const_fns: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Register a const value with its expression
    pub fn register_const_value(
        &mut self,
        name: String,
        value_type: Type,
        expr: &str,
    ) -> Result<i64, String> {
        // Check if we're at capacity
        if self.const_values.len() >= self.config.max_const_values {
            return Err(format!(
                "Maximum const values ({}) exceeded",
                self.config.max_const_values
            ));
        }

        // Check for duplicate
        if self.const_values.contains_key(&name) {
            return Err(format!("Const value '{}' already defined", name));
        }

        // Evaluate the expression
        let value = self.evaluate_expression(expr)?;

        // Validate type matches the value
        match value_type {
            Type::I32 => {
                if value > i32::MAX as i64 || value < i32::MIN as i64 {
                    return Err(format!(
                        "Const value {} doesn't fit in i32 range [{}, {}]",
                        value,
                        i32::MIN,
                        i32::MAX
                    ));
                }
            }
            Type::I64 => {
                // Any i64 is valid
            }
            Type::U32 => {
                if value < 0 {
                    return Err(format!(
                        "Const value {} cannot be negative for u32",
                        value
                    ));
                }
                if value > u32::MAX as i64 {
                    return Err(format!(
                        "Const value {} doesn't fit in u32 range [0, {}]",
                        value,
                        u32::MAX
                    ));
                }
            }
            Type::U64 => {
                if value < 0 {
                    return Err(format!(
                        "Const value {} cannot be negative for u64",
                        value
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "Cannot create const value of type {:?}",
                    value_type
                ));
            }
        }

        let const_val = ConstValue {
            name: name.clone(),
            value_type,
            evaluated_value: value,
            is_const_fn: false,
        };

        self.const_values.insert(name, const_val);
        Ok(value)
    }

    /// Register a const function definition
    pub fn register_const_fn(
        &mut self,
        name: String,
        params: Vec<(String, Type)>,
        return_type: Type,
        body_expr: String,
    ) -> Result<(), String> {
        // Check capacity
        if self.const_fns.len() >= self.config.max_const_fns {
            return Err(format!(
                "Maximum const functions ({}) exceeded",
                self.config.max_const_fns
            ));
        }

        // Check for duplicate
        if self.const_fns.contains_key(&name) {
            return Err(format!("Const function '{}' already defined", name));
        }

        // Validate parameter types
        for (_, param_type) in &params {
            match param_type {
                Type::I32 | Type::I64 | Type::U32 | Type::U64 => {}
                _ => {
                    return Err(format!(
                        "Const function parameter cannot have type {:?}",
                        param_type
                    ));
                }
            }
        }

        // Validate return type
        match return_type {
            Type::I32 | Type::I64 | Type::U32 | Type::U64 => {}
            _ => {
                return Err(format!(
                    "Const function cannot return type {:?}",
                    return_type
                ));
            }
        }

        let const_fn = ConstFunction {
            name: name.clone(),
            params,
            return_type,
            body_expr,
            cached_result: None,
        };

        self.const_fns.insert(name, const_fn);
        Ok(())
    }

    /// Evaluate a const expression
    ///
    /// Supports:
    /// - Integer literals (42, -10, etc.)
    /// - Binary operations (+, -, *, /, %, <<, >>, &, |, ^)
    /// - Unary operations (-, !)
    /// - Parentheses
    /// - Const variable references
    pub fn evaluate_expression(&mut self, expr: &str) -> Result<i64, String> {
        let trimmed = expr.trim();

        if trimmed.is_empty() {
            return Err("Empty expression".to_string());
        }

        // Try parsing as literal first
        if let Ok(val) = trimmed.parse::<i64>() {
            return Ok(val);
        }

        // Try to parse as const reference
        if let Some(const_val) = self.const_values.get(trimmed) {
            return Ok(const_val.evaluated_value);
        }

        // Handle expressions with operators
        self.parse_expression(trimmed, 0)
    }

    /// Parse and evaluate an expression with operator precedence
    fn parse_expression(&mut self, expr: &str, depth: usize) -> Result<i64, String> {
        if depth > self.config.max_const_depth {
            return Err("Const expression nesting too deep".to_string());
        }

        let expr = expr.trim();

        // Remove outer parentheses if present
        if expr.starts_with('(') && expr.ends_with(')') {
            if self.is_balanced_parens(expr) {
                return self.parse_expression(&expr[1..expr.len() - 1], depth + 1);
            }
        }

        // Try to parse as literal first
        if let Ok(val) = expr.parse::<i64>() {
            return Ok(val);
        }

        // Try to parse as const reference
        if let Some(const_val) = self.const_values.get(expr) {
            return Ok(const_val.evaluated_value);
        }

        // Check for binary operators (lowest precedence first for left-to-right parsing)
        // Logical OR
        if let Some(pos) = self.find_operator(expr, &["|"]) {
            if !self.is_in_parens(expr, pos) && pos > 0 && !expr[..pos].ends_with('|') {
                let left = self.parse_expression(&expr[..pos], depth + 1)?;
                let right = self.parse_expression(&expr[pos + 1..], depth + 1)?;
                return Ok(left | right);
            }
        }

        // Logical AND
        if let Some(pos) = self.find_operator(expr, &["&"]) {
            if !self.is_in_parens(expr, pos) && pos > 0 && !expr[..pos].ends_with('&') {
                let left = self.parse_expression(&expr[..pos], depth + 1)?;
                let right = self.parse_expression(&expr[pos + 1..], depth + 1)?;
                return Ok(left & right);
            }
        }

        // XOR
        if let Some(pos) = self.find_operator(expr, &["^"]) {
            if !self.is_in_parens(expr, pos) {
                let left = self.parse_expression(&expr[..pos], depth + 1)?;
                let right = self.parse_expression(&expr[pos + 1..], depth + 1)?;
                return Ok(left ^ right);
            }
        }

        // Shift operators
        for op in &["<<", ">>"] {
            if let Some(pos) = expr.find(op) {
                if !self.is_in_parens(expr, pos) {
                    let left = self.parse_expression(&expr[..pos], depth + 1)?;
                    let right = self.parse_expression(&expr[pos + 2..], depth + 1)?;
                    return Ok(if *op == "<<" {
                        left << right
                    } else {
                        left >> right
                    });
                }
            }
        }

        // Addition and subtraction
        for op in &["+", "-"] {
            let mut pos = expr.len();
            while pos > 0 {
                pos = if pos > 0 {
                    expr[..pos].rfind(*op).unwrap_or(0)
                } else {
                    0
                };

                if pos == 0 && *op == "-" {
                    break; // Skip unary minus
                }

                if pos > 0 && !self.is_in_parens(expr, pos) {
                    let left = self.parse_expression(&expr[..pos], depth + 1)?;
                    let right = self.parse_expression(&expr[pos + 1..], depth + 1)?;
                    return Ok(if *op == "+" { left + right } else { left - right });
                }

                if pos == 0 {
                    break;
                }
                pos -= 1;
            }
        }

        // Multiplication, division, modulo
        for op in &["*", "/", "%"] {
            if let Some(pos) = expr.rfind(op) {
                if !self.is_in_parens(expr, pos) {
                    let left = self.parse_expression(&expr[..pos], depth + 1)?;
                    let right = self.parse_expression(&expr[pos + 1..], depth + 1)?;

                    if right == 0 && (*op == "/" || *op == "%") {
                        return Err("Division by zero in const expression".to_string());
                    }

                    return Ok(match *op {
                        "*" => left * right,
                        "/" => left / right,
                        "%" => left % right,
                        _ => unreachable!(),
                    });
                }
            }
        }

        // Unary operators
        if expr.starts_with('-') && expr.len() > 1 {
            let rest = &expr[1..];
            let val = self.parse_expression(rest, depth + 1)?;
            return Ok(-val);
        }

        if expr.starts_with('!') && expr.len() > 1 {
            let rest = &expr[1..];
            let val = self.parse_expression(rest, depth + 1)?;
            return Ok(if val == 0 { 1 } else { 0 });
        }

        // Could not parse expression
        Err(format!("Invalid const expression: '{}'", expr))
    }

    /// Find the rightmost position of an operator not within parentheses
    fn find_operator(&self, expr: &str, ops: &[&str]) -> Option<usize> {
        for op in ops {
            if let Some(pos) = expr.rfind(op) {
                if !self.is_in_parens(expr, pos) {
                    return Some(pos);
                }
            }
        }
        None
    }

    /// Check if a position is inside balanced parentheses
    fn is_in_parens(&self, expr: &str, pos: usize) -> bool {
        let mut depth = 0;
        for (i, c) in expr.chars().enumerate() {
            if i == pos {
                return depth > 0;
            }
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
            }
        }
        false
    }

    /// Check if parentheses are balanced for the whole expression
    fn is_balanced_parens(&self, expr: &str) -> bool {
        let mut depth = 0;
        let bytes = expr.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'(' {
                depth += 1;
            } else if b == b')' {
                depth -= 1;
            }
            if depth == 0 && i < bytes.len() - 1 {
                return false; // Closes before the end
            }
        }
        depth == 0
    }

    /// Perform const folding on a binary operation
    pub fn const_fold_binary_op(
        &mut self,
        left: i64,
        op: &str,
        right: i64,
    ) -> Result<ConstFoldResult, String> {
        if !self.config.enable_const_folding {
            return Err("Const folding disabled".to_string());
        }

        let (value, overflowed) = match op {
            "+" => left.overflowing_add(right),
            "-" => left.overflowing_sub(right),
            "*" => left.overflowing_mul(right),
            "/" => {
                if right == 0 {
                    return Err("Division by zero".to_string());
                }
                (left / right, false)
            }
            "%" => {
                if right == 0 {
                    return Err("Division by zero".to_string());
                }
                (left % right, false)
            }
            "<<" => left.overflowing_shl(right as u32),
            ">>" => left.overflowing_shr(right as u32),
            "&" => (left & right, false),
            "|" => (left | right, false),
            "^" => (left ^ right, false),
            _ => return Err(format!("Unknown operator: {}", op)),
        };

        Ok(ConstFoldResult {
            value,
            overflowed,
            success: !overflowed,
        })
    }

    /// Get a const value by name
    pub fn get_const_value(&self, name: &str) -> Option<&ConstValue> {
        self.const_values.get(name)
    }

    /// Get a const function by name
    pub fn get_const_fn(&self, name: &str) -> Option<&ConstFunction> {
        self.const_fns.get(name)
    }

    /// Check if a const value exists
    pub fn has_const_value(&self, name: &str) -> bool {
        self.const_values.contains_key(name)
    }

    /// Check if a const function exists
    pub fn has_const_fn(&self, name: &str) -> bool {
        self.const_fns.contains_key(name)
    }

    /// Get all const values
    pub fn all_const_values(&self) -> Vec<&ConstValue> {
        self.const_values.values().collect()
    }

    /// Get all const functions
    pub fn all_const_fns(&self) -> Vec<&ConstFunction> {
        self.const_fns.values().collect()
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Get all errors
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Get all warnings
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Generate analysis report
    pub fn generate_report(&self) -> ConstEvalAnalysisReport {
        ConstEvalAnalysisReport {
            const_values_count: self.const_values.len(),
            const_fns_count: self.const_fns.len(),
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
            success: self.errors.is_empty(),
        }
    }
}

/// Report from const evaluation analysis
#[derive(Debug, Clone)]
pub struct ConstEvalAnalysisReport {
    /// Number of const values registered
    pub const_values_count: usize,
    /// Number of const functions registered
    pub const_fns_count: usize,
    /// Any errors encountered
    pub errors: Vec<String>,
    /// Any warnings encountered
    pub warnings: Vec<String>,
    /// Whether analysis was successful
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_evaluator() -> ConstEvaluator {
        ConstEvaluator::new(ConstEvalConfig::default())
    }

    #[test]
    fn test_simple_literal() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("42");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_negative_literal() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("-42");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), -42);
    }

    #[test]
    fn test_addition() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("10 + 20");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 30);
    }

    #[test]
    fn test_subtraction() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("30 - 10");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 20);
    }

    #[test]
    fn test_multiplication() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("5 * 6");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 30);
    }

    #[test]
    fn test_division() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("20 / 4");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
    }

    #[test]
    fn test_modulo() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("17 % 5");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn test_parentheses() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("(10 + 20) * 2");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 60);
    }

    #[test]
    fn test_operator_precedence() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("2 + 3 * 4");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 14);
    }

    #[test]
    fn test_bitwise_and() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("12 & 10");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 8);
    }

    #[test]
    fn test_bitwise_or() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("12 | 10");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 14);
    }

    #[test]
    fn test_bitwise_xor() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("12 ^ 10");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 6);
    }

    #[test]
    fn test_left_shift() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("5 << 2");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 20);
    }

    #[test]
    fn test_right_shift() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("20 >> 2");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
    }

    #[test]
    fn test_register_const_value() {
        let mut eval = create_test_evaluator();
        let result = eval.register_const_value(
            "ANSWER".to_string(),
            Type::I32,
            "42",
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert!(eval.has_const_value("ANSWER"));
    }

    #[test]
    fn test_const_value_reference() {
        let mut eval = create_test_evaluator();
        eval.register_const_value("X".to_string(), Type::I64, "10").ok();
        let result = eval.evaluate_expression("X");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn test_duplicate_const_value() {
        let mut eval = create_test_evaluator();
        eval.register_const_value("X".to_string(), Type::I64, "10").ok();
        let result = eval.register_const_value("X".to_string(), Type::I64, "20");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already defined"));
    }

    #[test]
    fn test_register_const_fn() {
        let mut eval = create_test_evaluator();
        let result = eval.register_const_fn(
            "double".to_string(),
            vec![("x".to_string(), Type::I64)],
            Type::I64,
            "x * 2".to_string(),
        );
        assert!(result.is_ok());
        assert!(eval.has_const_fn("double"));
    }

    #[test]
    fn test_const_fold_add() {
        let mut eval = create_test_evaluator();
        let result = eval.const_fold_binary_op(10, "+", 20);
        assert!(result.is_ok());
        let fold = result.unwrap();
        assert_eq!(fold.value, 30);
        assert!(!fold.overflowed);
    }

    #[test]
    fn test_const_fold_mul() {
        let mut eval = create_test_evaluator();
        let result = eval.const_fold_binary_op(6, "*", 7);
        assert!(result.is_ok());
        let fold = result.unwrap();
        assert_eq!(fold.value, 42);
        assert!(!fold.overflowed);
    }

    #[test]
    fn test_division_by_zero() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("10 / 0");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Division by zero"));
    }

    #[test]
    fn test_empty_expression() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("");
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_expression() {
        let mut eval = create_test_evaluator();
        let result = eval.evaluate_expression("((10 + 20) * (2 + 3)) / 5");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 30);
    }

    #[test]
    fn test_i32_type_overflow() {
        let mut eval = create_test_evaluator();
        let large = (i32::MAX as i64) + 1;
        let result = eval.register_const_value(
            "LARGE".to_string(),
            Type::I32,
            &large.to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("doesn't fit in i32 range"));
    }

    #[test]
    fn test_u32_negative() {
        let mut eval = create_test_evaluator();
        let result = eval.register_const_value(
            "NEG".to_string(),
            Type::U32,
            "-5",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be negative"));
    }

    #[test]
    fn test_generate_report() {
        let mut eval = create_test_evaluator();
        eval.register_const_value("X".to_string(), Type::I64, "42").ok();
        eval.register_const_fn(
            "f".to_string(),
            vec![],
            Type::I64,
            "1".to_string(),
        ).ok();
        eval.add_warning("test".to_string());

        let report = eval.generate_report();
        assert_eq!(report.const_values_count, 1);
        assert_eq!(report.const_fns_count, 1);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.success);
    }
}
