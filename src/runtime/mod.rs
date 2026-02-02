//! Runtime System
//!
//! Runtime execution environment including:
//! - Async/await support and task execution
//! - Threading and concurrency
//! - Panic handling and error unwinding
//! - Runtime initialization

pub mod async_executor;
pub mod async_lowering;
pub mod async_sync;
pub mod async_types;
pub mod panic_handler;
pub mod runtime;
pub mod threading;
pub mod state_machine_codegen;
pub mod smart_pointer_ops;

pub use runtime::{generate_main_wrapper, generate_runtime_assembly};
pub use state_machine_codegen::{StateMachineCodegen, StateMachineConfig, GeneratedStateMachine};
