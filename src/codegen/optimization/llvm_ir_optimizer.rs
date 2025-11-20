
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct LLVMIROptimizer {
    optimization_level: OptimizationLevel,
    stats: OptimizationStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    O0,
    O1,
    O2,
    O3,
}

#[derive(Debug, Clone, Copy)]
pub struct OptimizationStats {
    pub insts_removed: usize,
    pub insts_simplified: usize,
    pub blocks_merged: usize,
    pub branches_eliminated: usize,
}

impl LLVMIROptimizer {
    pub fn new(level: OptimizationLevel) -> Self {
        LLVMIROptimizer {
            optimization_level: level,
            stats: OptimizationStats {
                insts_removed: 0,
                insts_simplified: 0,
                blocks_merged: 0,
                branches_eliminated: 0,
            },
        }
    }

    pub fn optimize_ir(&mut self, ir: &str) -> String {
        match self.optimization_level {
            OptimizationLevel::O0 => ir.to_string(),
            OptimizationLevel::O1 => self.optimize_level_1(ir),
            OptimizationLevel::O2 => self.optimize_level_2(ir),
            OptimizationLevel::O3 => self.optimize_level_3(ir),
        }
    }

    fn optimize_level_1(&mut self, ir: &str) -> String {
        let mut result = ir.to_string();
        result = self.eliminate_unreachable_blocks(&result);
        result = self.simplify_constants(&result);
        result
    }

    fn optimize_level_2(&mut self, ir: &str) -> String {
        let mut result = self.optimize_level_1(ir);
        result = self.eliminate_dead_code(&result);
        result = self.merge_basic_blocks(&result);
        result = self.inline_small_functions(&result);
        result
    }

    fn optimize_level_3(&mut self, ir: &str) -> String {
        let mut result = self.optimize_level_2(ir);
        result = self.vectorization_analysis(&result);
        result = self.loop_unrolling(&result);
        result = self.aggressive_inlining(&result);
        result
    }

    fn eliminate_unreachable_blocks(&mut self, ir: &str) -> String {
        let lines: Vec<&str> = ir.lines().collect();
        let mut reachable = HashSet::new();
        let mut to_visit = vec![0];

        while let Some(idx) = to_visit.pop() {
            if idx >= lines.len() || reachable.contains(&idx) {
                continue;
            }
            reachable.insert(idx);

            if let Some(line) = lines.get(idx) {
                if line.contains("br ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for part in parts {
                        if part.starts_with("label%") {
                            if let Ok(label_num) = part[6..].parse::<usize>() {
                                to_visit.push(label_num);
                            }
                        }
                    }
                } else if !line.contains("ret ") {
                    to_visit.push(idx + 1);
                }
            }
        }

        lines
            .iter()
            .enumerate()
            .filter(|(idx, _)| reachable.contains(idx))
            .map(|(_, line)| *line)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn simplify_constants(&mut self, ir: &str) -> String {
        let mut result = String::new();

        for line in ir.lines() {
            let simplified = if line.contains("add i64") {
                self.simplify_add(line)
            } else if line.contains("mul i64") {
                self.simplify_mul(line)
            } else if line.contains("and i64") {
                self.simplify_and(line)
            } else if line.contains("or i64") {
                self.simplify_or(line)
            } else if line.contains("xor i64") {
                self.simplify_xor(line)
            } else {
                line.to_string()
            };

            if simplified != line {
                self.stats.insts_simplified += 1;
            }

            result.push_str(&simplified);
            result.push('\n');
        }

        result.trim().to_string()
    }

    fn simplify_add(&self, line: &str) -> String {
        if line.contains("add i64 0,") {
            line.replace("add i64 0,", "move ")
        } else if line.contains("add i64 ") && line.contains(", 0") {
            line.replace(" 0", "")
        } else {
            line.to_string()
        }
    }

