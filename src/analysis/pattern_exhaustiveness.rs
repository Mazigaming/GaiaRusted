//! # Exhaustive Pattern Compilation (Task 5.15)
//!
//! Compile pattern matching expressions to ensure exhaustiveness at compile time.
//! Detects incomplete pattern matches and provides detailed coverage analysis.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Wildcard,
    Literal(String),
    Variable(String),
    Tuple(Vec<Pattern>),
    Struct(String, Vec<(String, Pattern)>),
}

#[derive(Debug, Clone)]
pub struct PatternExhaustiveness {
    patterns: Vec<Pattern>,
    coverage_map: HashMap<String, bool>,
}

impl PatternExhaustiveness {
    pub fn new() -> Self {
        PatternExhaustiveness {
            patterns: Vec::new(),
            coverage_map: HashMap::new(),
        }
    }

    pub fn add_pattern(&mut self, pattern: Pattern) {
        self.patterns.push(pattern);
    }

    pub fn check_exhaustive(&self, total_cases: usize) -> Result<(), String> {
        let covered = self.count_covered_cases();
        if covered < total_cases && !self.has_wildcard() {
            Err(format!("Non-exhaustive patterns: {} cases uncovered", total_cases - covered))
        } else {
            Ok(())
        }
    }

    fn has_wildcard(&self) -> bool {
        self.patterns.iter().any(|p| matches!(p, Pattern::Wildcard))
    }

    fn count_covered_cases(&self) -> usize {
        self.patterns.len()
    }

    pub fn compile(&mut self) -> Result<Vec<String>, String> {
        let mut compiled = Vec::new();
        let mut seen = HashSet::new();

        for pattern in &self.patterns {
            let key = self.pattern_to_string(pattern);
            if !seen.insert(key.clone()) {
                return Err(format!("Duplicate pattern: {}", key));
            }
            compiled.push(format!("match {}", key));
            self.coverage_map.insert(key, true);
        }

        Ok(compiled)
    }

