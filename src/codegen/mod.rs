//! # Phase 8-10: CODE GENERATION & OBJECT FILE GENERATION
//!
//! Phases 8, 9, and 10 combined:
//! - Phase 8: Convert optimized MIR to x86-64 assembly
//! - Phase 9: Generate object files (ELF format)
//! - Phase 10: Testing, optimization, CLI polish
//!
//! ## What we do:
//! - Convert MIR to x86-64 instructions
//! - Register allocation
//! - Stack frame management
//! - Function calling conventions (System V AMD64 ABI)
//! - Generate assembly instructions
//!
//! ## System V AMD64 ABI:
//! - First 6 integer/pointer arguments: RDI, RSI, RDX, RCX, R8, R9
//! - Return value: RAX (and RDX for 128-bit)
//! - Caller-saved: RAX, RCX, RDX, RSI, RDI, R8-R11
//! - Callee-saved: RBX, RSP, RBP, R12-R15
//! - RSP must be 16-byte aligned before call instruction

pub mod object;
pub mod monomorphization;
pub mod backend;
pub mod optimization;

use crate::mir::{Mir, MirFunction, Statement, Terminator};
use crate::runtime;
use std::collections::HashMap;
use std::fmt;

/// Code generation error
#[derive(Debug, Clone)]
pub struct CodegenError {
    pub message: String,
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

type CodegenResult<T> = Result<T, CodegenError>;

/// x86-64 registers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Register {
    // Caller-saved (volatile)
    RAX,
    RCX,
    RDX,
    RSI,
    RDI,
    R8,
    R9,
    R10,
    R11,
    
    // Callee-saved (non-volatile)
    RBX,
    RBP,
    R12,
    R13,
    R14,
    R15,
    
    // Special
    RSP, // Stack pointer
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Register::RAX => write!(f, "rax"),
            Register::RCX => write!(f, "rcx"),
            Register::RDX => write!(f, "rdx"),
            Register::RSI => write!(f, "rsi"),
            Register::RDI => write!(f, "rdi"),
            Register::R8 => write!(f, "r8"),
            Register::R9 => write!(f, "r9"),
            Register::R10 => write!(f, "r10"),
            Register::R11 => write!(f, "r11"),
            Register::RBX => write!(f, "rbx"),
            Register::RBP => write!(f, "rbp"),
            Register::R12 => write!(f, "r12"),
            Register::R13 => write!(f, "r13"),
            Register::R14 => write!(f, "r14"),
            Register::R15 => write!(f, "r15"),
            Register::RSP => write!(f, "rsp"),
        }
    }
}

/// x86-64 operand
#[derive(Debug, Clone)]
pub enum X86Operand {
    Register(Register),
    Immediate(i64),
    Memory { base: Register, offset: i64 },
}

impl fmt::Display for X86Operand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            X86Operand::Register(reg) => write!(f, "{}", reg),
            X86Operand::Immediate(val) => write!(f, "{}", val),
            X86Operand::Memory { base, offset } => {
                if *offset == 0 {
                    write!(f, "[{}]", base)
                } else if *offset > 0 {
                    write!(f, "[{} + {}]", base, offset)
                } else {
                    write!(f, "[{} - {}]", base, -offset)
                }
            }
        }
    }
}

/// x86-64 instruction
#[derive(Debug, Clone)]
pub enum X86Instruction {
    /// mov dst, src
    Mov { dst: X86Operand, src: X86Operand },
    /// add dst, src
    Add { dst: X86Operand, src: X86Operand },
    /// sub dst, src
    Sub { dst: X86Operand, src: X86Operand },
    /// imul dst, src
    IMul { dst: X86Operand, src: X86Operand },
    /// idiv src (divides RDX:RAX by src, result in RAX, remainder in RDX)
    IDiv { src: X86Operand },
    /// cmp dst, src
    Cmp { dst: X86Operand, src: X86Operand },
    /// jmp label
    Jmp { label: String },
    /// je label (jump if equal)
    Je { label: String },
    /// jne label (jump if not equal)
    Jne { label: String },
    /// jl label (jump if less)
    Jl { label: String },
    /// jle label (jump if less or equal)
    Jle { label: String },
    /// jg label (jump if greater)
    Jg { label: String },
    /// jge label (jump if greater or equal)
    Jge { label: String },
    /// call function
    Call { func: String },
    /// ret
    Ret,
    /// push reg
    Push { reg: Register },
    /// pop reg
    Pop { reg: Register },
    /// Label
    Label { name: String },
    /// nop (no operation)
    Nop,
}

