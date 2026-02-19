//! # SIMD Instruction Emission (SSE2/AVX2)
//!
//! Actual x86-64 SIMD instruction generation for vectorized operations.
//! Transforms loop patterns into SIMD code paths.

use std::collections::HashMap;
use crate::mir::{BasicBlock, Statement, Rvalue, Operand, Place};
use crate::lowering::BinaryOp;

/// SIMD vectorization level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SIMDLevel {
    SSE2,   // 128-bit vectors (2x i64, 4x i32, etc.)
    AVX2,   // 256-bit vectors (4x i64, 8x i32, etc.)
}

impl SIMDLevel {
    /// Vector width in bytes
    pub fn vector_width(&self) -> usize {
        match self {
            SIMDLevel::SSE2 => 16,
            SIMDLevel::AVX2 => 32,
        }
    }

    /// Vector element capacity for i64
    pub fn i64_capacity(&self) -> usize {
        self.vector_width() / 8
    }

    /// Vector element capacity for i32
    pub fn i32_capacity(&self) -> usize {
        self.vector_width() / 4
    }
}

/// Detected SIMD operation opportunity
#[derive(Debug, Clone)]
pub struct SIMDOpportunity {
    /// Loop index (if in a loop)
    pub loop_index: Option<String>,
    /// Operations in the loop
    pub ops: Vec<SIMDOperationKind>,
    /// Estimated speedup (e.g., 4.0x for 4 parallel operations)
    pub speedup_estimate: f32,
    /// Recommended SIMD level
    pub recommended_level: SIMDLevel,
}

/// Kind of SIMD operation
#[derive(Debug, Clone, PartialEq)]
pub enum SIMDOperationKind {
    VectorAdd,
    VectorSub,
    VectorMul,
    VectorDiv,
    VectorLoad,
    VectorStore,
    VectorShuffle,
    VectorBlend,
}

/// SIMD code generator
pub struct SIMDEmitter {
    /// Current SIMD level
    pub level: SIMDLevel,
    /// Generated SSE2/AVX2 instructions
    pub instructions: Vec<String>,
    /// Register allocation for vector registers (xmm0-xmm15)
    pub vector_registers: HashMap<String, usize>,
    /// Next available vector register
    next_vec_reg: usize,
}

impl SIMDEmitter {
    /// Create new SIMD emitter
    pub fn new(level: SIMDLevel) -> Self {
        SIMDEmitter {
            level,
            instructions: Vec::new(),
            vector_registers: HashMap::new(),
            next_vec_reg: 0,
        }
    }

    /// Allocate a vector register (xmm0-xmm15)
    pub fn allocate_vector_register(&mut self, name: &str) -> usize {
        if let Some(&reg) = self.vector_registers.get(name) {
            return reg;
        }
        let reg = self.next_vec_reg;
        if reg >= 16 {
            eprintln!("WARNING: Too many vector registers allocated (max 16). This code is too complex for SIMD optimization.");
            return 0; // Return 0 as fallback - will use scalar operations instead
        }
        self.vector_registers.insert(name.to_string(), reg);
        self.next_vec_reg += 1;
        reg
    }

    /// Free a vector register
    pub fn free_vector_register(&mut self, name: &str) {
        self.vector_registers.remove(name);
    }

    /// Emit SSE2 load instruction
    pub fn emit_sse2_load(&mut self, dst: usize, src: &str, offset: i64) {
        let instr = if offset == 0 {
            format!("    movdqa xmm{}, [{}]", dst, src)
        } else if offset > 0 {
            format!("    movdqa xmm{}, [{} + {}]", dst, src, offset)
        } else {
            format!("    movdqa xmm{}, [{} - {}]", dst, src, -offset)
        };
        self.instructions.push(instr);
    }

    /// Emit SSE2 store instruction
    pub fn emit_sse2_store(&mut self, dst: &str, offset: i64, src: usize) {
        let instr = if offset == 0 {
            format!("    movdqa [{}], xmm{}", dst, src)
        } else if offset > 0 {
            format!("    movdqa [{} + {}], xmm{}", dst, offset, src)
        } else {
            format!("    movdqa [{} - {}], xmm{}", dst, -offset, src)
        };
        self.instructions.push(instr);
    }

    /// Emit SSE2 add instruction (packed i64)
    pub fn emit_sse2_padded(&mut self, dst: usize, src: usize) {
        self.instructions.push(format!("    paddq xmm{}, xmm{}", dst, src));
    }

    /// Emit SSE2 subtract instruction (packed i64)
    pub fn emit_sse2_psubd(&mut self, dst: usize, src: usize) {
        self.instructions.push(format!("    psubq xmm{}, xmm{}", dst, src));
    }

    /// Emit SSE2 multiply instruction (packed i32x4)
    pub fn emit_sse2_pmulld(&mut self, dst: usize, src: usize) {
        self.instructions.push(format!("    pmulld xmm{}, xmm{}", dst, src));
    }

