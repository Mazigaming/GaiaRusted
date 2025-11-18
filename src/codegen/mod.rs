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
pub mod simd;

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
                    write!(f, "qword ptr [{}]", base)
                } else if *offset > 0 {
                    write!(f, "qword ptr [{} + {}]", base, offset)
                } else {
                    write!(f, "qword ptr [{} - {}]", base, -offset)
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
    /// lea dst, [label]
    Lea { dst: X86Operand, src: String },
    /// add dst, src
    Add { dst: X86Operand, src: X86Operand },
    /// sub dst, src
    Sub { dst: X86Operand, src: X86Operand },
    /// imul dst, src
    IMul { dst: X86Operand, src: X86Operand },
    /// idiv src (divides RDX:RAX by src, result in RAX, remainder in RDX)
    IDiv { src: X86Operand },
    /// xor dst, src
    Xor { dst: X86Operand, src: X86Operand },
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
    /// sete dst (set if equal)
    Sete { dst: X86Operand },
    /// setne dst (set if not equal)
    Setne { dst: X86Operand },
    /// setl dst (set if less)
    Setl { dst: X86Operand },
    /// setle dst (set if less or equal)
    Setle { dst: X86Operand },
    /// setg dst (set if greater)
    Setg { dst: X86Operand },
    /// setge dst (set if greater or equal)
    Setge { dst: X86Operand },
    /// call function
    Call { func: String },
    /// ret
    Ret,
    /// movzx dst, src - move with zero extension
    Movzx { dst: Register, src: Register },
    /// push reg
    Push { reg: Register },
    /// pop reg
    Pop { reg: Register },
    /// Label
    Label { name: String },
    /// nop (no operation)
    Nop,
    /// cqo (sign extend RAX into RDX:RAX)
    Cqo,
    /// neg dst (negate)
    Neg { dst: X86Operand },
    /// and dst, src
    And { dst: X86Operand, src: X86Operand },
    /// or dst, src
    Or { dst: X86Operand, src: X86Operand },
    /// shl dst, src (shift left)
    Shl { dst: X86Operand, src: X86Operand },
    /// shr dst, src (shift right logical)
    Shr { dst: X86Operand, src: X86Operand },
    /// sar dst, src (shift right arithmetic)
    Sar { dst: X86Operand, src: X86Operand },
}

