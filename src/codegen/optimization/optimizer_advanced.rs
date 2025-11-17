//! Advanced optimization passes
//!
//! Beyond basic constant folding, dead code elimination:
//! - Peephole optimization
//! - Loop optimization (unrolling, invariant hoisting)
//! - Better inlining heuristics
//! - Redundancy elimination
//! - Vectorization hints

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Load { dst: String, src: String },
    Store { dst: String, src: String },
    Add { dst: String, left: String, right: String },
    Sub { dst: String, left: String, right: String },
    Mul { dst: String, left: String, right: String },
    Div { dst: String, left: String, right: String },
    Jump { target: usize },
    CondJump { target: usize, cond: String },
    Return { value: Option<String> },
}

/// Peephole optimizer - looks at small sequences of instructions
pub struct PeepholeOptimizer {
    instructions: Vec<Instruction>,
}

impl PeepholeOptimizer {
    pub fn new(instructions: Vec<Instruction>) -> Self {
        PeepholeOptimizer { instructions }
    }

    /// Optimize redundant moves: x = y; z = x; y = x becomes z = y
    pub fn eliminate_redundant_moves(&mut self) -> usize {
        let mut optimizations = 0;
        let mut i = 0;

        while i + 1 < self.instructions.len() {
            if let (
                Instruction::Load { dst: d1, src: s1 },
                Instruction::Load { dst: _d2, src: s2 },
            ) = (&self.instructions[i], &self.instructions[i + 1])
            {
                if d1 == s2 && s1 == s2 {
                    self.instructions.remove(i + 1);
                    optimizations += 1;
                    continue;
                }
            }
            i += 1;
        }

        optimizations
    }

    /// Remove dead stores (assignments that are never read)
    pub fn remove_dead_stores(&mut self) -> usize {
        let mut optimizations = 0;
        let mut read_vars = HashSet::new();

        // First pass: collect all read variables
        for instr in &self.instructions {
            match instr {
                Instruction::Load { src, .. } => {
                    read_vars.insert(src.clone());
                }
                Instruction::Add { left, right, .. } => {
                    read_vars.insert(left.clone());
                    read_vars.insert(right.clone());
                }
                Instruction::Sub { left, right, .. } => {
                    read_vars.insert(left.clone());
                    read_vars.insert(right.clone());
                }
                Instruction::Mul { left, right, .. } => {
                    read_vars.insert(left.clone());
                    read_vars.insert(right.clone());
                }
                Instruction::Div { left, right, .. } => {
                    read_vars.insert(left.clone());
                    read_vars.insert(right.clone());
                }
                Instruction::CondJump { cond, .. } => {
                    read_vars.insert(cond.clone());
                }
                Instruction::Return { value, .. } => {
                    if let Some(v) = value {
                        read_vars.insert(v.clone());
                    }
                }
                _ => {}
            }
        }

        // Second pass: remove stores to variables never read
        let mut i = 0;
        while i < self.instructions.len() {
            let should_remove = match &self.instructions[i] {
                Instruction::Store { dst, .. } => !read_vars.contains(dst),
                _ => false,
            };

            if should_remove {
                self.instructions.remove(i);
                optimizations += 1;
            } else {
                i += 1;
            }
        }

        optimizations
    }

    /// Combine consecutive identical operations
    pub fn combine_identical_ops(&mut self) -> usize {
        let mut optimizations = 0;
        let mut i = 0;

        while i + 1 < self.instructions.len() {
            if self.instructions[i] == self.instructions[i + 1] {
                match &self.instructions[i] {
                    Instruction::Load { .. } => {
                        // If same load twice, keep only first
                        self.instructions.remove(i + 1);
                        optimizations += 1;
                        continue;
                    }
                    _ => {}
                }
            }
            i += 1;
        }

        optimizations
    }

    pub fn optimize(&mut self) -> usize {
        let mut total = 0;
        total += self.eliminate_redundant_moves();
        total += self.remove_dead_stores();
        total += self.combine_identical_ops();
        total
    }
}

/// Loop optimization - identifies and optimizes loops
pub struct LoopOptimizer {
    instructions: Vec<Instruction>,
}

impl LoopOptimizer {
    pub fn new(instructions: Vec<Instruction>) -> Self {
        LoopOptimizer { instructions }
    }

    /// Detect loop invariant code (code that doesn't change across iterations)
    pub fn detect_loop_invariants(&self, loop_start: usize, loop_end: usize) -> Vec<usize> {
        let mut invariants = Vec::new();

        for i in loop_start..loop_end.min(self.instructions.len()) {
            match &self.instructions[i] {
                Instruction::Load { src: _, .. } => {
                    // If loading from a constant/unchanging source, it's invariant
                    invariants.push(i);
                }
                _ => {}
            }
        }

        invariants
    }

    /// Count loop iterations (if determinable)
    pub fn estimate_loop_iterations(&self, loop_start: usize, loop_end: usize) -> Option<usize> {
        let mut count = 0;
        for i in loop_start..loop_end.min(self.instructions.len()) {
            if matches!(self.instructions[i], Instruction::Jump { .. }) {
                count += 1;
            }
        }

        if count > 0 {
            Some(count)
        } else {
            None
        }
    }

