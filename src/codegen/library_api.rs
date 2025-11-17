//! # Enhanced Library API for v0.0.3
//!
//! Provides a user-friendly API for embedding GaiaRusted in other projects.
//!
//! ## Features
//! - Builder pattern for configuration
//! - Incremental compilation
//! - Custom callbacks for compilation phases
//! - Better error handling
//! - Performance metrics and diagnostics
//! - Support for custom built-in functions

use std::path::PathBuf;
use std::collections::HashMap;

/// Compilation phase callback type
pub type PhaseCallback = Box<dyn Fn(&str, u128) + Send + Sync>;

/// Configuration builder for the compiler
pub struct CompilerBuilder {
    source_files: Vec<PathBuf>,
    output_path: PathBuf,
    output_format: String,
    verbose: bool,
    optimize: bool,
    phase_callbacks: HashMap<String, PhaseCallback>,
    custom_builtins: HashMap<String, String>,
}

impl CompilerBuilder {
    /// Create a new compiler builder
    pub fn new() -> Self {
        CompilerBuilder {
            source_files: Vec::new(),
            output_path: PathBuf::from("output"),
            output_format: "executable".to_string(),
            verbose: false,
            optimize: false,
            phase_callbacks: HashMap::new(),
            custom_builtins: HashMap::new(),
        }
    }

    /// Add a source file
    pub fn add_source(mut self, path: impl Into<PathBuf>) -> Self {
        self.source_files.push(path.into());
        self
    }

    /// Add multiple source files
    pub fn add_sources(mut self, paths: Vec<PathBuf>) -> Self {
        self.source_files.extend(paths);
        self
    }

    /// Set output path
    pub fn output(mut self, path: impl Into<PathBuf>) -> Self {
        self.output_path = path.into();
        self
    }

    /// Set output format (executable, object, assembly, library, bash)
    pub fn format(mut self, format: &str) -> Self {
        self.output_format = format.to_string();
        self
    }

    /// Enable/disable verbose output
    pub fn verbose(mut self, enabled: bool) -> Self {
        self.verbose = enabled;
        self
    }

    /// Enable/disable optimizations
    pub fn optimize(mut self, enabled: bool) -> Self {
        self.optimize = enabled;
        self
    }

    /// Register a callback for a compilation phase
    pub fn on_phase<F>(mut self, phase: &str, callback: F) -> Self
    where
        F: Fn(&str, u128) + Send + Sync + 'static,
    {
        self.phase_callbacks
            .insert(phase.to_string(), Box::new(callback));
        self
    }

    /// Register a custom built-in function
    pub fn add_builtin(mut self, name: &str, implementation: &str) -> Self {
        self.custom_builtins
            .insert(name.to_string(), implementation.to_string());
        self
    }

    /// Build the compiler configuration
    pub fn build(self) -> CompilerConfig {
        CompilerConfig {
            source_files: self.source_files,
            output_path: self.output_path,
            output_format: self.output_format,
            verbose: self.verbose,
            optimize: self.optimize,
            phase_callbacks: self.phase_callbacks,
            custom_builtins: self.custom_builtins,
        }
    }
}

impl Default for CompilerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Compiler configuration
pub struct CompilerConfig {
    pub source_files: Vec<PathBuf>,
    pub output_path: PathBuf,
    pub output_format: String,
    pub verbose: bool,
    pub optimize: bool,
    pub phase_callbacks: HashMap<String, PhaseCallback>,
    pub custom_builtins: HashMap<String, String>,
}

impl CompilerConfig {
    /// Get all source files
    pub fn sources(&self) -> &[PathBuf] {
        &self.source_files
    }

    /// Get output path
    pub fn output(&self) -> &PathBuf {
        &self.output_path
    }

    /// Check if verbose mode is enabled
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Check if optimizations are enabled
    pub fn is_optimized(&self) -> bool {
        self.optimize
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.source_files.is_empty() {
            return Err("No source files specified".to_string());
        }

        for file in &self.source_files {
            if !file.exists() {
                return Err(format!("Source file does not exist: {:?}", file));
            }
        }

        match self.output_format.as_str() {
            "executable" | "object" | "assembly" | "library" | "bash" => Ok(()),
            _ => Err(format!("Unknown output format: {}", self.output_format)),
        }
    }
}