impl fmt::Display for X86Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            X86Instruction::Mov { dst, src } => write!(f, "    mov {}, {}", dst, src),
            X86Instruction::Lea { dst, src } => write!(f, "    lea {}, [rip + {}]", dst, src),
            X86Instruction::Add { dst, src } => write!(f, "    add {}, {}", dst, src),
            X86Instruction::Sub { dst, src } => write!(f, "    sub {}, {}", dst, src),
            X86Instruction::IMul { dst, src } => write!(f, "    imul {}, {}", dst, src),
            X86Instruction::IDiv { src } => write!(f, "    idiv {}", src),
            X86Instruction::Xor { dst, src } => write!(f, "    xor {}, {}", dst, src),
            X86Instruction::Cmp { dst, src } => write!(f, "    cmp {}, {}", dst, src),
            X86Instruction::Jmp { label } => write!(f, "    jmp {}", label),
            X86Instruction::Je { label } => write!(f, "    je {}", label),
            X86Instruction::Jne { label } => write!(f, "    jne {}", label),
            X86Instruction::Jl { label } => write!(f, "    jl {}", label),
            X86Instruction::Jle { label } => write!(f, "    jle {}", label),
            X86Instruction::Jg { label } => write!(f, "    jg {}", label),
            X86Instruction::Jge { label } => write!(f, "    jge {}", label),
            X86Instruction::Sete { dst } => {
                let operand = match dst {
                    X86Operand::Register(Register::RAX) => "al".to_string(),
                    X86Operand::Register(Register::RBX) => "bl".to_string(),
                    X86Operand::Register(Register::RCX) => "cl".to_string(),
                    X86Operand::Register(Register::RDX) => "dl".to_string(),
                    _ => format!("{}", dst),
                };
                write!(f, "    sete {}", operand)
            }
            X86Instruction::Setne { dst } => {
                let operand = match dst {
                    X86Operand::Register(Register::RAX) => "al".to_string(),
                    X86Operand::Register(Register::RBX) => "bl".to_string(),
                    X86Operand::Register(Register::RCX) => "cl".to_string(),
                    X86Operand::Register(Register::RDX) => "dl".to_string(),
                    _ => format!("{}", dst),
                };
                write!(f, "    setne {}", operand)
            }
            X86Instruction::Setl { dst } => {
                let operand = match dst {
                    X86Operand::Register(Register::RAX) => "al".to_string(),
                    X86Operand::Register(Register::RBX) => "bl".to_string(),
                    X86Operand::Register(Register::RCX) => "cl".to_string(),
                    X86Operand::Register(Register::RDX) => "dl".to_string(),
                    _ => format!("{}", dst),
                };
                write!(f, "    setl {}", operand)
            }
            X86Instruction::Setle { dst } => {
                let operand = match dst {
                    X86Operand::Register(Register::RAX) => "al".to_string(),
                    X86Operand::Register(Register::RBX) => "bl".to_string(),
                    X86Operand::Register(Register::RCX) => "cl".to_string(),
                    X86Operand::Register(Register::RDX) => "dl".to_string(),
                    _ => format!("{}", dst),
                };
                write!(f, "    setle {}", operand)
            }
            X86Instruction::Setg { dst } => {
                let operand = match dst {
                    X86Operand::Register(Register::RAX) => "al".to_string(),
                    X86Operand::Register(Register::RBX) => "bl".to_string(),
                    X86Operand::Register(Register::RCX) => "cl".to_string(),
                    X86Operand::Register(Register::RDX) => "dl".to_string(),
                    _ => format!("{}", dst),
                };
                write!(f, "    setg {}", operand)
            }
            X86Instruction::Setge { dst } => {
                let operand = match dst {
                    X86Operand::Register(Register::RAX) => "al".to_string(),
                    X86Operand::Register(Register::RBX) => "bl".to_string(),
                    X86Operand::Register(Register::RCX) => "cl".to_string(),
                    X86Operand::Register(Register::RDX) => "dl".to_string(),
                    _ => format!("{}", dst),
                };
                write!(f, "    setge {}", operand)
            }
            X86Instruction::Call { func } => write!(f, "    call {}", func),
            X86Instruction::Ret => write!(f, "    ret"),
            X86Instruction::Movzx { dst, src } => write!(f, "    movzx {}, {}", dst, src),
            X86Instruction::Push { reg } => write!(f, "    push {}", reg),
            X86Instruction::Pop { reg } => write!(f, "    pop {}", reg),
            X86Instruction::Label { name } => write!(f, "{}:", name),
            X86Instruction::Nop => write!(f, "    nop"),
            X86Instruction::Cqo => write!(f, "    cqo"),
            X86Instruction::Neg { dst } => write!(f, "    neg {}", dst),
            X86Instruction::And { dst, src } => write!(f, "    and {}, {}", dst, src),
            X86Instruction::Or { dst, src } => write!(f, "    or {}, {}", dst, src),
            X86Instruction::Shl { dst, src } => write!(f, "    shl {}, {}", dst, src),
            X86Instruction::Shr { dst, src } => write!(f, "    shr {}, {}", dst, src),
            X86Instruction::Sar { dst, src } => write!(f, "    sar {}, {}", dst, src),
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
    fn allocate(&mut self, _var_idx: usize) -> RegisterLocation {
        self.stack_offset -= 8;
        RegisterLocation::Stack(self.stack_offset)
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
    var_locations: HashMap<String, i64>,
    stack_offset: i64,
    string_constants: HashMap<String, String>,
}

