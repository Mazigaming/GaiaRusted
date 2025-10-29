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

pub mod lexer;
pub mod parser;
pub mod lowering;
pub mod typechecker;
pub mod borrowchecker;
pub mod mir;
pub mod codegen;
pub mod config;
pub mod compiler;

pub use config::{CompilationConfig, OutputFormat};
pub use compiler::{compile_files, CompilationResult, CompileError};

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