/// Compilation result with diagnostics
#[derive(Debug, Clone)]
pub struct CompilationResult {
    pub success: bool,
    pub output_path: PathBuf,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub metrics: CompilationMetrics,
}

/// Compilation metrics and diagnostics
#[derive(Debug, Clone, Default)]
pub struct CompilationMetrics {
    pub total_time_ms: u128,
    pub lexer_time_ms: u128,
    pub parser_time_ms: u128,
    pub typechecker_time_ms: u128,
    pub codegen_time_ms: u128,
    pub lines_of_code: usize,
    pub functions_compiled: usize,
    pub structs_compiled: usize,
    pub optimization_passes: usize,
}

impl CompilationMetrics {
    /// Get phase breakdown as percentage
    pub fn phase_breakdown(&self) -> HashMap<String, f64> {
        let mut breakdown = HashMap::new();
        let total = self.total_time_ms as f64;

        if total > 0.0 {
            breakdown.insert(
                "lexer".to_string(),
                (self.lexer_time_ms as f64 / total) * 100.0,
            );
            breakdown.insert(
                "parser".to_string(),
                (self.parser_time_ms as f64 / total) * 100.0,
            );
            breakdown.insert(
                "typechecker".to_string(),
                (self.typechecker_time_ms as f64 / total) * 100.0,
            );
            breakdown.insert(
                "codegen".to_string(),
                (self.codegen_time_ms as f64 / total) * 100.0,
            );
        }

        breakdown
    }

    /// Get slowest phase
    pub fn slowest_phase(&self) -> Option<(String, u128)> {
        let phases = vec![
            ("lexer", self.lexer_time_ms),
            ("parser", self.parser_time_ms),
            ("typechecker", self.typechecker_time_ms),
            ("codegen", self.codegen_time_ms),
        ];

        phases
            .into_iter()
            .max_by_key(|(_, time)| *time)
            .map(|(name, time)| (name.to_string(), time))
    }
}

/// Trait for custom compilation handlers
pub trait CompilationHandler {
    fn on_phase_start(&self, phase: &str);
    fn on_phase_end(&self, phase: &str, duration_ms: u128);
    fn on_error(&self, error: &str);
    fn on_warning(&self, warning: &str);
    fn on_complete(&self, result: &CompilationResult);
}

/// Default compilation handler
pub struct DefaultHandler;

impl CompilationHandler for DefaultHandler {
    fn on_phase_start(&self, phase: &str) {
        if cfg!(test) == false {
            println!("[✓] Starting phase: {}", phase);
        }
    }

    fn on_phase_end(&self, phase: &str, duration_ms: u128) {
        if cfg!(test) == false {
            println!("[✓] Completed {}: {}ms", phase, duration_ms);
        }
    }

    fn on_error(&self, error: &str) {
        eprintln!("[✗] Error: {}", error);
    }

    fn on_warning(&self, warning: &str) {
        println!("[⚠] Warning: {}", warning);
    }

    fn on_complete(&self, result: &CompilationResult) {
        if result.success {
            println!(
                "[✓] Compilation successful: {:?} ({} ms)",
                result.output_path, result.metrics.total_time_ms
            );
        } else {
            println!("[✗] Compilation failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_builder() {
        let config = CompilerBuilder::new()
            .add_source("test.rs")
            .output("output")
            .format("executable")
            .verbose(true)
            .optimize(true)
            .build();

        assert_eq!(config.source_files.len(), 1);
        assert!(config.is_verbose());
        assert!(config.is_optimized());
    }

    #[test]
    fn test_metrics_phase_breakdown() {
        let mut metrics = CompilationMetrics::default();
        metrics.total_time_ms = 100;
        metrics.lexer_time_ms = 25;
        metrics.parser_time_ms = 25;
        metrics.typechecker_time_ms = 25;
        metrics.codegen_time_ms = 25;

        let breakdown = metrics.phase_breakdown();
        assert_eq!(breakdown.get("lexer"), Some(&25.0));
    }

    #[test]
    fn test_metrics_slowest_phase() {
        let mut metrics = CompilationMetrics::default();
        metrics.lexer_time_ms = 10;
        metrics.parser_time_ms = 50;
        metrics.typechecker_time_ms = 20;
        metrics.codegen_time_ms = 30;

        let slowest = metrics.slowest_phase();
        assert_eq!(slowest, Some(("parser".to_string(), 50)));
    }
}