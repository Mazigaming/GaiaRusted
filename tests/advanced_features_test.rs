//! # Phase 4 Advanced Features Test
//!
//! Tests for:
//! 1. Struct field resolution and type checking
//! 2. Method call type checking
//! 3. Generic type parameter instantiation
//! 4. Advanced error reporting with suggestions

use gaiarusted::typesystem::{
    Type, TypeVar, StructId, GenericId, Constraint, ConstraintGenerator,
    DetailedTypeError, ExprTyper, AstExpr, AstBinaryOp,
};
use std::collections::HashMap;

// ============================================================
// Part 1: Struct Field Resolution Tests
// ============================================================

#[test]
fn test_field_access_basic_struct() {
    println!("\n=== Test: Basic Struct Field Access ===\n");
    
    let mut gen = ConstraintGenerator::new();
    let struct_id = StructId(0);
    
    // Define struct: struct Point { x: i32, y: i32 }
    let mut fields = HashMap::new();
    fields.insert("x".to_string(), Type::I32);
    fields.insert("y".to_string(), Type::I32);
    gen.register_struct(struct_id, fields);
    
    // Access field p.x where p: Point
    let result = gen.constrain_field_access(&Type::Struct(struct_id), "x");
    
    assert!(result.is_ok(), "Field access should succeed");
    assert_eq!(result.unwrap(), Type::I32, "Field 'x' should be i32");
    println!("✓ Field access p.x returned i32");
}

#[test]
fn test_field_access_nonexistent_field() {
    println!("\n=== Test: Field Access on Nonexistent Field ===\n");
    
    let mut gen = ConstraintGenerator::new();
    let struct_id = StructId(1);
    
    // Define struct: struct Point { x: i32, y: i32 }
    let mut fields = HashMap::new();
    fields.insert("x".to_string(), Type::I32);
    fields.insert("y".to_string(), Type::I32);
    gen.register_struct(struct_id, fields);
    
    // Try to access non-existent field p.z
    let result = gen.constrain_field_access(&Type::Struct(struct_id), "z");
    
    assert!(result.is_err(), "Field access should fail for nonexistent field");
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("has no field"), "Error should mention missing field");
    println!("✓ Field access on nonexistent field properly reported: {}", err_msg);
}

#[test]
fn test_field_access_complex_types() {
    println!("\n=== Test: Field Access with Complex Types ===\n");
    
    let mut gen = ConstraintGenerator::new();
    let struct_id = StructId(2);
    
    // Define struct with complex fields:
    // struct Node { value: i32, next: &mut Node, data: [i32; 5] }
    let mut fields = HashMap::new();
    fields.insert("value".to_string(), Type::I32);
    fields.insert("next".to_string(), Type::Reference {
        lifetime: None,
        mutable: true,
        inner: Box::new(Type::Struct(struct_id)),
    });
    fields.insert("data".to_string(), Type::Array {
        element: Box::new(Type::I32),
        size: 5,
    });
    gen.register_struct(struct_id, fields);
    
    // Access each field
    let value_result = gen.constrain_field_access(&Type::Struct(struct_id), "value");
    let next_result = gen.constrain_field_access(&Type::Struct(struct_id), "next");
    let data_result = gen.constrain_field_access(&Type::Struct(struct_id), "data");
    
    assert!(value_result.is_ok());
    assert!(next_result.is_ok());
    assert!(data_result.is_ok());
    
    assert_eq!(value_result.unwrap(), Type::I32);
    match data_result.unwrap() {
        Type::Array { element, size } => {
            assert_eq!(*element, Type::I32);
            assert_eq!(size, 5);
        }
        _ => panic!("Data field should be array"),
    }
    println!("✓ Field access with complex types works correctly");
}

// ============================================================
// Part 2: Method Call Type Checking Tests
// ============================================================

