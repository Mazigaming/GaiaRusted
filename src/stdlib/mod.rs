//! Standard Library
//!
//! Built-in types, traits, and functions:
//! - Option and Result types
//! - Collection trait implementations
//! - I/O operations
//! - String formatting support
//! - Standard library expansion

pub mod option_result;
pub mod collections_traits;
pub mod io_operations;
pub mod formatting;
pub mod stdlib_expanded;
pub mod smart_pointers;
pub mod collections;

pub use option_result::{Option, Result};

// Re-export submodules for direct access
pub mod option {
    pub use crate::stdlib::option_result::Option;
}
pub mod result {
    pub use crate::stdlib::option_result::Result;
}
