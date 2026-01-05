/// Memory Optimization Module for GaiaRusted v0.12.0
/// 
/// Implements advanced memory optimization techniques:
/// - Escape analysis for stack vs heap allocation decisions
/// - Reference counting optimization
/// - Lifetime-based memory pool allocation
/// - Data structure layout optimization

use std::collections::{HashMap, HashSet};
use crate::mir::{BasicBlock, Statement, Place, Terminator};

/// Escape Analysis: Determines whether values can be safely stack-allocated
#[derive(Debug, Clone)]
pub struct EscapeAnalysis {
    /// Maps local variables to their escape status (by name)
    escapes: HashMap<String, EscapeStatus>,
    /// Tracks which locals are used outside their defining scope
    usage_sites: HashMap<String, Vec<UsageSite>>,
    /// Set of locals that must be heap-allocated
    heap_required: HashSet<String>,
    /// Configuration for escape analysis
    config: EscapeAnalysisConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeStatus {
    /// Value never escapes the current scope (stack-safe)
    DoesNotEscape,
    /// Value may escape to function return (needs move semantics)
    MayEscapeReturn,
    /// Value stored in struct or array (escapes scope)
    EscapesToMemory,
    /// Value shared across threads (needs synchronization)
    EscapesThread,
}

#[derive(Debug, Clone)]
pub struct UsageSite {
    pub block_idx: usize,
    pub stmt_idx: usize,
    pub kind: UsageKind,
}

#[derive(Debug, Clone, Copy)]
pub enum UsageKind {
    Read,
    Write,
    FieldAccess,
    MethodCall,
    Return,
}

#[derive(Debug, Clone)]
pub struct EscapeAnalysisConfig {
    /// Enable field escape tracking
    pub track_field_escapes: bool,
    /// Enable return value tracking
    pub track_returns: bool,
    /// Enable thread escape tracking
    pub track_thread_escapes: bool,
}

impl Default for EscapeAnalysisConfig {
    fn default() -> Self {
        Self {
            track_field_escapes: true,
            track_returns: true,
            track_thread_escapes: false,
        }
    }
}

impl EscapeAnalysis {
    pub fn new(config: EscapeAnalysisConfig) -> Self {
        Self {
            escapes: HashMap::new(),
            usage_sites: HashMap::new(),
            heap_required: HashSet::new(),
            config,
        }
    }

    /// Insert escape status (for testing)
    pub fn insert_escape_status(&mut self, local_name: String, status: EscapeStatus) {
        self.escapes.insert(local_name, status);
    }

    /// Mark a local as heap-required (for testing)
    pub fn mark_heap_required(&mut self, local_name: String) {
        self.heap_required.insert(local_name);
    }

    /// Analyze a function to determine escape status of all locals
    pub fn analyze_function(&mut self, blocks: &[BasicBlock]) {
        // Scan all blocks to find unique local variables
        let mut local_names = HashSet::new();
        for block in blocks {
            for stmt in &block.statements {
                // Check if place is a Local and extract its name
                if let Place::Local(name) = &stmt.place {
                    // Mark this local for analysis using its actual name
                    local_names.insert(name.clone());
                }
            }
        }
        
        // Analyze each found local
        for local_name in local_names {
            self.analyze_local(local_name, blocks);
        }
    }

    /// Analyze a single local variable's usage throughout the function
    fn analyze_local(&mut self, local_name: String, blocks: &[BasicBlock]) {
        let mut usage_sites = Vec::new();
        let mut escape_status = EscapeStatus::DoesNotEscape;

        for (block_idx, block) in blocks.iter().enumerate() {
            for (stmt_idx, stmt) in block.statements.iter().enumerate() {
                // Check if this local is used in this statement
                if let Some(kind) = self.find_usage_kind(&local_name, stmt) {
                    usage_sites.push(UsageSite {
                        block_idx,
                        stmt_idx,
                        kind,
                    });

                    // Update escape status based on usage
                    match kind {
                        UsageKind::Return => {
                            escape_status = EscapeStatus::MayEscapeReturn;
                        }
                        UsageKind::FieldAccess if self.config.track_field_escapes => {
                            escape_status = EscapeStatus::EscapesToMemory;
                        }
                        UsageKind::MethodCall => {
                            escape_status = EscapeStatus::EscapesToMemory;
                        }
                        _ => {}
                    }
                }
            }

            // Check terminator for returns
            if self.config.track_returns {
                if let Terminator::Return(Some(_operand)) = &block.terminator {
                    // If a local is returned, it escapes
                    escape_status = EscapeStatus::MayEscapeReturn;
                }
            }
        }

        self.usage_sites.insert(local_name.clone(), usage_sites);
        self.escapes.insert(local_name.clone(), escape_status);

        // Mark for heap allocation if needed
        if escape_status == EscapeStatus::EscapesToMemory
            || escape_status == EscapeStatus::EscapesThread
        {
            self.heap_required.insert(local_name);
        }
    }

