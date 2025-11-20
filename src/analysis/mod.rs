//! Analysis Phase
//!
//! Semantic analysis, type checking, and pattern analysis:
//! - Type checking and inference
//! - Borrow checking and lifetime analysis
//! - Pattern matching and exhaustiveness
//! - Trait resolution and bounds checking

pub mod pattern_matching;
pub mod advanced_patterns;
pub mod enhanced_patterns;
pub mod traits;  // Consolidated: sealed traits, advanced traits, trait definitions
pub mod advanced_assoc_types;
pub mod const_generics;
pub mod generic_constraints;
pub mod trait_bounds_inference;
pub mod trait_system;
pub mod type_aliases;
pub mod type_specialization;
pub mod lifetime_inference;
pub mod lifetime_lexing;
pub mod lifetime_parsing;
pub mod lifetime_resolution;
pub mod pattern_exhaustiveness;
pub mod error_propagation;
pub mod associated_types;  // Associated types and where clauses

pub use pattern_matching::{
    EnhancedPatternMatcher, PatternAnalyzer, PatternCompiler, ReachabilityChecker,
    PatternBinding, MatchCompilation, CompiledArm, DestructuringError, DecisionNode,
    PatternMatchResult,
};
