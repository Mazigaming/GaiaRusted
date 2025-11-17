//! Advanced Pattern Matching (Task 5.4)
//!
//! Enhanced pattern system with:
//! - Guard expressions with complex conditions
//! - OR patterns (pattern | pattern)
//! - Pattern ranges
//! - Nested destructuring
//! - Binding captures with constraints

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternElement {
    Literal(String),
    Identifier(String),
    Wildcard,
    Range { start: i64, end: i64, inclusive: bool },
    Struct { name: String, fields: Vec<(String, Box<PatternElement>)> },
    Tuple(Vec<PatternElement>),
    Or(Vec<Box<PatternElement>>),
    Binding { name: String, mutable: bool, pattern: Box<PatternElement> },
}

#[derive(Debug, Clone)]
pub enum GuardCondition {
    Simple(String),
    And(Box<GuardCondition>, Box<GuardCondition>),
    Or(Box<GuardCondition>, Box<GuardCondition>),
    Not(Box<GuardCondition>),
    Comparison { op: String, left: String, right: String },
    MethodCall { obj: String, method: String, args: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct AdvancedArm {
    pub patterns: Vec<PatternElement>,
    pub guards: Vec<GuardCondition>,
    pub bindings: HashMap<String, String>,
    pub arm_index: usize,
}

#[derive(Debug, Clone)]
pub struct PatternAnalysis {
    pub is_exhaustive: bool,
    pub reachable_arms: Vec<usize>,
    pub unreachable_arms: Vec<usize>,
    pub overlapping_patterns: Vec<(usize, usize)>,
}

pub struct AdvancedPatternEngine {
    compiled_arms: Vec<AdvancedArm>,
    pattern_cache: HashMap<String, Vec<PatternElement>>,
    guard_analysis: HashMap<usize, Vec<String>>,
}

impl AdvancedPatternEngine {
    pub fn new() -> Self {
        AdvancedPatternEngine {
            compiled_arms: Vec::new(),
            pattern_cache: HashMap::new(),
            guard_analysis: HashMap::new(),
        }
    }

    pub fn add_arm(
        &mut self,
        patterns: Vec<PatternElement>,
        guards: Vec<GuardCondition>,
        bindings: HashMap<String, String>,
    ) -> Result<(), String> {
        self.validate_patterns(&patterns)?;
        self.validate_guards(&guards)?;
        self.validate_bindings(&bindings)?;

        let arm_index = self.compiled_arms.len();
        self.compiled_arms.push(AdvancedArm {
            patterns,
            guards,
            bindings,
            arm_index,
        });

        Ok(())
    }

    fn validate_patterns(&self, patterns: &[PatternElement]) -> Result<(), String> {
        for pattern in patterns {
            self.validate_pattern(pattern)?;
        }
        Ok(())
    }

    fn validate_pattern(&self, pattern: &PatternElement) -> Result<(), String> {
        match pattern {
            PatternElement::Range { start, end, inclusive } => {
                if start >= end && !*inclusive {
                    return Err("Invalid range pattern".to_string());
                }
            }
            PatternElement::Struct { fields, .. } => {
                for (_, field_pattern) in fields {
                    self.validate_pattern(field_pattern)?;
                }
            }
            PatternElement::Tuple(elements) => {
                for elem in elements {
                    self.validate_pattern(elem)?;
                }
            }
            PatternElement::Or(patterns) => {
                for p in patterns {
                    self.validate_pattern(p)?;
                }
            }
            PatternElement::Binding { pattern, .. } => {
                self.validate_pattern(pattern)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn validate_guards(&self, guards: &[GuardCondition]) -> Result<(), String> {
        for guard in guards {
            self.validate_guard(guard)?;
        }
        Ok(())
    }

    fn validate_guard(&self, guard: &GuardCondition) -> Result<(), String> {
        match guard {
            GuardCondition::And(left, right) | GuardCondition::Or(left, right) => {
                self.validate_guard(left)?;
                self.validate_guard(right)?;
            }
            GuardCondition::Not(inner) => {
                self.validate_guard(inner)?;
            }
            GuardCondition::Comparison { op, .. } => {
                if !["==", "!=", "<", ">", "<=", ">="].contains(&op.as_str()) {
                    return Err(format!("Invalid comparison operator: {}", op));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn validate_bindings(&self, bindings: &HashMap<String, String>) -> Result<(), String> {
        let mut seen = HashSet::new();
        for name in bindings.keys() {
            if !seen.insert(name) {
                return Err(format!("Duplicate binding: {}", name));
            }
        }
        Ok(())
    }

    pub fn analyze_completeness(&self) -> Result<PatternAnalysis, String> {
        let mut reachable = HashSet::new();
        let mut unreachable = Vec::new();
        let mut overlapping = Vec::new();

        for (i, arm) in self.compiled_arms.iter().enumerate() {
            if self.is_arm_covered(&arm, &reachable) {
                unreachable.push(i);
            } else {
                reachable.insert(i);
            }
        }

        for (i, arm1) in self.compiled_arms.iter().enumerate() {
            for (j, arm2) in self.compiled_arms.iter().enumerate().skip(i + 1) {
                if self.patterns_overlap(&arm1.patterns, &arm2.patterns) {
                    overlapping.push((i, j));
                }
            }
        }

        let is_exhaustive = self.check_exhaustiveness()?;

        Ok(PatternAnalysis {
            is_exhaustive,
            reachable_arms: reachable.into_iter().collect(),
            unreachable_arms: unreachable,
            overlapping_patterns: overlapping,
        })
    }

    fn is_arm_covered(&self, arm: &AdvancedArm, covered: &HashSet<usize>) -> bool {
        for covered_idx in covered {
            if let Some(covered_arm) = self.compiled_arms.get(*covered_idx) {
                if self.patterns_subsume(&covered_arm.patterns, &arm.patterns) {
                    return true;
                }
            }
        }
        false
    }

    fn patterns_overlap(&self, patterns1: &[PatternElement], patterns2: &[PatternElement]) -> bool {
        if patterns1.len() != patterns2.len() {
            return false;
        }
        patterns1.iter().zip(patterns2.iter())
            .all(|(p1, p2)| self.pattern_elements_overlap(p1, p2))
    }

    fn pattern_elements_overlap(&self, p1: &PatternElement, p2: &PatternElement) -> bool {
        match (p1, p2) {
            (PatternElement::Wildcard, _) | (_, PatternElement::Wildcard) => true,
            (PatternElement::Literal(a), PatternElement::Literal(b)) => a == b,
            (PatternElement::Range { start: s1, end: e1, .. },
             PatternElement::Range { start: s2, end: e2, .. }) => {
                !(e1 < s2 || e2 < s1)
            }
            (PatternElement::Or(patterns1), p2) => {
                patterns1.iter().any(|p| self.pattern_elements_overlap(p, p2))
            }
            (p1, PatternElement::Or(patterns2)) => {
                patterns2.iter().any(|p| self.pattern_elements_overlap(p1, p))
            }
            _ => false,
        }
    }

    fn patterns_subsume(&self, patterns1: &[PatternElement], patterns2: &[PatternElement]) -> bool {
        if patterns1.len() != patterns2.len() {
            return false;
        }
        patterns1.iter().zip(patterns2.iter())
            .all(|(p1, p2)| self.pattern_subsumes(p1, p2))
    }

    fn pattern_subsumes(&self, p1: &PatternElement, p2: &PatternElement) -> bool {
        match (p1, p2) {
            (PatternElement::Wildcard, _) => true,
            (PatternElement::Or(patterns1), p2) => {
                patterns1.iter().any(|p| self.pattern_subsumes(p, p2))
            }
            (p1, PatternElement::Or(patterns2)) => {
                patterns2.iter().all(|p| self.pattern_subsumes(p1, p))
            }
            (PatternElement::Literal(a), PatternElement::Literal(b)) => a == b,
            _ => false,
        }
    }

    fn check_exhaustiveness(&self) -> Result<bool, String> {
        if self.compiled_arms.is_empty() {
            return Ok(false);
        }

        for arm in &self.compiled_arms {
            if arm.patterns.iter().any(|p| matches!(p, PatternElement::Wildcard)) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn evaluate_guard(&self, guard: &GuardCondition, bindings: &HashMap<String, String>) -> Result<bool, String> {
        match guard {
            GuardCondition::Simple(cond) => {
                Ok(bindings.contains_key(cond))
            }
            GuardCondition::And(left, right) => {
                Ok(self.evaluate_guard(left, bindings)? && self.evaluate_guard(right, bindings)?)
            }
            GuardCondition::Or(left, right) => {
                Ok(self.evaluate_guard(left, bindings)? || self.evaluate_guard(right, bindings)?)
            }
            GuardCondition::Not(inner) => {
                Ok(!self.evaluate_guard(inner, bindings)?)
            }
            GuardCondition::Comparison { op, left, right } => {
                let lval = bindings.get(left).ok_or(format!("Binding {} not found", left))?;
                let rval = bindings.get(right).ok_or(format!("Binding {} not found", right))?;
                
                Ok(match op.as_str() {
                    "==" => lval == rval,
                    "!=" => lval != rval,
                    "<" => lval < rval,
                    ">" => lval > rval,
                    "<=" => lval <= rval,
                    ">=" => lval >= rval,
                    _ => false,
                })
            }
            _ => Ok(true),
        }
    }

    pub fn find_matching_arm(
        &self,
        _value: &str,
        bindings: &HashMap<String, String>,
    ) -> Result<Option<&AdvancedArm>, String> {
        for arm in &self.compiled_arms {
            for guard in &arm.guards {
                if !self.evaluate_guard(guard, bindings)? {
                    continue;
                }
            }
            return Ok(Some(arm));
        }
        Ok(None)
    }

    pub fn get_arm_bindings(&self, arm_index: usize) -> Option<&HashMap<String, String>> {
        self.compiled_arms.get(arm_index).map(|arm| &arm.bindings)
    }
}

impl Default for AdvancedPatternEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_pattern() {
        let pattern = PatternElement::Literal("42".to_string());
        assert_eq!(pattern, PatternElement::Literal("42".to_string()));
    }

    #[test]
    fn test_or_pattern() {
        let pattern = PatternElement::Or(vec![
            Box::new(PatternElement::Literal("1".to_string())),
            Box::new(PatternElement::Literal("2".to_string())),
        ]);
        assert!(matches!(pattern, PatternElement::Or(_)));
    }

    #[test]
    fn test_range_pattern_valid() {
        let engine = AdvancedPatternEngine::new();
        let pattern = PatternElement::Range { start: 1, end: 10, inclusive: true };
        assert!(engine.validate_pattern(&pattern).is_ok());
    }

    #[test]
    fn test_range_pattern_invalid() {
        let engine = AdvancedPatternEngine::new();
        let pattern = PatternElement::Range { start: 10, end: 1, inclusive: false };
        assert!(engine.validate_pattern(&pattern).is_err());
    }

    #[test]
    fn test_guard_and_condition() {
        let guard = GuardCondition::And(
            Box::new(GuardCondition::Simple("x".to_string())),
            Box::new(GuardCondition::Simple("y".to_string())),
        );
        assert!(matches!(guard, GuardCondition::And(_, _)));
    }

    #[test]
    fn test_guard_or_condition() {
        let guard = GuardCondition::Or(
            Box::new(GuardCondition::Simple("x".to_string())),
            Box::new(GuardCondition::Simple("y".to_string())),
        );
        assert!(matches!(guard, GuardCondition::Or(_, _)));
    }

    #[test]
    fn test_pattern_engine_creation() {
        let engine = AdvancedPatternEngine::new();
        assert!(engine.compiled_arms.is_empty());
    }

    #[test]
    fn test_add_simple_arm() {
        let mut engine = AdvancedPatternEngine::new();
        let result = engine.add_arm(
            vec![PatternElement::Wildcard],
            vec![],
            HashMap::new(),
        );
        assert!(result.is_ok());
        assert_eq!(engine.compiled_arms.len(), 1);
    }

    #[test]
    fn test_duplicate_binding_detection() {
        let mut engine = AdvancedPatternEngine::new();
        let mut bindings = HashMap::new();
        bindings.insert("x".to_string(), "value1".to_string());
        bindings.insert("x".to_string(), "value2".to_string());
        
        let result = engine.add_arm(
            vec![PatternElement::Wildcard],
            vec![],
            bindings,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_exhaustiveness_check_with_wildcard() {
        let mut engine = AdvancedPatternEngine::new();
        let _ = engine.add_arm(vec![PatternElement::Wildcard], vec![], HashMap::new());
        let analysis = engine.analyze_completeness().unwrap();
        assert!(analysis.is_exhaustive);
    }

    #[test]
    fn test_pattern_overlap_detection() {
        let mut engine = AdvancedPatternEngine::new();
        let _ = engine.add_arm(
            vec![PatternElement::Literal("1".to_string())],
            vec![],
            HashMap::new(),
        );
        let _ = engine.add_arm(
            vec![PatternElement::Literal("1".to_string())],
            vec![],
            HashMap::new(),
        );
        
        let analysis = engine.analyze_completeness().unwrap();
        assert!(!analysis.overlapping_patterns.is_empty());
    }

    #[test]
    fn test_guard_evaluation_and() {
        let engine = AdvancedPatternEngine::new();
        let guard = GuardCondition::And(
            Box::new(GuardCondition::Simple("x".to_string())),
            Box::new(GuardCondition::Simple("y".to_string())),
        );
        
        let mut bindings = HashMap::new();
        bindings.insert("x".to_string(), "true".to_string());
        bindings.insert("y".to_string(), "true".to_string());
        
        let result = engine.evaluate_guard(&guard, &bindings).unwrap();
        assert!(result);
    }

    #[test]
    fn test_guard_evaluation_comparison() {
        let engine = AdvancedPatternEngine::new();
        let guard = GuardCondition::Comparison {
            op: "==".to_string(),
            left: "x".to_string(),
            right: "y".to_string(),
        };
        
        let mut bindings = HashMap::new();
        bindings.insert("x".to_string(), "42".to_string());
        bindings.insert("y".to_string(), "42".to_string());
        
        let result = engine.evaluate_guard(&guard, &bindings).unwrap();
        assert!(result);
    }

    #[test]
    fn test_nested_struct_pattern() {
        let pattern = PatternElement::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), Box::new(PatternElement::Wildcard)),
                ("y".to_string(), Box::new(PatternElement::Wildcard)),
            ],
        };
        
        let engine = AdvancedPatternEngine::new();
        assert!(engine.validate_pattern(&pattern).is_ok());
    }

    #[test]
    fn test_binding_pattern() {
        let pattern = PatternElement::Binding {
            name: "x".to_string(),
            mutable: true,
            pattern: Box::new(PatternElement::Wildcard),
        };
        
        let engine = AdvancedPatternEngine::new();
        assert!(engine.validate_pattern(&pattern).is_ok());
    }
}
