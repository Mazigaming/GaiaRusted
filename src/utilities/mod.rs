//! Utility Modules
//!
//! Shared utilities and helpers for compilation and code generation:
//! - Error reporting and recovery
//! - Code profiling and statistics
//! - Built-in functions and types
//! - Color output for terminal
//! - Module system and visibility
//! - DWARF debug information generation
//! - NUMA-aware memory allocation

pub mod error_reporting;
pub mod error_recovery;
pub mod profiling;
pub mod builtins;
pub mod colors;
pub mod string_methods;
pub mod module_system;
pub mod modules;
pub mod module_visibility_enhanced;
pub mod advanced_module_system;
pub mod documentation;
pub mod dwarf_debug;
pub mod gdb_integration;
pub mod numa_allocation;

pub use error_reporting::{Diagnostic, ErrorReporter, SourceLocation, Severity};
pub use profiling::{Profiler, CompilationStats};
pub use builtins::BuiltinFunction;
pub use colors::{Color, Colored};
pub use advanced_module_system::{
    AdvancedModuleSystem, ModuleConstant, ModuleVisibility, ModuleReexport,
    NamespaceAlias, ImportResolution,
};
pub use numa_allocation::{
    NumaAllocator, NumaConfig, AllocationPolicy, AllocationInfo, NumaNodeInfo,
    NumaAllocationReport, NumaNodeSummary,
};
