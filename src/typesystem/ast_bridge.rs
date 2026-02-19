//! # AST Bridge Module
//!
//! Converts the parser's AST types to the type system's internal representations.
//! This module acts as a translator between the parser (Phase 2) and type checker (Phase 4).
//!
//! ## Conversion Flow:
//! - `parser::Expression` → `AstExpr`
//! - `parser::Type` → `type_system::Type`
//! - `parser::Item` (functions/structs) → constraint system definitions
//!
//! ## Key Features:
//! - Tracks known struct definitions for proper type resolution
//! - Handles generic type parameters
//! - Converts function signatures with generics support
//! - Registers struct fields and methods from impl blocks

use crate::parser::ast as parser_ast;
use super::types::{Type, GenericId, StructId};
use super::constraints::{StructDef, FunctionDef};
use super::expression_typing::{AstExpr, AstBinaryOp, AstUnaryOp};
use std::collections::HashMap;

/// Error type for AST conversion
#[derive(Debug, Clone)]
pub struct BridgeError {
    pub message: String,
}

impl BridgeError {
    pub fn new(msg: impl Into<String>) -> Self {
        BridgeError { message: msg.into() }
    }
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Bridge Error: {}", self.message)
    }
}

type BridgeResult<T> = Result<T, BridgeError>;

/// Conversion context that tracks known types and generics
#[derive(Debug, Clone)]
pub struct ConversionContext {
    /// Known struct names and their IDs
    pub known_structs: HashMap<String, StructId>,
    /// Generic parameter bindings (name -> GenericId)
    pub generic_bindings: HashMap<String, GenericId>,
    /// Next available struct ID
    struct_id_counter: usize,
    /// Next available generic ID
    generic_id_counter: usize,
}

impl ConversionContext {
    /// Create a new conversion context
    pub fn new() -> Self {
        ConversionContext {
            known_structs: HashMap::new(),
            generic_bindings: HashMap::new(),
            struct_id_counter: 0,
            generic_id_counter: 0,
        }
    }

    /// Allocate a new struct ID
    pub fn alloc_struct_id(&mut self) -> StructId {
        let id = StructId(self.struct_id_counter);
        self.struct_id_counter += 1;
        id
    }

    /// Allocate a new generic ID
    pub fn alloc_generic_id(&mut self) -> GenericId {
        let id = GenericId(self.generic_id_counter);
        self.generic_id_counter += 1;
        id
    }

    /// Register a struct type
    pub fn register_struct(&mut self, name: String) -> StructId {
        let id = self.alloc_struct_id();
        self.known_structs.insert(name, id);
        id
    }

    /// Register a generic parameter
    pub fn register_generic(&mut self, name: String) -> GenericId {
        let id = self.alloc_generic_id();
        self.generic_bindings.insert(name, id);
        id
    }

    /// Create a child context for generic scope
    pub fn child_scope(&self) -> Self {
        ConversionContext {
            known_structs: self.known_structs.clone(),
            generic_bindings: HashMap::new(),
            struct_id_counter: self.struct_id_counter,
            generic_id_counter: self.generic_id_counter,
        }
    }
}

/// Converts parser's Type to type system's Type (without context)
pub fn convert_type(parser_type: &parser_ast::Type) -> BridgeResult<Type> {
    let mut ctx = ConversionContext::new();
    convert_type_with_context(parser_type, &mut ctx)
}

