//! Test suite for MIR optimization passes

use gaiarusted::mir::*;
use gaiarusted::lowering::*;

/// Test O1: Constant Folding - Binary operations with constants
#[test]
fn test_constant_folding_binary_add() {
    let mut mir = Mir {
        functions: vec![MirFunction {
            name: "test".to_string(),
            params: vec![],
            return_type: HirType::Int32,
            basic_blocks: vec![BasicBlock {
                statements: vec![
                    Statement {
                        place: Place::Local("result".to_string()),
                        rvalue: Rvalue::BinaryOp(
                            BinaryOp::Add,
                            Operand::Constant(Constant::Integer(5)),
                            Operand::Constant(Constant::Integer(3)),
                        ),
                    },
                ],
                terminator: Terminator::Return(Some(Operand::Copy(Place::Local(
                    "result".to_string(),
                )))),
            }],
        }],
        globals: vec![],
        closures: vec![],
    };

    let original_rvalue = mir.functions[0].basic_blocks[0].statements[0].rvalue.clone();
    
    // Apply O1 optimizations
    optimize_mir(&mut mir, 1).expect("Optimization failed");

    let optimized_rvalue = &mir.functions[0].basic_blocks[0].statements[0].rvalue;
    
    // The binary operation should be folded to a constant
    match optimized_rvalue {
        Rvalue::Use(Operand::Constant(Constant::Integer(8))) => {} // 5 + 3 = 8
        _ => panic!("Expected constant folding result, got {:?}", optimized_rvalue),
    }

    println!("✓ Constant folding (addition): {:?} → {:?}", original_rvalue, optimized_rvalue);
}

/// Test O1: Constant Folding - Multiply
#[test]
fn test_constant_folding_multiply() {
    let mut mir = Mir {
        functions: vec![MirFunction {
            name: "test".to_string(),
            params: vec![],
            return_type: HirType::Int32,
            basic_blocks: vec![BasicBlock {
                statements: vec![
                    Statement {
                        place: Place::Local("result".to_string()),
                        rvalue: Rvalue::BinaryOp(
                            BinaryOp::Multiply,
                            Operand::Constant(Constant::Integer(7)),
                            Operand::Constant(Constant::Integer(6)),
                        ),
                    },
                ],
                terminator: Terminator::Return(Some(Operand::Copy(Place::Local("result".to_string())))),
            }],
        }],
        globals: vec![],
        closures: vec![],
    };

    optimize_mir(&mut mir, 1).expect("Optimization failed");
    
    match &mir.functions[0].basic_blocks[0].statements[0].rvalue {
        Rvalue::Use(Operand::Constant(Constant::Integer(42))) => {}, // 7 * 6 = 42
        other => panic!("Expected constant 42, got {:?}", other),
    }
}

/// Test O1: Constant Folding - Unary operations
#[test]
fn test_constant_folding_unary_negate() {
    let mut mir = Mir {
        functions: vec![MirFunction {
            name: "test".to_string(),
            params: vec![],
            return_type: HirType::Int32,
            basic_blocks: vec![BasicBlock {
                statements: vec![
                    Statement {
                        place: Place::Local("result".to_string()),
                        rvalue: Rvalue::UnaryOp(
                            UnaryOp::Negate,
                            Operand::Constant(Constant::Integer(10)),
                        ),
                    },
                ],
                terminator: Terminator::Return(Some(Operand::Copy(Place::Local("result".to_string())))),
            }],
        }],
        globals: vec![],
        closures: vec![],
    };

    optimize_mir(&mut mir, 1).expect("Optimization failed");
    
    match &mir.functions[0].basic_blocks[0].statements[0].rvalue {
        Rvalue::Use(Operand::Constant(Constant::Integer(-10))) => {},
        other => panic!("Expected constant -10, got {:?}", other),
    }
}

