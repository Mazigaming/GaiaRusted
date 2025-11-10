//! Linker Integration with LTO
//!
//! Provides LTO-aware linking, symbol table management, and whole-program
//! optimization during the linking phase.

use crate::codegen::optimization::lto::{SymbolTable, LinkTimeOptimizer, SymbolVisibility};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Metadata for a symbol (function or global)
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub module: String,
    pub visibility: SymbolVisibility,
    pub is_function: bool,
    pub address: Option<usize>,
    pub size: usize,
    pub references: Vec<String>,
}

/// LTO-aware linker
pub struct LtoLinker {
    symbol_table: SymbolTable,
    optimizer: LinkTimeOptimizer,
    symbols: HashMap<String, SymbolInfo>,
    entry_point: String,
    output_dir: PathBuf,
}

impl LtoLinker {
    pub fn new<P: AsRef<Path>>(output_dir: P) -> Self {
        LtoLinker {
            symbol_table: SymbolTable::new(),
            optimizer: LinkTimeOptimizer::new(SymbolTable::new()),
            symbols: HashMap::new(),
            entry_point: "main".to_string(),
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    /// Register a symbol for linking
    pub fn add_symbol(&mut self, info: SymbolInfo) {
        self.symbols.insert(info.name.clone(), info);
    }

    /// Set the entry point function
    pub fn set_entry_point(&mut self, name: String) {
        self.entry_point = name;
    }

    /// Perform reachability analysis from entry points
    pub fn analyze_reachability(&self, entry_points: &[&str]) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut work_queue: Vec<_> = entry_points.to_vec();

        while let Some(func) = work_queue.pop() {
            if reachable.insert(func.to_string()) {
                if let Some(callees) = self.symbol_table.get_callees(func) {
                    for callee in callees {
                        if !reachable.contains(callee) {
                            work_queue.push(callee);
                        }
                    }
                }
            }
        }

        reachable
    }

    /// Identify symbols to keep during linking
    pub fn compute_kept_symbols(&self) -> HashSet<String> {
        let entry_points = [self.entry_point.as_str()];
        let reachable = self.analyze_reachability(&entry_points);

        let mut kept = HashSet::new();

        for (name, _info) in &self.symbols {
            if reachable.contains(name) {
                kept.insert(name.clone());
            } else if let Some(sym) = self.symbols.get(name) {
                if sym.visibility == SymbolVisibility::Public {
                    kept.insert(name.clone());
                }
            }
        }

        kept
    }

    /// Compute symbols to strip (eliminate)
    pub fn compute_stripped_symbols(&self) -> Vec<String> {
        let kept = self.compute_kept_symbols();
        let mut stripped = Vec::new();

        for name in self.symbols.keys() {
            if !kept.contains(name) {
                stripped.push(name.clone());
            }
        }

        stripped
    }

    /// Build symbol address map for layout
    pub fn compute_symbol_layout(&self) -> HashMap<String, (usize, usize)> {
        let mut layout = HashMap::new();
        let kept_symbols = self.compute_kept_symbols();
        let mut current_address = 0x1000;

        let mut sorted_symbols: Vec<_> = kept_symbols.iter().cloned().collect();
        sorted_symbols.sort();

        for name in sorted_symbols {
            if let Some(sym) = self.symbols.get(&name) {
                layout.insert(name.clone(), (current_address, sym.size));
                current_address += sym.size;
            }
        }

        layout
    }

    /// Compute statistics about the linked binary
    pub fn compute_statistics(&self) -> LinkingStatistics {
        let kept = self.compute_kept_symbols();
        let stripped = self.compute_stripped_symbols();
        let layout = self.compute_symbol_layout();

        let total_code_size: usize = kept
            .iter()
            .filter_map(|name| self.symbols.get(name).map(|s| s.size))
            .sum();

        let removed_code_size: usize = stripped
            .iter()
            .filter_map(|name| self.symbols.get(name).map(|s| s.size))
            .sum();

        LinkingStatistics {
            kept_symbols: kept.len(),
            removed_symbols: stripped.len(),
            total_code_size,
            removed_code_size,
            estimated_savings_percent: if total_code_size > 0 {
                (removed_code_size * 100) / (total_code_size + removed_code_size)
            } else {
                0
            },
            layout_entries: layout.len(),
        }
    }

    /// Generate a symbol visibility map for linker
    pub fn generate_visibility_map(&self) -> HashMap<String, SymbolVisibility> {
        self.symbols
            .iter()
            .map(|(name, info)| (name.clone(), info.visibility))
            .collect()
    }

    /// Verify symbol references for validity
    pub fn verify_references(&self) -> Result<(), String> {
        for (name, info) in &self.symbols {
            for referenced in &info.references {
                if !self.symbols.contains_key(referenced) {
                    return Err(format!(
                        "Symbol {} references undefined symbol {}",
                        name, referenced
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Statistics about the linking process
#[derive(Debug, Clone)]
pub struct LinkingStatistics {
    pub kept_symbols: usize,
    pub removed_symbols: usize,
    pub total_code_size: usize,
    pub removed_code_size: usize,
    pub estimated_savings_percent: usize,
    pub layout_entries: usize,
}

impl LinkingStatistics {
    pub fn new() -> Self {
        LinkingStatistics {
            kept_symbols: 0,
            removed_symbols: 0,
            total_code_size: 0,
            removed_code_size: 0,
            estimated_savings_percent: 0,
            layout_entries: 0,
        }
    }
}

/// Symbol graph for whole-program analysis
#[derive(Debug, Clone)]
pub struct SymbolGraph {
    pub nodes: HashMap<String, SymbolInfo>,
    pub edges: HashMap<String, HashSet<String>>,
}

impl SymbolGraph {
    pub fn new() -> Self {
        SymbolGraph {
            nodes: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    /// Add a symbol node
    pub fn add_node(&mut self, info: SymbolInfo) {
        self.nodes.insert(info.name.clone(), info);
    }

    /// Add an edge (call) between symbols
    pub fn add_edge(&mut self, from: String, to: String) {
        self.edges.entry(from).or_insert_with(HashSet::new).insert(to);
    }

    /// Compute strongly connected components (cycles)
    pub fn find_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node_name in self.nodes.keys() {
            if !visited.contains(node_name) {
                self.dfs_cycles(node_name, &mut visited, &mut rec_stack, &mut cycles);
            }
        }

        cycles
    }

    fn dfs_cycles(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = self.edges.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.dfs_cycles(neighbor, visited, rec_stack, cycles);
                } else if rec_stack.contains(neighbor) {
                    cycles.push(vec![node.to_string(), neighbor.clone()]);
                }
            }
        }

        rec_stack.remove(node);
    }

    /// Compute call frequency for each symbol
    pub fn compute_call_frequency(&self) -> HashMap<String, usize> {
        let mut frequency = HashMap::new();

        for (caller, callees) in &self.edges {
            *frequency.entry(caller.clone()).or_insert(0) += 1;

            for callee in callees {
                *frequency.entry(callee.clone()).or_insert(0) += 1;
            }
        }

        frequency
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lto_linker_creation() {
        let linker = LtoLinker::new("/tmp");
        assert_eq!(linker.entry_point, "main");
    }

    #[test]
    fn test_add_symbol() {
        let mut linker = LtoLinker::new("/tmp");
        let sym = SymbolInfo {
            name: "test_fn".to_string(),
            module: "main".to_string(),
            visibility: SymbolVisibility::Public,
            is_function: true,
            address: None,
            size: 100,
            references: vec![],
        };
        linker.add_symbol(sym);
        assert_eq!(linker.symbols.len(), 1);
    }

    #[test]
    fn test_set_entry_point() {
        let mut linker = LtoLinker::new("/tmp");
        linker.set_entry_point("custom_main".to_string());
        assert_eq!(linker.entry_point, "custom_main");
    }

    #[test]
    fn test_compute_kept_symbols() {
        let mut linker = LtoLinker::new("/tmp");

        linker.add_symbol(SymbolInfo {
            name: "main".to_string(),
            module: "main".to_string(),
            visibility: SymbolVisibility::Public,
            is_function: true,
            address: None,
            size: 100,
            references: vec!["helper".to_string()],
        });

        linker.add_symbol(SymbolInfo {
            name: "helper".to_string(),
            module: "main".to_string(),
            visibility: SymbolVisibility::Public,
            is_function: true,
            address: None,
            size: 50,
            references: vec![],
        });

        linker.add_symbol(SymbolInfo {
            name: "dead".to_string(),
            module: "main".to_string(),
            visibility: SymbolVisibility::Internal,
            is_function: true,
            address: None,
            size: 30,
            references: vec![],
        });

        let kept = linker.compute_kept_symbols();
        assert!(kept.contains("main"));
        assert!(kept.contains("helper"));
        assert!(!kept.contains("dead"));
    }

    #[test]
    fn test_compute_stripped_symbols() {
        let mut linker = LtoLinker::new("/tmp");

        linker.add_symbol(SymbolInfo {
            name: "main".to_string(),
            module: "main".to_string(),
            visibility: SymbolVisibility::Public,
            is_function: true,
            address: None,
            size: 100,
            references: vec![],
        });

        linker.add_symbol(SymbolInfo {
            name: "dead".to_string(),
            module: "main".to_string(),
            visibility: SymbolVisibility::Internal,
            is_function: true,
            address: None,
            size: 30,
            references: vec![],
        });

        let stripped = linker.compute_stripped_symbols();
        assert!(stripped.contains(&"dead".to_string()));
    }

    #[test]
    fn test_symbol_layout() {
        let mut linker = LtoLinker::new("/tmp");

        linker.add_symbol(SymbolInfo {
            name: "main".to_string(),
            module: "main".to_string(),
            visibility: SymbolVisibility::Public,
            is_function: true,
            address: None,
            size: 100,
            references: vec![],
        });

        let layout = linker.compute_symbol_layout();
        assert!(!layout.is_empty());
    }

    #[test]
    fn test_linking_statistics() {
        let mut linker = LtoLinker::new("/tmp");

        linker.add_symbol(SymbolInfo {
            name: "main".to_string(),
            module: "main".to_string(),
            visibility: SymbolVisibility::Public,
            is_function: true,
            address: None,
            size: 100,
            references: vec![],
        });

        let stats = linker.compute_statistics();
        assert_eq!(stats.kept_symbols, 1);
    }

    #[test]
    fn test_symbol_graph_creation() {
        let mut graph = SymbolGraph::new();
        let sym = SymbolInfo {
            name: "test".to_string(),
            module: "main".to_string(),
            visibility: SymbolVisibility::Public,
            is_function: true,
            address: None,
            size: 100,
            references: vec![],
        };
        graph.add_node(sym);
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn test_symbol_graph_edges() {
        let mut graph = SymbolGraph::new();
        graph.add_edge("caller".to_string(), "callee".to_string());
        assert!(!graph.edges.is_empty());
    }

    #[test]
    fn test_verify_references_valid() {
        let mut linker = LtoLinker::new("/tmp");

        linker.add_symbol(SymbolInfo {
            name: "foo".to_string(),
            module: "main".to_string(),
            visibility: SymbolVisibility::Public,
            is_function: true,
            address: None,
            size: 100,
            references: vec!["bar".to_string()],
        });

        linker.add_symbol(SymbolInfo {
            name: "bar".to_string(),
            module: "main".to_string(),
            visibility: SymbolVisibility::Public,
            is_function: true,
            address: None,
            size: 100,
            references: vec![],
        });

        assert!(linker.verify_references().is_ok());
    }

    #[test]
    fn test_call_frequency_computation() {
        let mut graph = SymbolGraph::new();
        graph.add_edge("main".to_string(), "helper".to_string());
        graph.add_edge("main".to_string(), "other".to_string());

        let freq = graph.compute_call_frequency();
        assert_eq!(freq.get("main"), Some(&1));
        assert_eq!(freq.get("helper"), Some(&1));
        assert_eq!(freq.get("other"), Some(&1));
    }
}
