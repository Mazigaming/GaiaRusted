//! Optimization Passes
//!
//! Various optimization passes and specialization:
//! - Link-time optimization (LTO)
//! - Optimizer implementations
//! - Optimization passes
//! - LLVM IR optimizations

pub mod lto;
pub mod optimizer;
pub mod optimizer_advanced;
pub mod optimization_passes;
pub mod llvm_ir_optimizer;
pub mod const_prop;
pub mod dead_code_elim;
pub mod loop_opt;
pub mod inlining;
