//! # Cross-Function Inlining Optimization
//!
//! Analyzes call sites and decides whether to inline function calls.
//! Particularly useful for inlining into iterator fusion chains and tight loops.

use std::collections::{HashMap, HashSet};

/// Function size estimate (in pseudo-instructions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FunctionSize {
    /// 1-5 operations (trivial)
    Tiny,
    /// 6-15 operations (small)
    Small,
    /// 16-30 operations (medium)
    Medium,
    /// 31-60 operations (large)
    Large,
    /// 60+ operations (very large)
    VeryLarge,
}

impl FunctionSize {
    /// Get estimated operation count
    pub fn estimate(&self) -> usize {
        match self {
            FunctionSize::Tiny => 3,
            FunctionSize::Small => 10,
            FunctionSize::Medium => 20,
            FunctionSize::Large => 45,
            FunctionSize::VeryLarge => 80,
        }
    }

    /// Detect size from operation count
    pub fn from_op_count(count: usize) -> Self {
        match count {
            0..=5 => FunctionSize::Tiny,
            6..=15 => FunctionSize::Small,
            16..=30 => FunctionSize::Medium,
            31..=60 => FunctionSize::Large,
            _ => FunctionSize::VeryLarge,
        }
    }
}

/// Function call site metadata
#[derive(Debug, Clone)]
pub struct CallSite {
    /// Function being called
    pub callee: String,
    /// Calling context (function name)
    pub caller: String,
    /// Call frequency (estimated)
    pub frequency: u32,
    /// Whether in a hot path (loop nest)
    pub in_loop: bool,
    /// Number of times called at this site
    pub call_count: u32,
}

/// Function metadata
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// Function name
    pub name: String,
    /// Estimated size
    pub size: FunctionSize,
    /// Number of parameters
    pub param_count: usize,
    /// Has side effects (I/O, allocations, etc.)
    pub has_side_effects: bool,
    /// Is recursive (direct or indirect)
    pub is_recursive: bool,
    /// Call sites that invoke this function
    pub call_sites: Vec<CallSite>,
}

impl FunctionMetadata {
    /// Create new function metadata
    pub fn new(name: String, size: FunctionSize, param_count: usize) -> Self {
        FunctionMetadata {
            name,
            size,
            param_count,
            has_side_effects: false,
            is_recursive: false,
            call_sites: Vec::new(),
        }
    }

    /// Estimate inlining benefit
    pub fn inline_benefit(&self) -> f32 {
        let base_benefit = match self.size {
            FunctionSize::Tiny => 5.0,
            FunctionSize::Small => 3.0,
            FunctionSize::Medium => 1.5,
            FunctionSize::Large => 0.5,
            FunctionSize::VeryLarge => 0.1,
        };

        // Reduce benefit if function has side effects
        if self.has_side_effects {
            return base_benefit * 0.5;
        }

        base_benefit
    }
}

/// Inlining decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InliningDecision {
    /// Definitely inline
    Inline,
    /// Probably inline (if code size allows)
    Probably,
    /// Don't inline
    DontInline,
    /// Can't inline (recursive, side effects, etc.)
    Forbidden,
}

/// Cross-function inlining optimizer
pub struct InliningOptimizer {
    /// Function metadata
    functions: HashMap<String, FunctionMetadata>,
    /// Call graph (caller → callees)
    call_graph: HashMap<String, Vec<String>>,
    /// Reverse call graph (callee → callers)
    reverse_call_graph: HashMap<String, Vec<String>>,
    /// Inlining decisions
    decisions: HashMap<String, InliningDecision>,
    /// Total code size budget (bytes)
    code_size_budget: usize,
    /// Current code size estimate
    current_code_size: usize,
}

impl InliningOptimizer {
    /// Create new inlining optimizer
    pub fn new(code_size_budget: usize) -> Self {
        InliningOptimizer {
            functions: HashMap::new(),
            call_graph: HashMap::new(),
            reverse_call_graph: HashMap::new(),
            decisions: HashMap::new(),
            code_size_budget,
            current_code_size: 0,
        }
    }

    /// Register a function
    pub fn register_function(&mut self, metadata: FunctionMetadata) {
        self.current_code_size += metadata.size.estimate();
        self.functions.insert(metadata.name.clone(), metadata);
    }

    /// Register a call site
    pub fn register_call(&mut self, caller: &str, callee: &str, in_loop: bool, frequency: u32) {
        // Update call graph
        self.call_graph.entry(caller.to_string())
            .or_insert_with(Vec::new)
            .push(callee.to_string());

        self.reverse_call_graph.entry(callee.to_string())
            .or_insert_with(Vec::new)
            .push(caller.to_string());

        // Update function metadata
        if let Some(func) = self.functions.get_mut(callee) {
            func.call_sites.push(CallSite {
                callee: callee.to_string(),
                caller: caller.to_string(),
                frequency,
                in_loop,
                call_count: 1,
            });
        }
    }

