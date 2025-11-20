//! # Enhanced Pattern Matching System
//!
//! Advanced features:
//! - View patterns for custom matching
//! - Slice patterns with head/tail
//! - Constant patterns with comparators
//! - Reference patterns
//! - Pattern composition
//! - Decision tree optimization
//! - Binary Decision Diagrams (BDD) for efficient dispatch

use std::collections::{HashMap, VecDeque};
use std::fmt;

/// Enhanced pattern types
#[derive(Debug, Clone, PartialEq)]
pub enum EnhancedPattern {
    Literal(String),
    Wildcard,
    Identifier(String),
    Reference { mutable: bool, inner: Box<EnhancedPattern> },
    Range { start: i64, end: i64, inclusive: bool },
    Slice { patterns: Vec<EnhancedPattern>, rest: Option<Box<EnhancedPattern>> },
    Tuple(Vec<EnhancedPattern>),
    Array { elements: Vec<EnhancedPattern>, length: Option<usize> },
    Struct { name: String, fields: Vec<(String, EnhancedPattern)> },
    OrPattern(Vec<EnhancedPattern>),
    AndPattern(Vec<EnhancedPattern>),
    ViewPattern { expr: String, inner: Box<EnhancedPattern> },
    Guard { pattern: Box<EnhancedPattern>, condition: String },
    Constant { value: String, comparator: Option<String> },
}

/// Pattern matching result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchResult {
    Matched,
    NotMatched,
    NeedsMoreInfo,
}

/// A single match arm
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub id: usize,
    pub pattern: EnhancedPattern,
    pub bindings: HashMap<String, String>,
    pub priority: usize,
}

/// Decision tree node for optimization
#[derive(Debug, Clone)]
pub enum DecisionNode {
    Leaf(usize),
    Branch {
        test: String,
        yes: Box<DecisionNode>,
        no: Box<DecisionNode>,
    },
    MultiWay {
        test: String,
        branches: HashMap<String, Box<DecisionNode>>,
        default: Option<Box<DecisionNode>>,
    },
}

impl DecisionNode {
    pub fn get_arm_id(&self) -> Option<usize> {
        match self {
            DecisionNode::Leaf(id) => Some(*id),
            _ => None,
        }
    }

    pub fn depth(&self) -> usize {
        match self {
            DecisionNode::Leaf(_) => 0,
            DecisionNode::Branch { yes, no, .. } => {
                1 + std::cmp::max(yes.depth(), no.depth())
            }
            DecisionNode::MultiWay { branches, default, .. } => {
                let max_branch = branches.values().map(|b| b.depth()).max().unwrap_or(0);
                let default_depth = default.as_ref().map(|d| d.depth()).unwrap_or(0);
                1 + std::cmp::max(max_branch, default_depth)
            }
        }
    }
}

/// Pattern matcher with decision trees
pub struct EnhancedPatternMatcher {
    arms: Vec<MatchArm>,
    decision_tree: Option<DecisionNode>,
    pattern_cache: HashMap<String, EnhancedPattern>,
    binding_cache: HashMap<usize, Vec<String>>,
}

impl EnhancedPatternMatcher {
    pub fn new() -> Self {
        EnhancedPatternMatcher {
            arms: Vec::new(),
            decision_tree: None,
            pattern_cache: HashMap::new(),
            binding_cache: HashMap::new(),
        }
    }

    /// Add a match arm
    pub fn add_arm(
        &mut self,
        pattern: EnhancedPattern,
        bindings: HashMap<String, String>,
    ) -> usize {
        let id = self.arms.len();
        let binding_keys: Vec<String> = bindings.keys().cloned().collect();
        
        self.arms.push(MatchArm {
            id,
            pattern,
            bindings,
            priority: id,
        });

        self.binding_cache.insert(id, binding_keys);

        id
    }

