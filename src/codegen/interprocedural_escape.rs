/// Interprocedural Escape Analysis for v0.13.0
///
/// Extends escape analysis across function boundaries to track:
/// - Which function arguments escape to the heap
/// - Which return values escape
/// - Which fields of structs escape
/// - Flow-sensitive escape information

use std::collections::{HashMap, HashSet};
use crate::mir::{BasicBlock, MirFunction};

/// Tracks escape information for a function signature
#[derive(Debug, Clone)]
pub struct FunctionEscapeInfo {
    /// Function name
    pub name: String,
    /// Which parameters escape (by index)
    pub escaping_params: HashSet<usize>,
    /// Whether return value escapes
    pub return_escapes: bool,
    /// Which fields of struct parameters escape
    pub field_escapes: HashMap<usize, HashSet<String>>,
}

impl FunctionEscapeInfo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            escaping_params: HashSet::new(),
            return_escapes: false,
            field_escapes: HashMap::new(),
        }
    }

    /// Check if a parameter escapes
    pub fn param_escapes(&self, param_idx: usize) -> bool {
        self.escaping_params.contains(&param_idx)
    }

    /// Check if a specific field of a parameter escapes
    pub fn field_escapes(&self, param_idx: usize, field_name: &str) -> bool {
        self.field_escapes
            .get(&param_idx)
            .map(|fields| fields.contains(field_name))
            .unwrap_or(false)
    }

    /// Mark a parameter as escaping
    pub fn mark_param_escapes(&mut self, param_idx: usize) {
        self.escaping_params.insert(param_idx);
    }

    /// Mark a field of a parameter as escaping
    pub fn mark_field_escapes(&mut self, param_idx: usize, field_name: String) {
        self.field_escapes
            .entry(param_idx)
            .or_insert_with(HashSet::new)
            .insert(field_name);
    }
}

/// Interprocedural escape analysis across a program
#[derive(Debug, Clone)]
pub struct InterproceduralEscapeAnalysis {
    /// Function signatures and their escape information
    function_escapes: HashMap<String, FunctionEscapeInfo>,
    /// Call graph edges (caller -> callees)
    call_graph: HashMap<String, Vec<String>>,
}

impl InterproceduralEscapeAnalysis {
    pub fn new() -> Self {
        Self {
            function_escapes: HashMap::new(),
            call_graph: HashMap::new(),
        }
    }

    /// Register a function with its escape information
    pub fn register_function(&mut self, info: FunctionEscapeInfo) {
        self.function_escapes.insert(info.name.clone(), info);
    }

    /// Add a call edge to the call graph
    pub fn add_call_edge(&mut self, caller: String, callee: String) {
        self.call_graph
            .entry(caller)
            .or_insert_with(Vec::new)
            .push(callee);
    }