    fn find_usage_kind(&self, local_name: &str, stmt: &Statement) -> Option<UsageKind> {
        // Check if this statement assigns to our local
        if self.place_matches_local(local_name, &stmt.place) {
            Some(UsageKind::Write)
        } else {
            None
        }
    }

    fn place_matches_local(&self, local_name: &str, place: &Place) -> bool {
        // Check if the place refers to our local variable
        match place {
            Place::Local(name) => name == local_name,
            // For complex places (field access, indexing), we would check the base recursively
            // but for now we only handle direct local matches
            _ => false,
        }
    }

    /// Get escape status for a local
    pub fn get_escape_status(&self, local_name: &str) -> Option<EscapeStatus> {
        self.escapes.get(local_name).copied()
    }

    /// Check if a local requires heap allocation
    pub fn requires_heap(&self, local_name: &str) -> bool {
        self.heap_required.contains(local_name)
    }

    /// Get all usage sites for a local
    pub fn get_usage_sites(&self, local_name: &str) -> Option<&[UsageSite]> {
        self.usage_sites.get(local_name).map(|v| v.as_slice())
    }

    /// Report allocation recommendations
    pub fn report_allocations(&self) -> AllocationReport {
        let mut stack_candidates = Vec::new();
        let mut heap_required = Vec::new();
        let mut escape_to_return = Vec::new();

        for (local_name, status) in &self.escapes {
            match status {
                EscapeStatus::DoesNotEscape => {
                    stack_candidates.push(local_name.clone());
                }
                EscapeStatus::MayEscapeReturn => {
                    escape_to_return.push(local_name.clone());
                }
                EscapeStatus::EscapesToMemory | EscapeStatus::EscapesThread => {
                    heap_required.push(local_name.clone());
                }
            }
        }

        AllocationReport {
            stack_candidates,
            heap_required,
            escape_to_return,
            total_locals: self.escapes.len(),
        }
    }
}

/// Report on allocation decisions
#[derive(Debug, Clone)]
pub struct AllocationReport {
    pub stack_candidates: Vec<String>,
    pub heap_required: Vec<String>,
    pub escape_to_return: Vec<String>,
    pub total_locals: usize,
}

/// Reference Counting Optimization: Reduces refcount operations
#[derive(Debug, Clone)]
pub struct RefCountOptimizer {
    /// Tracks consecutive increments/decrements
    refcount_chains: HashMap<usize, Vec<RefCountOp>>,
    config: RefCountConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefCountOp {
    Increment,
    Decrement,
}

#[derive(Debug, Clone)]
pub struct RefCountConfig {
    pub enable_chain_fusion: bool,
    pub enable_move_semantics: bool,
}

impl Default for RefCountConfig {
    fn default() -> Self {
        Self {
            enable_chain_fusion: true,
            enable_move_semantics: true,
        }
    }
}

impl RefCountOptimizer {
    pub fn new(config: RefCountConfig) -> Self {
        Self {
            refcount_chains: HashMap::new(),
            config,
        }
    }

    /// Get mutable access to chains (for internal use)
    pub fn get_chains_mut(&mut self) -> &mut HashMap<usize, Vec<RefCountOp>> {
        &mut self.refcount_chains
    }

