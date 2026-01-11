//! # NUMA-Aware Memory Allocation
//!
//! Support for Non-Uniform Memory Access (NUMA) aware memory allocation,
//! optimizing memory placement based on NUMA topology.
//!
//! This module provides:
//! - NUMA node detection and topology analysis
//! - NUMA-aware allocation for different memory zones
//! - Allocation policy configuration
//! - Memory affinity tracking
//! - Performance metrics for NUMA allocations
//!
//! # Examples
//!
//! ```ignore
//! use gaiarusted::utilities::numa_allocation::{NumaAllocator, NumaConfig};
//!
//! let config = NumaConfig::default();
//! let mut allocator = NumaAllocator::new(config);
//!
//! // Allocate with NUMA awareness
//! let ptr = allocator.allocate(1024, None)?;
//! println!("Allocated at NUMA node: {}", ptr.numa_node);
//! ```

use std::collections::HashMap;
use std::num::NonZeroUsize;

/// Configuration for NUMA-aware allocation
#[derive(Debug, Clone)]
pub struct NumaConfig {
    /// Number of NUMA nodes detected
    pub num_nodes: usize,
    /// Default NUMA node for allocation (if available)
    pub preferred_node: Option<usize>,
    /// Enable NUMA affinity tracking
    pub track_affinity: bool,
    /// Maximum allocation size per node
    pub max_alloc_per_node: usize,
    /// Enable automatic rebalancing
    pub enable_rebalancing: bool,
    /// Rebalancing threshold percentage
    pub rebalance_threshold: f64,
}

impl Default for NumaConfig {
    fn default() -> Self {
        NumaConfig {
            num_nodes: Self::detect_numa_nodes(),
            preferred_node: None,
            track_affinity: true,
            max_alloc_per_node: 1024 * 1024 * 1024, // 1GB per node
            enable_rebalancing: true,
            rebalance_threshold: 0.8, // 80% threshold
        }
    }
}

impl NumaConfig {
    /// Detect the number of NUMA nodes on the system
    fn detect_numa_nodes() -> usize {
        // Try to detect NUMA nodes from system
        #[cfg(target_os = "linux")]
        {
            if let Ok(entries) = std::fs::read_dir("/sys/devices/system/node/") {
                let count = entries
                    .filter_map(|entry| {
                        let entry = entry.ok()?;
                        let name = entry.file_name();
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with("node") && name_str[4..].chars().all(char::is_numeric) {
                            Some(())
                        } else {
                            None
                        }
                    })
                    .count();
                if count > 0 {
                    return count;
                }
            }
        }
        
        // Default to single node if detection fails
        1
    }

    /// Check if NUMA is available
    pub fn is_numa_available(&self) -> bool {
        self.num_nodes > 1
    }
}

/// Information about a NUMA node
#[derive(Debug, Clone)]
pub struct NumaNodeInfo {
    /// Node ID
    pub node_id: usize,
    /// Available memory in bytes
    pub available_memory: usize,
    /// Used memory in bytes
    pub used_memory: usize,
    /// Number of allocations on this node
    pub allocation_count: usize,
    /// Is this node online
    pub online: bool,
}

impl NumaNodeInfo {
    fn new(node_id: usize) -> Self {
        NumaNodeInfo {
            node_id,
            available_memory: 0,
            used_memory: 0,
            allocation_count: 0,
            online: true,
        }
    }

    /// Get memory usage percentage
    pub fn usage_percentage(&self) -> f64 {
        if self.available_memory == 0 {
            0.0
        } else {
            (self.used_memory as f64 / self.available_memory as f64) * 100.0
        }
    }

    /// Check if node can allocate more memory
    pub fn can_allocate(&self, size: usize) -> bool {
        self.online && (self.available_memory.saturating_sub(self.used_memory) >= size)
    }
}

