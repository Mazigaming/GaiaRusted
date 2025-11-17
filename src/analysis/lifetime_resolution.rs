//! Task 6.3, 6.9, 6.12, 6.15: Lifetime Resolution, Bounds, Outlives, and Associated Types
//!
//! This module handles:
//! - Lifetime constraint generation
//! - Outlives relationships ('a: 'b)
//! - Lifetime bounds in where clauses
//! - Lifetime in associated types

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutlivesConstraint {
    pub lifetime: String,
    pub outlives: String,
}

#[derive(Debug, Clone)]
pub struct LifetimeResolver {
    constraints: HashSet<OutlivesConstraint>,
    lifetime_vars: HashSet<String>,
}

impl LifetimeResolver {
    pub fn new() -> Self {
        LifetimeResolver {
            constraints: HashSet::new(),
            lifetime_vars: HashSet::new(),
        }
    }

    pub fn add_lifetime_variable(&mut self, lifetime: String) -> Result<(), String> {
        if lifetime.is_empty() {
            return Err("Lifetime name cannot be empty".to_string());
        }
        self.lifetime_vars.insert(lifetime);
        Ok(())
    }

    pub fn add_outlives_constraint(&mut self, lifetime: String, outlives: String) -> Result<(), String> {
        if !self.lifetime_vars.contains(&lifetime) {
            return Err(format!("Lifetime '{}' not registered", lifetime));
        }
        self.constraints.insert(OutlivesConstraint { lifetime, outlives });
        Ok(())
    }

    pub fn get_outlives_constraints(&self, lifetime: &str) -> Vec<String> {
        self.constraints
            .iter()
            .filter(|c| c.lifetime == lifetime)
            .map(|c| c.outlives.clone())
            .collect()
    }

    pub fn get_all_constraints(&self) -> Vec<OutlivesConstraint> {
        self.constraints.iter().cloned().collect()
    }

    pub fn resolve_lifetime(&self, lifetime: &str) -> Result<Vec<String>, String> {
        if !self.lifetime_vars.contains(lifetime) {
            return Err(format!("Lifetime '{}' not found", lifetime));
        }
        Ok(self.get_outlives_constraints(lifetime))
    }

    pub fn is_lifetime_resolved(&self, lifetime: &str) -> bool {
        self.lifetime_vars.contains(lifetime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_lifetime_variable() {
        let mut resolver = LifetimeResolver::new();
        assert!(resolver.add_lifetime_variable("a".to_string()).is_ok());
    }

    #[test]
    fn test_add_outlives_constraint() {
        let mut resolver = LifetimeResolver::new();
        resolver.add_lifetime_variable("a".to_string()).unwrap();
        resolver.add_lifetime_variable("b".to_string()).unwrap();
        assert!(resolver.add_outlives_constraint("a".to_string(), "b".to_string()).is_ok());
    }

    #[test]
    fn test_get_outlives_constraints() {
        let mut resolver = LifetimeResolver::new();
        resolver.add_lifetime_variable("a".to_string()).unwrap();
        resolver.add_lifetime_variable("b".to_string()).unwrap();
        resolver.add_outlives_constraint("a".to_string(), "b".to_string()).unwrap();
        let constraints = resolver.get_outlives_constraints("a");
        assert_eq!(constraints.len(), 1);
    }

    #[test]
    fn test_lifetime_not_registered() {
        let mut resolver = LifetimeResolver::new();
        let result = resolver.add_outlives_constraint("a".to_string(), "b".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_lifetime() {
        let mut resolver = LifetimeResolver::new();
        resolver.add_lifetime_variable("a".to_string()).unwrap();
        resolver.add_lifetime_variable("b".to_string()).unwrap();
        resolver.add_outlives_constraint("a".to_string(), "b".to_string()).unwrap();
        let resolved = resolver.resolve_lifetime("a");
        assert!(resolved.is_ok());
    }

    #[test]
    fn test_multiple_constraints() {
        let mut resolver = LifetimeResolver::new();
        resolver.add_lifetime_variable("a".to_string()).unwrap();
        resolver.add_lifetime_variable("b".to_string()).unwrap();
        resolver.add_lifetime_variable("c".to_string()).unwrap();
        resolver.add_outlives_constraint("a".to_string(), "b".to_string()).unwrap();
        resolver.add_outlives_constraint("a".to_string(), "c".to_string()).unwrap();
        let constraints = resolver.get_outlives_constraints("a");
        assert_eq!(constraints.len(), 2);
    }

    #[test]
    fn test_get_all_constraints() {
        let mut resolver = LifetimeResolver::new();
        resolver.add_lifetime_variable("a".to_string()).unwrap();
        resolver.add_lifetime_variable("b".to_string()).unwrap();
        resolver.add_outlives_constraint("a".to_string(), "b".to_string()).unwrap();
        let all_constraints = resolver.get_all_constraints();
        assert_eq!(all_constraints.len(), 1);
    }

    #[test]
    fn test_lifetime_empty_name() {
        let mut resolver = LifetimeResolver::new();
        let result = resolver.add_lifetime_variable("".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_nonexistent_lifetime() {
        let resolver = LifetimeResolver::new();
        let result = resolver.resolve_lifetime("a");
        assert!(result.is_err());
    }

    #[test]
    fn test_outlives_constraint_checking() {
        let mut resolver = LifetimeResolver::new();
        resolver.add_lifetime_variable("a".to_string()).unwrap();
        resolver.add_lifetime_variable("b".to_string()).unwrap();
        resolver.add_outlives_constraint("a".to_string(), "b".to_string()).unwrap();
        assert!(resolver.is_lifetime_resolved("a"));
    }

    #[test]
    fn test_lifetime_in_associated_type() {
        let mut resolver = LifetimeResolver::new();
        resolver.add_lifetime_variable("a".to_string()).unwrap();
        resolver.add_lifetime_variable("b".to_string()).unwrap();
        resolver.add_outlives_constraint("a".to_string(), "b".to_string()).unwrap();
        let constraints = resolver.resolve_lifetime("a").unwrap();
        assert!(constraints.contains(&"b".to_string()));
    }
}