impl fmt::Display for X86Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            X86Instruction::Mov { dst, src } => write!(f, "    mov {}, {}", dst, src),
            X86Instruction::Add { dst, src } => write!(f, "    add {}, {}", dst, src),
            X86Instruction::Sub { dst, src } => write!(f, "    sub {}, {}", dst, src),
            X86Instruction::IMul { dst, src } => write!(f, "    imul {}, {}", dst, src),
            X86Instruction::IDiv { src } => write!(f, "    idiv {}", src),
            X86Instruction::Cmp { dst, src } => write!(f, "    cmp {}, {}", dst, src),
            X86Instruction::Jmp { label } => write!(f, "    jmp {}", label),
            X86Instruction::Je { label } => write!(f, "    je {}", label),
            X86Instruction::Jne { label } => write!(f, "    jne {}", label),
            X86Instruction::Jl { label } => write!(f, "    jl {}", label),
            X86Instruction::Jle { label } => write!(f, "    jle {}", label),
            X86Instruction::Jg { label } => write!(f, "    jg {}", label),
            X86Instruction::Jge { label } => write!(f, "    jge {}", label),
            X86Instruction::Call { func } => write!(f, "    call {}", func),
            X86Instruction::Ret => write!(f, "    ret"),
            X86Instruction::Push { reg } => write!(f, "    push {}", reg),
            X86Instruction::Pop { reg } => write!(f, "    pop {}", reg),
            X86Instruction::Label { name } => write!(f, "{}:", name),
            X86Instruction::Nop => write!(f, "    nop"),
        }
    }
}

/// Register allocator state
struct RegisterAllocator {
    /// Maps local variable index to register or stack offset
    var_locations: HashMap<usize, RegisterLocation>,
    /// Next available stack offset (growing downward from RBP)
    stack_offset: i64,
    /// Argument registers
    arg_registers: Vec<Register>,
}

#[derive(Debug, Clone)]
enum RegisterLocation {
    Register(Register),
    Stack(i64), // offset from RBP
}

impl RegisterAllocator {
    fn new() -> Self {
        RegisterAllocator {
            var_locations: HashMap::new(),
            stack_offset: 0,
            arg_registers: vec![Register::RDI, Register::RSI, Register::RDX, Register::RCX, Register::R8, Register::R9],
        }
    }

    /// Allocate a location for a variable
    fn allocate(&mut self, var_idx: usize) -> RegisterLocation {
        // Simple strategy: use registers first, then stack
        if var_idx < self.arg_registers.len() {
            RegisterLocation::Register(self.arg_registers[var_idx])
        } else {
            self.stack_offset -= 8;
            RegisterLocation::Stack(self.stack_offset)
        }
    }

    /// Get the location of a variable
    fn get_location(&self, var_idx: usize) -> Option<RegisterLocation> {
        self.var_locations.get(&var_idx).cloned()
    }
}

/// x86-64 code generator
pub struct Codegen {
    instructions: Vec<X86Instruction>,
    label_counter: usize,
}

impl Codegen {
    /// Create a new codegen
    pub fn new() -> Self {
        Codegen {
            instructions: Vec::new(),
            label_counter: 0,
        }
    }

    /// Generate code for entire program
    pub fn generate(&mut self, mir: &Mir) -> CodegenResult<String> {
        let mut asm = String::new();
        
        // Assembly header
        asm.push_str(".intel_syntax noprefix\n");
        asm.push_str(".text\n");
        asm.push_str(".globl gaia_main\n");
        asm.push_str(".globl main\n\n");
        
        // Generate code for each function
        for func in &mir.functions {
            self.generate_function(func)?;
        }
        
        // Convert instructions to assembly
        for instr in &self.instructions {
            asm.push_str(&format!("{}\n", instr));
        }
        
        // Include runtime support
        asm.push_str("\n");
        asm.push_str(&runtime::generate_main_wrapper());
        asm.push_str("\n");
        asm.push_str(&runtime::generate_runtime_assembly());
        
        Ok(asm)
    }