impl Codegen {
    /// Create a new codegen
    pub fn new() -> Self {
        Codegen {
            instructions: Vec::new(),
            label_counter: 0,
            var_locations: HashMap::new(),
            stack_offset: -8,
            string_constants: HashMap::new(),
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
        
        // Add rodata section with string constants
        if !self.string_constants.is_empty() {
            asm.push_str("\n.section .rodata\n");
            for (string, label) in &self.string_constants {
                let escaped = string
                    .replace("\\", "\\\\")
                    .replace("\"", "\\\"")
                    .replace("\n", "\\n")
                    .replace("\t", "\\t")
                    .replace("\r", "\\r");
                asm.push_str(&format!("    {}: .string \"{}\"\n", label, escaped));
            }
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
        // Reset per-function state
        self.var_locations.clear();
        self.stack_offset = -8;
        
        // Mangle function names for assembly compatibility
        // Replace :: with _impl_ for qualified names like Point::new
        let func_name = if func.name == "main" {
            "gaia_main".to_string()
        } else if func.name.contains("::") {
            // Mangle qualified names: Point::new -> Point_impl_new
            func.name.replace("::", "_impl_")
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
        
        // Load parameters from incoming registers to their allocated locations
        let param_regs = vec![Register::RDI, Register::RSI, Register::RDX, Register::RCX, Register::R8, Register::R9];
        for (i, param_reg) in param_regs.iter().enumerate() {
            if i < func.params.len() {
                let offset = -8 - (i as i64 * 8);
                self.instructions.push(X86Instruction::Mov {
                    dst: X86Operand::Register(Register::RAX),
                    src: X86Operand::Register(*param_reg),
                });
                self.instructions.push(X86Instruction::Mov {
                    dst: X86Operand::Memory { base: Register::RBP, offset },
                    src: X86Operand::Register(Register::RAX),
                });
                let (param_name, _param_type) = &func.params[i];
                self.var_locations.insert(param_name.clone(), offset);
            }
        }
        
        // Load stack-passed parameters (arg 6+)
        for i in 6..func.params.len() {
            let stack_offset = 16 + ((i - 6) as i64 * 8);
            let frame_offset = -8 - (i as i64 * 8);
            self.instructions.push(X86Instruction::Mov {
                dst: X86Operand::Register(Register::RAX),
                src: X86Operand::Memory { base: Register::RBP, offset: stack_offset },
            });
            self.instructions.push(X86Instruction::Mov {
                dst: X86Operand::Memory { base: Register::RBP, offset: frame_offset },
                src: X86Operand::Register(Register::RAX),
            });
            let (param_name, _param_type) = &func.params[i];
            self.var_locations.insert(param_name.clone(), frame_offset);
        }
        
        // Update stack_offset to allocate space after all parameters
        if func.params.len() > 0 {
            self.stack_offset = -8 - (func.params.len() as i64 * 8);
        }
        
        // Generate code for each basic block
        for (block_idx, block) in func.basic_blocks.iter().enumerate() {
            self.instructions.push(X86Instruction::Label {
                name: format!("{}_bb{}", func_name, block_idx),
            });
            
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
                Terminator::Return(Some(operand)) => {
                    // Move return value to RAX
                    if let Ok(operand_x86) = self.operand_to_x86(operand) {
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: operand_x86,
                        });
                    }
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
                match operand {
                    crate::mir::Operand::Constant(crate::mir::Constant::String(s)) => {
                        let label = self.allocate_string(s.clone());
                        self.instructions.push(X86Instruction::Lea {
                            dst: X86Operand::Register(Register::RAX),
                            src: label,
                        });
                    }
                    _ => {
                        let src = self.operand_to_x86(operand)?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src,
                        });
                    }
                }
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
                        self.instructions.push(X86Instruction::Cqo);
                        self.instructions.push(X86Instruction::IDiv {
                            src: right_val,
                        });
                    }
                    crate::lowering::BinaryOp::Modulo => {
                        self.instructions.push(X86Instruction::Cqo);
                        self.instructions.push(X86Instruction::IDiv {
                            src: right_val,
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RDX),
                        });
                    }
                    crate::lowering::BinaryOp::Equal => {
                        self.instructions.push(X86Instruction::Cmp {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RCX),
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Sete {
                            dst: X86Operand::Register(Register::RCX),
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RCX),
                        });
                    }
                    crate::lowering::BinaryOp::NotEqual => {
                        self.instructions.push(X86Instruction::Cmp {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RCX),
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Setne {
                            dst: X86Operand::Register(Register::RCX),
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RCX),
                        });
                    }
                    crate::lowering::BinaryOp::Less => {
                        self.instructions.push(X86Instruction::Cmp {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RCX),
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Setl {
                            dst: X86Operand::Register(Register::RCX),
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RCX),
                        });
                    }
                    crate::lowering::BinaryOp::LessEqual => {
                        self.instructions.push(X86Instruction::Cmp {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RCX),
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Setle {
                            dst: X86Operand::Register(Register::RCX),
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RCX),
                        });
                    }
                    crate::lowering::BinaryOp::Greater => {
                        self.instructions.push(X86Instruction::Cmp {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RCX),
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Setg {
                            dst: X86Operand::Register(Register::RCX),
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RCX),
                        });
                    }
                    crate::lowering::BinaryOp::GreaterEqual => {
                        self.instructions.push(X86Instruction::Cmp {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RCX),
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Setge {
                            dst: X86Operand::Register(Register::RCX),
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RCX),
                        });
                    }
                    crate::lowering::BinaryOp::BitwiseAnd => {
                        self.instructions.push(X86Instruction::And {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                    }
                    crate::lowering::BinaryOp::BitwiseOr => {
                        self.instructions.push(X86Instruction::Or {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                    }
                    crate::lowering::BinaryOp::BitwiseXor => {
                        self.instructions.push(X86Instruction::Xor {
                            dst: X86Operand::Register(Register::RAX),
                            src: right_val,
                        });
                    }
                    crate::lowering::BinaryOp::And => {
                        self.instructions.push(X86Instruction::Cmp {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RCX),
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Setne {
                            dst: X86Operand::Register(Register::RCX),
                        });
                        self.instructions.push(X86Instruction::Cmp {
                            dst: right_val,
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Setne {
                            dst: X86Operand::Register(Register::RDX),
                        });
                        self.instructions.push(X86Instruction::And {
                            dst: X86Operand::Register(Register::RCX),
                            src: X86Operand::Register(Register::RDX),
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RCX),
                        });
                    }
                    crate::lowering::BinaryOp::Or => {
                        self.instructions.push(X86Instruction::Cmp {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RCX),
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Setne {
                            dst: X86Operand::Register(Register::RCX),
                        });
                        self.instructions.push(X86Instruction::Cmp {
                            dst: right_val,
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Setne {
                            dst: X86Operand::Register(Register::RDX),
                        });
                        self.instructions.push(X86Instruction::Or {
                            dst: X86Operand::Register(Register::RCX),
                            src: X86Operand::Register(Register::RDX),
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RCX),
                        });
                    }
                    crate::lowering::BinaryOp::LeftShift => {
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RCX),
                            src: right_val,
                        });
                        self.instructions.push(X86Instruction::Shl {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RCX),
                        });
                    }
                    crate::lowering::BinaryOp::RightShift => {
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RCX),
                            src: right_val,
                        });
                        self.instructions.push(X86Instruction::Sar {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RCX),
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
                        self.instructions.push(X86Instruction::Neg {
                            dst: X86Operand::Register(Register::RAX),
                        });
                    }
                    crate::lowering::UnaryOp::Not => {
                        self.instructions.push(X86Instruction::Cmp {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RCX),
                            src: X86Operand::Immediate(0),
                        });
                        self.instructions.push(X86Instruction::Sete {
                            dst: X86Operand::Register(Register::RCX),
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RCX),
                        });
                    }
                    _ => {}
                }
            }
            crate::mir::Rvalue::Call(func_name, args) => {
                // Check if this is an enum constructor (like Ok, Some, Err, None)
                // Enum constructors start with a capital letter and may have a :: for path
                let is_enum_constructor = {
                    let parts: Vec<&str> = func_name.split("::").collect();
                    let last_part = parts.last().map(|s| *s).unwrap_or(func_name.as_str());
                    last_part.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) &&
                    !last_part.starts_with("_enum_constructor")
                };
                
                if is_enum_constructor && !args.is_empty() {
                    // For enum constructors with arguments, just move the first argument to RAX
                    // This is a simplified implementation - proper enums would wrap the value
                    let arg_val = self.operand_to_x86(&args[0])?;
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: arg_val,
                    });
                } else if is_enum_constructor && args.is_empty() {
                    // Unit enum variants - return 0 (or a special tag)
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(0),
                    });
                } else {
                    // Regular function call
                    // Mangle function names for assembly compatibility
                    let mangled_func_name = if func_name.contains("::") {
                        // Mangle qualified names: Point::new -> Point_impl_new
                        func_name.replace("::", "_impl_")
                    } else {
                        func_name.clone()
                    };
                    
                    let mut stack_adjust = 0;
                    for (i, arg) in args.iter().enumerate() {
                        let arg_val = self.operand_to_x86(arg)?;
                        match i {
                            0 => {
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::RDI),
                                    src: arg_val,
                                });
                            }
                            1 => {
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::RSI),
                                    src: arg_val,
                                });
                            }
                            2 => {
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::RDX),
                                    src: arg_val,
                                });
                            }
                            3 => {
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::RCX),
                                    src: arg_val,
                                });
                            }
                            4 => {
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::R8),
                                    src: arg_val,
                                });
                            }
                            5 => {
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::R9),
                                    src: arg_val,
                                });
                            }
                            _ => {
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::RAX),
                                    src: arg_val,
                                });
                                self.instructions.push(X86Instruction::Push {
                                    reg: Register::RAX,
                                });
                                stack_adjust += 8;
                            }
                        }
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: mangled_func_name,
                    });
                    if stack_adjust > 0 {
                        self.instructions.push(X86Instruction::Add {
                            dst: X86Operand::Register(Register::RSP),
                            src: X86Operand::Immediate(stack_adjust),
                        });
                    }
                }
            }
            _ => {
                self.instructions.push(X86Instruction::Nop);
            }
        }
        
        if let crate::mir::Place::Local(ref name) = stmt.place {
            let offset = self.get_var_location(name);
            self.instructions.push(X86Instruction::Mov {
                dst: X86Operand::Memory { base: Register::RBP, offset },
                src: X86Operand::Register(Register::RAX),
            });
        }
        Ok(())
    }
    
    /// Convert an operand to x86 operand
    fn operand_to_x86(&self, operand: &crate::mir::Operand) -> CodegenResult<X86Operand> {
        match operand {
            crate::mir::Operand::Constant(crate::mir::Constant::Integer(n)) => {
                Ok(X86Operand::Immediate(*n))
            }
            crate::mir::Operand::Constant(crate::mir::Constant::Float(_f)) => {
                Ok(X86Operand::Register(Register::RAX))
            }
            crate::mir::Operand::Constant(crate::mir::Constant::Bool(b)) => {
                Ok(X86Operand::Immediate(if *b { 1 } else { 0 }))
            }
            crate::mir::Operand::Constant(crate::mir::Constant::String(_s)) => {
                Ok(X86Operand::Register(Register::RAX))
            }
            crate::mir::Operand::Constant(crate::mir::Constant::Unit) => {
                Ok(X86Operand::Immediate(0))
            }
            crate::mir::Operand::Copy(crate::mir::Place::Local(name)) | crate::mir::Operand::Move(crate::mir::Place::Local(name)) => {
                if let Some(offset) = self.var_locations.get(name) {
                    Ok(X86Operand::Memory { base: Register::RBP, offset: *offset })
                } else {
                    Ok(X86Operand::Register(Register::RAX))
                }
            }
            crate::mir::Operand::Copy(_place) | crate::mir::Operand::Move(_place) => {
                Ok(X86Operand::Register(Register::RAX))
            }
        }
    }

    /// Generate a new label
    fn new_label(&mut self) -> String {
        let label = format!("L{}", self.label_counter);
        self.label_counter += 1;
        label
    }

    /// Allocate stack space for a variable
    fn allocate_var(&mut self, var_name: String) -> i64 {
        if !self.var_locations.contains_key(&var_name) {
            let offset = self.stack_offset;
            self.var_locations.insert(var_name, offset);
            self.stack_offset -= 8;
            offset
        } else {
            self.var_locations[&var_name]
        }
    }

    /// Get or allocate stack location for a variable
    fn get_var_location(&mut self, var_name: &str) -> i64 {
        if !self.var_locations.contains_key(var_name) {
            self.allocate_var(var_name.to_string())
        } else {
            self.var_locations[var_name]
        }
    }

    /// Allocate a label for a string constant
    fn allocate_string(&mut self, string: String) -> String {
        if let Some(label) = self.string_constants.get(&string) {
            label.clone()
        } else {
            let label = format!("str_{}", self.label_counter);
            self.label_counter += 1;
            self.string_constants.insert(string, label.clone());
            label
        }
    }
}

/// Generate x86-64 assembly from MIR
pub fn generate_code(mir: &Mir) -> CodegenResult<String> {
    let mut codegen = Codegen::new();
    codegen.generate(mir)
}
