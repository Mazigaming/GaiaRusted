//! # Advanced Monomorphization for Iterators
//!
//! Implements compile-time specialization of generic iterator code
//! for different type combinations, enabling:
//! - Type-specific optimizations (e.g., memcpy for Pod types)
//! - Specialized hot paths (e.g., fast path for i64)
//! - Code size control via selective instantiation
//! - Optimization hints for LLVM (e.g., loop unrolling)

use std::collections::HashMap;

/// Monomorphization key - uniquely identifies a type specialization
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MonomorphKey {
    /// Specialization for i64
    Int64,
    /// Specialization for bool
    Bool,
    /// Specialization for f64
    Float64,
    /// Specialization for string pointers
    String,
    /// Generic (unspecialized) specialization
    Generic(String),
}

impl MonomorphKey {
    /// Get a human-readable name for this specialization
    pub fn name(&self) -> String {
        match self {
            MonomorphKey::Int64 => "i64".to_string(),
            MonomorphKey::Bool => "bool".to_string(),
            MonomorphKey::Float64 => "f64".to_string(),
            MonomorphKey::String => "String".to_string(),
            MonomorphKey::Generic(name) => format!("Generic<{}>", name),
        }
    }

    /// Check if this is a "hot type" that should be specialized
    pub fn is_hot_type(&self) -> bool {
        matches!(self, MonomorphKey::Int64 | MonomorphKey::Bool | MonomorphKey::Float64)
    }

    /// Get estimated code size for this specialization (in bytes)
    pub fn estimated_size(&self) -> usize {
        match self {
            MonomorphKey::Int64 => 256,  // Highly optimizable
            MonomorphKey::Bool => 128,   // Very small
            MonomorphKey::Float64 => 256,
            MonomorphKey::String => 512,  // More complex
            MonomorphKey::Generic(_) => 1024, // Generic fallback
        }
    }
}

/// Specialization strategy - controls which types to monomorphize
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecializationStrategy {
    /// Monomorphize everything (larger code, faster execution)
    Aggressive,
    /// Monomorphize only hot types (balanced)
    Balanced,
    /// Minimal monomorphization (smaller code, slower)
    Conservative,
}

impl SpecializationStrategy {
    /// Check if a type should be monomorphized under this strategy
    pub fn should_monomorphize(&self, key: &MonomorphKey) -> bool {
        match self {
            SpecializationStrategy::Aggressive => true,
            SpecializationStrategy::Balanced => key.is_hot_type(),
            SpecializationStrategy::Conservative => false,
        }
    }
}

/// Monomorphization context - tracks generated specializations
pub struct MonomorphContext {
    /// Map from type keys to generated code
    generated: HashMap<MonomorphKey, String>,
    /// Strategy for this compilation
    strategy: SpecializationStrategy,
    /// Total code size budget (bytes)
    code_budget: usize,
    /// Current code size (bytes)
    current_size: usize,
}

impl MonomorphContext {
    /// Create a new monomorphization context
    pub fn new(strategy: SpecializationStrategy, code_budget: usize) -> Self {
        MonomorphContext {
            generated: HashMap::new(),
            strategy,
            code_budget,
            current_size: 0,
        }
    }

    /// Register a type specialization
    /// Returns true if the specialization was added, false if budget exceeded
    pub fn register(&mut self, key: MonomorphKey, code: String) -> bool {
        let size = code.len();

        if !self.strategy.should_monomorphize(&key) {
            return false;
        }

        if self.current_size + size > self.code_budget {
            eprintln!(
                "Monomorphization budget exceeded: {} + {} > {}",
                self.current_size, size, self.code_budget
            );
            return false;
        }

        self.generated.insert(key, code);
        self.current_size += size;
        true
    }

    /// Get specialization code if available
    pub fn get(&self, key: &MonomorphKey) -> Option<&str> {
        self.generated.get(key).map(|s| s.as_str())
    }

    /// Get all generated specializations
    pub fn all_specializations(&self) -> impl Iterator<Item = (&MonomorphKey, &String)> {
        self.generated.iter()
    }

    /// Report statistics on monomorphization
    pub fn report_stats(&self) {
        let total = self.generated.len();
        let size = self.current_size;
        let budget = self.code_budget;
        let remaining = budget.saturating_sub(size);

        println!("Monomorphization Statistics:");
        println!("  Total specializations: {}", total);
        println!("  Code size: {} / {} bytes", size, budget);
        println!("  Remaining budget: {} bytes", remaining);
        println!("  Strategy: {:?}", self.strategy);

        for (key, code) in &self.generated {
            println!("    - {} ({}B)", key.name(), code.len());
        }
    }
}

/// Iterator specializer - generates optimized code for specific types
pub struct IteratorSpecializer;

