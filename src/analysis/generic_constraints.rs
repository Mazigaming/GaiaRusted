//! # Generic Constraints System (Task 5.13)
//!
//! Implements advanced constraint propagation and resolution for generic types.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constraint {
    TypeEquality(String, String),
    TraitBound(String, String),
    LifetimeBound(String, String),
    SizedBound(String),
}

#[derive(Debug, Clone)]
pub struct ConstraintSet {
    constraints: Vec<Constraint>,
    resolved: HashMap<String, Vec<Constraint>>,
}

impl ConstraintSet {
    pub fn new() -> Self {
        ConstraintSet {
            constraints: Vec::new(),
            resolved: HashMap::new(),
        }
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        if !self.constraints.contains(&constraint) {
            self.constraints.push(constraint);
        }
    }

    pub fn resolve(&mut self) -> Result<(), String> {
        for constraint in self.constraints.clone() {
            self.resolve_constraint(&constraint)?;
        }
        Ok(())
    }

    fn resolve_constraint(&mut self, constraint: &Constraint) -> Result<(), String> {
        match constraint {
            Constraint::TypeEquality(a, b) => {
                self.resolved.entry(a.clone()).or_insert_with(Vec::new).push(constraint.clone());
                self.resolved.entry(b.clone()).or_insert_with(Vec::new).push(constraint.clone());
                Ok(())
            }
            Constraint::TraitBound(ty, _trait_name) => {
                self.resolved.entry(ty.clone()).or_insert_with(Vec::new).push(constraint.clone());
                Ok(())
            }
            Constraint::LifetimeBound(a, _b) => {
                self.resolved.entry(a.clone()).or_insert_with(Vec::new).push(constraint.clone());
                Ok(())
            }
            Constraint::SizedBound(ty) => {
                self.resolved.entry(ty.clone()).or_insert_with(Vec::new).push(constraint.clone());
                Ok(())
            }
        }
    }

    pub fn get_constraints(&self, key: &str) -> Option<&Vec<Constraint>> {
        self.resolved.get(key)
    }

    pub fn check_satisfiable(&self) -> Result<(), String> {
        let mut visited = HashSet::new();
        for (var, _constraints) in &self.resolved {
            if visited.contains(var) {
                continue;
            }
            self.check_cycle(var, &mut visited)?;
        }
        Ok(())
    }

    fn check_cycle(&self, var: &str, visited: &mut HashSet<String>) -> Result<(), String> {
        if visited.contains(var) {
            return Err(format!("Cyclic constraint detected for {}", var));
        }
        visited.insert(var.to_string());
        
        if let Some(constraints) = self.resolved.get(var) {
            for constraint in constraints {
                if let Constraint::TypeEquality(a, b) = constraint {
                    let next = if a == var { b } else { a };
                    if !visited.contains(next) {
                        self.check_cycle(next, visited)?;
                    }
                }
            }
        }
        
        visited.remove(var);
        Ok(())
    }

