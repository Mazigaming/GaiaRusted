//! Execute REPL expressions and collect results

/// Result of executing an expression
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// String value
    String(String),
    /// Boolean value
    Bool(bool),
    /// Unit value (empty)
    Unit,
    /// Value from function call
    FunctionResult(String),
}

impl std::fmt::Display for ExecutionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionResult::Int(n) => write!(f, "{}", n),
            ExecutionResult::Float(n) => write!(f, "{}", n),
            ExecutionResult::String(s) => write!(f, "\"{}\"", s),
            ExecutionResult::Bool(b) => write!(f, "{}", b),
            ExecutionResult::Unit => write!(f, "()"),
            ExecutionResult::FunctionResult(s) => write!(f, "{}", s),
        }
    }
}

/// Basic expression executor
/// Handles simple arithmetic and boolean operations
pub struct Executor {
    // Can be extended with variable/function context
}

impl Executor {
    /// Create a new executor
    pub fn new() -> Self {
        Executor {}
    }

    /// Execute a simple arithmetic expression
    pub fn execute_arithmetic(&self, expr: &str) -> Result<ExecutionResult, String> {
        let expr = expr.trim();

        // Try to parse as integer
        if let Ok(n) = expr.parse::<i64>() {
            return Ok(ExecutionResult::Int(n));
        }

        // Try to parse as float
        if let Ok(n) = expr.parse::<f64>() {
            return Ok(ExecutionResult::Float(n));
        }

        // For now, return error for complex expressions
        Err("expression execution not yet fully implemented".to_string())
    }

    /// Execute a boolean expression
    pub fn execute_bool(&self, expr: &str) -> Result<ExecutionResult, String> {
        let expr = expr.trim();

        match expr {
            "true" => Ok(ExecutionResult::Bool(true)),
            "false" => Ok(ExecutionResult::Bool(false)),
            _ => Err("invalid boolean expression".to_string()),
        }
    }

    /// Execute a string literal
    pub fn execute_string(&self, expr: &str) -> Result<ExecutionResult, String> {
        let expr = expr.trim();

        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('\'') && expr.ends_with('\''))
        {
            let content = &expr[1..expr.len() - 1];
            Ok(ExecutionResult::String(content.to_string()))
        } else {
            Err("invalid string literal".to_string())
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_int() {
        let executor = Executor::new();
        let result = executor.execute_arithmetic("42").unwrap();
        assert_eq!(result, ExecutionResult::Int(42));
    }

    #[test]
    fn test_execute_float() {
        let executor = Executor::new();
        let result = executor.execute_arithmetic("3.14").unwrap();
        match result {
            ExecutionResult::Float(n) => assert!((n - 3.14).abs() < 0.01),
            _ => panic!("expected float"),
        }
    }

    #[test]
    fn test_execute_bool_true() {
        let executor = Executor::new();
        let result = executor.execute_bool("true").unwrap();
        assert_eq!(result, ExecutionResult::Bool(true));
    }

    #[test]
    fn test_execute_bool_false() {
        let executor = Executor::new();
        let result = executor.execute_bool("false").unwrap();
        assert_eq!(result, ExecutionResult::Bool(false));
    }

    #[test]
    fn test_execute_string() {
        let executor = Executor::new();
        let result = executor.execute_string("\"hello\"").unwrap();
        assert_eq!(result, ExecutionResult::String("hello".to_string()));
    }

    #[test]
    fn test_execute_string_single_quote() {
        let executor = Executor::new();
        let result = executor.execute_string("'world'").unwrap();
        assert_eq!(result, ExecutionResult::String("world".to_string()));
    }

    #[test]
    fn test_display_int() {
        let result = ExecutionResult::Int(42);
        assert_eq!(result.to_string(), "42");
    }

    #[test]
    fn test_display_string() {
        let result = ExecutionResult::String("hello".to_string());
        assert_eq!(result.to_string(), "\"hello\"");
    }
}
