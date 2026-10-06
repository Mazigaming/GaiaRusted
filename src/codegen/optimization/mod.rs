//! Optimization Passes
//!
//! Various optimization passes and specialization:
//! - Link-time optimization (LTO)
//! - Optimizer implementations
//! - Optimization passes
//! - LLVM IR optimizations

pub mod const_prop;
pub mod dead_code_elim;
pub mod inlining;
pub mod llvm_ir_optimizer;
pub mod loop_opt;
pub mod lto;
pub mod optimization_passes;
pub mod optimizer;
pub mod optimizer_advanced;
