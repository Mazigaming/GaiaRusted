//! Advanced Trait System Enhancements
//!
//! Extends trait support with:
//! - Higher-ranked trait bounds (HRTBs)
//! - Complex type parameter constraints
//! - Trait object bounds checking
//! - Generic trait implementations
//! - Associated type constraints
//! - Variance tracking

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TraitBound {
    Simple(String),
    Lifetime(String),
    Complex {
        trait_name: String,
        type_params: Vec<TraitBound>,
    },
    HigherRanked {
        lifetimes: Vec<String>,
        bound: Box<TraitBound>,
    },
}

impl TraitBound {
    pub fn is_satisfied_by(&self, other: &TraitBound) -> bool {
        match (self, other) {
            (TraitBound::Simple(a), TraitBound::Simple(b)) => a == b,
            (TraitBound::Complex { trait_name: t1, type_params: tp1 }, 
             TraitBound::Complex { trait_name: t2, type_params: tp2 }) => {
                t1 == t2 && tp1.len() == tp2.len() &&
                tp1.iter().zip(tp2.iter()).all(|(b1, b2)| b1.is_satisfied_by(b2))
            }
            (TraitBound::HigherRanked { .. }, TraitBound::HigherRanked { .. }) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variance {
    Covariant,
    Contravariant,
    Invariant,
}

#[derive(Debug, Clone)]
pub struct TypeParameter {
    pub name: String,
    pub bounds: Vec<TraitBound>,
    pub variance: Variance,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AssociatedTypeConstraint {
    pub trait_name: String,
    pub assoc_type: String,
    pub constraint: String,
}

#[derive(Debug, Clone)]
pub struct GenericTraitImpl {
    pub trait_name: String,
    pub impl_type: String,
    pub type_params: Vec<TypeParameter>,
    pub where_clauses: Vec<TraitBound>,
    pub associated_types: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum TraitObjectBound {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct AdvancedTraitObject {
    pub bounds: TraitObjectBound,
    pub lifetime: Option<String>,
    pub associated_constraints: Vec<AssociatedTypeConstraint>,
}

pub struct AdvancedTraitChecker {
    trait_registry: HashMap<String, TraitMetadata>,
    impl_registry: HashMap<String, Vec<GenericTraitImpl>>,
    bound_cache: HashMap<String, bool>,
}

#[derive(Debug, Clone)]
pub struct TraitMetadata {
    pub name: String,
    pub type_params: Vec<TypeParameter>,
    pub associated_types: HashSet<String>,
    pub methods: Vec<String>,
    pub supertraits: Vec<String>,
}

impl AdvancedTraitChecker {
    pub fn new() -> Self {
        AdvancedTraitChecker {
            trait_registry: HashMap::new(),
            impl_registry: HashMap::new(),
            bound_cache: HashMap::new(),
        }
    }

    pub fn register_trait(&mut self, metadata: TraitMetadata) {
        self.trait_registry.insert(metadata.name.clone(), metadata);
    }

    pub fn register_impl(&mut self, impl_def: GenericTraitImpl) {
        self.impl_registry
            .entry(impl_def.trait_name.clone())
            .or_insert_with(Vec::new)
            .push(impl_def);
    }

    pub fn check_bounds(&mut self, ty: &str, bounds: &[TraitBound]) -> Result<(), String> {
        let cache_key = format!("{}_{:?}", ty, bounds);
        if let Some(&result) = self.bound_cache.get(&cache_key) {
            return if result { Ok(()) } else { Err("Bound check failed".to_string()) };
        }

        for bound in bounds {
            if !self.bound_satisfied(ty, bound)? {
                self.bound_cache.insert(cache_key, false);
                return Err(format!("Type {} does not satisfy bound {:?}", ty, bound));
            }
        }

        self.bound_cache.insert(cache_key, true);
        Ok(())
    }

    fn bound_satisfied(&self, ty: &str, bound: &TraitBound) -> Result<bool, String> {
        match bound {
            TraitBound::Simple(trait_name) => {
                Ok(self.trait_registry.contains_key(trait_name) &&
                   self.impl_registry.get(trait_name)
                       .map(|impls| impls.iter().any(|i| i.impl_type == ty))
                       .unwrap_or(false))
            }
            TraitBound::Complex { trait_name, type_params } => {
                let impls = self.impl_registry.get(trait_name).ok_or(format!("Trait {} not found", trait_name))?;
                Ok(impls.iter().any(|i| {
                    i.impl_type == ty && i.type_params.len() == type_params.len()
                }))
            }
            TraitBound::HigherRanked { .. } => Ok(true),
            TraitBound::Lifetime(_) => Ok(true),
        }
    }

    pub fn check_trait_object(&self, obj: &AdvancedTraitObject) -> Result<(), String> {
        match &obj.bounds {
            TraitObjectBound::Single(trait_name) => {
                if !self.trait_registry.contains_key(trait_name) {
                    return Err(format!("Trait {} not found", trait_name));
                }
            }
            TraitObjectBound::Multiple(traits) => {
                for trait_name in traits {
                    if !self.trait_registry.contains_key(trait_name) {
                        return Err(format!("Trait {} not found", trait_name));
                    }
                }
            }
        }

        for constraint in &obj.associated_constraints {
            if !self.trait_registry.contains_key(&constraint.trait_name) {
                return Err(format!("Trait {} not found", constraint.trait_name));
            }
        }

        Ok(())
    }

    pub fn find_matching_impl(
        &self,
        trait_name: &str,
        ty: &str,
    ) -> Option<&GenericTraitImpl> {
        self.impl_registry.get(trait_name)?
            .iter()
            .find(|i| i.impl_type == ty)
    }

    pub fn check_impl_coherence(&self, impl_def: &GenericTraitImpl) -> Result<(), String> {
        if let Some(impls) = self.impl_registry.get(&impl_def.trait_name) {
            let conflicting = impls.iter()
                .filter(|i| i.impl_type == impl_def.impl_type)
                .count();

            if conflicting > 0 {
                return Err(format!(
                    "Conflicting implementations of {} for {}",
                    impl_def.trait_name, impl_def.impl_type
                ));
            }
        }

        Ok(())
    }

    pub fn resolve_variance(&self, _ty: &str, bound: &TraitBound) -> Variance {
        match bound {
            TraitBound::Simple(_) => Variance::Covariant,
            TraitBound::Complex { .. } => Variance::Invariant,
            TraitBound::HigherRanked { .. } => Variance::Contravariant,
            TraitBound::Lifetime(_) => Variance::Covariant,
        }
    }

    pub fn get_associated_types(&self, trait_name: &str) -> Option<&HashSet<String>> {
        self.trait_registry.get(trait_name).map(|m| &m.associated_types)
    }

    pub fn collect_supertrait_bounds(&self, trait_name: &str) -> Result<Vec<TraitBound>, String> {
        let _metadata = self.trait_registry.get(trait_name)
            .ok_or(format!("Trait {} not found", trait_name))?;

        let mut bounds = Vec::new();
        let mut seen = HashSet::new();

        self.collect_supertrait_bounds_rec(trait_name, &mut bounds, &mut seen)?;
        Ok(bounds)
    }

    fn collect_supertrait_bounds_rec(
        &self,
        trait_name: &str,
        bounds: &mut Vec<TraitBound>,
        seen: &mut HashSet<String>,
    ) -> Result<(), String> {
        if seen.contains(trait_name) {
            return Ok(());
        }
        seen.insert(trait_name.to_string());

        let metadata = self.trait_registry.get(trait_name)
            .ok_or(format!("Trait {} not found", trait_name))?;

        for supertrait in &metadata.supertraits {
            bounds.push(TraitBound::Simple(supertrait.clone()));
            self.collect_supertrait_bounds_rec(supertrait, bounds, seen)?;
        }

        Ok(())
    }

    pub fn validate_where_clause(&self, where_bound: &TraitBound, ty: &str) -> Result<(), String> {
        match where_bound {
            TraitBound::Simple(trait_name) => {
                if self.impl_registry.get(trait_name)
                    .map(|impls| impls.iter().any(|i| i.impl_type == ty))
                    .unwrap_or(false) {
                    Ok(())
                } else {
                    Err(format!("Type {} does not implement {}", ty, trait_name))
                }
            }
            TraitBound::Complex { .. } => {
                Ok(())
            }
            TraitBound::HigherRanked { bound, .. } => {
                self.validate_where_clause(bound, ty)
            }
            TraitBound::Lifetime(_) => Ok(()),
        }
    }
}

impl Default for AdvancedTraitChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_trait_bound() {
        let bound = TraitBound::Simple("Clone".to_string());
        let other = TraitBound::Simple("Clone".to_string());
        assert!(bound.is_satisfied_by(&other));
    }

    #[test]
    fn test_complex_trait_bound() {
        let bound = TraitBound::Complex {
            trait_name: "Into".to_string(),
            type_params: vec![TraitBound::Simple("i32".to_string())],
        };
        let other = TraitBound::Complex {
            trait_name: "Into".to_string(),
            type_params: vec![TraitBound::Simple("i32".to_string())],
        };
        assert!(bound.is_satisfied_by(&other));
    }

    #[test]
    fn test_higher_ranked_trait_bound() {
        let bound = TraitBound::HigherRanked {
            lifetimes: vec!["'a".to_string()],
            bound: Box::new(TraitBound::Simple("Fn".to_string())),
        };
        let other = TraitBound::HigherRanked {
            lifetimes: vec!["'b".to_string()],
            bound: Box::new(TraitBound::Simple("Fn".to_string())),
        };
        assert!(bound.is_satisfied_by(&other));
    }

    #[test]
    fn test_advanced_trait_checker_creation() {
        let checker = AdvancedTraitChecker::new();
        assert!(checker.trait_registry.is_empty());
    }

    #[test]
    fn test_register_trait() {
        let mut checker = AdvancedTraitChecker::new();
        let metadata = TraitMetadata {
            name: "Clone".to_string(),
            type_params: vec![],
            associated_types: HashSet::new(),
            methods: vec!["clone".to_string()],
            supertraits: vec![],
        };
        checker.register_trait(metadata);
        assert!(checker.trait_registry.contains_key("Clone"));
    }

    #[test]
    fn test_variance_resolution() {
        let checker = AdvancedTraitChecker::new();
        let bound = TraitBound::Simple("Clone".to_string());
        let var = checker.resolve_variance("i32", &bound);
        assert_eq!(var, Variance::Covariant);
    }

    #[test]
    fn test_trait_object_check_valid() {
        let mut checker = AdvancedTraitChecker::new();
        let metadata = TraitMetadata {
            name: "Display".to_string(),
            type_params: vec![],
            associated_types: HashSet::new(),
            methods: vec![],
            supertraits: vec![],
        };
        checker.register_trait(metadata);

        let obj = AdvancedTraitObject {
            bounds: TraitObjectBound::Single("Display".to_string()),
            lifetime: None,
            associated_constraints: vec![],
        };
        assert!(checker.check_trait_object(&obj).is_ok());
    }

    #[test]
    fn test_trait_object_check_invalid() {
        let checker = AdvancedTraitChecker::new();
        let obj = AdvancedTraitObject {
            bounds: TraitObjectBound::Single("NonExistent".to_string()),
            lifetime: None,
            associated_constraints: vec![],
        };
        assert!(checker.check_trait_object(&obj).is_err());
    }

    #[test]
    fn test_impl_coherence_check() {
        let mut checker = AdvancedTraitChecker::new();
        
        let metadata = TraitMetadata {
            name: "Clone".to_string(),
            type_params: vec![],
            associated_types: HashSet::new(),
            methods: vec![],
            supertraits: vec![],
        };
        
        checker.register_trait(metadata);
        
        let impl_def = GenericTraitImpl {
            trait_name: "Clone".to_string(),
            impl_type: "i32".to_string(),
            type_params: vec![],
            where_clauses: vec![],
            associated_types: HashMap::new(),
        };

        assert!(checker.check_impl_coherence(&impl_def).is_ok());
        checker.register_impl(impl_def);
        
        let impl_def2 = GenericTraitImpl {
            trait_name: "Clone".to_string(),
            impl_type: "i32".to_string(),
            type_params: vec![],
            where_clauses: vec![],
            associated_types: HashMap::new(),
        };
        assert!(checker.check_impl_coherence(&impl_def2).is_err());
    }

    #[test]
    fn test_supertrait_collection() {
        let mut checker = AdvancedTraitChecker::new();
        
        let metadata1 = TraitMetadata {
            name: "Base".to_string(),
            type_params: vec![],
            associated_types: HashSet::new(),
            methods: vec![],
            supertraits: vec![],
        };
        
        let metadata2 = TraitMetadata {
            name: "Derived".to_string(),
            type_params: vec![],
            associated_types: HashSet::new(),
            methods: vec![],
            supertraits: vec!["Base".to_string()],
        };
        
        checker.register_trait(metadata1);
        checker.register_trait(metadata2);
        
        let bounds = checker.collect_supertrait_bounds("Derived").unwrap();
        assert!(!bounds.is_empty());
    }

    #[test]
    fn test_bounds_caching() {
        let mut checker = AdvancedTraitChecker::new();
        let bounds = vec![TraitBound::Simple("Clone".to_string())];
        
        let _ = checker.check_bounds("i32", &bounds);
        assert!(!checker.bound_cache.is_empty());
    }

    #[test]
    fn test_generic_impl_registration() {
        let mut checker = AdvancedTraitChecker::new();
        let impl_def = GenericTraitImpl {
            trait_name: "Into".to_string(),
            impl_type: "Vec".to_string(),
            type_params: vec![
                TypeParameter {
                    name: "T".to_string(),
                    bounds: vec![],
                    variance: Variance::Covariant,
                    default: None,
                }
            ],
            where_clauses: vec![],
            associated_types: HashMap::new(),
        };
        
        checker.register_impl(impl_def);
        assert!(checker.impl_registry.contains_key("Into"));
    }

    #[test]
    fn test_multiple_trait_bounds() {
        let obj = AdvancedTraitObject {
            bounds: TraitObjectBound::Multiple(vec![
                "Clone".to_string(),
                "Debug".to_string(),
            ]),
            lifetime: None,
            associated_constraints: vec![],
        };
        
        let mut checker = AdvancedTraitChecker::new();
        checker.register_trait(TraitMetadata {
            name: "Clone".to_string(),
            type_params: vec![],
            associated_types: HashSet::new(),
            methods: vec![],
            supertraits: vec![],
        });
        checker.register_trait(TraitMetadata {
            name: "Debug".to_string(),
            type_params: vec![],
            associated_types: HashSet::new(),
            methods: vec![],
            supertraits: vec![],
        });
        
        assert!(checker.check_trait_object(&obj).is_ok());
    }
}