/// Converts parser's Type to type system's Type (with context)
pub fn convert_type_with_context(parser_type: &parser_ast::Type, ctx: &mut ConversionContext) -> BridgeResult<Type> {
    match parser_type {
        parser_ast::Type::Named(name) => {
            Ok(match name.as_str() {
                "i8" => Type::I8,
                "i16" => Type::I16,
                "i32" => Type::I32,
                "i64" => Type::I64,
                "isize" => Type::Isize,
                "u8" => Type::U8,
                "u16" => Type::U16,
                "u32" => Type::U32,
                "u64" => Type::U64,
                "usize" => Type::Usize,
                "f32" => Type::F32,
                "f64" => Type::F64,
                "bool" => Type::Bool,
                "char" => Type::Char,
                "str" => Type::Str,
                "String" => Type::Unit,
                _ => {
                    if let Some(&struct_id) = ctx.known_structs.get(name) {
                        Type::Struct(struct_id)
                    } else if let Some(&generic_id) = ctx.generic_bindings.get(name) {
                        Type::Generic(generic_id)
                    } else {
                        Type::Unit
                    }
                }
            })
        }
        parser_ast::Type::Reference { lifetime: _, mutable, inner } => {
            let inner_type = convert_type_with_context(inner, ctx)?;
            Ok(Type::Reference {
                lifetime: None,
                mutable: *mutable,
                inner: Box::new(inner_type),
            })
        }
        parser_ast::Type::Pointer { mutable, inner } => {
            let inner_type = convert_type_with_context(inner, ctx)?;
            Ok(Type::RawPointer {
                mutable: *mutable,
                inner: Box::new(inner_type),
            })
        }
        parser_ast::Type::Array { element, size: _ } => {
            let elem_type = convert_type_with_context(element, ctx)?;
            let array_size = 1;
            Ok(Type::Array {
                element: Box::new(elem_type),
                size: array_size,
            })
        }
        parser_ast::Type::Tuple(elements) => {
            let element_types: BridgeResult<Vec<_>> = elements.iter()
                .map(|e| convert_type_with_context(e, ctx))
                .collect();
            Ok(Type::Tuple(element_types?))
        }
        parser_ast::Type::Function { params, return_type, is_unsafe: _, abi: _ } => {
            let param_types: BridgeResult<Vec<_>> = params.iter()
                .map(|p| convert_type_with_context(p, ctx))
                .collect();
            let ret_type = convert_type_with_context(return_type, ctx)?;
            Ok(Type::Function {
                params: param_types?,
                ret: Box::new(ret_type),
            })
        }
        parser_ast::Type::Generic { name, type_args } => {
            if let Some(&generic_id) = ctx.generic_bindings.get(name) {
                if type_args.is_empty() {
                    Ok(Type::Generic(generic_id))
                } else {
                    let _args: BridgeResult<Vec<_>> = type_args.iter()
                        .map(|a| convert_type_with_context(a, ctx))
                        .collect();
                    if let Some(&struct_id) = ctx.known_structs.get(name) {
                        Ok(Type::Struct(struct_id))
                    } else {
                        Ok(Type::Generic(generic_id))
                    }
                }
            } else if let Some(&struct_id) = ctx.known_structs.get(name) {
                Ok(Type::Struct(struct_id))
            } else {
                Ok(Type::Unit)
            }
        }
        parser_ast::Type::TraitObject { bounds: _, lifetime: _ } => {
            Err(BridgeError::new("E084: Trait objects not yet supported - use generic parameters instead"))
        }
        parser_ast::Type::ImplTrait { bounds: _ } => {
            Err(BridgeError::new("E085: Impl trait not yet supported - use concrete return types instead"))
        }
        parser_ast::Type::AssociatedType { ty: _, name: _ } => {
            // For simplicity, treat associated types as unit
            Ok(Type::Unit)
        }
        parser_ast::Type::QualifiedPath { ty: _, trait_name: _, name: _ } => {
            // For simplicity, treat qualified paths as unit
            Ok(Type::Unit)
        }
        parser_ast::Type::Closure { params, return_type } => {
            let param_types: BridgeResult<Vec<_>> = params.iter()
                .map(|p| convert_type_with_context(p, ctx))
                .collect();
            let ret_type = convert_type_with_context(return_type, ctx)?;
            Ok(Type::Function {
                params: param_types?,
                ret: Box::new(ret_type),
            })
        }
        parser_ast::Type::TypeVar(name) => {
            if let Some(&generic_id) = ctx.generic_bindings.get(name) {
                Ok(Type::Generic(generic_id))
            } else {
                Ok(Type::Unit)
            }
        }
        parser_ast::Type::Never => {
            Ok(Type::Never)
        }
    }
}

