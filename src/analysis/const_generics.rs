//! # Const Generics Support System
//!
//! Advanced const generic parameter handling:
//! - Const generic parameter definitions and bounds
//! - Const value evaluation at compile time
//! - Const generic monomorphization
//! - Const expression validation
//! - Const generic specialization

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstValue {
    Integer(i64),
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone)]
pub struct ConstGenericParam {
    pub name: String,
    pub ty: String,
    pub default: Option<ConstValue>,
}

#[derive(Debug, Clone)]
pub struct ConstGenericContext {
    pub params: Vec<ConstGenericParam>,
    pub values: HashMap<String, ConstValue>,
}

pub struct ConstGenericsEngine {
    contexts: HashMap<String, ConstGenericContext>,
    const_expressions: HashMap<String, String>,
    evaluation_cache: HashMap<String, ConstValue>,
}

impl ConstGenericsEngine {
    pub fn new() -> Self {
        ConstGenericsEngine {
            contexts: HashMap::new(),
            const_expressions: HashMap::new(),
            evaluation_cache: HashMap::new(),
        }
    }

    pub fn register_context(&mut self, name: String, context: ConstGenericContext) {
        self.contexts.insert(name, context);
    }

    pub fn add_const_param(
        &mut self,
        context_name: &str,
        param: ConstGenericParam,
    ) -> Result<(), String> {
        let ctx = self.contexts.get_mut(context_name)
            .ok_or(format!("Context {} not found", context_name))?;

        ctx.params.push(param);
        Ok(())
    }

    pub fn evaluate_const_expr(&mut self, expr: &str) -> Result<ConstValue, String> {
        if let Some(cached) = self.evaluation_cache.get(expr) {
            return Ok(cached.clone());
        }

        let result = self.evaluate_expr_internal(expr)?;
        self.evaluation_cache.insert(expr.to_string(), result.clone());
        Ok(result)
    }

    fn evaluate_expr_internal(&self, expr: &str) -> Result<ConstValue, String> {
        let expr = expr.trim();

        if let Ok(n) = expr.parse::<i64>() {
            return Ok(ConstValue::Integer(n));
        }

        if expr == "true" {
            return Ok(ConstValue::Bool(true));
        }

        if expr == "false" {
            return Ok(ConstValue::Bool(false));
        }

        if expr.starts_with('"') && expr.ends_with('"') {
            return Ok(ConstValue::String(expr[1..expr.len()-1].to_string()));
        }

        if expr.contains('+') {
            let parts: Vec<&str> = expr.split('+').collect();
            if parts.len() == 2 {
                let left = self.evaluate_expr_internal(parts[0])?;
                let right = self.evaluate_expr_internal(parts[1])?;

                if let (ConstValue::Integer(l), ConstValue::Integer(r)) = (left, right) {
                    return Ok(ConstValue::Integer(l + r));
                }
            }
        }

        if expr.contains('*') {
            let parts: Vec<&str> = expr.split('*').collect();
            if parts.len() == 2 {
                let left = self.evaluate_expr_internal(parts[0])?;
                let right = self.evaluate_expr_internal(parts[1])?;

                if let (ConstValue::Integer(l), ConstValue::Integer(r)) = (left, right) {
                    return Ok(ConstValue::Integer(l * r));
                }
            }
        }

        Err(format!("Cannot evaluate const expression: {}", expr))
    }

    pub fn bind_const_value(
        &mut self,
        context_name: &str,
        param_name: &str,
        value: ConstValue,
    ) -> Result<(), String> {
        let param_type = {
            let ctx = self.contexts.get(context_name)
                .ok_or(format!("Context {} not found", context_name))?;

            let param = ctx.params.iter().find(|p| p.name == param_name)
                .ok_or(format!("Parameter {} not found", param_name))?;

            param.ty.clone()
        };

        if self.validate_const_value(&param_type, &value)? {
            let ctx = self.contexts.get_mut(context_name).unwrap();
            ctx.values.insert(param_name.to_string(), value);
            Ok(())
        } else {
            Err(format!("Value does not match type {}", param_type))
        }
    }

    fn validate_const_value(&self, ty: &str, value: &ConstValue) -> Result<bool, String> {
        match value {
            ConstValue::Integer(_) => Ok(ty == "i32" || ty == "i64" || ty == "usize"),
            ConstValue::Bool(_) => Ok(ty == "bool"),
            ConstValue::String(_) => Ok(ty == "str" || ty == "String"),
        }
    }

