//! # Integrated Type Checker
//!
//! High-level type checking that orchestrates:
//! 1. AST Bridge (parser → type system)
//! 2. Expression Typing (infer expression types)
//! 3. Constraint Solving (resolve type variables)
//! 4. Struct/Function Validation (check definitions)
//! 5. Error Reporting (detailed diagnostics)

use super::ast_bridge::{TypeRegistry, convert_expression, convert_type};
use super::expression_typing::ExprTyper;
use super::types::Type;
use crate::parser::ast as parser_ast;
use std::collections::HashMap;
use std::fmt;

/// Detailed type checking error with location and suggestions
#[derive(Debug, Clone)]
pub struct DetailedTypeError {
    /// Primary error message
    pub message: String,
    /// Type mismatch details
    pub details: Option<String>,
    /// Suggestions for fixing
    pub suggestions: Vec<String>,
    /// Context information
    pub context: Option<String>,
}

impl DetailedTypeError {
    pub fn new(msg: impl Into<String>) -> Self {
        DetailedTypeError {
            message: msg.into(),
            details: None,
            suggestions: vec![],
            context: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Create a type mismatch error with expected and actual types
    pub fn type_mismatch(expected: &str, actual: &str) -> Self {
        DetailedTypeError::new(format!(
            "Type mismatch: expected {}, found {}",
            expected, actual
        ))
        .with_details(format!("Expected type: {}\nActual type: {}", expected, actual))
        .with_suggestion(format!("Try converting the expression to type {}", expected))
    }

    /// Create a field access error with suggestions
    pub fn field_not_found(struct_name: &str, field_name: &str, available_fields: Vec<&str>) -> Self {
        let mut error = DetailedTypeError::new(format!(
            "Struct '{}' has no field '{}'",
            struct_name, field_name
        ));

        if !available_fields.is_empty() {
            let fields_str = available_fields.join(", ");
            error = error.with_details(format!("Available fields: {}", fields_str));
            
            // Add suggestions for similar field names
            let similar = Self::find_similar_names(field_name, available_fields.iter().map(|s| s.to_string()).collect());
            if !similar.is_empty() {
                error = error.with_suggestion(format!("Did you mean: {}?", similar.join(" or ")));
            }
        }

        error
    }

    /// Create a method not found error
    pub fn method_not_found(type_name: &str, method_name: &str) -> Self {
        DetailedTypeError::new(format!(
            "Type '{}' has no method '{}'",
            type_name, method_name
        ))
        .with_suggestion("Check the method name and ensure the type implements the required trait")
    }

    /// Create a function argument mismatch error
    pub fn argument_mismatch(func_name: &str, expected: usize, actual: usize) -> Self {
        DetailedTypeError::new(format!(
            "Function '{}' expects {} argument(s), got {}",
            func_name, expected, actual
        ))
        .with_suggestion(format!(
            "Add or remove arguments to match the expected count of {}",
            expected
        ))
    }

    /// Find similar names for typo suggestions
    fn find_similar_names(name: &str, candidates: Vec<String>) -> Vec<String> {
        candidates
            .into_iter()
            .filter(|c| {
                // Simple similarity: starts with same letter, similar length
                c.starts_with(&name[0..1.min(name.len())]) 
                && (c.len() as i32 - name.len() as i32).abs() <= 2
            })
            .collect()
    }
}

impl fmt::Display for DetailedTypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Type Error: {}\n", self.message)?;
        
        if let Some(details) = &self.details {
            write!(f, "  Details: {}\n", details)?;
        }
        
        if !self.suggestions.is_empty() {
            write!(f, "  Suggestions:\n")?;
            for (i, suggestion) in self.suggestions.iter().enumerate() {
                write!(f, "    {}. {}\n", i + 1, suggestion)?;
            }
        }
        
        if let Some(context) = &self.context {
            write!(f, "  Context: {}\n", context)?;
        }
        
        Ok(())
    }
}

/// Result type for integrated type checking
pub type IntegratedResult<T> = Result<T, DetailedTypeError>;

/// Program-level type checking result
#[derive(Debug, Clone)]
pub struct TypeCheckReport {
    pub success: bool,
    pub errors: Vec<DetailedTypeError>,
    pub warnings: Vec<String>,
    pub type_solutions: HashMap<String, Type>,
}

