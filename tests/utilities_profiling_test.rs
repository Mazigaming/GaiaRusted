//! Tests for profiling and performance tracking

#[cfg(test)]
mod profiling_tests {
    use gaiarusted::profiling::{Profiler, PhaseProfile, CompilationStats};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_phase_profile_timing() {
        let mut profile = PhaseProfile::start("test_phase");
        thread::sleep(Duration::from_millis(10));
        profile.end();

        assert!(profile.duration.is_some());
        assert!(profile.duration_ms() >= 10.0);
    }

    #[test]
    fn test_profiler_single_phase() {
        let mut profiler = Profiler::new();
        let phase_id = profiler.start_phase("Lexer");
        thread::sleep(Duration::from_millis(5));
        profiler.end_phase(phase_id);

        assert_eq!(profiler.phases().len(), 1);
        assert!(profiler.total_time_ms() >= 5.0);
    }

    #[test]
    fn test_profiler_multiple_phases() {
        let mut profiler = Profiler::new();

        let phase1 = profiler.start_phase("Lexer");
        thread::sleep(Duration::from_millis(5));
        profiler.end_phase(phase1);

        let phase2 = profiler.start_phase("Parser");
        thread::sleep(Duration::from_millis(10));
        profiler.end_phase(phase2);

        let phase3 = profiler.start_phase("Typechecker");
        thread::sleep(Duration::from_millis(3));
        profiler.end_phase(phase3);

        assert_eq!(profiler.phases().len(), 3);
        assert!(profiler.total_time_ms() >= 18.0);
    }

    #[test]
    fn test_profiler_slowest_phase() {
        let mut profiler = Profiler::new();

        let p1 = profiler.start_phase("Fast");
        thread::sleep(Duration::from_millis(2));
        profiler.end_phase(p1);

        let p2 = profiler.start_phase("Slow");
        thread::sleep(Duration::from_millis(10));
        profiler.end_phase(p2);

        let slowest = profiler.slowest_phase();
        assert!(slowest.is_some());
        assert_eq!(slowest.unwrap().name, "Slow");
    }

    #[test]
    fn test_profiler_by_duration() {
        let mut profiler = Profiler::new();

        let p1 = profiler.start_phase("Medium");
        thread::sleep(Duration::from_millis(5));
        profiler.end_phase(p1);

        let p2 = profiler.start_phase("Longest");
        thread::sleep(Duration::from_millis(10));
        profiler.end_phase(p2);

        let p3 = profiler.start_phase("Shortest");
        thread::sleep(Duration::from_millis(2));
        profiler.end_phase(p3);

        let sorted = profiler.phases_by_duration();
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].name, "Longest");
        assert_eq!(sorted[2].name, "Shortest");
    }

    #[test]
    fn test_profiler_formatting() {
        let mut profiler = Profiler::new();

        let phase = profiler.start_phase("Test Phase");
        thread::sleep(Duration::from_millis(5));
        profiler.end_phase(phase);

        let report = profiler.format_report();
        assert!(report.contains("Compilation Profile"));
        assert!(report.contains("Test Phase"));
        assert!(report.contains("ms"));
    }

    #[test]
    fn test_compilation_stats() {
        let stats = CompilationStats {
            lines_processed: 100,
            tokens_generated: 500,
            ast_nodes: 50,
            functions: 5,
            structs: 2,
            variables: 20,
        };

        assert_eq!(stats.lines_processed, 100);
        assert_eq!(stats.tokens_generated, 500);

        let summary = stats.format_summary();
        assert!(summary.contains("100"));
        assert!(summary.contains("500"));
    }

    #[test]
    fn test_profiler_display() {
        let mut profiler = Profiler::new();
        let phase = profiler.start_phase("Phase");
        thread::sleep(Duration::from_millis(2));
        profiler.end_phase(phase);

        let display_string = profiler.to_string();
        assert!(!display_string.is_empty());
    }
}