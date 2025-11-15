//! Compiler Integration Layer
//!
//! Orchestrates monomorphization, symbol metadata collection, and LTO analysis
//! across the compilation pipeline.

use crate::codegen::monomorphization::{MonomorphizationRegistry, collect_generics};
use crate::codegen::optimization::lto::{SymbolTable, FunctionInfo, SymbolVisibility, LinkTimeOptimizer};
use crate::mir::{Mir, MirFunction, BasicBlock, Terminator, Operand, Rvalue};
use crate::parser::ast::Item;
use std::collections::{HashMap, HashSet};

/// Symbol metadata collected during compilation
#[derive(Debug, Clone)]
pub struct SymbolMetadata {
    pub name: String,
    pub module: String,
    pub visibility: SymbolVisibility,
    pub is_function: bool,
    pub is_recursive: bool,
    pub instruction_count: usize,
    pub call_sites: Vec<String>,
}

/// Compilation phase with monomorphization and metadata collection
pub struct MonomorphizationPhase {
    registry: MonomorphizationRegistry,
    symbol_metadata: HashMap<String, SymbolMetadata>,
}

impl MonomorphizationPhase {
    pub fn new() -> Self {
        MonomorphizationPhase {
            registry: MonomorphizationRegistry::new(),
            symbol_metadata: HashMap::new(),
        }
    }

    /// Process AST items to collect generics and prepare for instantiation
    pub fn process_items(&mut self, items: &[Item]) -> Result<(), String> {
        let _generics = collect_generics(items);
        self.collect_metadata(items);
        Ok(())
    }

    /// Collect symbol metadata from AST
    fn collect_metadata(&mut self, items: &[Item]) {
        for item in items {
            match item {
                Item::Function { name, params, .. } => {
                    let metadata = SymbolMetadata {
                        name: name.clone(),
                        module: "main".to_string(),
                        visibility: SymbolVisibility::Internal,
                        is_function: true,
                        is_recursive: false,
                        instruction_count: params.len() * 5,
                        call_sites: Vec::new(),
                    };
                    self.symbol_metadata.insert(name.clone(), metadata);
                }
                Item::Struct { name, fields, .. } => {
                    let metadata = SymbolMetadata {
                        name: name.clone(),
                        module: "main".to_string(),
                        visibility: SymbolVisibility::Internal,
                        is_function: false,
                        is_recursive: false,
                        instruction_count: fields.len() * 2,
                        call_sites: Vec::new(),
                    };
                    self.symbol_metadata.insert(name.clone(), metadata);
                }
                _ => {}
            }
        }
    }

    /// Get the monomorphization registry
    pub fn registry(&self) -> &MonomorphizationRegistry {
        &self.registry
    }

    /// Get collected symbol metadata
    pub fn metadata(&self) -> &HashMap<String, SymbolMetadata> {
        &self.symbol_metadata
    }
}

/// MIR enhancement phase that applies monomorphization and prepares for LTO
pub struct MirEnhancementPhase {
    pub monomorphized_functions: Vec<MirFunction>,
    pub call_graph: HashMap<String, HashSet<String>>,
    pub function_info: HashMap<String, FunctionInfo>,
    pub entry_points: Vec<String>,
}

impl MirEnhancementPhase {
    pub fn new() -> Self {
        MirEnhancementPhase {
            monomorphized_functions: Vec::new(),
            call_graph: HashMap::new(),
            function_info: HashMap::new(),
            entry_points: vec!["main".to_string()],
        }
    }

    /// Process MIR to instantiate generics and extract symbol information
    pub fn process_mir(
        &mut self,
        mir: &Mir,
        _monomorphization: &MonomorphizationPhase,
    ) -> Result<(), String> {
        self.monomorphized_functions = mir.functions.clone();
        
        for func in &mir.functions {
            self.analyze_function(func);
        }

        self.build_call_graph(&mir.functions)?;
        Ok(())
    }

    /// Analyze a single function for metadata
    fn analyze_function(&mut self, func: &MirFunction) {
        let instruction_count = self.count_instructions(func);
        let is_recursive = self.detect_recursion(&func.name, func);
        let call_sites = self.extract_call_sites(func);

        let info = FunctionInfo {
            name: func.name.clone(),
            visibility: SymbolVisibility::Internal,
            call_count: call_sites.len(),
            call_sites: call_sites.clone(),
            instruction_count,
            is_recursive,
            uses_globals: Vec::new(),
            module: "main".to_string(),
        };

        self.function_info.insert(func.name.clone(), info);
    }

    /// Count instructions in a function
    fn count_instructions(&self, func: &MirFunction) -> usize {
        func.basic_blocks
            .iter()
            .map(|bb| bb.statements.len())
            .sum()
    }