    fn simplify_mul(&self, line: &str) -> String {
        if line.contains("mul i64 1,") {
            line.replace("mul i64 1,", "move ")
        } else if line.contains("mul i64 0,") {
            line.replace("mul i64 0,", "move i64 0 ->")
        } else if line.contains("mul i64 ") && line.contains(", 1") {
            line.replace(" 1", "")
        } else {
            line.to_string()
        }
    }

    fn simplify_and(&self, line: &str) -> String {
        if line.contains("and i64 0,") {
            line.replace("and i64 0,", "move i64 0 ->")
        } else {
            line.to_string()
        }
    }

    fn simplify_or(&self, line: &str) -> String {
        if line.contains("or i64 0,") {
            line.replace("or i64 0,", "move ")
        } else {
            line.to_string()
        }
    }

    fn simplify_xor(&self, line: &str) -> String {
        if line.contains("xor i64 ") && line.contains(" 0") {
            line.replace("xor i64 ", "move ")
                .replace(" 0", "")
        } else {
            line.to_string()
        }
    }

    fn eliminate_dead_code(&mut self, ir: &str) -> String {
        let lines: Vec<&str> = ir.lines().collect();
        let mut used_vars = HashSet::new();
        let mut result = String::new();

        for line in lines.iter().rev() {
            if line.contains("ret ") {
                used_vars.insert(line.split_whitespace().last().unwrap_or(&"").to_string());
            }

            let mut keep_line = false;
            for word in line.split_whitespace() {
                if word.starts_with("%") && used_vars.contains(word) {
                    keep_line = true;
                    break;
                }
            }

            if keep_line || line.contains("ret ") || line.contains("call ") {
                if let Some(var) = line.split('=').next() {
                    let var_name = var.trim();
                    if var_name.starts_with("%") {
                        used_vars.insert(var_name.to_string());
                    }
                }
                result.push_str(line);
                result.push('\n');
                self.stats.insts_removed += 1;
            }
        }

        result.lines().collect::<Vec<_>>().join("\n")
    }

    fn merge_basic_blocks(&mut self, ir: &str) -> String {
        let mut result = String::new();
        let lines: Vec<&str> = ir.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let current = lines[i];
            result.push_str(current);
            result.push('\n');

            if i + 1 < lines.len() {
                let next = lines[i + 1];
                if current.contains("br label%") && next.starts_with("label%") {
                    self.stats.blocks_merged += 1;
                    i += 1;
                }
            }

            i += 1;
        }

        result.trim().to_string()
    }

    fn inline_small_functions(&mut self, ir: &str) -> String {
        let mut result = String::new();
        let lines: Vec<&str> = ir.lines().collect();

        for line in lines {
            if line.contains("call ") && !line.contains("@printf") && !line.contains("@malloc") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() <= 4 {
                    result.push_str("; inlined: ");
                    self.stats.insts_removed += 1;
                }
            }
            result.push_str(line);
            result.push('\n');
        }

        result.trim().to_string()
    }

    fn vectorization_analysis(&mut self, ir: &str) -> String {
        let mut result = String::new();

        for line in ir.lines() {
            if line.contains("i64") && line.contains("add ") {
                result.push_str("; VECTORIZABLE: ");
            }
            result.push_str(line);
            result.push('\n');
        }

        result.trim().to_string()
    }

    fn loop_unrolling(&mut self, ir: &str) -> String {
        ir.to_string()
    }

    fn aggressive_inlining(&mut self, ir: &str) -> String {
        ir.to_string()
    }

    pub fn get_stats(&self) -> OptimizationStats {
        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimizer_creation() {
        let _opt = LLVMIROptimizer::new(OptimizationLevel::O2);
    }

    #[test]
    fn test_eliminate_unreachable() {
        let mut opt = LLVMIROptimizer::new(OptimizationLevel::O1);
        let ir = "entry:\nbr label%1\nlabel%2:\nret i64 0";
        let _result = opt.eliminate_unreachable_blocks(ir);
    }

    #[test]
    fn test_simplify_constants() {
        let mut opt = LLVMIROptimizer::new(OptimizationLevel::O1);
        let ir = "%1 = add i64 0, %x";
        let _result = opt.simplify_constants(ir);
    }
}
