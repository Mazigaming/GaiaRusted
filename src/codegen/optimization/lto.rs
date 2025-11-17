//! Link-Time Optimization (LTO)
//!
//! Cross-module and cross-function optimization performed at link time.
//! Optimizations include:
//! - Inter-procedural dead code elimination
//! - Whole-program constant propagation
//! - Cross-module function inlining
//! - Cross-module code deduplication
//! - Global data flow analysis
//! - Function specialization for known call sites

use std::collections::{HashMap, HashSet};

/// Symbol visibility in LTO context
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolVisibility {
    Public,
    Internal,
    Hidden,
}

/// Information about a function for LTO analysis
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub visibility: SymbolVisibility,
    pub call_count: usize,
    pub call_sites: Vec<String>,
    pub instruction_count: usize,
    pub is_recursive: bool,
    pub uses_globals: Vec<String>,
    pub module: String,
}

/// Global variable information
#[derive(Debug, Clone)]
pub struct GlobalInfo {
    pub name: String,
    pub visibility: SymbolVisibility,
    pub read_count: usize,
    pub write_count: usize,
    pub readers: HashSet<String>,
    pub writers: HashSet<String>,
    pub is_constant: bool,
}

/// Whole-program symbol table
#[derive(Clone)]
pub struct SymbolTable {
    functions: HashMap<String, FunctionInfo>,
    globals: HashMap<String, GlobalInfo>,
    call_graph: HashMap<String, HashSet<String>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        SymbolTable {
            functions: HashMap::new(),
            globals: HashMap::new(),
            call_graph: HashMap::new(),
        }
    }

    /// Register a function for analysis
    pub fn add_function(&mut self, info: FunctionInfo) {
        self.functions.insert(info.name.clone(), info);
    }

    /// Register a global variable
    pub fn add_global(&mut self, info: GlobalInfo) {
        self.globals.insert(info.name.clone(), info);
    }

    /// Record a call edge in the call graph
    pub fn add_call_edge(&mut self, caller: String, callee: String) {
        self.call_graph
            .entry(caller)
            .or_insert_with(HashSet::new)
            .insert(callee);
    }

    /// Get all functions called by a function
    pub fn get_callees(&self, func: &str) -> Option<&HashSet<String>> {
        self.call_graph.get(func)
    }

    /// Get all functions that call a function (reverse call graph)
    pub fn get_callers(&self, target: &str) -> HashSet<String> {
        self.call_graph
            .iter()
            .filter(|(_, callees)| callees.contains(target))
            .map(|(caller, _)| caller.clone())
            .collect()
    }
}

/// Dead code analyzer - identifies unreachable code
pub struct DeadCodeAnalyzer {
    symbol_table: SymbolTable,
}

impl DeadCodeAnalyzer {
    pub fn new(symbol_table: SymbolTable) -> Self {
        DeadCodeAnalyzer { symbol_table }
    }

    /// Find all reachable functions starting from entry points
    pub fn find_reachable_functions(&self, entry_points: &[&str]) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut worklist = entry_points.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        while let Some(func) = worklist.pop() {
            if reachable.contains(&func) {
                continue;
            }
            reachable.insert(func.clone());

            if let Some(callees) = self.symbol_table.get_callees(&func) {
                for callee in callees {
                    if !reachable.contains(callee) {
                        worklist.push(callee.clone());
                    }
                }
            }
        }

        reachable
    }

    /// Find dead functions (unreachable and not exported)
    pub fn find_dead_functions(&self, entry_points: &[&str]) -> Vec<String> {
        let reachable = self.find_reachable_functions(entry_points);
        
        self.symbol_table
            .functions
            .values()
            .filter(|func| {
                !reachable.contains(&func.name)
                    && func.visibility == SymbolVisibility::Internal
            })
            .map(|f| f.name.clone())
            .collect()
    }

    /// Find dead globals (unused and not exported)
    pub fn find_dead_globals(&self) -> Vec<String> {
        self.symbol_table
            .globals
            .values()
            .filter(|global| {
                global.read_count == 0
                    && global.write_count == 0
                    && global.visibility == SymbolVisibility::Internal
            })
            .map(|g| g.name.clone())
            .collect()
    }

    /// Find unused function parameters (requires intra-procedural analysis)
    pub fn find_read_only_globals(&self) -> Vec<String> {
        self.symbol_table
            .globals
            .values()
            .filter(|global| global.write_count == 0 && global.read_count > 0)
            .map(|g| g.name.clone())
            .collect()
    }
}

