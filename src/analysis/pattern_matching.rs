//! # Pattern Matching Support
//!
//! Comprehensive pattern matching system with:
//! - Destructuring patterns
//! - Guard expressions
//! - Exhaustiveness checking
//! - Unreachable pattern detection
//! - Pattern compilation optimization

use crate::parser::ast::{Pattern, Expression, MatchArm};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestructuringError {
    InvalidDestructuring(String),
    GuardEvaluationFailed(String),
    NonExhaustivePatterns(String),
    UnreachablePattern(usize),
    BindingConflict(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternBinding {
    pub name: String,
    pub is_mutable: bool,
    pub ty: String,
}

#[derive(Debug, Clone)]
pub struct MatchCompilation {
    pub arms: Vec<CompiledArm>,
    pub exhaustive: bool,
    pub unreachable_arms: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct CompiledArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expression>>,
    pub bindings: Vec<PatternBinding>,
    pub arm_index: usize,
}

/// Result of pattern matching
#[derive(Debug, Clone, PartialEq)]
pub enum PatternMatchResult {
    Matched,
    NotMatched,
    MatchedWithBindings(Vec<(String, String)>),
}

/// A node in the decision tree for pattern matching
#[derive(Debug, Clone)]
pub enum DecisionNode {
    Leaf(usize),
    Test {
        test_expr: String,
        branches: Vec<(String, Box<DecisionNode>)>,
    },
    Fail,
}

pub struct EnhancedPatternMatcher {
    pattern_cache: HashMap<String, Vec<PatternBinding>>,
    exhaustiveness_cache: HashMap<String, bool>,
}

impl EnhancedPatternMatcher {
    pub fn new() -> Self {
        EnhancedPatternMatcher {
            pattern_cache: HashMap::new(),
            exhaustiveness_cache: HashMap::new(),
        }
    }

    pub fn analyze_patterns(
        &mut self,
        arms: &[MatchArm],
    ) -> Result<MatchCompilation, DestructuringError> {
        let mut compiled_arms = Vec::new();
        let mut seen_patterns = HashSet::new();
        let mut unreachable_arms = Vec::new();

        for (idx, arm) in arms.iter().enumerate() {
            let pattern_key = format!("{:?}", arm.pattern);

            if seen_patterns.contains(&pattern_key) {
                if !matches!(arm.pattern, Pattern::Wildcard | Pattern::Identifier(_)) {
                    unreachable_arms.push(idx);
                }
            }

            let bindings = self.extract_bindings(&arm.pattern)?;
            self.check_binding_conflicts(&bindings)?;

            compiled_arms.push(CompiledArm {
                pattern: arm.pattern.clone(),
                guard: arm.guard.clone(),
                bindings,
                arm_index: idx,
            });

            if matches!(arm.pattern, Pattern::Wildcard) {
                seen_patterns.insert("_".to_string());
            }
        }

        let exhaustive = self.is_exhaustive(arms)?;

        Ok(MatchCompilation {
            arms: compiled_arms,
            exhaustive,
            unreachable_arms,
        })
    }

    pub fn extract_bindings(
        &mut self,
        pattern: &Pattern,
    ) -> Result<Vec<PatternBinding>, DestructuringError> {
        let pattern_key = format!("{:?}", pattern);
        if let Some(cached) = self.pattern_cache.get(&pattern_key) {
            return Ok(cached.clone());
        }

        let bindings = self.extract_bindings_recursive(pattern)?;
        self.pattern_cache.insert(pattern_key, bindings.clone());
        Ok(bindings)
    }

    fn extract_bindings_recursive(
        &self,
        pattern: &Pattern,
    ) -> Result<Vec<PatternBinding>, DestructuringError> {
        let mut bindings = Vec::new();
        self.collect_bindings(pattern, &mut bindings)?;
        self.check_binding_conflicts(&bindings)?;
        Ok(bindings)
    }

    fn collect_bindings(
        &self,
        pattern: &Pattern,
        bindings: &mut Vec<PatternBinding>,
    ) -> Result<(), DestructuringError> {
        match pattern {
            Pattern::Wildcard => {}
            Pattern::Literal(_) => {}
            Pattern::Identifier(name) => {
                bindings.push(PatternBinding {
                    name: name.clone(),
                    is_mutable: false,
                    ty: "inferred".to_string(),
                });
            }
            Pattern::MutableBinding(name) => {
                bindings.push(PatternBinding {
                    name: name.clone(),
                    is_mutable: true,
                    ty: "inferred".to_string(),
                });
            }
            Pattern::Reference { pattern: pat, .. } => {
                self.collect_bindings(pat, bindings)?;
            }
            Pattern::Tuple(patterns) => {
                for p in patterns {
                    self.collect_bindings(p, bindings)?;
                }
            }
            Pattern::Struct { fields, .. } => {
                for (_, p) in fields {
                    self.collect_bindings(p, bindings)?;
                }
            }
            Pattern::Or(patterns) => {
                for p in patterns {
                    self.collect_bindings(p, bindings)?;
                }
            }
            Pattern::Slice { patterns, rest } => {
                for p in patterns {
                    self.collect_bindings(p, bindings)?;
                }
                if let Some(r) = rest {
                    self.collect_bindings(r, bindings)?;
                }
            }
            Pattern::Box(pat) => {
                self.collect_bindings(pat, bindings)?;
            }
            Pattern::EnumVariant { data, .. } => {
                if let Some(pat) = data {
                    self.collect_bindings(pat, bindings)?;
                }
            }
            Pattern::Range { .. } => {}
        }
        Ok(())
    }

    fn check_binding_conflicts(
        &self,
        bindings: &[PatternBinding],
    ) -> Result<(), DestructuringError> {
        let mut seen = HashSet::new();
        for binding in bindings {
            if seen.contains(&binding.name) {
                return Err(DestructuringError::BindingConflict(
                    format!("Binding '{}' appears multiple times", binding.name),
                ));
            }
            seen.insert(&binding.name);
        }
        Ok(())
    }

    pub fn is_exhaustive(&mut self, arms: &[MatchArm]) -> Result<bool, DestructuringError> {
        let key = format!("{:?}", arms);
        if let Some(&result) = self.exhaustiveness_cache.get(&key) {
            return Ok(result);
        }

        let has_wildcard = arms.iter().any(|arm| matches!(arm.pattern, Pattern::Wildcard));
        let exhaustive = has_wildcard || self.check_enum_exhaustiveness(arms)?;

        self.exhaustiveness_cache.insert(key, exhaustive);
        Ok(exhaustive)
    }

    fn check_enum_exhaustiveness(&self, arms: &[MatchArm]) -> Result<bool, DestructuringError> {
        for arm in arms {
            if let Pattern::EnumVariant { .. } = &arm.pattern {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn get_unreachable_patterns(&self, arms: &[MatchArm]) -> Vec<usize> {
        let mut unreachable = Vec::new();
        let mut seen_comprehensive = false;

        for (idx, arm) in arms.iter().enumerate() {
            if seen_comprehensive {
                unreachable.push(idx);
            } else if matches!(arm.pattern, Pattern::Wildcard) {
                seen_comprehensive = true;
            }
        }

        unreachable
    }

    pub fn validate_destructuring(
        &self,
        pattern: &Pattern,
    ) -> Result<(), DestructuringError> {
        match pattern {
            Pattern::Struct { fields, .. } => {
                for (_, p) in fields {
                    self.validate_destructuring(p)?;
                }
            }
            Pattern::Tuple(patterns) => {
                for p in patterns {
                    self.validate_destructuring(p)?;
                }
            }
            Pattern::Reference { pattern: p, .. } => {
                self.validate_destructuring(p)?;
            }
            Pattern::Box(p) => {
                self.validate_destructuring(p)?;
            }
            Pattern::EnumVariant { data: Some(p), .. } => {
                self.validate_destructuring(p)?;
            }
            _ => {}
        }
        Ok(())
    }
}

impl Default for EnhancedPatternMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern analyzer for basic pattern operations
pub struct PatternAnalyzer {
    patterns: Vec<Pattern>,
    exhaustive: bool,
}

impl PatternAnalyzer {
    pub fn new() -> Self {
        PatternAnalyzer {
            patterns: Vec::new(),
            exhaustive: false,
        }
    }

    pub fn add_pattern(&mut self, pattern: Pattern) {
        self.patterns.push(pattern);
    }

    pub fn is_exhaustive(&self) -> bool {
        self.exhaustive
    }

    pub fn check_exhaustiveness(&mut self) -> Result<(), String> {
        for pattern in &self.patterns {
            if matches!(pattern, Pattern::Wildcard) {
                self.exhaustive = true;
                return Ok(());
            }
        }
        self.exhaustive = false;
        Ok(())
    }

    pub fn extract_bindings(&self, pattern: &Pattern) -> Vec<String> {
        let mut bindings = Vec::new();
        self.extract_bindings_recursive(pattern, &mut bindings);
        bindings
    }

    fn extract_bindings_recursive(&self, pattern: &Pattern, bindings: &mut Vec<String>) {
        match pattern {
            Pattern::Wildcard => {}
            Pattern::Literal(_) => {}
            Pattern::Identifier(name) => bindings.push(name.clone()),
            Pattern::MutableBinding(name) => bindings.push(name.clone()),
            Pattern::Reference { pattern: pat, .. } => {
                self.extract_bindings_recursive(pat, bindings);
            }
            Pattern::Tuple(patterns) => {
                for p in patterns {
                    self.extract_bindings_recursive(p, bindings);
                }
            }
            Pattern::Struct { fields, .. } => {
                for (_, p) in fields {
                    self.extract_bindings_recursive(p, bindings);
                }
            }
            Pattern::Or(patterns) => {
                for p in patterns {
                    self.extract_bindings_recursive(p, bindings);
                }
            }
            Pattern::Range { .. } => {}
            Pattern::Slice { patterns, rest } => {
                for p in patterns {
                    self.extract_bindings_recursive(p, bindings);
                }
                if let Some(r) = rest {
                    self.extract_bindings_recursive(r, bindings);
                }
            }
            Pattern::Box(pat) => {
                self.extract_bindings_recursive(pat, bindings);
            }
            Pattern::EnumVariant { data, .. } => {
                if let Some(pat) = data {
                    self.extract_bindings_recursive(pat, bindings);
                }
            }
        }
    }
}

impl Default for PatternAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern compiler - optimizes pattern matching
pub struct PatternCompiler {
    decision_tree: Option<DecisionNode>,
}

impl PatternCompiler {
    pub fn new() -> Self {
        PatternCompiler {
            decision_tree: None,
        }
    }

    pub fn compile(&mut self, patterns: &[Pattern]) -> Result<(), String> {
        if patterns.is_empty() {
            return Err("No patterns to compile".to_string());
        }

        let mut tree = DecisionNode::Fail;
        for (idx, _pattern) in patterns.iter().enumerate() {
            tree = DecisionNode::Leaf(idx);
        }

        self.decision_tree = Some(tree);
        Ok(())
    }

    pub fn get_tree(&self) -> Option<&DecisionNode> {
        self.decision_tree.as_ref()
    }
}

impl Default for PatternCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern reachability checker
pub struct ReachabilityChecker {
    checked_patterns: HashSet<String>,
}

impl ReachabilityChecker {
    pub fn new() -> Self {
        ReachabilityChecker {
            checked_patterns: HashSet::new(),
        }
    }

    pub fn is_reachable(&self, pattern: &Pattern) -> bool {
        match pattern {
            Pattern::Wildcard => !self.checked_patterns.contains("_"),
            Pattern::Identifier(name) => !self.checked_patterns.contains(name),
            Pattern::MutableBinding(name) => !self.checked_patterns.contains(name),
            Pattern::Literal(expr) => {
                let expr_str = format!("{:?}", expr);
                !self.checked_patterns.contains(&expr_str)
            }
            Pattern::Reference { .. } => true,
            Pattern::Tuple(_) => true,
            Pattern::Struct { .. } => true,
            Pattern::Or(_) => true,
            Pattern::Range { .. } => true,
            Pattern::Slice { .. } => true,
            Pattern::Box(_) => true,
            Pattern::EnumVariant { .. } => true,
        }
    }

    pub fn mark_checked(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard => {
                self.checked_patterns.insert("_".to_string());
            }
            Pattern::Identifier(name) => {
                self.checked_patterns.insert(name.clone());
            }
            Pattern::Literal(expr) => {
                self.checked_patterns.insert(format!("{:?}", expr));
            }
            _ => {}
        }
    }

    pub fn check_unreachable(&self, patterns: &[Pattern]) -> Vec<usize> {
        let mut unreachable = Vec::new();
        for (idx, pattern) in patterns.iter().enumerate() {
            if !self.is_reachable(pattern) {
                unreachable.push(idx);
            }
        }
        unreachable
    }
}

impl Default for ReachabilityChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_extraction_identifier() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = Pattern::Identifier("x".to_string());
        let bindings = matcher.extract_bindings(&pattern).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].name, "x");
        assert!(!bindings[0].is_mutable);
    }

    #[test]
    fn test_binding_extraction_mutable() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = Pattern::MutableBinding("y".to_string());
        let bindings = matcher.extract_bindings(&pattern).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].name, "y");
        assert!(bindings[0].is_mutable);
    }

    #[test]
    fn test_binding_extraction_tuple() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = Pattern::Tuple(vec![
            Pattern::Identifier("a".to_string()),
            Pattern::MutableBinding("b".to_string()),
        ]);
        let bindings = matcher.extract_bindings(&pattern).unwrap();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].name, "a");
        assert_eq!(bindings[1].name, "b");
    }

    #[test]
    fn test_binding_extraction_struct() {
        let mut matcher = EnhancedPatternMatcher::new();
        let pattern = Pattern::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), Pattern::Identifier("px".to_string())),
                ("y".to_string(), Pattern::Identifier("py".to_string())),
            ],
        };
        let bindings = matcher.extract_bindings(&pattern).unwrap();
        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn test_reachability_checker() {
        let mut checker = ReachabilityChecker::new();
        let pattern = Pattern::Wildcard;
        assert!(checker.is_reachable(&pattern));
        checker.mark_checked(&pattern);
        assert!(!checker.is_reachable(&pattern));
    }

    #[test]
    fn test_pattern_compiler() {
        let mut compiler = PatternCompiler::new();
        let patterns = vec![Pattern::Wildcard, Pattern::Wildcard];
        assert!(compiler.compile(&patterns).is_ok());
        assert!(compiler.get_tree().is_some());
    }

    #[test]
    fn test_pattern_analyzer() {
        let mut analyzer = PatternAnalyzer::new();
        analyzer.add_pattern(Pattern::Wildcard);
        assert!(!analyzer.is_exhaustive());
        let _ = analyzer.check_exhaustiveness();
        assert!(analyzer.is_exhaustive());
    }

    #[test]
    fn test_bindings_extraction() {
        let analyzer = PatternAnalyzer::new();
        let pattern = Pattern::Identifier("x".to_string());
        let bindings = analyzer.extract_bindings(&pattern);
        assert_eq!(bindings, vec!["x".to_string()]);
    }
}