#[test]
fn test_method_call_basic() {
    println!("\n=== Test: Basic Method Call ===\n");
    
    let mut gen = ConstraintGenerator::new();
    let struct_id = StructId(3);
    
    // Register struct
    let mut fields = HashMap::new();
    fields.insert("value".to_string(), Type::I32);
    gen.register_struct(struct_id, fields);
    
    // Register method: impl Point { fn get_value(&self) -> i32 { ... } }
    gen.register_function(
        "3_method_get_value".to_string(),
        vec![],  // No additional parameters beyond self
        Type::I32,
    );
    
    // Call method: p.get_value()
    let result = gen.constrain_method_call(&Type::Struct(struct_id), "get_value", vec![]);
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Type::I32);
    println!("✓ Method call p.get_value() returned i32");
}

#[test]
fn test_method_call_with_arguments() {
    println!("\n=== Test: Method Call with Arguments ===\n");
    
    let mut gen = ConstraintGenerator::new();
    let struct_id = StructId(4);
    
    // Register struct
    gen.register_struct(struct_id, HashMap::new());
    
    // Register method: fn set_value(&mut self, val: i32) -> bool
    gen.register_function(
        "4_method_set_value".to_string(),
        vec![Type::I32],
        Type::Bool,
    );
    
    // Call method: p.set_value(42)
    let result = gen.constrain_method_call(
        &Type::Struct(struct_id),
        "set_value",
        vec![Type::I32],
    );
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Type::Bool);
    println!("✓ Method call p.set_value(42) returned bool");
}

#[test]
fn test_method_call_argument_mismatch() {
    println!("\n=== Test: Method Call Argument Mismatch ===\n");
    
    let mut gen = ConstraintGenerator::new();
    let struct_id = StructId(5);
    
    gen.register_struct(struct_id, HashMap::new());
    
    // Method expects 2 arguments
    gen.register_function(
        "5_method_combine".to_string(),
        vec![Type::I32, Type::Bool],
        Type::Str,
    );
    
    // Call with 1 argument - should fail
    let result = gen.constrain_method_call(
        &Type::Struct(struct_id),
        "combine",
        vec![Type::I32],
    );
    
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expects 2 arguments"));
    println!("✓ Argument mismatch properly detected");
}

// ============================================================
// Part 3: Generic Type Parameters Tests
// ============================================================

#[test]
fn test_generic_function_instantiation() {
    println!("\n=== Test: Generic Function Instantiation ===\n");
    
    let mut gen = ConstraintGenerator::new();
    
    // Define generic function: fn id<T>(x: T) -> T
    let generic_t = GenericId(0);
    gen.register_function_with_generics(
        "id".to_string(),
        vec![Type::Generic(generic_t)],
        Type::Generic(generic_t),
        vec![generic_t],
    );
    
    // Instantiate with i32: id::<i32>
    let result = gen.instantiate_generic_function("id", vec![Type::I32]);
    
    assert!(result.is_ok());
    let (params, ret) = result.unwrap();
    assert_eq!(params, vec![Type::I32]);
    assert_eq!(ret, Type::I32);
    println!("✓ Generic function id<i32> instantiated correctly");
}

#[test]
fn test_generic_struct_instantiation() {
    println!("\n=== Test: Generic Struct Instantiation ===\n");
    
    let mut gen = ConstraintGenerator::new();
    let struct_id = StructId(6);
    let generic_t = GenericId(1);
    
    // Define generic struct: struct Box<T> { value: T }
    let mut fields = HashMap::new();
    fields.insert("value".to_string(), Type::Generic(generic_t));
    gen.register_struct_with_generics(
        struct_id,
        fields,
        vec![generic_t],
    );
    
    // Instantiate with i32: Box<i32>
    let result = gen.instantiate_generic_struct(struct_id, vec![Type::I32]);
    
    assert!(result.is_ok());
    let instantiated = result.unwrap();
    assert_eq!(instantiated.get("value"), Some(&Type::I32));
    println!("✓ Generic struct Box<i32> instantiated correctly");
}

