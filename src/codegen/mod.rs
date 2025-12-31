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
pub mod trait_monomorphization;
pub mod backend;
pub mod optimization;
pub mod simd;
pub mod iterator_fusion;

use crate::mir::{Mir, MirFunction, Statement, Terminator};
use crate::runtime;
use crate::lowering::get_struct_field_index;
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
    /// movsd dst, src (move scalar double precision floating point)
    Movsd { dst: String, src: String },
    /// addsd dst, src (add scalar double precision floating point)
    Addsd { dst: String, src: String },
    /// subsd dst, src (subtract scalar double precision floating point)
    Subsd { dst: String, src: String },
    /// mulsd dst, src (multiply scalar double precision floating point)
    Mulsd { dst: String, src: String },
    /// divsd dst, src (divide scalar double precision floating point)
    Divsd { dst: String, src: String },
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
            X86Instruction::Movsd { dst, src } => write!(f, "    movsd {}, {}", dst, src),
            X86Instruction::Addsd { dst, src } => write!(f, "    addsd {}, {}", dst, src),
            X86Instruction::Subsd { dst, src } => write!(f, "    subsd {}, {}", dst, src),
            X86Instruction::Mulsd { dst, src } => write!(f, "    mulsd {}, {}", dst, src),
            X86Instruction::Divsd { dst, src } => write!(f, "    divsd {}, {}", dst, src),
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
    /// For struct variables: maps var name to the offset where struct data is stored
    struct_data_locations: HashMap<String, i64>,
    /// Track which stack offsets contain float values
    float_stack_offsets: std::collections::HashSet<i64>,
    stack_offset: i64,
    /// Minimum stack offset used by collections (don't allocate above this)
    min_collection_offset: i64,
    /// Size of the collection at min_collection_offset (for proper collision detection)
    collection_size: i64,
    string_constants: HashMap<String, String>,
    float_constants: HashMap<String, f64>, // label -> f64 value
    /// Maps variable name to struct name (for field index lookup)
    var_struct_types: HashMap<String, String>,
}

