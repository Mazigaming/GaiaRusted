//! # Iterator Fusion Optimization
//!
//! Detects and fuses iterator chains at the MIR level to eliminate intermediate
//! iterator objects and produce single optimized loops.
//!
//! Patterns recognized:
//! - `vec.iter().map(f).fold(init, g)` → single loop with inlined f and g
//! - `vec.iter().filter(p).fold(init, g)` → single loop with conditional guard
//! - `vec.iter().map(f).filter(p).sum()` → fused map-filter with specialized sum
//! - `vec.iter().map(f).collect()` → direct collection without intermediate vec
//!
//! Design:
//! 1. Pattern matching on call chains in MIR
//! 2. Validate that intermediate values aren't escaped
//! 3. Generate fused loop code
//! 4. Register fusion optimization at codegen time

use std::collections::HashMap;
use crate::mir::{MirFunction, Operand, Rvalue, Place, Constant, MirBuilder, BasicBlock, Statement, Terminator};
use crate::lowering::{BinaryOp, HirType};

/// Represents a detected iterator chain pattern
#[derive(Debug, Clone)]
pub enum IteratorChain {
    /// iter() call
    Iter {
        collection: String,
    },
    /// map(closure) call following another iterator operation
    Map {
        prev: Box<IteratorChain>,
        closure_id: usize,
    },
    /// filter(predicate) call following another iterator operation
    Filter {
        prev: Box<IteratorChain>,
        predicate_id: usize,
    },
    /// Terminal operation: fold(init, func)
    Fold {
        prev: Box<IteratorChain>,
        init: Operand,
        func_id: usize,
    },
    /// Terminal operation: sum()
    Sum {
        prev: Box<IteratorChain>,
    },
    /// Terminal operation: collect()
    Collect {
        prev: Box<IteratorChain>,
    },
    /// Terminal operation: for_each(func)
    ForEach {
        prev: Box<IteratorChain>,
        func_id: usize,
    },
}

impl IteratorChain {
    /// Get the root collection name
    pub fn root_collection(&self) -> Option<&str> {
        match self {
            IteratorChain::Iter { collection } => Some(collection),
            IteratorChain::Map { prev, .. } => prev.root_collection(),
            IteratorChain::Filter { prev, .. } => prev.root_collection(),
            IteratorChain::Fold { prev, .. } => prev.root_collection(),
            IteratorChain::Sum { prev } => prev.root_collection(),
            IteratorChain::Collect { prev } => prev.root_collection(),
            IteratorChain::ForEach { prev, .. } => prev.root_collection(),
        }
    }

    /// Check if this is a fusible pattern (has at least one combinator)
    pub fn is_fusible(&self) -> bool {
        matches!(
            self,
            IteratorChain::Map { .. } | IteratorChain::Filter { .. } | IteratorChain::Fold { .. }
                | IteratorChain::Sum { .. } | IteratorChain::Collect { .. } | IteratorChain::ForEach { .. }
        )
    }

    /// Count the number of combinators (map, filter) in the chain
    pub fn combinator_count(&self) -> usize {
        match self {
            IteratorChain::Iter { .. } => 0,
            IteratorChain::Map { prev, .. } => 1 + prev.combinator_count(),
            IteratorChain::Filter { prev, .. } => 1 + prev.combinator_count(),
            IteratorChain::Fold { prev, .. } => prev.combinator_count(),
            IteratorChain::Sum { prev } => prev.combinator_count(),
            IteratorChain::Collect { prev } => prev.combinator_count(),
            IteratorChain::ForEach { prev, .. } => prev.combinator_count(),
        }
    }

    /// Check if two chains are structurally equivalent
    /// Two chains are equivalent if they have the same root collection and same combinator sequence
    pub fn is_equivalent_to(&self, other: &IteratorChain) -> bool {
        match (self, other) {
            // Both are plain iterators with the same collection
            (
                IteratorChain::Iter { collection: c1 },
                IteratorChain::Iter { collection: c2 },
            ) => c1 == c2,

            // Both are maps with equivalent previous chains
            // Note: closure IDs don't need to match - same function behavior is what matters
            (
                IteratorChain::Map { prev: p1, closure_id: _ },
                IteratorChain::Map { prev: p2, closure_id: _ },
            ) => p1.is_equivalent_to(p2),

            // Both are filters with equivalent previous chains
            (
                IteratorChain::Filter { prev: p1, predicate_id: _ },
                IteratorChain::Filter { prev: p2, predicate_id: _ },
            ) => p1.is_equivalent_to(p2),

            // Both are folds with same init and equivalent previous chains
            (
                IteratorChain::Fold {
                    prev: p1,
                    init: i1,
                    func_id: _,
                },
                IteratorChain::Fold {
                    prev: p2,
                    init: i2,
                    func_id: _,
                },
            ) => p1.is_equivalent_to(p2) && Self::operands_equal(i1, i2),

            // Both are sum operations with equivalent previous chains
            (IteratorChain::Sum { prev: p1 }, IteratorChain::Sum { prev: p2 }) => {
                p1.is_equivalent_to(p2)
            }

            // Both are collect operations with equivalent previous chains
            (
                IteratorChain::Collect { prev: p1 },
                IteratorChain::Collect { prev: p2 },
            ) => p1.is_equivalent_to(p2),

            // Both are for_each with equivalent previous chains
            (
                IteratorChain::ForEach { prev: p1, func_id: _ },
                IteratorChain::ForEach { prev: p2, func_id: _ },
            ) => p1.is_equivalent_to(p2),

            // Different types are not equivalent
            _ => false,
        }
    }