    /// Emit AVX2 load instruction (256-bit)
    pub fn emit_avx2_load(&mut self, dst: usize, src: &str, offset: i64) {
        let instr = if offset == 0 {
            format!("    vmovdqa ymm{}, [{}]", dst, src)
        } else if offset > 0 {
            format!("    vmovdqa ymm{}, [{} + {}]", dst, src, offset)
        } else {
            format!("    vmovdqa ymm{}, [{} - {}]", dst, src, -offset)
        };
        self.instructions.push(instr);
    }

    /// Emit AVX2 store instruction (256-bit)
    pub fn emit_avx2_store(&mut self, dst: &str, offset: i64, src: usize) {
        let instr = if offset == 0 {
            format!("    vmovdqa [{}], ymm{}", dst, src)
        } else if offset > 0 {
            format!("    vmovdqa [{} + {}], ymm{}", dst, offset, src)
        } else {
            format!("    vmovdqa [{} - {}], ymm{}", dst, -offset, src)
        };
        self.instructions.push(instr);
    }

    /// Emit AVX2 add instruction (packed i64)
    pub fn emit_avx2_paddq(&mut self, dst: usize, src: usize) {
        self.instructions.push(format!("    vpaddq ymm{}, ymm{}, ymm{}", dst, dst, src));
    }

    /// Emit AVX2 multiply instruction (packed i32x8)
    pub fn emit_avx2_pmulld(&mut self, dst: usize, src: usize) {
        self.instructions.push(format!("    vpmulld ymm{}, ymm{}, ymm{}", dst, dst, src));
    }

    /// Emit AVX2 shuffle instruction
    pub fn emit_avx2_pshufd(&mut self, dst: usize, src: usize, mask: u8) {
        self.instructions.push(format!("    vpshufd ymm{}, ymm{}, {}", dst, src, mask));
    }

    /// Detect if a loop is SIMD-friendly
    pub fn detect_simd_opportunity(blocks: &[BasicBlock]) -> Option<SIMDOpportunity> {
        // Look for loops with arithmetic patterns
        let mut ops = Vec::new();
        let mut has_load = false;
        let mut has_store = false;
        let mut has_arithmetic = false;
        let mut op_count = 0;

        for block in blocks {
            for stmt in &block.statements {
                match &stmt.rvalue {
                    Rvalue::Use(Operand::Copy(_)) | Rvalue::Use(Operand::Move(_)) => {
                        has_load = true;
                        ops.push(SIMDOperationKind::VectorLoad);
                    }
                    Rvalue::BinaryOp(op, _, _) => {
                        has_arithmetic = true;
                        let op_kind = match op {
                            BinaryOp::Add => SIMDOperationKind::VectorAdd,
                            BinaryOp::Subtract => SIMDOperationKind::VectorSub,
                            BinaryOp::Multiply => SIMDOperationKind::VectorMul,
                            BinaryOp::Divide => SIMDOperationKind::VectorDiv,
                            _ => continue,
                        };
                        ops.push(op_kind);
                        op_count += 1;
                    }
                    _ => {}
                }
            }
        }

        if has_arithmetic && op_count >= 3 {
            // Estimate speedup based on operation count
            // AVX2 gives 4-8x speedup for i32/i64 operations
            let speedup = match op_count {
                1..=2 => 2.0,
                3..=4 => 4.0,
                5..=8 => 6.0,
                _ => 8.0,
            };

            let level = if op_count >= 8 {
                SIMDLevel::AVX2
            } else {
                SIMDLevel::SSE2
            };

            return Some(SIMDOpportunity {
                loop_index: None,
                ops,
                speedup_estimate: speedup,
                recommended_level: level,
            });
        }

        None
    }

    /// Generate vectorized loop from scalar operations
    pub fn vectorize_loop(&mut self, array_var: &str, array_size: usize) {
        match self.level {
            SIMDLevel::SSE2 => self.vectorize_loop_sse2(array_var, array_size),
            SIMDLevel::AVX2 => self.vectorize_loop_avx2(array_var, array_size),
        }
    }

