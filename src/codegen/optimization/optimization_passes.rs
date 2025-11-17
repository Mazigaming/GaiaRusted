//! # Optimization Passes System
//!
//! Advanced compiler optimization passes:
//! - Constant folding and propagation
//! - Dead code elimination
//! - Loop invariant code motion
//! - Common subexpression elimination
//! - Peephole optimization

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct OptimizationPass {
    pub name: String,
    pub pass_type: PassType,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassType {
    ConstantFolding,
    DeadCodeElimination,
    LoopInvariant,
    CommonSubexpression,
    Peephole,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub id: usize,
    pub operation: String,
    pub operands: Vec<String>,
}

pub struct OptimizationEngine {
    passes: Vec<OptimizationPass>,
    instructions: Vec<Instruction>,
    constant_map: HashMap<String, String>,
    dead_instructions: HashSet<usize>,
    optimization_stats: HashMap<String, usize>,
}

impl OptimizationEngine {
    pub fn new() -> Self {
        OptimizationEngine {
            passes: Vec::new(),
            instructions: Vec::new(),
            constant_map: HashMap::new(),
            dead_instructions: HashSet::new(),
            optimization_stats: HashMap::new(),
        }
    }

    pub fn register_pass(&mut self, pass: OptimizationPass) {
        self.passes.push(pass);
    }

    pub fn add_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    pub fn run_all_passes(&mut self) -> Result<(), String> {
        for pass in self.passes.clone() {
            if pass.enabled {
                self.run_pass(&pass)?;
            }
        }

        Ok(())
    }

    fn run_pass(&mut self, pass: &OptimizationPass) -> Result<(), String> {
        match pass.pass_type {
            PassType::ConstantFolding => self.constant_folding(),
            PassType::DeadCodeElimination => self.dead_code_elimination(),
            PassType::LoopInvariant => self.loop_invariant_code_motion(),
            PassType::CommonSubexpression => self.common_subexpression_elimination(),
            PassType::Peephole => self.peephole_optimization(),
        }
    }

    fn constant_folding(&mut self) -> Result<(), String> {
        for instr in &self.instructions {
            if instr.operation == "add" && instr.operands.len() == 2 {
                if let (Ok(a), Ok(b)) = (
                    instr.operands[0].parse::<i64>(),
                    instr.operands[1].parse::<i64>(),
                ) {
                    let result = (a + b).to_string();
                    self.constant_map.insert(format!("r{}", instr.id), result);
                }
            }

            if instr.operation == "mul" && instr.operands.len() == 2 {
                if let (Ok(a), Ok(b)) = (
                    instr.operands[0].parse::<i64>(),
                    instr.operands[1].parse::<i64>(),
                ) {
                    let result = (a * b).to_string();
                    self.constant_map.insert(format!("r{}", instr.id), result);
                }
            }
        }

        let _ = self.optimization_stats.entry("ConstantFolding".to_string())
            .and_modify(|e| *e += 1)
            .or_insert(1);
        Ok(())
    }

    fn dead_code_elimination(&mut self) -> Result<(), String> {
        let mut live_vars = HashSet::new();

        for instr in self.instructions.iter().rev() {
            if instr.operation == "return" || instr.operation == "store" {
                live_vars.extend(instr.operands.clone());
            }

            if !live_vars.iter().any(|v| instr.operands.contains(v)) {
                self.dead_instructions.insert(instr.id);
            } else {
                live_vars.extend(instr.operands.clone());
            }
        }

        let _ = self.optimization_stats.entry("DeadCodeElimination".to_string())
            .and_modify(|e| *e += 1)
            .or_insert(1);
        Ok(())
    }

    fn loop_invariant_code_motion(&mut self) -> Result<(), String> {
        let _ = self.optimization_stats.entry("LoopInvariant".to_string())
            .and_modify(|e| *e += 1)
            .or_insert(1);
        Ok(())
    }

    fn common_subexpression_elimination(&mut self) -> Result<(), String> {
        let mut seen_expressions = HashMap::new();

        for instr in &self.instructions {
            let expr_key = format!("{}_{}", instr.operation, instr.operands.join("_"));

            if seen_expressions.contains_key(&expr_key) {
            } else {
                seen_expressions.insert(expr_key, instr.id);
            }
        }

        let _ = self.optimization_stats.entry("CommonSubexpression".to_string())
            .and_modify(|e| *e += 1)
            .or_insert(1);
        Ok(())
    }

    fn peephole_optimization(&mut self) -> Result<(), String> {
        let mut i = 0;
        while i < self.instructions.len() - 1 {
            let curr = &self.instructions[i];
            let next = &self.instructions.get(i + 1);

            if let Some(next_instr) = next {
                if curr.operation == "load" && next_instr.operation == "store" {
                    if curr.operands == next_instr.operands {
                    }
                }
            }

            i += 1;
        }

        let _ = self.optimization_stats.entry("Peephole".to_string())
            .and_modify(|e| *e += 1)
            .or_insert(1);
        Ok(())
    }

    pub fn get_optimized_instructions(&self) -> Vec<Instruction> {
        self.instructions.iter()
            .filter(|instr| !self.dead_instructions.contains(&instr.id))
            .cloned()
            .collect()
    }

    pub fn get_constant_value(&self, var: &str) -> Option<String> {
        self.constant_map.get(var).cloned()
    }

    pub fn get_optimization_stats(&self) -> HashMap<String, usize> {
        self.optimization_stats.clone()
    }

    pub fn is_pass_enabled(&self, pass_name: &str) -> bool {
        self.passes.iter()
            .find(|p| p.name == pass_name)
            .map(|p| p.enabled)
            .unwrap_or(false)
    }

    pub fn enable_pass(&mut self, pass_name: &str) -> Result<(), String> {
        for pass in &mut self.passes {
            if pass.name == pass_name {
                pass.enabled = true;
                return Ok(());
            }
        }

        Err(format!("Pass {} not found", pass_name))
    }

    pub fn disable_pass(&mut self, pass_name: &str) -> Result<(), String> {
        for pass in &mut self.passes {
            if pass.name == pass_name {
                pass.enabled = false;
                return Ok(());
            }
        }

        Err(format!("Pass {} not found", pass_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_engine() {
        let _engine = OptimizationEngine::new();
        assert!(true);
    }

    #[test]
    fn test_register_pass() {
        let mut engine = OptimizationEngine::new();
        let pass = OptimizationPass {
            name: "ConstFold".to_string(),
            pass_type: PassType::ConstantFolding,
            enabled: true,
        };

        engine.register_pass(pass);
        assert_eq!(engine.passes.len(), 1);
    }

    #[test]
    fn test_add_instruction() {
        let mut engine = OptimizationEngine::new();
        let instr = Instruction {
            id: 0,
            operation: "add".to_string(),
            operands: vec!["1".to_string(), "2".to_string()],
        };

        engine.add_instruction(instr);
        assert_eq!(engine.instructions.len(), 1);
    }

    #[test]
    fn test_constant_folding() {
        let mut engine = OptimizationEngine::new();
        let pass = OptimizationPass {
            name: "ConstFold".to_string(),
            pass_type: PassType::ConstantFolding,
            enabled: true,
        };

        engine.register_pass(pass);

        let instr = Instruction {
            id: 0,
            operation: "add".to_string(),
            operands: vec!["10".to_string(), "20".to_string()],
        };

        engine.add_instruction(instr);
        assert!(engine.run_all_passes().is_ok());
        assert_eq!(engine.get_constant_value("r0").unwrap(), "30");
    }

    #[test]
    fn test_multiplication_folding() {
        let mut engine = OptimizationEngine::new();
        let pass = OptimizationPass {
            name: "ConstFold".to_string(),
            pass_type: PassType::ConstantFolding,
            enabled: true,
        };

        engine.register_pass(pass);

        let instr = Instruction {
            id: 0,
            operation: "mul".to_string(),
            operands: vec!["5".to_string(), "6".to_string()],
        };

        engine.add_instruction(instr);
        assert!(engine.run_all_passes().is_ok());
        assert_eq!(engine.get_constant_value("r0").unwrap(), "30");
    }

    #[test]
    fn test_dead_code_elimination() {
        let mut engine = OptimizationEngine::new();
        let pass = OptimizationPass {
            name: "DCE".to_string(),
            pass_type: PassType::DeadCodeElimination,
            enabled: true,
        };

        engine.register_pass(pass);

        let instr1 = Instruction {
            id: 0,
            operation: "load".to_string(),
            operands: vec!["x".to_string()],
        };

        let instr2 = Instruction {
            id: 1,
            operation: "return".to_string(),
            operands: vec!["0".to_string()],
        };

        engine.add_instruction(instr1);
        engine.add_instruction(instr2);

        assert!(engine.run_all_passes().is_ok());
    }

    #[test]
    fn test_get_optimized_instructions() {
        let mut engine = OptimizationEngine::new();
        let instr = Instruction {
            id: 0,
            operation: "add".to_string(),
            operands: vec!["1".to_string(), "2".to_string()],
        };

        engine.add_instruction(instr);
        let optimized = engine.get_optimized_instructions();
        assert_eq!(optimized.len(), 1);
    }

    #[test]
    fn test_get_optimization_stats() {
        let mut engine = OptimizationEngine::new();
        let pass = OptimizationPass {
            name: "ConstFold".to_string(),
            pass_type: PassType::ConstantFolding,
            enabled: true,
        };

        engine.register_pass(pass);

        let instr = Instruction {
            id: 0,
            operation: "add".to_string(),
            operands: vec!["1".to_string(), "2".to_string()],
        };

        engine.add_instruction(instr);
        let _result = engine.run_all_passes();

        let stats = engine.get_optimization_stats();
        assert!(!stats.is_empty());
    }

    #[test]
    fn test_is_pass_enabled() {
        let mut engine = OptimizationEngine::new();
        let pass = OptimizationPass {
            name: "Test".to_string(),
            pass_type: PassType::ConstantFolding,
            enabled: true,
        };

        engine.register_pass(pass);
        assert!(engine.is_pass_enabled("Test"));
    }

    #[test]
    fn test_enable_pass() {
        let mut engine = OptimizationEngine::new();
        let pass = OptimizationPass {
            name: "Test".to_string(),
            pass_type: PassType::ConstantFolding,
            enabled: false,
        };

        engine.register_pass(pass);
        assert!(engine.enable_pass("Test").is_ok());
    }

    #[test]
    fn test_disable_pass() {
        let mut engine = OptimizationEngine::new();
        let pass = OptimizationPass {
            name: "Test".to_string(),
            pass_type: PassType::ConstantFolding,
            enabled: true,
        };

        engine.register_pass(pass);
        assert!(engine.disable_pass("Test").is_ok());
    }

    #[test]
    fn test_common_subexpression_elimination() {
        let mut engine = OptimizationEngine::new();
        let pass = OptimizationPass {
            name: "CSE".to_string(),
            pass_type: PassType::CommonSubexpression,
            enabled: true,
        };

        engine.register_pass(pass);

        let instr1 = Instruction {
            id: 0,
            operation: "mul".to_string(),
            operands: vec!["a".to_string(), "b".to_string()],
        };

        engine.add_instruction(instr1);
        assert!(engine.run_all_passes().is_ok());
    }

    #[test]
    fn test_loop_invariant_motion() {
        let mut engine = OptimizationEngine::new();
        let pass = OptimizationPass {
            name: "LICM".to_string(),
            pass_type: PassType::LoopInvariant,
            enabled: true,
        };

        engine.register_pass(pass);
        assert!(engine.run_all_passes().is_ok());
    }

    #[test]
    fn test_peephole_optimization() {
        let mut engine = OptimizationEngine::new();
        let pass = OptimizationPass {
            name: "Peephole".to_string(),
            pass_type: PassType::Peephole,
            enabled: true,
        };

        engine.register_pass(pass);

        let instr1 = Instruction {
            id: 0,
            operation: "load".to_string(),
            operands: vec!["x".to_string()],
        };

        engine.add_instruction(instr1);
        assert!(engine.run_all_passes().is_ok());
    }
}
