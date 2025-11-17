//! # GiaRusted - A Rust Compiler Library
//!
//! A complete Rust compiler implementation built from scratch, now as a reusable library.
//!
//! ## Compilation Pipeline
//!
//! ```text
//! Rust Source Code
//!     ↓ [Lexer]
//! Token Stream
//!     ↓ [Parser]
//! Abstract Syntax Tree
//!     ↓ [Lowering]
//! High-Level IR
//!     ↓ [Type Checker]
//! Typed HIR
//!     ↓ [Borrow Checker]
//! Memory-Safe HIR
//!     ↓ [MIR Lowering]
//! Mid-Level IR
//!     ↓ [Optimizations]
//! Optimized MIR
//!     ↓ [Codegen]
//! x86-64 Machine Code → Object Files → Executable
//! ```
//!
//! ## Usage as a Library
//!
//! ```ignore
//! use gaiarusted::CompilationConfig;
//! use gaiarusted::compile_files;
//!
//! let config = CompilationConfig::new()
//!     .add_source_file("main.rs")?
//!     .set_output("output")
//!     .set_output_format(OutputFormat::Executable);
//!
//! let result = compile_files(&config)?;
//! ```

// Frontend: Lexing, Parsing & Macros
pub mod lexer;
pub mod parser;
pub mod macros;
pub mod frontend;

// Analysis: Type Checking, Traits, Lifetimes, Pattern Matching
pub mod typechecker;
pub mod typesystem;
pub mod borrowchecker;
pub mod analysis;

// Codegen: IR, Lowering, Code Generation
pub mod lowering;
pub mod mir;
pub mod codegen;

// Runtime: Execution Support
pub mod runtime;

// Compilation Pipeline
pub mod compiler;
pub mod config;
pub mod compiler_integration;
pub mod cargo_api;

// Standard Library
pub mod stdlib;
pub mod iterators;

// Testing Framework
pub mod testing;

// FFI & Interop
pub mod ffi;

// Utilities
pub mod utilities;
pub mod error_reporting {
    pub use crate::utilities::error_reporting::*;
}
pub mod profiling {
    pub use crate::utilities::profiling::*;
}

// Advanced Features
pub mod closures;

// Module re-exports for test compatibility
pub mod builtins {
    pub use crate::utilities::builtins::*;
}
pub mod pattern_matching {
    pub use crate::analysis::pattern_matching::*;
}
pub mod option_result {
    pub use crate::stdlib::option_result::*;
}
pub mod library_api {
    pub use crate::compiler::*;
    pub use crate::config::*;
}
pub mod modules {
    pub use crate::utilities::modules::*;
}

pub use config::{CompilationConfig, OutputFormat};
pub use compiler::{compile_files, CompilationResult, CompileError};
pub use utilities::error_reporting::{Diagnostic, ErrorReporter, SourceLocation, Severity};
pub use utilities::builtins::BuiltinFunction;
pub use utilities::profiling::{Profiler, CompilationStats as ProfileStats};
pub use utilities::colors::{Color, Colored};
pub use cargo_api::{CargoAPI, CargoProject, CargoManifest, CargoBuildConfig, BuildProfile, CrateType};

// Analysis re-exports
pub use analysis::pattern_matching::{PatternAnalyzer, PatternCompiler, ReachabilityChecker};
pub use stdlib::option_result::{Option, Result};

/// Compilation statistics
#[derive(Debug, Clone)]
pub struct CompilationStats {
    pub phase_times: std::collections::HashMap<String, std::time::Duration>,
    pub total_time: std::time::Duration,
    pub files_compiled: usize,
    pub lines_of_code: usize,
}

impl CompilationStats {
    pub fn new() -> Self {
        CompilationStats {
            phase_times: std::collections::HashMap::new(),
            total_time: std::time::Duration::ZERO,
            files_compiled: 0,
            lines_of_code: 0,
        }
    }
}