/// Inlining decision maker
pub struct InliningAnalyzer {
    symbol_table: SymbolTable,
    max_inline_size: usize,
}

impl InliningAnalyzer {
    pub fn new(symbol_table: SymbolTable, max_inline_size: usize) -> Self {
        InliningAnalyzer {
            symbol_table,
            max_inline_size,
        }
    }

    /// Determine if a function should be inlined
    pub fn should_inline(&self, func_name: &str) -> bool {
        if let Some(func) = self.symbol_table.functions.get(func_name) {
            // Don't inline recursive functions
            if func.is_recursive {
                return false;
            }
            // Inline small functions
            if func.instruction_count <= self.max_inline_size {
                return true;
            }
            // Inline functions called only once
            if func.call_count == 1 {
                return true;
            }
            // Inline hot functions (called many times, not too big)
            if func.call_count > 10 && func.instruction_count <= self.max_inline_size * 2 {
                return true;
            }
        }
        false
    }

    /// Find all functions that are good candidates for inlining
    pub fn find_inlining_candidates(&self) -> Vec<String> {
        self.symbol_table
            .functions
            .keys()
            .filter(|name| self.should_inline(name))
            .cloned()
            .collect()
    }

    /// Get inlining heuristic score (higher = better to inline)
    pub fn inlining_score(&self, func_name: &str) -> usize {
        if let Some(func) = self.symbol_table.functions.get(func_name) {
            let mut score: usize = 100;

            // Penalty for size
            score = score.saturating_sub(func.instruction_count);

            // Bonus for high call count
            score = score.saturating_add(func.call_count * 10);

            // Bonus for small functions
            if func.instruction_count < 10 {
                score = score.saturating_add(50);
            }

            // Penalty for recursion
            if func.is_recursive {
                score = 0;
            }

            score
        } else {
            0
        }
    }
}

/// Global constant propagation analyzer
pub struct ConstantPropagationAnalyzer {
    symbol_table: SymbolTable,
}

impl ConstantPropagationAnalyzer {
    pub fn new(symbol_table: SymbolTable) -> Self {
        ConstantPropagationAnalyzer { symbol_table }
    }

    /// Find globals that are written once and read multiple times
    pub fn find_effectively_constant_globals(&self) -> Vec<String> {
        self.symbol_table
            .globals
            .values()
            .filter(|global| {
                global.write_count == 1
                    && global.read_count > 0
                    && global.is_constant
            })
            .map(|g| g.name.clone())
            .collect()
    }

    /// Find globals that are written but never read
    pub fn find_write_only_globals(&self) -> Vec<String> {
        self.symbol_table
            .globals
            .values()
            .filter(|global| global.write_count > 0 && global.read_count == 0)
            .map(|g| g.name.clone())
            .collect()
    }
}

/// Code deduplication analyzer
pub struct DeduplicationAnalyzer {
    code_fingerprints: HashMap<String, Vec<String>>,
}

impl DeduplicationAnalyzer {
    pub fn new() -> Self {
        DeduplicationAnalyzer {
            code_fingerprints: HashMap::new(),
        }
    }

    /// Register a code sequence by its fingerprint
    pub fn register_code(&mut self, fingerprint: String, function: String) {
        self.code_fingerprints
            .entry(fingerprint)
            .or_insert_with(Vec::new)
            .push(function);
    }