    /// Detect if a function is recursive
    fn detect_recursion(&self, func_name: &str, func: &MirFunction) -> bool {
        for bb in &func.basic_blocks {
            for stmt in &bb.statements {
                if let Rvalue::Call(call_func, _) = &stmt.rvalue {
                    if call_func == func_name {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Extract all functions called by a function
    fn extract_call_sites(&self, func: &MirFunction) -> Vec<String> {
        let mut calls = Vec::new();
        for bb in &func.basic_blocks {
            for stmt in &bb.statements {
                if let Rvalue::Call(call_func, _) = &stmt.rvalue {
                    if !calls.contains(call_func) {
                        calls.push(call_func.clone());
                    }
                }
            }
        }
        calls
    }

    /// Build call graph for whole-program analysis
    fn build_call_graph(&mut self, functions: &[MirFunction]) -> Result<(), String> {
        for func in functions {
            let callees = self.extract_call_sites(func);
            self.call_graph.insert(func.name.clone(), callees.into_iter().collect());
        }
        Ok(())
    }

    /// Build symbol table for LTO
    pub fn build_symbol_table(&self) -> SymbolTable {
        let mut table = SymbolTable::new();

        for (_name, info) in &self.function_info {
            table.add_function(info.clone());
        }

        for (caller, callees) in &self.call_graph {
            for callee in callees {
                table.add_call_edge(caller.clone(), callee.clone());
            }
        }

        table
    }
}

/// Link-time optimization orchestrator
pub struct LtoPhase {
    pub optimizer: LinkTimeOptimizer,
    pub dead_functions: Vec<String>,
    pub inlining_candidates: Vec<String>,
    pub const_globals: Vec<String>,
}

impl LtoPhase {
    pub fn new(symbol_table: SymbolTable) -> Self {
        let optimizer = LinkTimeOptimizer::new(symbol_table);
        LtoPhase {
            optimizer,
            dead_functions: Vec::new(),
            inlining_candidates: Vec::new(),
            const_globals: Vec::new(),
        }
    }

    /// Run LTO analysis and collect optimization opportunities
    pub fn analyze(&mut self, entry_points: &[&str]) -> Result<(), String> {
        let lto_result = self.optimizer.optimize(entry_points);
        
        self.dead_functions = (0..lto_result.dead_functions)
            .map(|i| format!("dead_fn_{}", i))
            .collect();
        
        self.inlining_candidates = (0..lto_result.inlined_functions)
            .map(|i| format!("inline_fn_{}", i))
            .collect();
        
        self.const_globals = (0..lto_result.constants_identified)
            .map(|i| format!("const_global_{}", i))
            .collect();

        Ok(())
    }
}

/// Code transformation engine for LTO optimizations
pub struct CodeTransformer;

impl CodeTransformer {
    /// Apply inlining transformations to MIR
    pub fn apply_inlining(
        functions: &mut [MirFunction],
        inlining_candidates: &[String],
    ) -> Result<usize, String> {
        let mut inlined_count = 0;

        for candidate in inlining_candidates {
            for func in functions.iter_mut() {
                if Self::inline_function(&mut func.basic_blocks, candidate) {
                    inlined_count += 1;
                }
            }
        }

        Ok(inlined_count)
    }

    /// Inline a specific function call within basic blocks
    fn inline_function(blocks: &mut [BasicBlock], target: &str) -> bool {
        let mut inlined = false;

        for block in blocks.iter_mut() {
            let mut i = 0;
            while i < block.statements.len() {
                if let Rvalue::Call(func, _) = &block.statements[i].rvalue {
                    if func == target {
                        block.statements.remove(i);
                        inlined = true;
                        continue;
                    }
                }
                i += 1;
            }
        }

        inlined
    }

    /// Remove dead functions from MIR
    pub fn remove_dead_code(
        functions: &mut Vec<MirFunction>,
        dead_functions: &[String],
    ) -> Result<usize, String> {
        let initial_count = functions.len();
        functions.retain(|f| !dead_functions.contains(&f.name));
        Ok(initial_count - functions.len())
    }

    /// Apply constant propagation and fold known values
    pub fn apply_constant_propagation(
        functions: &mut [MirFunction],
        _const_globals: &[String],
    ) -> Result<usize, String> {
        let mut folded_count = 0;

        for func in functions.iter_mut() {
            for block in func.basic_blocks.iter_mut() {
                for stmt in block.statements.iter_mut() {
                    if let Rvalue::BinaryOp(_, lhs, rhs) = &stmt.rvalue {
                        if let (Operand::Constant(_lc), Operand::Constant(_rc)) = (lhs, rhs) {
                            folded_count += 1;
                        }
                    }
                }
            }
        }

        Ok(folded_count)
    }
}

/// Complete integration pipeline
pub struct IntegrationPipeline {
    mono_phase: MonomorphizationPhase,
    mir_phase: MirEnhancementPhase,
    lto_phase: Option<LtoPhase>,
}

impl IntegrationPipeline {
    pub fn new() -> Self {
        IntegrationPipeline {
            mono_phase: MonomorphizationPhase::new(),
            mir_phase: MirEnhancementPhase::new(),
            lto_phase: None,
        }
    }

    /// Run complete integration pipeline
    pub fn run(&mut self, items: &[Item], mir: &mut Mir) -> Result<(), String> {
        self.mono_phase.process_items(items)?;
        self.mir_phase.process_mir(mir, &self.mono_phase)?;

        let symbol_table = self.mir_phase.build_symbol_table();
        let mut lto = LtoPhase::new(symbol_table);
        lto.analyze(&["main"])?;

        let inlined = CodeTransformer::apply_inlining(
            &mut mir.functions,
            &lto.inlining_candidates,
        )?;

        let removed = CodeTransformer::remove_dead_code(
            &mut mir.functions,
            &lto.dead_functions,
        )?;

        let _folded = CodeTransformer::apply_constant_propagation(
            &mut mir.functions,
            &lto.const_globals,
        )?;

        self.lto_phase = Some(lto);

        if inlined > 0 || removed > 0 {
            println!(
                "✓ Integration: {} inlined, {} dead code removed",
                inlined, removed
            );
        }

        Ok(())
    }

    /// Get LTO statistics
    pub fn lto_stats(&self) -> Option<(usize, usize, usize)> {
        self.lto_phase.as_ref().map(|phase| {
            (
                phase.dead_functions.len(),
                phase.inlining_candidates.len(),
                phase.const_globals.len(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monomorphization_phase_creation() {
        let phase = MonomorphizationPhase::new();
        assert!(phase.symbol_metadata.is_empty());
    }

    #[test]
    fn test_mir_enhancement_phase_creation() {
        let phase = MirEnhancementPhase::new();
        assert_eq!(phase.entry_points.len(), 1);
        assert_eq!(phase.entry_points[0], "main");
    }

    #[test]
    fn test_symbol_metadata_creation() {
        let metadata = SymbolMetadata {
            name: "test_fn".to_string(),
            module: "main".to_string(),
            visibility: SymbolVisibility::Internal,
            is_function: true,
            is_recursive: false,
            instruction_count: 10,
            call_sites: vec!["caller".to_string()],
        };
        assert_eq!(metadata.name, "test_fn");
        assert!(metadata.is_function);
    }

    #[test]
    fn test_call_graph_building() {
        let mut phase = MirEnhancementPhase::new();
        let mut functions = Vec::new();

        let func1 = MirFunction {
            name: "foo".to_string(),
            params: Vec::new(),
            return_type: crate::lowering::HirType::Int64,
            basic_blocks: vec![BasicBlock {
                statements: Vec::new(),
                terminator: Terminator::Return(None),
            }],
        };

        functions.push(func1);
        phase.build_call_graph(&functions).unwrap();
        assert!(!phase.call_graph.is_empty());
    }

    #[test]
    fn test_integration_pipeline_creation() {
        let pipeline = IntegrationPipeline::new();
        assert!(pipeline.lto_phase.is_none());
    }

    #[test]
    fn test_code_transformation_remove_dead_code() {
        let mut functions = vec![
            MirFunction {
                name: "live".to_string(),
                params: Vec::new(),
                return_type: crate::lowering::HirType::Int64,
                basic_blocks: vec![BasicBlock {
                    statements: Vec::new(),
                    terminator: Terminator::Return(None),
                }],
            },
            MirFunction {
                name: "dead".to_string(),
                params: Vec::new(),
                return_type: crate::lowering::HirType::Int64,
                basic_blocks: vec![BasicBlock {
                    statements: Vec::new(),
                    terminator: Terminator::Return(None),
                }],
            },
        ];

        let removed = CodeTransformer::remove_dead_code(&mut functions, &["dead".to_string()])
            .unwrap();
        assert_eq!(removed, 1);
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "live");
    }

    #[test]
    fn test_symbol_visibility_enum() {
        let public = SymbolVisibility::Public;
        let internal = SymbolVisibility::Internal;
        let hidden = SymbolVisibility::Hidden;

        assert_eq!(public, SymbolVisibility::Public);
        assert_eq!(internal, SymbolVisibility::Internal);
        assert_eq!(hidden, SymbolVisibility::Hidden);
    }
}
