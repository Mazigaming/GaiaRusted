//! Async/Await code generation
//!
//! Generates x86-64 assembly for async state machines:
//! - State machine struct allocation and initialization
//! - Future poll method implementation
//! - State transitions and await point handling

use crate::async_await::lowering::{AsyncStateMachine, StateId};
use crate::codegen::{Codegen, CodegenResult, Register, X86Instruction, X86Operand};
use crate::lowering::HirType;

/// Generate x86-64 assembly for an async state machine poll function
pub fn generate_async_state_machine_poll(
    codegen: &mut Codegen,
    machine: &AsyncStateMachine,
) -> CodegenResult<()> {
    let poll_function_name = format!("{}_poll", machine.name);

    // Function label
    codegen.instructions.push(X86Instruction::Label {
        name: poll_function_name.clone(),
    });

    // Function prologue
    codegen
        .instructions
        .push(X86Instruction::Push { reg: Register::RBP });
    codegen.instructions.push(X86Instruction::Mov {
        dst: X86Operand::Register(Register::RBP),
        src: X86Operand::Register(Register::RSP),
    });

    // Allocate stack space for local variables
    let stack_size = calculate_stack_size(machine);
    if stack_size > 0 {
        codegen.instructions.push(X86Instruction::Sub {
            dst: X86Operand::Register(Register::RSP),
            src: X86Operand::Imm(stack_size),
        });
    }

    // Generate state machine logic
    generate_state_machine_logic(codegen, machine)?;

    // Function epilogue
    if stack_size > 0 {
        codegen.instructions.push(X86Instruction::Add {
            dst: X86Operand::Register(Register::RSP),
            src: X86Operand::Imm(stack_size),
        });
    }

    codegen
        .instructions
        .push(X86Instruction::Pop { reg: Register::RBP });
    codegen.instructions.push(X86Instruction::Ret);

    Ok(())
}

/// Calculate stack space needed for state machine
fn calculate_stack_size(machine: &AsyncStateMachine) -> i64 {
    // Each local variable needs space, plus state field
    let local_vars_size = machine.local_vars.len() as i64 * 8; // 8 bytes per variable
    let params_size = machine.params.len() as i64 * 8; // 8 bytes per parameter
    let state_size = 8; // state field

    // Align to 16 bytes
    let total = local_vars_size + params_size + state_size;
    ((total + 15) / 16) * 16
}

/// Generate the core state machine logic
fn generate_state_machine_logic(
    codegen: &mut Codegen,
    machine: &AsyncStateMachine,
) -> CodegenResult<()> {
    // Load state field (first parameter is self pointer in RDI)
    codegen.instructions.push(X86Instruction::Mov {
        dst: X86Operand::Register(Register::RAX),
        src: X86Operand::Mem(Register::RDI, 0), // state field at offset 0
    });

    // Generate state dispatch
    let mut state_labels = Vec::new();

    for (state_id, _) in &machine.state_blocks {
        let label = format!("{}_state_{}", machine.name, state_id.0);
        state_labels.push(label);
    }

    // Generate comparison and jumps for each state
    for (i, state_id) in machine.state_blocks.keys().enumerate() {
        if i == 0 {
            codegen.instructions.push(X86Instruction::Cmp {
                dst: X86Operand::Register(Register::RAX),
                src: X86Operand::Imm(state_id.0 as i64),
            });
        } else {
            codegen.instructions.push(X86Instruction::Je {
                label: state_labels[i - 1].clone(),
            });
            codegen.instructions.push(X86Instruction::Cmp {
                dst: X86Operand::Register(Register::RAX),
                src: X86Operand::Imm(state_id.0 as i64),
            });
        }
    }

    // Default case (invalid state)
    codegen.instructions.push(X86Instruction::Mov {
        dst: X86Operand::Register(Register::RAX),
        src: X86Operand::Imm(1), // Poll::Pending
    });
    codegen.instructions.push(X86Instruction::Ret);

    // Generate state implementations
    for (state_id, block) in &machine.state_blocks {
        let label = format!("{}_state_{}", machine.name, state_id.0);
        codegen
            .instructions
            .push(X86Instruction::Label { name: label });

        // Generate code for this state's block
        generate_state_block(codegen, machine, block)?;
    }

    Ok(())
}