/// Extract function signature from parser Function item
pub fn extract_function_signature(
    func: &parser_ast::Item,
    ctx: &mut ConversionContext,
) -> BridgeResult<(String, FunctionDef)> {
    match func {
        parser_ast::Item::Function {
            name,
            params,
            return_type,
            generics,
            ..
        } => {
            let func_name = name.clone();
            let mut func_ctx = ctx.child_scope();

            let mut generic_ids = Vec::new();
            for generic in generics {
                if let parser_ast::GenericParam::Type { name: gname, .. } = generic {
                    let gid = func_ctx.register_generic(gname.clone());
                    generic_ids.push(gid);
                }
            }

            let param_types: BridgeResult<Vec<_>> = params
                .iter()
                .map(|p| convert_type_with_context(&p.ty, &mut func_ctx))
                .collect();

            let ret_type = if let Some(rt) = return_type {
                convert_type_with_context(rt, &mut func_ctx)?
            } else {
                Type::Unit
            };

            Ok((
                func_name,
                FunctionDef {
                    param_types: param_types?,
                    return_type: ret_type,
                    generics: generic_ids,
                },
            ))
        }
        _ => Err(BridgeError::new("Expected Function item")),
    }
}

/// Extract struct definition from parser Struct item
pub fn extract_struct_definition(
    struct_item: &parser_ast::Item,
    ctx: &mut ConversionContext,
) -> BridgeResult<(String, StructId, StructDef)> {
    match struct_item {
        parser_ast::Item::Struct {
            name,
            fields,
            generics,
            ..
        } => {
            let struct_name = name.clone();
            let struct_id = ctx.register_struct(struct_name.clone());
            let mut struct_ctx = ctx.child_scope();

            let mut generic_ids = Vec::new();
            for generic in generics {
                if let parser_ast::GenericParam::Type { name: gname, .. } = generic {
                    let gid = struct_ctx.register_generic(gname.clone());
                    generic_ids.push(gid);
                }
            }

            let mut field_types = HashMap::new();
            for field in fields {
                let field_type = convert_type_with_context(&field.ty, &mut struct_ctx)?;
                field_types.insert(field.name.clone(), field_type);
            }

            Ok((
                struct_name,
                struct_id,
                StructDef {
                    id: struct_id,
                    fields: field_types,
                    generics: generic_ids,
                },
            ))
        }
        _ => Err(BridgeError::new("Expected Struct item")),
    }
}

/// Extract and register method from impl block
pub fn extract_methods_from_impl(
    impl_block: &parser_ast::Item,
    ctx: &mut ConversionContext,
) -> BridgeResult<Vec<(String, String, FunctionDef)>> {
    match impl_block {
        parser_ast::Item::Impl {
            trait_name: _,
            struct_name,
            methods,
            ..
        } => {
            let mut method_defs = Vec::new();

            for method_item in methods {
                if let Ok((method_name, func_def)) = extract_function_signature(method_item, ctx) {
                    method_defs.push((struct_name.clone(), method_name, func_def));
                }
            }

            Ok(method_defs)
        }
        _ => Err(BridgeError::new("Expected Impl item")),
    }
}