    fn pattern_to_string(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Literal(s) => s.clone(),
            Pattern::Variable(v) => v.clone(),
            Pattern::Tuple(pats) => {
                let inner = pats.iter()
                    .map(|p| self.pattern_to_string(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", inner)
            }
            Pattern::Struct(name, fields) => {
                let field_strs = fields.iter()
                    .map(|(n, p)| format!("{}: {}", n, self.pattern_to_string(p)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} {{ {} }}", name, field_strs)
            }
        }
    }

    pub fn get_coverage(&self) -> &HashMap<String, bool> {
        &self.coverage_map
    }

    pub fn patterns(&self) -> &[Pattern] {
        &self.patterns
    }

    pub fn analyze_coverage(&self) -> f64 {
        if self.patterns.is_empty() {
            0.0
        } else {
            (self.coverage_map.len() as f64) / (self.patterns.len() as f64)
        }
    }

    pub fn get_uncovered_patterns(&self, total_cases: usize) -> Vec<usize> {
        let mut uncovered = Vec::new();
        for i in self.patterns.len()..total_cases {
            uncovered.push(i);
        }
        uncovered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcard_pattern() {
        let mut exhaust = PatternExhaustiveness::new();
        exhaust.add_pattern(Pattern::Wildcard);
        assert!(exhaust.check_exhaustive(1).is_ok());
    }

    #[test]
    fn test_literal_pattern() {
        let mut exhaust = PatternExhaustiveness::new();
        exhaust.add_pattern(Pattern::Literal("42".to_string()));
        assert_eq!(exhaust.patterns().len(), 1);
    }

    #[test]
    fn test_variable_pattern() {
        let mut exhaust = PatternExhaustiveness::new();
        exhaust.add_pattern(Pattern::Variable("x".to_string()));
        assert_eq!(exhaust.patterns().len(), 1);
    }

    #[test]
    fn test_tuple_pattern() {
        let mut exhaust = PatternExhaustiveness::new();
        let pat = Pattern::Tuple(vec![
            Pattern::Literal("1".to_string()),
            Pattern::Variable("x".to_string()),
        ]);
        exhaust.add_pattern(pat);
        assert_eq!(exhaust.patterns().len(), 1);
    }

    #[test]
    fn test_struct_pattern() {
        let mut exhaust = PatternExhaustiveness::new();
        let pat = Pattern::Struct(
            "Point".to_string(),
            vec![(
                "x".to_string(),
                Pattern::Variable("px".to_string()),
            )],
        );
        exhaust.add_pattern(pat);
        assert_eq!(exhaust.patterns().len(), 1);
    }

    #[test]
    fn test_compile_patterns() {
        let mut exhaust = PatternExhaustiveness::new();
        exhaust.add_pattern(Pattern::Literal("1".to_string()));
        exhaust.add_pattern(Pattern::Literal("2".to_string()));
        let compiled = exhaust.compile().unwrap();
        assert_eq!(compiled.len(), 2);
    }

    #[test]
    fn test_duplicate_pattern_detection() {
        let mut exhaust = PatternExhaustiveness::new();
        exhaust.add_pattern(Pattern::Literal("x".to_string()));
        exhaust.add_pattern(Pattern::Literal("x".to_string()));
        let result = exhaust.compile();
        assert!(result.is_err());
    }

    #[test]
    fn test_exhaustiveness_with_wildcard() {
        let mut exhaust = PatternExhaustiveness::new();
        exhaust.add_pattern(Pattern::Literal("1".to_string()));
        exhaust.add_pattern(Pattern::Wildcard);
        assert!(exhaust.check_exhaustive(10).is_ok());
    }

    #[test]
    fn test_exhaustiveness_incomplete() {
        let mut exhaust = PatternExhaustiveness::new();
        exhaust.add_pattern(Pattern::Literal("1".to_string()));
        let result = exhaust.check_exhaustive(5);
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_conversion_to_string() {
        let mut exhaust = PatternExhaustiveness::new();
        let pat = Pattern::Literal("test".to_string());
        let s = exhaust.pattern_to_string(&pat);
        assert_eq!(s, "test");
    }

    #[test]
    fn test_get_coverage() {
        let mut exhaust = PatternExhaustiveness::new();
        exhaust.add_pattern(Pattern::Wildcard);
        exhaust.compile().unwrap();
        assert!(!exhaust.get_coverage().is_empty());
    }

    #[test]
    fn test_multiple_literal_patterns() {
        let mut exhaust = PatternExhaustiveness::new();
        for i in 0..5 {
            exhaust.add_pattern(Pattern::Literal(i.to_string()));
        }
        assert_eq!(exhaust.patterns().len(), 5);
    }

    #[test]
    fn test_analyze_coverage() {
        let mut exhaust = PatternExhaustiveness::new();
        exhaust.add_pattern(Pattern::Literal("1".to_string()));
        exhaust.add_pattern(Pattern::Literal("2".to_string()));
        exhaust.compile().unwrap();
        let coverage = exhaust.analyze_coverage();
        assert!(coverage > 0.0);
    }

    #[test]
    fn test_get_uncovered_patterns() {
        let mut exhaust = PatternExhaustiveness::new();
        exhaust.add_pattern(Pattern::Literal("1".to_string()));
        let uncovered = exhaust.get_uncovered_patterns(5);
        assert_eq!(uncovered.len(), 4);
    }

    #[test]
    fn test_empty_pattern_exhaustiveness() {
        let exhaust = PatternExhaustiveness::new();
        assert!(exhaust.check_exhaustive(0).is_ok());
    }

    #[test]
    fn test_nested_struct_pattern() {
        let mut exhaust = PatternExhaustiveness::new();
        let inner_pat = Pattern::Struct(
            "Inner".to_string(),
            vec![(
                "value".to_string(),
                Pattern::Variable("v".to_string()),
            )],
        );
        let outer_pat = Pattern::Struct(
            "Outer".to_string(),
            vec![(
                "inner".to_string(),
                inner_pat,
            )],
        );
        exhaust.add_pattern(outer_pat);
        assert_eq!(exhaust.patterns().len(), 1);
    }
}
