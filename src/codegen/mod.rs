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
pub mod simd_emitter;
pub mod cpu_detection;
pub mod iterator_fusion;
pub mod tail_loop;
pub mod inlining;
pub mod register_pressure;
pub mod loop_tiling;
pub mod memory_optimization;
pub mod profiling_diagnostics;
pub mod interprocedural_escape;
pub mod refcount_scheduler;
pub mod smart_pointer_codegen;
pub mod vtable_generation;
pub mod dynamic_dispatch;

use crate::mir::{Mir, MirFunction, Statement, Terminator};
use crate::runtime;
use crate::lowering::{get_struct_field_index, get_struct_field_count};
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
    /// Tracks array variables and their sizes: var_name -> (size, start_offset)
    array_variables: HashMap<String, (usize, i64)>,
    /// Maps function name to its return type (for handling struct returns on call site)
    function_return_types: HashMap<String, crate::lowering::HirType>,
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
            array_variables: HashMap::new(),
            function_return_types: HashMap::new(),
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
        
        // Pre-pass: build function return type map (needed for struct return handling on call site)
        for func in &mir.functions {
            let func_name = if func.name == "main" {
                "gaia_main".to_string()
            } else if func.name.contains("::") {
                func.name.replace("::", "_impl_")
            } else {
                func.name.clone()
            };
            self.function_return_types.insert(func_name, func.return_type.clone());
        }
        
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
                let (param_name, param_type) = &func.params[i];
                self.var_locations.insert(param_name.clone(), offset);
                
                // Check if this parameter is a struct type
                if let crate::lowering::HirType::Named(struct_name) = param_type {
                    // This is a struct parameter - register its data location
                    self.var_struct_types.insert(param_name.clone(), struct_name.clone());
                    self.struct_data_locations.insert(param_name.clone(), offset);
                } else if param_name == "self" && func.name.contains("::") {
                    // Also handle self parameter for methods
                    let struct_name = func.name.split("::").next().unwrap_or("").to_string();
                    if !struct_name.is_empty() {
                        self.var_struct_types.insert(param_name.clone(), struct_name.clone());
                        // IMPORTANT: Register struct data location for self parameter
                        // The self parameter is stored at 'offset' on stack, and that's where the struct data is
                        self.struct_data_locations.insert(param_name.clone(), offset);
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
            let (param_name, param_type) = &func.params[i];
            self.var_locations.insert(param_name.clone(), frame_offset);
            
            // Check if this parameter is a struct type
            if let crate::lowering::HirType::Named(struct_name) = param_type {
                // This is a struct parameter - register its data location
                self.var_struct_types.insert(param_name.clone(), struct_name.clone());
                self.struct_data_locations.insert(param_name.clone(), frame_offset);
            } else if param_name == "self" && func.name.contains("::") {
                // Also handle self parameter for methods
                let struct_name = func.name.split("::").next().unwrap_or("").to_string();
                if !struct_name.is_empty() {
                    self.var_struct_types.insert(param_name.clone(), struct_name.clone());
                    // IMPORTANT: Register struct data location for self parameter
                    // The self parameter is stored at 'frame_offset' on stack, and that's where the struct data is
                    self.struct_data_locations.insert(param_name.clone(), frame_offset);
                }
            }
        }
        
        // Update stack_offset to allocate space after all parameters
        if func.params.len() > 0 {
            self.stack_offset = -8 - (func.params.len() as i64 * 8);
        }
        
        // Generate code for each basic block
        for (block_idx, block) in func.basic_blocks.iter().enumerate() {
            for (stmt_idx, stmt) in block.statements.iter().enumerate() {
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
                    // For main function (gaia_main), always return 0, not the last expression
                    if func_name == "gaia_main" {
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Immediate(0),
                        });
                    } else {
                        // For other functions, handle return value
                        // Special handling for struct returns (aggregate types)
                        if let crate::mir::Operand::Copy(crate::mir::Place::Local(ref var_name)) |
                               crate::mir::Operand::Move(crate::mir::Place::Local(ref var_name)) = operand 
                        {
                            // Check if this is a struct stored on the stack
                            if let Some(&struct_offset) = self.struct_data_locations.get(var_name) {
                                // For structs, return the address on the stack
                                // Calculate the absolute address: RBP + struct_offset
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::RAX),
                                    src: X86Operand::Register(Register::RBP),
                                });
                                self.instructions.push(X86Instruction::Add {
                                    dst: X86Operand::Register(Register::RAX),
                                    src: X86Operand::Immediate(struct_offset),
                                });
                            } else if let Some(&var_offset) = self.var_locations.get(var_name) {
                                // Regular variable - move it to RAX
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::RAX),
                                    src: X86Operand::Memory { base: Register::RBP, offset: var_offset },
                                });
                            } else {
                                // Unknown variable, set to 0 as fallback
                                self.instructions.push(X86Instruction::Mov {
                                    dst: X86Operand::Register(Register::RAX),
                                    src: X86Operand::Immediate(0),
                                });
                            }
                        } else if let Ok(operand_x86) = self.operand_to_x86(operand) {
                            // For non-variable operands, use the standard conversion
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RAX),
                                src: operand_x86,
                            });
                        }
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
                         // The struct variable holds the struct data or a POINTER to struct data
                         match place.as_ref() {
                              crate::mir::Place::Local(name) => {
                                  // Check if this is a struct variable (has struct data location registered)
                                  if let Some(&struct_base) = self.struct_data_locations.get(name) {
                                      // Direct struct field access - the struct data is at struct_base
                                      let field_index = self.get_field_index(name, field_name);
                                      // Stack grows downward, so subtract offset from base
                                      let field_offset = struct_base - (field_index as i64) * 8;
                                      
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
                    crate::mir::Operand::Move(crate::mir::Place::Field(place, field_name)) => {
                          // Field access on a struct (Move variant - same as Copy for our purposes)
                          match place.as_ref() {
                               crate::mir::Place::Local(name) => {
                                  // Check if this is a struct variable (has struct data location registered)
                                  if let Some(&struct_base) = self.struct_data_locations.get(name) {
                                      // Direct struct field access - the struct data is at struct_base
                                      let field_index = self.get_field_index(name, field_name);
                                      // Stack grows downward, so subtract offset from base
                                      let field_offset = struct_base - (field_index as i64) * 8;
                                     
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
                    
                    // DO NOT propagate struct_data_locations!
                    // When we copy a struct variable, the destination gets a POINTER value, not the struct data itself.
                    // So the destination should NOT use struct_data_locations (which assumes direct offset).
                    // Instead, it will use var_locations to store the pointer, and field access will dereference it.
                    if self.struct_data_locations.contains_key(src_name) {
                    }
                    
                    // Propagate struct type information (crucial for field access lookups)
                    if let Some(struct_type) = self.var_struct_types.get(src_name).cloned() {
                        if let crate::mir::Place::Local(ref dst_name) = stmt.place {
                            self.var_struct_types.insert(dst_name.clone(), struct_type);
                        }
                    } else {
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
                    
                    // CRITICAL FIX FOR BUG #1: Propagate array metadata
                    // When we copy an array variable, the destination should also be tracked as an array
                    if let Some(&(elem_count, array_base)) = self.array_variables.get(src_name) {
                        if let crate::mir::Place::Local(ref dst_name) = stmt.place {
                            self.array_variables.insert(dst_name.clone(), (elem_count, array_base));
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
                } else if func_name == "LinkedList::new" {
                    // LinkedList constructor - allocate stack space and initialize (reuse vec layout)
                    self.stack_offset -= 8; // First slot for LinkedList pointer
                    let list_ptr_offset = self.stack_offset;
                    
                    self.stack_offset -= 512; // allocate space for linkedlist
                    let list_data_offset = self.stack_offset;
                    
                    // Track minimum collection offset so temp variables don't collide
                    if list_data_offset < self.min_collection_offset {
                        self.min_collection_offset = list_data_offset;
                        self.collection_size = 512; // LinkedList uses 512 bytes
                    }
                    
                    // Register this variable's location so subsequent statements can find it
                    if let crate::mir::Place::Local(ref var_name) = stmt.place {
                        self.var_locations.insert(var_name.clone(), list_ptr_offset);
                    }
                    
                    // Initialize capacity = 16
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: list_data_offset },
                        src: X86Operand::Immediate(16),
                    });
                    
                    // Initialize size = 0
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: list_data_offset + 8 },
                        src: X86Operand::Immediate(0),
                    });
                    
                    // Return address of linkedlist metadata in RAX
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Register(Register::RBP),
                    });
                    self.instructions.push(X86Instruction::Add {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(list_data_offset),
                    });
                    
                    // Store pointer in variable slot
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: list_ptr_offset },
                        src: X86Operand::Register(Register::RAX),
                    });
                    skip_final_store = true;
                } else if func_name == "BTreeMap::new" {
                    // BTreeMap constructor - allocate stack space and initialize (reuse hashmap layout)
                    self.stack_offset -= 8; // First slot for BTreeMap pointer
                    let bmap_ptr_offset = self.stack_offset;
                    
                    self.stack_offset -= 512; // allocate space for btreemap
                    let bmap_data_offset = self.stack_offset;
                    
                    // Track minimum collection offset so temp variables don't collide
                    if bmap_data_offset < self.min_collection_offset {
                        self.min_collection_offset = bmap_data_offset;
                        self.collection_size = 512; // BTreeMap uses 512 bytes
                    }
                    
                    // Register this variable's location so subsequent statements can find it
                    if let crate::mir::Place::Local(ref var_name) = stmt.place {
                        self.var_locations.insert(var_name.clone(), bmap_ptr_offset);
                    }
                    
                    // Initialize capacity = 16
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: bmap_data_offset },
                        src: X86Operand::Immediate(16),
                    });
                    
                    // Initialize size = 0
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: bmap_data_offset + 8 },
                        src: X86Operand::Immediate(0),
                    });
                    
                    // Return address of btreemap metadata in RAX
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Register(Register::RBP),
                    });
                    self.instructions.push(X86Instruction::Add {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(bmap_data_offset),
                    });
                    
                    // Store pointer in variable slot
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: bmap_ptr_offset },
                        src: X86Operand::Register(Register::RAX),
                    });
                    skip_final_store = true;
                } else if func_name == "push" || func_name == "Vec::push" || func_name == "HashMap::push" {
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
                } else if func_name == "pop" || func_name == "Vec::pop" || func_name == "HashMap::pop" {
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
                } else if func_name == "get" || func_name == "Vec::get" || func_name == "HashMap::get" || func_name == "BTreeMap::get" {
                    // Vec::get, HashMap::get, or BTreeMap::get - call runtime function
                    // rdi = self (collection pointer), rsi = index/key
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
                    // Use HashMap get for BTreeMap/HashMap, Vec get for Vec
                    let runtime_func = if func_name.contains("HashMap") || func_name.contains("BTreeMap") {
                        "gaia_hashmap_get"
                    } else {
                        "gaia_vec_get"
                    };
                    self.instructions.push(X86Instruction::Call {
                        func: runtime_func.to_string(),
                    });
                } else if func_name == "Vec::insert" && args.len() >= 3 {
                    // Vec::insert - call runtime function
                    // rdi = self (vec pointer), rsi = index, rdx = value
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
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_vec_insert".to_string(),
                    });
                    skip_final_store = true;
                } else if func_name == "Vec::remove" && args.len() >= 2 {
                    // Vec::remove - call runtime function
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
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_vec_remove".to_string(),
                    });
                } else if func_name == "Vec::clear" && args.len() >= 1 {
                    // Vec::clear - call runtime function
                    // rdi = self (vec pointer)
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_vec_clear".to_string(),
                    });
                    skip_final_store = true;
                } else if func_name == "Vec::reserve" && args.len() >= 2 {
                    // Vec::reserve - call runtime function
                    // rdi = self (vec pointer), rsi = additional capacity
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
                        func: "gaia_vec_reserve".to_string(),
                    });
                    skip_final_store = true;
                } else if (func_name == "insert" || func_name == "HashMap::insert" || func_name == "HashSet::insert" || func_name == "BTreeMap::insert") && args.len() >= 3 {
                    // HashMap/BTreeMap::insert or collection insert - call appropriate runtime function
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
                        // HashMap or BTreeMap
                        self.instructions.push(X86Instruction::Call {
                            func: "gaia_hashmap_insert".to_string(),
                        });
                    }
                } else if (func_name == "insert" || func_name == "HashMap::insert" || func_name == "HashSet::insert") && args.len() == 2 {
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
                } else if func_name == "remove" || func_name == "HashMap::remove" || func_name == "HashSet::remove" || func_name == "BTreeMap::remove" {
                    // Remove function for HashMap, HashSet, or BTreeMap
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
                    if func_name.contains("HashMap") || func_name.contains("BTreeMap") {
                        self.instructions.push(X86Instruction::Call {
                            func: "gaia_hashmap_remove".to_string(),
                        });
                    } else {
                        self.instructions.push(X86Instruction::Call {
                            func: "gaia_hashset_remove".to_string(),
                        });
                    }
                } else if func_name == "contains" || func_name == "HashMap::contains" || func_name == "HashSet::contains" {
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
                } else if func_name == "contains_key" || func_name == "HashMap::contains_key" || func_name == "BTreeMap::contains_key" {
                    // HashMap/BTreeMap::contains_key - check if key exists
                    // rdi = self (map pointer), rsi = key
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
                        func: "gaia_hashmap_contains_key".to_string(),
                    });
                } else if func_name == "push_front" || func_name == "LinkedList::push_front" || func_name == "push_back" || func_name == "LinkedList::push_back" {
                    // LinkedList::push_front/push_back - push value to front/back
                    // rdi = self (list pointer), rsi = value
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
                    // For now, use vec_push (same memory layout)
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_vec_push".to_string(),
                    });
                } else if func_name == "pop_front" || func_name == "LinkedList::pop_front" || func_name == "pop_back" || func_name == "LinkedList::pop_back" {
                    // LinkedList::pop_front/pop_back - pop value from front/back
                    // rdi = self (list pointer)
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    // For now, use vec_pop (same memory layout)
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_vec_pop".to_string(),
                    });
                } else if func_name == "String::len" {
                    // String::len - get string length
                    // rdi = string pointer
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_string_len".to_string(),
                    });
                } else if func_name == "String::is_empty" {
                    // String::is_empty - check if empty
                    // rdi = string pointer
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_string_is_empty".to_string(),
                    });
                } else if func_name == "String::starts_with" {
                    // String::starts_with - check prefix
                    // rdi = string pointer, rsi = prefix
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    if args.len() >= 2 {
                        let prefix_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: prefix_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_string_starts_with".to_string(),
                    });
                } else if func_name == "String::ends_with" {
                    // String::ends_with - check suffix
                    // rdi = string pointer, rsi = suffix
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    if args.len() >= 2 {
                        let suffix_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: suffix_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_string_ends_with".to_string(),
                    });
                } else if func_name == "String::contains_str" {
                    // String::contains_str - check if contains substring
                    // rdi = string pointer, rsi = substring
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    if args.len() >= 2 {
                        let substring_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: substring_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_string_contains".to_string(),
                    });
                } else if func_name == "len" || func_name == "Vec::len" || func_name == "HashMap::len" || func_name == "HashSet::len" || func_name == "LinkedList::len" || func_name == "BTreeMap::len" {
                    // Collection length method
                    // rdi = self (collection pointer)
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    // Use appropriate runtime function based on method name
                    let runtime_func = if func_name == "HashMap::len" {
                        "gaia_hashmap_len"
                    } else if func_name == "HashSet::len" {
                        "gaia_hashset_len"
                    } else if func_name == "BTreeMap::len" {
                        "gaia_hashmap_len" // BTreeMap reuses HashMap len implementation
                    } else {
                        "gaia_vec_len" // default to vec (LinkedList also uses this)
                    };
                    self.instructions.push(X86Instruction::Call {
                        func: runtime_func.to_string(),
                    });
                } else if func_name == "is_empty" || func_name == "Vec::is_empty" || func_name == "HashMap::is_empty" || func_name == "HashSet::is_empty" || func_name == "LinkedList::is_empty" || func_name == "BTreeMap::is_empty" {
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
                } else if func_name == "clear" || func_name == "Vec::clear" || func_name == "HashMap::clear" || func_name == "HashSet::clear" || func_name == "LinkedList::clear" || func_name == "BTreeMap::clear" {
                    // Clear collection (reset size to 0)
                    // rdi = self (collection pointer)
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    // Use appropriate runtime function
                    let runtime_func = if func_name == "HashMap::clear" {
                        "gaia_hashmap_clear"
                    } else if func_name == "HashSet::clear" {
                        "gaia_hashset_clear"
                    } else if func_name == "BTreeMap::clear" {
                        "gaia_hashmap_clear" // BTreeMap reuses HashMap clear
                    } else {
                        "gaia_vec_clear" // default to vec (LinkedList also uses this)
                    };
                    self.instructions.push(X86Instruction::Call {
                        func: runtime_func.to_string(),
                    });
                } else if func_name == "__builtin_vec_from" {
                    // __builtin_vec_from([elements]) - Create vector from array
                    // Arguments: array operand
                    // Returns: vector (in RAX)
                    
                    if args.is_empty() {
                        // No array argument, create empty vector
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Immediate(0),
                        });
                    } else {
                        // Get the array argument
                        let array_operand = &args[0];
                        
                        // Allocate a vector structure (capacity + length + data)
                        self.stack_offset -= 8; // Pointer to vec metadata
                        let vec_ptr_offset = self.stack_offset;
                        
                        // Determine array size from the operand
                        let elem_count = if let crate::mir::Operand::Copy(crate::mir::Place::Local(var_name)) |
                                              crate::mir::Operand::Move(crate::mir::Place::Local(var_name)) = array_operand {
                            // Look up the array's element count
                            // For now, allocate enough space (simplified)
                            16 // Conservative estimate
                        } else {
                            8 // Default
                        };
                        
                        let vec_size = 16 + (elem_count as i64) * 8; // capacity + length + elements
                        self.stack_offset -= vec_size;
                        let vec_data_offset = self.stack_offset;
                        
                        // Track minimum collection offset
                        if vec_data_offset < self.min_collection_offset {
                            self.min_collection_offset = vec_data_offset;
                            self.collection_size = vec_size;
                        }
                        
                        // Initialize capacity field (at vec_data_offset)
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Memory { base: Register::RBP, offset: vec_data_offset },
                            src: X86Operand::Immediate(elem_count),
                        });
                        
                        // Initialize length field (at vec_data_offset + 8)
                        // Length = number of elements being inserted
                        if let crate::mir::Operand::Copy(crate::mir::Place::Local(var_name)) |
                               crate::mir::Operand::Move(crate::mir::Place::Local(var_name)) = array_operand {
                            // If source is a named array, get its length from metadata
                            // For now, set to elem_count
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Memory { base: Register::RBP, offset: vec_data_offset + 8 },
                                src: X86Operand::Immediate(elem_count),
                            });
                        } else {
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Memory { base: Register::RBP, offset: vec_data_offset + 8 },
                                src: X86Operand::Immediate(elem_count),
                            });
                        }
                        
                        // Copy array elements to vector data area
                        if let crate::mir::Operand::Copy(crate::mir::Place::Local(var_name)) |
                               crate::mir::Operand::Move(crate::mir::Place::Local(var_name)) = array_operand {
                            // Copy from source array to vector
                            if let Some(&src_offset) = self.struct_data_locations.get(var_name) {
                                for i in 0..elem_count {
                                    // Stack grows downward, so subtract offsets
                                    let src_elem_offset = src_offset - (i as i64) * 8;
                                    let dst_elem_offset = vec_data_offset - 16 - (i as i64) * 8;
                                    
                                    self.instructions.push(X86Instruction::Mov {
                                        dst: X86Operand::Register(Register::RAX),
                                        src: X86Operand::Memory { base: Register::RBP, offset: src_elem_offset },
                                    });
                                    self.instructions.push(X86Instruction::Mov {
                                        dst: X86Operand::Memory { base: Register::RBP, offset: dst_elem_offset },
                                        src: X86Operand::Register(Register::RAX),
                                    });
                                }
                            }
                        }
                        
                        // Return address of vector metadata in RAX
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RBP),
                        });
                        self.instructions.push(X86Instruction::Add {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Immediate(vec_data_offset),
                        });
                        
                        // Store vector pointer in variable slot
                        if let crate::mir::Place::Local(ref var_name) = stmt.place {
                            self.var_locations.insert(var_name.clone(), vec_ptr_offset);
                        }
                        skip_final_store = true;
                    }
                } else if func_name == "__builtin_vec_repeat" {
                    // __builtin_vec_repeat(element, count) - Create vector with repeated element
                    // Arguments: element (i64), count (i64)
                    // Returns: vector (in RAX)
                    
                    if args.len() >= 2 {
                        let element = &args[0];
                        let count = &args[1];
                        
                        // Get count value
                        let count_val = if let crate::mir::Operand::Constant(crate::mir::Constant::Integer(n)) = count {
                            *n
                        } else {
                            // Load count from operand
                            let count_x86 = self.operand_to_x86(count)?;
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RAX),
                                src: count_x86,
                            });
                            // Can't determine exact count, use conservative estimate
                            64
                        };
                        
                        // Allocate vector
                        self.stack_offset -= 8; // Pointer to vec metadata
                        let vec_ptr_offset = self.stack_offset;
                        
                        let vec_size = 16 + (count_val * 8); // capacity + length + elements
                        self.stack_offset -= vec_size;
                        let vec_data_offset = self.stack_offset;
                        
                        // Track minimum collection offset
                        if vec_data_offset < self.min_collection_offset {
                            self.min_collection_offset = vec_data_offset;
                            self.collection_size = vec_size;
                        }
                        
                        // Initialize capacity
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Memory { base: Register::RBP, offset: vec_data_offset },
                            src: X86Operand::Immediate(count_val),
                        });
                        
                        // Initialize length
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Memory { base: Register::RBP, offset: vec_data_offset + 8 },
                            src: X86Operand::Immediate(count_val),
                        });
                        
                        // Fill all elements with the repeated value
                        let elem_val = self.operand_to_x86(element)?;
                        for i in 0..count_val {
                            let elem_offset = vec_data_offset + 16 + (i * 8);
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RAX),
                                src: elem_val.clone(),
                            });
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Memory { base: Register::RBP, offset: elem_offset },
                                src: X86Operand::Register(Register::RAX),
                            });
                        }
                        
                        // Return address of vector metadata in RAX
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Register(Register::RBP),
                        });
                        self.instructions.push(X86Instruction::Add {
                            dst: X86Operand::Register(Register::RAX),
                            src: X86Operand::Immediate(vec_data_offset),
                        });
                        
                        // Store vector pointer in variable slot
                        if let crate::mir::Place::Local(ref var_name) = stmt.place {
                            self.var_locations.insert(var_name.clone(), vec_ptr_offset);
                        }
                        skip_final_store = true;
                    }
                } else if func_name == "Option::is_some" || func_name == "Option::is_none" {
                    // Option::is_some / is_none
                    // rdi = option pointer
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    let runtime_func = if func_name == "Option::is_some" {
                        "gaia_option_is_some"
                    } else {
                        "gaia_option_is_none"
                    };
                    self.instructions.push(X86Instruction::Call {
                        func: runtime_func.to_string(),
                    });
                } else if func_name == "Option::unwrap" {
                    // Option::unwrap
                    // rdi = option pointer
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_option_unwrap".to_string(),
                    });
                } else if func_name == "Option::unwrap_or" {
                    // Option::unwrap_or
                    // rdi = option pointer, rsi = default value
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    if args.len() >= 2 {
                        let default_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: default_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_option_unwrap_or".to_string(),
                    });
                } else if func_name == "Result::is_ok" || func_name == "Result::is_err" {
                    // Result::is_ok / is_err
                    // rdi = result pointer
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    let runtime_func = if func_name == "Result::is_ok" {
                        "gaia_result_is_ok"
                    } else {
                        "gaia_result_is_err"
                    };
                    self.instructions.push(X86Instruction::Call {
                        func: runtime_func.to_string(),
                    });
                } else if func_name == "Result::unwrap" {
                    // Result::unwrap
                    // rdi = result pointer
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_result_unwrap".to_string(),
                    });
                } else if func_name == "Result::unwrap_err" {
                    // Result::unwrap_err
                    // rdi = result pointer
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_result_unwrap_err".to_string(),
                    });
                } else if func_name == "Result::unwrap_or" {
                    // Result::unwrap_or
                    // rdi = result pointer, rsi = default value
                    if args.len() >= 1 {
                        let self_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: self_val,
                        });
                    }
                    if args.len() >= 2 {
                        let default_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: default_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_result_unwrap_or".to_string(),
                    });
                } else if func_name == "Iterator::map" {
                    // Iterator::map(closure)
                    // rdi = iterator, rsi = closure
                    if args.len() >= 2 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                        let closure_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: closure_val,
                        });
                    }
                    // Call runtime function
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_iterator_map".to_string(),
                    });
                } else if func_name == "Iterator::filter" {
                    // Iterator::filter(closure)
                    // rdi = iterator, rsi = closure
                    if args.len() >= 2 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                        let closure_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: closure_val,
                        });
                    }
                    // Call runtime function
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_iterator_filter".to_string(),
                    });
                } else if func_name == "Iterator::fold" {
                    // Iterator::fold(init, closure)
                    // rdi = iterator, rsi = init, rdx = closure
                    if args.len() >= 3 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                        let init_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: init_val,
                        });
                        let closure_val = self.operand_to_x86(&args[2])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDX),
                            src: closure_val,
                        });
                    }
                    // Call runtime function
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_iterator_fold".to_string(),
                    });
                } else if func_name == "Iterator::collect" {
                    // Iterator::collect() -> Collection
                    // rdi = iterator
                    if args.len() >= 1 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                    }
                    // For now: simplified - just return iterator as-is
                    // Full implementation would create a new collection
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Register(Register::RDI),
                    });
                } else if func_name == "Iterator::for_each" {
                    // Iterator::for_each(closure) -> ()
                    // rdi = iterator, rsi = closure
                    if args.len() >= 2 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                        let closure_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: closure_val,
                        });
                    }
                    // Call runtime function
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_iterator_for_each".to_string(),
                    });
                } else if func_name == "Iterator::sum" {
                    // Iterator::sum() -> T
                    // rdi = iterator
                    if args.len() >= 1 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                    }
                    // Call runtime function
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_iterator_sum".to_string(),
                    });
                } else if func_name == "Iterator::count" {
                    // Iterator::count() -> i64
                    // rdi = iterator
                    if args.len() >= 1 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                    }
                    // Call runtime function
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_iterator_count".to_string(),
                    });
                } else if func_name == "Iterator::take" {
                    // Iterator::take(n) -> Iterator
                    // rdi = iterator, rsi = n
                    if args.len() >= 2 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                        let n_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: n_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_iterator_take".to_string(),
                    });
                } else if func_name == "Iterator::skip" {
                    // Iterator::skip(n) -> Iterator
                    // rdi = iterator, rsi = n
                    if args.len() >= 2 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                        let n_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: n_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_iterator_skip".to_string(),
                    });
                } else if func_name == "Iterator::chain" {
                    // Iterator::chain(other) -> Iterator
                    // rdi = iterator, rsi = other iterator
                    if args.len() >= 2 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                        let other_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: other_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_iterator_chain".to_string(),
                    });
                } else if func_name == "Iterator::find" {
                    // Iterator::find(closure) -> Option<T>
                    // rdi = iterator, rsi = closure
                    if args.len() >= 2 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                        let closure_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: closure_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_iterator_find".to_string(),
                    });
                } else if func_name == "Iterator::any" {
                    // Iterator::any(closure) -> bool
                    // rdi = iterator, rsi = closure
                    if args.len() >= 2 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                        let closure_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: closure_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_iterator_any".to_string(),
                    });
                } else if func_name == "Iterator::all" {
                    // Iterator::all(closure) -> bool
                    // rdi = iterator, rsi = closure
                    if args.len() >= 2 {
                        let iter_val = self.operand_to_x86(&args[0])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: iter_val,
                        });
                        let closure_val = self.operand_to_x86(&args[1])?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RSI),
                            src: closure_val,
                        });
                    }
                    self.instructions.push(X86Instruction::Call {
                        func: "gaia_iterator_all".to_string(),
                    });
                } else if func_name == "Vec::into_iter" {
                    // Vec::into_iter() -> Iterator
                    // rdi = vector
                    // Simply call __into_iter with the vector pointer
                    if let Some(arg) = args.first() {
                        let arg_val = self.operand_to_x86(arg)?;
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RDI),
                            src: arg_val,
                        });
                    }
                    // Call __into_iter to initialize iterator state
                    self.instructions.push(X86Instruction::Call {
                        func: "__into_iter".to_string(),
                    });
                } else if func_name == "__into_iter" {
                    // CRITICAL FIX FOR BUG #1: Array iterator protocol
                    // When __into_iter is called with an array, we need to wrap it with metadata
                    // (capacity, length) so __next can use it properly
                    
                    if let Some(crate::mir::Operand::Copy(crate::mir::Place::Local(ref array_var))) = args.first() {
                        if let Some(&(elem_count, _array_base)) = self.array_variables.get(array_var) {
                            // This is an array - create wrapper with metadata
                            // Allocate space for: [capacity:i64][length:i64][data...]
                            let wrapper_size = 16 + (elem_count as i64) * 8;
                            self.stack_offset -= wrapper_size;
                            let wrapper_offset = self.stack_offset;
                            
                            // Initialize capacity
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Memory { base: Register::RBP, offset: wrapper_offset },
                                src: X86Operand::Immediate(elem_count as i64),
                            });
                            
                            // Initialize length
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Memory { base: Register::RBP, offset: wrapper_offset + 8 },
                                src: X86Operand::Immediate(elem_count as i64),
                            });
                            
                            // Copy array elements into wrapper
                            // The array is stored at [RBP + array_base], we need to copy to [RBP + wrapper_offset + 16]
                            if let Some(&(_size, array_base)) = self.array_variables.get(array_var) {
                                for i in 0..elem_count {
                                    // Load element from original array
                                    self.instructions.push(X86Instruction::Mov {
                                        dst: X86Operand::Register(Register::RAX),
                                        src: X86Operand::Memory { 
                                            base: Register::RBP, 
                                            offset: array_base + (i as i64) * 8
                                        },
                                    });
                                    // Store to wrapper
                                    self.instructions.push(X86Instruction::Mov {
                                        dst: X86Operand::Memory { 
                                            base: Register::RBP, 
                                            offset: wrapper_offset + 16 + (i as i64) * 8
                                        },
                                        src: X86Operand::Register(Register::RAX),
                                    });
                                }
                            }
                            
                            // Pass wrapper address to __into_iter in RDI
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RAX),
                                src: X86Operand::Register(Register::RBP),
                            });
                            self.instructions.push(X86Instruction::Add {
                                dst: X86Operand::Register(Register::RAX),
                                src: X86Operand::Immediate(wrapper_offset),
                            });
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RDI),
                                src: X86Operand::Register(Register::RAX),
                            });
                            
                            // Call __into_iter
                            self.instructions.push(X86Instruction::Call {
                                func: "__into_iter".to_string(),
                            });
                            // Result stays in RAX
                        } else {
                            // Not an array - handle as regular function call
                            let arg_val = self.operand_to_x86(&args[0])?;
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RDI),
                                src: arg_val,
                            });
                            self.instructions.push(X86Instruction::Call {
                                func: "__into_iter".to_string(),
                            });
                        }
                    } else {
                        // Fallback - call normally
                        if let Some(arg) = args.first() {
                            let arg_val = self.operand_to_x86(arg)?;
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RDI),
                                src: arg_val,
                            });
                            self.instructions.push(X86Instruction::Call {
                                func: "__into_iter".to_string(),
                            });
                        }
                    }
                    skip_final_store = false;
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
                                    // Stack grows downward, so subtract offset from base
                                    let fo = sb - (fld_idx as i64) * 8;
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
                // First check struct_data_locations (direct struct access), then fall back to var_locations
                match place {
                    crate::mir::Place::Local(name) => {
                        if let Some(&struct_base_offset) = self.struct_data_locations.get(name) {
                            // The struct data is directly stored at struct_base_offset
                            // Calculate field offset using dynamic field index lookup
                            let field_index = self.get_field_index(name, field_name);
                            // Stack grows downward, so subtract offset from base
                            let field_offset = struct_base_offset - (field_index as i64) * 8;
                            
                            // Load the field value from memory
                            self.instructions.push(X86Instruction::Mov {
                                dst: X86Operand::Register(Register::RAX),
                                src: X86Operand::Memory { base: Register::RBP, offset: field_offset },
                            });
                        } else if let Some(&var_offset) = self.var_locations.get(name) {
                            // The struct pointer is stored at var_offset (indirect struct access)
                            // This would be for structs stored as pointers
                            let field_index = self.get_field_index(name, field_name);
                            let field_offset = (field_index as i64) * 8;
                            
                            // Load the pointer
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
                // For local struct variables, store struct fields directly at the variable location.
                // Do NOT use a separate pointer - this was causing field access to fail.
                
                if operands.is_empty() {
                    // Empty struct, return 0
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: X86Operand::Immediate(0),
                    });
                } else {
                    // Allocate space for the struct fields
                    let field_count = operands.len();
                    let struct_size = (field_count as i64) * 8;
                    
                    // Allocate space on stack for all struct fields
                    // struct_base should point to the START of the allocated space (the current stack_offset)
                    // BEFORE we decrement stack_offset.
                    // Then decrement stack_offset to mark the space as allocated.
                    let struct_base = self.stack_offset;
                    self.stack_offset -= struct_size;
                    
                    // Now fields are stored at: struct_base, struct_base-8, struct_base-16, ...
                    // And stack_offset points to the next available location
                    
                    // Register the variable location - this is where the struct data STARTS
                    if let crate::mir::Place::Local(ref var_name) = stmt.place {
                        self.var_locations.insert(var_name.clone(), struct_base);
                    }
                    
                    // Store each field value to the struct memory area
                    // Fields are laid out from stack_offset going downward: field[0] at offset, field[1] at offset-8, etc.
                    for (i, operand) in operands.iter().enumerate() {
                        let field_val = self.operand_to_x86(operand)?;
                        let field_offset = struct_base - (i as i64) * 8;
                        
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: field_val,
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Memory { base: Register::RBP, offset: field_offset },
                            src: X86Operand::Register(Register::RAX),
                        });
                    }
                    
                    // IMPORTANT: Register the struct data location
                    // For local structs, the struct data is stored directly at struct_base
                    if let crate::mir::Place::Local(ref var_name) = stmt.place {
                        // Store a mapping from variable name to where the struct data is stored
                        // struct_base points to the FIRST FIELD location
                        self.struct_data_locations.insert(var_name.clone(), struct_base);
                        // Also track the struct type name for later field lookups
                        self.var_struct_types.insert(var_name.clone(), struct_name.clone());
                    }
                    
                    // DON'T put anything in RAX - final_store will handle storing the struct data correctly.
                    // Since we directly stored all fields to their locations, there's nothing left to do.
                    skip_final_store = true;
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
                       // Stack grows downward, so elements are at: array_base, array_base-8, array_base-16, ...
                       let elem_offset = array_base - (*idx as i64) * 8;
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
                    // Allocate space: array_base should be set BEFORE decrementing stack_offset
                    let array_base = self.stack_offset;
                    self.stack_offset -= array_size;
                    
                    // Store each element value to the array memory area
                    // Stack grows downward, so elements are at: array_base, array_base-8, array_base-16, ...
                    for (i, operand) in operands.iter().enumerate() {
                        let elem_val = self.operand_to_x86(operand)?;
                        let elem_offset = array_base - (i as i64) * 8;
                        
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Register(Register::RAX),
                            src: elem_val,
                        });
                        self.instructions.push(X86Instruction::Mov {
                            dst: X86Operand::Memory { base: Register::RBP, offset: elem_offset },
                            src: X86Operand::Register(Register::RAX),
                        });
                    }
                    
                    // Register the array data location
                    // DON'T put anything in RAX - the array is stored directly on stack
                    if let crate::mir::Place::Local(ref var_name) = stmt.place {
                        self.struct_data_locations.insert(var_name.clone(), array_base);
                        self.array_variables.insert(var_name.clone(), (elem_count, array_base));
                        // DON'T call allocate_var here - the array is already allocated directly
                        // Calling allocate_var would create a separate var_locations entry
                        // which confuses the Index code into thinking it's a pointer
                    }
                    skip_final_store = true;
                }
            }
            crate::mir::Rvalue::Closure { fn_ptr, captures } => {
                // Closure creation: allocate closure object with fn_ptr and captured values
                // Closure layout: [fn_ptr:i64][capture1:i64][capture2:i64]...
                let closure_size = 8 + (captures.len() as i64) * 8; // fn_ptr + captured values
                // closure_base should be set BEFORE decrementing stack_offset
                let closure_base = self.stack_offset;
                self.stack_offset -= closure_size;
                
                // Store function pointer at offset 0
                self.instructions.push(X86Instruction::Lea {
                    dst: X86Operand::Register(Register::RAX),
                    src: fn_ptr.clone(),
                });
                self.instructions.push(X86Instruction::Mov {
                    dst: X86Operand::Memory { base: Register::RBP, offset: closure_base },
                    src: X86Operand::Register(Register::RAX),
                });
                
                // Store captured values at offsets -8, -16, -24, etc. (stack grows downward)
                for (i, operand) in captures.iter().enumerate() {
                    let capture_offset = closure_base - 8 - (i as i64) * 8;
                    let val = self.operand_to_x86(operand)?;
                    
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Register(Register::RAX),
                        src: val,
                    });
                    self.instructions.push(X86Instruction::Mov {
                        dst: X86Operand::Memory { base: Register::RBP, offset: capture_offset },
                        src: X86Operand::Register(Register::RAX),
                    });
                }
                
                // Return closure pointer (in RAX)
                self.instructions.push(X86Instruction::Mov {
                    dst: X86Operand::Register(Register::RAX),
                    src: X86Operand::Register(Register::RBP),
                });
                self.instructions.push(X86Instruction::Add {
                    dst: X86Operand::Register(Register::RAX),
                    src: X86Operand::Immediate(closure_base),
                });
                
                // Register the closure data location
                if let crate::mir::Place::Local(ref var_name) = stmt.place {
                    self.struct_data_locations.insert(var_name.clone(), closure_base);
                    self.allocate_var(var_name.clone());
                }
                skip_final_store = true;
            }
            _ => {
                self.instructions.push(X86Instruction::Nop);
            }
        }
        
        // IMPORTANT: Check for struct return from function call BEFORE checking should_skip_store
        // When a function returns a struct, RAX contains an address we need to copy from
        if let crate::mir::Rvalue::Call(func_name, _args) = &stmt.rvalue {
            if let crate::mir::Place::Local(name) = &stmt.place {
                // Mangle the function name to match what we're tracking
                let mangled_func_name = if func_name.contains("::") {
                    func_name.replace("::", "_impl_")
                } else {
                    func_name.clone()
                };
                
                // Check if this function returns a struct
                // Clone the struct_name to avoid borrow issues
                if let Some(return_type) = self.function_return_types.get(&mangled_func_name).cloned() {
                    if let crate::lowering::HirType::Named(struct_name) = return_type {
                        // This function returns a struct - handle the struct return
                        self.handle_struct_return(&struct_name, name)?;
                        // Skip the regular store, we've already handled it
                        skip_final_store = true;
                    }
                }
            }
        }
        
        // Check if this variable is directly allocated (array/struct)
        let should_skip_store = if let crate::mir::Place::Local(name) = &stmt.place {
            self.struct_data_locations.contains_key(name)
        } else {
            false
        };
        
        if !skip_final_store && !should_skip_store {
            match &stmt.place {
                crate::mir::Place::Local(name) => {
                    
                    let offset = self.get_var_location(name);
                    
                    // IMPORTANT: Propagate struct/array metadata for copies
                    // When we copy a struct or array variable, the destination inherits the data location
                    if let crate::mir::Rvalue::Use(operand) = &stmt.rvalue {
                       match operand {
                           crate::mir::Operand::Copy(crate::mir::Place::Local(src_name)) |
                           crate::mir::Operand::Move(crate::mir::Place::Local(src_name)) => {
                               // Check if source is a struct variable
                               if let Some(struct_type) = self.var_struct_types.get(src_name).cloned() {
                                   self.var_struct_types.insert(name.clone(), struct_type);
                                   // IMPORTANT: Register the destination's struct data location
                                   // The copied struct data is now at this variable's offset
                                   self.struct_data_locations.insert(name.clone(), offset);
                               }
                               
                               // Check if source is an array variable
                               if let Some(&(elem_count, src_array_base)) = self.array_variables.get(src_name) {
                                   // When copying an array, the destination should point to the SOURCE array's location
                                   // not to the newly allocated var_locations
                                   // Register the destination as pointing to the same array base as the source
                                   self.array_variables.insert(name.clone(), (elem_count, src_array_base));
                                   self.struct_data_locations.insert(name.clone(), src_array_base);
                               }
                           }
                           _ => {}
                       }
                    }

                    if !skip_final_store {
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
                }
                crate::mir::Place::Field(place, field_name) => {
                    if let crate::mir::Place::Local(obj_name) = place.as_ref() {
                        if let Some(&struct_base) = self.struct_data_locations.get(obj_name) {
                            let field_idx = self.get_field_index(obj_name, field_name);
                            // Stack grows downward, so subtract offset from base
                            let field_off = struct_base - (field_idx as i64) * 8;
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
                }
            }
            
            let offset = self.stack_offset;
            self.var_locations.insert(var_name.clone(), offset);
            self.stack_offset -= 8;
            offset
        } else {
            self.var_locations[&var_name]
        }
    }

    /// Get or allocate stack location for a variable
    fn get_var_location(&mut self, var_name: &str) -> i64 {
        if !self.var_locations.contains_key(var_name) {
            let offset = self.allocate_var(var_name.to_string());
            offset
        } else {
            let offset = self.var_locations[var_name];
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
            return 0;
        }
        
        // Try to look up the struct type and get field index from registry
        if let Some(struct_name) = self.var_struct_types.get(var_name) {
            if let Some(idx) = get_struct_field_index(struct_name, field_name) {
                return idx;
            }
            // Struct type known but field not found in registry
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
                0
            }
        };
        fallback_idx
    }
    
    /// Handle struct return values on the call site
    /// When a function returns a struct, it returns an address in RAX.
    /// We need to:
    /// 1. Allocate space on the caller's stack for the struct
    /// 2. Copy the struct data from the address in RAX to our allocated space
    /// 3. Register the destination variable as having struct data
    fn handle_struct_return(&mut self, struct_name: &str, dst_var: &str) -> CodegenResult<()> {
        // Get the struct field count to know how much data to copy
        let field_count = get_struct_field_count(struct_name);
        if field_count == 0 {
            // Struct not found or has no fields - just store RAX as-is
            return Ok(());
        }
        
        let struct_size = (field_count as i64) * 8;
        
        // Allocate space on the stack for the struct
        // IMPORTANT: Get the base BEFORE decrement (to match normal Aggregate layout)
        let struct_base = self.stack_offset;
        self.stack_offset -= struct_size;
        
        // RAX contains the source address of the struct data
        // The function returns the address of the FIRST FIELD (struct_base)
        // Fields in source are laid out downward: [RAX], [RAX-8], [RAX-16], ...
        // We need to copy them to our destination (also downward): struct_base, struct_base-8, ...
        // So field[i] is at [RAX - i*8] in source
        for i in 0..field_count {
            // Load from source address: [RAX - i*8]
            let source_offset = -(i as i64) * 8;  // Note: negative offset for downward layout
            self.instructions.push(X86Instruction::Mov {
                dst: X86Operand::Register(Register::R10),
                src: X86Operand::Memory { base: Register::RAX, offset: source_offset },
            });
            // Store to destination (following the downward field layout)
            let dest_offset = struct_base - (i as i64) * 8;
            self.instructions.push(X86Instruction::Mov {
                dst: X86Operand::Memory { base: Register::RBP, offset: dest_offset },
                src: X86Operand::Register(Register::R10),
            });
        }
        
        // Register this variable in both var_locations and struct_data_locations
        self.var_locations.insert(dst_var.to_string(), struct_base);
        self.var_struct_types.insert(dst_var.to_string(), struct_name.to_string());
        self.struct_data_locations.insert(dst_var.to_string(), struct_base);
        
        Ok(())
    }
}

/// Generate x86-64 assembly from MIR
pub fn generate_code(mir: &Mir) -> CodegenResult<String> {
    let mut codegen = Codegen::new();
    codegen.generate(mir)
}