    /// Find duplicate code sequences
    pub fn find_duplicates(&self) -> Vec<(String, Vec<String>)> {
        self.code_fingerprints
            .iter()
            .filter(|(_, funcs)| funcs.len() > 1)
            .map(|(fp, funcs)| (fp.clone(), funcs.clone()))
            .collect()
    }
}

/// Complete LTO optimizer
pub struct LinkTimeOptimizer {
    symbol_table: SymbolTable,
    dce: DeadCodeAnalyzer,
    inliner: InliningAnalyzer,
    const_prop: ConstantPropagationAnalyzer,
    dedup: DeduplicationAnalyzer,
}

impl LinkTimeOptimizer {
    pub fn new(symbol_table: SymbolTable) -> Self {
        let dce = DeadCodeAnalyzer::new(symbol_table.clone());
        let inliner = InliningAnalyzer::new(symbol_table.clone(), 50);
        let const_prop = ConstantPropagationAnalyzer::new(symbol_table.clone());
        let dedup = DeduplicationAnalyzer::new();

        LinkTimeOptimizer {
            symbol_table,
            dce,
            inliner,
            const_prop,
            dedup,
        }
    }

    /// Run all LTO passes
    pub fn optimize(&mut self, entry_points: &[&str]) -> LTOResult {
        let mut result = LTOResult::new();

        // Dead code elimination
        let dead_funcs = self.dce.find_dead_functions(entry_points);
        result.dead_functions = dead_funcs.len();

        let dead_globals = self.dce.find_dead_globals();
        result.dead_globals = dead_globals.len();

        // Inlining
        let inline_candidates = self.inliner.find_inlining_candidates();
        result.inlined_functions = inline_candidates.len();

        // Constant propagation
        let const_globals = self.const_prop.find_effectively_constant_globals();
        result.constants_identified = const_globals.len();

        // Deduplication
        let duplicates = self.dedup.find_duplicates();
        result.duplicate_sequences = duplicates.len();

        result
    }
}

/// Result of LTO optimization
#[derive(Debug, Clone)]
pub struct LTOResult {
    pub dead_functions: usize,
    pub dead_globals: usize,
    pub inlined_functions: usize,
    pub constants_identified: usize,
    pub duplicate_sequences: usize,
    pub estimated_size_reduction: usize,
}

impl LTOResult {
    pub fn new() -> Self {
        LTOResult {
            dead_functions: 0,
            dead_globals: 0,
            inlined_functions: 0,
            constants_identified: 0,
            duplicate_sequences: 0,
            estimated_size_reduction: 0,
        }
    }