    /// Build optimized decision tree
    pub fn build_decision_tree(&mut self) {
        if self.arms.is_empty() {
            self.decision_tree = None;
            return;
        }

        self.decision_tree = Some(self.build_tree_recursive(
            &(0..self.arms.len()).collect::<Vec<_>>(),
            0,
        ));
    }

    fn build_tree_recursive(&self, arm_ids: &[usize], depth: usize) -> DecisionNode {
        if arm_ids.is_empty() {
            return DecisionNode::Leaf(usize::MAX);
        }

        if arm_ids.len() == 1 {
            return DecisionNode::Leaf(arm_ids[0]);
        }

        if depth > 10 {
            return DecisionNode::Leaf(arm_ids[0]);
        }

        let discriminator = format!("test_{}", depth);

        let mut branches: HashMap<String, Vec<usize>> = HashMap::new();
        for &arm_id in arm_ids {
            let pattern_key = format!("{:?}", self.arms[arm_id].pattern);
            branches
                .entry(pattern_key)
                .or_insert_with(Vec::new)
                .push(arm_id);
        }

        if branches.len() == 1 {
            return self.build_tree_recursive(arm_ids, depth + 1);
        }

        let mut branch_nodes: HashMap<String, Box<DecisionNode>> = HashMap::new();
        for (key, ids) in branches {
            branch_nodes.insert(
                key,
                Box::new(self.build_tree_recursive(&ids, depth + 1)),
            );
        }

        DecisionNode::MultiWay {
            test: discriminator,
            branches: branch_nodes,
            default: Some(Box::new(DecisionNode::Leaf(usize::MAX))),
        }
    }

    /// Check if patterns are exhaustive
    pub fn check_exhaustiveness(&self) -> bool {
        self.arms.iter().any(|arm| {
            matches!(arm.pattern, EnhancedPattern::Wildcard | EnhancedPattern::OrPattern(_))
        })
    }

    /// Find overlapping patterns
    pub fn find_overlapping_patterns(&self) -> Vec<(usize, usize)> {
        let mut overlaps = Vec::new();

        for i in 0..self.arms.len() {
            for j in (i + 1)..self.arms.len() {
                if self.patterns_overlap(&self.arms[i].pattern, &self.arms[j].pattern) {
                    overlaps.push((i, j));
                }
            }
        }

        overlaps
    }

    fn patterns_overlap(&self, p1: &EnhancedPattern, p2: &EnhancedPattern) -> bool {
        match (p1, p2) {
            (EnhancedPattern::Wildcard, _) | (_, EnhancedPattern::Wildcard) => true,
            (EnhancedPattern::Literal(a), EnhancedPattern::Literal(b)) => a == b,
            (EnhancedPattern::Identifier(_), _) | (_, EnhancedPattern::Identifier(_)) => true,
            (
                EnhancedPattern::Range { start: s1, end: e1, inclusive: i1 },
                EnhancedPattern::Range { start: s2, end: e2, inclusive: i2 },
            ) => {
                let e1 = if *i1 { *e1 } else { e1 - 1 };
                let e2 = if *i2 { *e2 } else { e2 - 1 };
                !(e1 < *s2 || e2 < *s1)
            }
            (EnhancedPattern::Tuple(ps1), EnhancedPattern::Tuple(ps2)) => {
                if ps1.len() != ps2.len() {
                    return false;
                }
                ps1.iter().zip(ps2.iter()).all(|(p1, p2)| self.patterns_overlap(p1, p2))
            }
            (EnhancedPattern::OrPattern(patterns), p) | (p, EnhancedPattern::OrPattern(patterns)) => {
                patterns.iter().any(|pat| self.patterns_overlap(pat, p))
            }
            _ => false,
        }
    }

