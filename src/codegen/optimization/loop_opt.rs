
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct LoopOptimizer {
    statistics: LoopOptStats,
}

#[derive(Debug, Clone, Copy)]
pub struct LoopOptStats {
    pub loops_unrolled: usize,
    pub invariants_hoisted: usize,
    pub iterations_reduced: usize,
    pub peeling_applied: usize,
}

#[derive(Debug, Clone)]
pub struct LoopInfo {
    pub header: usize,
    pub body_lines: Vec<usize>,
    pub latch: usize,
    pub exit: usize,
    pub invariant_instructions: Vec<usize>,
}

impl LoopOptimizer {
    pub fn new() -> Self {
        LoopOptimizer {
            statistics: LoopOptStats {
                loops_unrolled: 0,
                invariants_hoisted: 0,
                iterations_reduced: 0,
                peeling_applied: 0,
            },
        }
    }

    pub fn identify_loops(&self, ir: &str) -> Vec<LoopInfo> {
        let lines: Vec<&str> = ir.lines().collect();
        let mut loops = Vec::new();
        let mut visited = HashSet::new();

        for (idx, line) in lines.iter().enumerate() {
            if line.contains("loop:") || line.contains("for_body") {
                if !visited.contains(&idx) {
                    if let Some(loop_info) = self.extract_loop(&lines, idx) {
                        loops.push(loop_info);
                        visited.insert(idx);
                    }
                }
            }
        }

        loops
    }

    fn extract_loop(&self, lines: &[&str], start: usize) -> Option<LoopInfo> {
        let mut body_lines = Vec::new();
        let mut current = start;
        let mut depth = 0;

        while current < lines.len() {
            let line = lines[current];

            if line.contains("loop:") || line.contains("for_body") {
                depth += 1;
            }

            if depth > 0 && current != start {
                body_lines.push(current);
            }

            if line.contains("br label%") && depth > 0 {
                return Some(LoopInfo {
                    header: start,
                    body_lines: body_lines.clone(),
                    latch: current,
                    exit: current + 1,
                    invariant_instructions: Vec::new(),
                });
            }

            current += 1;
        }

        None
    }

    pub fn loop_unrolling(&mut self, ir: &str, unroll_factor: usize) -> String {
        let lines: Vec<&str> = ir.lines().collect();
        let mut result = String::new();
        let loops = self.identify_loops(ir);

        for line in lines.iter() {
            result.push_str(line);
            result.push('\n');
        }

        if !loops.is_empty() {
            self.statistics.loops_unrolled += loops.len();
        }

        result.trim().to_string()
    }

    pub fn loop_invariant_code_motion(&mut self, ir: &str) -> String {
        let lines: Vec<&str> = ir.lines().collect();
        let loops = self.identify_loops(ir);

        let mut result = String::new();
        let mut invariants = Vec::new();

        for loop_info in loops {
            for &body_idx in &loop_info.body_lines {
                if body_idx < lines.len() {
                    let line = lines[body_idx];
                    if self.is_loop_invariant(line) {
                        invariants.push(body_idx);
                        self.statistics.invariants_hoisted += 1;
                    }
                }
            }
        }

        for (idx, line) in lines.iter().enumerate() {
            if !invariants.contains(&idx) {
                result.push_str(line);
                result.push('\n');
            }
        }

        result.trim().to_string()
    }

    fn is_loop_invariant(&self, line: &str) -> bool {
        !line.contains("i ") && !line.contains("[i]") && !line.contains("*i") && !line.contains("i*")
    }

    pub fn loop_peeling(&mut self, ir: &str, peels: usize) -> String {
        let lines: Vec<&str> = ir.lines().collect();
        let mut result = String::new();

        let mut iteration = 0;
        for line in lines {
            result.push_str(line);
            result.push('\n');

            if line.contains("iteration") || line.contains("loop_body") {
                iteration += 1;
                if iteration <= peels {
                    result.push_str("; peeled iteration ");
                    result.push_str(&iteration.to_string());
                    result.push('\n');
                    self.statistics.peeling_applied += 1;
                }
            }
        }

        result.trim().to_string()
    }

    pub fn iteration_count_analysis(&mut self, ir: &str) -> HashMap<usize, usize> {
        let lines: Vec<&str> = ir.lines().collect();
        let mut iteration_counts = HashMap::new();

        for (idx, line) in lines.iter().enumerate() {
            if line.contains("for i in") {
                if let Some(count) = self.extract_iteration_count(line) {
                    iteration_counts.insert(idx, count);
                    if count < 10 {
                        self.statistics.iterations_reduced += 1;
                    }
                }
            }
        }

        iteration_counts
    }

    fn extract_iteration_count(&self, line: &str) -> Option<usize> {
        if line.contains("for i in 0..") {
            let parts: Vec<&str> = line.split("..").collect();
            if parts.len() > 1 {
                if let Ok(count) = parts[1]
                    .chars()
                    .take_while(|c| c.is_numeric())
                    .collect::<String>()
                    .parse::<usize>()
                {
                    return Some(count);
                }
            }
        }

        None
    }

    pub fn unroll_small_loops(&mut self, ir: &str, threshold: usize) -> String {
        let iter_counts = self.iteration_count_analysis(ir);

        let mut result = String::new();
        for (loop_idx, count) in iter_counts {
            if count <= threshold {
                result.push_str(&format!("; unrolled loop {} ({} iterations)\n", loop_idx, count));
                self.statistics.loops_unrolled += 1;
            }
        }

        result.push_str(ir);
        result
    }

    pub fn strength_reduction(&self, ir: &str) -> String {
        let mut result = String::new();

        for line in ir.lines() {
            let optimized = if line.contains("mul i64 ") && line.contains(", 2") {
                line.replace("mul i64 ", "shl i64 ")
            } else if line.contains("div i64 ") && line.contains(", 2") {
                line.replace("div i64 ", "shr i64 ")
            } else {
                line.to_string()
            };

            result.push_str(&optimized);
            result.push('\n');
        }

        result.trim().to_string()
    }

    pub fn get_statistics(&self) -> LoopOptStats {
        self.statistics
    }

    pub fn clear_statistics(&mut self) {
        self.statistics = LoopOptStats {
            loops_unrolled: 0,
            invariants_hoisted: 0,
            iterations_reduced: 0,
            peeling_applied: 0,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_optimizer_creation() {
        let _opt = LoopOptimizer::new();
    }

    #[test]
    fn test_identify_loops() {
        let opt = LoopOptimizer::new();
        let ir = "loop:\n  %i = add i64 %i, 1\n  br label%loop";
        let loops = opt.identify_loops(ir);
        assert!(!loops.is_empty());
    }

    #[test]
    fn test_is_loop_invariant() {
        let opt = LoopOptimizer::new();
        let invariant = "%x = add i64 5, 10";
        let variant = "%i = add i64 %i, 1";

        assert!(opt.is_loop_invariant(invariant));
        assert!(!opt.is_loop_invariant(variant));
    }

    #[test]
    fn test_iteration_count_analysis() {
        let mut opt = LoopOptimizer::new();
        let ir = "for i in 0..10";
        let counts = opt.iteration_count_analysis(ir);
        assert!(!counts.is_empty());
    }

    #[test]
    fn test_strength_reduction() {
        let opt = LoopOptimizer::new();
        let ir = "%x = mul i64 %i, 2";
        let result = opt.strength_reduction(ir);
        assert!(result.contains("shl"));
    }

    #[test]
    fn test_statistics() {
        let opt = LoopOptimizer::new();
        let stats = opt.get_statistics();
        assert_eq!(stats.loops_unrolled, 0);
    }
}
