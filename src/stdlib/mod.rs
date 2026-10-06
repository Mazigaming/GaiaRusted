//! # GaiaRusted Standard Library
//!
//! Core types and methods that enable practical Rust programs.
//! Includes String, Vec<T>, iterators, file I/O, and common utility methods.

mod integration_tests;
pub mod io_operations;
pub mod iterators;
pub mod method_resolution;
pub mod options_results;
pub mod strings;
pub mod vec;

// Re-export commonly used types and traits
pub use io_operations::{BufferedReader, BufferedWriter, FileHandle, FileMode, FileSystem};
pub use iterators::{IntoIterator, Iterator};
pub use method_resolution::{MethodInfo, StdlibMethodResolver};
pub use strings::StringType;
pub use vec::VecType;

/// Prelude - Types automatically available in all modules
pub mod prelude {
    pub use crate::stdlib::iterators::{IntoIterator, Iterator};
    pub use crate::stdlib::strings::StringType;
    pub use crate::stdlib::vec::VecType;
}

/// Initialize standard library - Called at compiler startup
pub fn init() {
    // Register stdlib types in type system
    // This happens during type system initialization
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdlib_module_loads() {
        // Verify stdlib module is accessible
        let _ = prelude::StringType;
        let _ = prelude::VecType;
    }
}