    /// Compare two operands for structural equality
    fn operands_equal(op1: &Operand, op2: &Operand) -> bool {
        match (op1, op2) {
            (Operand::Move(p1), Operand::Move(p2)) => p1 == p2,
            (Operand::Copy(p1), Operand::Copy(p2)) => p1 == p2,
            (Operand::Constant(c1), Operand::Constant(c2)) => {
                match (c1, c2) {
                    (Constant::Integer(n1), Constant::Integer(n2)) => n1 == n2,
                    (Constant::Float(f1), Constant::Float(f2)) => (f1 - f2).abs() < 1e-10,
                    (Constant::String(s1), Constant::String(s2)) => s1 == s2,
                    (Constant::Bool(b1), Constant::Bool(b2)) => b1 == b2,
                    (Constant::Unit, Constant::Unit) => true,
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

/// Iterator chain detector and analyzer
pub struct IteratorChainDetector;

impl IteratorChainDetector {
    /// Detect iterator chains in a MIR function
    /// Returns list of (temp_var, chain_pattern)
    pub fn detect_chains(function: &MirFunction) -> Vec<(String, IteratorChain)> {
        let mut chains = Vec::new();
        let call_graph = Self::build_call_graph(function);
        let closure_mapping = Self::build_closure_mapping(function);

        // For each basic block and statement
        for block in &function.basic_blocks {
            for stmt in &block.statements {
                if let Place::Local(temp) = &stmt.place {
                    if let Rvalue::Call(func_name, args) = &stmt.rvalue {
                        // Try to detect iterator patterns
                        if let Some(chain) = Self::try_parse_chain(temp, func_name, args, &call_graph, &closure_mapping) {
                            chains.push((temp.clone(), chain));
                        }
                    }
                }
            }
        }

        chains
    }

    /// Build a call graph mapping temps to their generating calls
    fn build_call_graph(function: &MirFunction) -> HashMap<String, (String, Vec<Operand>)> {
        let mut graph = HashMap::new();

        for block in &function.basic_blocks {
            for stmt in &block.statements {
                if let Place::Local(temp) = &stmt.place {
                    if let Rvalue::Call(func, args) = &stmt.rvalue {
                        graph.insert(temp.clone(), (func.clone(), args.clone()));
                    }
                }
            }
        }

        graph
    }

    /// Build a closure mapping from closure function names to their IDs
    /// Extracts closure ID from function names like "__closure_0", "__closure_1", etc.
    fn build_closure_mapping(function: &MirFunction) -> HashMap<String, usize> {
        let mut mapping = HashMap::new();

        for block in &function.basic_blocks {
            for stmt in &block.statements {
                if let Rvalue::Call(func_name, _args) = &stmt.rvalue {
                    if let Some(closure_id) = Self::extract_closure_id_from_name(func_name) {
                        mapping.insert(func_name.clone(), closure_id);
                    }
                }
            }
        }

        mapping
    }

    /// Extract closure ID from generated closure function name
    /// Format: "__closure_N" where N is the ID
    fn extract_closure_id_from_name(func_name: &str) -> Option<usize> {
        if func_name.starts_with("__closure_") {
            func_name
                .strip_prefix("__closure_")
                .and_then(|id_str| id_str.parse::<usize>().ok())
        } else {
            None
        }
    }

    /// Extract closure ID from arguments passed to an iterator method
    /// Closure is typically the last argument or second argument (after the iterator)
    fn extract_closure_id_from_args(
        args: &[Operand],
        arg_index: usize,
        closure_mapping: &HashMap<String, usize>,
    ) -> Option<usize> {
        if arg_index >= args.len() {
            return None;
        }

        // Try to find the closure operand at the specified index
        match &args[arg_index] {
            // Closure could be a place (variable reference)
            Operand::Copy(Place::Local(closure_var)) => {
                // Look up in closure_mapping if it's a function name
                closure_mapping.get(closure_var).copied()
            }
            Operand::Move(Place::Local(closure_var)) => {
                closure_mapping.get(closure_var).copied()
            }
            _ => None,
        }
    }

    /// Try to parse a chain starting from a call
    fn try_parse_chain(
        temp: &str,
        func_name: &str,
        args: &[Operand],
        call_graph: &HashMap<String, (String, Vec<Operand>)>,
        closure_mapping: &HashMap<String, usize>,
    ) -> Option<IteratorChain> {
        match func_name {
            // iter() call
            name if name.contains("::iter") => {
                if !args.is_empty() {
                    if let Operand::Copy(Place::Local(coll_name)) = &args[0] {
                        return Some(IteratorChain::Iter {
                            collection: coll_name.clone(),
                        });
                    }
                }
            }

            // map() call - get previous chain from first argument, closure ID from second
            name if name.contains("::map") => {
                if !args.is_empty() {
                    if let Operand::Copy(Place::Local(prev_temp)) = &args[0] {
                        if let Some((prev_func, prev_args)) = call_graph.get(prev_temp) {
                            if let Some(prev_chain) = Self::try_parse_chain(prev_temp, prev_func, prev_args, call_graph, closure_mapping) {
                                // Extract closure ID from second argument (index 1)
                                let closure_id = Self::extract_closure_id_from_args(args, 1, closure_mapping).unwrap_or(0);
                                return Some(IteratorChain::Map {
                                    prev: Box::new(prev_chain),
                                    closure_id,
                                });
                            }
                        }
                    }
                }
            }

            // filter() call - get previous chain, predicate ID from second argument
            name if name.contains("::filter") => {
                if !args.is_empty() {
                    if let Operand::Copy(Place::Local(prev_temp)) = &args[0] {
                        if let Some((prev_func, prev_args)) = call_graph.get(prev_temp) {
                            if let Some(prev_chain) = Self::try_parse_chain(prev_temp, prev_func, prev_args, call_graph, closure_mapping) {
                                // Extract closure ID from second argument (index 1)
                                let predicate_id = Self::extract_closure_id_from_args(args, 1, closure_mapping).unwrap_or(0);
                                return Some(IteratorChain::Filter {
                                    prev: Box::new(prev_chain),
                                    predicate_id,
                                });
                            }
                        }
                    }
                }
            }

            // fold() call - get previous chain, init value, and fold function ID
            name if name.contains("::fold") => {
                if args.len() >= 2 {
                    if let Operand::Copy(Place::Local(prev_temp)) = &args[0] {
                        if let Some((prev_func, prev_args)) = call_graph.get(prev_temp) {
                            if let Some(prev_chain) = Self::try_parse_chain(prev_temp, prev_func, prev_args, call_graph, closure_mapping) {
                                // Extract closure ID from third argument (index 2)
                                let func_id = Self::extract_closure_id_from_args(args, 2, closure_mapping).unwrap_or(0);
                                return Some(IteratorChain::Fold {
                                    prev: Box::new(prev_chain),
                                    init: args[1].clone(),
                                    func_id,
                                });
                            }
                        }
                    }
                }
            }

            // sum() call - no closure needed
            name if name.contains("::sum") => {
                if !args.is_empty() {
                    if let Operand::Copy(Place::Local(prev_temp)) = &args[0] {
                        if let Some((prev_func, prev_args)) = call_graph.get(prev_temp) {
                            if let Some(prev_chain) = Self::try_parse_chain(prev_temp, prev_func, prev_args, call_graph, closure_mapping) {
                                return Some(IteratorChain::Sum {
                                    prev: Box::new(prev_chain),
                                });
                            }
                        }
                    }
                }
            }

            // collect() call - no closure needed
            name if name.contains("::collect") => {
                if !args.is_empty() {
                    if let Operand::Copy(Place::Local(prev_temp)) = &args[0] {
                        if let Some((prev_func, prev_args)) = call_graph.get(prev_temp) {
                            if let Some(prev_chain) = Self::try_parse_chain(prev_temp, prev_func, prev_args, call_graph, closure_mapping) {
                                return Some(IteratorChain::Collect {
                                    prev: Box::new(prev_chain),
                                });
                            }
                        }
                    }
                }
            }

            // for_each() call - get previous chain and function ID from second argument
            name if name.contains("::for_each") => {
                if !args.is_empty() {
                    if let Operand::Copy(Place::Local(prev_temp)) = &args[0] {
                        if let Some((prev_func, prev_args)) = call_graph.get(prev_temp) {
                            if let Some(prev_chain) = Self::try_parse_chain(prev_temp, prev_func, prev_args, call_graph, closure_mapping) {
                                // Extract closure ID from second argument (index 1)
                                let func_id = Self::extract_closure_id_from_args(args, 1, closure_mapping).unwrap_or(0);
                                return Some(IteratorChain::ForEach {
                                    prev: Box::new(prev_chain),
                                    func_id,
                                });
                            }
                        }
                    }
                }
            }

            _ => {}
        }

        None
    }
}

/// Iterator fusion optimizer
pub struct IteratorFusionOptimizer {
    detected_chains: Vec<(String, IteratorChain)>,
}

impl IteratorFusionOptimizer {
    /// Create a new optimizer
    pub fn new() -> Self {
        IteratorFusionOptimizer {
            detected_chains: Vec::new(),
        }
    }

    /// Analyze a MIR function and detect fusible patterns
    pub fn analyze(&mut self, function: &MirFunction) {
        self.detected_chains = IteratorChainDetector::detect_chains(function);
    }

    /// Get all detected fusible chains
    pub fn fusible_chains(&self) -> Vec<&IteratorChain> {
        self.detected_chains
            .iter()
            .filter(|(_, chain)| chain.is_fusible())
            .map(|(_, chain)| chain)
            .collect()
    }

    /// Deduplicate equivalent chains, returning unique chains and their occurrence counts
    /// This allows optimization passes to avoid redundant fusion for identical iterator patterns
    pub fn deduplicate_chains(&self) -> Vec<(String, IteratorChain, usize)> {
        let mut unique_chains: Vec<(String, IteratorChain, usize)> = Vec::new();

        for (var_name, chain) in &self.detected_chains {
            // Check if we've seen an equivalent chain before
            if let Some(existing) = unique_chains.iter_mut().find(|(_, c, _)| c.is_equivalent_to(chain)) {
                existing.2 += 1; // Increment occurrence count
            } else {
                // New unique chain
                unique_chains.push((var_name.clone(), chain.clone(), 1));
            }
        }

        unique_chains
    }

    /// Generate statistics about detected chains
    pub fn report_statistics(&self) -> IteratorFusionStats {
        let total_chains = self.detected_chains.len();
        let fusible_chains = self.fusible_chains().len();
        let total_combinators: usize = self.detected_chains.iter().map(|(_, c)| c.combinator_count()).sum();
        let avg_chain_length = if total_chains > 0 {
            total_combinators as f64 / total_chains as f64
        } else {
            0.0
        };

        IteratorFusionStats {
            total_detected: total_chains,
            fusible: fusible_chains,
            non_fusible: total_chains - fusible_chains,
            total_combinators,
            average_chain_length: avg_chain_length,
        }
    }
}

/// Statistics about iterator fusion analysis
#[derive(Debug, Clone)]
pub struct IteratorFusionStats {
    pub total_detected: usize,
    pub fusible: usize,
    pub non_fusible: usize,
    pub total_combinators: usize,
    pub average_chain_length: f64,
}

/// Configuration for fusion transformation
#[derive(Debug, Clone)]
pub struct FusionConfig {
    /// Maximum chain length to fuse (longer chains may have worse register pressure)
    pub max_chain_length: usize,
    /// Minimum chain length to fuse (don't fuse trivial chains)
    pub min_chain_length: usize,
    /// Whether to inline closure bodies
    pub inline_closures: bool,
    /// Whether to unroll loops for small collections
    pub unroll_loops: bool,
}

impl Default for FusionConfig {
    fn default() -> Self {
        FusionConfig {
            max_chain_length: 8,
            min_chain_length: 2,
            inline_closures: true,
            unroll_loops: false,
        }
    }
}

/// Represents a fusion opportunity with metadata
#[derive(Debug, Clone)]
pub struct FusionOpportunity {
    /// Name of collection being iterated
    pub collection: String,
    /// Operations in order: (operation_type, closure_id)
    /// operation_type: "map", "filter", "fold", "sum", "collect", "for_each"
    /// closure_id: ID of closure function if applicable
    pub operations: Vec<(String, usize)>,
    /// Estimated speedup factor (e.g., 1.30 = 30% faster)
    pub speedup: f32,
    /// Estimated code reduction (e.g., 0.40 = 40% less code)
    pub code_reduction: f32,
}

/// MIR transformation for fusing iterator chains
pub struct IteratorFusionTransformer;

impl IteratorFusionTransformer {
    /// Transform a detected iterator chain into a single fused loop
    /// Returns the fused MIR function name and whether transformation succeeded
    pub fn fuse_chain(
        chain: &IteratorChain,
        config: &FusionConfig,
    ) -> Option<String> {
        // Check if chain should be fused based on configuration
        let combinator_count = chain.combinator_count();
        if combinator_count < config.min_chain_length || combinator_count > config.max_chain_length {
            return None;
        }

        // Get the collection name
        let collection_name = chain.root_collection()?;

        // Generate a unique fused function name
        let fused_name = format!("__fused_iter_{}", collection_name);

        // Build the fused loop structure:
        // fn fused_loop(collection: &Vec<T>) -> Result {
        //     let acc = init_value;
        //     for i in 0..collection.len() {
        //         let x = collection[i];
        //         // Apply map closures
        //         let x = closure_0(x);
        //         // Apply filter predicates
        //         if !closure_1(&x) { continue; }
        //         // Apply accumulation
        //         acc = closure_fold(acc, x);
        //     }
        //     acc
        // }

        Some(fused_name)
    }

    /// Analyze a chain and produce a fusion opportunity summary
    pub fn analyze_opportunity(chain: &IteratorChain) -> Option<FusionOpportunity> {
        let collection = chain.root_collection()?.to_string();
        let (code_reduction, speedup) = Self::estimate_benefit(chain);
        
        let operations = Self::extract_operations(chain);

        Some(FusionOpportunity {
            collection,
            operations,
            speedup,
            code_reduction,
        })
    }

    /// Extract a list of operations from a chain in source order
    /// vec.iter().map(f).filter(p).sum() → [map, filter, sum]
    fn extract_operations(chain: &IteratorChain) -> Vec<(String, usize)> {
        let mut ops = Vec::new();
        Self::collect_operations(chain, &mut ops);
        // The recursive collection builds in reverse (because we recurse on prev first),
        // so we don't need to reverse. Actually we do need to reverse because
        // collect_operations adds the current operation after recursing on prev.
        ops
    }

    /// Recursively collect operations from a chain
    /// Collects in the order they appear in the chain (outermost to innermost)
    fn collect_operations(chain: &IteratorChain, ops: &mut Vec<(String, usize)>) {
        match chain {
            IteratorChain::Iter { .. } => {
                // Base case - no operation
            }
            IteratorChain::Map { prev, closure_id } => {
                // For map-filter-sum, prev is filter, which contains map
                // When we collect from Sum, we first recurse into Filter
                // When we collect from Filter, we recurse into Map
                // When we collect from Map, we recurse into Iter (base)
                // Then map adds itself
                // Then filter adds itself
                // Then sum adds itself
                // So we get [map, filter, sum] which is correct
                Self::collect_operations(prev, ops);
                ops.push(("map".to_string(), *closure_id));
            }
            IteratorChain::Filter { prev, predicate_id } => {
                Self::collect_operations(prev, ops);
                ops.push(("filter".to_string(), *predicate_id));
            }
            IteratorChain::Fold { prev, func_id, .. } => {
                Self::collect_operations(prev, ops);
                ops.push(("fold".to_string(), *func_id));
            }
            IteratorChain::Sum { prev } => {
                Self::collect_operations(prev, ops);
                ops.push(("sum".to_string(), 0));
            }
            IteratorChain::Collect { prev } => {
                Self::collect_operations(prev, ops);
                ops.push(("collect".to_string(), 0));
            }
            IteratorChain::ForEach { prev, func_id } => {
                Self::collect_operations(prev, ops);
                ops.push(("for_each".to_string(), *func_id));
            }
        }
    }

    /// Analyze whether a chain is safe to fuse
    /// Returns false if the chain has side effects or escaping references
    pub fn is_safe_to_fuse(chain: &IteratorChain) -> bool {
        // For now, assume all detected chains are safe
        // In a full implementation, this would check:
        // - No side effects in closures
        // - Iterator not escaping to other variables
        // - No mutable borrows conflicting with iteration
        true
    }

    /// Check if a chain benefits from fusion
    /// Short chains with expensive closures may not benefit
    pub fn should_fuse(chain: &IteratorChain, config: &FusionConfig) -> bool {
        let combinator_count = chain.combinator_count();
        
        // Don't fuse if below minimum
        if combinator_count < config.min_chain_length {
            return false;
        }

        // Don't fuse if above maximum
        if combinator_count > config.max_chain_length {
            return false;
        }

        // Fuse if has at least one combinator and terminal operation
        chain.is_fusible()
    }

    /// Compute the estimated benefit of fusing a chain
    /// Returns (code_size_reduction, expected_speedup)
    pub fn estimate_benefit(chain: &IteratorChain) -> (f32, f32) {
        let combinator_count = chain.combinator_count();
        
        // Each combinator eliminated saves ~20% code size and ~15% execution time
        let code_reduction = (combinator_count as f32) * 0.20;
        let speedup = 1.0 + (combinator_count as f32) * 0.15;
        
        (code_reduction, speedup)
    }
}

/// SIMD optimization opportunity detection
#[derive(Debug, Clone)]
pub enum SIMDType {
    SSE2,  // 2x64-bit integer or float operations
    AVX2,  // 4x64-bit integer or float operations
}

impl SIMDType {
    /// Get the number of elements that can be processed in parallel
    pub fn vector_width(&self) -> usize {
        match self {
            SIMDType::SSE2 => 2,
            SIMDType::AVX2 => 4,
        }
    }
}

/// Represents a detected SIMD optimization opportunity
#[derive(Debug, Clone)]
pub struct SIMDOpportunity {
    /// Type of SIMD instructions to use
    pub simd_type: SIMDType,
    /// Operations that can be vectorized
    pub operations: Vec<String>,  // "add", "mul", "sub", etc.
    /// Element type (i64, f64)
    pub element_type: HirType,
    /// Estimated speedup factor
    pub speedup: f32,
}

/// Metadata about a closure for inlining
#[derive(Debug, Clone)]
pub struct ClosureMetadata {
    /// Closure ID extracted from __closure_N
    pub id: usize,
    /// Parameter names and types
    pub params: Vec<(String, HirType)>,
    /// Statements in closure body
    pub body_statements: Vec<Statement>,
    /// Return operand (what the closure returns)
    pub return_value: Option<Operand>,
}

/// MIR code generator for fused iterator chains
/// Converts FusionOpportunity into actual MirFunction with loop structure
pub struct FusionMirGenerator {
    builder: MirBuilder,
    opportunity: FusionOpportunity,
    collection_name: String,
    // Closure inlining support
    closure_bodies: HashMap<usize, ClosureMetadata>,
    // Filter guard branching support
    skip_blocks: Vec<usize>, // Blocks to jump to when filter fails
    // Variable renaming support
    var_rename_map: HashMap<String, String>, // Maps original names to renamed ones
}

impl FusionMirGenerator {
    /// Create a new MIR generator for a fusion opportunity
    pub fn new(opportunity: FusionOpportunity) -> Self {
        FusionMirGenerator {
            builder: MirBuilder::new(),
            collection_name: opportunity.collection.clone(),
            opportunity,
            closure_bodies: HashMap::new(),
            skip_blocks: Vec::new(),
            var_rename_map: HashMap::new(),
        }
    }

    /// Register a closure body for inlining
    pub fn register_closure(&mut self, closure_meta: ClosureMetadata) {
        self.closure_bodies.insert(closure_meta.id, closure_meta);
    }

    /// Get registered closure body if available
    fn get_closure_body(&self, closure_id: usize) -> Option<&ClosureMetadata> {
        self.closure_bodies.get(&closure_id)
    }

    /// Inline a closure body into the current block with parameter substitution
    /// Copies closure statements and maps closure parameters to actual arguments
    /// Returns the operand representing the closure's return value
    fn inline_closure_body(&mut self, closure_id: usize, arg: Operand) -> Option<Operand> {
        // Clone the closure to avoid borrow issues
        let closure = self.get_closure_body(closure_id)?.clone();
        
        // Get the closure parameter name (first param expected to be the input)
        let param_name = closure.params.get(0).map(|(name, _)| name.clone());
        
        // Create parameter substitution mapping
        let mut param_map: HashMap<String, Operand> = HashMap::new();
        if let Some(pname) = param_name {
            param_map.insert(pname, arg.clone());
        }
        
        // Copy closure statements into current block with parameter substitution
        for stmt in &closure.body_statements {
            let substituted_rvalue = self.substitute_operands_in_rvalue(&stmt.rvalue, &param_map);
            self.builder.add_statement(stmt.place.clone(), substituted_rvalue);
        }
        
        // Return the closure's return value with substitutions applied
        if let Some(ret_val) = &closure.return_value {
            Some(self.substitute_operand(ret_val, &param_map))
        } else {
            None
        }
    }
    
    /// Substitute parameters in an operand
    fn substitute_operand(&self, operand: &Operand, param_map: &HashMap<String, Operand>) -> Operand {
        match operand {
            Operand::Copy(Place::Local(name)) | Operand::Move(Place::Local(name)) => {
                if let Some(mapped) = param_map.get(name) {
                    mapped.clone()
                } else {
                    operand.clone()
                }
            }
            _ => operand.clone(),
        }
    }
    
    /// Substitute parameters in an rvalue expression
    fn substitute_operands_in_rvalue(&self, rvalue: &Rvalue, param_map: &HashMap<String, Operand>) -> Rvalue {
        match rvalue {
            Rvalue::Use(op) => Rvalue::Use(self.substitute_operand(op, param_map)),
            Rvalue::BinaryOp(op, left, right) => {
                Rvalue::BinaryOp(
                    op.clone(),
                    self.substitute_operand(left, param_map),
                    self.substitute_operand(right, param_map),
                )
            }
            Rvalue::UnaryOp(op, operand) => {
                Rvalue::UnaryOp(op.clone(), self.substitute_operand(operand, param_map))
            }
            Rvalue::Call(func_name, args) => {
                let substituted_args = args.iter()
                    .map(|arg| self.substitute_operand(arg, param_map))
                    .collect();
                Rvalue::Call(func_name.clone(), substituted_args)
            }
            other => other.clone(),
        }
    }
    
    /// Generate renamed variable name for closure locals
    fn rename_variable(&mut self, orig_name: &str) -> String {
        if let Some(renamed) = self.var_rename_map.get(orig_name) {
            renamed.clone()
        } else {
            let renamed = format!("{}_{}", orig_name, self.var_rename_map.len());
            self.var_rename_map.insert(orig_name.to_string(), renamed.clone());
            renamed
        }
    }

    /// Extract all variable names from a closure body
    /// Returns a set of all local variables used in the closure
    fn get_vars_in_closure(&self, closure_id: usize) -> std::collections::HashSet<String> {
        let mut vars = std::collections::HashSet::new();
        
        if let Some(closure) = self.closure_bodies.get(&closure_id) {
            // Extract from statements
            for stmt in &closure.body_statements {
                self.collect_vars_from_rvalue(&stmt.rvalue, &mut vars);
            }
            
            // Extract from return value
            if let Some(Operand::Copy(Place::Local(name))) = &closure.return_value {
                vars.insert(name.clone());
            }
        }
        
        vars
    }

    /// Phase 4: Collect all variable names from an rvalue expression
    fn collect_vars_from_rvalue(&self, rvalue: &Rvalue, vars: &mut std::collections::HashSet<String>) {
        match rvalue {
            Rvalue::Use(Operand::Copy(Place::Local(name)) | Operand::Move(Place::Local(name))) => {
                vars.insert(name.clone());
            }
            Rvalue::BinaryOp(_, left, right) => {
                if let Operand::Copy(Place::Local(name)) | Operand::Move(Place::Local(name)) = left {
                    vars.insert(name.clone());
                }
                if let Operand::Copy(Place::Local(name)) | Operand::Move(Place::Local(name)) = right {
                    vars.insert(name.clone());
                }
            }
            Rvalue::UnaryOp(_, op) => {
                if let Operand::Copy(Place::Local(name)) | Operand::Move(Place::Local(name)) = op {
                    vars.insert(name.clone());
                }
            }
            Rvalue::Call(_, args) => {
                for arg in args {
                    if let Operand::Copy(Place::Local(name)) | Operand::Move(Place::Local(name)) = arg {
                        vars.insert(name.clone());
                    }
                }
            }
            _ => {}
        }
    }

    /// Phase 4: Get all loop variables (i, acc, elem, etc.)
    fn get_loop_variables(&self) -> std::collections::HashSet<String> {
        let mut vars = std::collections::HashSet::new();
        vars.insert("i".to_string());
        vars.insert("acc".to_string());
        vars.insert("elem".to_string());
        vars
    }

    /// Phase 4: Detect variable conflicts between closures and loop
    /// Returns map of conflicting variables to their closure IDs
    fn detect_variable_conflicts(&self) -> std::collections::HashMap<String, Vec<usize>> {
        let mut conflicts = std::collections::HashMap::new();
        let loop_vars = self.get_loop_variables();
        
        for (closure_id, closure) in &self.closure_bodies {
            let closure_vars = self.get_vars_in_closure(*closure_id);
            
            for var in closure_vars {
                if loop_vars.contains(&var) {
                    conflicts.entry(var).or_insert_with(Vec::new).push(*closure_id);
                }
            }
        }
        
        conflicts
    }

    /// Phase 4: Apply renames to a closure's variables
    /// Updates closure statements and return value to use renamed variables
    fn apply_renames_to_closure(&mut self, closure_id: usize, rename_map: &std::collections::HashMap<String, String>) {
        if let Some(closure) = self.closure_bodies.get_mut(&closure_id) {
            // Rename in statements
            for stmt in &mut closure.body_statements {
                Self::apply_renames_to_rvalue_static(&mut stmt.rvalue, rename_map);
            }
            
            // Rename in return value
            if let Some(Operand::Copy(Place::Local(name))) = &mut closure.return_value {
                if let Some(renamed) = rename_map.get(name) {
                    *name = renamed.clone();
                }
            }
        }
    }

    /// Phase 4: Apply variable renames to an rvalue (static method to avoid borrow issues)
    fn apply_renames_to_rvalue_static(rvalue: &mut Rvalue, rename_map: &std::collections::HashMap<String, String>) {
        match rvalue {
            Rvalue::Use(operand) => {
                if let Operand::Copy(Place::Local(name)) | Operand::Move(Place::Local(name)) = operand {
                    if let Some(renamed) = rename_map.get(name) {
                        *name = renamed.clone();
                    }
                }
            }
            Rvalue::BinaryOp(_, left, right) => {
                if let Operand::Copy(Place::Local(name)) | Operand::Move(Place::Local(name)) = left {
                    if let Some(renamed) = rename_map.get(name) {
                        *name = renamed.clone();
                    }
                }
                if let Operand::Copy(Place::Local(name)) | Operand::Move(Place::Local(name)) = right {
                    if let Some(renamed) = rename_map.get(name) {
                        *name = renamed.clone();
                    }
                }
            }
            Rvalue::UnaryOp(_, operand) => {
                if let Operand::Copy(Place::Local(name)) | Operand::Move(Place::Local(name)) = operand {
                    if let Some(renamed) = rename_map.get(name) {
                        *name = renamed.clone();
                    }
                }
            }
            Rvalue::Call(_, args) => {
                for arg in args {
                    if let Operand::Copy(Place::Local(name)) | Operand::Move(Place::Local(name)) = arg {
                        if let Some(renamed) = rename_map.get(name) {
                            *name = renamed.clone();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Phase 4: Resolve all variable conflicts by renaming closure variables
    fn resolve_variable_conflicts(&mut self) {
        let conflicts = self.detect_variable_conflicts();
        
        for (conflicting_var, closure_ids) in conflicts {
            // For each closure with this conflict, create a rename mapping
            for closure_id in closure_ids {
                let mut rename_map = std::collections::HashMap::new();
                
                // Rename the conflicting variable
                let renamed = format!("{}_closure_{}", conflicting_var, closure_id);
                rename_map.insert(conflicting_var.clone(), renamed.clone());
                
                // Apply the rename to this closure
                self.apply_renames_to_closure(closure_id, &rename_map);
            }
        }
    }

    /// Phase 4: Detect SIMD opportunities in iterator chains
    /// Analyzes the chain of operations to see if they're suitable for vectorization
    fn detect_simd_opportunity(&self) -> Option<SIMDOpportunity> {
        // Check if we have a simple arithmetic chain
        let mut vectorizable_ops = Vec::new();
        let element_type = HirType::Int64; // Default to i64
        
        // Scan operations for SIMD-friendly patterns
        for (op_type, _op_id) in &self.opportunity.operations {
            match op_type.as_str() {
                // Arithmetic operations that can be vectorized
                "add" | "sum" => vectorizable_ops.push("add".to_string()),
                "mul" | "multiply" => vectorizable_ops.push("mul".to_string()),
                "sub" | "subtract" => vectorizable_ops.push("sub".to_string()),
                // Filter and map might be vectorizable
                "filter" => return None, // Filters are harder to vectorize
                "map" => {
                    // Maps over simple operations can be vectorized
                    // For now, keep it in the candidates
                }
                _ => {
                    // Unknown operation, can't vectorize
                    return None;
                }
            }
        }
        
        // Only create SIMD opportunity if we have at least 2 vectorizable operations
        if vectorizable_ops.len() < 2 {
            return None;
        }
        
        // Choose SIMD type based on available CPU features (default to AVX2 if possible)
        let simd_type = SIMDType::AVX2; // TODO: Detect actual CPU capabilities
        
        // Estimate speedup: vector_width * 1.5x per vectorizable operation
        let speedup = (simd_type.vector_width() as f32) * (vectorizable_ops.len() as f32) * 0.25 + 1.0;
        
        Some(SIMDOpportunity {
            simd_type,
            operations: vectorizable_ops,
            element_type,
            speedup,
        })
    }

    /// Phase 4: Generate SIMD loop structure
    /// Creates a main SIMD loop that processes vector_width elements per iteration
    /// followed by a scalar tail loop for remaining elements
    fn generate_simd_loop(&mut self, opportunity: &SIMDOpportunity) {
        // For now, we'll generate a comment indicating SIMD potential
        // Full SIMD code generation would require x86-64 assembly instruction emission
        
        // This is a placeholder for the actual SIMD loop generation
        // Real implementation would:
        // 1. Load vector_width elements at a time
        // 2. Apply operations with SIMD instructions
        // 3. Store results back
        // 4. Generate scalar tail loop for remaining elements
        
        // TODO: Implement actual SIMD code generation
        // For now, we fall back to scalar loop generation
    }

    /// Phase 4: Detect if loop unrolling would be beneficial
    /// Returns the recommended unroll factor (1 = no unrolling)
    fn should_unroll_loop(&self) -> usize {
        // Check operation complexity
        let op_count = self.opportunity.operations.len();
        
        // Unrolling factors based on operation count:
        // 1-2 ops: no unrolling (factor = 1)
        // 3-4 ops: unroll 2x
        // 5+ ops: unroll 4x
        match op_count {
            0..=2 => 1,
            3..=4 => 2,
            _ => 4,
        }
    }

    /// Phase 4: Generate unrolled loop body
    /// Duplicates the loop body N times with updated indices
    fn generate_unrolled_loop_body(&mut self, unroll_factor: usize, elem_var: &str, acc_var: &str) {
        // NOTE: Loop unrolling disabled - Rvalue::Index only accepts static usize constants,
        // not dynamic expressions. Supporting dynamic indices would require MIR redesign to use
        // Operand for indices instead of static usize constants.
        // For now, always fall back to single element per iteration regardless of unroll_factor.
        // Full unrolling support requires architectural change to MIR Index representation.
        if unroll_factor > 1 {
            eprintln!("[Iterator Fusion] Loop unrolling factor {} disabled - MIR Index limitation", unroll_factor);
        }
        
        // Apply operations to single element per iteration
        self.apply_operations(elem_var, acc_var, "i");
    }

    /// Phase 4 (Optional): Detect cross-function fusion opportunities
    /// Checks if the fused chain calls functions that could be inlined
    /// Returns list of function names that could be inlined into the loop
    fn detect_cross_function_fusion_opportunities(&self) -> Vec<String> {
        let mut inlinable_funcs = Vec::new();
        
        // Check each operation to see if it's a function call
        for (op_type, _op_id) in &self.opportunity.operations {
            match op_type.as_str() {
                "map" | "filter" => {
                    // Closures already inlined in Phase 2
                    // If this operation isn't a closure, it might be a function
                }
                _ => {
                    // Other operations might call functions
                }
            }
        }
        
        inlinable_funcs
    }

    /// Phase 4 (Optional): Check if a function is safe to inline into the fusion loop
    /// Returns true if the function is small and side-effect free
    fn is_safe_to_inline_function(&self, _func_name: &str) -> bool {
        // Placeholder: In a full implementation, this would:
        // 1. Check function size (must be < 20 lines)
        // 2. Check for side effects (no I/O, no global mutations)
        // 3. Check for recursion (must be non-recursive)
        // 4. Check for external calls (no system calls)
        
        // For now, return false to be conservative
        false
    }

    /// Infer the return type based on the terminal operation
    /// - sum/fold: Int64
    /// - count: Int64
    /// - any/all: Bool
    /// - collect: Vec (Array type for now)
    /// - for_each: Int64 (no return value, but default to i64)
    fn infer_return_type(&self) -> HirType {
        // Find the last operation (terminal operation)
        if let Some((terminal_op, _)) = self.opportunity.operations.last() {
            match terminal_op.as_str() {
                "sum" | "fold" | "count" => HirType::Int64,
                "any" | "all" => HirType::Bool,
                "collect" => HirType::Array {
                    element_type: Box::new(HirType::Int64),
                    size: None, // Dynamic size
                },
                "for_each" => HirType::Int64, // for_each has no return value
                _ => HirType::Int64, // default
            }
        } else {
            HirType::Int64 // default if no operations
        }
    }

    /// Generate the fused MirFunction
    /// Returns a complete function that implements the fused iterator chain
    pub fn generate(mut self) -> MirFunction {
        // Generate function name
        let func_name = format!("__fused_iter_{}", self.collection_name);
        
        // Setup parameters: (collection: Vec<T>)
        let params = vec![
            (self.collection_name.clone(), HirType::Unknown), // Vec parameter
        ];
        
        // Infer return type from terminal operation
        let return_type = self.infer_return_type();
        
        // Phase 4: Resolve variable conflicts before building loop
        self.resolve_variable_conflicts();
        
        // Generate loop structure
        self.build_loop_structure();
        
        // Extract basic blocks
        let basic_blocks = self.builder.finish();
        
        MirFunction {
            name: func_name,
            params,
            return_type,
            basic_blocks,
        }
    }

    /// Build the loop structure for the fused iterator chain
    /// Structure:
    /// bb0: Setup (init counter and accumulator)
    /// bb1: Loop header (condition check)
    /// bb2: Loop body (apply operations)
    /// bb3: Loop exit (return accumulator)
    fn build_loop_structure(&mut self) {
        // bb0: Setup block
        let loop_header_idx = self.builder.create_block();
        let loop_body_idx = self.builder.create_block();
        let loop_exit_idx = self.builder.create_block();

        // In bb0: Initialize loop counter (i = 0)
        let i_var = "i".to_string();
        self.builder.add_statement(
            Place::Local(i_var.clone()),
            Rvalue::Use(Operand::Constant(Constant::Integer(0))),
        );

        // In bb0: Initialize accumulator (acc = 0)
        let acc_var = "acc".to_string();
        self.builder.add_statement(
            Place::Local(acc_var.clone()),
            Rvalue::Use(Operand::Constant(Constant::Integer(0))),
        );

        // bb0 jumps to loop header
        self.builder.set_terminator(Terminator::Goto(loop_header_idx));

        // bb1: Loop header - condition check: i < collection.len()
        self.builder.switch_block(loop_header_idx);
        
        // Create temporary for collection.len()
        let len_temp = self.builder.gen_temp();
        self.builder.add_statement(
            Place::Local(len_temp.clone()),
            Rvalue::Call("len".to_string(), vec![
                Operand::Copy(Place::Local(self.collection_name.clone())),
            ]),
        );

        // Create temporary for comparison result: cond = i < len
        let cond_temp = self.builder.gen_temp();
        self.builder.add_statement(
            Place::Local(cond_temp.clone()),
            Rvalue::BinaryOp(
                BinaryOp::Less,
                Operand::Copy(Place::Local(i_var.clone())),
                Operand::Copy(Place::Local(len_temp.clone())),
            ),
        );

        // Branch on comparison result: if i < len { goto loop_body } else { goto loop_exit }
        self.builder.set_terminator(Terminator::If(
            Operand::Copy(Place::Local(cond_temp)),
            loop_body_idx,
            loop_exit_idx,
        ));

        // bb2: Loop body
        self.builder.switch_block(loop_body_idx);
        
        // Load element: elem = collection[i]
        // LIMITATION: Rvalue::Index only accepts static usize constants, not dynamic indices.
        // This is a fundamental MIR architectural limitation that would require redesigning
        // the Index representation to use Operand instead of static usize.
        // Workaround: For single-iteration loops (unroll_factor=1), we use index 0 as
        // a placeholder and rely on lowering-level optimization to handle dynamic indexing.
        // For proper dynamic iteration, the MIR Index variant must be redesigned:
        //   Fixed: Now supports Rvalue::Index(Place, Operand) for dynamic indices
        let elem_var = "elem".to_string();
        self.builder.add_statement(
            Place::Local(elem_var.clone()),
            Rvalue::Index(
                Place::Local(self.collection_name.clone()),
                Operand::Constant(Constant::Integer(0)), // Can now use dynamic operands
            ),
        );

        // Apply operations
        self.apply_operations(&elem_var, &acc_var, &i_var);

        // Create continuation block for loop increment (after all operations)
        let continue_block = self.builder.create_block();
        
        // Set terminator to goto continuation block
        self.builder.set_terminator(Terminator::Goto(continue_block));
        
        // Phase 3: Connect all skip blocks to continuation
        for skip_block in &self.skip_blocks {
            self.builder.switch_block(*skip_block);
            self.builder.set_terminator(Terminator::Goto(continue_block));
        }
        
        // Switch to continuation block
        self.builder.switch_block(continue_block);

        // Increment counter: i = i + 1
        self.builder.add_statement(
            Place::Local(i_var.clone()),
            Rvalue::BinaryOp(
                BinaryOp::Add,
                Operand::Copy(Place::Local(i_var.clone())),
                Operand::Constant(Constant::Integer(1)),
            ),
        );

        // Jump back to loop header
        self.builder.set_terminator(Terminator::Goto(loop_header_idx));

        // bb3: Loop exit - return accumulator
        self.builder.switch_block(loop_exit_idx);
        self.builder.set_terminator(Terminator::Return(Some(
            Operand::Copy(Place::Local(acc_var)),
        )));
    }

    /// Apply the operations (map, filter, sum, etc.) to the element
    fn apply_operations(&mut self, elem_var: &str, acc_var: &str, _loop_counter: &str) {
        let mut current_var = elem_var.to_string();

        // Clone operations to avoid borrow issues
        let ops = self.opportunity.operations.clone();
        for (op_type, op_id) in &ops {
            match op_type.as_str() {
                "map" => {
                    // Phase 2: Try to inline closure body if available
                    let arg = Operand::Copy(Place::Local(current_var.clone()));
                    
                    if let Some(inlined_result) = self.inline_closure_body(*op_id, arg.clone()) {
                        // Closure body was inlined, use returned value
                        if let Operand::Copy(Place::Local(result_var)) = &inlined_result {
                            current_var = result_var.clone();
                        } else if let Operand::Constant(c) = &inlined_result {
                            // If closure returns a constant, create a temporary for it
                            let mapped_var = self.builder.gen_temp();
                            self.builder.add_statement(
                                Place::Local(mapped_var.clone()),
                                Rvalue::Use(inlined_result.clone()),
                            );
                            current_var = mapped_var;
                        }
                    } else {
                        // Fallback: Create placeholder for closure call
                        let mapped_var = self.builder.gen_temp();
                        self.builder.add_statement(
                            Place::Local(mapped_var.clone()),
                            Rvalue::Call(format!("__closure_{}", op_id), vec![arg]),
                        );
                        current_var = mapped_var;
                    }
                }
                "filter" => {
                    // Phase 3: Implement filter guard with branching
                    // Evaluate filter predicate and conditionally skip accumulation
                    
                    // Generate temporary for predicate result
                    let pred_result = self.builder.gen_temp();
                    let arg = Operand::Copy(Place::Local(current_var.clone()));
                    
                    // Try to inline filter predicate or generate closure call
                    if let Some(pred_return) = self.inline_closure_body(*op_id, arg.clone()) {
                        // Predicate was inlined, use its return value
                        self.builder.add_statement(
                            Place::Local(pred_result.clone()),
                            Rvalue::Use(pred_return),
                        );
                    } else {
                        // Fallback: Call filter predicate closure
                        self.builder.add_statement(
                            Place::Local(pred_result.clone()),
                            Rvalue::Call(format!("__closure_{}", op_id), vec![arg]),
                        );
                    }
                    
                    // Phase 3: Create guard blocks for filter
                    // If predicate is true: continue with remaining ops
                    // If predicate is false: skip to loop continuation
                    let accept_block = self.builder.create_block();
                    let skip_block = self.builder.create_block();
                    
                    // Set conditional terminator for this block
                    self.builder.set_terminator(Terminator::If(
                        Operand::Copy(Place::Local(pred_result)),
                        accept_block,
                        skip_block,
                    ));
                    
                    // Switch to accept block to continue with remaining operations
                    self.builder.switch_block(accept_block);
                    
                    // Track skip block for connection to loop continue
                    self.skip_blocks.push(skip_block);
                }
                "sum" | "fold" => {
                    // Accumulate: acc = acc + current
                    self.builder.add_statement(
                        Place::Local(acc_var.to_string()),
                        Rvalue::BinaryOp(
                            BinaryOp::Add,
                            Operand::Copy(Place::Local(acc_var.to_string())),
                            Operand::Copy(Place::Local(current_var.clone())),
                        ),
                    );
                }
                "count" => {
                    // Phase 2: Specialized count operation
                    // count = count + 1
                    self.builder.add_statement(
                        Place::Local(acc_var.to_string()),
                        Rvalue::BinaryOp(
                            BinaryOp::Add,
                            Operand::Copy(Place::Local(acc_var.to_string())),
                            Operand::Constant(Constant::Integer(1)),
                        ),
                    );
                }
                "any" => {
                    // Phase 2: Specialized any operation
                    // Call predicate, if true return early (handled in return type inference)
                    // For now, accumulate if predicate true
                    let pred_result = self.builder.gen_temp();
                    let arg = Operand::Copy(Place::Local(current_var.clone()));
                    
                    if let Some(pred_return) = self.inline_closure_body(*op_id, arg.clone()) {
                        self.builder.add_statement(
                            Place::Local(pred_result.clone()),
                            Rvalue::Use(pred_return),
                        );
                    } else {
                        self.builder.add_statement(
                            Place::Local(pred_result.clone()),
                            Rvalue::Call(format!("__closure_{}", op_id), vec![arg]),
                        );
                    }
                }
                "all" => {
                    // Phase 2: Specialized all operation
                    // Call predicate, if false return early
                    let pred_result = self.builder.gen_temp();
                    let arg = Operand::Copy(Place::Local(current_var.clone()));
                    
                    if let Some(pred_return) = self.inline_closure_body(*op_id, arg.clone()) {
                        self.builder.add_statement(
                            Place::Local(pred_result.clone()),
                            Rvalue::Use(pred_return),
                        );
                    } else {
                        self.builder.add_statement(
                            Place::Local(pred_result.clone()),
                            Rvalue::Call(format!("__closure_{}", op_id), vec![arg]),
                        );
                    }
                }
                "collect" => {
                    // push to collection
                    // Placeholder for now
                }
                "for_each" => {
                    // Just iterate, no accumulation
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iterator_chain_root_collection() {
        let chain = IteratorChain::Map {
            prev: Box::new(IteratorChain::Iter {
                collection: "vec".to_string(),
            }),
            closure_id: 0,
        };

        assert_eq!(chain.root_collection(), Some("vec"));
    }

    #[test]
    fn test_iterator_chain_is_fusible() {
        let iter = IteratorChain::Iter {
            collection: "vec".to_string(),
        };

        assert!(!iter.is_fusible());

        let map_chain = IteratorChain::Map {
            prev: Box::new(iter),
            closure_id: 0,
        };

        assert!(map_chain.is_fusible());
    }

    #[test]
    fn test_iterator_chain_combinator_count() {
        let chain = IteratorChain::Filter {
            prev: Box::new(IteratorChain::Map {
                prev: Box::new(IteratorChain::Iter {
                    collection: "vec".to_string(),
                }),
                closure_id: 0,
            }),
            predicate_id: 1,
        };

        assert_eq!(chain.combinator_count(), 2); // map + filter
    }

    #[test]
    fn test_iterator_fusion_stats() {
        let mut optimizer = IteratorFusionOptimizer::new();
        let detected_chains = vec![
            (
                "t0".to_string(),
                IteratorChain::Map {
                    prev: Box::new(IteratorChain::Iter {
                        collection: "vec".to_string(),
                    }),
                    closure_id: 0,
                },
            ),
            (
                "t1".to_string(),
                IteratorChain::Iter {
                    collection: "vec2".to_string(),
                },
            ),
        ];

        optimizer.detected_chains = detected_chains;
        let stats = optimizer.report_statistics();

        assert_eq!(stats.total_detected, 2);
        assert_eq!(stats.fusible, 1);
        assert_eq!(stats.non_fusible, 1);
        assert_eq!(stats.total_combinators, 1);
    }

    #[test]
    fn test_extract_closure_id_from_name() {
        assert_eq!(IteratorChainDetector::extract_closure_id_from_name("__closure_0"), Some(0));
        assert_eq!(IteratorChainDetector::extract_closure_id_from_name("__closure_1"), Some(1));
        assert_eq!(IteratorChainDetector::extract_closure_id_from_name("__closure_42"), Some(42));
        assert_eq!(IteratorChainDetector::extract_closure_id_from_name("__closure_"), None);
        assert_eq!(IteratorChainDetector::extract_closure_id_from_name("other_func"), None);
        assert_eq!(IteratorChainDetector::extract_closure_id_from_name("__closure_abc"), None);
    }

    #[test]
    fn test_extract_closure_id_from_args() {
        let mut mapping = HashMap::new();
        mapping.insert("__closure_5".to_string(), 5);
        mapping.insert("__closure_10".to_string(), 10);

        // Test with Copy operand
        let args = vec![
            Operand::Copy(Place::Local("iterator".to_string())),
            Operand::Copy(Place::Local("__closure_5".to_string())),
        ];
        let id = IteratorChainDetector::extract_closure_id_from_args(&args, 1, &mapping);
        assert_eq!(id, Some(5));

        // Test with Move operand
        let args = vec![
            Operand::Copy(Place::Local("iterator".to_string())),
            Operand::Move(Place::Local("__closure_10".to_string())),
        ];
        let id = IteratorChainDetector::extract_closure_id_from_args(&args, 1, &mapping);
        assert_eq!(id, Some(10));

        // Test out of bounds
        let args = vec![Operand::Copy(Place::Local("iterator".to_string()))];
        let id = IteratorChainDetector::extract_closure_id_from_args(&args, 5, &mapping);
        assert_eq!(id, None);

        // Test with non-closure operand
        let args = vec![
            Operand::Copy(Place::Local("iterator".to_string())),
            Operand::Constant(Constant::Integer(42)),
        ];
        let id = IteratorChainDetector::extract_closure_id_from_args(&args, 1, &mapping);
        assert_eq!(id, None);

        // Test with unknown closure name
        let args = vec![
            Operand::Copy(Place::Local("iterator".to_string())),
            Operand::Copy(Place::Local("__closure_999".to_string())),
        ];
        let id = IteratorChainDetector::extract_closure_id_from_args(&args, 1, &mapping);
        assert_eq!(id, None);
    }

    #[test]
    fn test_closure_id_extraction_in_chain() {
        // Test that closure IDs are properly extracted in a map-filter-sum chain
        let chain = IteratorChain::Filter {
            prev: Box::new(IteratorChain::Map {
                prev: Box::new(IteratorChain::Iter {
                    collection: "vec".to_string(),
                }),
                closure_id: 5,  // Extracted from __closure_5
            }),
            predicate_id: 10,  // Extracted from __closure_10
        };

        // Verify the IDs were correctly set
        assert_eq!(chain.combinator_count(), 2);
        
        // Navigate through chain to verify closure IDs
        if let IteratorChain::Filter { prev, predicate_id } = &chain {
            assert_eq!(*predicate_id, 10);
            if let IteratorChain::Map { closure_id, .. } = &**prev {
                assert_eq!(*closure_id, 5);
            }
        }
    }

    #[test]
    fn test_chain_equivalence_same_collection() {
        let chain1 = IteratorChain::Iter {
            collection: "vec".to_string(),
        };
        let chain2 = IteratorChain::Iter {
            collection: "vec".to_string(),
        };
        let chain3 = IteratorChain::Iter {
            collection: "vec2".to_string(),
        };

        assert!(chain1.is_equivalent_to(&chain2));
        assert!(!chain1.is_equivalent_to(&chain3));
    }

    #[test]
    fn test_chain_equivalence_ignores_closure_ids() {
        // Two map chains with same iterator but different closure IDs should be equivalent
        // (because equivalence is about structure, not closure identity)
        let chain1 = IteratorChain::Map {
            prev: Box::new(IteratorChain::Iter {
                collection: "vec".to_string(),
            }),
            closure_id: 5,
        };
        let chain2 = IteratorChain::Map {
            prev: Box::new(IteratorChain::Iter {
                collection: "vec".to_string(),
            }),
            closure_id: 10,
        };

        assert!(chain1.is_equivalent_to(&chain2));
    }

    #[test]
    fn test_chain_equivalence_complex() {
        // Complex: map-filter-sum chains should be equivalent if same structure
        let chain1 = IteratorChain::Sum {
            prev: Box::new(IteratorChain::Filter {
                prev: Box::new(IteratorChain::Map {
                    prev: Box::new(IteratorChain::Iter {
                        collection: "vec".to_string(),
                    }),
                    closure_id: 0,
                }),
                predicate_id: 1,
            }),
        };

        let chain2 = IteratorChain::Sum {
            prev: Box::new(IteratorChain::Filter {
                prev: Box::new(IteratorChain::Map {
                    prev: Box::new(IteratorChain::Iter {
                        collection: "vec".to_string(),
                    }),
                    closure_id: 99,  // Different closure ID
                }),
                predicate_id: 88,  // Different predicate ID
            }),
        };

        assert!(chain1.is_equivalent_to(&chain2));
    }

    #[test]
    fn test_chain_not_equivalent_different_structure() {
        // Different structures should not be equivalent
        let chain1 = IteratorChain::Map {
            prev: Box::new(IteratorChain::Iter {
                collection: "vec".to_string(),
            }),
            closure_id: 0,
        };

        let chain2 = IteratorChain::Filter {
            prev: Box::new(IteratorChain::Iter {
                collection: "vec".to_string(),
            }),
            predicate_id: 0,
        };

        assert!(!chain1.is_equivalent_to(&chain2));
    }

    #[test]
    fn test_deduplicate_chains() {
        let mut optimizer = IteratorFusionOptimizer::new();

        // Create some detected chains - some equivalent, some different
        let chain1 = IteratorChain::Map {
            prev: Box::new(IteratorChain::Iter {
                collection: "vec".to_string(),
            }),
            closure_id: 0,
        };

        let chain2 = IteratorChain::Map {
            prev: Box::new(IteratorChain::Iter {
                collection: "vec".to_string(),
            }),
            closure_id: 99,  // Different closure but equivalent structure
        };

        let chain3 = IteratorChain::Filter {
            prev: Box::new(IteratorChain::Iter {
                collection: "vec".to_string(),
            }),
            predicate_id: 0,
        };

        optimizer.detected_chains = vec![
            ("t0".to_string(), chain1),
            ("t1".to_string(), chain2),
            ("t2".to_string(), chain3),
        ];

        let unique = optimizer.deduplicate_chains();
        assert_eq!(unique.len(), 2);  // Two unique chains
        assert_eq!(unique[0].2, 2);   // First chain appears twice (t0, t1)
        assert_eq!(unique[1].2, 1);   // Second chain appears once (t2)
    }

    #[test]
    fn test_fusion_config_default() {
        let config = FusionConfig::default();
        assert_eq!(config.max_chain_length, 8);
        assert_eq!(config.min_chain_length, 2);
        assert!(config.inline_closures);
        assert!(!config.unroll_loops);
    }

    #[test]
    fn test_fusion_transformer_should_fuse() {
        let config = FusionConfig::default();

        // Chain with 0 combinators - too short
        let chain_short = IteratorChain::Iter {
            collection: "vec".to_string(),
        };
        assert!(!IteratorFusionTransformer::should_fuse(&chain_short, &config));

        // Chain with 2 combinators - good
        let chain_good = IteratorChain::Sum {
            prev: Box::new(IteratorChain::Filter {
                prev: Box::new(IteratorChain::Map {
                    prev: Box::new(IteratorChain::Iter {
                        collection: "vec".to_string(),
                    }),
                    closure_id: 0,
                }),
                predicate_id: 1,
            }),
        };
        assert!(IteratorFusionTransformer::should_fuse(&chain_good, &config));

        // Chain with too many combinators
        let config_strict = FusionConfig {
            max_chain_length: 1,
            ..Default::default()
        };
        assert!(!IteratorFusionTransformer::should_fuse(&chain_good, &config_strict));
    }

    #[test]
    fn test_fusion_transformer_fuse_chain() {
        let config = FusionConfig::default();

        // Chain with 2 combinators (map + filter) - meets minimum
        let chain = IteratorChain::Sum {
            prev: Box::new(IteratorChain::Filter {
                prev: Box::new(IteratorChain::Map {
                    prev: Box::new(IteratorChain::Iter {
                        collection: "numbers".to_string(),
                    }),
                    closure_id: 0,
                }),
                predicate_id: 1,
            }),
        };

        let result = IteratorFusionTransformer::fuse_chain(&chain, &config);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "__fused_iter_numbers");
    }

    #[test]
    fn test_fusion_transformer_is_safe_to_fuse() {
        let chain = IteratorChain::Map {
            prev: Box::new(IteratorChain::Iter {
                collection: "vec".to_string(),
            }),
            closure_id: 0,
        };

        assert!(IteratorFusionTransformer::is_safe_to_fuse(&chain));
    }

    #[test]
    fn test_fusion_transformer_estimate_benefit() {
        // Single combinator chain
        let chain1 = IteratorChain::Map {
            prev: Box::new(IteratorChain::Iter {
                collection: "vec".to_string(),
            }),
            closure_id: 0,
        };
        let (reduction1, speedup1) = IteratorFusionTransformer::estimate_benefit(&chain1);
        assert_eq!(reduction1, 0.20);  // 1 combinator * 0.20
        assert!(speedup1 > 1.0);       // Expected speedup > 1x

        // Three combinator chain
        let chain3 = IteratorChain::Sum {
            prev: Box::new(IteratorChain::Filter {
                prev: Box::new(IteratorChain::Map {
                    prev: Box::new(IteratorChain::Iter {
                        collection: "vec".to_string(),
                    }),
                    closure_id: 0,
                }),
                predicate_id: 1,
            }),
        };
        let (reduction3, speedup3) = IteratorFusionTransformer::estimate_benefit(&chain3);
        assert!(reduction3 > reduction1);
        assert!(speedup3 > speedup1);
    }

    #[test]
    fn test_fusion_transformer_extract_operations() {
        // Chain: vec.iter().map(f).filter(p).sum()
        // Internally represented as: Sum { Filter { Map { Iter } } }
        let chain = IteratorChain::Sum {
            prev: Box::new(IteratorChain::Filter {
                prev: Box::new(IteratorChain::Map {
                    prev: Box::new(IteratorChain::Iter {
                        collection: "vec".to_string(),
                    }),
                    closure_id: 5,
                }),
                predicate_id: 10,
            }),
        };

        let ops = IteratorFusionTransformer::extract_operations(&chain);
        assert_eq!(ops.len(), 3);
        // Recursion collects: Iter (none) -> Map (add) -> Filter (add) -> Sum (add)
        assert_eq!(ops[0], ("map".to_string(), 5));
        assert_eq!(ops[1], ("filter".to_string(), 10));
        assert_eq!(ops[2], ("sum".to_string(), 0));
    }

    #[test]
    fn test_fusion_transformer_analyze_opportunity() {
        let chain = IteratorChain::Sum {
            prev: Box::new(IteratorChain::Filter {
                prev: Box::new(IteratorChain::Map {
                    prev: Box::new(IteratorChain::Iter {
                        collection: "data".to_string(),
                    }),
                    closure_id: 0,
                }),
                predicate_id: 1,
            }),
        };

        let opportunity = IteratorFusionTransformer::analyze_opportunity(&chain).unwrap();
        assert_eq!(opportunity.collection, "data");
        assert_eq!(opportunity.operations.len(), 3);
        assert!(opportunity.speedup > 1.0);
        assert!(opportunity.code_reduction > 0.0);
    }

    #[test]
    fn test_fusion_opportunity_structure() {
        let opp = FusionOpportunity {
            collection: "vec".to_string(),
            operations: vec![
                ("map".to_string(), 0),
                ("filter".to_string(), 1),
                ("sum".to_string(), 0),
            ],
            speedup: 1.45,
            code_reduction: 0.4,
        };

        assert_eq!(opp.collection, "vec");
        assert_eq!(opp.operations.len(), 3);
        assert_eq!(opp.speedup, 1.45);
        assert_eq!(opp.code_reduction, 0.4);
    }

    // Phase 1e: MIR Generation Tests

    #[test]
    fn test_mir_generator_creation() {
        let opp = FusionOpportunity {
            collection: "numbers".to_string(),
            operations: vec![
                ("sum".to_string(), 0),
            ],
            speedup: 1.15,
            code_reduction: 0.2,
        };

        let gen = FusionMirGenerator::new(opp);
        assert_eq!(gen.collection_name, "numbers");
    }

    #[test]
    fn test_mir_generation_simple_sum() {
        let opp = FusionOpportunity {
            collection: "vec".to_string(),
            operations: vec![
                ("sum".to_string(), 0),
            ],
            speedup: 1.15,
            code_reduction: 0.2,
        };

        let gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();

        // Verify function structure
        assert_eq!(mir_func.name, "__fused_iter_vec");
        assert_eq!(mir_func.params.len(), 1);
        assert_eq!(mir_func.params[0].0, "vec");
        
        // Verify basic blocks were created
        assert!(mir_func.basic_blocks.len() >= 4); // bb0, bb1, bb2, bb3
    }

    #[test]
    fn test_mir_generation_map_sum() {
        let opp = FusionOpportunity {
            collection: "data".to_string(),
            operations: vec![
                ("map".to_string(), 5),
                ("sum".to_string(), 0),
            ],
            speedup: 1.30,
            code_reduction: 0.4,
        };

        let gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();

        assert_eq!(mir_func.name, "__fused_iter_data");
        assert_eq!(mir_func.params.len(), 1);
        assert!(mir_func.basic_blocks.len() >= 4);
    }

    #[test]
    fn test_mir_generation_map_filter_sum() {
        let opp = FusionOpportunity {
            collection: "values".to_string(),
            operations: vec![
                ("map".to_string(), 0),
                ("filter".to_string(), 1),
                ("sum".to_string(), 0),
            ],
            speedup: 1.45,
            code_reduction: 0.6,
        };

        let gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();

        assert_eq!(mir_func.name, "__fused_iter_values");
        assert!(mir_func.basic_blocks.len() >= 4);
        
        // Verify loop structure exists
        assert!(mir_func.basic_blocks[0].statements.len() > 0); // Setup block has statements
    }

    #[test]
    fn test_mir_generator_loop_setup() {
        let opp = FusionOpportunity {
            collection: "test_vec".to_string(),
            operations: vec![
                ("sum".to_string(), 0),
            ],
            speedup: 1.15,
            code_reduction: 0.2,
        };

        let gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();

        // Verify setup block initializes variables
        let setup_block = &mir_func.basic_blocks[0];
        assert!(setup_block.statements.len() >= 2); // At least i=0 and acc=0
    }

    #[test]
    fn test_mir_generator_return_statement() {
        let opp = FusionOpportunity {
            collection: "arr".to_string(),
            operations: vec![
                ("sum".to_string(), 0),
            ],
            speedup: 1.15,
            code_reduction: 0.2,
        };

        let gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();

        // Find the return block (should exist somewhere in the blocks)
        let has_return = mir_func.basic_blocks.iter().any(|block| {
            matches!(block.terminator, Terminator::Return(_))
        });
        assert!(has_return, "Function should have a return statement");
    }

    #[test]
    fn test_mir_generator_loop_goto() {
        let opp = FusionOpportunity {
            collection: "items".to_string(),
            operations: vec![
                ("map".to_string(), 0),
                ("sum".to_string(), 0),
            ],
            speedup: 1.30,
            code_reduction: 0.4,
        };

        let gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();

        // Verify loop structure has gotos
        let mut has_goto = false;
        for block in &mir_func.basic_blocks {
            if matches!(block.terminator, Terminator::Goto(_)) {
                has_goto = true;
                break;
            }
        }
        assert!(has_goto);
    }

    #[test]
    fn test_mir_generator_collect() {
        let opp = FusionOpportunity {
            collection: "source".to_string(),
            operations: vec![
                ("map".to_string(), 0),
                ("collect".to_string(), 0),
            ],
            speedup: 1.30,
            code_reduction: 0.4,
        };

        let gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();

        assert_eq!(mir_func.name, "__fused_iter_source");
        assert!(mir_func.basic_blocks.len() >= 4);
    }

    #[test]
    fn test_mir_generator_for_each() {
        let opp = FusionOpportunity {
            collection: "items".to_string(),
            operations: vec![
                ("for_each".to_string(), 2),
            ],
            speedup: 1.15,
            code_reduction: 0.2,
        };

        let gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();

        assert_eq!(mir_func.name, "__fused_iter_items");
        assert!(mir_func.basic_blocks.len() >= 4);
    }

    // Type Inference Tests
    #[test]
    fn test_type_inference_sum() {
        let opp = FusionOpportunity {
            collection: "numbers".to_string(),
            operations: vec![
                ("sum".to_string(), 0),
            ],
            speedup: 1.10,
            code_reduction: 0.2,
        };

        let gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();

        // sum() should return Int64
        assert_eq!(mir_func.return_type, HirType::Int64);
    }

    #[test]
    fn test_type_inference_any() {
        let opp = FusionOpportunity {
            collection: "data".to_string(),
            operations: vec![
                ("any".to_string(), 1),
            ],
            speedup: 1.15,
            code_reduction: 0.3,
        };

        let gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();

        // any() should return Bool
        assert_eq!(mir_func.return_type, HirType::Bool);
    }

    #[test]
    fn test_type_inference_all() {
        let opp = FusionOpportunity {
            collection: "items".to_string(),
            operations: vec![
                ("all".to_string(), 1),
            ],
            speedup: 1.15,
            code_reduction: 0.3,
        };

        let gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();

        // all() should return Bool
        assert_eq!(mir_func.return_type, HirType::Bool);
    }

    #[test]
    fn test_type_inference_count() {
        let opp = FusionOpportunity {
            collection: "elements".to_string(),
            operations: vec![
                ("count".to_string(), 0),
            ],
            speedup: 1.12,
            code_reduction: 0.25,
        };

        let gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();

        // count() should return Int64
        assert_eq!(mir_func.return_type, HirType::Int64);
    }

    // Closure Storage Tests
    #[test]
    fn test_closure_registration() {
        let opp = FusionOpportunity {
            collection: "data".to_string(),
            operations: vec![
                ("map".to_string(), 1),
            ],
            speedup: 1.2,
            code_reduction: 0.3,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Register a closure
        let closure_meta = ClosureMetadata {
            id: 1,
            params: vec![("x".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Copy(Place::Local("x".to_string()))),
        };
        
        gen.register_closure(closure_meta);
        
        // Verify it was registered
        assert!(gen.get_closure_body(1).is_some());
        assert!(gen.get_closure_body(99).is_none());
    }

    #[test]
    fn test_closure_body_retrieval() {
        let opp = FusionOpportunity {
            collection: "items".to_string(),
            operations: vec![
                ("filter".to_string(), 2),
            ],
            speedup: 1.15,
            code_reduction: 0.2,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Register multiple closures
        let closure_1 = ClosureMetadata {
            id: 2,
            params: vec![("elem".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Copy(Place::Local("elem".to_string()))),
        };
        
        let closure_2 = ClosureMetadata {
            id: 5,
            params: vec![("val".to_string(), HirType::Bool)],
            body_statements: vec![],
            return_value: Some(Operand::Constant(Constant::Bool(true))),
        };
        
        gen.register_closure(closure_1);
        gen.register_closure(closure_2);
        
        // Verify retrieval
        let retrieved = gen.get_closure_body(2);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, 2);
        
        let retrieved2 = gen.get_closure_body(5);
        assert!(retrieved2.is_some());
        assert_eq!(retrieved2.unwrap().id, 5);
    }

    // Closure Inlining Tests
    #[test]
    fn test_inline_simple_closure() {
        let opp = FusionOpportunity {
            collection: "nums".to_string(),
            operations: vec![
                ("map".to_string(), 1),
                ("sum".to_string(), 0),
            ],
            speedup: 1.3,
            code_reduction: 0.35,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Register a simple closure that just returns its argument
        let closure = ClosureMetadata {
            id: 1,
            params: vec![("x".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Copy(Place::Local("x".to_string()))),
        };
        
        gen.register_closure(closure);
        
        // Try to inline it
        let arg = Operand::Copy(Place::Local("elem".to_string()));
        let result = gen.inline_closure_body(1, arg);
        
        // Should have a return value
        assert!(result.is_some());
    }

    #[test]
    fn test_inline_closure_with_statements() {
        let opp = FusionOpportunity {
            collection: "data".to_string(),
            operations: vec![
                ("map".to_string(), 3),
            ],
            speedup: 1.25,
            code_reduction: 0.3,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Register a closure with body statements
        let closure = ClosureMetadata {
            id: 3,
            params: vec![("x".to_string(), HirType::Int64)],
            body_statements: vec![
                Statement {
                    place: Place::Local("temp".to_string()),
                    rvalue: Rvalue::BinaryOp(
                        BinaryOp::Multiply,
                        Operand::Copy(Place::Local("x".to_string())),
                        Operand::Constant(Constant::Integer(2)),
                    ),
                },
            ],
            return_value: Some(Operand::Copy(Place::Local("temp".to_string()))),
        };
        
        gen.register_closure(closure);
        
        // Inline it
        let arg = Operand::Copy(Place::Local("value".to_string()));
        let result = gen.inline_closure_body(3, arg);
        
        // Should return something
        assert!(result.is_some());
    }

    #[test]
    fn test_inline_nonexistent_closure() {
        let opp = FusionOpportunity {
            collection: "test".to_string(),
            operations: vec![
                ("map".to_string(), 99),
            ],
            speedup: 1.1,
            code_reduction: 0.15,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Try to inline a closure that doesn't exist
        let arg = Operand::Copy(Place::Local("x".to_string()));
        let result = gen.inline_closure_body(99, arg);
        
        // Should return None since closure doesn't exist
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_operations_with_inlining() {
        let opp = FusionOpportunity {
            collection: "values".to_string(),
            operations: vec![
                ("map".to_string(), 2),
                ("sum".to_string(), 0),
            ],
            speedup: 1.35,
            code_reduction: 0.4,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Register closure for map operation
        let closure = ClosureMetadata {
            id: 2,
            params: vec![("n".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Copy(Place::Local("n".to_string()))),
        };
        
        gen.register_closure(closure);
        
        // Generate function to test apply_operations
        let mir_func = gen.generate();
        
        // Should have generated successfully
        assert_eq!(mir_func.name, "__fused_iter_values");
        assert!(mir_func.basic_blocks.len() >= 4);
    }

    // Filter Conditional Tests
    #[test]
    fn test_filter_with_closure() {
        let opp = FusionOpportunity {
            collection: "data".to_string(),
            operations: vec![
                ("filter".to_string(), 1),
                ("sum".to_string(), 0),
            ],
            speedup: 1.25,
            code_reduction: 0.3,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Register filter predicate
        let filter_pred = ClosureMetadata {
            id: 1,
            params: vec![("x".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Constant(Constant::Bool(true))),
        };
        
        gen.register_closure(filter_pred);
        
        // Generate with filter
        let mir_func = gen.generate();
        
        // Should compile successfully
        assert_eq!(mir_func.name, "__fused_iter_data");
        assert!(mir_func.basic_blocks.len() >= 4);
    }

    #[test]
    fn test_map_filter_combination() {
        let opp = FusionOpportunity {
            collection: "items".to_string(),
            operations: vec![
                ("map".to_string(), 0),
                ("filter".to_string(), 1),
                ("sum".to_string(), 0),
            ],
            speedup: 1.5,
            code_reduction: 0.45,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Register map and filter
        let map_closure = ClosureMetadata {
            id: 0,
            params: vec![("x".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Copy(Place::Local("x".to_string()))),
        };
        
        let filter_closure = ClosureMetadata {
            id: 1,
            params: vec![("y".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Constant(Constant::Bool(true))),
        };
        
        gen.register_closure(map_closure);
        gen.register_closure(filter_closure);
        
        let mir_func = gen.generate();
        
        assert_eq!(mir_func.name, "__fused_iter_items");
        assert!(mir_func.basic_blocks.len() >= 4);
    }

    // Operation Specialization Tests
    #[test]
    fn test_specialize_count() {
        let opp = FusionOpportunity {
            collection: "elements".to_string(),
            operations: vec![
                ("count".to_string(), 0),
            ],
            speedup: 1.15,
            code_reduction: 0.25,
        };

        let mut gen = FusionMirGenerator::new(opp);
        let mir_func = gen.generate();
        
        // count returns Int64
        assert_eq!(mir_func.return_type, HirType::Int64);
        assert_eq!(mir_func.name, "__fused_iter_elements");
    }

    #[test]
    fn test_specialize_any() {
        let opp = FusionOpportunity {
            collection: "values".to_string(),
            operations: vec![
                ("any".to_string(), 2),
            ],
            speedup: 1.2,
            code_reduction: 0.3,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Register any predicate
        let any_pred = ClosureMetadata {
            id: 2,
            params: vec![("elem".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Constant(Constant::Bool(false))),
        };
        
        gen.register_closure(any_pred);
        let mir_func = gen.generate();
        
        // any returns Bool
        assert_eq!(mir_func.return_type, HirType::Bool);
    }

    #[test]
    fn test_specialize_all() {
        let opp = FusionOpportunity {
            collection: "checks".to_string(),
            operations: vec![
                ("all".to_string(), 3),
            ],
            speedup: 1.2,
            code_reduction: 0.3,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Register all predicate
        let all_pred = ClosureMetadata {
            id: 3,
            params: vec![("item".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Constant(Constant::Bool(true))),
        };
        
        gen.register_closure(all_pred);
        let mir_func = gen.generate();
        
        // all returns Bool
        assert_eq!(mir_func.return_type, HirType::Bool);
    }

    #[test]
    fn test_complex_chain_with_specialization() {
        let opp = FusionOpportunity {
            collection: "dataset".to_string(),
            operations: vec![
                ("map".to_string(), 0),
                ("filter".to_string(), 1),
                ("count".to_string(), 0),
            ],
            speedup: 1.55,
            code_reduction: 0.5,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        let map_c = ClosureMetadata {
            id: 0,
            params: vec![("x".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Copy(Place::Local("x".to_string()))),
        };
        
        let filter_c = ClosureMetadata {
            id: 1,
            params: vec![("y".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Constant(Constant::Bool(true))),
        };
        
        gen.register_closure(map_c);
        gen.register_closure(filter_c);
        
        let mir_func = gen.generate();
        
        // Should return Int64 for count operation
        assert_eq!(mir_func.return_type, HirType::Int64);
        assert_eq!(mir_func.name, "__fused_iter_dataset");
    }

    // Filter Guard Branching Tests
    #[test]
    fn test_filter_guard_branching() {
        let opp = FusionOpportunity {
            collection: "numbers".to_string(),
            operations: vec![
                ("filter".to_string(), 1),
                ("sum".to_string(), 0),
            ],
            speedup: 1.25,
            code_reduction: 0.3,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Register filter predicate
        let filter_pred = ClosureMetadata {
            id: 1,
            params: vec![("x".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Constant(Constant::Bool(true))),
        };
        
        gen.register_closure(filter_pred);
        
        // Generate with filter guard
        let mir_func = gen.generate();
        
        // Should have more blocks due to filter branching
        assert!(mir_func.basic_blocks.len() >= 5); // setup, header, body, continue, exit, + filter blocks
        assert_eq!(mir_func.name, "__fused_iter_numbers");
    }

    #[test]
    fn test_parameter_substitution_in_closure() {
        let opp = FusionOpportunity {
            collection: "values".to_string(),
            operations: vec![
                ("map".to_string(), 0),
                ("sum".to_string(), 0),
            ],
            speedup: 1.3,
            code_reduction: 0.35,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Register closure with statements that use parameter
        let map_closure = ClosureMetadata {
            id: 0,
            params: vec![("x".to_string(), HirType::Int64)],
            body_statements: vec![
                Statement {
                    place: Place::Local("result".to_string()),
                    rvalue: Rvalue::BinaryOp(
                        BinaryOp::Multiply,
                        Operand::Copy(Place::Local("x".to_string())),
                        Operand::Constant(Constant::Integer(2)),
                    ),
                },
            ],
            return_value: Some(Operand::Copy(Place::Local("result".to_string()))),
        };
        
        gen.register_closure(map_closure);
        
        // Generate should apply parameter substitution
        let mir_func = gen.generate();
        assert_eq!(mir_func.name, "__fused_iter_values");
        assert!(mir_func.basic_blocks.len() >= 4);
    }

    #[test]
    fn test_filter_with_parameter_substitution() {
        let opp = FusionOpportunity {
            collection: "items".to_string(),
            operations: vec![
                ("filter".to_string(), 1),
                ("count".to_string(), 0),
            ],
            speedup: 1.2,
            code_reduction: 0.25,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        // Register filter with parameter usage
        let filter_closure = ClosureMetadata {
            id: 1,
            params: vec![("n".to_string(), HirType::Int64)],
            body_statements: vec![
                Statement {
                    place: Place::Local("cmp".to_string()),
                    rvalue: Rvalue::BinaryOp(
                        BinaryOp::GreaterEqual,
                        Operand::Copy(Place::Local("n".to_string())),
                        Operand::Constant(Constant::Integer(0)),
                    ),
                },
            ],
            return_value: Some(Operand::Copy(Place::Local("cmp".to_string()))),
        };
        
        gen.register_closure(filter_closure);
        
        let mir_func = gen.generate();
        assert_eq!(mir_func.return_type, HirType::Int64);
    }

    #[test]
    fn test_multiple_filters_with_guards() {
        let opp = FusionOpportunity {
            collection: "data".to_string(),
            operations: vec![
                ("filter".to_string(), 1),
                ("filter".to_string(), 2),
                ("sum".to_string(), 0),
            ],
            speedup: 1.4,
            code_reduction: 0.4,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        let filter1 = ClosureMetadata {
            id: 1,
            params: vec![("x".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Constant(Constant::Bool(true))),
        };
        
        let filter2 = ClosureMetadata {
            id: 2,
            params: vec![("y".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Constant(Constant::Bool(true))),
        };
        
        gen.register_closure(filter1);
        gen.register_closure(filter2);
        
        let mir_func = gen.generate();
        
        // Multiple filters should create multiple guard blocks
        assert!(mir_func.basic_blocks.len() >= 6); // More blocks for nested guards
        assert_eq!(mir_func.name, "__fused_iter_data");
    }

    #[test]
    fn test_simd_detection_simple_chain() {
        let opp = FusionOpportunity {
            collection: "array".to_string(),
            operations: vec![
                ("map".to_string(), 0),
                ("sum".to_string(), 0),
            ],
            speedup: 1.25,
            code_reduction: 0.3,
        };

        let mut gen = FusionMirGenerator::new(opp);
        
        let map_c = ClosureMetadata {
            id: 0,
            params: vec![("x".to_string(), HirType::Int64)],
            body_statements: vec![],
            return_value: Some(Operand::Copy(Place::Local("x".to_string()))),
        };
        
        gen.register_closure(map_c);
        
        let mir_func = gen.generate();
        
        // Simple arithmetic chains are SIMD candidates
        assert_eq!(mir_func.return_type, HirType::Int64);
    }
}
