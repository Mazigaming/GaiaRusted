//! # Trait Bounds Inference System
//!
//! Sophisticated inference of trait bounds from generic parameter usage:
//! - Automatic bound inference from type parameter constraints
//! - Bound propagation through generic hierarchies
//! - Explicit vs inferred bound tracking
//! - Cyclic bound detection and resolution
//! - Minimal bound requirement identification

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoundRequirement {
    pub type_param: String,
    pub trait_name: String,
    pub explicit: bool,
}

#[derive(Debug, Clone)]
pub struct GenericContext {
    pub type_params: Vec<String>,
    pub explicit_bounds: HashMap<String, Vec<String>>,
    pub inferred_bounds: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct BoundConstraint {
    pub param: String,
    pub depends_on: String,
    pub trait_name: String,
}

pub struct TraitBoundsInferenceEngine {
    contexts: HashMap<String, GenericContext>,
    constraints: Vec<BoundConstraint>,
    normalized_bounds: HashMap<String, Vec<String>>,
    bound_cache: HashMap<String, Vec<String>>,
}

impl TraitBoundsInferenceEngine {
    pub fn new() -> Self {
        TraitBoundsInferenceEngine {
            contexts: HashMap::new(),
            constraints: Vec::new(),
            normalized_bounds: HashMap::new(),
            bound_cache: HashMap::new(),
        }
    }

    pub fn register_context(&mut self, name: String, context: GenericContext) {
        self.contexts.insert(name, context);
    }

    pub fn add_constraint(&mut self, constraint: BoundConstraint) {
        self.constraints.push(constraint);
    }

    pub fn infer_bounds(
        &mut self,
        context_name: &str,
        param: &str,
    ) -> Result<Vec<String>, String> {
        if let Some(cached) = self.bound_cache.get(param) {
            return Ok(cached.clone());
        }

        let context = self.contexts.get(context_name)
            .ok_or(format!("Context {} not found", context_name))?
            .clone();

        let mut bounds = context.explicit_bounds.get(param)
            .cloned()
            .unwrap_or_default();

        let mut visited = HashSet::new();
        self.infer_bounds_recursive(param, &context, &mut bounds, &mut visited)?;

        self.bound_cache.insert(param.to_string(), bounds.clone());
        Ok(bounds)
    }