/// Converts parser's BinaryOp to AstBinaryOp
pub fn convert_binary_op(op: &parser_ast::BinaryOp) -> AstBinaryOp {
    match op {
        parser_ast::BinaryOp::Add => AstBinaryOp::Add,
        parser_ast::BinaryOp::Subtract => AstBinaryOp::Sub,
        parser_ast::BinaryOp::Multiply => AstBinaryOp::Mul,
        parser_ast::BinaryOp::Divide => AstBinaryOp::Div,
        parser_ast::BinaryOp::Modulo => AstBinaryOp::Mod,
        parser_ast::BinaryOp::Equal => AstBinaryOp::Eq,
        parser_ast::BinaryOp::NotEqual => AstBinaryOp::Ne,
        parser_ast::BinaryOp::Less => AstBinaryOp::Lt,
        parser_ast::BinaryOp::LessEq => AstBinaryOp::Le,
        parser_ast::BinaryOp::Greater => AstBinaryOp::Gt,
        parser_ast::BinaryOp::GreaterEq => AstBinaryOp::Ge,
        parser_ast::BinaryOp::And => AstBinaryOp::And,
        parser_ast::BinaryOp::Or => AstBinaryOp::Or,
        parser_ast::BinaryOp::BitwiseAnd => AstBinaryOp::BitwiseAnd,
        parser_ast::BinaryOp::BitwiseOr => AstBinaryOp::BitwiseOr,
        parser_ast::BinaryOp::BitwiseXor => AstBinaryOp::BitwiseXor,
        parser_ast::BinaryOp::LeftShift => AstBinaryOp::LeftShift,
        parser_ast::BinaryOp::RightShift => AstBinaryOp::RightShift,
    }
}

/// Converts parser's UnaryOp to AstUnaryOp
pub fn convert_unary_op(op: &parser_ast::UnaryOp) -> AstUnaryOp {
    match op {
        parser_ast::UnaryOp::Negate => AstUnaryOp::Negate,
        parser_ast::UnaryOp::Not => AstUnaryOp::Not,
        parser_ast::UnaryOp::Dereference => AstUnaryOp::Deref,
        parser_ast::UnaryOp::Reference => AstUnaryOp::Reference,
        parser_ast::UnaryOp::MutableReference => AstUnaryOp::MutableReference,
        parser_ast::UnaryOp::BitwiseNot => AstUnaryOp::BitwiseNot,
    }
}

