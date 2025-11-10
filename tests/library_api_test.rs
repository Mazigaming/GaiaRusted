//! Integration tests for enhanced library API (v0.0.3)

#[cfg(test)]
mod library_api_tests {
    use gaiarusted::library_api::{CompilerBuilder, CompilationMetrics, DefaultHandler};
    use std::path::PathBuf;

    #[test]
    fn test_compiler_builder_creation() {
        let builder = CompilerBuilder::new();
        let config = builder.build();
        
        assert!(config.source_files.is_empty());
        assert!(!config.is_verbose());
        assert!(!config.is_optimized());
    }

    #[test]
    fn test_compiler_builder_verbose() {
        let config = CompilerBuilder::new()
            .verbose(true)
            .build();

        assert!(config.is_verbose());
    }

    #[test]
    fn test_compiler_builder_optimize() {
        let config = CompilerBuilder::new()
            .optimize(true)
            .build();

        assert!(config.is_optimized());
    }

    #[test]
    fn test_compiler_builder_output_format() {
        let config = CompilerBuilder::new()
            .format("object")
            .build();

        assert_eq!(config.output_format, "object");
    }

    #[test]
    fn test_compiler_builder_chaining() {
        let config = CompilerBuilder::new()
            .verbose(true)
            .optimize(true)
            .format("executable")
            .output("bin/output")
            .build();

        assert!(config.is_verbose());
        assert!(config.is_optimized());
        assert_eq!(config.output_format, "executable");
        assert_eq!(config.output_path, PathBuf::from("bin/output"));
    }

    #[test]
    fn test_compilation_metrics_defaults() {
        let metrics = CompilationMetrics::default();
        
        assert_eq!(metrics.total_time_ms, 0);
        assert_eq!(metrics.lexer_time_ms, 0);
        assert_eq!(metrics.parser_time_ms, 0);
        assert_eq!(metrics.typechecker_time_ms, 0);
        assert_eq!(metrics.codegen_time_ms, 0);
    }

    #[test]
    fn test_compilation_metrics_phase_breakdown() {
        let mut metrics = CompilationMetrics::default();
        metrics.total_time_ms = 100;
        metrics.lexer_time_ms = 25;
        metrics.parser_time_ms = 25;
        metrics.typechecker_time_ms = 25;
        metrics.codegen_time_ms = 25;

        let breakdown = metrics.phase_breakdown();
        assert_eq!(breakdown.len(), 4);
        assert_eq!(breakdown.get("lexer"), Some(&25.0));
        assert_eq!(breakdown.get("parser"), Some(&25.0));
    }

    #[test]
    fn test_compilation_metrics_slowest_phase() {
        let mut metrics = CompilationMetrics::default();
        metrics.lexer_time_ms = 10;
        metrics.parser_time_ms = 50;
        metrics.typechecker_time_ms = 20;
        metrics.codegen_time_ms = 30;

        let slowest = metrics.slowest_phase();
        assert_eq!(slowest, Some(("parser".to_string(), 50)));
    }

    #[test]
    fn test_compilation_metrics_slowest_phase_empty() {
        let metrics = CompilationMetrics::default();
        let slowest = metrics.slowest_phase();
        // When all phases are 0, we get whichever one max_by_key returns
        assert!(slowest.is_some());
        let (_, time) = slowest.unwrap();
        assert_eq!(time, 0);
    }

    #[test]
    fn test_default_handler_creation() {
        let _handler = DefaultHandler;
        // Handler can be created and is a valid CompilationHandler
    }

    #[test]
    fn test_compiler_config_validation_empty() {
        let config = CompilerBuilder::new().build();
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_compiler_config_output_format_validation() {
        let config = CompilerBuilder::new()
            .format("invalid_format")
            .build();
        let result = config.validate();
        assert!(result.is_err());
    }
}