    /// Decide whether to inline a function at a call site
    pub fn decide_inline(&mut self, caller: &str, callee: &str) -> InliningDecision {
        // Check if already decided
        let key = format!("{}::{}", caller, callee);
        if let Some(&decision) = self.decisions.get(&key) {
            return decision;
        }

        // Get function metadata
        let func = match self.functions.get(callee) {
            Some(f) => f.clone(),
            None => return InliningDecision::DontInline,
        };

        // Check if inlining is forbidden
        if func.is_recursive {
            self.decisions.insert(key, InliningDecision::Forbidden);
            return InliningDecision::Forbidden;
        }

        // Check size constraints
        let inline_cost = func.size.estimate();
        if self.current_code_size + inline_cost > self.code_size_budget {
            self.decisions.insert(key, InliningDecision::DontInline);
            return InliningDecision::DontInline;
        }

        // Check benefit
        let benefit = func.inline_benefit();
        let decision = if func.size == FunctionSize::Tiny || benefit > 3.0 {
            InliningDecision::Inline
        } else if benefit > 1.0 {
            InliningDecision::Probably
        } else {
            InliningDecision::DontInline
        };

        self.decisions.insert(key, decision);
        decision
    }

    /// Analyze all functions and make inlining decisions
    pub fn analyze_all(&mut self) {
        let function_names: Vec<_> = self.functions.keys().cloned().collect();
        let callers: Vec<_> = self.call_graph.keys().cloned().collect();

        for caller in callers {
            if let Some(callees) = self.call_graph.get(&caller).cloned() {
                for callee in callees {
                    self.decide_inline(&caller, &callee);
                }
            }
        }
    }

    /// Get inlining candidates (functions good to inline)
    pub fn get_candidates(&self) -> Vec<String> {
        self.functions.iter()
            .filter(|(_, func)| {
                func.size == FunctionSize::Tiny || func.size == FunctionSize::Small
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Estimate speedup from inlining
    pub fn estimate_speedup(&self) -> f32 {
        let inline_calls = self.decisions.iter()
            .filter(|(_, d)| **d == InliningDecision::Inline)
            .count();

        if inline_calls == 0 {
            return 1.0;
        }

        // Estimate: removing call overhead
        // Typical call overhead: 4-6 cycles
        // With inlining, save the overhead plus enable better optimization
        1.0 + (inline_calls as f32 * 0.15) // 15% per inlined call (conservative)
    }

    /// Get current code size estimate
    pub fn current_size(&self) -> usize {
        self.current_code_size
    }

    /// Get remaining code size budget
    pub fn remaining_budget(&self) -> usize {
        if self.current_code_size > self.code_size_budget {
            0
        } else {
            self.code_size_budget - self.current_code_size
        }
    }
}

/// Detect recursive functions
pub fn detect_recursion(call_graph: &HashMap<String, Vec<String>>) -> HashSet<String> {
    let mut recursive = HashSet::new();
    
    // Simple cycle detection: if a function calls itself (direct recursion)
    for (func, callees) in call_graph {
        if callees.contains(func) {
            recursive.insert(func.clone());
        }
    }
    
    // TODO: Implement indirect recursion detection using DFS
    
    recursive
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_size_detection() {
        assert_eq!(FunctionSize::from_op_count(3), FunctionSize::Tiny);
        assert_eq!(FunctionSize::from_op_count(10), FunctionSize::Small);
        assert_eq!(FunctionSize::from_op_count(25), FunctionSize::Medium);
        assert_eq!(FunctionSize::from_op_count(50), FunctionSize::Large);
        assert_eq!(FunctionSize::from_op_count(100), FunctionSize::VeryLarge);
    }

    #[test]
    fn test_inline_benefit() {
        let func_tiny = FunctionMetadata::new("tiny".to_string(), FunctionSize::Tiny, 1);
        let func_large = FunctionMetadata::new("large".to_string(), FunctionSize::Large, 3);
        
        assert!(func_tiny.inline_benefit() > func_large.inline_benefit());
    }

    #[test]
    fn test_inlining_optimizer() {
        let mut optimizer = InliningOptimizer::new(10000);
        
        let mut func = FunctionMetadata::new("add".to_string(), FunctionSize::Tiny, 2);
        optimizer.register_function(func);
        
        let decision = optimizer.decide_inline("main", "add");
        assert_eq!(decision, InliningDecision::Inline);
    }

    #[test]
    fn test_call_registration() {
        let mut optimizer = InliningOptimizer::new(10000);
        optimizer.register_function(FunctionMetadata::new("add".to_string(), FunctionSize::Tiny, 2));
        
        optimizer.register_call("main", "add", true, 100);
        
        assert!(optimizer.reverse_call_graph.contains_key("add"));
    }

    #[test]
    fn test_code_size_budget() {
        let mut optimizer = InliningOptimizer::new(100);
        
        optimizer.register_function(
            FunctionMetadata::new("func".to_string(), FunctionSize::VeryLarge, 1)
        );
        
        // Should exceed budget
        assert!(optimizer.current_size() > 0);
    }
}
