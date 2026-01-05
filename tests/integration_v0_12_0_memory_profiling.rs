//! Integration tests for v0.12.0 Memory Optimization & Profiling
//!
//! Tests the new memory optimization and profiling/diagnostics modules
//! introduced in v0.12.0.

#[cfg(test)]
mod v0_12_0_integration_tests {
    use gaiarusted::codegen::memory_optimization::{
        EscapeAnalysis, EscapeAnalysisConfig, EscapeStatus, RefCountOptimizer,
        RefCountConfig, RefCountOp, MemoryPoolAllocator, MemoryPoolAllocator as Allocator,
        PoolConfig, LifetimeScope, LayoutOptimizer, FieldLayout,
    };
    use gaiarusted::codegen::profiling_diagnostics::{
        PerformanceProfiler, ProfilerConfig, CoverageReporter, CoverageConfig,
        RegressionDetector, RegressionConfig, EnhancedDiagnostics, DiagnosticLevel,
        Location,
    };
    use std::time::Duration;

    // ============================================================================
    // ESCAPE ANALYSIS TESTS
    // ============================================================================

    #[test]
    fn test_escape_analysis_stack_candidates() {
        let config = EscapeAnalysisConfig {
            track_field_escapes: true,
            track_returns: true,
            track_thread_escapes: false,
        };
        let mut ea = EscapeAnalysis::new(config);
        
        // Manually simulate a local that doesn't escape
        ea.insert_escape_status("local_var".to_string(), EscapeStatus::DoesNotEscape);
        
        let report = ea.report_allocations();
        assert_eq!(report.stack_candidates.len(), 1);
        assert_eq!(report.heap_required.len(), 0);
    }

    #[test]
    fn test_escape_analysis_heap_required() {
        let config = EscapeAnalysisConfig::default();
        let mut ea = EscapeAnalysis::new(config);
        
        // Simulate a local that escapes to memory
        ea.insert_escape_status("heap_var".to_string(), EscapeStatus::EscapesToMemory);
        ea.mark_heap_required("heap_var".to_string());
        
        assert!(ea.requires_heap("heap_var"));
        let report = ea.report_allocations();
        assert_eq!(report.heap_required.len(), 1);
    }

    #[test]
    fn test_escape_analysis_return_values() {
        let config = EscapeAnalysisConfig::default();
        let mut ea = EscapeAnalysis::new(config);
        
        // Simulate locals that escape via return
        ea.insert_escape_status("return_var".to_string(), EscapeStatus::MayEscapeReturn);
        ea.insert_escape_status("stack_var".to_string(), EscapeStatus::DoesNotEscape);
        
        let report = ea.report_allocations();
        assert_eq!(report.escape_to_return.len(), 1);
        assert_eq!(report.stack_candidates.len(), 1);
    }

    // ============================================================================
    // REFERENCE COUNTING OPTIMIZATION TESTS
    // ============================================================================

    #[test]
    fn test_refcount_inc_dec_elimination() {
        let mut optimizer = RefCountOptimizer::new(RefCountConfig::default());
        optimizer.add_operation(0, RefCountOp::Increment);
        optimizer.add_operation(0, RefCountOp::Decrement);
        
        let result = optimizer.optimize_chains();
        assert_eq!(result.pairs_eliminated, 1);
        assert_eq!(result.operations_fused, 0);
    }

    #[test]
    fn test_refcount_move_semantics() {
        let optimizer = RefCountOptimizer::new(RefCountConfig {
            enable_chain_fusion: true,
            enable_move_semantics: true,
        });
        
        // Empty chain should allow move semantics
        assert!(optimizer.can_use_move_semantics(0));
    }

    #[test]
    fn test_refcount_chain_fusion() {
        let mut optimizer = RefCountOptimizer::new(RefCountConfig {
            enable_chain_fusion: true,
            enable_move_semantics: true,
        });
        
        optimizer.add_operation(0, RefCountOp::Increment);
        optimizer.add_operation(0, RefCountOp::Increment);
        
        let result = optimizer.optimize_chains();
        assert_eq!(result.operations_fused, 1);
    }

