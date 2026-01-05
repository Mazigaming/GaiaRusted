/// Profiling & Diagnostics Module for GaiaRusted v0.12.0
///
/// Implements performance profiling, code coverage reporting,
/// regression detection, and enhanced diagnostics.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Performance Profiler: Tracks compilation and execution metrics
#[derive(Debug, Clone)]
pub struct PerformanceProfiler {
    /// Timestamps for different phases
    phase_timings: HashMap<String, Vec<Duration>>,
    /// Current running timer
    active_timer: Option<(String, Instant)>,
    /// Enable profiling
    enabled: bool,
    /// Configuration
    config: ProfilerConfig,
}

#[derive(Debug, Clone)]
pub struct ProfilerConfig {
    /// Collect detailed metrics
    pub detailed_metrics: bool,
    /// Track memory usage
    pub track_memory: bool,
    /// Enable sampling
    pub enable_sampling: bool,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            detailed_metrics: true,
            track_memory: true,
            enable_sampling: false,
        }
    }
}

impl PerformanceProfiler {
    pub fn new(config: ProfilerConfig) -> Self {
        Self {
            phase_timings: HashMap::new(),
            active_timer: None,
            enabled: true,
            config,
        }
    }

    /// Start timing a phase
    pub fn start_phase(&mut self, phase_name: &str) {
        if !self.enabled {
            return;
        }

        self.active_timer = Some((phase_name.to_string(), Instant::now()));
    }

    /// End timing a phase
    pub fn end_phase(&mut self) {
        if !self.enabled {
            return;
        }

        if let Some((phase_name, start_time)) = self.active_timer.take() {
            let elapsed = start_time.elapsed();
            self.phase_timings
                .entry(phase_name)
                .or_insert_with(Vec::new)
                .push(elapsed);
        }
    }

    /// Get timing report for a phase
    pub fn get_phase_timing(&self, phase: &str) -> Option<PhaseTimingReport> {
        self.phase_timings.get(phase).map(|durations| {
            let total: Duration = durations.iter().sum();
            let count = durations.len();
            let avg = if count > 0 {
                Duration::from_micros(total.as_micros() as u64 / count as u64)
            } else {
                Duration::ZERO
            };
            let min = durations.iter().min().copied().unwrap_or(Duration::ZERO);
            let max = durations.iter().max().copied().unwrap_or(Duration::ZERO);

            PhaseTimingReport {
                phase_name: phase.to_string(),
                total,
                count,
                average: avg,
                min,
                max,
            }
        })
    }

    /// Get overall profiling report
    pub fn report(&self) -> ProfilingReport {
        let mut phase_reports = Vec::new();
        let total_time: Duration = self
            .phase_timings
            .values()
            .flat_map(|v| v.iter())
            .sum();

        for (phase, durations) in &self.phase_timings {
            if let Some(report) = self.get_phase_timing(phase) {
                phase_reports.push(report);
            }
        }

        phase_reports.sort_by(|a, b| b.total.cmp(&a.total));

        ProfilingReport {
            total_time,
            phase_count: self.phase_timings.len(),
            phase_reports,
        }
    }