/// Converts parser's Expression to AstExpr
pub fn convert_expression(expr: &parser_ast::Expression) -> BridgeResult<AstExpr> {
    match expr {
        parser_ast::Expression::Integer(n) => {
            Ok(AstExpr::Integer(*n))
        }
        parser_ast::Expression::Float(f) => {
            Ok(AstExpr::Float(*f))
        }
        parser_ast::Expression::String(s) => {
            Ok(AstExpr::String(s.clone()))
        }
        parser_ast::Expression::Bool(b) => {
            Ok(AstExpr::Bool(*b))
        }
        parser_ast::Expression::Char(_c) => {
            // Character literals not yet in AstExpr, convert to unit
            Ok(AstExpr::Integer(0))
        }
        parser_ast::Expression::Variable(name) => {
            Ok(AstExpr::Variable(name.clone()))
        }
        parser_ast::Expression::FunctionCall { name, args } => {
            let converted_args: BridgeResult<Vec<_>> = args.iter()
                .map(convert_expression)
                .collect();
            Ok(AstExpr::FunctionCall {
                name: name.clone(),
                args: converted_args?,
            })
        }
        parser_ast::Expression::Binary { left, op, right } => {
            let left_expr = convert_expression(left)?;
            let right_expr = convert_expression(right)?;
            let converted_op = convert_binary_op(op);
            Ok(AstExpr::BinaryOp {
                left: Box::new(left_expr),
                op: converted_op,
                right: Box::new(right_expr),
            })
        }
        parser_ast::Expression::Unary { op, operand } => {
            let operand_expr = convert_expression(operand)?;
            let converted_op = convert_unary_op(op);
            Ok(AstExpr::UnaryOp {
                op: converted_op,
                operand: Box::new(operand_expr),
            })
        }
        parser_ast::Expression::Assign { target, value } => {
            let target_expr = convert_expression(target)?;
            let value_expr = convert_expression(value)?;
            Ok(AstExpr::Assign {
                target: Box::new(target_expr),
                value: Box::new(value_expr),
            })
        }
        parser_ast::Expression::Array(elements) => {
            let converted_elements: BridgeResult<Vec<_>> = elements.iter()
                .map(convert_expression)
                .collect();
            Ok(AstExpr::Array(converted_elements?))
        }
        parser_ast::Expression::Tuple(elements) => {
            let converted_elements: BridgeResult<Vec<_>> = elements.iter()
                .map(convert_expression)
                .collect();
            Ok(AstExpr::Tuple(converted_elements?))
        }
        parser_ast::Expression::Block(block) => {
            let mut exprs = Vec::new();
            for stmt in &block.statements {
                match stmt {
                    parser_ast::Statement::Expression(e) => {
                        exprs.push(convert_expression(e)?);
                    }
                    parser_ast::Statement::Let { name, ty: _, mutable: _, initializer, attributes: _, pattern: _ } => {
                        exprs.push(AstExpr::Variable(name.clone()));
                        exprs.push(convert_expression(initializer)?);
                    }
                    _ => {} // Skip other statement types for now
                }
            }
            if let Some(final_expr) = &block.expression {
                exprs.push(convert_expression(final_expr)?);
            }
            if exprs.is_empty() {
                Ok(AstExpr::Integer(0)) // Default to unit-like value
            } else if exprs.len() == 1 {
                // Safe because we checked len() == 1
                Ok(exprs.into_iter().next().expect("Expression vector has length 1"))
            } else {
                Ok(AstExpr::Tuple(exprs))
            }
        }
        parser_ast::Expression::FieldAccess { object, field } => {
            let obj_expr = convert_expression(object)?;
            Ok(AstExpr::FieldAccess {
                object: Box::new(obj_expr),
                field: field.clone(),
            })
        }
        parser_ast::Expression::Index { array, index } => {
            let array_expr = convert_expression(array)?;
            let index_expr = convert_expression(index)?;
            Ok(AstExpr::Index {
                array: Box::new(array_expr),
                index: Box::new(index_expr),
            })
        }
        parser_ast::Expression::If { condition, then_body, else_body: _ } => {
            // Simplified: convert condition and then body, return tuple
            let cond_expr = convert_expression(condition)?;
            let then_expr = convert_block_to_expr(then_body)?;
            Ok(AstExpr::Tuple(vec![cond_expr, then_expr]))
        }
        _ => {
            Err(BridgeError::new(format!("E087: Expression type not yet supported: {:?} - use simpler expressions", expr)))
        }
    }
}

/// Helper to convert a block to a single expression
fn convert_block_to_expr(block: &parser_ast::Block) -> BridgeResult<AstExpr> {
    let mut exprs = Vec::new();
    
    for stmt in &block.statements {
        match stmt {
            parser_ast::Statement::Expression(e) => {
                exprs.push(convert_expression(e)?);
            }
            parser_ast::Statement::Let { name: _, ty: _, mutable: _, initializer, attributes: _, pattern: _ } => {
                exprs.push(convert_expression(initializer)?);
            }
            _ => {}
        }
    }
    
    if let Some(final_expr) = &block.expression {
        exprs.push(convert_expression(final_expr)?);
    }
    
    if exprs.is_empty() {
        Ok(AstExpr::Integer(0))
    } else if exprs.len() == 1 {
        // Safe because we checked len() == 1
        Ok(exprs.into_iter().next().expect("Expression vector has length 1"))
    } else {
        Ok(AstExpr::Tuple(exprs))
    }
}

/// Struct definition information
#[derive(Debug, Clone)]
pub struct StructTypeInfo {
    pub name: String,
    pub fields: HashMap<String, Type>,
}

/// Function definition information
#[derive(Debug, Clone)]
pub struct FunctionTypeInfo {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
}

/// Registry of type definitions extracted from parser AST
/// Provides compatibility with legacy code while using new extraction functions
#[derive(Debug, Clone)]
pub struct TypeRegistry {
    pub structs: HashMap<String, StructTypeInfo>,
    pub functions: HashMap<String, FunctionTypeInfo>,
    conversion_context: ConversionContext,
}

