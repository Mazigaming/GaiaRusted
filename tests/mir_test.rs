use gaiarusted::mir::{MirBuilder, Place, Operand, Constant, Rvalue, Terminator};

#[test]
fn test_mir_builder_creation() {
    let builder = MirBuilder::new();
    let _ = builder;
}

#[test]
fn test_place_local() {
    let place = Place::Local("x".to_string());
    assert_eq!(place.to_string(), "x");
}

#[test]
fn test_constant_integer() {
    let c = Constant::Integer(42);
    assert_eq!(c.to_string(), "42");
}

#[test]
fn test_constant_bool() {
    let c = Constant::Bool(true);
    assert_eq!(c.to_string(), "true");
}

#[test]
fn test_operand_constant() {
    let op = Operand::Constant(Constant::Integer(10));
    assert_eq!(op.to_string(), "10");
}

#[test]
fn test_rvalue_use() {
    let op = Operand::Constant(Constant::String("hello".to_string()));
    let rval = Rvalue::Use(op);
    let s = rval.to_string();
    assert!(s.contains("hello"));
}

#[test]
fn test_terminator_return() {
    let term = Terminator::Return(None);
    assert_eq!(term.to_string(), "return");
}

#[test]
fn test_terminator_goto() {
    let term = Terminator::Goto(1);
    assert_eq!(term.to_string(), "goto bb1");
}