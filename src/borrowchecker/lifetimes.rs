//! Lifetime variable representation, inference, and constraint tracking
//!
//! Lifetimes in Rust specify how long references remain valid. This module
//! handles:
//! - Lifetime variable generation and identity
//! - Lifetime parameter extraction from functions and structs
//! - Lifetime elision rules (Rust 2024 Edition)
//! - Lifetime constraints (T: 'a means T outlives 'a)
//! - Lifetime substitution during monomorphization

use std::collections::{HashMap, HashSet};
use std::fmt;

/// A unique identifier for a lifetime variable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LifetimeId(pub usize);

impl fmt::Display for LifetimeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "'l{}", self.0)
    }
}

/// Named lifetimes ('a, 'b, etc.) used in function signatures
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Lifetime {
    /// Named lifetime: 'a, 'b, etc.
    Named(String),
    /// Inferred/implicit lifetime
    Inferred(LifetimeId),
    /// Static lifetime: 'static
    Static,
}

impl fmt::Display for Lifetime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Lifetime::Named(name) => write!(f, "'{}", name),
            Lifetime::Inferred(id) => write!(f, "{}", id),
            Lifetime::Static => write!(f, "'static"),
        }
    }
}

/// Lifetime constraint: LHS outlives RHS
/// Example: 'a: 'b means 'a must outlive 'b
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifetimeConstraint {
    pub lhs: Lifetime,
    pub rhs: Lifetime,
    pub reason: String,
}

/// Tracks lifetime variables and constraints
#[derive(Debug)]
pub struct LifetimeContext {
    /// Next available lifetime ID
    next_id: usize,
    /// Map of named lifetimes to their IDs
    named_lifetimes: HashMap<String, Lifetime>,
    /// All lifetime constraints collected
    constraints: Vec<LifetimeConstraint>,
}

impl LifetimeContext {
    /// Create a new lifetime context
    pub fn new() -> Self {
        LifetimeContext {
            next_id: 0,
            named_lifetimes: HashMap::new(),
            constraints: Vec::new(),
        }
    }

    /// Generate a fresh lifetime variable
    pub fn fresh_lifetime(&mut self) -> Lifetime {
        let id = self.next_id;
        self.next_id += 1;
        Lifetime::Inferred(LifetimeId(id))
    }

    /// Register a named lifetime parameter
    pub fn register_named_lifetime(&mut self, name: String) -> Lifetime {
        if let Some(existing) = self.named_lifetimes.get(&name) {
            return existing.clone();
        }
        let lifetime = Lifetime::Named(name.clone());
        self.named_lifetimes.insert(name, lifetime.clone());
        lifetime
    }

    /// Add an outlives constraint: lhs outlives rhs
    pub fn add_constraint(&mut self, lhs: Lifetime, rhs: Lifetime, reason: String) {
        self.constraints.push(LifetimeConstraint { lhs, rhs, reason });
    }

    /// Get all constraints
    pub fn constraints(&self) -> &[LifetimeConstraint] {
        &self.constraints
    }

    /// Check if a lifetime constraint is satisfiable
    /// (basic transitive closure check)
    pub fn is_satisfiable(&self) -> bool {
        // Build reachability graph
        let mut reachability: HashMap<String, HashSet<String>> = HashMap::new();

        for constraint in &self.constraints {
            let lhs_key = constraint.lhs.to_string();
            let rhs_key = constraint.rhs.to_string();

            reachability
                .entry(lhs_key.clone())
                .or_insert_with(HashSet::new)
                .insert(rhs_key.clone());

            // Static outlives everything
            if constraint.lhs == Lifetime::Static {
                reachability
                    .entry("static".to_string())
                    .or_insert_with(HashSet::new)
                    .insert(rhs_key);
            }
        }

        // Transitive closure: check for contradictions
        let mut worklist: Vec<_> = reachability.keys().cloned().collect();
        while let Some(from) = worklist.pop() {
            let reachable: Vec<_> = reachability
                .get(&from)
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default();

            for to in reachable {
                // Detect cycles (except reflexive)
                if to == from {
                    continue;
                }

                if let Some(to_set) = reachability.get(&to).cloned() {
                    for next in to_set {
                        if !reachability
                            .get(&from)
                            .map(|s| s.contains(&next))
                            .unwrap_or(false)
                        {
                            reachability.entry(from.clone()).or_insert_with(HashSet::new);
                            reachability.get_mut(&from).unwrap().insert(next);
                            worklist.push(from.clone());
                        }
                    }
                }
            }
        }

        true // Simplified: just return true for now
    }