    /// Optimize refcount chains (e.g., inc+dec = no-op)
    pub fn optimize_chains(&mut self) -> RefCountOptimizationResult {
        let mut eliminated = 0;
        let mut fused = 0;

        for chains in self.refcount_chains.values_mut() {
            let mut i = 0;
            while i < chains.len() {
                // Look for inc/dec pairs
                if i + 1 < chains.len() {
                    match (chains[i], chains[i + 1]) {
                        (RefCountOp::Increment, RefCountOp::Decrement) => {
                            chains.remove(i + 1);
                            chains.remove(i);
                            eliminated += 1;
                            continue;
                        }
                        _ => {}
                    }
                }
                i += 1;
            }

            // Fuse consecutive same operations
            if self.config.enable_chain_fusion && !chains.is_empty() {
                let mut fused_chains = vec![chains[0]];
                for op in chains.iter().skip(1) {
                    if fused_chains.last() == Some(op) {
                        fused += 1;
                    } else {
                        fused_chains.push(*op);
                    }
                }
                *chains = fused_chains;
            }
        }

        RefCountOptimizationResult {
            pairs_eliminated: eliminated,
            operations_fused: fused,
        }
    }

    /// Add a refcount operation to a chain
    pub fn add_operation(&mut self, local_idx: usize, op: RefCountOp) {
        self.refcount_chains.entry(local_idx).or_insert_with(Vec::new).push(op);
    }

    /// Check if we can use move semantics instead of refcount
    pub fn can_use_move_semantics(&self, local_idx: usize) -> bool {
        if !self.config.enable_move_semantics {
            return false;
        }

        if let Some(chains) = self.refcount_chains.get(&local_idx) {
            // If there's only one increment followed by one decrement, use move
            chains.len() <= 2
        } else {
            true
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefCountOptimizationResult {
    pub pairs_eliminated: usize,
    pub operations_fused: usize,
}

/// Lifetime-based Memory Pool Allocation
#[derive(Debug, Clone)]
pub struct MemoryPoolAllocator {
    /// Pool configurations by lifetime scope
    pools: HashMap<LifetimeScope, PoolConfig>,
    config: PoolConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifetimeScope {
    /// Valid for entire function
    Function,
    /// Valid for a single loop
    Loop,
    /// Valid for a single block
    Block,
}

#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Initial pool size in bytes
    pub initial_size: usize,
    /// Growth strategy
    pub growth_strategy: GrowthStrategy,
    /// Enable pool reuse
    pub enable_reuse: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum GrowthStrategy {
    Linear(usize),
    Exponential(f32),
    Fixed,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            initial_size: 4096,
            growth_strategy: GrowthStrategy::Exponential(1.5),
            enable_reuse: true,
        }
    }
}

impl MemoryPoolAllocator {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            pools: HashMap::new(),
            config,
        }
    }

    /// Initialize a memory pool for a scope
    pub fn init_pool(&mut self, scope: LifetimeScope) {
        let mut pool_config = self.config.clone();
        pool_config.initial_size = match scope {
            LifetimeScope::Function => 16384,
            LifetimeScope::Loop => 4096,
            LifetimeScope::Block => 1024,
        };
        self.pools.insert(scope, pool_config);
    }

    /// Allocate from a pool
    pub fn allocate(&self, scope: LifetimeScope, size: usize) -> Result<PoolAllocation, String> {
        if let Some(pool) = self.pools.get(&scope) {
            if size <= pool.initial_size {
                Ok(PoolAllocation {
                    scope,
                    size,
                    offset: 0,
                })
            } else {
                Err(format!("Allocation {} exceeds pool size {}", size, pool.initial_size))
            }
        } else {
            Err(format!("Pool for scope {:?} not initialized", scope))
        }
    }