/// Generate code for a single state block
fn generate_state_block(
    codegen: &mut Codegen,
    machine: &AsyncStateMachine,
    block: &[crate::lowering::HirStatement],
) -> CodegenResult<()> {
    for stmt in block {
        match stmt {
            crate::lowering::HirStatement::Let {
                name, initializer, ..
            } => {
                // Generate code for initializer
                if let Some(init) = initializer {
                    generate_expression(codegen, init)?;
                    // Store result in local variable slot
                    // For now, simplified - assume all locals are at known offsets
                }
            }
            crate::lowering::HirStatement::Expression(expr) => {
                generate_expression(codegen, expr)?;
            }
            crate::lowering::HirStatement::Return(Some(expr)) => {
                generate_expression(codegen, expr)?;
                // Set state to completed and return Ready
                codegen.instructions.push(X86Instruction::Mov {
                    dst: X86Operand::Register(Register::RAX),
                    src: X86Operand::Imm(0), // Poll::Ready
                });
                codegen.instructions.push(X86Instruction::Ret);
            }
            _ => {
                // Handle other statement types
            }
        }
    }

    // If we reach the end of the block without returning,
    // this state is pending - update state and return Pending
    if let Some(next_state) = get_next_state(machine) {
        codegen.instructions.push(X86Instruction::Mov {
            dst: X86Operand::Mem(Register::RDI, 0),
            src: X86Operand::Imm(next_state.0 as i64),
        });
    }

    codegen.instructions.push(X86Instruction::Mov {
        dst: X86Operand::Register(Register::RAX),
        src: X86Operand::Imm(1), // Poll::Pending
    });
    codegen.instructions.push(X86Instruction::Ret);

    Ok(())
}

/// Generate code for an expression
fn generate_expression(
    codegen: &mut Codegen,
    expr: &crate::lowering::HirExpression,
) -> CodegenResult<()> {
    match expr {
        crate::lowering::HirExpression::Integer(n) => {
            codegen.instructions.push(X86Instruction::Mov {
                dst: X86Operand::Register(Register::RAX),
                src: X86Operand::Imm(*n),
            });
        }
        crate::lowering::HirExpression::Await { value } => {
            // For await, we need to:
            // 1. Generate code for the awaited expression (which should be a future)
            // 2. Call its poll method
            // 3. Handle the result (Ready vs Pending)

            // This is simplified - in a real implementation, we'd need to:
            // - Generate code for the future expression
            // - Call the poll method
            // - Check if it's ready or pending
            // - Store state and return if pending

            // For now, just generate a placeholder
            codegen.instructions.push(X86Instruction::Comment {
                text: "Await expression - placeholder implementation".to_string(),
            });
        }
        _ => {
            // Handle other expression types
            codegen.instructions.push(X86Instruction::Comment {
                text: format!("Expression type not implemented: {:?}", expr),
            });
        }
    }

    Ok(())
}

