//! # Compilation Dashboard - Real-time animated progress display
//!
//! Shows where compilation time goes with animated ASCII progress bars.
//! Integrates with actual compiler phases for real timing data.

use std::time::{Duration, Instant};
use std::collections::HashMap;

pub mod animator;
pub mod metrics;

pub use animator::ProgressBar;
pub use metrics::CompilationMetrics;

/// Main dashboard for displaying compilation progress
pub struct Dashboard {
    /// Phase timings
    phases: HashMap<String, Duration>,
    /// Start time
    start_time: Instant,
    /// Current total time
    total_time: Duration,
    /// Whether dashboard is enabled
    enabled: bool,
    /// Metrics collector
    metrics: CompilationMetrics,
}

impl Dashboard {
    /// Create a new dashboard
    pub fn new() -> Self {
        Dashboard {
            phases: HashMap::new(),
            start_time: Instant::now(),
            total_time: Duration::ZERO,
            enabled: true,
            metrics: CompilationMetrics::new(),
        }
    }

    /// Disable the dashboard (for quiet mode)
    pub fn disabled() -> Self {
        Dashboard {
            phases: HashMap::new(),
            start_time: Instant::now(),
            total_time: Duration::ZERO,
            enabled: false,
            metrics: CompilationMetrics::new(),
        }
    }

    /// Start timing a phase
    pub fn start_phase(&mut self, phase_name: &str) {
        self.metrics.start_phase(phase_name);
        if self.enabled {
            print!("{}  {:<30}", crate::formatter::Colors::CYAN, phase_name);
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
    }

    /// End timing a phase and display progress
    pub fn end_phase(&mut self, phase_name: &str) {
        let duration = self.metrics.end_phase(phase_name);
        self.phases.insert(phase_name.to_string(), duration);
        self.total_time = self.start_time.elapsed();

        if self.enabled {
            let bar = self.create_progress_bar(&duration);
            println!("{} {:>6}ms{}", bar, duration.as_millis(), crate::formatter::Colors::RESET);
        }
    }

    /// Create an animated progress bar for a duration
    fn create_progress_bar(&self, duration: &Duration) -> String {
        let filled = if self.total_time.as_millis() > 0 {
            let percent = (duration.as_millis() as f64 / self.total_time.as_millis() as f64) * 100.0;
            ((percent / 100.0) * 15.0) as usize
        } else {
            8  // Default to half-full for minimal times
        };
        let empty = 15 - filled;

        format!(
            "{}{}{}",
            crate::formatter::Colors::GREEN,
            "█".repeat(filled),
            "░".repeat(empty)
        )
    }

    /// Display the final dashboard report
    pub fn display_report(&self) {
        if !self.enabled {
            return;
        }

        println!("\n");
        println!(
            "{}╔═══════════════════════════════════════════════════════════════╗{}",
            crate::formatter::Colors::CYAN,
            crate::formatter::Colors::RESET
        );
        println!(
            "{}║{} Compilation Summary{}{}                                        {}║{}",
            crate::formatter::Colors::CYAN,
            crate::formatter::Colors::BOLD,
            crate::formatter::Colors::CYAN,
            crate::formatter::Colors::RESET,
            crate::formatter::Colors::CYAN,
            crate::formatter::Colors::RESET
        );
        println!(
            "{}╠═══════════════════════════════════════════════════════════════╣{}",
            crate::formatter::Colors::CYAN,
            crate::formatter::Colors::RESET
        );

        // Display phase breakdown
        for (phase_name, duration) in &self.phases {
            let percent = if self.total_time.as_micros() > 0 {
                (duration.as_micros() as f64 / self.total_time.as_micros() as f64) * 100.0
            } else {
                0.0
            };
            
            // Create bar proportional to percentage (wider for better visuals)
            let filled = ((percent / 100.0) * 20.0) as usize;
            let empty = 20 - filled;
            let bar = format!(
                "{}{}{}{}",
                crate::formatter::Colors::GREEN,
                "█".repeat(filled),
                "░".repeat(empty),
                crate::formatter::Colors::RESET
            );
            
            // Display in decimal milliseconds for better precision on fast phases
            let duration_ms = duration.as_micros() as f64 / 1000.0;
            println!(
                "{}║{} {:<20} {} {:>7.2}ms  {:>5.1}%{}                    {}║{}",
                crate::formatter::Colors::CYAN,
                crate::formatter::Colors::RESET,
                phase_name,
                bar,
                duration_ms,
                percent,
                crate::formatter::Colors::RESET,
                crate::formatter::Colors::CYAN,
                crate::formatter::Colors::RESET
            );
        }

        // Display total with fancy box
        println!(
            "{}╠═══════════════════════════════════════════════════════════════╣{}",
            crate::formatter::Colors::CYAN,
            crate::formatter::Colors::RESET
        );
        let total_ms = self.total_time.as_micros() as f64 / 1000.0;
        println!(
            "{}║{} Total Compilation Time: {}{:.2}ms{}{}                          {}║{}",
            crate::formatter::Colors::CYAN,
            crate::formatter::Colors::RESET,
            crate::formatter::Colors::BOLD,
            total_ms,
            crate::formatter::Colors::RESET,
            crate::formatter::Colors::RESET,
            crate::formatter::Colors::CYAN,
            crate::formatter::Colors::RESET
        );
        println!(
            "{}╚═══════════════════════════════════════════════════════════════╝{}",
            crate::formatter::Colors::CYAN,
            crate::formatter::Colors::RESET
        );
        println!();
    }

    /// Get total compilation time
    pub fn total_time(&self) -> Duration {
        self.total_time
    }

    /// Get a specific phase's duration
    pub fn phase_duration(&self, phase_name: &str) -> Option<Duration> {
        self.phases.get(phase_name).copied()
    }

    /// Get all phases
    pub fn phases(&self) -> &HashMap<String, Duration> {
        &self.phases
    }

    /// Reset the dashboard for a new compilation
    pub fn reset(&mut self) {
        self.phases.clear();
        self.start_time = Instant::now();
        self.total_time = Duration::ZERO;
        self.metrics.reset();
    }
}

impl Default for Dashboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_creation() {
        let dashboard = Dashboard::new();
        assert!(dashboard.enabled);
        assert_eq!(dashboard.phases.len(), 0);
    }

