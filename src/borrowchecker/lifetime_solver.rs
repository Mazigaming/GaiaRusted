use super::lifetimes::{LifetimeConstraint, LifetimeContext};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifetimeError {
    pub message: String,
}

impl LifetimeError {
    pub fn new(msg: impl Into<String>) -> Self {
        LifetimeError {
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for LifetimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Lifetime Error: {}", self.message)
    }
}

pub type LifetimeResult<T> = Result<T, LifetimeError>;

#[derive(Debug, Clone)]
struct LifetimeVar {
    name: String,
}

#[derive(Debug)]
pub struct LifetimeConstraintSolver {
    constraints: Vec<LifetimeConstraint>,
    outlives_graph: HashMap<String, Vec<String>>,
    reachability: HashMap<String, HashSet<String>>,
}

impl LifetimeConstraintSolver {
    pub fn new() -> Self {
        LifetimeConstraintSolver {
            constraints: Vec::new(),
            outlives_graph: HashMap::new(),
            reachability: HashMap::new(),
        }
    }

    pub fn from_context(ctx: &LifetimeContext) -> Self {
        let constraints = ctx.constraints().to_vec();
        let mut solver = LifetimeConstraintSolver::new();
        solver.constraints = constraints;
        solver
    }

    pub fn add_constraint(&mut self, constraint: LifetimeConstraint) {
        self.constraints.push(constraint);
    }

    fn build_outlives_graph(&mut self) -> LifetimeResult<()> {
        self.outlives_graph.clear();

        for constraint in &self.constraints {
            let lhs_key = constraint.lhs.to_string();
            let rhs_key = constraint.rhs.to_string();

            self.outlives_graph
                .entry(lhs_key)
                .or_insert_with(Vec::new)
                .push(rhs_key);
        }

        Ok(())
    }