    pub fn total_optimizations(&self) -> usize {
        self.dead_functions
            + self.dead_globals
            + self.inlined_functions
            + self.constants_identified
            + self.duplicate_sequences
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_table_creation() {
        let table = SymbolTable::new();
        assert_eq!(table.functions.len(), 0);
        assert_eq!(table.globals.len(), 0);
    }

    #[test]
    fn test_add_function() {
        let mut table = SymbolTable::new();
        let func = FunctionInfo {
            name: "test_func".to_string(),
            visibility: SymbolVisibility::Public,
            call_count: 5,
            call_sites: vec![],
            instruction_count: 100,
            is_recursive: false,
            uses_globals: vec![],
            module: "test".to_string(),
        };

        table.add_function(func);
        assert_eq!(table.functions.len(), 1);
        assert!(table.functions.contains_key("test_func"));
    }

    #[test]
    fn test_add_global() {
        let mut table = SymbolTable::new();
        let global = GlobalInfo {
            name: "global_var".to_string(),
            visibility: SymbolVisibility::Internal,
            read_count: 10,
            write_count: 1,
            readers: HashSet::new(),
            writers: HashSet::new(),
            is_constant: true,
        };

        table.add_global(global);
        assert_eq!(table.globals.len(), 1);
        assert!(table.globals.contains_key("global_var"));
    }

    #[test]
    fn test_call_graph_building() {
        let mut table = SymbolTable::new();
        table.add_call_edge("main".to_string(), "func_a".to_string());
        table.add_call_edge("main".to_string(), "func_b".to_string());
        table.add_call_edge("func_a".to_string(), "func_c".to_string());

        let main_callees = table.get_callees("main").unwrap();
        assert_eq!(main_callees.len(), 2);
        assert!(main_callees.contains("func_a"));
        assert!(main_callees.contains("func_b"));
    }

    #[test]
    fn test_reverse_call_graph() {
        let mut table = SymbolTable::new();
        table.add_call_edge("main".to_string(), "func_a".to_string());
        table.add_call_edge("func_b".to_string(), "func_a".to_string());

        let callers = table.get_callers("func_a");
        assert_eq!(callers.len(), 2);
        assert!(callers.contains("main"));
        assert!(callers.contains("func_b"));
    }

    #[test]
    fn test_find_reachable_functions() {
        let mut table = SymbolTable::new();
        table.add_call_edge("main".to_string(), "reachable1".to_string());
        table.add_call_edge("reachable1".to_string(), "reachable2".to_string());

        let analyzer = DeadCodeAnalyzer::new(table);
        let reachable = analyzer.find_reachable_functions(&["main"]);

        assert!(reachable.contains("main"));
        assert!(reachable.contains("reachable1"));
        assert!(reachable.contains("reachable2"));
    }

    #[test]
    fn test_find_dead_functions() {
        let mut table = SymbolTable::new();

        let public_func = FunctionInfo {
            name: "public_func".to_string(),
            visibility: SymbolVisibility::Public,
            call_count: 0,
            call_sites: vec![],
            instruction_count: 50,
            is_recursive: false,
            uses_globals: vec![],
            module: "test".to_string(),
        };

        let dead_func = FunctionInfo {
            name: "dead_func".to_string(),
            visibility: SymbolVisibility::Internal,
            call_count: 0,
            call_sites: vec![],
            instruction_count: 50,
            is_recursive: false,
            uses_globals: vec![],
            module: "test".to_string(),
        };

        table.add_function(public_func);
        table.add_function(dead_func);

        let analyzer = DeadCodeAnalyzer::new(table);
        let dead = analyzer.find_dead_functions(&["public_func"]);

        assert_eq!(dead.len(), 1);
        assert!(dead.contains(&"dead_func".to_string()));
    }

    #[test]
    fn test_find_dead_globals() {
        let mut table = SymbolTable::new();

        let used_global = GlobalInfo {
            name: "used".to_string(),
            visibility: SymbolVisibility::Internal,
            read_count: 5,
            write_count: 1,
            readers: HashSet::new(),
            writers: HashSet::new(),
            is_constant: true,
        };

        let unused_global = GlobalInfo {
            name: "unused".to_string(),
            visibility: SymbolVisibility::Internal,
            read_count: 0,
            write_count: 0,
            readers: HashSet::new(),
            writers: HashSet::new(),
            is_constant: false,
        };

        table.add_global(used_global);
        table.add_global(unused_global);

        let analyzer = DeadCodeAnalyzer::new(table);
        let dead = analyzer.find_dead_globals();

        assert_eq!(dead.len(), 1);
        assert!(dead.contains(&"unused".to_string()));
    }

    #[test]
    fn test_inlining_small_functions() {
        let mut table = SymbolTable::new();
        let small_func = FunctionInfo {
            name: "small".to_string(),
            visibility: SymbolVisibility::Internal,
            call_count: 3,
            call_sites: vec![],
            instruction_count: 20,
            is_recursive: false,
            uses_globals: vec![],
            module: "test".to_string(),
        };

        table.add_function(small_func);

        let analyzer = InliningAnalyzer::new(table, 50);
        assert!(analyzer.should_inline("small"));
    }

    #[test]
    fn test_inlining_single_call() {
        let mut table = SymbolTable::new();
        let func = FunctionInfo {
            name: "called_once".to_string(),
            visibility: SymbolVisibility::Internal,
            call_count: 1,
            call_sites: vec!["main".to_string()],
            instruction_count: 100,
            is_recursive: false,
            uses_globals: vec![],
            module: "test".to_string(),
        };

        table.add_function(func);

        let analyzer = InliningAnalyzer::new(table, 50);
        assert!(analyzer.should_inline("called_once"));
    }

    #[test]
    fn test_no_inline_recursive() {
        let mut table = SymbolTable::new();
        let func = FunctionInfo {
            name: "recursive".to_string(),
            visibility: SymbolVisibility::Internal,
            call_count: 5,
            call_sites: vec![],
            instruction_count: 30,
            is_recursive: true,
            uses_globals: vec![],
            module: "test".to_string(),
        };

        table.add_function(func);

        let analyzer = InliningAnalyzer::new(table, 50);
        assert!(!analyzer.should_inline("recursive"));
    }

    #[test]
    fn test_inlining_candidates() {
        let mut table = SymbolTable::new();

        let small = FunctionInfo {
            name: "small".to_string(),
            visibility: SymbolVisibility::Internal,
            call_count: 2,
            call_sites: vec![],
            instruction_count: 15,
            is_recursive: false,
            uses_globals: vec![],
            module: "test".to_string(),
        };

        let large = FunctionInfo {
            name: "large".to_string(),
            visibility: SymbolVisibility::Internal,
            call_count: 2,
            call_sites: vec![],
            instruction_count: 200,
            is_recursive: false,
            uses_globals: vec![],
            module: "test".to_string(),
        };

        table.add_function(small);
        table.add_function(large);

        let analyzer = InliningAnalyzer::new(table, 50);
        let candidates = analyzer.find_inlining_candidates();

        assert!(candidates.contains(&"small".to_string()));
        assert!(!candidates.contains(&"large".to_string()));
    }

    #[test]
    fn test_effectively_constant_globals() {
        let mut table = SymbolTable::new();

        let const_global = GlobalInfo {
            name: "const_val".to_string(),
            visibility: SymbolVisibility::Internal,
            read_count: 10,
            write_count: 1,
            readers: HashSet::new(),
            writers: HashSet::new(),
            is_constant: true,
        };

        let mutable_global = GlobalInfo {
            name: "mut_val".to_string(),
            visibility: SymbolVisibility::Internal,
            read_count: 5,
            write_count: 5,
            readers: HashSet::new(),
            writers: HashSet::new(),
            is_constant: false,
        };

        table.add_global(const_global);
        table.add_global(mutable_global);

        let analyzer = ConstantPropagationAnalyzer::new(table);
        let constants = analyzer.find_effectively_constant_globals();

        assert_eq!(constants.len(), 1);
        assert!(constants.contains(&"const_val".to_string()));
    }

    #[test]
    fn test_write_only_globals() {
        let mut table = SymbolTable::new();

        let write_only = GlobalInfo {
            name: "write_only".to_string(),
            visibility: SymbolVisibility::Internal,
            read_count: 0,
            write_count: 3,
            readers: HashSet::new(),
            writers: HashSet::new(),
            is_constant: false,
        };

        table.add_global(write_only);

        let analyzer = ConstantPropagationAnalyzer::new(table);
        let write_only_globals = analyzer.find_write_only_globals();

        assert_eq!(write_only_globals.len(), 1);
    }

    #[test]
    fn test_deduplication_analysis() {
        let mut dedup = DeduplicationAnalyzer::new();

        dedup.register_code("fingerprint_1".to_string(), "func_a".to_string());
        dedup.register_code("fingerprint_1".to_string(), "func_b".to_string());
        dedup.register_code("fingerprint_2".to_string(), "func_c".to_string());

        let duplicates = dedup.find_duplicates();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].1.len(), 2);
    }

    #[test]
    fn test_lto_result_totals() {
        let mut result = LTOResult::new();
        result.dead_functions = 5;
        result.dead_globals = 3;
        result.inlined_functions = 7;
        result.constants_identified = 10;
        result.duplicate_sequences = 2;

        assert_eq!(result.total_optimizations(), 27);
    }
}