    // ============================================================================
    // MEMORY POOL ALLOCATION TESTS
    // ============================================================================

    #[test]
    fn test_memory_pool_function_scope() {
        let config = PoolConfig::default();
        let mut allocator = MemoryPoolAllocator::new(config);
        allocator.init_pool(LifetimeScope::Function);
        
        let allocation = allocator.allocate(LifetimeScope::Function, 1024);
        assert!(allocation.is_ok());
        assert_eq!(allocation.unwrap().size, 1024);
    }

    #[test]
    fn test_memory_pool_loop_scope() {
        let config = PoolConfig::default();
        let mut allocator = MemoryPoolAllocator::new(config);
        allocator.init_pool(LifetimeScope::Loop);
        
        let allocation = allocator.allocate(LifetimeScope::Loop, 512);
        assert!(allocation.is_ok());
    }

    #[test]
    fn test_memory_pool_block_scope() {
        let config = PoolConfig::default();
        let mut allocator = MemoryPoolAllocator::new(config);
        allocator.init_pool(LifetimeScope::Block);
        
        let allocation = allocator.allocate(LifetimeScope::Block, 256);
        assert!(allocation.is_ok());
    }

    #[test]
    fn test_memory_pool_uninitialized_scope() {
        let config = PoolConfig::default();
        let allocator = MemoryPoolAllocator::new(config);
        
        let allocation = allocator.allocate(LifetimeScope::Function, 1024);
        assert!(allocation.is_err());
    }

    #[test]
    fn test_memory_pool_report() {
        let config = PoolConfig::default();
        let mut allocator = MemoryPoolAllocator::new(config);
        allocator.init_pool(LifetimeScope::Function);
        allocator.init_pool(LifetimeScope::Loop);
        
        let report = allocator.report();
        assert_eq!(report.total_pools, 2);
        assert!(report.total_capacity > 0);
    }

    // ============================================================================
    // DATA STRUCTURE LAYOUT OPTIMIZATION TESTS
    // ============================================================================