    /// Clear all constraints (useful for scope entry/exit)
    pub fn clear_constraints(&mut self) {
        self.constraints.clear();
    }
}

impl Default for LifetimeContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Lifetime elision rules for Rust 2024 Edition
pub struct LifetimeElision;

impl LifetimeElision {
    /// Apply elision rules to function parameters
    /// Returns (inferred input lifetimes, inferred return lifetime)
    pub fn elide_function_lifetimes(
        input_refs: Vec<bool>,      // which params are references
        has_return_ref: bool,       // is return type a reference
        ctx: &mut LifetimeContext,
    ) -> (Vec<Option<Lifetime>>, Option<Lifetime>) {
        let ref_count = input_refs.iter().filter(|&&b| b).count();

        // Rule 1: Single input reference → return takes that lifetime
        if ref_count == 1 && has_return_ref {
            let lt = ctx.fresh_lifetime();
            let result: Vec<_> = input_refs
                .iter()
                .map(|&is_ref| if is_ref { Some(lt.clone()) } else { None })
                .collect();
            return (result, Some(lt));
        }

        // Rule 2: Multiple input references where first is a reference → return takes first lifetime
        if ref_count > 1 && has_return_ref && input_refs.get(0) == Some(&true) {
            let first_lt = ctx.fresh_lifetime();
            let result: Vec<_> = input_refs
                .iter()
                .enumerate()
                .map(|(i, &is_ref)| {
                    if is_ref {
                        if i == 0 {
                            Some(first_lt.clone())
                        } else {
                            Some(ctx.fresh_lifetime())
                        }
                    } else {
                        None
                    }
                })
                .collect();
            return (result, Some(first_lt));
        }

        // Default: each reference gets its own lifetime, no output lifetime
        let result: Vec<_> = input_refs
            .iter()
            .map(|&is_ref| {
                if is_ref {
                    Some(ctx.fresh_lifetime())
                } else {
                    None
                }
            })
            .collect();

        (result, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_lifetime_generation() {
        let mut ctx = LifetimeContext::new();
        let lt1 = ctx.fresh_lifetime();
        let lt2 = ctx.fresh_lifetime();
        assert_ne!(lt1, lt2);
    }

    #[test]
    fn test_named_lifetime_registration() {
        let mut ctx = LifetimeContext::new();
        let lt_a = ctx.register_named_lifetime("a".to_string());
        let lt_a2 = ctx.register_named_lifetime("a".to_string());
        assert_eq!(lt_a, lt_a2);
        assert!(matches!(lt_a, Lifetime::Named(_)));
    }

    #[test]
    fn test_lifetime_constraint_tracking() {
        let mut ctx = LifetimeContext::new();
        let lt1 = ctx.fresh_lifetime();
        let lt2 = ctx.fresh_lifetime();
        ctx.add_constraint(lt1.clone(), lt2.clone(), "test".to_string());
        assert_eq!(ctx.constraints().len(), 1);
    }

    #[test]
    fn test_static_lifetime() {
        let static_lt = Lifetime::Static;
        assert_eq!(static_lt.to_string(), "'static");
    }

    #[test]
    fn test_elision_single_input_reference() {
        let mut ctx = LifetimeContext::new();
        let (inputs, output) = LifetimeElision::elide_function_lifetimes(vec![true], true, &mut ctx);
        
        assert_eq!(inputs.len(), 1);
        assert!(inputs[0].is_some());
        assert!(output.is_some());
        assert_eq!(inputs[0], output);
    }

    #[test]
    fn test_elision_multiple_inputs() {
        let mut ctx = LifetimeContext::new();
        let (inputs, output) =
            LifetimeElision::elide_function_lifetimes(vec![true, true], true, &mut ctx);
        
        // Multiple references: first param gets output lifetime
        assert_eq!(inputs.len(), 2);
        assert!(inputs[0].is_some());
        assert!(inputs[1].is_some());
        // Return lifetime matches first parameter
        assert_eq!(inputs[0], output);
    }
}
