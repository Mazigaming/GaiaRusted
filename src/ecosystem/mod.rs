//! Ecosystem Integration for GaiaRusted
//!
//! Complete ecosystem support including:
//! - Package manager integration (Cargo improvements)
//! - Standard library bindings
//! - Community package registry
//! - Workspace support enhancements

pub mod package_manager;
pub mod stdlib_bindings;
pub mod registry;
pub mod workspace;

pub use package_manager::{PackageManager, PackageInfo, PackageResolution};
pub use stdlib_bindings::{StdlibBinding, StdlibModule};
pub use registry::{PackageRegistry, PackageMetadata};
pub use workspace::{Workspace, WorkspaceConfig, WorkspaceMember};