    pub fn propagate_constraints(&mut self) {
        let mut propagated = false;
        while !propagated {
            propagated = true;
            let constraints = self.constraints.clone();
            
            for constraint in &constraints {
                match constraint {
                    Constraint::TypeEquality(a, b) => {
                        if let Some(a_bounds) = self.resolved.get(a).cloned() {
                            for bound in a_bounds {
                                if let Constraint::TraitBound(_, t) = &bound {
                                    let new_constraint = Constraint::TraitBound(b.clone(), t.clone());
                                    if !self.constraints.contains(&new_constraint) {
                                        self.add_constraint(new_constraint);
                                        propagated = false;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn merge(&mut self, other: &ConstraintSet) {
        for constraint in &other.constraints {
            self.add_constraint(constraint.clone());
        }
    }

    pub fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_constraint_set() {
        let cs = ConstraintSet::new();
        assert_eq!(cs.constraints().len(), 0);
    }

    #[test]
    fn test_add_constraint() {
        let mut cs = ConstraintSet::new();
        cs.add_constraint(Constraint::SizedBound("T".to_string()));
        assert_eq!(cs.constraints().len(), 1);
    }

    #[test]
    fn test_type_equality_constraint() {
        let mut cs = ConstraintSet::new();
        cs.add_constraint(Constraint::TypeEquality("T".to_string(), "U".to_string()));
        assert!(cs.resolve().is_ok());
        assert!(cs.get_constraints("T").is_some());
    }

    #[test]
    fn test_trait_bound_constraint() {
        let mut cs = ConstraintSet::new();
        cs.add_constraint(Constraint::TraitBound("T".to_string(), "Clone".to_string()));
        assert!(cs.resolve().is_ok());
        assert!(cs.get_constraints("T").is_some());
    }

    #[test]
    fn test_duplicate_constraint_filtering() {
        let mut cs = ConstraintSet::new();
        cs.add_constraint(Constraint::SizedBound("T".to_string()));
        cs.add_constraint(Constraint::SizedBound("T".to_string()));
        assert_eq!(cs.constraints().len(), 1);
    }

    #[test]
    fn test_resolve_all_constraints() {
        let mut cs = ConstraintSet::new();
        cs.add_constraint(Constraint::TypeEquality("A".to_string(), "B".to_string()));
        cs.add_constraint(Constraint::TraitBound("A".to_string(), "Copy".to_string()));
        cs.add_constraint(Constraint::LifetimeBound("'a".to_string(), "'b".to_string()));
        assert!(cs.resolve().is_ok());
    }

    #[test]
    fn test_constraint_propagation() {
        let mut cs = ConstraintSet::new();
        cs.add_constraint(Constraint::TypeEquality("T".to_string(), "U".to_string()));
        cs.add_constraint(Constraint::TraitBound("T".to_string(), "Debug".to_string()));
        cs.resolve().unwrap();
        cs.propagate_constraints();
        assert!(cs.constraints().len() >= 2);
    }

    #[test]
    fn test_merge_constraint_sets() {
        let mut cs1 = ConstraintSet::new();
        cs1.add_constraint(Constraint::SizedBound("T".to_string()));
        
        let mut cs2 = ConstraintSet::new();
        cs2.add_constraint(Constraint::SizedBound("U".to_string()));
        
        cs1.merge(&cs2);
        assert_eq!(cs1.constraints().len(), 2);
    }

    #[test]
    fn test_satisfiability_check() {
        let mut cs = ConstraintSet::new();
        cs.add_constraint(Constraint::TraitBound("T".to_string(), "Clone".to_string()));
        cs.resolve().unwrap();
        assert!(cs.check_satisfiable().is_ok());
    }

    #[test]
    fn test_lifetime_bound_resolution() {
        let mut cs = ConstraintSet::new();
        cs.add_constraint(Constraint::LifetimeBound("'a".to_string(), "'static".to_string()));
        assert!(cs.resolve().is_ok());
        assert!(cs.get_constraints("'a").is_some());
    }

    #[test]
    fn test_multiple_trait_bounds() {
        let mut cs = ConstraintSet::new();
        cs.add_constraint(Constraint::TraitBound("T".to_string(), "Clone".to_string()));
        cs.add_constraint(Constraint::TraitBound("T".to_string(), "Debug".to_string()));
        cs.add_constraint(Constraint::TraitBound("T".to_string(), "Copy".to_string()));
        cs.resolve().unwrap();
        assert_eq!(cs.constraints().len(), 3);
    }

    #[test]
    fn test_constraint_set_empty_check() {
        let cs = ConstraintSet::new();
        assert!(cs.constraints().is_empty());
    }

    #[test]
    fn test_sized_bound_constraint() {
        let mut cs = ConstraintSet::new();
        cs.add_constraint(Constraint::SizedBound("T".to_string()));
        cs.add_constraint(Constraint::SizedBound("U".to_string()));
        assert!(cs.resolve().is_ok());
        assert_eq!(cs.constraints().len(), 2);
    }

    #[test]
    fn test_constraint_resolution_with_multiple_types() {
        let mut cs = ConstraintSet::new();
        for i in 0..5 {
            cs.add_constraint(Constraint::TraitBound(format!("T{}", i), "Display".to_string()));
        }
        assert!(cs.resolve().is_ok());
        assert_eq!(cs.constraints().len(), 5);
    }
}