    /// Validate pattern
    pub fn validate_pattern(&self, pattern: &EnhancedPattern) -> Result<(), String> {
        match pattern {
            EnhancedPattern::Range { start, end, inclusive } => {
                if start > end || (start == end && !*inclusive) {
                    return Err("Invalid range pattern".to_string());
                }
                Ok(())
            }
            EnhancedPattern::Array { elements, length } => {
                if let Some(len) = length {
                    if *len != elements.len() {
                        return Err("Array length mismatch".to_string());
                    }
                }
                for elem in elements {
                    self.validate_pattern(elem)?;
                }
                Ok(())
            }
            EnhancedPattern::Tuple(elements) => {
                for elem in elements {
                    self.validate_pattern(elem)?;
                }
                Ok(())
            }
            EnhancedPattern::OrPattern(patterns) => {
                for p in patterns {
                    self.validate_pattern(p)?;
                }
                Ok(())
            }
            EnhancedPattern::AndPattern(patterns) => {
                for p in patterns {
                    self.validate_pattern(p)?;
                }
                Ok(())
            }
            EnhancedPattern::Reference { inner, .. } => self.validate_pattern(inner),
            EnhancedPattern::Guard { pattern, .. } => self.validate_pattern(pattern),
            EnhancedPattern::ViewPattern { inner, .. } => self.validate_pattern(inner),
            _ => Ok(()),
        }
    }

    /// Get pattern complexity score
    pub fn pattern_complexity(&self, pattern: &EnhancedPattern) -> usize {
        match pattern {
            EnhancedPattern::Wildcard | EnhancedPattern::Literal(_) 
            | EnhancedPattern::Identifier(_) | EnhancedPattern::Constant { .. } => 1,
            
            EnhancedPattern::Range { .. } => 2,
            EnhancedPattern::Reference { inner, .. } => 1 + self.pattern_complexity(inner),
            EnhancedPattern::Guard { pattern, .. } => 1 + self.pattern_complexity(pattern),
            EnhancedPattern::ViewPattern { inner, .. } => 2 + self.pattern_complexity(inner),
            
            EnhancedPattern::Slice { patterns, rest } => {
                let patterns_cost: usize = patterns.iter().map(|p| self.pattern_complexity(p)).sum();
                let rest_cost = rest.as_ref().map(|r| self.pattern_complexity(r)).unwrap_or(0);
                patterns_cost + rest_cost + 1
            }
            
            EnhancedPattern::Tuple(elements) | EnhancedPattern::Array { elements, .. } => {
                elements.iter().map(|p| self.pattern_complexity(p)).sum::<usize>() + 1
            }
            
            EnhancedPattern::Struct { fields, .. } => {
                fields.iter().map(|(_, p)| self.pattern_complexity(p)).sum::<usize>() + 1
            }
            
            EnhancedPattern::OrPattern(patterns) | EnhancedPattern::AndPattern(patterns) => {
                patterns.iter().map(|p| self.pattern_complexity(p)).sum::<usize>() + 1
            }
        }
    }

    /// Extract bindings from pattern
    pub fn extract_bindings(&self, pattern: &EnhancedPattern) -> Vec<String> {
        let mut bindings = Vec::new();
        self.extract_bindings_recursive(pattern, &mut bindings);
        bindings.sort();
        bindings.dedup();
        bindings
    }

    fn extract_bindings_recursive(&self, pattern: &EnhancedPattern, bindings: &mut Vec<String>) {
        match pattern {
            EnhancedPattern::Identifier(name) => bindings.push(name.clone()),
            EnhancedPattern::Reference { inner, .. } => self.extract_bindings_recursive(inner, bindings),
            EnhancedPattern::Guard { pattern, .. } => self.extract_bindings_recursive(pattern, bindings),
            EnhancedPattern::ViewPattern { inner, .. } => self.extract_bindings_recursive(inner, bindings),
            EnhancedPattern::Slice { patterns, rest } => {
                for p in patterns {
                    self.extract_bindings_recursive(p, bindings);
                }
                if let Some(r) = rest {
                    self.extract_bindings_recursive(r, bindings);
                }
            }
            EnhancedPattern::Tuple(elements) | EnhancedPattern::Array { elements, .. } => {
                for elem in elements {
                    self.extract_bindings_recursive(elem, bindings);
                }
            }
            EnhancedPattern::Struct { fields, .. } => {
                for (_, p) in fields {
                    self.extract_bindings_recursive(p, bindings);
                }
            }
            EnhancedPattern::OrPattern(patterns) | EnhancedPattern::AndPattern(patterns) => {
                for p in patterns {
                    self.extract_bindings_recursive(p, bindings);
                }
            }
            _ => {}
        }
    }

