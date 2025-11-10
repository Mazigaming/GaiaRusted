use gaiarusted::codegen::Codegen;
use gaiarusted::lowering::{HirExpression, HirType};

#[test]
fn test_codegen_initialization() {
    let gen = Codegen::new();
    let _ = gen;
}

#[test]
fn test_integer_expression() {
    let expr = HirExpression::Integer(42);
    match expr {
        HirExpression::Integer(n) => assert_eq!(n, 42),
        _ => panic!("Expected integer"),
    }
}

#[test]
fn test_bool_expression() {
    let expr = HirExpression::Bool(true);
    match expr {
        HirExpression::Bool(b) => assert!(b),
        _ => panic!("Expected bool"),
    }
}

#[test]
fn test_hir_types() {
    let int_type = HirType::Int32;
    let i64_type = HirType::Int64;
    let bool_type = HirType::Bool;
    let _ = (int_type, i64_type, bool_type);
}