/// Get the next state for pending operations
fn get_next_state(machine: &AsyncStateMachine) -> Option<StateId> {
    // Simplified - return the first await state
    machine.await_points.first().map(|_| StateId(1))
}



    // Add local variable fields
    for (var_name, var_type) in &machine.local_vars {
        let hir_type_str = hir_type_to_string(var_type);
        // Wrap in Option to handle uninitialized state
        code.push_str(&format!("    {}: Option<{}>,\n", var_name, hir_type_str));
    }

    // Add output field
    let return_type_str = hir_type_to_string(&machine.return_type);
    code.push_str(&format!("    __output: Option<{}>,\n", return_type_str));

    code.push_str("}\n");

    Ok(code)
}



        code.push_str("                }\n");
    }

    // Catch invalid states
    code.push_str("                _ => {\n");
    code.push_str("                    // Invalid state\n");
    code.push_str("                    panic!(\"Invalid future state: {}\", self.__state);\n");
    code.push_str("                }\n");

    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    Ok(code)
}


        HirType::MutableReference(inner) => {
            format!("&mut {}", hir_type_to_string(inner))
        }
        HirType::Pointer(inner) => {
            format!("*const {}", hir_type_to_string(inner))
        }
        HirType::Array { element_type, size } => {
            let elem = hir_type_to_string(element_type);
            if let Some(sz) = size {
                format!("[{}; {}]", elem, sz)
            } else {
                format!("[{}]", elem)
            }
        }
        HirType::Function {
            params,
            return_type,
        } => {
            let param_strs: Vec<String> = params.iter().map(hir_type_to_string).collect();
            let ret = hir_type_to_string(return_type);
            format!("fn({}) -> {}", param_strs.join(", "), ret)
        }
        HirType::Tuple(types) => {
            let type_strs: Vec<String> = types.iter().map(hir_type_to_string).collect();
            format!("({})", type_strs.join(", "))
        }
        HirType::Closure {
            params,
            return_type,
            ..
        } => {
            let param_strs: Vec<String> = params.iter().map(hir_type_to_string).collect();
            let ret = hir_type_to_string(return_type);
            format!("|{}| -> {}", param_strs.join(", "), ret)
        }
        HirType::Vec(inner) => {
            format!("Vec<{}>", hir_type_to_string(inner))
        }
        HirType::Option(inner) => {
            format!("Option<{}>", hir_type_to_string(inner))
        }
        HirType::Box(inner) => {
            format!("Box<{}>", hir_type_to_string(inner))
        }
        HirType::Result { ok_type, err_type } => {
            let ok_str = hir_type_to_string(ok_type);
            let err_str = hir_type_to_string(err_type);
            format!("Result<{}, {}>", ok_str, err_str)
        }
        HirType::DynTrait { trait_name } => {
            format!("dyn {}", trait_name)
        }
        HirType::Range => "std::ops::Range".to_string(),
        HirType::Unknown => "Unknown".to_string(),
    }
}