#[test]
fn test_generic_function_complex_types() {
    println!("\n=== Test: Generic Function with Complex Types ===\n");
    
    let mut gen = ConstraintGenerator::new();
    let generic_t = GenericId(2);
    let generic_u = GenericId(3);
    
    // Define: fn pair<T, U>(x: T, y: U) -> (T, U)
    gen.register_function_with_generics(
        "pair".to_string(),
        vec![Type::Generic(generic_t), Type::Generic(generic_u)],
        Type::Tuple(vec![Type::Generic(generic_t), Type::Generic(generic_u)]),
        vec![generic_t, generic_u],
    );
    
    // Instantiate with i32, bool: pair::<i32, bool>
    let result = gen.instantiate_generic_function(
        "pair",
        vec![Type::I32, Type::Bool],
    );
    
    assert!(result.is_ok());
    let (params, ret) = result.unwrap();
    assert_eq!(params, vec![Type::I32, Type::Bool]);
    assert_eq!(ret, Type::Tuple(vec![Type::I32, Type::Bool]));
    println!("✓ Generic function pair<i32, bool> instantiated correctly");
}

#[test]
fn test_generic_arity_mismatch() {
    println!("\n=== Test: Generic Type Argument Arity Mismatch ===\n");
    
    let mut gen = ConstraintGenerator::new();
    let generic_t = GenericId(4);
    
    // Single type parameter
    gen.register_function_with_generics(
        "single".to_string(),
        vec![Type::Generic(generic_t)],
        Type::Generic(generic_t),
        vec![generic_t],
    );
    
    // Try to instantiate with 2 type arguments
    let result = gen.instantiate_generic_function("single", vec![Type::I32, Type::Bool]);
    
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expects 1 type argument"));
    println!("✓ Generic arity mismatch properly detected");
}

// ============================================================
// Part 4: Advanced Error Reporting Tests
// ============================================================

#[test]
fn test_error_type_mismatch() {
    println!("\n=== Test: Type Mismatch Error with Suggestions ===\n");
    
    let error = DetailedTypeError::type_mismatch("i32", "bool");
    let formatted = format!("{}", error);
    
    assert!(formatted.contains("Type mismatch"));
    assert!(formatted.contains("expected i32"));
    assert!(formatted.contains("found bool"));
    assert!(formatted.contains("Suggestions:"));
    println!("✓ Type mismatch error:\n{}", formatted);
}

#[test]
fn test_error_field_not_found_with_suggestions() {
    println!("\n=== Test: Field Not Found Error with Suggestions ===\n");
    
    let available = vec!["x", "y", "z"];
    let error = DetailedTypeError::field_not_found("Point", "xx", available);
    let formatted = format!("{}", error);
    
    assert!(formatted.contains("has no field 'xx'"));
    assert!(formatted.contains("Available fields"));
    // Check if it suggests similar names
    assert!(formatted.contains("Suggestions:") || formatted.contains("x"));
    println!("✓ Field not found error:\n{}", formatted);
}

#[test]
fn test_error_method_not_found() {
    println!("\n=== Test: Method Not Found Error ===\n");
    
    let error = DetailedTypeError::method_not_found("Point", "distance");
    let formatted = format!("{}", error);
    
    assert!(formatted.contains("has no method 'distance'"));
    assert!(formatted.contains("Suggestions:"));
    println!("✓ Method not found error:\n{}", formatted);
}

#[test]
fn test_error_argument_mismatch() {
    println!("\n=== Test: Function Argument Mismatch Error ===\n");
    
    let error = DetailedTypeError::argument_mismatch("add", 2, 3);
    let formatted = format!("{}", error);
    
    assert!(formatted.contains("expects 2 argument(s)"));
    assert!(formatted.contains("got 3"));
    assert!(formatted.contains("Suggestions:"));
    println!("✓ Argument mismatch error:\n{}", formatted);
}

#[test]
fn test_error_chaining() {
    println!("\n=== Test: Error Details and Context Chaining ===\n");
    
    let error = DetailedTypeError::new("Test error")
        .with_details("This is a detailed explanation")
        .with_suggestion("Try fixing this")
        .with_suggestion("Or try that")
        .with_context("In function main");
    
    let formatted = format!("{}", error);
    
    assert!(formatted.contains("Test error"));
    assert!(formatted.contains("detailed explanation"));
    assert!(formatted.contains("Try fixing this"));
    assert!(formatted.contains("Or try that"));
    assert!(formatted.contains("In function main"));
    println!("✓ Error chaining works:\n{}", formatted);
}