    /// Disable profiling
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Enable profiling
    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

#[derive(Debug, Clone)]
pub struct PhaseTimingReport {
    pub phase_name: String,
    pub total: Duration,
    pub count: usize,
    pub average: Duration,
    pub min: Duration,
    pub max: Duration,
}

#[derive(Debug, Clone)]
pub struct ProfilingReport {
    pub total_time: Duration,
    pub phase_count: usize,
    pub phase_reports: Vec<PhaseTimingReport>,
}

/// Code Coverage Reporter: Tracks which code paths are executed
#[derive(Debug, Clone)]
pub struct CoverageReporter {
    /// Basic blocks reached
    basic_blocks_reached: HashMap<String, HashSet<usize>>,
    /// Paths executed
    paths_executed: HashMap<String, usize>,
    /// Branch coverage
    branch_coverage: HashMap<String, BranchCoverage>,
    config: CoverageConfig,
}

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct CoverageConfig {
    pub track_basic_blocks: bool,
    pub track_branches: bool,
    pub track_paths: bool,
}

impl Default for CoverageConfig {
    fn default() -> Self {
        Self {
            track_basic_blocks: true,
            track_branches: true,
            track_paths: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BranchCoverage {
    pub true_taken: usize,
    pub false_taken: usize,
}

impl CoverageReporter {
    pub fn new(config: CoverageConfig) -> Self {
        Self {
            basic_blocks_reached: HashMap::new(),
            paths_executed: HashMap::new(),
            branch_coverage: HashMap::new(),
            config,
        }
    }

    /// Record basic block execution
    pub fn record_basic_block(&mut self, function: &str, block_id: usize) {
        if !self.config.track_basic_blocks {
            return;
        }

        self.basic_blocks_reached
            .entry(function.to_string())
            .or_insert_with(HashSet::new)
            .insert(block_id);
    }

    /// Record branch taken
    pub fn record_branch(&mut self, function: &str, branch_id: &str, taken_true: bool) {
        if !self.config.track_branches {
            return;
        }

        let key = format!("{}::{}", function, branch_id);
        let coverage = self
            .branch_coverage
            .entry(key)
            .or_insert(BranchCoverage {
                true_taken: 0,
                false_taken: 0,
            });

        if taken_true {
            coverage.true_taken += 1;
        } else {
            coverage.false_taken += 1;
        }
    }

    /// Record path execution
    pub fn record_path(&mut self, function: &str, path_id: &str) {
        if !self.config.track_paths {
            return;
        }

        let key = format!("{}::{}", function, path_id);
        *self.paths_executed.entry(key).or_insert(0) += 1;
    }

    /// Generate coverage report
    pub fn report(&self) -> CoverageReport {
        // Count total blocks reached across all functions
        let block_coverage: usize = self.basic_blocks_reached
            .values()
            .map(|blocks| blocks.len())
            .sum();

        let total_branches = self.branch_coverage.len();
        let covered_branches = self
            .branch_coverage
            .values()
            .filter(|c| c.true_taken > 0 && c.false_taken > 0)
            .count();

        CoverageReport {
            basic_blocks_reached: block_coverage,
            branches_covered: covered_branches,
            total_branches,
            paths_executed: self.paths_executed.len(),
            functions_tested: self.basic_blocks_reached.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CoverageReport {
    pub basic_blocks_reached: usize,
    pub branches_covered: usize,
    pub total_branches: usize,
    pub paths_executed: usize,
    pub functions_tested: usize,
}

/// Regression Detector: Identifies performance regressions
#[derive(Debug, Clone)]
pub struct RegressionDetector {
    /// Baseline metrics
    baselines: HashMap<String, MetricBaseline>,
    /// Current measurements
    current: HashMap<String, f64>,
    /// Detection thresholds
    config: RegressionConfig,
}

#[derive(Debug, Clone)]
pub struct MetricBaseline {
    pub name: String,
    pub value: f64,
    pub stddev: f64,
}

#[derive(Debug, Clone)]
pub struct RegressionConfig {
    /// Regression threshold (percentage)
    pub regression_threshold: f64,
    /// Enable automatic baseline update
    pub auto_update_baseline: bool,
    /// Minimum samples before regression detection
    pub min_samples: usize,
}

impl Default for RegressionConfig {
    fn default() -> Self {
        Self {
            regression_threshold: 10.0, // 10% regression threshold
            auto_update_baseline: false,
            min_samples: 5,
        }
    }
}

impl RegressionDetector {
    pub fn new(config: RegressionConfig) -> Self {
        Self {
            baselines: HashMap::new(),
            current: HashMap::new(),
            config,
        }
    }

    /// Set baseline for a metric
    pub fn set_baseline(&mut self, name: &str, value: f64, stddev: f64) {
        self.baselines.insert(
            name.to_string(),
            MetricBaseline {
                name: name.to_string(),
                value,
                stddev,
            },
        );
    }

    /// Record current measurement
    pub fn record_measurement(&mut self, name: &str, value: f64) {
        self.current.insert(name.to_string(), value);
    }

    /// Detect regressions
    pub fn detect_regressions(&self) -> RegressionReport {
        let mut regressions = Vec::new();
        let mut improvements = Vec::new();

        for (name, baseline) in &self.baselines {
            if let Some(&current_value) = self.current.get(name) {
                let change_percent = ((current_value - baseline.value) / baseline.value) * 100.0;

                if change_percent > self.config.regression_threshold {
                    regressions.push(RegressionItem {
                        metric: name.clone(),
                        baseline: baseline.value,
                        current: current_value,
                        change_percent,
                    });
                } else if change_percent < -self.config.regression_threshold {
                    improvements.push(ImprovementItem {
                        metric: name.clone(),
                        baseline: baseline.value,
                        current: current_value,
                        improvement_percent: -change_percent,
                    });
                }
            }
        }

        RegressionReport {
            regressions,
            improvements,
            total_metrics: self.baselines.len(),
        }
    }

    /// Get regression report
    pub fn report(&self) -> RegressionReport {
        self.detect_regressions()
    }
}

#[derive(Debug, Clone)]
pub struct RegressionItem {
    pub metric: String,
    pub baseline: f64,
    pub current: f64,
    pub change_percent: f64,
}

#[derive(Debug, Clone)]
pub struct ImprovementItem {
    pub metric: String,
    pub baseline: f64,
    pub current: f64,
    pub improvement_percent: f64,
}

#[derive(Debug, Clone)]
pub struct RegressionReport {
    pub regressions: Vec<RegressionItem>,
    pub improvements: Vec<ImprovementItem>,
    pub total_metrics: usize,
}

/// Enhanced Diagnostics: Better error messages with fix suggestions
#[derive(Debug, Clone)]
pub struct EnhancedDiagnostics {
    /// Collected diagnostics
    diagnostics: Vec<DiagnosticMessage>,
    /// Fix suggestions database
    suggestions: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticMessage {
    pub code: String,
    pub level: DiagnosticLevel,
    pub message: String,
    pub location: Option<Location>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
    Help,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl EnhancedDiagnostics {
    pub fn new() -> Self {
        let mut d = Self {
            diagnostics: Vec::new(),
            suggestions: HashMap::new(),
        };
        d.init_suggestions();
        d
    }

    fn init_suggestions(&mut self) {
        // Initialize common fix suggestions
        self.suggestions.insert(
            "E0308".to_string(),
            vec![
                "Check type annotations".to_string(),
                "Ensure variables are initialized".to_string(),
                "Cast types explicitly if needed".to_string(),
            ],
        );

        self.suggestions.insert(
            "E0382".to_string(),
            vec![
                "Use references (&) to avoid moves".to_string(),
                "Clone values when needed".to_string(),
                "Use move semantics carefully".to_string(),
            ],
        );

        self.suggestions.insert(
            "W0001".to_string(),
            vec![
                "Remove unused variables".to_string(),
                "Prefix with underscore if intentional".to_string(),
            ],
        );
    }

    /// Report a diagnostic with suggestions
    pub fn report(&mut self, code: &str, level: DiagnosticLevel, message: &str, location: Option<Location>) {
        let suggestions = self
            .suggestions
            .get(code)
            .cloned()
            .unwrap_or_default();

        self.diagnostics.push(DiagnosticMessage {
            code: code.to_string(),
            level,
            message: message.to_string(),
            location,
            suggestions,
        });
    }

    /// Get all diagnostics
    pub fn get_diagnostics(&self) -> &[DiagnosticMessage] {
        &self.diagnostics
    }

    /// Get diagnostic summary
    pub fn summary(&self) -> DiagnosticSummary {
        let mut errors = 0;
        let mut warnings = 0;
        let mut notes = 0;

        for diag in &self.diagnostics {
            match diag.level {
                DiagnosticLevel::Error => errors += 1,
                DiagnosticLevel::Warning => warnings += 1,
                DiagnosticLevel::Note | DiagnosticLevel::Help => notes += 1,
            }
        }

        DiagnosticSummary {
            total: self.diagnostics.len(),
            errors,
            warnings,
            notes,
        }
    }

    /// Format diagnostics for display
    pub fn format_report(&self) -> String {
        let mut output = String::new();
        let summary = self.summary();

        output.push_str(&format!("Diagnostics: {} errors, {} warnings, {} notes\n", 
            summary.errors, summary.warnings, summary.notes));
        output.push_str(&"─".repeat(60));
        output.push('\n');

        for diag in &self.diagnostics {
            output.push_str(&format!("[{}] {}: {}\n", 
                match diag.level {
                    DiagnosticLevel::Error => "ERROR",
                    DiagnosticLevel::Warning => "WARN ",
                    DiagnosticLevel::Note => "NOTE ",
                    DiagnosticLevel::Help => "HELP ",
                },
                diag.code,
                diag.message
            ));

            if let Some(loc) = &diag.location {
                output.push_str(&format!("  at {}:{}:{}\n", loc.file, loc.line, loc.column));
            }

            if !diag.suggestions.is_empty() {
                output.push_str("  Suggestions:\n");
                for suggestion in &diag.suggestions {
                    output.push_str(&format!("    - {}\n", suggestion));
                }
            }
        }

        output
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticSummary {
    pub total: usize,
    pub errors: usize,
    pub warnings: usize,
    pub notes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = PerformanceProfiler::new(ProfilerConfig::default());
        assert!(profiler.enabled);
    }

    #[test]
    fn test_profiler_timing() {
        let mut profiler = PerformanceProfiler::new(ProfilerConfig::default());
        profiler.start_phase("test_phase");
        std::thread::sleep(std::time::Duration::from_millis(10));
        profiler.end_phase();

        let report = profiler.report();
        assert_eq!(report.phase_count, 1);
        assert!(report.total_time >= Duration::from_millis(10));
    }

    #[test]
    fn test_coverage_reporter() {
        let mut reporter = CoverageReporter::new(CoverageConfig::default());
        reporter.record_basic_block("test_fn", 0);
        reporter.record_branch("test_fn", "if_0", true);
        reporter.record_path("test_fn", "path_0");

        let report = reporter.report();
        assert!(report.basic_blocks_reached > 0);
    }

    #[test]
    fn test_regression_detection() {
        let mut detector = RegressionDetector::new(RegressionConfig::default());
        detector.set_baseline("metric1", 100.0, 5.0);
        detector.record_measurement("metric1", 115.0); // 15% regression

        let report = detector.report();
        assert!(!report.regressions.is_empty());
    }

    #[test]
    fn test_enhanced_diagnostics() {
        let mut diag = EnhancedDiagnostics::new();
        diag.report("E0308", DiagnosticLevel::Error, "Type mismatch", None);
        diag.report("W0001", DiagnosticLevel::Warning, "Unused variable", None);

        let summary = diag.summary();
        assert_eq!(summary.errors, 1);
        assert_eq!(summary.warnings, 1);
    }

    #[test]
    fn test_diagnostic_formatting() {
        let mut diag = EnhancedDiagnostics::new();
        diag.report("E0308", DiagnosticLevel::Error, "Type mismatch", 
            Some(Location {
                file: "test.rs".to_string(),
                line: 10,
                column: 5,
            })
        );

        let formatted = diag.format_report();
        assert!(formatted.contains("ERROR"));
        assert!(formatted.contains("E0308"));
    }
}