    /// Generate code for a function
    fn generate_function(&mut self, func: &MirFunction) -> CodegenResult<()> {
        // Rename main to gaia_main for runtime wrapper
        let func_name = if func.name == "main" {
            "gaia_main".to_string()
        } else {
            func.name.clone()
        };
        
        // Function label
        self.instructions.push(X86Instruction::Label {
            name: func_name.clone(),
        });
        
        // Function prologue
        self.instructions.push(X86Instruction::Push { reg: Register::RBP });
        self.instructions.push(X86Instruction::Mov {
            dst: X86Operand::Register(Register::RBP),
            src: X86Operand::Register(Register::RSP),
        });
        
        // Allocate space for locals (parameters)
        let mut allocator = RegisterAllocator::new();
        for i in 0..func.params.len() {
            let loc = allocator.allocate(i);
            allocator.var_locations.insert(i, loc);
        }
        
        // Generate code for each basic block
        for (block_idx, block) in func.basic_blocks.iter().enumerate() {
            if block_idx > 0 {
                self.instructions.push(X86Instruction::Label {
                    name: format!("{}_bb{}", func_name, block_idx),
                });
            }
            
            // Generate statements
            for stmt in &block.statements {
                self.generate_statement(stmt, &allocator)?;
            }
            
            // Generate terminator
            match &block.terminator {
                Terminator::Return(None) => {
                    // Return void - set RAX to 0
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(0),
                    });
                    self.instructions.push(X86Instruction::Pop { reg: Register::RBP });
                    self.instructions.push(X86Instruction::Ret);
                }
                Terminator::Return(Some(_operand)) => {
                    // Return value should be in RAX already
                    self.instructions.push(X86Instruction::Pop { reg: Register::RBP });
                    self.instructions.push(X86Instruction::Ret);
                }
                Terminator::Goto(target) => {
                    self.instructions.push(X86Instruction::Jmp {
                        label: format!("{}_bb{}", func_name, target),
                    });
                }
                Terminator::If(cond, then_target, else_target) => {
                    match cond {
                        crate::mir::Operand::Constant(crate::mir::Constant::Bool(b)) => {
                            if *b {
                                self.instructions.push(X86Instruction::Jmp {
                                    label: format!("{}_bb{}", func_name, then_target),
                                });
                            } else {
                                self.instructions.push(X86Instruction::Jmp {
                                    label: format!("{}_bb{}", func_name, else_target),
                                });
                            }
                        }
                        _ => {
                            if let Ok(cond_operand) = self.operand_to_x86(cond) {
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::RAX),
                                    src: cond_operand,
                                });
                                self.instructions.push(X86Instruction::Cmp {
                                    dst: X86Operand::Register(Register::RAX),
                                    src: X86Operand::Immediate(0),
                                });
                                self.instructions.push(X86Instruction::Jne {
                                    label: format!("{}_bb{}", func_name, then_target),
                                });
                                self.instructions.push(X86Instruction::Jmp {
                                    label: format!("{}_bb{}", func_name, else_target),
                                });
                            }
                        }
                    }
                }
                Terminator::Unreachable => {
                    self.instructions.push(X86Instruction::Nop);
                }
            }
        }
        
        Ok(())
    }

    /// Generate code for a statement
    fn generate_statement(&mut self, stmt: &Statement, _allocator: &RegisterAllocator) -> CodegenResult<()> {
        match &stmt.rvalue {
            crate::mir::Rvalue::Use(operand) => {
                let src = self.operand_to_x86(operand)?;
                self.instructions.push(X86Instruction::Mov {
                    dst: X86Operand::Register(Register::RAX),
                    src,
                });
            }
            crate::mir::Rvalue::BinaryOp(op, left, right) => {
                let left_val = self.operand_to_x86(left)?;
                let right_val = self.operand_to_x86(right)?;
                
                self.instructions.push(X86Instruction::Mov {
                    dst: X86Operand::Register(Register::RAX),
                    src: left_val,
                });
                
                match op {
                    crate::lowering::BinaryOp::Add => {
                        self.instructions.push(X86Instruction::Add {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                    }
                    crate::lowering::BinaryOp::Subtract => {
                        self.instructions.push(X86Instruction::Sub {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                    }
                    crate::lowering::BinaryOp::Multiply => {
                        self.instructions.push(X86Instruction::IMul {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                    }
                    crate::lowering::BinaryOp::Divide => {
                        self.instructions.push(X86Instruction::IDiv {
                            src: right_val,
                        });
                    }
                    crate::lowering::BinaryOp::Equal | crate::lowering::BinaryOp::NotEqual |
                    crate::lowering::BinaryOp::Less | crate::lowering::BinaryOp::LessEqual |
                    crate::lowering::BinaryOp::Greater | crate::lowering::BinaryOp::GreaterEqual => {
                        self.instructions.push(X86Instruction::Cmp {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                    }
                    _ => {
                        self.instructions.push(X86Instruction::Nop);
                    }
                }
            }
            crate::mir::Rvalue::UnaryOp(op, operand) => {
                let src = self.operand_to_x86(operand)?;
                self.instructions.push(X86Instruction::Mov {
                    dst: X86Operand::Register(Register::RAX),
                    src,
                });
                match op {
                    crate::lowering::UnaryOp::Negate => {
                        self.instructions.push(X86Instruction::Sub {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Immediate(0),
                        });
                    }
                    crate::lowering::UnaryOp::Not => {
                        self.instructions.push(X86Instruction::Cmp {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Immediate(0),
                        });
                    }
                    _ => {}
                }
            }
            _ => {
                self.instructions.push(X86Instruction::Nop);
            }
        }
        Ok(())
    }
    
    /// Convert an operand to x86 operand
    fn operand_to_x86(&self, operand: &crate::mir::Operand) -> CodegenResult<X86Operand> {
        match operand {
            crate::mir::Operand::Constant(crate::mir::Constant::Integer(n)) => {
                Ok(X86Operand::Immediate(*n))
            }
            crate::mir::Operand::Constant(crate::mir::Constant::Bool(b)) => {
                Ok(X86Operand::Immediate(if *b { 1 } else { 0 }))
            }
            crate::mir::Operand::Copy(_place) | crate::mir::Operand::Move(_place) => {
                Ok(X86Operand::Register(Register::RAX))
            }
            _ => Err(CodegenError {
                message: "Unsupported operand type".to_string(),
            })
        }
    }

    /// Generate a new label
    fn new_label(&mut self) -> String {
        let label = format!("L{}", self.label_counter);
        self.label_counter += 1;
        label
    }
}

/// Generate x86-64 assembly from MIR
pub fn generate_code(mir: &Mir) -> CodegenResult<String> {
    let mut codegen = Codegen::new();
    codegen.generate(mir)
}