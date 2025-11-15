//! # Type Specialization System
//!
//! Specialized implementations for concrete types in generic contexts:
//! - Trait implementation specialization
//! - Generic vs specialized selection
//! - Specialization coherence checking
//! - Specialization overlap detection
//! - Monomorphization target selection

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpecializationRule {
    pub pattern: String,
    pub specialization_index: usize,
}

#[derive(Debug, Clone)]
pub struct GenericImpl {
    pub name: String,
    pub type_params: Vec<String>,
    pub impl_for: String,
}

#[derive(Debug, Clone)]
pub struct SpecializedImpl {
    pub name: String,
    pub concrete_types: Vec<String>,
    pub impl_for: String,
    pub overrides: GenericImpl,
}

pub struct TypeSpecializationEngine {
    generic_impls: HashMap<String, GenericImpl>,
    specialized_impls: HashMap<String, Vec<SpecializedImpl>>,
    specialization_rules: HashMap<String, Vec<SpecializationRule>>,
    specialization_cache: HashMap<String, SpecializedImpl>,
}

impl TypeSpecializationEngine {
    pub fn new() -> Self {
        TypeSpecializationEngine {
            generic_impls: HashMap::new(),
            specialized_impls: HashMap::new(),
            specialization_rules: HashMap::new(),
            specialization_cache: HashMap::new(),
        }
    }

    pub fn register_generic_impl(&mut self, name: String, generic_impl: GenericImpl) {
        self.generic_impls.insert(name, generic_impl);
    }

    pub fn register_specialization(
        &mut self,
        generic_name: String,
        specialized: SpecializedImpl,
    ) -> Result<(), String> {
        let generic = self.generic_impls.get(&generic_name)
            .ok_or(format!("Generic impl {} not found", generic_name))?;

        if specialized.overrides.type_params.len() > generic.type_params.len() {
            return Err("Specialization has more type params than generic".to_string());
        }

        self.specialized_impls
            .entry(generic_name)
            .or_insert_with(Vec::new)
            .push(specialized);

        Ok(())
    }

    pub fn add_specialization_rule(&mut self, generic_name: String, rule: SpecializationRule) {
        self.specialization_rules
            .entry(generic_name)
            .or_insert_with(Vec::new)
            .push(rule);
    }

    pub fn select_implementation(
        &mut self,
        generic_name: &str,
        concrete_types: &[String],
    ) -> Result<String, String> {
        let cache_key = format!("{}:{}", generic_name, concrete_types.join(","));
        if let Some(cached) = self.specialization_cache.get(&cache_key) {
            return Ok(cached.name.clone());
        }

        let specialized = self.specialized_impls.get(generic_name)
            .and_then(|specs| {
                specs.iter().find(|s| self.matches_pattern(&s.concrete_types, concrete_types))
            });

        let result = if let Some(spec) = specialized {
            spec.name.clone()
        } else {
            generic_name.to_string()
        };

        if let Some(spec) = specialized {
            self.specialization_cache.insert(cache_key, spec.clone());
        }

        Ok(result)
    }

    fn matches_pattern(&self, pattern: &[String], concrete: &[String]) -> bool {
        if pattern.len() != concrete.len() {
            return false;
        }

        pattern.iter().zip(concrete.iter()).all(|(p, c)| {
            p == "_" || p == c || self.is_pattern_match(p, c)
        })
    }

    fn is_pattern_match(&self, pattern: &str, concrete: &str) -> bool {
        if pattern.starts_with('&') && concrete.starts_with('&') {
            return pattern[1..] == concrete[1..];
        }

        if pattern.starts_with("Vec") && concrete.starts_with("Vec") {
            return true;
        }

        pattern == concrete
    }