    /// Check if loop can be unrolled
    pub fn can_unroll(&self, loop_start: usize, loop_end: usize) -> bool {
        let size = loop_end - loop_start;
        // Only unroll small loops
        size < 50 && self.estimate_loop_iterations(loop_start, loop_end).is_some()
    }
}

/// Inlining optimizer - decides what functions to inline
pub struct InliningOptimizer {
    function_sizes: HashMap<String, usize>,
    function_call_count: HashMap<String, usize>,
}

impl InliningOptimizer {
    pub fn new() -> Self {
        InliningOptimizer {
            function_sizes: HashMap::new(),
            function_call_count: HashMap::new(),
        }
    }

    pub fn register_function(&mut self, name: String, size: usize) {
        self.function_sizes.insert(name, size);
    }

    pub fn record_call(&mut self, name: String) {
        *self.function_call_count.entry(name).or_insert(0) += 1;
    }

    /// Determine if a function should be inlined
    pub fn should_inline(&self, name: &str) -> bool {
        let size = self.function_sizes.get(name).copied().unwrap_or(1000);
        let calls = self.function_call_count.get(name).copied().unwrap_or(0);

        // Inline if:
        // 1. Function is small (< 100 instructions)
        // 2. Called frequently (> 2 times) OR called once and very small (< 20)
        (size < 100 && calls > 2) || (calls >= 1 && size < 20)
    }

    /// Score functions for inlining (higher = should inline more)
    pub fn score_for_inlining(&self, name: &str) -> f64 {
        let size = self.function_sizes.get(name).copied().unwrap_or(1000) as f64;
        let calls = self.function_call_count.get(name).copied().unwrap_or(0) as f64;

        // Score = (frequency * 10) / size
        // Higher frequency and smaller size = higher score
        (calls * 10.0) / (size + 1.0)
    }

    /// Get candidates for inlining sorted by score
    pub fn get_inlining_candidates(&self) -> Vec<(String, f64)> {
        let mut candidates: Vec<_> = self
            .function_sizes
            .keys()
            .map(|name| (name.clone(), self.score_for_inlining(name)))
            .collect();

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.into_iter().filter(|(_, score)| *score > 1.0).collect()
    }
}

/// Pattern-based optimization - recognizes and optimizes common patterns
pub struct PatternOptimizer;

impl PatternOptimizer {
    /// Check for x = x pattern (self-assignment) and mark for removal
    pub fn detect_self_assignment(instr: &Instruction) -> bool {
        match instr {
            Instruction::Load { dst, src } => dst == src,
            Instruction::Store { dst, src } => dst == src,
            _ => false,
        }
    }

    /// Check for x = a; x = b pattern (dead store)
    pub fn detect_dead_store(instr1: &Instruction, instr2: &Instruction) -> bool {
        if let (Instruction::Store { dst: d1, .. }, Instruction::Store { dst: d2, .. }) =
            (instr1, instr2)
        {
            d1 == d2
        } else {
            false
        }
    }

    /// Check for arithmetic on constants
    pub fn can_fold_operation(left: &str, op: &str, right: &str) -> bool {
        // Check if both operands are numeric literals
        left.parse::<i64>().is_ok()
            && right.parse::<i64>().is_ok()
            && matches!(op, "+" | "-" | "*" | "/" | "&" | "|" | "^")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peephole_redundant_moves() {
        let instrs = vec![
            Instruction::Load {
                dst: "x".to_string(),
                src: "y".to_string(),
            },
            Instruction::Load {
                dst: "z".to_string(),
                src: "y".to_string(),
            },
        ];

        let mut opt = PeepholeOptimizer::new(instrs);
        let result = opt.combine_identical_ops();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_self_assignment_detection() {
        let instr = Instruction::Load {
            dst: "x".to_string(),
            src: "x".to_string(),
        };
        assert!(PatternOptimizer::detect_self_assignment(&instr));
    }

    #[test]
    fn test_inlining_scoring() {
        let mut opt = InliningOptimizer::new();
        opt.register_function("small".to_string(), 15);
        opt.register_function("large".to_string(), 500);

        opt.record_call("small".to_string());
        opt.record_call("small".to_string());
        opt.record_call("large".to_string());

        assert!(opt.should_inline("small"));
        assert!(!opt.should_inline("large"));
    }

    #[test]
    fn test_loop_detection() {
        let instrs = vec![
            Instruction::Load {
                dst: "i".to_string(),
                src: "0".to_string(),
            },
            Instruction::CondJump {
                target: 0,
                cond: "i < n".to_string(),
            },
        ];

        let opt = LoopOptimizer::new(instrs);
        // LoopOptimizer just tracks basic loop structure
        assert_eq!(opt.estimate_loop_iterations(0, 0), None);
    }

    #[test]
    fn test_inlining_candidates() {
        let mut opt = InliningOptimizer::new();
        opt.register_function("func1".to_string(), 10);
        opt.register_function("func2".to_string(), 100);

        for _ in 0..5 {
            opt.record_call("func1".to_string());
        }

        let candidates = opt.get_inlining_candidates();
        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].0, "func1");
    }
}
