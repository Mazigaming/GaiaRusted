//! # GaiaRusted Standard Library
//!
//! Core types and methods that enable practical Rust programs.
//! Includes String, Vec<T>, iterators, and common utility methods.

pub mod strings;
pub mod vec;
pub mod iterators;
pub mod options_results;
pub mod method_resolution;
mod integration_tests;

// Re-export commonly used types and traits
pub use strings::StringType;
pub use vec::VecType;
pub use iterators::{Iterator, IntoIterator};
pub use method_resolution::{StdlibMethodResolver, MethodInfo};

/// Prelude - Types automatically available in all modules
pub mod prelude {
    pub use crate::stdlib::strings::StringType;
    pub use crate::stdlib::vec::VecType;
    pub use crate::stdlib::iterators::{Iterator, IntoIterator};
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