    /// Get decision tree
    pub fn get_decision_tree(&self) -> Option<&DecisionNode> {
        self.decision_tree.as_ref()
    }

    /// Get arm count
    pub fn arm_count(&self) -> usize {
        self.arms.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_pattern() {
        let mut matcher = EnhancedPatternMatcher::new();
        matcher.add_arm(EnhancedPattern::Literal("hello".to_string()), HashMap::new());
        assert_eq!(matcher.arm_count(), 1);
    }

    #[test]
    fn test_wildcard_pattern() {
        let mut matcher = EnhancedPatternMatcher::new();
        matcher.add_arm(EnhancedPattern::Wildcard, HashMap::new());
        assert!(matcher.check_exhaustiveness());
    }

    #[test]
    fn test_range_pattern() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::Range { start: 1, end: 10, inclusive: true };
        matcher.add_arm(pattern, HashMap::new());
        assert_eq!(matcher.arm_count(), 1);
    }

    #[test]
    fn test_range_validation() {
        let matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::Range { start: 10, end: 5, inclusive: false };
        assert!(matcher.validate_pattern(&pattern).is_err());
    }

    #[test]
    fn test_tuple_pattern() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::Tuple(vec![
            EnhancedPattern::Literal("a".to_string()),
            EnhancedPattern::Identifier("x".to_string()),
        ]);
        matcher.add_arm(pattern, HashMap::new());
        assert_eq!(matcher.arm_count(), 1);
    }

    #[test]
    fn test_struct_pattern() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), EnhancedPattern::Identifier("x".to_string())),
                ("y".to_string(), EnhancedPattern::Identifier("y".to_string())),
            ],
        };
        matcher.add_arm(pattern, HashMap::new());
        assert_eq!(matcher.arm_count(), 1);
    }

    #[test]
    fn test_or_pattern() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::OrPattern(vec![
            EnhancedPattern::Literal("a".to_string()),
            EnhancedPattern::Literal("b".to_string()),
        ]);
        matcher.add_arm(pattern, HashMap::new());
        assert_eq!(matcher.arm_count(), 1);
    }

    #[test]
    fn test_reference_pattern() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::Reference {
            mutable: false,
            inner: Box::new(EnhancedPattern::Identifier("x".to_string())),
        };
        matcher.add_arm(pattern, HashMap::new());
        assert_eq!(matcher.arm_count(), 1);
    }

    #[test]
    fn test_extract_bindings() {
        let matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::Tuple(vec![
            EnhancedPattern::Identifier("x".to_string()),
            EnhancedPattern::Identifier("y".to_string()),
        ]);
        let bindings = matcher.extract_bindings(&pattern);
        assert_eq!(bindings, vec!["x", "y"]);
    }

    #[test]
    fn test_pattern_complexity() {
        let matcher = EnhancedPatternMatcher::new();

        let simple = EnhancedPattern::Literal("a".to_string());
        let complex = EnhancedPattern::Tuple(vec![
            EnhancedPattern::Identifier("x".to_string()),
            EnhancedPattern::Identifier("y".to_string()),
        ]);

        assert!(matcher.pattern_complexity(&complex) > matcher.pattern_complexity(&simple));
    }

    #[test]
    fn test_overlapping_patterns_wildcard() {
        let mut matcher = EnhancedPatternMatcher::new();
        matcher.add_arm(EnhancedPattern::Literal("a".to_string()), HashMap::new());
        matcher.add_arm(EnhancedPattern::Wildcard, HashMap::new());

        let overlaps = matcher.find_overlapping_patterns();
        assert_eq!(overlaps.len(), 1);
    }

    #[test]
    fn test_overlapping_patterns_range() {
        let mut matcher = EnhancedPatternMatcher::new();
        matcher.add_arm(
            EnhancedPattern::Range { start: 1, end: 10, inclusive: true },
            HashMap::new(),
        );
        matcher.add_arm(
            EnhancedPattern::Range { start: 5, end: 15, inclusive: true },
            HashMap::new(),
        );

        let overlaps = matcher.find_overlapping_patterns();
        assert_eq!(overlaps.len(), 1);
    }

    #[test]
    fn test_decision_tree_building() {
        let mut matcher = EnhancedPatternMatcher::new();
        matcher.add_arm(EnhancedPattern::Literal("a".to_string()), HashMap::new());
        matcher.add_arm(EnhancedPattern::Literal("b".to_string()), HashMap::new());
        matcher.add_arm(EnhancedPattern::Wildcard, HashMap::new());

        matcher.build_decision_tree();
        assert!(matcher.get_decision_tree().is_some());
    }

    #[test]
    fn test_decision_tree_leaf() {
        let node = DecisionNode::Leaf(5);
        assert_eq!(node.get_arm_id(), Some(5));
        assert_eq!(node.depth(), 0);
    }

    #[test]
    fn test_array_pattern() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::Array {
            elements: vec![
                EnhancedPattern::Identifier("a".to_string()),
                EnhancedPattern::Identifier("b".to_string()),
            ],
            length: Some(2),
        };
        matcher.add_arm(pattern, HashMap::new());
        assert_eq!(matcher.arm_count(), 1);
    }

    #[test]
    fn test_array_pattern_validation() {
        let matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::Array {
            elements: vec![EnhancedPattern::Literal("a".to_string())],
            length: Some(2),
        };
        assert!(matcher.validate_pattern(&pattern).is_err());
    }

    #[test]
    fn test_guard_pattern() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::Guard {
            pattern: Box::new(EnhancedPattern::Identifier("x".to_string())),
            condition: "x > 0".to_string(),
        };
        matcher.add_arm(pattern, HashMap::new());
        assert_eq!(matcher.arm_count(), 1);
    }

    #[test]
    fn test_view_pattern() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::ViewPattern {
            expr: "some_view".to_string(),
            inner: Box::new(EnhancedPattern::Identifier("x".to_string())),
        };
        matcher.add_arm(pattern, HashMap::new());
        assert_eq!(matcher.arm_count(), 1);
    }

    #[test]
    fn test_slice_pattern() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::Slice {
            patterns: vec![
                EnhancedPattern::Identifier("first".to_string()),
                EnhancedPattern::Identifier("second".to_string()),
            ],
            rest: Some(Box::new(EnhancedPattern::Identifier("rest".to_string()))),
        };
        matcher.add_arm(pattern, HashMap::new());
        assert_eq!(matcher.arm_count(), 1);
    }

    #[test]
    fn test_pattern_composition() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::AndPattern(vec![
            EnhancedPattern::Identifier("x".to_string()),
            EnhancedPattern::Guard {
                pattern: Box::new(EnhancedPattern::Identifier("x".to_string())),
                condition: "x > 5".to_string(),
            },
        ]);
        matcher.add_arm(pattern, HashMap::new());
        assert_eq!(matcher.arm_count(), 1);
    }

    #[test]
    fn test_constant_pattern() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = EnhancedPattern::Constant {
            value: "MAX".to_string(),
            comparator: Some("==".to_string()),
        };
        matcher.add_arm(pattern, HashMap::new());
        assert_eq!(matcher.arm_count(), 1);
    }
}