impl Codegen {
    /// Create a new codegen
    pub fn new() -> Self {
        Codegen {
            instructions: Vec::new(),
            label_counter: 0,
            var_locations: HashMap::new(),
            struct_data_locations: HashMap::new(),
            float_stack_offsets: std::collections::HashSet::new(),
            stack_offset: -8,
            min_collection_offset: i64::MAX,
            float_constants: HashMap::new(),
            collection_size: 0,
            string_constants: HashMap::new(),
            var_struct_types: HashMap::new(),
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
        
        // Add data section for mutable static variables
        if mir.globals.iter().any(|g| g.is_static && g.is_mutable) {
            asm.push_str("\n.section .data\n");
            for global in &mir.globals {
                if global.is_static && global.is_mutable {
                    asm.push_str(&format!("    {}: .quad {}\n", global.name, global.value));
                }
            }
        }
        
        // Add rodata section with string constants and const values
        let has_rodata_globals = mir.globals.iter().any(|g| !g.is_static || !g.is_mutable);
        if !self.string_constants.is_empty() || !self.float_constants.is_empty() || has_rodata_globals {
            asm.push_str("\n.section .rodata\n");
            
            // Add read-only globals (constants and immutable statics)
            for global in &mir.globals {
                if !global.is_static || !global.is_mutable {
                    asm.push_str(&format!("    {}: .quad {}\n", global.name, global.value));
                }
            }
            
            // Add float constants
            for (float_key, float_value) in &self.float_constants {
                // Use .quad to store 64-bit floating point as bits
                let bits = float_value.to_bits();
                asm.push_str(&format!("    {}: .quad {}\n", float_key, bits));
            }
            
            // Add string constants
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
        self.var_struct_types.clear();
        self.stack_offset = -8;
        self.min_collection_offset = i64::MAX;
        self.collection_size = 0;
        
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
        
        // Remember position of prologue so we can add stack allocation later
        let prologue_end_idx = self.instructions.len();
        
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
                
                if param_name == "self" && func.name.contains("::") {
                    let struct_name = func.name.split("::").next().unwrap_or("").to_string();
                    if !struct_name.is_empty() {
                        self.var_struct_types.insert(param_name.clone(), struct_name);
                    }
                }
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
            
            if param_name == "self" && func.name.contains("::") {
                let struct_name = func.name.split("::").next().unwrap_or("").to_string();
                if !struct_name.is_empty() {
                    self.var_struct_types.insert(param_name.clone(), struct_name);
                }
            }
        }
        
        // Update stack_offset to allocate space after all parameters
        if func.params.len() > 0 {
            self.stack_offset = -8 - (func.params.len() as i64 * 8);
        }
        
        // Generate code for each basic block
        for (block_idx, block) in func.basic_blocks.iter().enumerate() {
            eprintln!("[Codegen] Block {}: {} statements", block_idx, block.statements.len());
            for (stmt_idx, stmt) in block.statements.iter().enumerate() {
                eprintln!("[Codegen] Block {} Statement {}: {:?} = {:?}", block_idx, stmt_idx, stmt.place, stmt.rvalue);
            }
            
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
                    // Restore stack pointer before restoring RBP
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RSP),
                        src: X86Operand::Register(Register::RBP),
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
                    // Restore stack pointer before restoring RBP
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RSP),
                        src: X86Operand::Register(Register::RBP),
                    });
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
        
        // If we've allocated local stack space, add sub rsp instruction
        // The allocation must maintain 16-byte stack alignment before CALL instructions
        // After push rbp, RSP % 16 == 8
        // We need sub rsp with N where N % 16 == 0 to keep RSP % 16 == 8 before CALL
        if self.stack_offset < 0 {
            let mut total_alloc = -self.stack_offset;
            
            // Round up to nearest multiple of 16
            if total_alloc % 16 != 0 {
                total_alloc = ((total_alloc / 16) + 1) * 16;
            }
            
            // Insert sub rsp instruction right after prologue
            self.instructions.insert(prologue_end_idx, X86Instruction::Sub {
                dst: X86Operand::Register(Register::RSP),
                src: X86Operand::Immediate(total_alloc),
            });
        }
        
        Ok(())
    }

    /// Generate code for a statement
    fn generate_statement(&mut self, stmt: &Statement, _allocator: &RegisterAllocator) -> CodegenResult<()> {
        let mut skip_final_store = false;  // Track if we've already stored the result
        
        match &stmt.rvalue {
            crate::mir::Rvalue::Use(operand) => {
                // Check operand type for debugging
                let _is_field = matches!(operand, crate::mir::Operand::Copy(crate::mir::Place::Field(_, _)) | crate::mir::Operand::Move(crate::mir::Place::Field(_, _)));
                match operand {
                    crate::mir::Operand::Constant(crate::mir::Constant::String(s)) => {
                        let label = self.allocate_string(s.clone());
                        self.instructions.push(X86Instruction::Lea {
                            dst: X86Operand::Register(Register::RAX),
                            src: label,
                        });
                    }
                    crate::mir::Operand::Constant(crate::mir::Constant::Float(f)) => {
                        let label = self.allocate_float(*f);
                        // Load the address of the float constant
                        self.instructions.push(X86Instruction::Lea {
                            dst: X86Operand::Register(Register::RAX),
                            src: label,
                        });
                        // Load the float value from memory into RAX
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Memory { base: Register::RAX, offset: 0 },
                        });
                        
                        // Track that the destination stack location will contain a float
                        // We'll mark it as float AFTER storing to memory
                        skip_final_store = true;  // Will do custom store below
                        if let crate::mir::Place::Local(ref dst_name) = stmt.place {
                            let offset = self.get_var_location(dst_name);
                            self.float_stack_offsets.insert(offset);
                            // Store directly without final store
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Memory { base: Register::RBP, offset },
                                src: X86Operand::Register(Register::RAX),
                            });
                        }
                    }
                    crate::mir::Operand::Copy(crate::mir::Place::Field(place, field_name)) => {
                         eprintln!("[Codegen] Field access: Trying to access field '{}' from place {:?}", field_name, place);
                         // The struct variable holds the struct data or a POINTER to struct data
                         match place.as_ref() {
                              crate::mir::Place::Local(name) => {
                                  eprintln!("[Codegen] Field access: Local variable '{}'", name);
                                  // Check if this is a struct variable (has struct data location registered)
                                  if let Some(&struct_base) = self.struct_data_locations.get(name) {
                                      eprintln!("[Codegen] Field access: FOUND in struct_data_locations: base={}", struct_base);
                                     // Direct struct field access - the struct data is at struct_base
                                     let field_index = self.get_field_index(name, field_name);
                                     let field_offset = struct_base + (field_index as i64) * 8;
                                     eprintln!("[Codegen] Field access: field_index={}, struct_base={}, field_offset={}", field_index, struct_base, field_offset);
                                     
                                     // Load the field value directly from the struct
                                     eprintln!("[Codegen] Field access: Loading from [rbp {}]", field_offset);
                                     self.instructions.push(X86Instruction::Mov {
                                         dst: X86Operand::Register(Register::RAX),
                                         src: X86Operand::Memory { base: Register::RBP, offset: field_offset },
                                     });
                                 } else if let Some(&var_offset) = self.var_locations.get(name) {
                                     eprintln!("[Codegen] Field access: FOUND in var_locations (indirect): offset={}", var_offset);
                                     // Indirect struct field access - the variable holds a POINTER to struct data
                                     let field_index = self.get_field_index(name, field_name);
                                     let field_offset = (field_index as i64) * 8;
                                     
                                     // Load the pointer from memory
                                     self.instructions.push(X86Instruction::Mov {
                                         dst: X86Operand::Register(Register::RAX),
                                         src: X86Operand::Memory { base: Register::RBP, offset: var_offset },
                                     });
                                     
                                     // Dereference the pointer to get the field
                                     self.instructions.push(X86Instruction::Mov {
                                         dst: X86Operand::Register(Register::RAX),
                                         src: X86Operand::Memory { base: Register::RAX, offset: field_offset },
                                     });
                                 } else {
                                     // Fallback: return 0
                                     eprintln!("[Codegen] Field access: NOT FOUND in either struct_data_locations or var_locations! Returning 0");
                                     eprintln!("[Codegen] Available in struct_data_locations: {:?}", 
                                              self.struct_data_locations.keys().collect::<Vec<_>>());
                                     eprintln!("[Codegen] Available in var_locations: {:?}", 
                                              self.var_locations.keys().collect::<Vec<_>>());
                                     self.instructions.push(X86Instruction::Mov {
                                         dst: X86Operand::Register(Register::RAX),
                                         src: X86Operand::Immediate(0),
                                     });
                                 }
                             }
                             _ => {
                                 // Fallback: return 0
                                 self.instructions.push(X86Instruction::Mov {
                                     dst: X86Operand::Register(Register::RAX),
                                     src: X86Operand::Immediate(0),
                                 });
                             }
                         }
                     }
                    crate::mir::Operand::Move(crate::mir::Place::Field(place, field_name)) => {
                         // Field access on a struct (Move variant - same as Copy for our purposes)
                         match place.as_ref() {
                              crate::mir::Place::Local(name) => {
                                 // Check if this is a struct variable (has struct data location registered)
                                 if let Some(&struct_base) = self.struct_data_locations.get(name) {
                                     // Direct struct field access - the struct data is at struct_base
                                     let field_index = self.get_field_index(name, field_name);
                                     let field_offset = struct_base + (field_index as i64) * 8;
                                     
                                     // Load the field value directly from the struct
                                     self.instructions.push(X86Instruction::Mov {
                                         dst: X86Operand::Register(Register::RAX),
                                         src: X86Operand::Memory { base: Register::RBP, offset: field_offset },
                                     });
                                 } else if let Some(&var_offset) = self.var_locations.get(name) {
                                     // Indirect struct field access - the variable holds a POINTER to struct data
                                     let field_index = self.get_field_index(name, field_name);
                                     let field_offset = (field_index as i64) * 8;
                                     
                                     // Load the pointer from memory
                                     self.instructions.push(X86Instruction::Mov {
                                         dst: X86Operand::Register(Register::RAX),
                                         src: X86Operand::Memory { base: Register::RBP, offset: var_offset },
                                     });
                                     
                                     // Dereference the pointer to get the field
                                     self.instructions.push(X86Instruction::Mov {
                                         dst: X86Operand::Register(Register::RAX),
                                         src: X86Operand::Memory { base: Register::RAX, offset: field_offset },
                                     });
                                 } else {
                                     // Fallback: return 0
                                     self.instructions.push(X86Instruction::Mov {
                                         dst: X86Operand::Register(Register::RAX),
                                         src: X86Operand::Immediate(0),
                                     });
                                 }
                             }
                             _ => {
                                 // Fallback: return 0
                                 self.instructions.push(X86Instruction::Mov {
                                     dst: X86Operand::Register(Register::RAX),
                                     src: X86Operand::Immediate(0),
                                 });
                             }
                         }
                     }
                    crate::mir::Operand::Copy(crate::mir::Place::Local(src_name)) => {
                        // Check if source is a float variable
                        if let Some(&src_offset) = self.var_locations.get(src_name) {
                            if self.float_stack_offsets.contains(&src_offset) {
                                // Source is a float - use movsd to copy
                                skip_final_store = true;
                                if let crate::mir::Place::Local(ref dst_name) = stmt.place {
                                    let dst_offset = self.get_var_location(dst_name);
                                    self.float_stack_offsets.insert(dst_offset);
                                    // Use movsd to copy float from source to destination
                                    self.instructions.push(X86Instruction::Movsd {
                                        dst: "xmm0".to_string(),
                                        src: format!("qword ptr [rbp {}]", if src_offset < 0 { format!("- {}", -src_offset) } else { format!("+ {}", src_offset) }),
                                    });
                                    self.instructions.push(X86Instruction::Movsd {
                                        dst: format!("qword ptr [rbp {}]", if dst_offset < 0 { format!("- {}", -dst_offset) } else { format!("+ {}", dst_offset) }),
                                        src: "xmm0".to_string(),
                                    });
                                }
                            } else {
                                // Source is not a float - use regular copy
                                let src = self.operand_to_x86(operand)?;
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::RAX),
                                    src,
                                });
                            }
                        } else {
                            // Source location unknown - use regular copy
                            let src = self.operand_to_x86(operand)?;
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RAX),
                                src,
                            });
                        }
                    }
                    _ => {
                        let src = self.operand_to_x86(operand)?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src,
                        });
                    }
                }
                
                // After processing Use, check if we're copying from a struct variable or float variable
                // If so, also register the destination with the same properties
                if let crate::mir::Operand::Copy(crate::mir::Place::Local(src_name)) = operand {
                    eprintln!("[Codegen] Detected Copy(Local('{}')), checking for propagation", src_name);
                    eprintln!("[Codegen] struct_data_locations contains: {:?}", 
                              self.struct_data_locations.keys().collect::<Vec<_>>());
                    eprintln!("[Codegen] var_struct_types contains: {:?}", 
                              self.var_struct_types.keys().collect::<Vec<_>>());
                    
                    // DO NOT propagate struct_data_locations!
                    // When we copy a struct variable, the destination gets a POINTER value, not the struct data itself.
                    // So the destination should NOT use struct_data_locations (which assumes direct offset).
                    // Instead, it will use var_locations to store the pointer, and field access will dereference it.
                    if self.struct_data_locations.contains_key(src_name) {
                        eprintln!("[Codegen] Detected copy of struct variable '{}' - destination '{}' will use indirect access", 
                                 src_name, if let crate::mir::Place::Local(ref n) = stmt.place { n } else { "?" });
                    }
                    
                    // Propagate struct type information (crucial for field access lookups)
                    if let Some(struct_type) = self.var_struct_types.get(src_name).cloned() {
                        if let crate::mir::Place::Local(ref dst_name) = stmt.place {
                            eprintln!("[Codegen] Propagating struct type '{}' from '{}' to '{}'", 
                                     struct_type, src_name, dst_name);
                            self.var_struct_types.insert(dst_name.clone(), struct_type);
                        }
                    } else {
                        eprintln!("[Codegen] No var_struct_types entry for '{}'", src_name);
                    }
                    
                    // Propagate float metadata
                    if let Some(&src_offset) = self.var_locations.get(src_name) {
                        if self.float_stack_offsets.contains(&src_offset) {
                            if let crate::mir::Place::Local(ref dst_name) = stmt.place {
                                // Make sure destination has an offset (allocate if needed)
                                let dst_offset = self.get_var_location(dst_name);
                                self.float_stack_offsets.insert(dst_offset);
                            }
                        }
                    }
                }
            }
            crate::mir::Rvalue::BinaryOp(op, left, right) => {
                // Check if this is floating point arithmetic
                let is_float_const_left = matches!(left, crate::mir::Operand::Constant(crate::mir::Constant::Float(_)));
                let is_float_const_right = matches!(right, crate::mir::Operand::Constant(crate::mir::Constant::Float(_)));
                
                // Also check if stack locations are known to be floats
                let is_float_stack_left = if let crate::mir::Operand::Copy(crate::mir::Place::Local(name)) = left {
                    if let Some(&offset) = self.var_locations.get(name) {
                        self.float_stack_offsets.contains(&offset)
                    } else {
                        false
                    }
                } else {
                    false
                };
                let is_float_stack_right = if let crate::mir::Operand::Copy(crate::mir::Place::Local(name)) = right {
                    if let Some(&offset) = self.var_locations.get(name) {
                        self.float_stack_offsets.contains(&offset)
                    } else {
                        false
                    }
                } else {
                    false
                };
                
                let is_float = is_float_const_left || is_float_const_right || is_float_stack_left || is_float_stack_right;
                
                let mut handled_float = false;
                if is_float {
                    // Handle floating-point arithmetic with SSE instructions
                    // For floats, we use XMM0 and XMM1 registers
                    
                    // Load left operand into XMM0
                    let mut left_ok = false;
                    match left {
                        crate::mir::Operand::Constant(crate::mir::Constant::Float(f)) => {
                            let label = self.allocate_float(*f);
                            self.instructions.push(X86Instruction::Movsd {
                                dst: "xmm0".to_string(),
                                src: format!("qword ptr [rip + {}]", label),
                            });
                            left_ok = true;
                        }
                        crate::mir::Operand::Copy(crate::mir::Place::Local(name)) => {
                            let offset = self.get_var_location(name);
                            self.instructions.push(X86Instruction::Movsd {
                                dst: "xmm0".to_string(),
                                src: format!("qword ptr [rbp {}]", if offset < 0 { format!("- {}", -offset) } else { format!("+ {}", offset) }),
                            });
                            left_ok = true;
                        }
                        _ => {}
                    }
                    
                    // Load right operand into XMM1
                    let mut right_ok = false;
                    match right {
                        crate::mir::Operand::Constant(crate::mir::Constant::Float(f)) => {
                            let label = self.allocate_float(*f);
                            self.instructions.push(X86Instruction::Movsd {
                                dst: "xmm1".to_string(),
                                src: format!("qword ptr [rip + {}]", label),
                            });
                            right_ok = true;
                        }
                        crate::mir::Operand::Copy(crate::mir::Place::Local(name)) => {
                            let offset = self.get_var_location(name);
                            self.instructions.push(X86Instruction::Movsd {
                                dst: "xmm1".to_string(),
                                src: format!("qword ptr [rbp {}]", if offset < 0 { format!("- {}", -offset) } else { format!("+ {}", offset) }),
                            });
                            right_ok = true;
                        }
                        _ => {}
                    }
                    
                    // Perform operation if both operands loaded successfully
                    if left_ok && right_ok {
                        // Perform the operation
                        match op {
                            crate::lowering::BinaryOp::Add => {
                                self.instructions.push(X86Instruction::Addsd {
                                    dst: "xmm0".to_string(),
                                    src: "xmm1".to_string(),
                                });
                            }
                            crate::lowering::BinaryOp::Subtract => {
                                self.instructions.push(X86Instruction::Subsd {
                                    dst: "xmm0".to_string(),
                                    src: "xmm1".to_string(),
                                });
                            }
                            crate::lowering::BinaryOp::Multiply => {
                                self.instructions.push(X86Instruction::Mulsd {
                                    dst: "xmm0".to_string(),
                                    src: "xmm1".to_string(),
                                });
                            }
                            crate::lowering::BinaryOp::Divide => {
                                self.instructions.push(X86Instruction::Divsd {
                                    dst: "xmm0".to_string(),
                                    src: "xmm1".to_string(),
                                });
                            }
                            _ => {
                                // For comparison operators, would need to implement float comparisons
                                // For now, skip
                            }
                        }
                        
                        // Store result from xmm0 to target variable
                        if let crate::mir::Place::Local(ref var_name) = stmt.place {
                            let offset = self.get_var_location(var_name);
                            self.float_stack_offsets.insert(offset);
                            self.instructions.push(X86Instruction::Movsd {
                                dst: format!("qword ptr [rbp {}]", if offset < 0 { format!("- {}", -offset) } else { format!("+ {}", offset) }),
                                src: "xmm0".to_string(),
                            });
                        }
                        skip_final_store = true;  // Avoid double-storing
                        handled_float = true;
                    }
                }
                
                // If we didn't handle a float operation above, use integer arithmetic
                if !handled_float {
                
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
                } // End of if !handled_float
            }
            crate::mir::Rvalue::UnaryOp(op, operand) => {
                match op {
                    crate::lowering::UnaryOp::Reference | crate::lowering::UnaryOp::MutableReference => {
                        // Create a reference: &x or &mut x
                        // This means we need to get the address of the operand
                        if let crate::mir::Operand::Copy(crate::mir::Place::Local(var_name)) = operand {
                            if let Some(&var_offset) = self.var_locations.get(var_name) {
                                // Calculate address: RBP + var_offset
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::RAX),
                                    src: X86Operand::Register(Register::RBP),
                                });
                                self.instructions.push(X86Instruction::Add {
                                    dst: X86Operand::Register(Register::RAX),
                                    src: X86Operand::Immediate(var_offset),
                                });
                            } else {
                                // Variable not found, return 0
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::RAX),
                                    src: X86Operand::Immediate(0),
                                });
                            }
                        } else {
                            // For non-local operands, we can't create a reference
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RAX),
                                src: X86Operand::Immediate(0),
                            });
                        }
                    }
                    _ => {
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
                } else if func_name == "Vec::new" {
                    // Vec constructor - allocate stack space and initialize
                    // Vec layout: [capacity:i64][length:i64][data...]
                    // Allocate 128 bytes (enough for ~15 i64 values + metadata)
                    
                    // Stack overflow protection: Maximum stack frame size = 2MB
                    const MAX_STACK_SIZE: i64 = 2 * 1024 * 1024;
                    if self.stack_offset.abs() + 128 + 8 > MAX_STACK_SIZE {
                        eprintln!("[CodeGen] WARNING: Stack allocation exceeds maximum frame size");
                    }
                    
                    self.stack_offset -= 8; // First slot for Vec pointer
                    let vec_ptr_offset = self.stack_offset; // Remember where we store the pointer
                    
                    self.stack_offset -= 128; // allocate space for vec data
                    let vec_data_offset = self.stack_offset; // Start of actual vec data
                    
                    // CRITICAL: The Vec data area extends 128 bytes from vec_data_offset
                    // So it occupies the range [vec_data_offset - 128, vec_data_offset]
                    // We need to move stack_offset beyond this range to prevent temp allocation collisions
                    // The Vec actually extends beyond vec_data_offset since it starts at vec_data_offset
                    // and is 128 bytes large. So move stack_offset down by another 128 bytes total.
                    // Actually, stack_offset is currently at -144 after the -= 128 above.
                    // The 128 bytes we allocated are from -16 to -144.
                    // But the Vec metadata is stored starting at -144, and the Vec data extends from there.
                    // So we need to account for the FULL Vec size by additional decrement
                    // Vec is 128 bytes, starting at -144, so it goes to -144-128 = -272
                    // stack_offset is already at -144, which is the START of the Vec
                    // We should not track min_collection_offset at -144, because we can't allocate AT that address.
                    // Instead, we need allocate_var to skip the entire Vec region.
                    
                    // Track the collection for collision detection
                    if vec_data_offset < self.min_collection_offset {
                        self.min_collection_offset = vec_data_offset;
                        self.collection_size = 128; // Vec uses 128 bytes
                    }
                    
                    // Register this variable's location so subsequent statements can find it
                    if let crate::mir::Place::Local(ref var_name) = stmt.place {
                        self.var_locations.insert(var_name.clone(), vec_ptr_offset);
                    }
                    
                    // Initialize capacity = 30 (space for 30 i64 values)
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: vec_data_offset },
                        src: X86Operand::Immediate(30),
                    });
                    
                    // Initialize length = 0
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: vec_data_offset + 8 },
                        src: X86Operand::Immediate(0),
                    });
                    
                    // Return address of vec metadata in RAX
                    // Calculate: RAX = RBP + vec_data_offset
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Register(Register::RBP),
                    });
                    self.instructions.push(X86Instruction::Add {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(vec_data_offset),
                    });
                    
                    // Store the pointer in the "variable slot" for this Vec
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: vec_ptr_offset },
                        src: X86Operand::Register(Register::RAX),
                    });
                    skip_final_store = true;  // Don't store again at the end
                    
                } else if func_name == "HashMap::new" {
                    // HashMap constructor - allocate stack space and initialize
                    self.stack_offset -= 8; // First slot for HashMap pointer
                    let hmap_ptr_offset = self.stack_offset;
                    
                    self.stack_offset -= 512; // allocate space for hashmap
                    let hmap_data_offset = self.stack_offset;
                    
                    // Track minimum collection offset so temp variables don't collide
                    if hmap_data_offset < self.min_collection_offset {
                        self.min_collection_offset = hmap_data_offset;
                        self.collection_size = 512; // HashMap uses 512 bytes
                    }
                    
                    // Register this variable's location so subsequent statements can find it
                    if let crate::mir::Place::Local(ref var_name) = stmt.place {
                        self.var_locations.insert(var_name.clone(), hmap_ptr_offset);
                    }
                    
                    // Initialize capacity = 16
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: hmap_data_offset },
                        src: X86Operand::Immediate(16),
                    });
                    
                    // Initialize size = 0
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: hmap_data_offset + 8 },
                        src: X86Operand::Immediate(0),
                    });
                    
                    // Return address of hashmap metadata in RAX
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Register(Register::RBP),
                    });
                    self.instructions.push(X86Instruction::Add {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(hmap_data_offset),
                    });
                    
                    // Store pointer in variable slot
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: hmap_ptr_offset },
                        src: X86Operand::Register(Register::RAX),
                    });
                    skip_final_store = true;
                } else if func_name == "HashSet::new" {
                    // HashSet constructor - allocate stack space and initialize
                    self.stack_offset -= 8; // First slot for HashSet pointer
                    let hset_ptr_offset = self.stack_offset;
                    
                    self.stack_offset -= 512; // allocate space for hashset
                    let hset_data_offset = self.stack_offset;
                    
                    // Track minimum collection offset so temp variables don't collide
                    if hset_data_offset < self.min_collection_offset {
                        self.min_collection_offset = hset_data_offset;
                        self.collection_size = 512; // HashSet uses 512 bytes
                    }
                    
                    // Register this variable's location so subsequent statements can find it
                    if let crate::mir::Place::Local(ref var_name) = stmt.place {
                        self.var_locations.insert(var_name.clone(), hset_ptr_offset);
                    }
                    
                    // Initialize capacity = 16
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: hset_data_offset },
                        src: X86Operand::Immediate(16),
                    });
                    
                    // Initialize size = 0
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: hset_data_offset + 8 },
                        src: X86Operand::Immediate(0),
                    });
                    
                    // Return address of hashset metadata in RAX
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Register(Register::RBP),
                    });
                    self.instructions.push(X86Instruction::Add {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(hset_data_offset),
                    });
                    
                    // Store pointer in variable slot
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: hset_ptr_offset },
                        src: X86Operand::Register(Register::RAX),
                    });
                    skip_final_store = true;
                } else if func_name == "push" || func_name.contains("::push") {
                    // Vec::push - call runtime function
                    // rdi = self (vec pointer), rsi = value
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    if args.len() >= 2 {
                        let arg_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: arg_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_vec_push".to_string(),
                    });
                    // push() returns unit (void), don't store result
                    skip_final_store = true;
                } else if func_name == "pop" || func_name.contains("::pop") {
                    // Vec::pop - call runtime function
                    // rdi = self (vec pointer)
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_vec_pop".to_string(),
                    });
                } else if func_name == "get" || func_name.contains("::get") {
                    // Vec::get or HashMap::get - call runtime function
                    // rdi = self (vec pointer), rsi = index
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    if args.len() >= 2 {
                        let arg_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: arg_val,
                        });
                    }
                    // For now assume Vec, can be improved with type info
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_vec_get".to_string(),
                    });
                } else if (func_name == "insert" || func_name.contains("::insert")) && args.len() >= 3 {
                    // HashMap::insert or collection insert - call appropriate runtime function
                    // rdi = self, rsi = key/first_arg, rdx = value/second_arg
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    if args.len() >= 2 {
                        let arg_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: arg_val,
                        });
                    }
                    if args.len() >= 3 {
                        let arg_val = self.operand_to_x86(&args[2])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDX),
                            src: arg_val,
                        });
                    }
                    if func_name.contains("HashSet") || args.len() == 2 {
                        self.instructions.push(X86Instruction::Call {
                            func: "gaia_hashset_insert".to_string(),
                        });
                    } else {
                        self.instructions.push(X86Instruction::Call {
                            func: "gaia_hashmap_insert".to_string(),
                        });
                    }
                } else if (func_name == "insert" || func_name.contains("::insert")) && args.len() == 2 {
                    // HashSet::insert (2 args: self + element)
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    if args.len() >= 2 {
                        let arg_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: arg_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_hashset_insert".to_string(),
                    });
                } else if func_name == "remove" || func_name.contains("::remove") {
                    // Remove function
                    // rdi = self (collection pointer), rsi = key/element
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    if args.len() >= 2 {
                        let arg_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: arg_val,
                        });
                    }
                    if func_name.contains("HashMap") {
                        self.instructions.push(X86Instruction::Call {
                            func: "gaia_hashmap_remove".to_string(),
                        });
                    } else {
                        self.instructions.push(X86Instruction::Call {
                            func: "gaia_hashset_remove".to_string(),
                        });
                    }
                } else if func_name == "contains" || func_name.contains("::contains") {
                    // HashSet::contains or collection contains - call runtime function
                    // rdi = self (collection pointer), rsi = element
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    if args.len() >= 2 {
                        let arg_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: arg_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_hashset_contains".to_string(),
                    });
                } else if func_name == "len" || func_name.contains("::len") {
                    // Vec::len or collection length method
                    // rdi = self (vec pointer)
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_vec_len".to_string(),
                    });
                } else if func_name == "is_empty" || func_name.contains("::is_empty") {
                    // Vec::is_empty, HashMap::is_empty, or HashSet::is_empty
                    // All use same memory layout with size/length at offset +8
                    // rdi = self (collection pointer)
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    // Use generic is_empty that works for all collections
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_collection_is_empty".to_string(),
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
                        // Special handling for string constants - need to load their address
                        // Special handling for float constants - need to load from memory
                        let arg_val = if let crate::mir::Operand::Constant(crate::mir::Constant::String(s)) = arg {
                            let label = self.allocate_string(s.clone());
                            self.instructions.push(X86Instruction::Lea {
                                dst: X86Operand::Register(Register::RAX),
                                src: label,
                            });
                            X86Operand::Register(Register::RAX)
                        } else if let crate::mir::Operand::Constant(crate::mir::Constant::Float(f)) = arg {
                            let label = self.allocate_float(*f);
                            // Load the float address and move to RSI (second argument register)
                            // For floats being passed to printf, they need to be in RSI
                            self.instructions.push(X86Instruction::Lea {
                                dst: X86Operand::Register(Register::RAX),
                                src: label,
                            });
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RAX),
                                src: X86Operand::Memory { base: Register::RAX, offset: 0 },
                            });
                            X86Operand::Register(Register::RAX)
                        } else if let crate::mir::Operand::Copy(crate::mir::Place::Field(place, field_name)) | crate::mir::Operand::Move(crate::mir::Place::Field(place, field_name)) = arg {
                            // Field access as argument - must load field value properly
                            if let crate::mir::Place::Local(obj_name) = place.as_ref() {
                                let fld_idx = self.get_field_index(obj_name, field_name);
                                if let Some(&sb) = self.struct_data_locations.get(obj_name) {
                                    let fo = sb + (fld_idx as i64) * 8;
                                    self.instructions.push(X86Instruction::Mov {
                                        dst: X86Operand::Register(Register::RAX),
                                        src: X86Operand::Memory { base: Register::RBP, offset: fo },
                                    });
                                } else if let Some(&vo) = self.var_locations.get(obj_name) {
                                    self.instructions.push(X86Instruction::Mov {
                                        dst: X86Operand::Register(Register::RAX),
                                        src: X86Operand::Memory { base: Register::RBP, offset: vo },
                                    });
                                    self.instructions.push(X86Instruction::Mov {
                                        dst: X86Operand::Register(Register::RAX),
                                        src: X86Operand::Memory { base: Register::RAX, offset: (fld_idx as i64) * 8 },
                                    });
                                } else {
                                    self.instructions.push(X86Instruction::Mov {
                                        dst: X86Operand::Register(Register::RAX),
                                        src: X86Operand::Immediate(0),
                                    });
                                }
                            }
                            X86Operand::Register(Register::RAX)
                        } else {
                            self.operand_to_x86(arg)?
                        };
                        
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
            crate::mir::Rvalue::Field(place, field_name) => {
                // Field access on a struct
                // Get the struct's base location from var_locations
                match place {
                    crate::mir::Place::Local(name) => {
                        if let Some(&struct_base_offset) = self.var_locations.get(name) {
                            // The struct is stored at struct_base_offset
                            // Calculate field offset using dynamic field index lookup
                            let field_index = self.get_field_index(name, field_name);
                            let field_offset = struct_base_offset + (field_index as i64) * 8;
                            
                            // Load the field value from memory
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RAX),
                                src: X86Operand::Memory { base: Register::RBP, offset: field_offset },
                            });
                        } else {
                            // Not a struct location, just move the value
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RAX),
                                src: X86Operand::Register(Register::RAX),
                            });
                        }
                    }
                    _ => {
                        // Field access on non-local (e.g., from function return)
                        // For now, just return RAX unchanged
                        // This would need better handling for complex cases
                    }
                }
            }
            crate::mir::Rvalue::Aggregate(struct_name, operands) => {
                // Aggregate (struct) construction
                // Store struct fields on stack and return pointer for later field access
                
                if operands.is_empty() {
                    // Empty struct, return 0
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(0),
                    });
                } else {
                    // Allocate space for the struct fields and the pointer variable
                    let field_count = operands.len();
                    let struct_size = (field_count as i64) * 8;
                    
                    // Allocate pointer variable slot FIRST (this will hold the address of the struct)
                    // This decrement ensures p gets its own slot, not shared with previous variables
                    self.stack_offset -= 8;
                    let var_location = self.stack_offset;
                    
                    // Allocate struct slots (which come after the pointer variable)
                    // Stack layout: [var_location:pointer] [struct_base to struct_base-struct_size:fields]
                    // struct_base is where the fields START (most negative offset of the fields)
                    let struct_base = self.stack_offset - struct_size;
                    // Next free location is after all the fields
                    self.stack_offset = struct_base - struct_size;
                    
                    eprintln!("[Codegen] Aggregate: Allocated var_location={}, struct_base={}, new stack_offset={}", var_location, struct_base, self.stack_offset);
                    
                    // Register the variable location for the pointer
                    if let crate::mir::Place::Local(ref var_name) = stmt.place {
                        self.var_locations.insert(var_name.clone(), var_location);
                        eprintln!("[Codegen] Aggregate: var_name='{}', var_location={}, struct_base={}", var_name, var_location, struct_base);
                    }
                    
                    // Store each field value to the struct memory area
                    for (i, operand) in operands.iter().enumerate() {
                        let field_val = self.operand_to_x86(operand)?;
                        let field_offset = struct_base + (i as i64) * 8;
                        eprintln!("[Codegen] Aggregate: Storing field {} at offset {}, operand: {:?}", i, field_offset, operand);
                        eprintln!("[Codegen] Aggregate: Will load from {:?} into [rbp {}]", field_val, field_offset);
                        
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: field_val,
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Memory { base: Register::RBP, offset: field_offset },
                            src: X86Operand::Register(Register::RAX),
                        });
                    }
                    
                    // Return pointer to struct (in RAX)
                    // This pointer can be used for field access later
                    // Calculate RBP + struct_base (struct_base is negative)
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Register(Register::RBP),
                    });
                    self.instructions.push(X86Instruction::Add {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(struct_base),
                    });
                    // RAX now contains the pointer to the struct
                    // It will be stored normally via the final_store mechanism
                    
                    // IMPORTANT: Register the struct data location and type
                    if let crate::mir::Place::Local(ref var_name) = stmt.place {
                        // Store a mapping from variable name to where the struct data is stored
                        self.struct_data_locations.insert(var_name.clone(), struct_base);
                        // Also track the struct type name for later field lookups
                        self.var_struct_types.insert(var_name.clone(), struct_name.clone());
                        eprintln!("[Codegen] Aggregate: Registered {} as struct '{}' with data at offset {}", 
                                 var_name, struct_name, struct_base);
                    }
                    // IMPORTANT: DO NOT skip final_store! We need to store the pointer into the variable's location
                    // so that when we later access fields, we can find the pointer and dereference it
                }
            }
            crate::mir::Rvalue::Index(place, idx) => {
               // Array/Vec element access
               // Place can be Place::Local(var_name) or Place::Index(base, _)
               let var_name = match place {
                   crate::mir::Place::Local(ref name) => Some(name.clone()),
                   crate::mir::Place::Index(ref base, _) => {
                       // Recursively extract the base variable name
                       let mut current = base.as_ref();
                       let mut found_name = None;
                       loop {
                           match current {
                               crate::mir::Place::Local(ref name) => {
                                   found_name = Some(name.clone());
                                   break;
                               }
                               crate::mir::Place::Index(ref next_base, _) => current = next_base.as_ref(),
                               _ => break,
                           }
                       }
                       found_name
                   }
                   _ => None,
               };
               
               if let Some(array_name) = var_name {
                   if let Some(&array_base) = self.struct_data_locations.get(&array_name) {
                       // Found in struct_data_locations
                       // Array is stored directly on stack at array_base
                       let elem_offset = array_base + (*idx as i64) * 8;
                       self.instructions.push(X86Instruction::Mov {
                           dst: X86Operand::Register(Register::RAX),
                           src: X86Operand::Memory { base: Register::RBP, offset: elem_offset },
                       });
                   } else if let Some(&var_offset) = self.var_locations.get(&array_name) {
                       // Found in var_locations
                       // Array pointer is stored at var_offset
                       self.instructions.push(X86Instruction::Mov {
                           dst: X86Operand::Register(Register::RAX),
                           src: X86Operand::Memory { base: Register::RBP, offset: var_offset },
                       });
                       // Vector layout: [capacity:i64][length:i64][data...]
                       // Data starts at offset 16, then add index * 8
                       let elem_offset = 16 + (*idx as i64) * 8;
                       self.instructions.push(X86Instruction::Mov {
                           dst: X86Operand::Register(Register::RAX),
                           src: X86Operand::Memory { base: Register::RAX, offset: elem_offset },
                       });
                   } else {
                       // Fallback: return 0 (not found in either map)
                       self.instructions.push(X86Instruction::Mov {
                           dst: X86Operand::Register(Register::RAX),
                           src: X86Operand::Immediate(0),
                       });
                   }
               } else {
                   // Couldn't extract variable name
                   self.instructions.push(X86Instruction::Mov {
                       dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(0),
                    });
                }
            }
            crate::mir::Rvalue::Array(operands) => {
                // Array construction - allocate space and store elements
                if operands.is_empty() {
                    // Empty array
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(0),
                    });
                } else {
                    // For non-empty arrays: allocate stack space and store elements
                    let elem_count = operands.len();
                    let array_size = (elem_count as i64) * 8;
                    self.stack_offset -= array_size;
                    let array_base = self.stack_offset;
                    
                    // Store each element value to the array memory area
                    for (i, operand) in operands.iter().enumerate() {
                        let elem_val = self.operand_to_x86(operand)?;
                        let elem_offset = array_base + (i as i64) * 8;
                        
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: elem_val,
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Memory { base: Register::RBP, offset: elem_offset },
                            src: X86Operand::Register(Register::RAX),
                        });
                    }
                    
                    // Return pointer to array (in RAX)
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Register(Register::RBP),
                    });
                    self.instructions.push(X86Instruction::Add {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(array_base),
                    });
                    
                    // Register the array data location
                    if let crate::mir::Place::Local(ref var_name) = stmt.place {
                        self.struct_data_locations.insert(var_name.clone(), array_base);
                        self.allocate_var(var_name.clone());
                    }
                    skip_final_store = true;
                }
            }
            _ => {
                self.instructions.push(X86Instruction::Nop);
            }
        }
        
        if !skip_final_store {
            match &stmt.place {
                crate::mir::Place::Local(name) => {
                    let offset = self.get_var_location(name);
                    
                    if self.float_stack_offsets.contains(&offset) {
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Memory { base: Register::RBP, offset },
                            src: X86Operand::Register(Register::RAX),
                        });
                    } else {
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Memory { base: Register::RBP, offset },
                            src: X86Operand::Register(Register::RAX),
                        });
                    }
                }
                crate::mir::Place::Field(place, field_name) => {
                    if let crate::mir::Place::Local(obj_name) = place.as_ref() {
                        if let Some(&struct_base) = self.struct_data_locations.get(obj_name) {
                            let field_idx = self.get_field_index(obj_name, field_name);
                            let field_off = struct_base + (field_idx as i64) * 8;
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Memory { base: Register::RBP, offset: field_off },
                                src: X86Operand::Register(Register::RAX),
                            });
                        } else if let Some(&var_off) = self.var_locations.get(obj_name) {
                            let field_idx = self.get_field_index(obj_name, field_name);
                            let field_off = (field_idx as i64) * 8;
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RCX),
                                src: X86Operand::Memory { base: Register::RBP, offset: var_off },
                            });
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Memory { base: Register::RCX, offset: field_off },
                                src: X86Operand::Register(Register::RAX),
                            });
                        }
                    }
                }
                crate::mir::Place::Deref(inner_place) => {
                    // Dereference assignment: *ptr = value
                    // Inner place contains the pointer value
                    if let crate::mir::Place::Local(ptr_name) = inner_place.as_ref() {
                        // Get the pointer from the variable (allocate if needed)
                        let ptr_offset = self.get_var_location(ptr_name);
                        // Load pointer from stack into RCX
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RCX),
                            src: X86Operand::Memory { base: Register::RBP, offset: ptr_offset },
                        });
                        // Store the value (in RAX) to the dereferenced location
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Memory { base: Register::RCX, offset: 0 },
                            src: X86Operand::Register(Register::RAX),
                        });
                    }
                }
                _ => {}
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
            crate::mir::Operand::Copy(crate::mir::Place::Field(place, field_name)) | crate::mir::Operand::Move(crate::mir::Place::Field(place, field_name)) => {
                // Field access on a struct
                // The struct variable holds a POINTER to the struct data
                match place.as_ref() {
                    crate::mir::Place::Local(name) => {
                        if let Some(&var_offset) = self.var_locations.get(name) {
                            // var_offset points to where the POINTER is stored
                            // So we need to:
                            // 1. Load the pointer: mov rax, [rbp + var_offset]
                            // 2. Dereference it: mov rax, [rax + field_offset]
                            
                            // Calculate field offset using dynamic lookup
                            let _field_index = self.get_field_index(name, field_name);
                            
                            // We can't express this in operand_to_x86 (which returns an X86Operand)
                            // because it requires two loads: load pointer, then dereference
                            // So we return a special marker that tells the caller to do the dereference
                            // For now, return the variable location - the caller will handle the dereference
                            Ok(X86Operand::Memory { base: Register::RBP, offset: var_offset })
                        } else {
                            Ok(X86Operand::Register(Register::RAX))
                        }
                    }
                    _ => Ok(X86Operand::Register(Register::RAX)),
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
            eprintln!("[Codegen] allocate_var: Before allocation - stack_offset={}", self.stack_offset);
            
            // Make sure we don't allocate in collection regions
            // Collections can be large (Vec=128, HashMap/HashSet=512), so we need to skip past them
            // If stack_offset is within or above the collection region, jump below it
            if self.min_collection_offset < i64::MAX && self.collection_size > 0 {
                // The collection occupies memory from min_collection_offset down to min_collection_offset - collection_size
                // We need to ensure stack_offset stays below (more negative than) the collection
                let collection_end = self.min_collection_offset - self.collection_size;
                if self.stack_offset >= collection_end {
                    // Allocate right below the collection
                    self.stack_offset = collection_end - 8;
                    eprintln!("[Codegen] allocate_var: Adjusted for collection, new stack_offset={}", self.stack_offset);
                }
            }
            
            let offset = self.stack_offset;
            self.var_locations.insert(var_name.clone(), offset);
            self.stack_offset -= 8;
            eprintln!("[Codegen] allocate_var('{}'):  Allocated at {}, new stack_offset={}", var_name, offset, self.stack_offset);
            offset
        } else {
            self.var_locations[&var_name]
        }
    }

    /// Get or allocate stack location for a variable
    fn get_var_location(&mut self, var_name: &str) -> i64 {
        if !self.var_locations.contains_key(var_name) {
            let offset = self.allocate_var(var_name.to_string());
            eprintln!("[Codegen] get_var_location: Allocated '{}' at offset {}", var_name, offset);
            offset
        } else {
            let offset = self.var_locations[var_name];
            eprintln!("[Codegen] get_var_location: Found existing '{}' at offset {}", var_name, offset);
            offset
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

    fn allocate_float(&mut self, float: f64) -> String {
        // Check if we already have this float constant
        if let Some((label, _)) = self.float_constants.iter().find(|(_, &v)| (v - float).abs() < f64::EPSILON) {
            label.clone()
        } else {
            let label = format!("float_{}", self.label_counter);
            self.label_counter += 1;
            self.float_constants.insert(label.clone(), float);
            label
        }
    }

    /// Get the field index for a struct field
    /// First tries the struct registry, then falls back to hardcoded mappings
    fn get_field_index(&self, var_name: &str, field_name: &str) -> usize {
        // Validate inputs
        if field_name.is_empty() {
            eprintln!("[CodeGen] Warning: Empty field name requested for variable '{}'", var_name);
            return 0;
        }
        
        // Try to look up the struct type and get field index from registry
        if let Some(struct_name) = self.var_struct_types.get(var_name) {
            if let Some(idx) = get_struct_field_index(struct_name, field_name) {
                return idx;
            }
            // Struct type known but field not found in registry
            eprintln!("[CodeGen] Warning: Field '{}' not found in struct '{}' registry, using fallback mapping", 
                     field_name, struct_name);
        }
        
        // Fallback to hardcoded mappings for backwards compatibility
        // These are standard field names used in common structs
        let fallback_idx = match field_name {
            "x" | "first" | "width" | "value" | "items" => 0,
            "y" | "height" | "second" => 1,
            "z" | "third" => 2,
            "w" | "fourth" => 3,
            // For unknown field names, log a warning instead of silently returning 0
            _ => {
                eprintln!("[CodeGen] Warning: Unknown field name '{}' for variable '{}', defaulting to index 0", 
                         field_name, var_name);
                0
            }
        };
        fallback_idx
    }
}

/// Generate x86-64 assembly from MIR
pub fn generate_code(mir: &Mir) -> CodegenResult<String> {
    let mut codegen = Codegen::new();
    codegen.generate(mir)
}