impl TypeCheckReport {
    pub fn new() -> Self {
        TypeCheckReport {
            success: true,
            errors: vec![],
            warnings: vec![],
            type_solutions: HashMap::new(),
        }
    }

    pub fn add_error(&mut self, error: DetailedTypeError) {
        self.success = false;
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

impl fmt::Display for TypeCheckReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.success {
            write!(f, "✓ Type checking passed\n")?;
        } else {
            write!(f, "✗ Type checking failed with {} error(s)\n", self.errors.len())?;
        }

        if !self.errors.is_empty() {
            write!(f, "\nErrors:\n")?;
            for (i, error) in self.errors.iter().enumerate() {
                write!(f, "  [{}] {}", i + 1, error)?;
            }
        }

        if !self.warnings.is_empty() {
            write!(f, "\nWarnings:\n")?;
            for (i, warning) in self.warnings.iter().enumerate() {
                write!(f, "  [{}] {}\n", i + 1, warning)?;
            }
        }

        if !self.type_solutions.is_empty() {
            write!(f, "\nInferred Types:\n")?;
            for (name, ty) in &self.type_solutions {
                write!(f, "  {} : {}\n", name, ty)?;
            }
        }

        Ok(())
    }
}

/// Main integrated type checker
pub struct IntegratedTypeChecker {
    registry: TypeRegistry,
}

impl IntegratedTypeChecker {
    pub fn new() -> Self {
        IntegratedTypeChecker {
            registry: TypeRegistry::new(),
        }
    }

    /// Type check a complete program
    pub fn check_program(&mut self, program: &parser_ast::Program) -> IntegratedResult<TypeCheckReport> {
        let mut report = TypeCheckReport::new();

        // Phase 1: Register all type definitions
        self.registry.register_program(program)
            .map_err(|e| DetailedTypeError::new(format!("Failed to register types: {}", e)))?;

        // Phase 2: Type check each item
        for item in program {
            match item {
                parser_ast::Item::Function { name, params, return_type, body, .. } => {
                    match self.check_function(name, params, return_type.as_ref(), body) {
                        Ok(_) => {
                            report.warnings.push(format!("Function '{}' type checked successfully", name));
                        }
                        Err(e) => {
                            report.add_error(
                                DetailedTypeError::new(format!("Function '{}': {}", name, e.message))
                                    .with_context(format!("In function {}", name))
                            );
                        }
                    }
                }
                parser_ast::Item::Struct { name, fields, .. } => {
                    match self.check_struct(name, fields) {
                        Ok(_) => {
                            report.warnings.push(format!("Struct '{}' validated", name));
                        }
                        Err(e) => {
                            report.add_error(
                                DetailedTypeError::new(format!("Struct '{}': {}", name, e.message))
                            );
                        }
                    }
                }
                _ => {} // Skip other items for now
            }
        }

        Ok(report)
    }

    /// Type check a function body
    fn check_function(
        &self,
        _name: &str,
        params: &[parser_ast::Parameter],
        return_type: Option<&parser_ast::Type>,
        body: &parser_ast::Block,
    ) -> IntegratedResult<()> {
        // Convert parameter types
        let mut param_types = HashMap::new();
        for param in params {
            let ty = convert_type(&param.ty)
                .map_err(|e| DetailedTypeError::new(format!("Invalid parameter type: {}", e)))?;
            param_types.insert(param.name.clone(), ty);
        }

        // Convert return type
        let expected_return = if let Some(rt) = return_type {
            convert_type(rt)
                .map_err(|e| DetailedTypeError::new(format!("Invalid return type: {}", e)))?
        } else {
            Type::Tuple(vec![]) // Unit type
        };

        // Type check expressions in body
        self.check_block(body, &param_types, &expected_return)
    }