    #[test]
    fn test_dashboard_disabled() {
        let dashboard = Dashboard::disabled();
        assert!(!dashboard.enabled);
    }

    #[test]
    fn test_phase_tracking() {
        let mut dashboard = Dashboard::new();
        dashboard.start_phase("Parsing");
        std::thread::sleep(Duration::from_millis(10));
        dashboard.end_phase("Parsing");

        assert!(dashboard.phases.contains_key("Parsing"));
        let duration = dashboard.phase_duration("Parsing").unwrap();
        assert!(duration.as_millis() >= 10);
    }

    #[test]
    fn test_multiple_phases() {
        let mut dashboard = Dashboard::new();

        dashboard.start_phase("Parsing");
        std::thread::sleep(Duration::from_millis(5));
        dashboard.end_phase("Parsing");

        dashboard.start_phase("Type Checking");
        std::thread::sleep(Duration::from_millis(5));
        dashboard.end_phase("Type Checking");

        assert_eq!(dashboard.phases.len(), 2);
        assert!(dashboard.total_time().as_millis() >= 10);
    }

    #[test]
    fn test_progress_bar_creation() {
        let dashboard = Dashboard::new();
        let bar = dashboard.create_progress_bar(&Duration::from_millis(50));
        assert!(!bar.is_empty());
    }

    #[test]
    fn test_reset() {
        let mut dashboard = Dashboard::new();
        dashboard.start_phase("Test");
        dashboard.end_phase("Test");
        assert_eq!(dashboard.phases.len(), 1);

        dashboard.reset();
        assert_eq!(dashboard.phases.len(), 0);
    }
}