/// Capitalize first letter of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::Codegen;
    use crate::async_await::state_machine::{StateId, generate_state_struct, generate_poll_method};
    use crate::lowering::{HirStatement, HirExpression, HirType};

    #[test]
    fn test_calculate_stack_size_simple() {
        // Test with minimal state machine
        let machine = AsyncStateMachine {
            name: "test".to_string(),
            params: vec![],
            return_type: HirType::Int32,
            local_vars: vec![],
            await_points: vec![],
            state_blocks: std::collections::HashMap::new(),
            original_item: crate::parser::ast::Item::Function {
                name: "test".to_string(),
                generics: vec![],
                params: vec![],
                return_type: crate::parser::ast::Type::Named("i32".to_string()),
                body: vec![],
                is_unsafe: false,
                is_async: false,
                is_pub: false,
                attributes: vec![],
                where_clause: None,
                abi: None,
            },
        };

        // Only state field (8 bytes) aligned to 16 = 16 bytes
        assert_eq!(calculate_stack_size(&machine), 16);
    }

    #[test]
    fn test_calculate_stack_size_with_params_and_locals() {
        let machine = AsyncStateMachine {
            name: "test".to_string(),
            params: vec![
                ("x".to_string(), HirType::Int32),
                ("y".to_string(), HirType::Int64),
            ],
            return_type: HirType::Int32,
            local_vars: vec![
                ("a".to_string(), HirType::Int32),
                ("b".to_string(), HirType::Int32),
                ("c".to_string(), HirType::Int32),
            ],
            await_points: vec![],
            state_blocks: std::collections::HashMap::new(),
            original_item: crate::parser::ast::Item::Function {
                name: "test".to_string(),
                generics: vec![],
                params: vec![],
                return_type: crate::parser::ast::Type::Named("i32".to_string()),
                body: vec![],
                is_unsafe: false,
                is_async: false,
                is_pub: false,
                attributes: vec![],
                where_clause: None,
                abi: None,
            },
        };

        // 2 params (16 bytes) + 3 locals (24 bytes) + 1 state (8 bytes) = 48 bytes
        // Aligned to 16 bytes = 48 bytes (already aligned)
        assert_eq!(calculate_stack_size(&machine), 48);
    }

    #[test]
    fn test_calculate_stack_size_large_alignment() {
        let machine = AsyncStateMachine {
            name: "test".to_string(),
            params: vec![("x".to_string(), HirType::Int32)], // 8 bytes
            return_type: HirType::Int32,
            local_vars: vec![("y".to_string(), HirType::Int32)], // 8 bytes
            await_points: vec![],
            state_blocks: std::collections::HashMap::new(),
            original_item: crate::parser::ast::Item::Function {
                name: "test".to_string(),
                generics: vec![],
                params: vec![],
                return_type: crate::parser::ast::Type::Named("i32".to_string()),
                body: vec![],
                is_unsafe: false,
                is_async: false,
                is_pub: false,
                attributes: vec![],
                where_clause: None,
                abi: None,
            },
        };

        // 1 param (8) + 1 local (8) + 1 state (8) = 24 bytes
        // Aligned to 16 bytes = 32 bytes
        assert_eq!(calculate_stack_size(&machine), 32);
    }

    #[test]
    fn test_generate_async_poll_function_basic() {
        let mut codegen = Codegen::new();

        // Create a simple MIR function that would represent a poll function
        let mir_func = crate::mir::MirFunction {
            name: "test_poll".to_string(),
            params: vec![
                ("self".to_string(), HirType::Unknown), // &mut Self
                ("cx".to_string(), HirType::Unknown),    // &mut Context
            ],
            return_type: HirType::Named("Poll<i32>".to_string()),
            basic_blocks: vec![], // Empty for this test
        };

        // This should not panic - basic smoke test
        let result = codegen.generate_async_poll_function(&mir_func);
        assert!(result.is_ok(), "generate_async_poll_function should succeed");

        // Check that some instructions were generated
        assert!(!codegen.instructions.is_empty(), "Should generate some assembly instructions");
    }

    #[test]
    fn test_generate_async_poll_function_instructions() {
        let mut codegen = Codegen::new();

        let mir_func = crate::mir::MirFunction {
            name: "my_async_poll".to_string(),
            params: vec![
                ("self".to_string(), HirType::Unknown),
                ("cx".to_string(), HirType::Unknown),
            ],
            return_type: HirType::Named("Poll<i32>".to_string()),
            basic_blocks: vec![],
        };

        codegen.generate_async_poll_function(&mir_func).unwrap();

        // Check for key instructions
        let instr_strings: Vec<String> = codegen.instructions.iter().map(|i| format!("{}", i)).collect();

        // Should have function label
        assert!(instr_strings.iter().any(|s| s.contains("my_async_poll:")),
                "Should generate function label");

        // Should have prologue
        assert!(instr_strings.iter().any(|s| s.contains("push rbp")),
                "Should have prologue push");
        assert!(instr_strings.iter().any(|s| s.contains("mov rbp, rsp")),
                "Should set up frame pointer");

        // Should have epilogue
        assert!(instr_strings.iter().any(|s| s.contains("pop rbp")),
                "Should have epilogue pop");
        assert!(instr_strings.iter().any(|s| s.contains("ret")),
                "Should have return instruction");
    }

    #[test]
    fn test_generate_async_poll_function_state_logic() {
        let mut codegen = Codegen::new();

        let mir_func = crate::mir::MirFunction {
            name: "state_machine_poll".to_string(),
            params: vec![
                ("self".to_string(), HirType::Unknown),
                ("cx".to_string(), HirType::Unknown),
            ],
            return_type: HirType::Named("Poll<i32>".to_string()),
            basic_blocks: vec![],
        };

        codegen.generate_async_poll_function(&mir_func).unwrap();

        let instr_strings: Vec<String> = codegen.instructions.iter().map(|i| format!("{}", i)).collect();

        // Should load state field
        assert!(instr_strings.iter().any(|s| s.contains("mov rax, [rdi]")),
                "Should load state from self pointer");

        // Should compare state
        assert!(instr_strings.iter().any(|s| s.contains("cmp rax")),
                "Should compare state value");

        // Should have conditional jump
        assert!(instr_strings.iter().any(|s| s.contains("jne")),
                "Should have conditional jump for state dispatch");
    }

    #[test]
    fn test_generate_async_poll_function_return_values() {
        let mut codegen = Codegen::new();

        let mir_func = crate::mir::MirFunction {
            name: "poll_func".to_string(),
            params: vec![
                ("self".to_string(), HirType::Unknown),
                ("cx".to_string(), HirType::Unknown),
            ],
            return_type: HirType::Named("Poll<i32>".to_string()),
            basic_blocks: vec![],
        };

        codegen.generate_async_poll_function(&mir_func).unwrap();

        let instr_strings: Vec<String> = codegen.instructions.iter().map(|i| format!("{}", i)).collect();

        // Should set return values in RAX
        assert!(instr_strings.iter().any(|s| s.contains("mov rax, 0")),
                "Should return Poll::Ready (0)");
        assert!(instr_strings.iter().any(|s| s.contains("mov rax, 1")),
                "Should return Poll::Pending (1)");
    }

    #[test]
    fn test_generate_async_poll_function_stack_alignment() {
        let mut codegen = Codegen::new();

        let mir_func = crate::mir::MirFunction {
            name: "aligned_poll".to_string(),
            params: vec![
                ("self".to_string(), HirType::Unknown),
                ("cx".to_string(), HirType::Unknown),
            ],
            return_type: HirType::Named("Poll<i32>".to_string()),
            basic_blocks: vec![],
        };

        codegen.generate_async_poll_function(&mir_func).unwrap();

        let instr_strings: Vec<String> = codegen.instructions.iter().map(|i| format!("{}", i)).collect();

        // Should align stack to 16 bytes before operations
        assert!(instr_strings.iter().any(|s| s.contains("and rsp, -16")),
                "Should align stack to 16 bytes");
    }

    #[test]
    fn test_generate_async_poll_function_with_different_names() {
        let test_cases = vec![
            "simple_poll",
            "complex_async_func_poll",
            "MyStruct_poll",
            "poll_123",
        ];

        for func_name in test_cases {
            let mut codegen = Codegen::new();

            let mir_func = crate::mir::MirFunction {
                name: func_name.to_string(),
                params: vec![
                    ("self".to_string(), HirType::Unknown),
                    ("cx".to_string(), HirType::Unknown),
                ],
                return_type: HirType::Named("Poll<i32>".to_string()),
                basic_blocks: vec![],
            };

            codegen.generate_async_poll_function(&mir_func).unwrap();

            let instr_strings: Vec<String> = codegen.instructions.iter().map(|i| format!("{}", i)).collect();

            // Should generate correct function label
            assert!(instr_strings.iter().any(|s| s.contains(&format!("{}:", func_name))),
                    "Should generate correct label for {}", func_name);
        }
    }

    #[test]
    fn test_get_next_state_no_awaits() {
        let machine = AsyncStateMachine {
            name: "no_awaits".to_string(),
            params: vec![],
            return_type: HirType::Int32,
            local_vars: vec![],
            await_points: vec![], // No await points
            state_blocks: std::collections::HashMap::new(),
            original_item: crate::parser::ast::Item::Function {
                name: "test".to_string(),
                generics: vec![],
                params: vec![],
                return_type: crate::parser::ast::Type::Named("i32".to_string()),
                body: vec![],
                is_unsafe: false,
                is_async: false,
                is_pub: false,
                attributes: vec![],
                where_clause: None,
                abi: None,
            },
        };

        // Should return None when no await points
        assert!(get_next_state(&machine).is_none());
    }

    #[test]
    fn test_get_next_state_with_awaits() {
        use crate::parser::Expression;
        use crate::async_await::lowering::AwaitPoint;

        let machine = AsyncStateMachine {
            name: "with_awaits".to_string(),
            params: vec![],
            return_type: HirType::Int32,
            local_vars: vec![],
            await_points: vec![
                AwaitPoint {
                    id: 0,
                    expr: Expression::Integer(42),
                    result_var: None,
                }
            ],
            state_blocks: std::collections::HashMap::new(),
            original_item: crate::parser::ast::Item::Function {
                name: "test".to_string(),
                generics: vec![],
                params: vec![],
                return_type: crate::parser::ast::Type::Named("i32".to_string()),
                body: vec![],
                is_unsafe: false,
                is_async: false,
                is_pub: false,
                attributes: vec![],
                where_clause: None,
                abi: None,
            },
        };

        // Should return StateId(1) for first await
        let next_state = get_next_state(&machine);
        assert!(next_state.is_some());
        assert_eq!(next_state.unwrap(), StateId(1));
    }

    #[test]
    fn test_hir_type_to_string_comprehensive() {
        // Test all supported types
        let test_cases = vec![
            (HirType::Int32, "i32"),
            (HirType::Int64, "i64"),
            (HirType::UInt32, "u32"),
            (HirType::UInt64, "u64"),
            (HirType::USize, "usize"),
            (HirType::ISize, "isize"),
            (HirType::Float64, "f64"),
            (HirType::Bool, "bool"),
            (HirType::Char, "char"),
            (HirType::String, "String"),
            (HirType::Named("MyType".to_string()), "MyType"),
            (HirType::Unknown, "Unknown"),
        ];

        for (ty, expected) in test_cases {
            assert_eq!(hir_type_to_string(&ty), expected,
                      "Failed for {:?}", ty);
        }
    }

    #[test]
    fn test_hir_type_to_string_complex() {
        // Test complex types
        let reference_type = HirType::Reference(Box::new(HirType::Int32));
        assert_eq!(hir_type_to_string(&reference_type), "&i32");

        let mutable_ref = HirType::MutableReference(Box::new(HirType::String));
        assert_eq!(hir_type_to_string(&mutable_ref), "&mut String");

        let array_type = HirType::Array {
            element_type: Box::new(HirType::Int32),
            size: Some(10),
        };
        assert_eq!(hir_type_to_string(&array_type), "[i32; 10]");

        let vec_type = HirType::Vec(Box::new(HirType::String));
        assert_eq!(hir_type_to_string(&vec_type), "Vec<String>");

        let option_type = HirType::Option(Box::new(HirType::Int32));
        assert_eq!(hir_type_to_string(&option_type), "Option<i32>");

        let result_type = HirType::Result {
            ok_type: Box::new(HirType::String),
            err_type: Box::new(HirType::Int32),
        };
        assert_eq!(hir_type_to_string(&result_type), "Result<String, i32>");

        let dyn_trait = HirType::DynTrait {
            trait_name: "Display".to_string(),
        };
        assert_eq!(hir_type_to_string(&dyn_trait), "dyn Display");
    }

    #[test]
    fn test_capitalize_first_comprehensive() {
        let test_cases = vec![
            ("hello", "Hello"),
            ("world", "World"),
            ("a", "A"),
            ("ABC", "ABC"),
            ("123test", "123test"),
            ("", ""),
            ("üñíçødé", "Üñíçødé"), // Unicode handling
        ];

        for (input, expected) in test_cases {
            assert_eq!(capitalize_first(input), expected,
                      "Failed for input: {}", input);
        }
    }

    #[test]
    #[should_panic(expected = "Invalid future state")]
    fn test_generate_async_poll_function_invalid_state() {
        // This test would need a state machine with invalid state
        // For now, just document that invalid states should panic
        // In a real implementation, we'd generate code that panics on invalid states
    }
}