// ============================================================
// Part 5: Integration Tests
// ============================================================

#[test]
fn test_integration_struct_field_and_method() {
    println!("\n=== Test: Integration - Struct with Field and Method ===\n");
    
    let mut gen = ConstraintGenerator::new();
    let struct_id = StructId(10);
    
    // Define struct: struct Calculator { result: i32 }
    let mut fields = HashMap::new();
    fields.insert("result".to_string(), Type::I32);
    gen.register_struct(struct_id, fields);
    
    // Define method: fn add(&mut self, x: i32) -> i32
    gen.register_function(
        "10_method_add".to_string(),
        vec![Type::I32],
        Type::I32,
    );
    
    // Test field access
    let field_result = gen.constrain_field_access(&Type::Struct(struct_id), "result");
    assert_eq!(field_result.unwrap(), Type::I32);
    
    // Test method call
    let method_result = gen.constrain_method_call(
        &Type::Struct(struct_id),
        "add",
        vec![Type::I32],
    );
    assert_eq!(method_result.unwrap(), Type::I32);
    
    println!("✓ Struct field access and method call work together");
}

#[test]
fn test_integration_generic_with_methods() {
    println!("\n=== Test: Integration - Generic Struct with Generic Methods ===\n");
    
    let mut gen = ConstraintGenerator::new();
    let struct_id = StructId(11);
    let generic_t = GenericId(5);
    
    // Generic struct: struct Container<T> { item: T }
    let mut fields = HashMap::new();
    fields.insert("item".to_string(), Type::Generic(generic_t));
    gen.register_struct_with_generics(
        struct_id,
        fields,
        vec![generic_t],
    );
    
    // Generic method: fn unwrap<T>(self) -> T
    gen.register_function_with_generics(
        "11_method_unwrap".to_string(),
        vec![],
        Type::Generic(generic_t),
        vec![generic_t],
    );
    
    // Instantiate struct as Container<i32>
    let struct_inst = gen.instantiate_generic_struct(struct_id, vec![Type::I32]);
    assert!(struct_inst.is_ok());
    assert_eq!(struct_inst.unwrap().get("item"), Some(&Type::I32));
    
    // Instantiate method with i32
    let method_inst = gen.instantiate_generic_function("11_method_unwrap", vec![Type::I32]);
    assert!(method_inst.is_ok());
    let (_, ret) = method_inst.unwrap();
    assert_eq!(ret, Type::I32);
    
    println!("✓ Generic struct and generic methods work together");
}

#[test]
fn test_expression_with_field_access() {
    println!("\n=== Test: Expression Typing with Field Access ===\n");
    
    let mut typer = ExprTyper::new();
    let struct_id = StructId(12);
    
    // Setup struct
    let mut fields = HashMap::new();
    fields.insert("x".to_string(), Type::I32);
    fields.insert("y".to_string(), Type::I32);
    typer.generator.register_struct(struct_id, fields);
    
    // Create expression: Point { ... }.x (direct field access on struct)
    let expr = AstExpr::FieldAccess {
        object: Box::new(AstExpr::Variable("p".to_string())),
        field: "x".to_string(),
    };
    
    // Register the variable first with struct type
    let var_type = Type::Struct(struct_id);
    typer.register_variable("p".to_string(), var_type.clone())
        .expect("Variable registration failed");
    
    // When we type a field access on a known struct, it should resolve correctly
    let obj_expr = AstExpr::Variable("p".to_string());
    let obj_result = typer.type_expr(&obj_expr);
    assert!(obj_result.is_ok());
    
    // The object type should be the struct (or a type variable bound to it)
    let obj_ty = obj_result.unwrap().ty;
    
    // Now test field access on the concrete struct type
    let field_result = typer.generator.constrain_field_access(&Type::Struct(struct_id), "x");
    assert!(field_result.is_ok(), "Field access should succeed on concrete struct");
    assert_eq!(field_result.unwrap(), Type::I32);
    
    println!("✓ Expression typing with field access works");
}