//! Performance profiling and metrics collection
//!
//! This module provides tools for profiling the compiler's performance,
//! tracking phase execution times, and identifying bottlenecks.

use std::time::{Duration, Instant};
use std::fmt;

/// A performance profile for a specific phase
#[derive(Debug, Clone)]
pub struct PhaseProfile {
    pub name: String,
    pub start_time: Instant,
    pub duration: Option<Duration>,
    pub memory_before: Option<usize>,
    pub memory_after: Option<usize>,
}

impl PhaseProfile {
    /// Create a new phase profile
    pub fn start(name: &str) -> Self {
        PhaseProfile {
            name: name.to_string(),
            start_time: Instant::now(),
            duration: None,
            memory_before: Self::current_memory(),
            memory_after: None,
        }
    }

    /// End the profile and record the duration
    pub fn end(&mut self) {
        self.duration = Some(self.start_time.elapsed());
        self.memory_after = Self::current_memory();
    }

    /// Get estimated memory usage in bytes
    fn current_memory() -> Option<usize> {
        // Try to read from /proc/self/status on Linux
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(mem_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = mem_str.parse::<usize>() {
                                return Some(kb * 1024);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Get duration in milliseconds
    pub fn duration_ms(&self) -> f64 {
        self.duration
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0)
    }

    /// Get memory delta in MB
    pub fn memory_delta_mb(&self) -> Option<f64> {
        match (self.memory_before, self.memory_after) {
            (Some(before), Some(after)) => {
                Some((after as i64 - before as i64) as f64 / (1024.0 * 1024.0))
            },
            _ => None,
        }
    }
}

/// Main profiler for tracking all compilation phases
pub struct Profiler {
    phases: Vec<PhaseProfile>,
    total_start: Instant,
}

impl Profiler {
    /// Create a new profiler
    pub fn new() -> Self {
        Profiler {
            phases: Vec::new(),
            total_start: Instant::now(),
        }
    }

    /// Start profiling a phase
    pub fn start_phase(&mut self, name: &str) -> usize {
        let profile = PhaseProfile::start(name);
        self.phases.push(profile);
        self.phases.len() - 1
    }

    /// End profiling a phase
    pub fn end_phase(&mut self, phase_id: usize) {
        if phase_id < self.phases.len() {
            self.phases[phase_id].end();
        }
    }

    /// Get total compilation time
    pub fn total_time(&self) -> Duration {
        self.total_start.elapsed()
    }

    /// Get total time in milliseconds
    pub fn total_time_ms(&self) -> f64 {
        self.total_time().as_secs_f64() * 1000.0
    }

    /// Get all phases
    pub fn phases(&self) -> &[PhaseProfile] {
        &self.phases
    }

    /// Get the slowest phase
    pub fn slowest_phase(&self) -> Option<&PhaseProfile> {
        self.phases.iter().max_by_key(|p| p.duration_ms() as u64)
    }

    /// Get phases sorted by duration (descending)
    pub fn phases_by_duration(&self) -> Vec<&PhaseProfile> {
        let mut phases = self.phases.iter().collect::<Vec<_>>();
        phases.sort_by(|a, b| {
            b.duration_ms().partial_cmp(&a.duration_ms()).unwrap_or(std::cmp::Ordering::Equal)
        });
        phases
    }

    /// Format profiling results
    pub fn format_report(&self) -> String {
        let mut output = String::new();
        
        output.push_str("╭─────────── Compilation Profile ───────────╮\n");
        output.push_str("│                                            │\n");

        let total_ms = self.total_time_ms();
        
        for phase in self.phases_by_duration() {
            let percent = if total_ms > 0.0 {
                (phase.duration_ms() / total_ms) * 100.0
            } else {
                0.0
            };
            
            let bar_width = (percent / 2.0) as usize;
            let bar = "█".repeat(bar_width);
            
            let memory_str = phase.memory_delta_mb()
                .map(|delta| format!(" | Δ{:+.1}MB", delta))
                .unwrap_or_default();

            output.push_str(&format!(
                "│ {:<20} {:>5.1}ms [{:<25}]{}\n",
                phase.name,
                phase.duration_ms(),
                bar,
                memory_str
            ));
        }

        output.push_str("│                                            │\n");
        output.push_str(&format!(
            "│ {:<20} {:>5.1}ms                           │\n",
            "Total", total_ms
        ));
        output.push_str("╰────────────────────────────────────────────╯\n");

        output
    }

    /// Print the profiling report
    pub fn print_report(&self) {
        println!("{}", self.format_report());
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Profiler {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.format_report())
    }
}

/// Scoped phase profiler - automatically records start and end
pub struct ScopedPhase {
    phase_id: usize,
    profiler: *mut Profiler,
}

impl ScopedPhase {
    /// Create a new scoped phase (requires profiler reference)
    pub fn new(phase_id: usize, profiler: *mut Profiler) -> Self {
        ScopedPhase { phase_id, profiler }
    }
}

impl Drop for ScopedPhase {
    fn drop(&mut self) {
        unsafe {
            if !self.profiler.is_null() {
                (*self.profiler).end_phase(self.phase_id);
            }
        }
    }
}

/// Global statistics tracker
pub struct CompilationStats {
    pub lines_processed: usize,
    pub tokens_generated: usize,
    pub ast_nodes: usize,
    pub functions: usize,
    pub structs: usize,
    pub variables: usize,
}

impl CompilationStats {
    pub fn new() -> Self {
        CompilationStats {
            lines_processed: 0,
            tokens_generated: 0,
            ast_nodes: 0,
            functions: 0,
            structs: 0,
            variables: 0,
        }
    }

    pub fn format_summary(&self) -> String {
        let mut output = String::new();
        output.push_str("╭──────── Compilation Statistics ────────╮\n");
        output.push_str(&format!("│ Lines processed:      {:>10}         │\n", self.lines_processed));
        output.push_str(&format!("│ Tokens generated:     {:>10}         │\n", self.tokens_generated));
        output.push_str(&format!("│ AST nodes:            {:>10}         │\n", self.ast_nodes));
        output.push_str(&format!("│ Functions:            {:>10}         │\n", self.functions));
        output.push_str(&format!("│ Structs:              {:>10}         │\n", self.structs));
        output.push_str(&format!("│ Variables:            {:>10}         │\n", self.variables));
        output.push_str("╰────────────────────────────────────────╯\n");
        output
    }

    pub fn print_summary(&self) {
        println!("{}", self.format_summary());
    }
}

impl Default for CompilationStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_phase_profile() {
        let mut profile = PhaseProfile::start("test_phase");
        thread::sleep(Duration::from_millis(10));
        profile.end();
        
        assert!(profile.duration_ms() >= 10.0);
    }

    #[test]
    fn test_profiler() {
        let mut profiler = Profiler::new();
        
        let phase1 = profiler.start_phase("Phase 1");
        thread::sleep(Duration::from_millis(5));
        profiler.end_phase(phase1);

        let phase2 = profiler.start_phase("Phase 2");
        thread::sleep(Duration::from_millis(10));
        profiler.end_phase(phase2);

        assert!(profiler.total_time_ms() >= 15.0);
        assert_eq!(profiler.phases.len(), 2);
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
        
        let summary = stats.format_summary();
        assert!(summary.contains("100"));
        assert!(summary.contains("500"));
    }
}