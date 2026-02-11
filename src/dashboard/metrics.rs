//! Compilation metrics collection and analysis

use std::time::{Duration, Instant};
use std::collections::HashMap;

/// Tracks timing for all compilation phases
#[derive(Debug, Clone)]
pub struct CompilationMetrics {
    /// Phase start times
    phase_starts: HashMap<String, Instant>,
    /// Phase durations (completed phases)
    phase_durations: HashMap<String, Duration>,
    /// Compilation start time
    compilation_start: Instant,
}

impl CompilationMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        CompilationMetrics {
            phase_starts: HashMap::new(),
            phase_durations: HashMap::new(),
            compilation_start: Instant::now(),
        }
    }

    /// Start timing a phase
    pub fn start_phase(&mut self, phase_name: &str) {
        self.phase_starts.insert(phase_name.to_string(), Instant::now());
    }

    /// End timing a phase and return duration
    pub fn end_phase(&mut self, phase_name: &str) -> Duration {
        if let Some(start) = self.phase_starts.remove(phase_name) {
            let duration = start.elapsed();
            self.phase_durations.insert(phase_name.to_string(), duration);
            duration
        } else {
            Duration::ZERO
        }
    }

    /// Get duration for a phase
    pub fn phase_duration(&self, phase_name: &str) -> Option<Duration> {
        self.phase_durations.get(phase_name).copied()
    }

    /// Get all phase durations
    pub fn all_phases(&self) -> &HashMap<String, Duration> {
        &self.phase_durations
    }

    /// Get total compilation time
    pub fn total_time(&self) -> Duration {
        self.compilation_start.elapsed()
    }

    /// Get phase statistics
    pub fn stats(&self) -> MetricsStats {
        let total = self.total_time();
        let mut phase_list: Vec<_> = self.phase_durations.iter().collect();
        phase_list.sort_by_key(|(_, duration)| std::cmp::Reverse(*duration));

        let slowest = phase_list.first().map(|(name, duration)| ((*name).clone(), **duration));
        let fastest = phase_list.last().map(|(name, duration)| ((*name).clone(), **duration));

        MetricsStats {
            total_time: total,
            phase_count: self.phase_durations.len(),
            slowest_phase: slowest,
            fastest_phase: fastest,
        }
    }

    /// Reset metrics for new compilation
    pub fn reset(&mut self) {
        self.phase_starts.clear();
        self.phase_durations.clear();
        self.compilation_start = Instant::now();
    }

    /// Get percentage of total time for a phase
    pub fn phase_percentage(&self, phase_name: &str) -> f64 {
        if let Some(duration) = self.phase_duration(phase_name) {
            let total = self.total_time().as_millis() as f64;
            if total > 0.0 {
                (duration.as_millis() as f64 / total) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

impl Default for CompilationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics from metrics
#[derive(Debug, Clone)]
pub struct MetricsStats {
    pub total_time: Duration,
    pub phase_count: usize,
    pub slowest_phase: Option<(String, Duration)>,
    pub fastest_phase: Option<(String, Duration)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = CompilationMetrics::new();
        assert_eq!(metrics.all_phases().len(), 0);
    }

    #[test]
    fn test_phase_timing() {
        let mut metrics = CompilationMetrics::new();
        metrics.start_phase("Parsing");
        std::thread::sleep(Duration::from_millis(10));
        let duration = metrics.end_phase("Parsing");
        assert!(duration.as_millis() >= 10);
    }

    #[test]
    fn test_phase_duration_retrieval() {
        let mut metrics = CompilationMetrics::new();
        metrics.start_phase("TypeCheck");
        std::thread::sleep(Duration::from_millis(5));
        metrics.end_phase("TypeCheck");

        let duration = metrics.phase_duration("TypeCheck").unwrap();
        assert!(duration.as_millis() >= 5);
    }

    #[test]
    fn test_multiple_phases() {
        let mut metrics = CompilationMetrics::new();

        metrics.start_phase("Phase1");
        std::thread::sleep(Duration::from_millis(5));
        metrics.end_phase("Phase1");

        metrics.start_phase("Phase2");
        std::thread::sleep(Duration::from_millis(3));
        metrics.end_phase("Phase2");

        assert_eq!(metrics.all_phases().len(), 2);
    }

    #[test]
    fn test_phase_percentage() {
        let mut metrics = CompilationMetrics::new();
        metrics.start_phase("Test");
        std::thread::sleep(Duration::from_millis(10));
        metrics.end_phase("Test");

        let percent = metrics.phase_percentage("Test");
        assert!(percent > 0.0 && percent <= 100.0);
    }

    #[test]
    fn test_stats() {
        let mut metrics = CompilationMetrics::new();
        metrics.start_phase("Quick");
        std::thread::sleep(Duration::from_millis(2));
        metrics.end_phase("Quick");

        let stats = metrics.stats();
        assert_eq!(stats.phase_count, 1);
        assert!(stats.total_time.as_millis() >= 2);
    }

    #[test]
    fn test_reset() {
        let mut metrics = CompilationMetrics::new();
        metrics.start_phase("Test");
        metrics.end_phase("Test");
        assert_eq!(metrics.all_phases().len(), 1);

        metrics.reset();
        assert_eq!(metrics.all_phases().len(), 0);
    }
}
