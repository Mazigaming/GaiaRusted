//! Standard Library
//!
//! Built-in types, traits, and functions:
//! - Option and Result types
//! - Collection trait implementations
//! - I/O operations
//! - Networking (TCP/UDP, HTTP)
//! - String formatting support
//! - Standard library expansion

pub mod option_result;
pub mod collections_traits;
pub mod io_operations;
pub mod advanced_file_io;
pub mod formatting;
pub mod stdlib_expanded;
pub mod smart_pointers;
pub mod collections;
pub mod networking;
pub mod json;
pub mod paths;
pub mod advanced_error_handling;
pub mod advanced_collections;
pub mod math_functions;
pub mod random;

pub use option_result::{Option, Result};

// Re-export submodules for direct access
pub mod option {
    pub use crate::stdlib::option_result::Option;
}
pub mod result {
    pub use crate::stdlib::option_result::Result;
}
