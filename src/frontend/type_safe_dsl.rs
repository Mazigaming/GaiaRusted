//! # Type-Safe DSL Support (Task 5.14)
//!
//! Provides mechanisms for creating type-safe domain-specific languages with
//! compile-time verification and safe composition.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum DslValue {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    List(Vec<DslValue>),
    Map(HashMap<String, DslValue>),
    Custom(String, Box<DslValue>),
}

impl DslValue {
    pub fn type_name(&self) -> &str {
        match self {
            DslValue::Integer(_) => "i64",
            DslValue::Float(_) => "f64",
            DslValue::String(_) => "String",
            DslValue::Bool(_) => "bool",
            DslValue::List(_) => "List",
            DslValue::Map(_) => "Map",
            DslValue::Custom(name, _) => name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeSignature {
    pub params: Vec<(String, String)>,
    pub return_type: String,
}

#[derive(Debug, Clone)]
pub struct DslBuilder {
    functions: HashMap<String, TypeSignature>,
    type_checks: Vec<String>,
}

impl DslBuilder {
    pub fn new() -> Self {
        DslBuilder {
            functions: HashMap::new(),
            type_checks: Vec::new(),
        }
    }

    pub fn register_function(&mut self, name: String, sig: TypeSignature) {
        self.functions.insert(name, sig);
    }

    pub fn get_function(&self, name: &str) -> Option<&TypeSignature> {
        self.functions.get(name)
    }

    pub fn check_call(&mut self, name: &str, args: &[&str]) -> Result<String, String> {
        if let Some(sig) = self.functions.get(name) {
            if sig.params.len() != args.len() {
                return Err(format!(
                    "Function {} expects {} args, got {}",
                    name,
                    sig.params.len(),
                    args.len()
                ));
            }

            for (i, (_, expected_type)) in sig.params.iter().enumerate() {
                if args[i] != expected_type {
                    return Err(format!(
                        "Argument {} to {} has wrong type: expected {}, got {}",
                        i, name, expected_type, args[i]
                    ));
                }
            }

            self.type_checks.push(format!("Checked call to {}", name));
            Ok(sig.return_type.clone())
        } else {
            Err(format!("Unknown function: {}", name))
        }
    }

    pub fn type_checks(&self) -> &[String] {
        &self.type_checks
    }
}

#[derive(Debug)]
pub struct DslCompiler {
    builder: DslBuilder,
    expressions: Vec<String>,
}

impl DslCompiler {
    pub fn new() -> Self {
        DslCompiler {
            builder: DslBuilder::new(),
            expressions: Vec::new(),
        }
    }

    pub fn register_builtin(&mut self, name: &str) {
        let sig = match name {
            "add" => TypeSignature {
                params: vec![("a".to_string(), "i64".to_string()), ("b".to_string(), "i64".to_string())],
                return_type: "i64".to_string(),
            },
            "concat" => TypeSignature {
                params: vec![("a".to_string(), "String".to_string()), ("b".to_string(), "String".to_string())],
                return_type: "String".to_string(),
            },
            "length" => TypeSignature {
                params: vec![("s".to_string(), "String".to_string())],
                return_type: "i64".to_string(),
            },
            "is_empty" => TypeSignature {
                params: vec![("s".to_string(), "String".to_string())],
                return_type: "bool".to_string(),
            },
            _ => return,
        };
        self.builder.register_function(name.to_string(), sig);
    }

    pub fn register_custom_function(&mut self, name: String, sig: TypeSignature) {
        self.builder.register_function(name, sig);
    }

    pub fn compile_call(&mut self, name: &str, args: &[&str]) -> Result<String, String> {
        self.builder.check_call(name, args)
    }

    pub fn add_expression(&mut self, expr: String) {
        self.expressions.push(expr);
    }

    pub fn get_expressions(&self) -> &[String] {
        &self.expressions
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.expressions.is_empty() {
            Err("No expressions compiled".to_string())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsl_value_integer() {
        let val = DslValue::Integer(42);
        assert_eq!(val.type_name(), "i64");
    }

    #[test]
    fn test_dsl_value_string() {
        let val = DslValue::String("hello".to_string());
        assert_eq!(val.type_name(), "String");
    }

    #[test]
    fn test_dsl_value_list() {
        let val = DslValue::List(vec![DslValue::Integer(1), DslValue::Integer(2)]);
        assert_eq!(val.type_name(), "List");
    }

    #[test]
    fn test_builder_register_function() {
        let mut builder = DslBuilder::new();
        let sig = TypeSignature {
            params: vec![("x".to_string(), "i64".to_string())],
            return_type: "i64".to_string(),
        };
        builder.register_function("square".to_string(), sig);
        assert!(builder.get_function("square").is_some());
    }

    #[test]
    fn test_builder_check_call_success() {
        let mut builder = DslBuilder::new();
        let sig = TypeSignature {
            params: vec![("a".to_string(), "i64".to_string()), ("b".to_string(), "i64".to_string())],
            return_type: "i64".to_string(),
        };
        builder.register_function("add".to_string(), sig);
        let result = builder.check_call("add", &["i64", "i64"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "i64");
    }

    #[test]
    fn test_builder_check_call_wrong_arg_count() {
        let mut builder = DslBuilder::new();
        let sig = TypeSignature {
            params: vec![("a".to_string(), "i64".to_string())],
            return_type: "i64".to_string(),
        };
        builder.register_function("inc".to_string(), sig);
        let result = builder.check_call("inc", &["i64", "i64"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_check_call_wrong_type() {
        let mut builder = DslBuilder::new();
        let sig = TypeSignature {
            params: vec![("a".to_string(), "i64".to_string())],
            return_type: "i64".to_string(),
        };
        builder.register_function("double".to_string(), sig);
        let result = builder.check_call("double", &["String"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_compiler_register_builtin() {
        let mut compiler = DslCompiler::new();
        compiler.register_builtin("add");
        assert!(compiler.builder.get_function("add").is_some());
    }

    #[test]
    fn test_compiler_compile_call() {
        let mut compiler = DslCompiler::new();
        compiler.register_builtin("add");
        let result = compiler.compile_call("add", &["i64", "i64"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compiler_add_expression() {
        let mut compiler = DslCompiler::new();
        compiler.add_expression("x + y".to_string());
        compiler.add_expression("x * y".to_string());
        assert_eq!(compiler.get_expressions().len(), 2);
    }

    #[test]
    fn test_compiler_validate_empty() {
        let compiler = DslCompiler::new();
        assert!(compiler.validate().is_err());
    }

    #[test]
    fn test_compiler_validate_nonempty() {
        let mut compiler = DslCompiler::new();
        compiler.add_expression("some_expr".to_string());
        assert!(compiler.validate().is_ok());
    }

    #[test]
    fn test_type_signature_creation() {
        let sig = TypeSignature {
            params: vec![("x".to_string(), "i32".to_string()), ("y".to_string(), "i32".to_string())],
            return_type: "i32".to_string(),
        };
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.return_type, "i32");
    }

    #[test]
    fn test_dsl_map_value() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), DslValue::String("value".to_string()));
        let val = DslValue::Map(map);
        assert_eq!(val.type_name(), "Map");
    }

    #[test]
    fn test_dsl_custom_value() {
        let val = DslValue::Custom("Point".to_string(), Box::new(DslValue::Integer(42)));
        assert_eq!(val.type_name(), "Point");
    }

    #[test]
    fn test_multiple_builtin_registration() {
        let mut compiler = DslCompiler::new();
        compiler.register_builtin("add");
        compiler.register_builtin("concat");
        compiler.register_builtin("length");
        assert_eq!(compiler.builder.functions.len(), 3);
    }

    #[test]
    fn test_compiler_unknown_function() {
        let mut compiler = DslCompiler::new();
        compiler.register_builtin("add");
        let result = compiler.compile_call("subtract", &["i64", "i64"]);
        assert!(result.is_err());
    }
}
