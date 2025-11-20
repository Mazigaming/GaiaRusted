
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct InliningStrategy {
    pub name: String,
    pub enabled: bool,
    pub threshold: usize,
}

#[derive(Debug, Clone)]
pub struct FunctionMetrics {
    pub size: usize,
    pub call_count: usize,
    pub is_recursive: bool,
    pub complexity: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct InliningStats {
    pub functions_inlined: usize,
    pub call_sites_eliminated: usize,
    pub bytes_saved: usize,
    pub code_bloat: usize,
}

pub struct Inliner {
    strategies: Vec<InliningStrategy>,
    function_metrics: HashMap<String, FunctionMetrics>,
    statistics: InliningStats,
}

impl Inliner {
    pub fn new() -> Self {
        Inliner {
            strategies: vec![
                InliningStrategy {
                    name: "Simple".to_string(),
                    enabled: true,
                    threshold: 50,
                },
                InliningStrategy {
                    name: "Aggressive".to_string(),
                    enabled: false,
                    threshold: 200,
                },
                InliningStrategy {
                    name: "Conservative".to_string(),
                    enabled: false,
                    threshold: 20,
                },
            ],
            function_metrics: HashMap::new(),
            statistics: InliningStats {
                functions_inlined: 0,
                call_sites_eliminated: 0,
                bytes_saved: 0,
                code_bloat: 0,
            },
        }
    }

    pub fn analyze_functions(&mut self, ir: &str) {
        let lines: Vec<&str> = ir.lines().collect();
        let mut current_function: Option<String> = None;
        let mut current_size = 0;

        for line in lines {
            if line.contains("define ") {
                if let Some(func_name) = self.extract_function_name(line) {
                    current_function = Some(func_name);
                    current_size = 0;
                }
            } else if line.contains("ret ") {
                if let Some(func) = current_function.clone() {
                    let metrics = FunctionMetrics {
                        size: current_size,
                        call_count: 0,
                        is_recursive: false,
                        complexity: self.estimate_complexity(ir, &func),
                    };
                    self.function_metrics.insert(func, metrics);
                    current_function = None;
                }
            } else if current_function.is_some() {
                current_size += line.len();
            }
        }
    }

    fn extract_function_name(&self, line: &str) -> Option<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        for part in parts {
            if part.starts_with("@") {
                let name = part[1..].to_string();
                let name = if let Some(paren_pos) = name.find('(') {
                    name[..paren_pos].to_string()
                } else {
                    name
                };
                return Some(name);
            }
        }
        None
    }

    fn estimate_complexity(&self, ir: &str, func_name: &str) -> usize {
        let mut complexity = 0;
        let mut in_function = false;

        for line in ir.lines() {
            if line.contains(&format!("@{}", func_name)) {
                in_function = true;
            }

            if in_function {
                if line.contains("call ") {
                    complexity += 5;
                }
                if line.contains("br i1 ") || line.contains("switch ") {
                    complexity += 3;
                }
                if line.contains("loop") || line.contains("for") {
                    complexity += 10;
                }

                if line.contains("ret ") {
                    break;
                }
            }
        }

        complexity
    }

    pub fn should_inline(&self, func_name: &str) -> bool {
        if let Some(metrics) = self.function_metrics.get(func_name) {
            for strategy in &self.strategies {
                if strategy.enabled && metrics.size < strategy.threshold {
                    return !metrics.is_recursive;
                }
            }
        }
        false
    }

    pub fn inline_functions(&mut self, ir: &str) -> String {
        let mut result = String::new();

        for line in ir.lines() {
            if line.contains("call ") {
                if let Some(func_name) = self.extract_call_target(line) {
                    if self.should_inline(&func_name) {
                        result.push_str("; inlined ");
                        result.push_str(&func_name);
                        result.push('\n');
                        self.statistics.call_sites_eliminated += 1;
                        self.statistics.functions_inlined += 1;
                        continue;
                    }
                }
            }
            result.push_str(line);
            result.push('\n');
        }

        result.trim().to_string()
    }

    fn extract_call_target(&self, line: &str) -> Option<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        for part in parts {
            if part.starts_with("@") {
                let name = part[1..].to_string();
                let name = if let Some(paren_pos) = name.find('(') {
                    name[..paren_pos].to_string()
                } else {
                    name
                };
                return Some(name);
            }
        }
        None
    }

    pub fn enable_strategy(&mut self, strategy_name: &str) -> Result<(), String> {
        for strategy in &mut self.strategies {
            if strategy.name == strategy_name {
                strategy.enabled = true;
                return Ok(());
            }
        }
        Err(format!("Strategy {} not found", strategy_name))
    }

    pub fn disable_strategy(&mut self, strategy_name: &str) -> Result<(), String> {
        for strategy in &mut self.strategies {
            if strategy.name == strategy_name {
                strategy.enabled = false;
                return Ok(());
            }
        }
        Err(format!("Strategy {} not found", strategy_name))
    }

    pub fn set_threshold(&mut self, strategy_name: &str, threshold: usize) -> Result<(), String> {
        for strategy in &mut self.strategies {
            if strategy.name == strategy_name {
                strategy.threshold = threshold;
                return Ok(());
            }
        }
        Err(format!("Strategy {} not found", strategy_name))
    }

    pub fn estimate_bloat(&self) -> usize {
        let mut bloat = 0;
        for metrics in self.function_metrics.values() {
            if metrics.size > 100 {
                bloat += metrics.size / 10;
            }
        }
        bloat
    }

    pub fn get_inlinable_functions(&self) -> Vec<String> {
        self.function_metrics
            .iter()
            .filter(|(name, metrics)| {
                self.should_inline(name) && !metrics.is_recursive && metrics.size < 100
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn get_statistics(&self) -> InliningStats {
        self.statistics
    }

    pub fn get_function_metrics(&self, func_name: &str) -> Option<FunctionMetrics> {
        self.function_metrics.get(func_name).cloned()
    }

    pub fn clear_statistics(&mut self) {
        self.statistics = InliningStats {
            functions_inlined: 0,
            call_sites_eliminated: 0,
            bytes_saved: 0,
            code_bloat: 0,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inliner_creation() {
        let _inliner = Inliner::new();
    }

    #[test]
    fn test_extract_function_name() {
        let inliner = Inliner::new();
        let line = "define i64 @add(i64 %a, i64 %b)";
        assert_eq!(inliner.extract_function_name(line), Some("add".to_string()));
    }

    #[test]
    fn test_extract_call_target() {
        let inliner = Inliner::new();
        let line = "%result = call i64 @add(i64 1, i64 2)";
        assert_eq!(inliner.extract_call_target(line), Some("add".to_string()));
    }

    #[test]
    fn test_enable_strategy() {
        let mut inliner = Inliner::new();
        assert!(inliner.enable_strategy("Aggressive").is_ok());
    }

    #[test]
    fn test_disable_strategy() {
        let mut inliner = Inliner::new();
        assert!(inliner.disable_strategy("Simple").is_ok());
    }

    #[test]
    fn test_set_threshold() {
        let mut inliner = Inliner::new();
        assert!(inliner.set_threshold("Simple", 100).is_ok());
    }

    #[test]
    fn test_statistics() {
        let inliner = Inliner::new();
        let stats = inliner.get_statistics();
        assert_eq!(stats.functions_inlined, 0);
    }

    #[test]
    fn test_inlinable_functions() {
        let inliner = Inliner::new();
        let inlinable = inliner.get_inlinable_functions();
        assert!(inlinable.is_empty());
    }
}