    pub fn get_const_value(
        &self,
        context_name: &str,
        param_name: &str,
    ) -> Result<ConstValue, String> {
        let ctx = self.contexts.get(context_name)
            .ok_or(format!("Context {} not found", context_name))?;

        ctx.values.get(param_name)
            .cloned()
            .or_else(|| {
                ctx.params.iter()
                    .find(|p| p.name == param_name)
                    .and_then(|p| p.default.clone())
            })
            .ok_or(format!("No value bound for parameter {}", param_name))
    }

    pub fn validate_const_parameters(&self, context_name: &str) -> Result<(), String> {
        let ctx = self.contexts.get(context_name)
            .ok_or(format!("Context {} not found", context_name))?;

        for param in &ctx.params {
            if !ctx.values.contains_key(&param.name) && param.default.is_none() {
                return Err(format!(
                    "Required const parameter {} has no value or default",
                    param.name
                ));
            }
        }

        Ok(())
    }

    pub fn monomorphize_const_generic(
        &mut self,
        context_name: &str,
        concrete_values: &HashMap<String, ConstValue>,
    ) -> Result<String, String> {
        let ctx = self.contexts.get(context_name)
            .ok_or(format!("Context {} not found", context_name))?
            .clone();

        for (param_name, value) in concrete_values {
            if !ctx.params.iter().any(|p| &p.name == param_name) {
                return Err(format!("Unknown const parameter: {}", param_name));
            }

            self.validate_const_value(
                &ctx.params.iter()
                    .find(|p| &p.name == param_name)
                    .unwrap()
                    .ty,
                value,
            )?;
        }

        let mut mono_name = context_name.to_string();
        for (name, value) in concrete_values {
            mono_name.push_str(&format!("_{}_{:?}", name, value));
        }

        Ok(mono_name)
    }

    pub fn register_const_expr(&mut self, expr_id: String, expr: String) {
        self.const_expressions.insert(expr_id, expr);
    }

    pub fn get_const_expr(&self, expr_id: &str) -> Option<String> {
        self.const_expressions.get(expr_id).cloned()
    }

    pub fn specialize_const_generic(
        &mut self,
        context_name: &str,
        specializations: &[(String, ConstValue)],
    ) -> Result<String, String> {
        let ctx = self.contexts.get(context_name)
            .ok_or(format!("Context {} not found", context_name))?;

        let mut specialized_name = context_name.to_string();

        for (param_name, value) in specializations {
            if !ctx.params.iter().any(|p| &p.name == param_name) {
                return Err(format!("Unknown parameter: {}", param_name));
            }

            specialized_name.push_str(&format!("_{:?}", value));
        }

        Ok(specialized_name)
    }