    #[test]
    fn test_layout_optimizer_simple_struct() {
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
                offset: 0,
            },
        ];
        
        optimizer.analyze_struct("TestStruct".to_string(), fields);
        let layout = optimizer.get_layout("TestStruct");
        
        assert!(layout.is_some());
        let layout = layout.unwrap();
        assert_eq!(layout.fields.len(), 2);
    }

    #[test]
    fn test_layout_optimizer_report() {
        let mut optimizer = LayoutOptimizer::new();
        let fields = vec![
            FieldLayout {
                name: "x".to_string(),
                typ: "i64".to_string(),
                size: 8,
                alignment: 8,
                offset: 0,
            },
        ];
        
        optimizer.analyze_struct("Point".to_string(), fields);
        
        let report = optimizer.report();
        assert_eq!(report.structs_analyzed, 1);
    }

    // ============================================================================
    // PERFORMANCE PROFILER TESTS
    // ============================================================================

    #[test]
    fn test_profiler_basic_timing() {
        let mut profiler = PerformanceProfiler::new(ProfilerConfig::default());
        
        profiler.start_phase("test_phase");
        std::thread::sleep(Duration::from_millis(10));
        profiler.end_phase();
        
        let report = profiler.report();
        assert_eq!(report.phase_count, 1);
        assert!(report.total_time >= Duration::from_millis(10));
    }

    #[test]
    fn test_profiler_multiple_phases() {
        let mut profiler = PerformanceProfiler::new(ProfilerConfig::default());
        
        profiler.start_phase("phase1");
        std::thread::sleep(Duration::from_millis(5));
        profiler.end_phase();
        
        profiler.start_phase("phase2");
        std::thread::sleep(Duration::from_millis(10));
        profiler.end_phase();
        
        let report = profiler.report();
        assert_eq!(report.phase_count, 2);
    }

    #[test]
    fn test_profiler_average_calculation() {
        let mut profiler = PerformanceProfiler::new(ProfilerConfig::default());
        
        // Record multiple timings for same phase
        for _ in 0..3 {
            profiler.start_phase("repeated");
            std::thread::sleep(Duration::from_millis(5));
            profiler.end_phase();
        }
        
        if let Some(timing) = profiler.get_phase_timing("repeated") {
            assert_eq!(timing.count, 3);
            assert!(timing.average > Duration::ZERO);
        }
    }

    #[test]
    fn test_profiler_disable() {
        let mut profiler = PerformanceProfiler::new(ProfilerConfig::default());
        profiler.disable();
        
        profiler.start_phase("disabled_phase");
        profiler.end_phase();
        
        let report = profiler.report();
        assert_eq!(report.phase_count, 0);
    }

    // ============================================================================
    // CODE COVERAGE TESTS
    // ============================================================================

    #[test]
    fn test_coverage_basic_blocks() {
        let mut reporter = CoverageReporter::new(CoverageConfig::default());
        
        reporter.record_basic_block("main", 0);
        reporter.record_basic_block("main", 1);
        reporter.record_basic_block("helper", 0);
        
        let report = reporter.report();
        assert_eq!(report.basic_blocks_reached, 3);
        assert_eq!(report.functions_tested, 2);
    }

    #[test]
    fn test_coverage_branch_recording() {
        let mut reporter = CoverageReporter::new(CoverageConfig::default());
        
        reporter.record_branch("test_fn", "if_0", true);
        reporter.record_branch("test_fn", "if_0", false);
        reporter.record_branch("test_fn", "if_1", true);
        
        let report = reporter.report();
        assert_eq!(report.total_branches, 2);
        assert!(report.branches_covered > 0);
    }

    #[test]
    fn test_coverage_path_tracking() {
        let mut reporter = CoverageReporter::new(CoverageConfig::default());
        
        reporter.record_path("test_fn", "path_0");
        reporter.record_path("test_fn", "path_0");
        reporter.record_path("test_fn", "path_1");
        
        let report = reporter.report();
        assert_eq!(report.paths_executed, 2);
    }

    // ============================================================================
    // REGRESSION DETECTION TESTS
    // ============================================================================

    #[test]
    fn test_regression_detection_positive() {
        let mut detector = RegressionDetector::new(RegressionConfig {
            regression_threshold: 10.0,
            auto_update_baseline: false,
            min_samples: 1,
        });
        
        detector.set_baseline("metric1", 100.0, 5.0);
        detector.record_measurement("metric1", 115.0); // 15% regression
        
        let report = detector.report();
        assert!(!report.regressions.is_empty());
        assert_eq!(report.regressions[0].change_percent, 15.0);
    }

    #[test]
    fn test_improvement_detection() {
        let mut detector = RegressionDetector::new(RegressionConfig {
            regression_threshold: 10.0,
            auto_update_baseline: false,
            min_samples: 1,
        });
        
        detector.set_baseline("metric1", 100.0, 5.0);
        detector.record_measurement("metric1", 80.0); // 20% improvement
        
        let report = detector.report();
        assert!(!report.improvements.is_empty());
        assert_eq!(report.improvements[0].improvement_percent, 20.0);
    }

    #[test]
    fn test_no_regression_within_threshold() {
        let mut detector = RegressionDetector::new(RegressionConfig {
            regression_threshold: 10.0,
            auto_update_baseline: false,
            min_samples: 1,
        });
        
        detector.set_baseline("metric1", 100.0, 5.0);
        detector.record_measurement("metric1", 105.0); // 5% within threshold
        
        let report = detector.report();
        assert!(report.regressions.is_empty());
        assert!(report.improvements.is_empty());
    }

    // ============================================================================
    // ENHANCED DIAGNOSTICS TESTS
    // ============================================================================

    #[test]
    fn test_diagnostics_error_reporting() {
        let mut diag = EnhancedDiagnostics::new();
        diag.report("E0308", DiagnosticLevel::Error, "Type mismatch", None);
        
        let diagnostics = diag.get_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E0308");
    }

    #[test]
    fn test_diagnostics_with_location() {
        let mut diag = EnhancedDiagnostics::new();
        let location = Location {
            file: "test.rs".to_string(),
            line: 10,
            column: 5,
        };
        
        diag.report("E0308", DiagnosticLevel::Error, "Type mismatch", Some(location));
        
        let diagnostics = diag.get_diagnostics();
        assert!(diagnostics[0].location.is_some());
    }

    #[test]
    fn test_diagnostics_with_suggestions() {
        let mut diag = EnhancedDiagnostics::new();
        diag.report("E0308", DiagnosticLevel::Error, "Type mismatch", None);
        
        let diagnostics = diag.get_diagnostics();
        assert!(!diagnostics[0].suggestions.is_empty());
    }

    #[test]
    fn test_diagnostics_summary() {
        let mut diag = EnhancedDiagnostics::new();
        diag.report("E0308", DiagnosticLevel::Error, "Error 1", None);
        diag.report("E0308", DiagnosticLevel::Error, "Error 2", None);
        diag.report("W0001", DiagnosticLevel::Warning, "Warning 1", None);
        
        let summary = diag.summary();
        assert_eq!(summary.errors, 2);
        assert_eq!(summary.warnings, 1);
        assert_eq!(summary.total, 3);
    }

    #[test]
    fn test_diagnostics_formatting() {
        let mut diag = EnhancedDiagnostics::new();
        diag.report("E0308", DiagnosticLevel::Error, "Type mismatch", None);
        
        let formatted = diag.format_report();
        assert!(formatted.contains("ERROR"));
        assert!(formatted.contains("E0308"));
        assert!(formatted.contains("Type mismatch"));
    }

    // ============================================================================
    // INTEGRATED SCENARIO TESTS
    // ============================================================================

    #[test]
    fn test_memory_optimization_pipeline() {
        // Test escape analysis + memory pooling
        let mut escape = EscapeAnalysis::new(EscapeAnalysisConfig::default());
        escape.insert_escape_status("var".to_string(), EscapeStatus::DoesNotEscape);
        
        let mut pool = MemoryPoolAllocator::new(PoolConfig::default());
        pool.init_pool(LifetimeScope::Function);
        
        let escape_report = escape.report_allocations();
        assert_eq!(escape_report.stack_candidates.len(), 1);
        
        let pool_alloc = pool.allocate(LifetimeScope::Function, 1024);
        assert!(pool_alloc.is_ok());
    }

    #[test]
    fn test_profiling_with_regression_detection() {
        let mut profiler = PerformanceProfiler::new(ProfilerConfig::default());
        profiler.start_phase("parse");
        std::thread::sleep(Duration::from_millis(10));
        profiler.end_phase();
        
        let prof_report = profiler.report();
        let parse_time = prof_report.total_time;
        
        // Now set baseline and measure again
        let mut detector = RegressionDetector::new(RegressionConfig::default());
        detector.set_baseline("parse_time", 15.0, 2.0);
        detector.record_measurement("parse_time", parse_time.as_millis() as f64);
        
        let reg_report = detector.report();
        // No regression expected since our measured time is less than baseline
        assert!(reg_report.regressions.is_empty() || 
                reg_report.improvements.len() > 0);
    }

    #[test]
    fn test_comprehensive_diagnostics_workflow() {
        let mut profiler = PerformanceProfiler::new(ProfilerConfig::default());
        let mut coverage = CoverageReporter::new(CoverageConfig::default());
        let mut diags = EnhancedDiagnostics::new();
        
        // Simulate compilation
        profiler.start_phase("lexer");
        std::thread::sleep(Duration::from_millis(1));
        profiler.end_phase();
        
        coverage.record_basic_block("main", 0);
        diags.report("W0001", DiagnosticLevel::Warning, "Unused variable", None);
        
        let prof = profiler.report();
        let cov = coverage.report();
        let diag_summary = diags.summary();
        
        assert_eq!(prof.phase_count, 1);
        assert!(cov.basic_blocks_reached > 0);
        assert_eq!(diag_summary.warnings, 1);
    }
}