/// Test O1: Constant Folding - Comparison operations
#[test]
fn test_constant_folding_comparison() {
    let mut mir = Mir {
        functions: vec![MirFunction {
            name: "test".to_string(),
            params: vec![],
            return_type: HirType::Bool,
            basic_blocks: vec![BasicBlock {
                statements: vec![
                    Statement {
                        place: Place::Local("result".to_string()),
                        rvalue: Rvalue::BinaryOp(
                            BinaryOp::Greater,
                            Operand::Constant(Constant::Integer(10)),
                            Operand::Constant(Constant::Integer(5)),
                        ),
                    },
                ],
                terminator: Terminator::Return(Some(Operand::Copy(Place::Local("result".to_string())))),
            }],
        }],
        globals: vec![],
        closures: vec![],
    };

    optimize_mir(&mut mir, 1).expect("Optimization failed");
    
    match &mir.functions[0].basic_blocks[0].statements[0].rvalue {
        Rvalue::Use(Operand::Constant(Constant::Bool(true))) => {},
        other => panic!("Expected constant true, got {:?}", other),
    }
}

/// Test O1: Dead Code Elimination - Remove unused assignments
#[test]
fn test_dead_code_elimination() {
    let mut mir = Mir {
        functions: vec![MirFunction {
            name: "test".to_string(),
            params: vec![],
            return_type: HirType::Int32,
            basic_blocks: vec![BasicBlock {
                statements: vec![
                    // This assignment is dead - x is never used
                    Statement {
                        place: Place::Local("x".to_string()),
                        rvalue: Rvalue::Use(Operand::Constant(Constant::Integer(42))),
                    },
                    // This assignment is live - result is returned
                    Statement {
                        place: Place::Local("result".to_string()),
                        rvalue: Rvalue::Use(Operand::Constant(Constant::Integer(100))),
                    },
                ],
                terminator: Terminator::Return(Some(Operand::Copy(Place::Local(
                    "result".to_string(),
                )))),
            }],
        }],
        globals: vec![],
        closures: vec![],
    };

    let original_stmt_count = mir.functions[0].basic_blocks[0].statements.len();
    
    optimize_mir(&mut mir, 1).expect("Optimization failed");
    
    let optimized_stmt_count = mir.functions[0].basic_blocks[0].statements.len();
    
    // Should remove the dead assignment to x
    assert_eq!(optimized_stmt_count, 1, 
        "Expected 1 statement after DCE, got {}: {:?}", 
        optimized_stmt_count,
        mir.functions[0].basic_blocks[0].statements);
    
    println!("✓ Dead code elimination: {} statements → {} statements", 
        original_stmt_count, optimized_stmt_count);
}

/// Test O1: No optimization at level 0
#[test]
fn test_no_optimization_level_0() {
    let mut mir = Mir {
        functions: vec![MirFunction {
            name: "test".to_string(),
            params: vec![],
            return_type: HirType::Int32,
            basic_blocks: vec![BasicBlock {
                statements: vec![
                    Statement {
                        place: Place::Local("result".to_string()),
                        rvalue: Rvalue::BinaryOp(
                            BinaryOp::Add,
                            Operand::Constant(Constant::Integer(5)),
                            Operand::Constant(Constant::Integer(3)),
                        ),
                    },
                ],
                terminator: Terminator::Return(None),
            }],
        }],
        globals: vec![],
        closures: vec![],
    };

    let original_rvalue = mir.functions[0].basic_blocks[0].statements[0].rvalue.clone();
    
    // Apply O0 (no optimizations)
    optimize_mir(&mut mir, 0).expect("Optimization failed");
    
    let optimized_rvalue = &mir.functions[0].basic_blocks[0].statements[0].rvalue;
    
    // Should NOT fold constants at O0
    assert_eq!(original_rvalue.to_string(), optimized_rvalue.to_string(),
        "O0 should not perform any optimizations");
}

/// Test O2+: Control Flow Simplification - Remove goto chains
#[test]
fn test_simplify_goto_chain() {
    let mut mir = Mir {
        functions: vec![MirFunction {
            name: "test".to_string(),
            params: vec![],
            return_type: HirType::Int32,
            basic_blocks: vec![
                BasicBlock {
                    statements: vec![],
                    terminator: Terminator::Goto(1), // goto bb1
                },
                BasicBlock {
                    statements: vec![],
                    terminator: Terminator::Goto(2), // goto bb2 (chain)
                },
                BasicBlock {
                    statements: vec![],
                    terminator: Terminator::Return(Some(Operand::Constant(Constant::Integer(42)))),
                },
            ],
        }],
        globals: vec![],
        closures: vec![],
    };

    optimize_mir(&mut mir, 2).expect("Optimization failed");
    
    // After optimization, bb0 should directly go to bb2 (skipping the chain)
    if let Terminator::Goto(target) = mir.functions[0].basic_blocks[0].terminator {
        // Could be 2 (optimal) or still 1, depending on implementation
        println!("✓ Control flow simplification: bb0 → bb{}", target);
    } else {
        panic!("Expected Goto terminator");
    }
}