    pub fn collect_const_params(&self, context_name: &str) -> Result<Vec<String>, String> {
        let ctx = self.contexts.get(context_name)
            .ok_or(format!("Context {} not found", context_name))?;

        Ok(ctx.params.iter().map(|p| p.name.clone()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_context() {
        let mut engine = ConstGenericsEngine::new();
        let context = ConstGenericContext {
            params: vec![],
            values: HashMap::new(),
        };

        engine.register_context("Array".to_string(), context);
        assert_eq!(engine.contexts.len(), 1);
    }

    #[test]
    fn test_add_const_param() {
        let mut engine = ConstGenericsEngine::new();
        let context = ConstGenericContext {
            params: vec![],
            values: HashMap::new(),
        };

        engine.register_context("Array".to_string(), context);

        let param = ConstGenericParam {
            name: "N".to_string(),
            ty: "usize".to_string(),
            default: None,
        };

        assert!(engine.add_const_param("Array", param).is_ok());
    }

    #[test]
    fn test_evaluate_integer_expr() {
        let mut engine = ConstGenericsEngine::new();
        let result = engine.evaluate_const_expr("42");
        assert_eq!(result.unwrap(), ConstValue::Integer(42));
    }

    #[test]
    fn test_evaluate_bool_expr() {
        let mut engine = ConstGenericsEngine::new();
        let result = engine.evaluate_const_expr("true");
        assert_eq!(result.unwrap(), ConstValue::Bool(true));
    }

    #[test]
    fn test_evaluate_string_expr() {
        let mut engine = ConstGenericsEngine::new();
        let result = engine.evaluate_const_expr("\"hello\"");
        assert_eq!(result.unwrap(), ConstValue::String("hello".to_string()));
    }

    #[test]
    fn test_evaluate_addition() {
        let mut engine = ConstGenericsEngine::new();
        let result = engine.evaluate_const_expr("10 + 20");
        assert_eq!(result.unwrap(), ConstValue::Integer(30));
    }

    #[test]
    fn test_evaluate_multiplication() {
        let mut engine = ConstGenericsEngine::new();
        let result = engine.evaluate_const_expr("5 * 6");
        assert_eq!(result.unwrap(), ConstValue::Integer(30));
    }

    #[test]
    fn test_bind_const_value() {
        let mut engine = ConstGenericsEngine::new();
        let context = ConstGenericContext {
            params: vec![ConstGenericParam {
                name: "N".to_string(),
                ty: "usize".to_string(),
                default: None,
            }],
            values: HashMap::new(),
        };

        engine.register_context("Array".to_string(), context);

        assert!(engine.bind_const_value("Array", "N", ConstValue::Integer(10)).is_ok());
    }

    #[test]
    fn test_get_const_value() {
        let mut engine = ConstGenericsEngine::new();
        let mut values = HashMap::new();
        values.insert("N".to_string(), ConstValue::Integer(10));

        let context = ConstGenericContext {
            params: vec![],
            values,
        };

        engine.register_context("Array".to_string(), context);

        let value = engine.get_const_value("Array", "N");
        assert_eq!(value.unwrap(), ConstValue::Integer(10));
    }

    #[test]
    fn test_validate_const_parameters() {
        let mut engine = ConstGenericsEngine::new();
        let mut values = HashMap::new();
        values.insert("N".to_string(), ConstValue::Integer(10));

        let context = ConstGenericContext {
            params: vec![ConstGenericParam {
                name: "N".to_string(),
                ty: "usize".to_string(),
                default: None,
            }],
            values,
        };

        engine.register_context("Array".to_string(), context);

        assert!(engine.validate_const_parameters("Array").is_ok());
    }

    #[test]
    fn test_monomorphize_const_generic() {
        let mut engine = ConstGenericsEngine::new();
        let context = ConstGenericContext {
            params: vec![ConstGenericParam {
                name: "N".to_string(),
                ty: "usize".to_string(),
                default: None,
            }],
            values: HashMap::new(),
        };

        engine.register_context("Array".to_string(), context);

        let mut concrete_values = HashMap::new();
        concrete_values.insert("N".to_string(), ConstValue::Integer(32));

        let mono = engine.monomorphize_const_generic("Array", &concrete_values);
        assert!(mono.is_ok());
    }

    #[test]
    fn test_register_const_expr() {
        let mut engine = ConstGenericsEngine::new();
        engine.register_const_expr("CAPACITY".to_string(), "1024".to_string());

        assert!(engine.get_const_expr("CAPACITY").is_some());
    }

    #[test]
    fn test_specialize_const_generic() {
        let mut engine = ConstGenericsEngine::new();
        let context = ConstGenericContext {
            params: vec![ConstGenericParam {
                name: "N".to_string(),
                ty: "usize".to_string(),
                default: None,
            }],
            values: HashMap::new(),
        };

        engine.register_context("Array".to_string(), context);

        let specs = vec![("N".to_string(), ConstValue::Integer(32))];
        let specialized = engine.specialize_const_generic("Array", &specs);
        assert!(specialized.is_ok());
    }

    #[test]
    fn test_collect_const_params() {
        let mut engine = ConstGenericsEngine::new();
        let context = ConstGenericContext {
            params: vec![
                ConstGenericParam {
                    name: "N".to_string(),
                    ty: "usize".to_string(),
                    default: None,
                },
                ConstGenericParam {
                    name: "M".to_string(),
                    ty: "usize".to_string(),
                    default: None,
                },
            ],
            values: HashMap::new(),
        };

        engine.register_context("Matrix".to_string(), context);

        let params = engine.collect_const_params("Matrix").unwrap();
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_const_value_caching() {
        let mut engine = ConstGenericsEngine::new();
        let _result1 = engine.evaluate_const_expr("50");
        assert!(!engine.evaluation_cache.is_empty());
    }

    #[test]
    fn test_default_const_value() {
        let mut engine = ConstGenericsEngine::new();
        let context = ConstGenericContext {
            params: vec![ConstGenericParam {
                name: "N".to_string(),
                ty: "usize".to_string(),
                default: Some(ConstValue::Integer(16)),
            }],
            values: HashMap::new(),
        };

        engine.register_context("Array".to_string(), context);

        let value = engine.get_const_value("Array", "N");
        assert_eq!(value.unwrap(), ConstValue::Integer(16));
    }
}
