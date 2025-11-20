
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct DeadCodeEliminator {
    live_variables: HashSet<String>,
    dead_instructions: Vec<usize>,
    statistics: DCEStats,
}

#[derive(Debug, Clone, Copy)]
pub struct DCEStats {
    pub lines_removed: usize,
    pub variables_eliminated: usize,
    pub blocks_removed: usize,
    pub bytes_saved: usize,
}

impl DeadCodeEliminator {
    pub fn new() -> Self {
        DeadCodeEliminator {
            live_variables: HashSet::new(),
            dead_instructions: Vec::new(),
            statistics: DCEStats {
                lines_removed: 0,
                variables_eliminated: 0,
                blocks_removed: 0,
                bytes_saved: 0,
            },
        }
    }

    pub fn eliminate(&mut self, ir: &str) -> String {
        let lines: Vec<&str> = ir.lines().collect();
        let mut result = String::new();

        self.collect_live_variables(&lines);
        self.mark_dead_instructions(&lines);

        for (idx, line) in lines.iter().enumerate() {
            if !self.dead_instructions.contains(&idx) {
                result.push_str(line);
                result.push('\n');
            } else {
                self.statistics.lines_removed += 1;
                self.statistics.bytes_saved += line.len();
            }
        }

        result.trim().to_string()
    }

    fn collect_live_variables(&mut self, lines: &[&str]) {
        for line in lines.iter().rev() {
            if line.contains("ret ") {
                if let Some(var) = line.split_whitespace().last() {
                    self.live_variables.insert(var.to_string());
                }
            }

            if line.contains("printf") || line.contains("call ") {
                for word in line.split_whitespace() {
                    if word.starts_with("%") {
                        self.live_variables.insert(word.to_string());
                    }
                }
            }

            if line.contains("store ") || line.contains("br ") {
                for word in line.split_whitespace() {
                    if word.starts_with("%") {
                        self.live_variables.insert(word.to_string());
                    }
                }
            }

            if let Some(var) = self.extract_definition(line) {
                if !self.live_variables.contains(&var) {
                    self.statistics.variables_eliminated += 1;
                }
            }

            self.extract_uses(line)
                .iter()
                .for_each(|v| {
                    self.live_variables.insert(v.clone());
                });
        }
    }

    fn mark_dead_instructions(&mut self, lines: &[&str]) {
        for (idx, line) in lines.iter().enumerate() {
            if let Some(var) = self.extract_definition(line) {
                if !self.live_variables.contains(&var) && !self.has_side_effects(line) {
                    self.dead_instructions.push(idx);
                }
            }
        }
    }

    fn extract_definition(&self, line: &str) -> Option<String> {
        if let Some(eq_pos) = line.find('=') {
            let lhs = line[..eq_pos].trim();
            if lhs.starts_with("%") {
                return Some(lhs.to_string());
            }
        }
        None
    }

    fn extract_uses(&self, line: &str) -> Vec<String> {
        let mut uses = Vec::new();
        
        for word in line.split_whitespace() {
            if word.starts_with("%") {
                let clean_word = word.trim_end_matches(',').to_string();
                if line.contains(&format!("{} =", clean_word)) {
                    continue;
                }
                uses.push(clean_word);
            }
        }
        uses
    }

    fn has_side_effects(&self, line: &str) -> bool {
        line.contains("call ")
            || line.contains("store ")
            || line.contains("ret ")
            || line.contains("br ")
            || line.contains("printf")
    }

    pub fn eliminate_unreachable_blocks(&mut self, ir: &str) -> String {
        let lines: Vec<&str> = ir.lines().collect();
        let mut reachable_blocks = HashSet::new();
        let mut to_visit = vec![0];

        while let Some(block_idx) = to_visit.pop() {
            if reachable_blocks.contains(&block_idx) {
                continue;
            }
            reachable_blocks.insert(block_idx);

            if block_idx < lines.len() {
                let line = lines[block_idx];

                if line.contains("br label%") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for part in parts {
                        if part.starts_with("label%") {
                            if let Ok(num) = part[6..].parse::<usize>() {
                                to_visit.push(num);
                            }
                        }
                    }
                } else if !line.contains("ret ") && !line.contains("br ") {
                    to_visit.push(block_idx + 1);
                }
            }
        }

        let mut result = String::new();
        for (idx, line) in lines.iter().enumerate() {
            if reachable_blocks.contains(&idx) {
                result.push_str(line);
                result.push('\n');
            } else {
                self.statistics.blocks_removed += 1;
            }
        }

        result.trim().to_string()
    }

    pub fn eliminate_unused_variables(&mut self, ir: &str) -> String {
        let mut used_vars = HashSet::new();
        let lines: Vec<&str> = ir.lines().collect();

        for line in lines.iter().rev() {
            if line.contains("ret ") || line.contains("call ") {
                for word in line.split_whitespace() {
                    if word.starts_with("%") {
                        used_vars.insert(word.to_string());
                    }
                }
            }
        }

        let mut result = String::new();
        for line in lines {
            let keep = if let Some(var) = self.extract_definition(line) {
                used_vars.contains(&var) || self.has_side_effects(line)
            } else {
                true
            };

            if keep {
                result.push_str(line);
                result.push('\n');
            }
        }

        result.trim().to_string()
    }

    pub fn get_statistics(&self) -> DCEStats {
        self.statistics
    }

    pub fn clear_statistics(&mut self) {
        self.statistics = DCEStats {
            lines_removed: 0,
            variables_eliminated: 0,
            blocks_removed: 0,
            bytes_saved: 0,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dce_creation() {
        let _dce = DeadCodeEliminator::new();
    }

    #[test]
    fn test_extract_definition() {
        let dce = DeadCodeEliminator::new();
        let line = "%x = add i64 1, 2";
        assert_eq!(dce.extract_definition(line), Some("%x".to_string()));
    }

    #[test]
    fn test_extract_uses() {
        let dce = DeadCodeEliminator::new();
        let line = "%x = add i64 %y, %z";
        let uses = dce.extract_uses(line);
        assert!(uses.contains(&"%y".to_string()));
        assert!(uses.contains(&"%z".to_string()));
    }

    #[test]
    fn test_has_side_effects() {
        let dce = DeadCodeEliminator::new();
        assert!(dce.has_side_effects("call @printf(...)"));
        assert!(dce.has_side_effects("store i64 1, i64* %x"));
        assert!(dce.has_side_effects("ret i64 0"));
    }

    #[test]
    fn test_simple_elimination() {
        let mut dce = DeadCodeEliminator::new();
        let ir = "%x = add i64 1, 2\nret i64 0";
        let result = dce.eliminate(ir);
        assert!(result.contains("ret i64 0"));
    }
}