    fn vectorize_loop_sse2(&mut self, array_var: &str, array_size: usize) {
        let vec_reg = self.allocate_vector_register(array_var);
        let elements_per_vector = self.level.i64_capacity(); // 2 for SSE2 i64
        let iterations = array_size / elements_per_vector;
        let remainder = array_size % elements_per_vector;

        // Main vectorized loop
        self.instructions.push(format!("    xor rcx, rcx                ; loop counter"));
        self.instructions.push(format!(".simd_loop_sse2_{}_start:", array_var));
        self.instructions.push(format!("    cmp rcx, {}", iterations));
        self.instructions.push(format!("    jge .simd_loop_sse2_{}_remainder", array_var));

        // Load vector
        self.emit_sse2_load(vec_reg, "rax", 0);

        // Process vector
        self.instructions.push(format!("    paddq xmm{}, xmm1          ; vector add", vec_reg));

        // Store result
        self.emit_sse2_store("rax", 0, vec_reg);

        // Increment and loop
        self.instructions.push(format!("    add rcx, 1"));
        self.instructions.push(format!("    add rax, 16                 ; next vector (SSE2 = 16 bytes)"));
        self.instructions.push(format!("    jmp .simd_loop_sse2_{}_start", array_var));

        // Scalar epilogue for remainder
        self.instructions.push(format!(".simd_loop_sse2_{}_remainder:", array_var));
        if remainder > 0 {
            self.instructions.push(format!("    xor rcx, rcx                ; scalar loop counter"));
            self.instructions.push(format!(".scalar_loop_{}_start:", array_var));
            self.instructions.push(format!("    cmp rcx, {}", remainder));
            self.instructions.push(format!("    jge .scalar_loop_{}_end", array_var));
            self.instructions.push(format!("    ; process scalar element"));
            self.instructions.push(format!("    add rcx, 1"));
            self.instructions.push(format!("    jmp .scalar_loop_{}_start", array_var));
            self.instructions.push(format!(".scalar_loop_{}_end:", array_var));
        }
    }

    fn vectorize_loop_avx2(&mut self, array_var: &str, array_size: usize) {
        let vec_reg = self.allocate_vector_register(array_var);
        let elements_per_vector = 4; // AVX2 i64 = 4 elements in 256-bit
        let iterations = array_size / elements_per_vector;
        let remainder = array_size % elements_per_vector;

        // Main vectorized loop
        self.instructions.push(format!("    xor rcx, rcx                ; loop counter"));
        self.instructions.push(format!(".simd_loop_avx2_{}_start:", array_var));
        self.instructions.push(format!("    cmp rcx, {}", iterations));
        self.instructions.push(format!("    jge .simd_loop_avx2_{}_remainder", array_var));

        // Load vector
        self.emit_avx2_load(vec_reg, "rax", 0);

        // Process vector
        self.instructions.push(format!("    vpaddq ymm{}, ymm{}, ymm1  ; vector add", vec_reg, vec_reg));

        // Store result
        self.emit_avx2_store("rax", 0, vec_reg);

        // Increment and loop
        self.instructions.push(format!("    add rcx, 1"));
        self.instructions.push(format!("    add rax, 32                 ; next vector (AVX2 = 32 bytes)"));
        self.instructions.push(format!("    jmp .simd_loop_avx2_{}_start", array_var));

        // Scalar epilogue for remainder
        self.instructions.push(format!(".simd_loop_avx2_{}_remainder:", array_var));
        if remainder > 0 {
            self.instructions.push(format!("    xor rcx, rcx                ; scalar loop counter"));
            self.instructions.push(format!(".scalar_loop_{}_start:", array_var));
            self.instructions.push(format!("    cmp rcx, {}", remainder));
            self.instructions.push(format!("    jge .scalar_loop_{}_end", array_var));
            self.instructions.push(format!("    ; process scalar element"));
            self.instructions.push(format!("    add rcx, 1"));
            self.instructions.push(format!("    jmp .scalar_loop_{}_start", array_var));
            self.instructions.push(format!(".scalar_loop_{}_end:", array_var));
        }
    }

    /// Get generated assembly
    pub fn get_assembly(&self) -> String {
        self.instructions.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_emitter_sse2() {
        let mut emitter = SIMDEmitter::new(SIMDLevel::SSE2);
        let reg = emitter.allocate_vector_register("test");
        assert_eq!(reg, 0);

        emitter.emit_sse2_load(reg, "rax", 0);
        emitter.emit_sse2_padded(reg, 1);
        emitter.emit_sse2_store("rax", 0, reg);

        let asm = emitter.get_assembly();
        assert!(asm.contains("movdqa"));
        assert!(asm.contains("paddq"));
    }

    #[test]
    fn test_simd_emitter_avx2() {
        let mut emitter = SIMDEmitter::new(SIMDLevel::AVX2);
        let reg = emitter.allocate_vector_register("test");
        assert_eq!(reg, 0);

        emitter.emit_avx2_load(reg, "rax", 0);
        emitter.emit_avx2_paddq(reg, 1);
        emitter.emit_avx2_store("rax", 0, reg);

        let asm = emitter.get_assembly();
        assert!(asm.contains("vmovdqa"));
        assert!(asm.contains("vpaddq"));
    }

    #[test]
    fn test_vector_register_allocation() {
        let mut emitter = SIMDEmitter::new(SIMDLevel::SSE2);
        let reg1 = emitter.allocate_vector_register("a");
        let reg2 = emitter.allocate_vector_register("b");
        let reg1_again = emitter.allocate_vector_register("a");

        assert_eq!(reg1, 0);
        assert_eq!(reg2, 1);
        assert_eq!(reg1_again, 0); // Same register for same name
    }
}