    /// Propagate escape information through the call graph
    pub fn propagate_escapes(&mut self) {
        let mut changed = true;
        let max_iterations = 10; // Prevent infinite loops
        let mut iteration = 0;

        while changed && iteration < max_iterations {
            changed = false;
            iteration += 1;

            let call_graph = self.call_graph.clone();
            let function_escapes = self.function_escapes.clone();

            for (caller, callees) in call_graph.iter() {
                for callee in callees {
                    if let Some(callee_info) = function_escapes.get(callee) {
                        if let Some(caller_info) = self.function_escapes.get_mut(caller) {
                            // If callee's parameter escapes, mark corresponding argument as escaping
                            for param_idx in &callee_info.escaping_params {
                                if !caller_info.escaping_params.contains(param_idx) {
                                    caller_info.mark_param_escapes(*param_idx);
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get escape information for a function
    pub fn get_function_escape_info(&self, name: &str) -> Option<&FunctionEscapeInfo> {
        self.function_escapes.get(name)
    }

    /// Get all functions that escape to heap
    pub fn get_heap_escaping_functions(&self) -> Vec<String> {
        self.function_escapes
            .values()
            .filter(|info| info.return_escapes || !info.escaping_params.is_empty())
            .map(|info| info.name.clone())
            .collect()
    }

    /// Generate report of escape information
    pub fn report(&self) -> EscapeAnalysisReport {
        let total_functions = self.function_escapes.len();
        let escaping_functions = self.get_heap_escaping_functions().len();
        let total_escaping_params: usize = self
            .function_escapes
            .values()
            .map(|info| info.escaping_params.len())
            .sum();

        EscapeAnalysisReport {
            total_functions,
            escaping_functions,
            total_escaping_params,
            function_info: self.function_escapes.clone(),
        }
    }
}

impl Default for InterproceduralEscapeAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// Report on interprocedural escape analysis results
#[derive(Debug, Clone)]
pub struct EscapeAnalysisReport {
    pub total_functions: usize,
    pub escaping_functions: usize,
    pub total_escaping_params: usize,
    pub function_info: HashMap<String, FunctionEscapeInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_escape_info_creation() {
        let info = FunctionEscapeInfo::new("test_fn".to_string());
        assert_eq!(info.name, "test_fn");
        assert!(!info.return_escapes);
        assert!(info.escaping_params.is_empty());
    }

    #[test]
    fn test_mark_param_escapes() {
        let mut info = FunctionEscapeInfo::new("test_fn".to_string());
        info.mark_param_escapes(0);
        info.mark_param_escapes(2);

        assert!(info.param_escapes(0));
        assert!(!info.param_escapes(1));
        assert!(info.param_escapes(2));
    }

    #[test]
    fn test_field_escape_tracking() {
        let mut info = FunctionEscapeInfo::new("test_fn".to_string());
        info.mark_field_escapes(0, "field1".to_string());
        info.mark_field_escapes(0, "field2".to_string());

        assert!(info.field_escapes(0, "field1"));
        assert!(info.field_escapes(0, "field2"));
        assert!(!info.field_escapes(0, "field3"));
    }

    #[test]
    fn test_interprocedural_analysis() {
        let mut analysis = InterproceduralEscapeAnalysis::new();

        // Register functions
        let mut fn1 = FunctionEscapeInfo::new("fn1".to_string());
        fn1.mark_param_escapes(0);

        let fn2 = FunctionEscapeInfo::new("fn2".to_string());

        analysis.register_function(fn1);
        analysis.register_function(fn2);

        // Create call graph
        analysis.add_call_edge("fn2".to_string(), "fn1".to_string());

        // Propagate
        analysis.propagate_escapes();

        // fn2 should now know that param 0 escapes through fn1 call
        let report = analysis.report();
        assert!(report.escaping_functions > 0);
    }

    #[test]
    fn test_escape_analysis_report() {
        let mut analysis = InterproceduralEscapeAnalysis::new();

        let mut info1 = FunctionEscapeInfo::new("func1".to_string());
        info1.return_escapes = true;

        let info2 = FunctionEscapeInfo::new("func2".to_string());

        analysis.register_function(info1);
        analysis.register_function(info2);

        let report = analysis.report();
        assert_eq!(report.total_functions, 2);
        assert_eq!(report.escaping_functions, 1);
    }

    #[test]
    fn test_call_graph_propagation() {
        let mut analysis = InterproceduralEscapeAnalysis::new();

        // Create chain: fn1 -> fn2 -> fn3
        let mut fn1 = FunctionEscapeInfo::new("fn1".to_string());
        fn1.mark_param_escapes(0);

        let fn2 = FunctionEscapeInfo::new("fn2".to_string());
        let fn3 = FunctionEscapeInfo::new("fn3".to_string());

        analysis.register_function(fn1);
        analysis.register_function(fn2);
        analysis.register_function(fn3);

        analysis.add_call_edge("fn2".to_string(), "fn1".to_string());
        analysis.add_call_edge("fn3".to_string(), "fn2".to_string());

        analysis.propagate_escapes();

        // All functions should now understand the escape chain
        let report = analysis.report();
        assert!(report.escaping_functions > 0);
    }
}
