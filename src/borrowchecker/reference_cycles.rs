//! Reference cycle detection
//!
//! Detects potential circular reference patterns that could lead to memory leaks,
//! particularly with Rc<T> and other shared pointer types.

use std::collections::{HashMap, HashSet};

/// Represents a reference relationship between types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceEdge {
    /// Source type
    pub from: String,
    /// Target type
    pub to: String,
    /// Kind of reference (direct, rc, arc)
    pub kind: ReferenceKind,
}

/// Kind of reference
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    /// Direct reference or field
    Direct,
    /// Through Rc pointer
    Rc,
    /// Through Arc pointer
    Arc,
}

impl ReferenceKind {
    /// Check if this kind can form cycles
    pub fn can_form_cycle(&self) -> bool {
        matches!(self, ReferenceKind::Rc | ReferenceKind::Arc)
    }

    /// Get display name
    pub fn as_str(&self) -> &'static str {
        match self {
            ReferenceKind::Direct => "direct",
            ReferenceKind::Rc => "Rc",
            ReferenceKind::Arc => "Arc",
        }
    }
}

/// Graph for tracking references between types
#[derive(Debug)]
pub struct ReferenceGraph {
    /// Edges in the graph
    edges: Vec<ReferenceEdge>,
    /// Adjacency list for fast lookup
    adjacency: HashMap<String, Vec<String>>,
}

impl ReferenceGraph {
    /// Create a new graph
    pub fn new() -> Self {
        ReferenceGraph {
            edges: Vec::new(),
            adjacency: HashMap::new(),
        }
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, from: String, to: String, kind: ReferenceKind) {
        self.edges.push(ReferenceEdge { from: from.clone(), to: to.clone(), kind });
        
        self.adjacency
            .entry(from)
            .or_insert_with(Vec::new)
            .push(to);
    }

    /// Get all edges from a type
    pub fn get_edges_from(&self, from: &str) -> Vec<&ReferenceEdge> {
        self.edges
            .iter()
            .filter(|e| e.from == from)
            .collect()
    }

    /// Check if there's a cycle starting from a node
    pub fn has_cycle_from(&self, start: &str) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        self.dfs_has_cycle(start, &mut visited, &mut rec_stack)
    }

    fn dfs_has_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = self.adjacency.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.dfs_has_cycle(neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    /// Find all cycles in the graph
    pub fn find_all_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();

        for node in self.adjacency.keys() {
            if !visited.contains(node) {
                self.find_cycles_from(node, &mut visited, &mut cycles);
            }
        }

        cycles
    }

    fn find_cycles_from(
        &self,
        start: &str,
        _visited: &mut HashSet<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        let mut path = Vec::new();
        let mut path_set = HashSet::new();
        self.dfs_find_cycles(start, start, &mut path, &mut path_set, cycles);
    }

    fn dfs_find_cycles(
        &self,
        current: &str,
        start: &str,
        path: &mut Vec<String>,
        path_set: &mut HashSet<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        path.push(current.to_string());
        path_set.insert(current.to_string());

        if let Some(neighbors) = self.adjacency.get(current) {
            for neighbor in neighbors {
                if neighbor == start && path.len() > 1 {
                    cycles.push(path.clone());
                } else if !path_set.contains(neighbor) {
                    self.dfs_find_cycles(neighbor, start, path, path_set, cycles);
                }
            }
        }

        path.pop();
        path_set.remove(current);
    }

    /// Get all nodes in the graph
    pub fn nodes(&self) -> HashSet<String> {
        let mut nodes = HashSet::new();
        for edge in &self.edges {
            nodes.insert(edge.from.clone());
            nodes.insert(edge.to.clone());
        }
        nodes
    }

    /// Get number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// Cycle detector for potential memory leaks
pub struct CycleDetector {
    graph: ReferenceGraph,
}

impl CycleDetector {
    /// Create a new cycle detector
    pub fn new() -> Self {
        CycleDetector {
            graph: ReferenceGraph::new(),
        }
    }

    /// Register a reference between types
    pub fn add_reference(&mut self, from: String, to: String, kind: ReferenceKind) {
        self.graph.add_edge(from, to, kind);
    }