impl IteratorSpecializer {
    /// Generate optimized fold code for a specific type
    pub fn specialize_fold(key: &MonomorphKey) -> String {
        match key {
            MonomorphKey::Int64 => {
                // Specialized code for i64 with unrolling hints
                r#"
// Specialized fold for i64 - loop unroll opportunities
#[inline(always)]
fn fold_i64(vec: &[i64], init: i64) -> i64 {
    let mut acc = init;
    // Process 4 at a time for better ILP
    let mut i = 0;
    while i + 4 <= vec.len() {
        acc += vec[i];
        acc += vec[i + 1];
        acc += vec[i + 2];
        acc += vec[i + 3];
        i += 4;
    }
    // Handle remainder
    while i < vec.len() {
        acc += vec[i];
        i += 1;
    }
    acc
}
                "#.to_string()
            }
            MonomorphKey::Bool => {
                // Specialized code for bool - branchless operations
                r#"
// Specialized fold for bool - branch-free counting
#[inline(always)]
fn fold_bool(vec: &[bool], init: i64) -> i64 {
    let mut count = init;
    for &b in vec {
        count += b as i64;
    }
    count
}
                "#.to_string()
            }
            MonomorphKey::Float64 => {
                // Specialized code for f64 with IEEE semantics
                r#"
// Specialized fold for f64
#[inline(always)]
fn fold_f64(vec: &[f64], init: f64) -> f64 {
    let mut acc = init;
    for &item in vec {
        acc += item;
    }
    acc
}
                "#.to_string()
            }
            _ => {
                // Generic fallback
                r#"
// Generic fold
fn fold_generic<T: Clone>(vec: &[T], init: T, f: fn(T, T) -> T) -> T {
    let mut acc = init;
    for item in vec {
        acc = f(acc, item.clone());
    }
    acc
}
                "#.to_string()
            }
        }
    }

    /// Generate optimized filter code for a specific type
    pub fn specialize_filter(key: &MonomorphKey) -> String {
        match key {
            MonomorphKey::Int64 => {
                // Specialized code for i64 filtering
                r#"
#[inline]
fn filter_i64(vec: &[i64], predicate: fn(i64) -> bool) -> Vec<i64> {
    vec.iter().copied().filter(|x| predicate(*x)).collect()
}
                "#.to_string()
            }
            _ => {
                // Generic fallback
                r#"
fn filter_generic<T: Clone>(
    vec: &[T],
    predicate: fn(&T) -> bool,
) -> Vec<T> {
    vec.iter().filter(|x| predicate(x)).cloned().collect()
}
                "#.to_string()
            }
        }
    }

    /// Generate optimized map code for a specific type
    pub fn specialize_map(key: &MonomorphKey) -> String {
        match key {
            MonomorphKey::Int64 => {
                // Specialized code for i64 mapping with arithmetic optimization
                r#"
#[inline]
fn map_i64(vec: &[i64], f: fn(i64) -> i64) -> Vec<i64> {
    vec.iter().map(|x| f(*x)).collect()
}
                "#.to_string()
            }
            _ => {
                // Generic fallback
                r#"
fn map_generic<T: Clone, U>(vec: &[T], f: fn(T) -> U) -> Vec<U> {
    vec.iter().map(|x| f(x.clone())).collect()
}
                "#.to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monomorph_key_equality() {
        assert_eq!(MonomorphKey::Int64, MonomorphKey::Int64);
        assert_ne!(MonomorphKey::Int64, MonomorphKey::Bool);
    }

    #[test]
    fn test_monomorph_key_name() {
        assert_eq!(MonomorphKey::Int64.name(), "i64");
        assert_eq!(MonomorphKey::Bool.name(), "bool");
        assert!(MonomorphKey::Generic("Custom".to_string()).name().contains("Custom"));
    }

    #[test]
    fn test_specialization_strategy() {
        let aggressive = SpecializationStrategy::Aggressive;
        let balanced = SpecializationStrategy::Balanced;
        let conservative = SpecializationStrategy::Conservative;

        assert!(aggressive.should_monomorphize(&MonomorphKey::String));
        assert!(balanced.should_monomorphize(&MonomorphKey::Int64));
        assert!(!balanced.should_monomorphize(&MonomorphKey::String));
        assert!(!conservative.should_monomorphize(&MonomorphKey::Int64));
    }

    #[test]
    fn test_monomorph_context_budget() {
        let mut ctx = MonomorphContext::new(SpecializationStrategy::Aggressive, 2000);
        let key1 = MonomorphKey::Int64;
        let code1 = "x".repeat(500);

        assert!(ctx.register(key1.clone(), code1));
        assert_eq!(ctx.current_size, 500);

        let key2 = MonomorphKey::Bool;
        let code2 = "y".repeat(1600);
        assert!(!ctx.register(key2, code2)); // Exceeds budget
    }

    #[test]
    fn test_iterator_specializer_fold() {
        let code = IteratorSpecializer::specialize_fold(&MonomorphKey::Int64);
        assert!(code.contains("fold_i64"));
        assert!(code.contains("inline"));
    }
}
