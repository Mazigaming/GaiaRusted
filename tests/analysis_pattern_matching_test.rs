//! Integration tests for pattern matching engine (v0.0.3)

#[cfg(test)]
mod pattern_matching_tests {
    use gaiarusted::pattern_matching::{PatternAnalyzer, PatternCompiler, ReachabilityChecker};
    use gaiarusted::parser::ast::{Pattern, Expression};

    #[test]
    fn test_pattern_analyzer_creation() {
        let analyzer = PatternAnalyzer::new();
        assert!(!analyzer.is_exhaustive());
    }

    #[test]
    fn test_pattern_analyzer_default() {
        let analyzer = PatternAnalyzer::default();
        assert!(!analyzer.is_exhaustive());
    }

    #[test]
    fn test_pattern_analyzer_exhaustiveness_check() {
        let mut analyzer = PatternAnalyzer::new();
        analyzer.add_pattern(Pattern::Wildcard);
        let result = analyzer.check_exhaustiveness();
        assert!(result.is_ok());
        assert!(analyzer.is_exhaustive());
    }

    #[test]
    fn test_pattern_analyzer_extract_bindings() {
        let analyzer = PatternAnalyzer::new();
        let pattern = Pattern::Identifier("x".to_string());
        let bindings = analyzer.extract_bindings(&pattern);
        assert_eq!(bindings, vec!["x".to_string()]);
    }

    #[test]
    fn test_pattern_compiler_creation() {
        let compiler = PatternCompiler::new();
        assert!(compiler.get_tree().is_none());
    }

    #[test]
    fn test_pattern_compiler_default() {
        let compiler = PatternCompiler::default();
        assert!(compiler.get_tree().is_none());
    }

    #[test]
    fn test_pattern_compiler_compile() {
        let mut compiler = PatternCompiler::new();
        let patterns = vec![Pattern::Wildcard];
        let result = compiler.compile(&patterns);
        assert!(result.is_ok());
        assert!(compiler.get_tree().is_some());
    }

    #[test]
    fn test_pattern_compiler_empty_patterns_error() {
        let mut compiler = PatternCompiler::new();
        let patterns: Vec<Pattern> = vec![];
        let result = compiler.compile(&patterns);
        assert!(result.is_err());
    }

    #[test]
    fn test_reachability_checker_creation() {
        let checker = ReachabilityChecker::new();
        let pattern = Pattern::Wildcard;
        assert!(checker.is_reachable(&pattern));
    }

    #[test]
    fn test_reachability_checker_default() {
        let checker = ReachabilityChecker::default();
        let pattern = Pattern::Wildcard;
        assert!(checker.is_reachable(&pattern));
    }

    #[test]
    fn test_reachability_checker_mark_wildcard() {
        let mut checker = ReachabilityChecker::new();
        let pattern = Pattern::Wildcard;
        assert!(checker.is_reachable(&pattern));
        checker.mark_checked(&pattern);
        assert!(!checker.is_reachable(&pattern));
    }

    #[test]
    fn test_reachability_checker_mark_identifier() {
        let mut checker = ReachabilityChecker::new();
        let pattern = Pattern::Identifier("x".to_string());
        assert!(checker.is_reachable(&pattern));
        checker.mark_checked(&pattern);
        assert!(!checker.is_reachable(&pattern));
    }

    #[test]
    fn test_reachability_checker_check_unreachable() {
        let mut checker = ReachabilityChecker::new();
        let patterns = vec![
            Pattern::Literal(Expression::Integer(42)),
            Pattern::Identifier("x".to_string()),
        ];
        // Mark first literal as checked
        checker.mark_checked(&patterns[0]);
        // The second pattern should still be reachable since it's an identifier
        let unreachable = checker.check_unreachable(&patterns);
        // Only the first literal should be unreachable now
        assert_eq!(unreachable.len(), 1);
        assert_eq!(unreachable[0], 0);
    }
}