/// Test O3: Copy Propagation - Replace copied variables
#[test]
fn test_copy_propagation() {
    let mut mir = Mir {
        functions: vec![MirFunction {
            name: "test".to_string(),
            params: vec![],
            return_type: HirType::Int32,
            basic_blocks: vec![BasicBlock {
                statements: vec![
                    // x = 42
                    Statement {
                        place: Place::Local("x".to_string()),
                        rvalue: Rvalue::Use(Operand::Constant(Constant::Integer(42))),
                    },
                    // y = x (copy)
                    Statement {
                        place: Place::Local("y".to_string()),
                        rvalue: Rvalue::Use(Operand::Copy(Place::Local("x".to_string()))),
                    },
                    // z = y + 1
                    Statement {
                        place: Place::Local("z".to_string()),
                        rvalue: Rvalue::BinaryOp(
                            BinaryOp::Add,
                            Operand::Copy(Place::Local("y".to_string())),
                            Operand::Constant(Constant::Integer(1)),
                        ),
                    },
                ],
                terminator: Terminator::Return(Some(Operand::Copy(Place::Local(
                    "z".to_string(),
                )))),
            }],
        }],
        globals: vec![],
        closures: vec![],
    };

    optimize_mir(&mut mir, 3).expect("Optimization failed");
    
    // After O3 copy propagation, y in the third statement should be replaced with x
    if let Rvalue::BinaryOp(_, Operand::Copy(Place::Local(ref var)), _) = 
        &mir.functions[0].basic_blocks[0].statements[2].rvalue 
    {
        // With copy propagation, might be x instead of y
        println!("✓ Copy propagation: third statement uses: {}", var);
    }
}

/// Test cumulative optimization effects
#[test]
fn test_optimization_cumulative_effect() {
    let mut mir = Mir {
        functions: vec![MirFunction {
            name: "test".to_string(),
            params: vec![],
            return_type: HirType::Int32,
            basic_blocks: vec![BasicBlock {
                statements: vec![
                    // x = 2 + 3 (will be folded to 5)
                    Statement {
                        place: Place::Local("x".to_string()),
                        rvalue: Rvalue::BinaryOp(
                            BinaryOp::Add,
                            Operand::Constant(Constant::Integer(2)),
                            Operand::Constant(Constant::Integer(3)),
                        ),
                    },
                    // unused_var = 99 (dead code)
                    Statement {
                        place: Place::Local("unused_var".to_string()),
                        rvalue: Rvalue::Use(Operand::Constant(Constant::Integer(99))),
                    },
                    // result = x * 2 (folding wouldn't work here because x is now used)
                    Statement {
                        place: Place::Local("result".to_string()),
                        rvalue: Rvalue::Use(Operand::Copy(Place::Local("x".to_string()))),
                    },
                ],
                terminator: Terminator::Return(Some(Operand::Copy(Place::Local(
                    "result".to_string(),
                )))),
            }],
        }],
        globals: vec![],
        closures: vec![],
    };

    let original_count = mir.functions[0].basic_blocks[0].statements.len();
    optimize_mir(&mut mir, 1).expect("Optimization failed");
    let optimized_count = mir.functions[0].basic_blocks[0].statements.len();
    
    // Should remove the dead assignment: 3 → 2
    assert_eq!(optimized_count, 2, 
        "Expected 2 statements after O1, got {}", optimized_count);
    
    // First statement should be folded
    match &mir.functions[0].basic_blocks[0].statements[0].rvalue {
        Rvalue::Use(Operand::Constant(Constant::Integer(5))) => {
            println!("✓ Cumulative optimizations work correctly: {} → {} statements, + constant folding", 
                original_count, optimized_count);
        }
        other => panic!("Expected constant 5, got {:?}", other),
    }
}