    /// Check for cycles
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        self.graph.find_all_cycles()
    }

    /// Check if specific type can form a cycle
    pub fn can_form_cycle(&self, ty: &str) -> bool {
        self.graph.has_cycle_from(ty)
    }

    /// Get all reference edges
    pub fn get_edges(&self) -> &[ReferenceEdge] {
        &self.graph.edges
    }

    /// Warn about potential cycles
    pub fn warn_potential_cycles(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let cycles = self.detect_cycles();

        for cycle in cycles {
            let cycle_str = cycle.join(" -> ");
            warnings.push(format!(
                "Potential reference cycle detected: {} (may leak memory)",
                cycle_str
            ));
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reference_edge() {
        let edge = ReferenceEdge {
            from: "A".to_string(),
            to: "B".to_string(),
            kind: ReferenceKind::Direct,
        };

        assert_eq!(edge.from, "A");
        assert_eq!(edge.to, "B");
        assert_eq!(edge.kind, ReferenceKind::Direct);
    }

    #[test]
    fn test_reference_kind_display() {
        assert_eq!(ReferenceKind::Direct.as_str(), "direct");
        assert_eq!(ReferenceKind::Rc.as_str(), "Rc");
        assert_eq!(ReferenceKind::Arc.as_str(), "Arc");
    }

    #[test]
    fn test_reference_kind_can_form_cycle() {
        assert!(!ReferenceKind::Direct.can_form_cycle());
        assert!(ReferenceKind::Rc.can_form_cycle());
        assert!(ReferenceKind::Arc.can_form_cycle());
    }

    #[test]
    fn test_graph_no_cycle() {
        let mut graph = ReferenceGraph::new();
        graph.add_edge("A".to_string(), "B".to_string(), ReferenceKind::Direct);
        graph.add_edge("B".to_string(), "C".to_string(), ReferenceKind::Direct);

        assert!(!graph.has_cycle_from("A"));
    }

    #[test]
    fn test_graph_simple_cycle() {
        let mut graph = ReferenceGraph::new();
        graph.add_edge("A".to_string(), "B".to_string(), ReferenceKind::Rc);
        graph.add_edge("B".to_string(), "A".to_string(), ReferenceKind::Rc);

        assert!(graph.has_cycle_from("A"));
    }

    #[test]
    fn test_graph_self_cycle() {
        let mut graph = ReferenceGraph::new();
        graph.add_edge("A".to_string(), "A".to_string(), ReferenceKind::Rc);

        assert!(graph.has_cycle_from("A"));
    }

    #[test]
    fn test_graph_complex_cycle() {
        let mut graph = ReferenceGraph::new();
        graph.add_edge("A".to_string(), "B".to_string(), ReferenceKind::Rc);
        graph.add_edge("B".to_string(), "C".to_string(), ReferenceKind::Rc);
        graph.add_edge("C".to_string(), "A".to_string(), ReferenceKind::Rc);

        assert!(graph.has_cycle_from("A"));
    }

    #[test]
    fn test_find_all_cycles() {
        let mut graph = ReferenceGraph::new();
        graph.add_edge("A".to_string(), "B".to_string(), ReferenceKind::Rc);
        graph.add_edge("B".to_string(), "A".to_string(), ReferenceKind::Rc);

        let cycles = graph.find_all_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_cycle_detector() {
        let mut detector = CycleDetector::new();
        detector.add_reference("A".to_string(), "B".to_string(), ReferenceKind::Rc);
        detector.add_reference("B".to_string(), "A".to_string(), ReferenceKind::Rc);

        assert!(detector.can_form_cycle("A"));
    }

    #[test]
    fn test_cycle_detector_warnings() {
        let mut detector = CycleDetector::new();
        detector.add_reference("A".to_string(), "B".to_string(), ReferenceKind::Rc);
        detector.add_reference("B".to_string(), "A".to_string(), ReferenceKind::Rc);

        let warnings = detector.warn_potential_cycles();
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("Potential reference cycle"));
    }

    #[test]
    fn test_graph_nodes() {
        let mut graph = ReferenceGraph::new();
        graph.add_edge("A".to_string(), "B".to_string(), ReferenceKind::Direct);
        graph.add_edge("B".to_string(), "C".to_string(), ReferenceKind::Direct);

        let nodes = graph.nodes();
        assert_eq!(nodes.len(), 3);
        assert!(nodes.contains("A"));
        assert!(nodes.contains("B"));
        assert!(nodes.contains("C"));
    }

    #[test]
    fn test_graph_edge_count() {
        let mut graph = ReferenceGraph::new();
        graph.add_edge("A".to_string(), "B".to_string(), ReferenceKind::Direct);
        graph.add_edge("B".to_string(), "C".to_string(), ReferenceKind::Direct);

        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn test_get_edges_from() {
        let mut graph = ReferenceGraph::new();
        graph.add_edge("A".to_string(), "B".to_string(), ReferenceKind::Direct);
        graph.add_edge("A".to_string(), "C".to_string(), ReferenceKind::Direct);

        let edges = graph.get_edges_from("A");
        assert_eq!(edges.len(), 2);
    }
}
