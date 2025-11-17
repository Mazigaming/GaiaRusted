//! # Lifetime Inference System
//!
//! Advanced lifetime parameter inference and constraint propagation:
//! - Lifetime elision rules (input/output positions)
//! - Lifetime parameter constraints from usage
//! - Bound lifetime variable tracking
//! - Region outlives constraints
//! - Lifetime variance resolution

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LifetimeParam {
    pub name: String,
    pub bounds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifetimeConstraint {
    OutlivesLifetime(String, String),
    OutlivesType(String, String),
    Equal(String, String),
}

#[derive(Debug, Clone)]
pub struct FunctionLifetimes {
    pub params: Vec<LifetimeParam>,
    pub return_lifetime: Option<String>,
    pub input_positions: Vec<String>,
    pub output_positions: Vec<String>,
}

pub struct LifetimeInferenceEngine {
    functions: HashMap<String, FunctionLifetimes>,
    constraints: Vec<LifetimeConstraint>,
    substitutions: HashMap<String, String>,
    variance_map: HashMap<String, Variance>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variance {
    Covariant,
    Contravariant,
    Invariant,
}

impl LifetimeInferenceEngine {
    pub fn new() -> Self {
        LifetimeInferenceEngine {
            functions: HashMap::new(),
            constraints: Vec::new(),
            substitutions: HashMap::new(),
            variance_map: HashMap::new(),
        }
    }

    pub fn register_function(&mut self, name: String, lifetimes: FunctionLifetimes) {
        self.functions.insert(name, lifetimes);
    }

    pub fn add_constraint(&mut self, constraint: LifetimeConstraint) {
        self.constraints.push(constraint);
    }

    pub fn apply_elision_rules(&mut self, func_name: &str) -> Result<(), String> {
        let lifetimes = self.functions.get(func_name)
            .ok_or(format!("Function {} not found", func_name))?
            .clone();

        if lifetimes.params.is_empty() {
            return Ok(());
        }

        if lifetimes.params.len() == 1 {
            let param_name = &lifetimes.params[0].name;
            for output in &lifetimes.output_positions {
                self.substitutions.insert(output.clone(), param_name.clone());
            }
        }

        if lifetimes.input_positions.len() == 1 && lifetimes.output_positions.len() == 1 {
            let input = &lifetimes.input_positions[0];
            let output = &lifetimes.output_positions[0];
            if let Some(lifetime) = self.substitutions.get(input) {
                self.substitutions.insert(output.clone(), lifetime.clone());
            }
        }

        Ok(())
    }

    pub fn infer_lifetime(&mut self, position: &str) -> Result<String, String> {
        if let Some(subst) = self.substitutions.get(position) {
            return Ok(subst.clone());
        }

        let outlives_constraints: Vec<_> = self.constraints.iter()
            .filter_map(|c| {
                if let LifetimeConstraint::OutlivesLifetime(a, b) = c {
                    Some((a.clone(), b.clone()))
                } else {
                    None
                }
            })
            .collect();

        if !outlives_constraints.is_empty() {
            let (a, _b) = &outlives_constraints[0];
            return Ok(a.clone());
        }

        Err(format!("Cannot infer lifetime for {}", position))
    }

    pub fn solve_constraints(&mut self) -> Result<(), String> {
        let mut iterations = 0;
        let max_iterations = 100;

        while !self.constraints.is_empty() && iterations < max_iterations {
            iterations += 1;
            let mut resolved = false;

            let constraints_copy = self.constraints.clone();
            for constraint in constraints_copy {
                match constraint {
                    LifetimeConstraint::Equal(a, b) => {
                        if let Some(subst_a) = self.substitutions.get(&a).cloned() {
                            self.substitutions.insert(b.clone(), subst_a);
                            resolved = true;
                        } else if let Some(subst_b) = self.substitutions.get(&b).cloned() {
                            self.substitutions.insert(a.clone(), subst_b);
                            resolved = true;
                        }
                    }
                    LifetimeConstraint::OutlivesLifetime(a, b) => {
                        if !self.substitutions.contains_key(&a) {
                            self.substitutions.insert(a.clone(), b.clone());
                            resolved = true;
                        }
                    }
                    LifetimeConstraint::OutlivesType(a, _ty) => {
                        if !self.substitutions.contains_key(&a) {
                            self.substitutions.insert(a.clone(), "static".to_string());
                            resolved = true;
                        }
                    }
                }
            }

            if !resolved {
                break;
            }

            self.constraints.clear();
        }

        if iterations >= max_iterations {
            return Err("Lifetime constraint solving exceeded max iterations".to_string());
        }

        Ok(())
    }

    pub fn check_lifetime_bounds(&self, lifetime: &str, bounds: &[String]) -> Result<(), String> {
        for bound in bounds {
            if bound == "static" && lifetime != "static" {
                return Err(format!("Lifetime {} does not satisfy static bound", lifetime));
            }
        }

        Ok(())
    }

    pub fn resolve_variance(&mut self, param: &str) -> Variance {
        if let Some(variance) = self.variance_map.get(param) {
            return variance.clone();
        }

        let outlives_count = self.constraints.iter()
            .filter(|c| {
                if let LifetimeConstraint::OutlivesLifetime(a, _) = c {
                    a == param
                } else {
                    false
                }
            })
            .count();

        let outlivedby_count = self.constraints.iter()
            .filter(|c| {
                if let LifetimeConstraint::OutlivesLifetime(_, b) = c {
                    b == param
                } else {
                    false
                }
            })
            .count();

        let variance = if outlives_count > 0 && outlivedby_count == 0 {
            Variance::Covariant
        } else if outlives_count == 0 && outlivedby_count > 0 {
            Variance::Contravariant
        } else {
            Variance::Invariant
        };

        self.variance_map.insert(param.to_string(), variance.clone());
        variance
    }

    pub fn collect_used_lifetimes(&self, func_name: &str) -> Result<Vec<String>, String> {
        let lifetimes = self.functions.get(func_name)
            .ok_or(format!("Function {} not found", func_name))?;

        let mut used = HashSet::new();
        for param in &lifetimes.params {
            used.insert(param.name.clone());
        }

        Ok(used.into_iter().collect())
    }

    pub fn validate_lifetime_params(&self, func_name: &str) -> Result<(), String> {
        let lifetimes = self.functions.get(func_name)
            .ok_or(format!("Function {} not found", func_name))?;

        for param in &lifetimes.params {
            for bound in &param.bounds {
                if bound != "static" && !lifetimes.params.iter().any(|p| &p.name == bound) {
                    return Err(format!("Lifetime bound {} not found", bound));
                }
            }
        }

        Ok(())
    }

    pub fn get_lifetime_substitution(&self, position: &str) -> Option<String> {
        self.substitutions.get(position).cloned()
    }

    pub fn clear_constraints(&mut self) {
        self.constraints.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_function() {
        let mut engine = LifetimeInferenceEngine::new();
        let lifetimes = FunctionLifetimes {
            params: vec![LifetimeParam {
                name: "'a".to_string(),
                bounds: vec![],
            }],
            return_lifetime: Some("'a".to_string()),
            input_positions: vec!["input".to_string()],
            output_positions: vec!["output".to_string()],
        };

        engine.register_function("foo".to_string(), lifetimes);
        assert_eq!(engine.functions.len(), 1);
    }

    #[test]
    fn test_apply_elision_single_param() {
        let mut engine = LifetimeInferenceEngine::new();
        let lifetimes = FunctionLifetimes {
            params: vec![LifetimeParam {
                name: "'a".to_string(),
                bounds: vec![],
            }],
            return_lifetime: Some("'a".to_string()),
            input_positions: vec!["input".to_string()],
            output_positions: vec!["output".to_string()],
        };

        engine.register_function("foo".to_string(), lifetimes);
        assert!(engine.apply_elision_rules("foo").is_ok());
    }

    #[test]
    fn test_add_and_solve_constraints() {
        let mut engine = LifetimeInferenceEngine::new();

        engine.add_constraint(LifetimeConstraint::OutlivesLifetime(
            "'static".to_string(),
            "'a".to_string(),
        ));

        assert!(engine.solve_constraints().is_ok());
        assert!(engine.substitutions.len() > 0);
    }

    #[test]
    fn test_outlives_constraint() {
        let mut engine = LifetimeInferenceEngine::new();

        engine.add_constraint(LifetimeConstraint::OutlivesLifetime(
            "'static".to_string(),
            "'a".to_string(),
        ));

        assert!(engine.solve_constraints().is_ok());
    }

    #[test]
    fn test_check_lifetime_bounds_static() {
        let engine = LifetimeInferenceEngine::new();
        let bounds = vec!["static".to_string()];

        assert!(engine.check_lifetime_bounds("static", &bounds).is_ok());
        assert!(engine.check_lifetime_bounds("'a", &bounds).is_err());
    }

    #[test]
    fn test_infer_lifetime_from_substitution() {
        let mut engine = LifetimeInferenceEngine::new();
        engine.substitutions.insert("pos1".to_string(), "'a".to_string());

        let result = engine.infer_lifetime("pos1");
        assert_eq!(result.unwrap(), "'a");
    }

    #[test]
    fn test_resolve_variance_covariant() {
        let mut engine = LifetimeInferenceEngine::new();

        engine.add_constraint(LifetimeConstraint::OutlivesLifetime(
            "'a".to_string(),
            "'b".to_string(),
        ));

        let variance = engine.resolve_variance("'a");
        assert_eq!(variance, Variance::Covariant);
    }

    #[test]
    fn test_resolve_variance_contravariant() {
        let mut engine = LifetimeInferenceEngine::new();

        engine.add_constraint(LifetimeConstraint::OutlivesLifetime(
            "'b".to_string(),
            "'a".to_string(),
        ));

        let variance = engine.resolve_variance("'a");
        assert_eq!(variance, Variance::Contravariant);
    }

    #[test]
    fn test_collect_used_lifetimes() {
        let mut engine = LifetimeInferenceEngine::new();
        let lifetimes = FunctionLifetimes {
            params: vec![
                LifetimeParam {
                    name: "'a".to_string(),
                    bounds: vec![],
                },
                LifetimeParam {
                    name: "'b".to_string(),
                    bounds: vec![],
                },
            ],
            return_lifetime: Some("'a".to_string()),
            input_positions: vec![],
            output_positions: vec![],
        };

        engine.register_function("bar".to_string(), lifetimes);
        let used = engine.collect_used_lifetimes("bar").unwrap();
        assert_eq!(used.len(), 2);
    }

    #[test]
    fn test_validate_lifetime_params() {
        let mut engine = LifetimeInferenceEngine::new();
        let lifetimes = FunctionLifetimes {
            params: vec![LifetimeParam {
                name: "'a".to_string(),
                bounds: vec!["static".to_string()],
            }],
            return_lifetime: None,
            input_positions: vec![],
            output_positions: vec![],
        };

        engine.register_function("baz".to_string(), lifetimes);
        assert!(engine.validate_lifetime_params("baz").is_ok());
    }

    #[test]
    fn test_get_lifetime_substitution() {
        let mut engine = LifetimeInferenceEngine::new();
        engine.substitutions.insert("input".to_string(), "'a".to_string());

        let subst = engine.get_lifetime_substitution("input");
        assert_eq!(subst, Some("'a".to_string()));
    }

    #[test]
    fn test_outlives_type_constraint() {
        let mut engine = LifetimeInferenceEngine::new();

        engine.add_constraint(LifetimeConstraint::OutlivesType(
            "'a".to_string(),
            "String".to_string(),
        ));

        assert!(engine.solve_constraints().is_ok());
        assert_eq!(
            engine.get_lifetime_substitution("'a"),
            Some("static".to_string())
        );
    }

    #[test]
    fn test_clear_constraints() {
        let mut engine = LifetimeInferenceEngine::new();

        engine.add_constraint(LifetimeConstraint::Equal(
            "'a".to_string(),
            "'b".to_string(),
        ));

        assert_eq!(engine.constraints.len(), 1);
        engine.clear_constraints();
        assert_eq!(engine.constraints.len(), 0);
    }

    #[test]
    fn test_multiple_constraints_resolution() {
        let mut engine = LifetimeInferenceEngine::new();

        engine.add_constraint(LifetimeConstraint::OutlivesLifetime(
            "'static".to_string(),
            "'a".to_string(),
        ));

        engine.add_constraint(LifetimeConstraint::Equal(
            "'a".to_string(),
            "'b".to_string(),
        ));

        engine.add_constraint(LifetimeConstraint::Equal(
            "'b".to_string(),
            "'c".to_string(),
        ));

        assert!(engine.solve_constraints().is_ok());
        assert!(engine.substitutions.len() > 0);
    }
}
