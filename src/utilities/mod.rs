//! Utility Modules
//!
//! Shared utilities and helpers for compilation and code generation:
//! - Error reporting and recovery
//! - Code profiling and statistics
//! - Built-in functions and types
//! - Color output for terminal
//! - Module system and visibility

pub mod error_reporting;
pub mod error_recovery;
pub mod profiling;
pub mod builtins;
pub mod colors;
pub mod module_system;
pub mod modules;
pub mod module_visibility_enhanced;
pub mod documentation;

pub use error_reporting::{Diagnostic, ErrorReporter, SourceLocation, Severity};
pub use profiling::{Profiler, CompilationStats};
pub use builtins::BuiltinFunction;
pub use colors::{Color, Colored};