    pub fn check_specialization_coherence(&self, generic_name: &str) -> Result<(), String> {
        if let Some(specs) = self.specialized_impls.get(generic_name) {
            for (i, spec1) in specs.iter().enumerate() {
                for (j, spec2) in specs.iter().enumerate() {
                    if i < j && self.patterns_overlap(&spec1.concrete_types, &spec2.concrete_types) {
                        return Err(format!(
                            "Overlapping specializations for {} and {}",
                            spec1.name, spec2.name
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    fn patterns_overlap(&self, pattern1: &[String], pattern2: &[String]) -> bool {
        if pattern1.len() != pattern2.len() {
            return false;
        }

        pattern1.iter().zip(pattern2.iter()).all(|(p1, p2)| {
            p1 == "_" || p2 == "_" || p1 == p2
        })
    }

    pub fn get_specialization_specificity(
        &self,
        generic_name: &str,
        concrete_types: &[String],
    ) -> Result<usize, String> {
        let mut max_specificity = 0;

        if let Some(specs) = self.specialized_impls.get(generic_name) {
            for spec in specs {
                if self.matches_pattern(&spec.concrete_types, concrete_types) {
                    let specificity = spec.concrete_types.iter()
                        .filter(|t| *t != "_")
                        .count();
                    max_specificity = max_specificity.max(specificity);
                }
            }
        }

        Ok(max_specificity)
    }

    pub fn collect_applicable_specializations(
        &self,
        generic_name: &str,
        concrete_types: &[String],
    ) -> Result<Vec<String>, String> {
        let mut applicable = Vec::new();

        if let Some(specs) = self.specialized_impls.get(generic_name) {
            for spec in specs {
                if self.matches_pattern(&spec.concrete_types, concrete_types) {
                    applicable.push(spec.name.clone());
                }
            }
        }

        Ok(applicable)
    }

    pub fn get_generic_impl(&self, name: &str) -> Option<GenericImpl> {
        self.generic_impls.get(name).cloned()
    }

    pub fn get_specialized_impl(&self, name: &str) -> Option<SpecializedImpl> {
        self.specialized_impls.values()
            .flat_map(|specs| specs.iter())
            .find(|s| s.name == name)
            .cloned()
    }

    pub fn validate_specialization_coverage(
        &mut self,
        generic_name: &str,
        common_types: &[String],
    ) -> Result<(), String> {
        for concrete_types in common_types.chunks(1) {
            let _impl = self.select_implementation(generic_name, concrete_types)?;
        }

        Ok(())
    }

    pub fn get_specialization_rule_matches(
        &self,
        generic_name: &str,
        concrete_types: &[String],
    ) -> Result<Vec<usize>, String> {
        let mut matches = Vec::new();

        if let Some(rules) = self.specialization_rules.get(generic_name) {
            for (idx, rule) in rules.iter().enumerate() {
                if self.rule_matches(&rule.pattern, concrete_types) {
                    matches.push(idx);
                }
            }
        }

        Ok(matches)
    }

    fn rule_matches(&self, rule_pattern: &str, concrete_types: &[String]) -> bool {
        if concrete_types.is_empty() {
            return rule_pattern.is_empty();
        }

        concrete_types.iter().any(|t| rule_pattern.contains(t))
    }

    pub fn clear_cache(&mut self) {
        self.specialization_cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_generic_impl() {
        let mut engine = TypeSpecializationEngine::new();
        let generic = GenericImpl {
            name: "Clone<T>".to_string(),
            type_params: vec!["T".to_string()],
            impl_for: "Clone".to_string(),
        };

        engine.register_generic_impl("Clone<T>".to_string(), generic);
        assert_eq!(engine.generic_impls.len(), 1);
    }

    #[test]
    fn test_register_specialization() {
        let mut engine = TypeSpecializationEngine::new();
        let generic = GenericImpl {
            name: "Clone<T>".to_string(),
            type_params: vec!["T".to_string()],
            impl_for: "Clone".to_string(),
        };

        engine.register_generic_impl("Clone<T>".to_string(), generic.clone());

        let specialized = SpecializedImpl {
            name: "Clone<i32>".to_string(),
            concrete_types: vec!["i32".to_string()],
            impl_for: "Clone".to_string(),
            overrides: generic,
        };

        assert!(engine.register_specialization("Clone<T>".to_string(), specialized).is_ok());
    }

    #[test]
    fn test_select_generic_implementation() {
        let mut engine = TypeSpecializationEngine::new();
        let generic = GenericImpl {
            name: "Clone<T>".to_string(),
            type_params: vec!["T".to_string()],
            impl_for: "Clone".to_string(),
        };

        engine.register_generic_impl("Clone<T>".to_string(), generic);

        let result = engine.select_implementation("Clone<T>", &["String".to_string()]);
        assert_eq!(result.unwrap(), "Clone<T>");
    }

    #[test]
    fn test_select_specialized_implementation() {
        let mut engine = TypeSpecializationEngine::new();
        let generic = GenericImpl {
            name: "Clone<T>".to_string(),
            type_params: vec!["T".to_string()],
            impl_for: "Clone".to_string(),
        };

        engine.register_generic_impl("Clone<T>".to_string(), generic.clone());

        let specialized = SpecializedImpl {
            name: "Clone<i32>".to_string(),
            concrete_types: vec!["i32".to_string()],
            impl_for: "Clone".to_string(),
            overrides: generic,
        };

        engine.register_specialization("Clone<T>".to_string(), specialized).unwrap();

        let result = engine.select_implementation("Clone<T>", &["i32".to_string()]);
        assert_eq!(result.unwrap(), "Clone<i32>");
    }

    #[test]
    fn test_check_specialization_coherence() {
        let mut engine = TypeSpecializationEngine::new();
        let generic = GenericImpl {
            name: "Debug<T>".to_string(),
            type_params: vec!["T".to_string()],
            impl_for: "Debug".to_string(),
        };

        engine.register_generic_impl("Debug<T>".to_string(), generic);

        assert!(engine.check_specialization_coherence("Debug<T>").is_ok());
    }

    #[test]
    fn test_get_specialization_specificity() {
        let mut engine = TypeSpecializationEngine::new();
        let generic = GenericImpl {
            name: "Trait<T>".to_string(),
            type_params: vec!["T".to_string()],
            impl_for: "Trait".to_string(),
        };

        engine.register_generic_impl("Trait<T>".to_string(), generic.clone());

        let specialized = SpecializedImpl {
            name: "Trait<Vec<i32>>".to_string(),
            concrete_types: vec!["Vec<i32>".to_string()],
            impl_for: "Trait".to_string(),
            overrides: generic,
        };

        engine.register_specialization("Trait<T>".to_string(), specialized).unwrap();

        let specificity = engine.get_specialization_specificity("Trait<T>", &["Vec<i32>".to_string()]);
        assert_eq!(specificity.unwrap(), 1);
    }

    #[test]
    fn test_collect_applicable_specializations() {
        let mut engine = TypeSpecializationEngine::new();
        let generic = GenericImpl {
            name: "Impl<T>".to_string(),
            type_params: vec!["T".to_string()],
            impl_for: "Impl".to_string(),
        };

        engine.register_generic_impl("Impl<T>".to_string(), generic.clone());

        let specialized = SpecializedImpl {
            name: "Impl<String>".to_string(),
            concrete_types: vec!["String".to_string()],
            impl_for: "Impl".to_string(),
            overrides: generic,
        };

        engine.register_specialization("Impl<T>".to_string(), specialized).unwrap();

        let applicable = engine.collect_applicable_specializations("Impl<T>", &["String".to_string()]);
        assert!(!applicable.unwrap().is_empty());
    }

    #[test]
    fn test_get_generic_impl() {
        let mut engine = TypeSpecializationEngine::new();
        let generic = GenericImpl {
            name: "Clone<T>".to_string(),
            type_params: vec!["T".to_string()],
            impl_for: "Clone".to_string(),
        };

        engine.register_generic_impl("Clone<T>".to_string(), generic.clone());

        let retrieved = engine.get_generic_impl("Clone<T>");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Clone<T>");
    }

    #[test]
    fn test_add_specialization_rule() {
        let mut engine = TypeSpecializationEngine::new();
        let rule = SpecializationRule {
            pattern: "Vec<_>".to_string(),
            specialization_index: 0,
        };

        engine.add_specialization_rule("Iterable".to_string(), rule);
        assert!(!engine.specialization_rules.is_empty());
    }

    #[test]
    fn test_specialization_caching() {
        let mut engine = TypeSpecializationEngine::new();
        let generic = GenericImpl {
            name: "Clone<T>".to_string(),
            type_params: vec!["T".to_string()],
            impl_for: "Clone".to_string(),
        };

        engine.register_generic_impl("Clone<T>".to_string(), generic.clone());

        let specialized = SpecializedImpl {
            name: "Clone<String>".to_string(),
            concrete_types: vec!["String".to_string()],
            impl_for: "Clone".to_string(),
            overrides: generic,
        };
        engine.register_specialization("Clone<T>".to_string(), specialized).unwrap();

        let _result1 = engine.select_implementation("Clone<T>", &["String".to_string()]);
        assert!(!engine.specialization_cache.is_empty());
    }

    #[test]
    fn test_pattern_matching_wildcard() {
        let mut engine = TypeSpecializationEngine::new();
        let generic = GenericImpl {
            name: "Generic<T>".to_string(),
            type_params: vec!["T".to_string()],
            impl_for: "Generic".to_string(),
        };

        engine.register_generic_impl("Generic<T>".to_string(), generic.clone());

        let specialized = SpecializedImpl {
            name: "Generic<Vec>".to_string(),
            concrete_types: vec!["Vec".to_string()],
            impl_for: "Generic".to_string(),
            overrides: generic,
        };

        engine.register_specialization("Generic<T>".to_string(), specialized).unwrap();

        let result = engine.select_implementation("Generic<T>", &["Vec".to_string()]);
        assert_eq!(result.unwrap(), "Generic<Vec>");
    }

    #[test]
    fn test_clear_cache() {
        let mut engine = TypeSpecializationEngine::new();
        let generic = GenericImpl {
            name: "Clone<T>".to_string(),
            type_params: vec!["T".to_string()],
            impl_for: "Clone".to_string(),
        };

        engine.register_generic_impl("Clone<T>".to_string(), generic.clone());

        let specialized = SpecializedImpl {
            name: "Clone<i32>".to_string(),
            concrete_types: vec!["i32".to_string()],
            impl_for: "Clone".to_string(),
            overrides: generic,
        };
        engine.register_specialization("Clone<T>".to_string(), specialized).unwrap();

        let _result = engine.select_implementation("Clone<T>", &["i32".to_string()]);

        assert!(!engine.specialization_cache.is_empty());
        engine.clear_cache();
        assert!(engine.specialization_cache.is_empty());
    }

    #[test]
    fn test_get_specialization_rule_matches() {
        let mut engine = TypeSpecializationEngine::new();
        let rule = SpecializationRule {
            pattern: "Vec".to_string(),
            specialization_index: 0,
        };

        engine.add_specialization_rule("Generic".to_string(), rule);

        let matches = engine.get_specialization_rule_matches("Generic", &["Vec".to_string()]);
        assert!(!matches.unwrap().is_empty());
    }
}