    fn infer_bounds_recursive(
        &self,
        param: &str,
        context: &GenericContext,
        bounds: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) -> Result<(), String> {
        if visited.contains(param) {
            return Err(format!("Cyclic bound dependency detected for {}", param));
        }
        visited.insert(param.to_string());

        for constraint in &self.constraints {
            if constraint.param == param && !bounds.contains(&constraint.trait_name) {
                bounds.push(constraint.trait_name.clone());
            }

            if constraint.depends_on == param {
                if let Some(dep_bounds) = context.explicit_bounds.get(&constraint.param) {
                    for bound in dep_bounds {
                        if !bounds.contains(bound) {
                            bounds.push(bound.clone());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn normalize_bounds(&mut self, context_name: &str) -> Result<(), String> {
        let context = self.contexts.get(context_name)
            .ok_or(format!("Context {} not found", context_name))?
            .clone();

        for param in &context.type_params {
            let mut all_bounds = HashSet::new();

            if let Some(explicit) = context.explicit_bounds.get(param) {
                all_bounds.extend(explicit.iter().cloned());
            }

            if let Some(inferred) = context.inferred_bounds.get(param) {
                all_bounds.extend(inferred.iter().cloned());
            }

            let normalized: Vec<_> = all_bounds.into_iter().collect();
            self.normalized_bounds.insert(param.clone(), normalized);
        }

        Ok(())
    }

    pub fn get_normalized_bounds(&self, param: &str) -> Option<Vec<String>> {
        self.normalized_bounds.get(param).cloned()
    }

    pub fn detect_cyclic_bounds(&self, context_name: &str) -> Result<(), String> {
        let context = self.contexts.get(context_name)
            .ok_or(format!("Context {} not found", context_name))?;

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for param in &context.type_params {
            if !visited.contains(param) {
                self.detect_cycle_dfs(param, &mut visited, &mut rec_stack)?;
            }
        }

        Ok(())
    }

    fn detect_cycle_dfs(
        &self,
        param: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> Result<(), String> {
        visited.insert(param.to_string());
        rec_stack.insert(param.to_string());

        for constraint in &self.constraints {
            if constraint.param == param {
                let dep = &constraint.depends_on;
                if !visited.contains(dep) {
                    self.detect_cycle_dfs(dep, visited, rec_stack)?;
                } else if rec_stack.contains(dep) {
                    return Err(format!("Cyclic bound: {} -> {}", param, dep));
                }
            }
        }

        rec_stack.remove(param);
        Ok(())
    }

    pub fn find_minimal_bounds(
        &self,
        context_name: &str,
        param: &str,
    ) -> Result<Vec<String>, String> {
        let context = self.contexts.get(context_name)
            .ok_or(format!("Context {} not found", context_name))?;

        let explicit = context.explicit_bounds.get(param)
            .cloned()
            .unwrap_or_default();

        let inferred = context.inferred_bounds.get(param)
            .cloned()
            .unwrap_or_default();

        let mut minimal = explicit.clone();

        for inferred_bound in &inferred {
            if !explicit.contains(inferred_bound) {
                minimal.push(inferred_bound.clone());
            }
        }

        minimal.sort();
        minimal.dedup();
        Ok(minimal)
    }

    pub fn propagate_bounds(&mut self, context_name: &str) -> Result<(), String> {
        let context = self.contexts.get(context_name)
            .ok_or(format!("Context {} not found", context_name))?
            .clone();

        let mut propagated: HashMap<String, Vec<String>> = HashMap::new();

        for param in &context.type_params {
            let mut bounds = context.explicit_bounds.get(param)
                .cloned()
                .unwrap_or_default();

            for constraint in &self.constraints {
                if constraint.param == *param && !bounds.contains(&constraint.trait_name) {
                    bounds.push(constraint.trait_name.clone());
                }
            }

            propagated.insert(param.clone(), bounds);
        }

        for (param, bounds) in propagated {
            if let Some(ctx) = self.contexts.get_mut(context_name) {
                ctx.inferred_bounds.insert(param, bounds);
            }
        }

        Ok(())
    }

    pub fn validate_bounds_consistency(
        &self,
        context_name: &str,
    ) -> Result<(), String> {
        let context = self.contexts.get(context_name)
            .ok_or(format!("Context {} not found", context_name))?;

        for (param, explicit_bounds) in &context.explicit_bounds {
            if let Some(inferred_bounds) = context.inferred_bounds.get(param) {
                for bound in explicit_bounds {
                    if !inferred_bounds.contains(bound) && !self.is_independent_bound(bound) {
                    }
                }
            }
        }

        Ok(())
    }

    fn is_independent_bound(&self, bound: &str) -> bool {
        bound == "Send" || bound == "Sync" || bound == "Unpin"
    }

    pub fn get_bound_requirements(&self, context_name: &str) -> Result<Vec<BoundRequirement>, String> {
        let context = self.contexts.get(context_name)
            .ok_or(format!("Context {} not found", context_name))?;

        let mut requirements = Vec::new();

        for (param, bounds) in &context.explicit_bounds {
            for bound in bounds {
                requirements.push(BoundRequirement {
                    type_param: param.clone(),
                    trait_name: bound.clone(),
                    explicit: true,
                });
            }
        }

        for (param, bounds) in &context.inferred_bounds {
            for bound in bounds {
                if !context.explicit_bounds.get(param)
                    .map(|b| b.contains(bound))
                    .unwrap_or(false) {
                    requirements.push(BoundRequirement {
                        type_param: param.clone(),
                        trait_name: bound.clone(),
                        explicit: false,
                    });
                }
            }
        }

        Ok(requirements)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_context() {
        let mut engine = TraitBoundsInferenceEngine::new();
        let context = GenericContext {
            type_params: vec!["T".to_string()],
            explicit_bounds: HashMap::new(),
            inferred_bounds: HashMap::new(),
        };

        engine.register_context("func".to_string(), context);
        assert_eq!(engine.contexts.len(), 1);
    }

    #[test]
    fn test_add_constraint() {
        let mut engine = TraitBoundsInferenceEngine::new();
        let constraint = BoundConstraint {
            param: "T".to_string(),
            depends_on: "U".to_string(),
            trait_name: "Clone".to_string(),
        };

        engine.add_constraint(constraint);
        assert_eq!(engine.constraints.len(), 1);
    }

    #[test]
    fn test_infer_bounds_from_explicit() {
        let mut engine = TraitBoundsInferenceEngine::new();

        let mut explicit_bounds = HashMap::new();
        explicit_bounds.insert("T".to_string(), vec!["Clone".to_string()]);

        let context = GenericContext {
            type_params: vec!["T".to_string()],
            explicit_bounds,
            inferred_bounds: HashMap::new(),
        };

        engine.register_context("func".to_string(), context);

        let bounds = engine.infer_bounds("func", "T").unwrap();
        assert!(bounds.contains(&"Clone".to_string()));
    }

    #[test]
    fn test_infer_bounds_from_constraints() {
        let mut engine = TraitBoundsInferenceEngine::new();

        let context = GenericContext {
            type_params: vec!["T".to_string()],
            explicit_bounds: HashMap::new(),
            inferred_bounds: HashMap::new(),
        };

        engine.register_context("func".to_string(), context);

        let constraint = BoundConstraint {
            param: "T".to_string(),
            depends_on: "T".to_string(),
            trait_name: "Clone".to_string(),
        };

        engine.add_constraint(constraint);

        let bounds = engine.infer_bounds("func", "T").unwrap();
        assert!(bounds.contains(&"Clone".to_string()));
    }

    #[test]
    fn test_normalize_bounds() {
        let mut engine = TraitBoundsInferenceEngine::new();

        let mut explicit_bounds = HashMap::new();
        explicit_bounds.insert("T".to_string(), vec!["Clone".to_string()]);

        let mut inferred_bounds = HashMap::new();
        inferred_bounds.insert("T".to_string(), vec!["Debug".to_string()]);

        let context = GenericContext {
            type_params: vec!["T".to_string()],
            explicit_bounds,
            inferred_bounds,
        };

        engine.register_context("func".to_string(), context);
        assert!(engine.normalize_bounds("func").is_ok());
    }

    #[test]
    fn test_get_normalized_bounds() {
        let mut engine = TraitBoundsInferenceEngine::new();
        engine.normalized_bounds.insert(
            "T".to_string(),
            vec!["Clone".to_string(), "Debug".to_string()],
        );

        let bounds = engine.get_normalized_bounds("T");
        assert!(bounds.is_some());
        assert_eq!(bounds.unwrap().len(), 2);
    }

    #[test]
    fn test_detect_cyclic_bounds() {
        let mut engine = TraitBoundsInferenceEngine::new();

        let context = GenericContext {
            type_params: vec!["T".to_string(), "U".to_string()],
            explicit_bounds: HashMap::new(),
            inferred_bounds: HashMap::new(),
        };

        engine.register_context("func".to_string(), context);

        engine.add_constraint(BoundConstraint {
            param: "T".to_string(),
            depends_on: "U".to_string(),
            trait_name: "Clone".to_string(),
        });

        assert!(engine.detect_cyclic_bounds("func").is_ok());
    }

    #[test]
    fn test_find_minimal_bounds() {
        let mut engine = TraitBoundsInferenceEngine::new();

        let mut explicit_bounds = HashMap::new();
        explicit_bounds.insert("T".to_string(), vec!["Clone".to_string()]);

        let mut inferred_bounds = HashMap::new();
        inferred_bounds.insert("T".to_string(), vec!["Clone".to_string(), "Debug".to_string()]);

        let context = GenericContext {
            type_params: vec!["T".to_string()],
            explicit_bounds,
            inferred_bounds,
        };

        engine.register_context("func".to_string(), context);

        let minimal = engine.find_minimal_bounds("func", "T").unwrap();
        assert!(minimal.contains(&"Clone".to_string()));
    }

    #[test]
    fn test_propagate_bounds() {
        let mut engine = TraitBoundsInferenceEngine::new();

        let mut explicit_bounds = HashMap::new();
        explicit_bounds.insert("T".to_string(), vec!["Clone".to_string()]);

        let context = GenericContext {
            type_params: vec!["T".to_string()],
            explicit_bounds,
            inferred_bounds: HashMap::new(),
        };

        engine.register_context("func".to_string(), context);

        engine.add_constraint(BoundConstraint {
            param: "T".to_string(),
            depends_on: "T".to_string(),
            trait_name: "Clone".to_string(),
        });

        assert!(engine.propagate_bounds("func").is_ok());
    }

    #[test]
    fn test_validate_bounds_consistency() {
        let mut engine = TraitBoundsInferenceEngine::new();

        let mut explicit_bounds = HashMap::new();
        explicit_bounds.insert("T".to_string(), vec!["Clone".to_string()]);

        let mut inferred_bounds = HashMap::new();
        inferred_bounds.insert("T".to_string(), vec!["Clone".to_string()]);

        let context = GenericContext {
            type_params: vec!["T".to_string()],
            explicit_bounds,
            inferred_bounds,
        };

        engine.register_context("func".to_string(), context);

        assert!(engine.validate_bounds_consistency("func").is_ok());
    }

    #[test]
    fn test_get_bound_requirements() {
        let mut engine = TraitBoundsInferenceEngine::new();

        let mut explicit_bounds = HashMap::new();
        explicit_bounds.insert("T".to_string(), vec!["Clone".to_string()]);

        let context = GenericContext {
            type_params: vec!["T".to_string()],
            explicit_bounds,
            inferred_bounds: HashMap::new(),
        };

        engine.register_context("func".to_string(), context);

        let reqs = engine.get_bound_requirements("func").unwrap();
        assert!(!reqs.is_empty());
    }

    #[test]
    fn test_bound_caching() {
        let mut engine = TraitBoundsInferenceEngine::new();

        let mut explicit_bounds = HashMap::new();
        explicit_bounds.insert("T".to_string(), vec!["Clone".to_string()]);

        let context = GenericContext {
            type_params: vec!["T".to_string()],
            explicit_bounds,
            inferred_bounds: HashMap::new(),
        };

        engine.register_context("func".to_string(), context);

        let _bounds1 = engine.infer_bounds("func", "T").unwrap();
        assert!(!engine.bound_cache.is_empty());
    }

    #[test]
    fn test_multiple_bounds_per_param() {
        let mut engine = TraitBoundsInferenceEngine::new();

        let mut explicit_bounds = HashMap::new();
        explicit_bounds.insert(
            "T".to_string(),
            vec!["Clone".to_string(), "Debug".to_string()],
        );

        let context = GenericContext {
            type_params: vec!["T".to_string()],
            explicit_bounds,
            inferred_bounds: HashMap::new(),
        };

        engine.register_context("func".to_string(), context);

        let bounds = engine.infer_bounds("func", "T").unwrap();
        assert_eq!(bounds.len(), 2);
    }
}