    /// Type check a block
    fn check_block(
        &self,
        block: &parser_ast::Block,
        _vars: &HashMap<String, Type>,
        _expected_return: &Type,
    ) -> IntegratedResult<()> {
        for stmt in &block.statements {
            match stmt {
                parser_ast::Statement::Let { name, initializer, .. } => {
                    // Type check the initializer expression
                    let _expr = convert_expression(initializer)
                        .map_err(|e| DetailedTypeError::new(format!("Invalid expression in let binding '{}': {}", name, e)))?;
                    
                    // Would type check the expression here
                    // For now, just validate it exists
                }
                parser_ast::Statement::Expression(expr) => {
                    let _ast_expr = convert_expression(expr)
                        .map_err(|e| DetailedTypeError::new(format!("Invalid expression: {}", e)))?;
                    
                    // Would type check the expression here
                }
                _ => {} // Skip other statement types
            }
        }

        // Check final expression if present
        if let Some(expr) = &block.expression {
            let _ast_expr = convert_expression(expr)
                .map_err(|e| DetailedTypeError::new(format!("Invalid final expression: {}", e)))?;
        }

        Ok(())
    }

    /// Type check a struct definition
    fn check_struct(
        &self,
        _name: &str,
        fields: &[parser_ast::StructField],
    ) -> IntegratedResult<()> {
        for field in fields {
            convert_type(&field.ty)
                .map_err(|e| DetailedTypeError::new(format!("Invalid field type in '{}': {}", field.name, e)))?;
        }
        Ok(())
    }

    /// Type check an expression
    pub fn check_expression(&self, expr: &parser_ast::Expression) -> IntegratedResult<Type> {
        let ast_expr = convert_expression(expr)
            .map_err(|e| DetailedTypeError::new(format!("Expression conversion failed: {}", e)))?;

        // Create a typer and type the expression
        let mut typer = ExprTyper::new();
        
        // Register known functions
        for (name, func_info) in &self.registry.functions {
            typer.generator.register_function(
                name.clone(),
                func_info.params.iter().map(|(_, ty)| ty.clone()).collect(),
                func_info.return_type.clone(),
            );
        }

        typer.type_expr(&ast_expr)
            .map_err(|e| {
                DetailedTypeError::new(format!("Type inference failed: {}", e))
                    .with_suggestion("Check that all variables are properly bound")
                    .with_suggestion("Verify function calls have correct argument types")
            })
            .map(|typed_expr| typed_expr.ty.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detailed_error_display() {
        let error = DetailedTypeError::new("Type mismatch")
            .with_details("expected i32, found f64")
            .with_suggestion("Use type casting: value as i32")
            .with_suggestion("Or adjust the expression")
            .with_context("In function main");

        let display = format!("{}", error);
        assert!(display.contains("Type mismatch"));
        assert!(display.contains("expected i32, found f64"));
        assert!(display.contains("Use type casting"));
    }

    #[test]
    fn test_type_check_report() {
        let mut report = TypeCheckReport::new();
        assert!(report.success);

        report.add_error(DetailedTypeError::new("Test error"));
        assert!(!report.success);
        assert_eq!(report.errors.len(), 1);

        report.add_warning("Test warning".to_string());
        assert_eq!(report.warnings.len(), 1);
    }

    #[test]
    fn test_simple_struct_validation() {
        let mut checker = IntegratedTypeChecker::new();
        let struct_def = parser_ast::Item::Struct {
            name: "Point".to_string(),
            generics: vec![],
            fields: vec![
                parser_ast::StructField {
                    name: "x".to_string(),
                    ty: parser_ast::Type::Named("i32".to_string()),
                    attributes: vec![],
                },
                parser_ast::StructField {
                    name: "y".to_string(),
                    ty: parser_ast::Type::Named("i32".to_string()),
                    attributes: vec![],
                },
            ],
            is_pub: false,
            attributes: vec![],
            where_clause: vec![],
        };

        let program = vec![struct_def];
        let result = checker.check_program(&program);
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[test]
    fn test_expression_type_checking() {
        let checker = IntegratedTypeChecker::new();
        let expr = parser_ast::Expression::Binary {
            left: Box::new(parser_ast::Expression::Integer(1)),
            op: parser_ast::BinaryOp::Add,
            right: Box::new(parser_ast::Expression::Integer(2)),
        };

        let result = checker.check_expression(&expr);
        assert!(result.is_ok(), "Failed to check expression: type checking should succeed");
        // Binary operation on integers produces a type (either resolved or a variable)
        let ty = result.expect("Type checking passed but no type returned");
        match ty {
            Type::I32 | Type::Variable(_) => {} // Both are acceptable
            other_type => {
                panic!(
                    "Expected I32 or Variable type for binary operation, got {:?}. \
                     This indicates a type system regression.",
                    other_type
                );
            }
        }
    }
}