impl TypeRegistry {
    pub fn new() -> Self {
        TypeRegistry {
            structs: HashMap::new(),
            functions: HashMap::new(),
            conversion_context: ConversionContext::new(),
        }
    }

    /// Register all items from a parsed program
    pub fn register_program(&mut self, items: &[parser_ast::Item]) -> BridgeResult<()> {
        for item in items {
            self.register_item(item)?;
        }
        Ok(())
    }

    /// Register a single item (structs must be registered before functions that use them)
    pub fn register_item(&mut self, item: &parser_ast::Item) -> BridgeResult<()> {
        match item {
            parser_ast::Item::Struct { name, fields, .. } => {
                let struct_name = name.clone();
                self.conversion_context.register_struct(struct_name.clone());

                let mut field_types = HashMap::new();
                for field in fields {
                    let field_type = convert_type_with_context(&field.ty, &mut self.conversion_context)?;
                    field_types.insert(field.name.clone(), field_type);
                }

                self.structs.insert(
                    name.clone(),
                    StructTypeInfo {
                        name: name.clone(),
                        fields: field_types,
                    },
                );
                Ok(())
            }
            parser_ast::Item::Function {
                name,
                params,
                return_type,
                ..
            } => {
                let param_types: BridgeResult<Vec<_>> = params
                    .iter()
                    .map(|p| {
                        let ty = convert_type_with_context(&p.ty, &mut self.conversion_context)?;
                        Ok((p.name.clone(), ty))
                    })
                    .collect();

                let ret_type = if let Some(rt) = return_type {
                    convert_type_with_context(rt, &mut self.conversion_context)?
                } else {
                    Type::Unit
                };

                self.functions.insert(
                    name.clone(),
                    FunctionTypeInfo {
                        name: name.clone(),
                        params: param_types?,
                        return_type: ret_type,
                    },
                );
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Get the conversion context (for advanced usage)
    pub fn context(&self) -> &ConversionContext {
        &self.conversion_context
    }

    /// Get mutable conversion context
    pub fn context_mut(&mut self) -> &mut ConversionContext {
        &mut self.conversion_context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_basic_types() {
        assert!(matches!(
            convert_type(&parser_ast::Type::Named("i32".to_string())),
            Ok(Type::I32)
        ));
        assert!(matches!(
            convert_type(&parser_ast::Type::Named("f64".to_string())),
            Ok(Type::F64)
        ));
        assert!(matches!(
            convert_type(&parser_ast::Type::Named("bool".to_string())),
            Ok(Type::Bool)
        ));
    }

    #[test]
    fn test_convert_reference_type() {
        let ref_type = parser_ast::Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(parser_ast::Type::Named("i32".to_string())),
        };
        assert!(matches!(
            convert_type(&ref_type),
            Ok(Type::Reference { mutable: false, .. })
        ));
    }

    #[test]
    fn test_convert_tuple_type() {
        let tuple_type = parser_ast::Type::Tuple(vec![
            parser_ast::Type::Named("i32".to_string()),
            parser_ast::Type::Named("bool".to_string()),
        ]);
        assert!(matches!(
            convert_type(&tuple_type),
            Ok(Type::Tuple(ref types)) if types.len() == 2
        ));
    }

    #[test]
    fn test_convert_simple_expression() {
        let expr = parser_ast::Expression::Integer(42);
        let converted = convert_expression(&expr)
            .expect("Failed to convert integer expression");
        assert_eq!(
            converted,
            AstExpr::Integer(42),
            "Integer expression should convert to AstExpr::Integer"
        );
    }

    #[test]
    fn test_convert_binary_expression() {
        let expr = parser_ast::Expression::Binary {
            left: Box::new(parser_ast::Expression::Integer(1)),
            op: parser_ast::BinaryOp::Add,
            right: Box::new(parser_ast::Expression::Integer(2)),
        };
        let converted = convert_expression(&expr)
            .expect("Failed to convert binary expression. AST bridge should handle parser AST");
        match converted {
            AstExpr::BinaryOp { op, .. } => {
                assert_eq!(op, AstBinaryOp::Add, "Binary operation should convert Add to Add");
            }
            other => {
                panic!(
                    "Expected binary operation expression, but got {:?}. \
                     This indicates an AST bridge conversion regression.",
                    other
                );
            }
        }
    }

    #[test]
    fn test_type_registry() {
        let mut registry = TypeRegistry::new();
        let func = parser_ast::Item::Function {
            name: "add".to_string(),
            generics: vec![],
            params: vec![
                parser_ast::Parameter {
                    name: "a".to_string(),
                    mutable: false,
                    ty: parser_ast::Type::Named("i32".to_string()),
                },
                parser_ast::Parameter {
                    name: "b".to_string(),
                    mutable: false,
                    ty: parser_ast::Type::Named("i32".to_string()),
                },
            ],
            return_type: Some(parser_ast::Type::Named("i32".to_string())),
            body: parser_ast::Block {
                statements: vec![],
                expression: None,
            },
            is_unsafe: false,
            is_async: false,
            is_pub: false,
            attributes: vec![],
            where_clause: vec![],
            abi: None,
        };

        registry.register_item(&func)
            .expect("Failed to register function in type registry");
        assert!(registry.functions.contains_key("add"));
        assert_eq!(registry.functions["add"].params.len(), 2);
    }

    #[test]
    fn test_extract_function_with_generics() {
        let func = parser_ast::Item::Function {
            name: "identity".to_string(),
            generics: vec![parser_ast::GenericParam::Type {
                name: "T".to_string(),
                bounds: vec![],
                default: None,
            }],
            params: vec![parser_ast::Parameter {
                name: "x".to_string(),
                mutable: false,
                ty: parser_ast::Type::TypeVar("T".to_string()),
            }],
            return_type: Some(parser_ast::Type::TypeVar("T".to_string())),
            body: parser_ast::Block {
                statements: vec![],
                expression: None,
            },
            is_unsafe: false,
            is_async: false,
            is_pub: false,
            attributes: vec![],
            where_clause: vec![],
            abi: None,
        };

        let mut ctx = ConversionContext::new();
        let (name, def) = extract_function_signature(&func, &mut ctx)
            .expect("Failed to extract function signature for generic function");
        assert_eq!(name, "identity", "Function name should be 'identity'");
        assert_eq!(def.generics.len(), 1, "Function should have one generic parameter");
    }

    #[test]
    fn test_extract_struct_definition() {
        let struct_item = parser_ast::Item::Struct {
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

        let mut ctx = ConversionContext::new();
        let (name, _id, def) = extract_struct_definition(&struct_item, &mut ctx)
            .expect("Failed to extract struct definition");
        assert_eq!(name, "Point", "Struct name should be 'Point'");
        assert_eq!(def.fields.len(), 2, "Struct should have 2 fields");
        assert!(def.fields.contains_key("x"), "Struct should have field 'x'");
        assert!(def.fields.contains_key("y"), "Struct should have field 'y'");
    }

    #[test]
    fn test_conversion_context_generics() {
        let mut ctx = ConversionContext::new();
        let g1 = ctx.register_generic("T".to_string());
        let g2 = ctx.register_generic("U".to_string());
        
        assert_ne!(g1, g2);
        assert_eq!(ctx.generic_bindings.len(), 2);
    }

    #[test]
    fn test_struct_type_in_context() {
        let mut ctx = ConversionContext::new();
        let struct_id = ctx.register_struct("Vec".to_string());
        
        let named_type = parser_ast::Type::Named("Vec".to_string());
        let converted = convert_type_with_context(&named_type, &mut ctx)
            .expect("Failed to convert struct type in context");
        
        assert_eq!(converted, Type::Struct(struct_id), "Named type should convert to Struct");
    }
}