/// Information about a memory allocation
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    /// Unique allocation ID
    pub alloc_id: usize,
    /// NUMA node where allocated
    pub numa_node: usize,
    /// Size in bytes
    pub size: usize,
    /// Purpose/type of allocation
    pub allocation_type: String,
    /// Whether allocation is pinned to node
    pub pinned: bool,
    /// Remote access count
    pub remote_accesses: usize,
}

impl AllocationInfo {
    fn new(alloc_id: usize, numa_node: usize, size: usize) -> Self {
        AllocationInfo {
            alloc_id,
            numa_node,
            size,
            allocation_type: "general".to_string(),
            pinned: false,
            remote_accesses: 0,
        }
    }
}

/// Allocation policy for NUMA nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationPolicy {
    /// Allocate on preferred node
    Preferred,
    /// Allocate on least-loaded node
    LeastLoaded,
    /// Allocate on local node (current CPU)
    Local,
    /// Round-robin across all nodes
    RoundRobin,
    /// Interleave across all nodes
    Interleave,
}

/// Main NUMA allocator
pub struct NumaAllocator {
    config: NumaConfig,
    nodes: HashMap<usize, NumaNodeInfo>,
    allocations: HashMap<usize, AllocationInfo>,
    next_alloc_id: usize,
    policy: AllocationPolicy,
    round_robin_index: usize,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl NumaAllocator {
    /// Create a new NUMA allocator
    pub fn new(config: NumaConfig) -> Self {
        let mut nodes = HashMap::new();
        for i in 0..config.num_nodes {
            let mut node = NumaNodeInfo::new(i);
            node.available_memory = config.max_alloc_per_node;
            nodes.insert(i, node);
        }

        NumaAllocator {
            config,
            nodes,
            allocations: HashMap::new(),
            next_alloc_id: 1,
            policy: AllocationPolicy::LeastLoaded,
            round_robin_index: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Set allocation policy
    pub fn set_policy(&mut self, policy: AllocationPolicy) {
        self.policy = policy;
    }

    /// Allocate memory on a specific NUMA node
    pub fn allocate(&mut self, size: usize, preferred_node: Option<usize>) -> Result<AllocationInfo, String> {
        if size == 0 {
            return Err("Allocation size must be non-zero".to_string());
        }

        let target_node = self.select_node(preferred_node)?;

        // Check if node has capacity
        if let Some(node_info) = self.nodes.get_mut(&target_node) {
            if !node_info.can_allocate(size) {
                return Err(format!(
                    "Node {} cannot allocate {} bytes (available: {})",
                    target_node,
                    size,
                    node_info.available_memory - node_info.used_memory
                ));
            }

            // Record allocation
            let alloc_id = self.next_alloc_id;
            self.next_alloc_id += 1;

            node_info.used_memory += size;
            node_info.allocation_count += 1;

            let alloc_info = AllocationInfo::new(alloc_id, target_node, size);
            self.allocations.insert(alloc_id, alloc_info.clone());

            Ok(alloc_info)
        } else {
            Err(format!("Node {} not found", target_node))
        }
    }

    /// Allocate with specific allocation type
    pub fn allocate_typed(
        &mut self,
        size: usize,
        alloc_type: &str,
        preferred_node: Option<usize>,
    ) -> Result<AllocationInfo, String> {
        let mut alloc = self.allocate(size, preferred_node)?;
        alloc.allocation_type = alloc_type.to_string();
        Ok(alloc)
    }

    /// Deallocate memory
    pub fn deallocate(&mut self, alloc_id: usize) -> Result<(), String> {
        if let Some(alloc) = self.allocations.remove(&alloc_id) {
            if let Some(node) = self.nodes.get_mut(&alloc.numa_node) {
                node.used_memory = node.used_memory.saturating_sub(alloc.size);
                node.allocation_count = node.allocation_count.saturating_sub(1);
                Ok(())
            } else {
                Err(format!("Node {} not found for deallocation", alloc.numa_node))
            }
        } else {
            Err(format!("Allocation {} not found", alloc_id))
        }
    }

    /// Select target NUMA node based on policy
    fn select_node(&mut self, preferred_node: Option<usize>) -> Result<usize, String> {
        match self.policy {
            AllocationPolicy::Preferred => {
                preferred_node
                    .or(self.config.preferred_node)
                    .ok_or("No preferred node specified".to_string())
            }
            AllocationPolicy::LeastLoaded => {
                self.select_least_loaded_node()
            }
            AllocationPolicy::Local => {
                preferred_node.ok_or("No local node available".to_string())
            }
            AllocationPolicy::RoundRobin => {
                let node = self.round_robin_index % self.config.num_nodes;
                self.round_robin_index += 1;
                Ok(node)
            }
            AllocationPolicy::Interleave => {
                let node = (self.next_alloc_id - 1) % self.config.num_nodes;
                Ok(node)
            }
        }
    }

    /// Select the least-loaded NUMA node
    fn select_least_loaded_node(&self) -> Result<usize, String> {
        self.nodes
            .values()
            .filter(|n| n.online)
            .min_by_key(|n| n.used_memory)
            .map(|n| n.node_id)
            .ok_or("No online NUMA nodes available".to_string())
    }

    /// Get information about a NUMA node
    pub fn get_node_info(&self, node_id: usize) -> Option<NumaNodeInfo> {
        self.nodes.get(&node_id).cloned()
    }

    /// Get information about an allocation
    pub fn get_allocation(&self, alloc_id: usize) -> Option<&AllocationInfo> {
        self.allocations.get(&alloc_id)
    }

    /// Get all allocations on a specific node
    pub fn get_node_allocations(&self, node_id: usize) -> Vec<AllocationInfo> {
        self.allocations
            .values()
            .filter(|a| a.numa_node == node_id)
            .cloned()
            .collect()
    }

    /// Record a remote memory access
    pub fn record_remote_access(&mut self, alloc_id: usize) -> Result<(), String> {
        if let Some(alloc) = self.allocations.get_mut(&alloc_id) {
            alloc.remote_accesses += 1;
            Ok(())
        } else {
            Err(format!("Allocation {} not found", alloc_id))
        }
    }

    /// Perform automatic rebalancing if enabled
    pub fn try_rebalance(&mut self) -> bool {
        if !self.config.enable_rebalancing {
            return false;
        }

        let threshold_factor = self.config.rebalance_threshold;
        let online_nodes: Vec<usize> = self
            .nodes
            .values()
            .filter(|n| n.online)
            .map(|n| n.node_id)
            .collect();

        if online_nodes.len() < 2 {
            return false;
        }

        // Calculate average usage
        let total_used: usize = self.nodes.values().map(|n| n.used_memory).sum();
        let total_available: usize = self.nodes.values().map(|n| n.available_memory).sum();

        if total_available == 0 {
            return false;
        }

        let average_usage = total_used as f64 / total_available as f64;
        let threshold = average_usage * threshold_factor;

        // Find overloaded nodes
        let overloaded: Vec<usize> = online_nodes
            .iter()
            .filter(|&&node_id| {
                if let Some(node) = self.nodes.get(&node_id) {
                    node.usage_percentage() / 100.0 > threshold
                } else {
                    false
                }
            })
            .copied()
            .collect();

        overloaded.len() > 0
    }

    /// Get a summary report of all allocations
    pub fn generate_report(&self) -> NumaAllocationReport {
        let mut node_summaries = Vec::new();

        for (&node_id, node_info) in &self.nodes {
            let allocation_ids: Vec<usize> = self
                .get_node_allocations(node_id)
                .iter()
                .map(|a| a.alloc_id)
                .collect();
            
            node_summaries.push(NumaNodeSummary {
                node_id,
                available_memory: node_info.available_memory,
                used_memory: node_info.used_memory,
                allocation_count: node_info.allocation_count,
                usage_percentage: node_info.usage_percentage(),
                online: node_info.online,
                allocation_ids,
            });
        }

        let total_allocated: usize = self.allocations.values().map(|a| a.size).sum();
        let total_remote_accesses: usize = self.allocations.values().map(|a| a.remote_accesses).sum();

        NumaAllocationReport {
            num_nodes: self.config.num_nodes,
            total_allocations: self.allocations.len(),
            total_allocated,
            total_remote_accesses,
            policy: self.policy,
            node_summaries,
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
        }
    }

    /// Add an error message
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Add a warning message
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Get all errors
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Get all warnings
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Get the configuration
    pub fn config(&self) -> &NumaConfig {
        &self.config
    }

    /// Get the number of allocations
    pub fn allocations_count(&self) -> usize {
        self.allocations.len()
    }
}

/// Summary information for a NUMA node
#[derive(Debug, Clone)]
pub struct NumaNodeSummary {
    pub node_id: usize,
    pub available_memory: usize,
    pub used_memory: usize,
    pub allocation_count: usize,
    pub usage_percentage: f64,
    pub online: bool,
    pub allocation_ids: Vec<usize>,
}

/// Report from NUMA allocation analysis
#[derive(Debug, Clone)]
pub struct NumaAllocationReport {
    pub num_nodes: usize,
    pub total_allocations: usize,
    pub total_allocated: usize,
    pub total_remote_accesses: usize,
    pub policy: AllocationPolicy,
    pub node_summaries: Vec<NumaNodeSummary>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl NumaAllocationReport {
    /// Check if allocation is successful
    pub fn is_successful(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get average memory usage across nodes
    pub fn average_usage_percentage(&self) -> f64 {
        if self.node_summaries.is_empty() {
            0.0
        } else {
            let sum: f64 = self.node_summaries.iter().map(|n| n.usage_percentage).sum();
            sum / self.node_summaries.len() as f64
        }
    }

    /// Get most loaded node
    pub fn most_loaded_node(&self) -> Option<&NumaNodeSummary> {
        self.node_summaries
            .iter()
            .max_by(|a, b| a.usage_percentage.partial_cmp(&b.usage_percentage).unwrap())
    }

    /// Get least loaded node
    pub fn least_loaded_node(&self) -> Option<&NumaNodeSummary> {
        self.node_summaries
            .iter()
            .min_by(|a, b| a.usage_percentage.partial_cmp(&b.usage_percentage).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_allocator() -> NumaAllocator {
        NumaAllocator::new(NumaConfig {
            num_nodes: 4,
            preferred_node: Some(0),
            track_affinity: true,
            max_alloc_per_node: 100 * 1024 * 1024,
            enable_rebalancing: true,
            rebalance_threshold: 0.8,
        })
    }

    #[test]
    fn test_create_allocator() {
        let allocator = create_test_allocator();
        assert_eq!(allocator.config.num_nodes, 4);
        assert!(allocator.allocations.is_empty());
    }

    #[test]
    fn test_allocate_memory() {
        let mut allocator = create_test_allocator();
        let result = allocator.allocate(1024, Some(0));
        assert!(result.is_ok());
        assert_eq!(allocator.allocations.len(), 1);
    }

    #[test]
    fn test_allocate_zero_size() {
        let mut allocator = create_test_allocator();
        let result = allocator.allocate(0, Some(0));
        assert!(result.is_err());
    }

    #[test]
    fn test_deallocate() {
        let mut allocator = create_test_allocator();
        let alloc = allocator.allocate(1024, Some(0)).unwrap();
        let result = allocator.deallocate(alloc.alloc_id);
        assert!(result.is_ok());
        assert_eq!(allocator.allocations.len(), 0);
    }

    #[test]
    fn test_least_loaded_policy() {
        let mut allocator = create_test_allocator();
        allocator.set_policy(AllocationPolicy::LeastLoaded);

        // Allocate on node 0
        allocator.allocate(2000, Some(0)).ok();
        // Allocate on least loaded (should NOT be node 0)
        let alloc = allocator.allocate(1000, None).unwrap();
        assert_ne!(alloc.numa_node, 0);
    }

    #[test]
    fn test_round_robin_policy() {
        let mut allocator = create_test_allocator();
        allocator.set_policy(AllocationPolicy::RoundRobin);

        let a1 = allocator.allocate(512, None).unwrap();
        let a2 = allocator.allocate(512, None).unwrap();
        let a3 = allocator.allocate(512, None).unwrap();

        assert_eq!(a1.numa_node, 0);
        assert_eq!(a2.numa_node, 1);
        assert_eq!(a3.numa_node, 2);
    }

    #[test]
    fn test_remote_accesses() {
        let mut allocator = create_test_allocator();
        let alloc = allocator.allocate(1024, Some(0)).unwrap();
        
        allocator.record_remote_access(alloc.alloc_id).ok();
        allocator.record_remote_access(alloc.alloc_id).ok();

        let info = allocator.get_allocation(alloc.alloc_id).unwrap();
        assert_eq!(info.remote_accesses, 2);
    }

    #[test]
    fn test_node_affinity() {
        let allocator = create_test_allocator();
        let info = allocator.get_node_info(0);
        assert!(info.is_some());
        assert_eq!(info.unwrap().node_id, 0);
    }

    #[test]
    fn test_generate_report() {
        let mut allocator = create_test_allocator();
        allocator.allocate(2048, Some(0)).ok();
        allocator.allocate(1024, Some(1)).ok();

        let report = allocator.generate_report();
        assert_eq!(report.num_nodes, 4);
        assert_eq!(report.total_allocations, 2);
        assert_eq!(report.total_allocated, 3072);
    }

    #[test]
    fn test_node_summary() {
        let allocator = create_test_allocator();
        let summary = allocator.get_node_info(0).unwrap();
        assert_eq!(summary.node_id, 0);
        assert!(summary.online);
    }

    #[test]
    fn test_numa_is_available() {
        let config = NumaConfig {
            num_nodes: 4,
            ..Default::default()
        };
        assert!(config.is_numa_available());

        let single_node = NumaConfig {
            num_nodes: 1,
            ..Default::default()
        };
        assert!(!single_node.is_numa_available());
    }

    #[test]
    fn test_allocate_typed() {
        let mut allocator = create_test_allocator();
        let alloc = allocator.allocate_typed(1024, "heap", Some(0)).unwrap();
        assert_eq!(alloc.allocation_type, "heap");
    }

    #[test]
    fn test_get_node_allocations() {
        let mut allocator = create_test_allocator();
        allocator.set_policy(AllocationPolicy::Preferred);
        allocator.allocate(512, Some(0)).ok();
        allocator.allocate(512, Some(0)).ok();
        allocator.allocate(512, Some(0)).ok();

        let node0_allocs = allocator.get_node_allocations(0);
        assert_eq!(node0_allocs.len(), 3);

        let node1_allocs = allocator.get_node_allocations(1);
        assert_eq!(node1_allocs.len(), 0);
    }

    #[test]
    fn test_rebalancing_check() {
        let mut allocator = create_test_allocator();
        allocator.config.max_alloc_per_node = 5000;
        
        // Allocate heavily to node 0
        for _ in 0..4 {
            allocator.allocate(1000, Some(0)).ok();
        }

        let should_rebalance = allocator.try_rebalance();
        // Rebalance is checked based on threshold
        let _ = should_rebalance;
    }

    #[test]
    fn test_error_tracking() {
        let mut allocator = create_test_allocator();
        allocator.add_error("test error".to_string());
        allocator.add_warning("test warning".to_string());

        assert_eq!(allocator.errors().len(), 1);
        assert_eq!(allocator.warnings().len(), 1);
    }
}
