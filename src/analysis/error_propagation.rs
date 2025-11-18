//! Error Propagation Operator (?)
//!
//! Implements the ? operator for Result and Option types

use std::collections::HashMap;

/// Error propagation expression
#[derive(Debug, Clone)]
pub struct ErrorPropagation {
    pub expr: String,
    pub operator: PropagationOp,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropagationOp {
    QuestionMark,  // ?
}

/// Try expression (Result-based)
#[derive(Debug, Clone)]
pub struct TryExpr {
    pub expr: String,
    pub error_type: String,
}

/// Propagation context
#[derive(Debug, Clone)]
pub struct PropagationContext {
    pub return_type: String,
    pub error_type: String,
    pub in_try_block: bool,
}

/// Error propagation compiler
pub struct PropagationCompiler {
    contexts: Vec<PropagationContext>,
    propagations: HashMap<String, ErrorPropagation>,
}

impl PropagationCompiler {
    /// Create new propagation compiler
    pub fn new() -> Self {
        PropagationCompiler {
            contexts: Vec::new(),
            propagations: HashMap::new(),
        }
    }

    /// Push propagation context
    pub fn push_context(&mut self, context: PropagationContext) {
        self.contexts.push(context);
    }

    /// Pop propagation context
    pub fn pop_context(&mut self) {
        self.contexts.pop();
    }

    /// Get current context
    pub fn current_context(&self) -> Option<&PropagationContext> {
        self.contexts.last()
    }

    /// Check if ? operator can be used
    pub fn can_propagate(&self, expr_type: &str) -> bool {
        if let Some(context) = self.current_context() {
            expr_type.starts_with("Result<") || expr_type.starts_with("Option<")
        } else {
            false
        }
    }

    /// Register propagation
    pub fn register_propagation(&mut self, id: String, prop: ErrorPropagation) {
        self.propagations.insert(id, prop);
    }

    /// Compile ? operator usage
    pub fn compile_propagation(&self, expr: &str) -> Result<String, String> {
        if !self.can_propagate(expr) {
            return Err(format!("Cannot use ? with non-Result/Option type: {}", expr));
        }

        if expr.starts_with("Result<") {
            Ok(self.compile_result_propagation(expr))
        } else {
            Ok(self.compile_option_propagation(expr))
        }
    }

    /// Compile Result propagation
    fn compile_result_propagation(&self, expr: &str) -> String {
        format!(
            "match {} {{\n\
             Ok(val) => val,\n\
             Err(e) => return Err(e.into()),\n\
             }}",
            expr
        )
    }

    /// Compile Option propagation
    fn compile_option_propagation(&self, expr: &str) -> String {
        format!(
            "match {} {{\n\
             Some(val) => val,\n\
             None => return None,\n\
             }}",
            expr
        )
    }

    /// Generate error conversion code
    pub fn generate_error_conversion(from_type: &str, to_type: &str) -> String {
        format!(
            "impl From<{}> for {} {{\n\
             fn from(err: {}) -> Self {{\n\
             {}::from(err)\n\
             }}\n\
             }}\n",
            from_type, to_type, from_type, to_type
        )
    }
}

/// Early return handler
pub struct EarlyReturn {
    pub value: Option<String>,
}

impl EarlyReturn {
    /// Generate early return code
    pub fn generate_early_return(value: Option<&str>) -> String {
        match value {
            Some(v) => format!("return {};", v),
            None => "return;".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propagation_compiler_creation() {
        let compiler = PropagationCompiler::new();
        assert_eq!(compiler.contexts.len(), 0);
    }

    #[test]
    fn test_push_pop_context() {
        let mut compiler = PropagationCompiler::new();
        let context = PropagationContext {
            return_type: "Result<i32, String>".to_string(),
            error_type: "String".to_string(),
            in_try_block: false,
        };
        compiler.push_context(context);
        assert_eq!(compiler.contexts.len(), 1);

        compiler.pop_context();
        assert_eq!(compiler.contexts.len(), 0);
    }

    #[test]
    fn test_can_propagate() {
        let mut compiler = PropagationCompiler::new();
        let context = PropagationContext {
            return_type: "Result<i32, String>".to_string(),
            error_type: "String".to_string(),
            in_try_block: false,
        };
        compiler.push_context(context);

        assert!(compiler.can_propagate("Result<i32, String>"));
        assert!(compiler.can_propagate("Option<i32>"));
        assert!(!compiler.can_propagate("i32"));
    }

    #[test]
    fn test_compile_result_propagation() {
        let compiler = PropagationCompiler::new();
        let code = compiler.compile_result_propagation("some_result()");
        assert!(code.contains("Ok"));
        assert!(code.contains("Err"));
    }

    #[test]
    fn test_compile_option_propagation() {
        let compiler = PropagationCompiler::new();
        let code = compiler.compile_option_propagation("some_option()");
        assert!(code.contains("Some"));
        assert!(code.contains("None"));
    }

    #[test]
    fn test_compile_propagation() {
        let mut compiler = PropagationCompiler::new();
        compiler.push_context(PropagationContext {
            return_type: "Result<i32, String>".to_string(),
            error_type: "String".to_string(),
            in_try_block: false,
        });

        let result = compiler.compile_propagation("Result<i32, String>");
        assert!(result.is_ok());
    }

    #[test]
    fn test_early_return() {
        let code = EarlyReturn::generate_early_return(Some("Ok(42)"));
        assert_eq!(code, "return Ok(42);");

        let code = EarlyReturn::generate_early_return(None);
        assert_eq!(code, "return;");
    }

    #[test]
    fn test_error_conversion_code() {
        let code = PropagationCompiler::generate_error_conversion("IoError", "AppError");
        assert!(code.contains("impl From<IoError>"));
        assert!(code.contains("AppError"));
    }
}
