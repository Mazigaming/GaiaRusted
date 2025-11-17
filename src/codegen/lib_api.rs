//! # Library Embedding API (v0.0.3)
//!
//! High-level API for embedding GaiaRusted in other Rust projects
//! Provides a fluent interface for compilation configuration and control

use crate::compiler::{CompilationResult, CompileError};
use crate::config::{CompilationConfig, OutputFormat};
use std::path::Path;

/// Builder pattern for creating compilation configurations
#[derive(Clone)]
pub struct CompilerBuilder {
    config: CompilationConfig,
}

impl CompilerBuilder {
    /// Creates a new compiler builder
    pub fn new() -> Self {
        CompilerBuilder {
            config: CompilationConfig::new(),
        }
    }

    /// Adds a source file to compile
    pub fn add_source_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, String> {
        self.config.source_files.push(path.as_ref().to_path_buf());
        Ok(self)
    }

    /// Adds multiple source files
    pub fn add_source_files<P: AsRef<Path>>(mut self, paths: Vec<P>) -> Result<Self, String> {
        for path in paths {
            self.config.source_files.push(path.as_ref().to_path_buf());
        }
        Ok(self)
    }

    /// Sets the output file path
    pub fn output<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config.output_path = path.as_ref().to_path_buf();
        self
    }

    /// Sets the output format
    pub fn format(mut self, format: OutputFormat) -> Self {
        self.config.output_format = format;
        self
    }

    /// Enables verbose output
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.config.verbose = verbose;
        self
    }

    /// Enables optimizations
    pub fn optimize(mut self, level: u32) -> Self {
        self.config.opt_level = level;
        self
    }

    /// Enables debug info
    pub fn with_debug(mut self, enable: bool) -> Self {
        self.config.debug = enable;
        self
    }

    /// Gets the configuration
    pub fn config(&self) -> &CompilationConfig {
        &self.config
    }

    /// Gets mutable configuration
    pub fn config_mut(&mut self) -> &mut CompilationConfig {
        &mut self.config
    }

    /// Builds and compiles
    pub fn compile(self) -> Result<CompilationResult, CompileError> {
        crate::compiler::compile_files(&self.config)
    }
}

impl Default for CompilerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Compiler session - manages state across multiple compilations
pub struct CompilerSession {
    builder: CompilerBuilder,
    last_result: Option<CompilationResult>,
}

impl CompilerSession {
    /// Creates a new compiler session
    pub fn new() -> Self {
        CompilerSession {
            builder: CompilerBuilder::new(),
            last_result: None,
        }
    }

    /// Gets the builder
    pub fn builder(&mut self) -> &mut CompilerBuilder {
        &mut self.builder
    }

    /// Compiles with current configuration
    pub fn compile(&mut self) -> Result<CompilationResult, CompileError> {
        let result = self.builder.clone().compile()?;
        self.last_result = Some(result.clone());
        Ok(result)
    }

    /// Gets the last compilation result
    pub fn last_result(&self) -> Option<&CompilationResult> {
        self.last_result.as_ref()
    }

    /// Resets the session
    pub fn reset(&mut self) {
        self.builder = CompilerBuilder::new();
        self.last_result = None;
    }
}

impl Default for CompilerSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Pipeline builder - for complex multi-stage compilations
pub struct PipelineBuilder {
    stages: Vec<Box<dyn Fn(&CompilationResult) -> Result<CompilationResult, CompileError>>>,
}

impl PipelineBuilder {
    /// Creates a new pipeline builder
    pub fn new() -> Self {
        PipelineBuilder { stages: Vec::new() }
    }

    /// Adds a compilation stage
    pub fn add_stage<F>(mut self, stage: F) -> Self
    where
        F: Fn(&CompilationResult) -> Result<CompilationResult, CompileError> + 'static,
    {
        self.stages.push(Box::new(stage));
        self
    }

    /// Executes the pipeline
    pub fn execute(&self, initial: CompilationResult) -> Result<CompilationResult, CompileError> {
        let mut result = initial;
        for stage in &self.stages {
            result = stage(&result)?;
        }
        Ok(result)
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Callback-based compilation interface
pub trait CompilationObserver {
    /// Called when compilation starts
    fn on_start(&mut self) {}

    /// Called on each phase completion
    fn on_phase_complete(&mut self, _phase_name: &str, _duration_ms: u64) {}

    /// Called on compilation success
    fn on_success(&mut self, _result: &CompilationResult) {}

    /// Called on compilation error
    fn on_error(&mut self, _error: &CompileError) {}

    /// Called when compilation ends
    fn on_complete(&mut self) {}
}

/// Default observer implementation
pub struct DefaultCompilationObserver;

impl CompilationObserver for DefaultCompilationObserver {
    fn on_phase_complete(&mut self, _phase_name: &str, _duration_ms: u64) {}
    fn on_success(&mut self, _result: &CompilationResult) {}
    fn on_error(&mut self, _error: &CompileError) {}
}

/// Observed compilation - wraps compilation with callbacks
pub struct ObservedCompilation {
    observer: Box<dyn CompilationObserver>,
}

impl ObservedCompilation {
    /// Creates a new observed compilation
    pub fn new<O: CompilationObserver + 'static>(observer: O) -> Self {
        ObservedCompilation {
            observer: Box::new(observer),
        }
    }

    /// Compiles with observation
    pub fn compile(&mut self, config: &CompilationConfig) -> Result<CompilationResult, CompileError> {
        self.observer.on_start();
        
        match crate::compiler::compile_files(config) {
            Ok(result) => {
                self.observer.on_success(&result);
                self.observer.on_complete();
                Ok(result)
            }
            Err(err) => {
                self.observer.on_error(&err);
                self.observer.on_complete();
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_builder() {
        let builder = CompilerBuilder::new()
            .verbose(true)
            .optimize(2)
            .format(OutputFormat::Executable);

        assert_eq!(builder.config().verbose, true);
    }

    #[test]
    fn test_compiler_session() {
        let _session = CompilerSession::new();
        assert!(_session.last_result().is_none());
    }

    #[test]
    fn test_pipeline_builder() {
        let _pipeline = PipelineBuilder::new();
        // Pipeline execution would require mock compilation
    }
}