    /// Get pool report
    pub fn report(&self) -> MemoryPoolReport {
        let total_pools = self.pools.len();
        let total_capacity: usize = self.pools.values().map(|p| p.initial_size).sum();

        MemoryPoolReport {
            total_pools,
            total_capacity,
            pools: self.pools.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolAllocation {
    pub scope: LifetimeScope,
    pub size: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryPoolReport {
    pub total_pools: usize,
    pub total_capacity: usize,
    pub pools: HashMap<LifetimeScope, PoolConfig>,
}

/// Data Structure Layout Optimization
#[derive(Debug, Clone)]
pub struct LayoutOptimizer {
    /// Field layout analyses
    layouts: HashMap<String, StructLayout>,
}

#[derive(Debug, Clone)]
pub struct StructLayout {
    pub name: String,
    pub fields: Vec<FieldLayout>,
    pub current_size: usize,
    pub optimized_size: usize,
    pub padding_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct FieldLayout {
    pub name: String,
    pub typ: String,
    pub size: usize,
    pub alignment: usize,
    pub offset: usize,
}

impl LayoutOptimizer {
    pub fn new() -> Self {
        Self {
            layouts: HashMap::new(),
        }
    }

    /// Analyze and optimize a struct layout
    pub fn analyze_struct(&mut self, name: String, fields: Vec<FieldLayout>) {
        let current_size = fields.iter().map(|f| f.size).sum();
        let mut optimized_fields = fields.clone();
        
        // Sort by alignment (largest first) for better packing
        optimized_fields.sort_by_key(|f| std::cmp::Reverse(f.alignment));

        let mut offset = 0;
        let mut padding = 0;
        for field in &mut optimized_fields {
            // Add padding for alignment
            let align = field.alignment;
            if offset % align != 0 {
                let pad = align - (offset % align);
                padding += pad;
                offset += pad;
            }
            field.offset = offset;
            offset += field.size;
        }

        let optimized_size = offset;
        let padding_bytes = if optimized_size > 0 {
            optimized_size - (current_size)
        } else {
            0
        };

        self.layouts.insert(
            name.clone(),
            StructLayout {
                name,
                fields: optimized_fields,
                current_size,
                optimized_size,
                padding_bytes,
            },
        );
    }

    /// Get layout for a struct
    pub fn get_layout(&self, name: &str) -> Option<&StructLayout> {
        self.layouts.get(name)
    }

    /// Get all layouts
    pub fn get_all_layouts(&self) -> Vec<&StructLayout> {
        self.layouts.values().collect()
    }

    /// Generate optimization report
    pub fn report(&self) -> LayoutOptimizationReport {
        let total_saved: usize = self
            .layouts
            .values()
            .map(|l| if l.current_size > l.optimized_size {
                l.current_size - l.optimized_size
            } else {
                0
            })
            .sum();

        LayoutOptimizationReport {
            structs_analyzed: self.layouts.len(),
            total_bytes_saved: total_saved,
            layouts: self.layouts.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LayoutOptimizationReport {
    pub structs_analyzed: usize,
    pub total_bytes_saved: usize,
    pub layouts: HashMap<String, StructLayout>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_analysis_default() {
        let config = EscapeAnalysisConfig::default();
        let ea = EscapeAnalysis::new(config);
        assert_eq!(ea.escapes.len(), 0);
    }

    #[test]
    fn test_escape_status_transitions() {
        let mut ea = EscapeAnalysis::new(EscapeAnalysisConfig::default());
        ea.escapes.insert("var".to_string(), EscapeStatus::DoesNotEscape);
        assert_eq!(ea.get_escape_status("var"), Some(EscapeStatus::DoesNotEscape));
        assert!(!ea.requires_heap("var"));
    }

    #[test]
    fn test_refcount_optimization() {
        let mut opt = RefCountOptimizer::new(RefCountConfig::default());
        opt.add_operation(0, RefCountOp::Increment);
        opt.add_operation(0, RefCountOp::Decrement);
        
        let result = opt.optimize_chains();
        assert_eq!(result.pairs_eliminated, 1);
    }

    #[test]
    fn test_memory_pool_allocation() {
        let config = PoolConfig::default();
        let mut allocator = MemoryPoolAllocator::new(config);
        allocator.init_pool(LifetimeScope::Function);
        
        let result = allocator.allocate(LifetimeScope::Function, 1024);
        assert!(result.is_ok());
    }

    #[test]
    fn test_layout_optimization() {
        let mut optimizer = LayoutOptimizer::new();
        let fields = vec![
            FieldLayout {
                name: "a".to_string(),
                typ: "i64".to_string(),
                size: 8,
                alignment: 8,
                offset: 0,
            },
            FieldLayout {
                name: "b".to_string(),
                typ: "i32".to_string(),
                size: 4,
                alignment: 4,
                offset: 8,
            },
        ];
        
        optimizer.analyze_struct("TestStruct".to_string(), fields);
        assert_eq!(optimizer.get_all_layouts().len(), 1);
    }

    #[test]
    fn test_allocation_report() {
        let config = EscapeAnalysisConfig::default();
        let mut ea = EscapeAnalysis::new(config);
        ea.escapes.insert("stack_var".to_string(), EscapeStatus::DoesNotEscape);
        ea.escapes.insert("heap_var".to_string(), EscapeStatus::EscapesToMemory);
        
        let report = ea.report_allocations();
        assert_eq!(report.stack_candidates.len(), 1);
        assert_eq!(report.heap_required.len(), 1);
    }
}