    fn compute_transitive_closure(&mut self) -> LifetimeResult<()> {
        self.reachability.clear();

        let nodes: Vec<_> = self
            .outlives_graph
            .keys()
            .chain(self.outlives_graph.values().flat_map(|v| v.iter()))
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        for node in &nodes {
            self.reachability.insert(node.clone(), HashSet::new());
        }

        for (from, tos) in &self.outlives_graph {
            for to in tos {
                if let Some(set) = self.reachability.get_mut(from) {
                    set.insert(to.clone());
                }
            }
        }

        let mut changed = true;
        while changed {
            changed = false;

            for node in nodes.clone() {
                let current_reachable = self
                    .reachability
                    .get(&node)
                    .map(|s| s.clone())
                    .unwrap_or_default();

                for reachable in current_reachable {
                    let targets = self
                        .reachability
                        .get(&reachable)
                        .map(|s| s.clone())
                        .unwrap_or_default();

                    for target in targets {
                        if !self
                            .reachability
                            .get(&node)
                            .map(|s| s.contains(&target))
                            .unwrap_or(false)
                        {
                            self.reachability
                                .get_mut(&node)
                                .unwrap()
                                .insert(target);
                            changed = true;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn is_satisfiable(&mut self) -> LifetimeResult<()> {
        self.build_outlives_graph()?;
        self.compute_transitive_closure()?;

        for (node, reachable) in &self.reachability {
            if reachable.contains(node) {
                return Err(LifetimeError::new(format!(
                    "Lifetime '{}' cannot outlive itself",
                    node
                )));
            }
        }

        Ok(())
    }

    pub fn get_outlives(&mut self, lifetime: &str) -> LifetimeResult<HashSet<String>> {
        if self.reachability.is_empty() {
            self.build_outlives_graph()?;
            self.compute_transitive_closure()?;
        }
        Ok(self
            .reachability
            .get(lifetime)
            .cloned()
            .unwrap_or_default())
    }

    pub fn get_outlived_by(&mut self, lifetime: &str) -> LifetimeResult<HashSet<String>> {
        if self.reachability.is_empty() {
            self.build_outlives_graph()?;
            self.compute_transitive_closure()?;
        }
        let mut result = HashSet::new();
        for (other, reachable) in &self.reachability {
            if reachable.contains(&lifetime.to_string()) && other != lifetime {
                result.insert(other.clone());
            }
        }
        Ok(result)
    }

    pub fn constraints(&self) -> &[LifetimeConstraint] {
        &self.constraints
    }

    pub fn violations(&mut self) -> LifetimeResult<Vec<(String, String)>> {
        self.build_outlives_graph()?;
        self.compute_transitive_closure()?;

        let mut violations = Vec::new();

        for (node, reachable) in &self.reachability {
            if reachable.contains(node) {
                violations.push((node.clone(), node.clone()));
            }
        }

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::borrowchecker::lifetimes::Lifetime;

    #[test]
    fn test_simple_constraint() {
        let mut ctx = LifetimeContext::new();
        let a = ctx.register_named_lifetime("a".to_string());
        let b = ctx.register_named_lifetime("b".to_string());

        ctx.add_constraint(a.clone(), b.clone(), "test".to_string());

        let mut solver = LifetimeConstraintSolver::from_context(&ctx);
        assert!(solver.is_satisfiable().is_ok());
    }

    #[test]
    fn test_transitive_constraints() {
        let mut ctx = LifetimeContext::new();
        let a = ctx.register_named_lifetime("a".to_string());
        let b = ctx.register_named_lifetime("b".to_string());
        let c = ctx.register_named_lifetime("c".to_string());

        ctx.add_constraint(a.clone(), b.clone(), "a > b".to_string());
        ctx.add_constraint(b.clone(), c.clone(), "b > c".to_string());

        let mut solver = LifetimeConstraintSolver::from_context(&ctx);
        assert!(solver.is_satisfiable().is_ok());

        let a_outlives = solver.get_outlives("'a").unwrap();
        assert!(a_outlives.contains("'b"));
        assert!(a_outlives.contains("'c"));
    }

    #[test]
    fn test_cycle_detection() {
        let mut ctx = LifetimeContext::new();
        let a = ctx.register_named_lifetime("a".to_string());
        let b = ctx.register_named_lifetime("b".to_string());

        ctx.add_constraint(a.clone(), b.clone(), "a > b".to_string());
        ctx.add_constraint(b.clone(), a.clone(), "b > a".to_string());

        let mut solver = LifetimeConstraintSolver::from_context(&ctx);
        assert!(solver.is_satisfiable().is_err());
    }

    #[test]
    fn test_self_cycle() {
        let mut ctx = LifetimeContext::new();
        let a = ctx.register_named_lifetime("a".to_string());

        ctx.add_constraint(a.clone(), a.clone(), "a > a".to_string());

        let mut solver = LifetimeConstraintSolver::from_context(&ctx);
        assert!(solver.is_satisfiable().is_err());
    }

    #[test]
    fn test_static_lifetime() {
        let mut ctx = LifetimeContext::new();
        let static_lt = Lifetime::Static;
        let a = ctx.register_named_lifetime("a".to_string());

        ctx.add_constraint(static_lt.clone(), a.clone(), "static > a".to_string());

        let mut solver = LifetimeConstraintSolver::from_context(&ctx);
        assert!(solver.is_satisfiable().is_ok());
    }

    #[test]
    fn test_multiple_constraints_single_lifetime() {
        let mut ctx = LifetimeContext::new();
        let a = ctx.register_named_lifetime("a".to_string());
        let b = ctx.register_named_lifetime("b".to_string());
        let c = ctx.register_named_lifetime("c".to_string());

        ctx.add_constraint(a.clone(), b.clone(), "a > b".to_string());
        ctx.add_constraint(a.clone(), c.clone(), "a > c".to_string());

        let mut solver = LifetimeConstraintSolver::from_context(&ctx);
        assert!(solver.is_satisfiable().is_ok());

        let a_outlives = solver.get_outlives("'a").unwrap();
        assert!(a_outlives.contains("'b"));
        assert!(a_outlives.contains("'c"));
    }

    #[test]
    fn test_outlived_by() {
        let mut ctx = LifetimeContext::new();
        let a = ctx.register_named_lifetime("a".to_string());
        let b = ctx.register_named_lifetime("b".to_string());
        let c = ctx.register_named_lifetime("c".to_string());

        ctx.add_constraint(a.clone(), b.clone(), "a > b".to_string());
        ctx.add_constraint(a.clone(), c.clone(), "a > c".to_string());

        let mut solver = LifetimeConstraintSolver::from_context(&ctx);
        solver.is_satisfiable().ok();

        let b_outlived_by = solver.get_outlived_by("'b").unwrap();
        assert!(b_outlived_by.contains("'a"));
    }

    #[test]
    fn test_three_way_cycle() {
        let mut ctx = LifetimeContext::new();
        let a = ctx.register_named_lifetime("a".to_string());
        let b = ctx.register_named_lifetime("b".to_string());
        let c = ctx.register_named_lifetime("c".to_string());

        ctx.add_constraint(a.clone(), b.clone(), "a > b".to_string());
        ctx.add_constraint(b.clone(), c.clone(), "b > c".to_string());
        ctx.add_constraint(c.clone(), a.clone(), "c > a".to_string());

        let mut solver = LifetimeConstraintSolver::from_context(&ctx);
        assert!(solver.is_satisfiable().is_err());
    }

    #[test]
    fn test_inferred_lifetimes() {
        let mut ctx = LifetimeContext::new();
        let l1 = ctx.fresh_lifetime();
        let l2 = ctx.fresh_lifetime();

        ctx.add_constraint(l1.clone(), l2.clone(), "inferred".to_string());

        let mut solver = LifetimeConstraintSolver::from_context(&ctx);
        assert!(solver.is_satisfiable().is_ok());
    }

    #[test]
    fn test_mixed_named_and_inferred() {
        let mut ctx = LifetimeContext::new();
        let named_a = ctx.register_named_lifetime("'a".to_string());
        let inferred_b = ctx.fresh_lifetime();

        ctx.add_constraint(named_a.clone(), inferred_b.clone(), "mixed".to_string());

        let mut solver = LifetimeConstraintSolver::from_context(&ctx);
        assert!(solver.is_satisfiable().is_ok());
    }
}
