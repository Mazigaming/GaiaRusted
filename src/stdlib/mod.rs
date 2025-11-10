//! Standard Library
//!
//! Built-in types, traits, and functions:
//! - Option and Result types
//! - Collection trait implementations
//! - I/O operations
//! - String formatting support
//! - Standard library expansion

pub mod stdlib;
pub mod option_result;
pub mod collections_traits;
pub mod io_operations;
pub mod formatting;
pub mod stdlib_expanded;

pub use option_result::{Option, Result};

// Re-export submodules for direct access
pub use stdlib::collections;
pub mod option {
    pub use crate::stdlib::option_result::Option;
}
pub mod result {
    pub use crate::stdlib::option_result::Result;
}
pub mod string {
    pub use crate::stdlib::stdlib::string::String;
}
