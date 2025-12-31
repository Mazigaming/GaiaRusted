//! Comprehensive Trait System Implementation
//!
//! Complete trait support including:
//! - Sealed traits and trait refinement
//! - Advanced trait bounds (HRTBs, complex constraints)
//! - Trait definitions and implementations
//! - Associated type constraints
//! - Where clauses and generic bounds
//! - Variance tracking and coherence checking

use crate::typesystem::types::Type;
use std::collections::{HashMap, HashSet};

// ============================================================================
// TRAIT DEFINITIONS AND BASICS
// ============================================================================

/// Trait definition
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    pub methods: Vec<TraitMethod>,
    pub associated_types: Vec<String>,
    pub super_traits: Vec<String>,
}

/// Trait method signature
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub generic_params: Vec<String>,
    pub self_type: SelfType,
    pub params: Vec<(String, String)>,  // (name, type)
    pub return_type: String,
    pub has_default: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelfType {
    Owned,
    Ref,
    MutRef,
    None,
}

// ============================================================================
// TRAIT IMPLEMENTATIONS
// ============================================================================

/// Trait implementation for a concrete type
#[derive(Debug, Clone)]
pub struct TraitImpl {
    pub trait_name: String,
    pub type_name: String,
    pub generic_params: Vec<String>,
    pub methods: HashMap<String, MethodImpl>,
    pub where_clauses: Vec<String>,
}

/// Method implementation
#[derive(Debug, Clone)]
pub struct MethodImpl {
    pub name: String,
    pub body: String,
}

/// Trait bound
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

/// A normalized set of constraints from where clauses
/// Used for coherence checking to distinguish blanket impls from constrained specializations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintSet {
    /// Sorted bounds for canonical comparison
    pub bounds: Vec<TraitBound>,
}

impl ConstraintSet {
    /// Create a constraint set from where clauses
    /// Automatically sorts bounds for consistent comparison
    pub fn from_clauses(clauses: &[TraitBound]) -> Self {
        let mut bounds = clauses.to_vec();
        bounds.sort();  // Sort for consistent comparison
        ConstraintSet { bounds }
    }
    
    /// Check if this constraint set is empty (blanket impl)
    pub fn is_empty(&self) -> bool {
        self.bounds.is_empty()
    }
    
    /// Check if all of our bounds are present in another constraint set
    /// Example: [Clone] is subset of [Clone, Debug]
    pub fn is_subset_of(&self, other: &ConstraintSet) -> bool {
        self.bounds.iter().all(|b| other.bounds.contains(b))
    }
    
    /// Check if these constraints are compatible with another set
    /// Compatible means: identical, one is subset of other, or both are empty
    pub fn compatible_with(&self, other: &ConstraintSet) -> bool {
        // Identical constraints
        if self == other {
            return true;
        }
        
        // If both are empty (blanket), compatible
        if self.is_empty() && other.is_empty() {
            return true;
        }
        
        // If one is subset of other, they're compatible
        // Example: [Clone] and [Clone, Debug] are compatible
        self.is_subset_of(other) || other.is_subset_of(self)
    }
    
    /// Check if constraints can coexist
    /// Returns false if they apply to mutually exclusive sets of types
    /// Example: T: Clone and T: Default apply to different types (not all Clone are Default)
    pub fn can_coexist_with(&self, other: &ConstraintSet) -> bool {
        // If compatible (subset or equal), they might coexist
        if self.compatible_with(other) {
            return true;
        }
        
        // If incompatible and non-empty, they apply to different type sets
        // Example: T: Clone vs T: Default - these are different constraints
        if !self.is_empty() && !other.is_empty() {
            // Check if they have completely different constraints
            // (no common bounds)
            let self_traits: HashSet<_> = self.bounds.iter()
                .filter_map(|b| if let TraitBound::Simple(s) = b { Some(s.clone()) } else { None })
                .collect();
            let other_traits: HashSet<_> = other.bounds.iter()
                .filter_map(|b| if let TraitBound::Simple(s) = b { Some(s.clone()) } else { None })
                .collect();
            
            // If no common traits, they're incompatible
            self_traits.is_disjoint(&other_traits)
        } else {
            false
        }
    }
}

impl TraitBound {
    pub fn is_satisfied_by(&self, other: &TraitBound) -> bool {
        match (self, other) {
            (TraitBound::Simple(a), TraitBound::Simple(b)) => a == b,
            (
                TraitBound::Complex {
                    trait_name: t1,
                    type_params: tp1,
                },
                TraitBound::Complex {
                    trait_name: t2,
                    type_params: tp2,
                },
            ) => {
                t1 == t2
                    && tp1.len() == tp2.len()
                    && tp1.iter().zip(tp2.iter()).all(|(b1, b2)| b1.is_satisfied_by(b2))
            }
            (TraitBound::HigherRanked { .. }, TraitBound::HigherRanked { .. }) => true,
            _ => false,
        }
    }
}

// ============================================================================
// SEALED TRAITS
// ============================================================================

/// Sealed trait marker
#[derive(Debug, Clone)]
pub struct SealedTrait {
    pub name: String,
    pub allowed_types: HashSet<String>,
    pub sealing_module: String,
}

/// Sealing marker
#[derive(Debug, Clone)]
pub struct SealingMarker {
    pub trait_name: String,
    pub marker_name: String,
}

/// Trait refinement level
#[derive(Debug, Clone, PartialEq)]
pub enum RefinementLevel {
    Private,
    Internal,
    Public,
    Sealed(String),
}

/// Trait with refinement metadata
#[derive(Debug, Clone)]
pub struct RefinedTrait {
    pub name: String,
    pub level: RefinementLevel,
    pub methods: Vec<TraitMethodSignature>,
    pub super_traits: Vec<String>,
    pub bounds: Vec<String>,
}

/// Trait method for refined traits
#[derive(Debug, Clone)]
pub struct TraitMethodSignature {
    pub name: String,
    pub receiver: MethodReceiver,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
}

/// Method receiver type
#[derive(Debug, Clone, PartialEq)]
pub enum MethodReceiver {
    Owned,
    Immutable,
    Mutable,
}

impl RefinedTrait {
    /// Create new refined trait
    pub fn new(name: String, level: RefinementLevel) -> Self {
        RefinedTrait {
            name,
            level,
            methods: Vec::new(),
            super_traits: Vec::new(),
            bounds: Vec::new(),
        }
    }

    /// Add method to trait
    pub fn add_method(&mut self, method: TraitMethodSignature) {
        self.methods.push(method);
    }

    /// Add super trait bound
    pub fn add_super_trait(&mut self, trait_name: String) {
        self.super_traits.push(trait_name);
    }

    /// Add generic bound
    pub fn add_bound(&mut self, bound: String) {
        self.bounds.push(bound);
    }

    /// Check if trait is accessible
    pub fn is_accessible(&self, from_module: &str) -> bool {
        match &self.level {
            RefinementLevel::Public => true,
            RefinementLevel::Private => false,
            RefinementLevel::Internal => from_module == "internal",
            RefinementLevel::Sealed(seal_module) => from_module == seal_module,
        }
    }
}

// ============================================================================
// ADVANCED TRAIT BOUNDS
// ============================================================================

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

#[derive(Debug, Clone)]
pub struct TraitMetadata {
    pub name: String,
    pub type_params: Vec<TypeParameter>,
    pub associated_types: HashSet<String>,
    pub methods: Vec<String>,
    pub supertraits: Vec<String>,
}

// ============================================================================
// SEALED TRAIT MANAGER
// ============================================================================

pub struct SealedTraitManager {
    sealed_traits: HashMap<String, SealedTrait>,
    markers: Vec<SealingMarker>,
}

impl SealedTraitManager {
    pub fn new() -> Self {
        SealedTraitManager {
            sealed_traits: HashMap::new(),
            markers: Vec::new(),
        }
    }

    pub fn seal_trait(
        &mut self,
        trait_name: String,
        allowed_types: HashSet<String>,
        sealing_module: String,
    ) -> Result<(), String> {
        if self.sealed_traits.contains_key(&trait_name) {
            return Err(format!("Trait {} is already sealed", trait_name));
        }

        let sealed_trait = SealedTrait {
            name: trait_name.clone(),
            allowed_types,
            sealing_module,
        };

        self.sealed_traits.insert(trait_name.clone(), sealed_trait);

        let marker = SealingMarker {
            trait_name: trait_name.clone(),
            marker_name: format!("{}Sealed", trait_name),
        };
        self.markers.push(marker);

        Ok(())
    }

    pub fn can_implement(&self, trait_name: &str, implementing_type: &str) -> bool {
        if let Some(sealed_trait) = self.sealed_traits.get(trait_name) {
            sealed_trait.allowed_types.contains(implementing_type)
        } else {
            true
        }
    }

    pub fn is_sealed(&self, trait_name: &str) -> bool {
        self.sealed_traits.contains_key(trait_name)
    }

    pub fn get_allowed_types(&self, trait_name: &str) -> Option<Vec<String>> {
        self.sealed_traits
            .get(trait_name)
            .map(|t| t.allowed_types.iter().cloned().collect())
    }

    pub fn add_allowed_type(&mut self, trait_name: &str, type_name: String) -> Result<(), String> {
        if let Some(sealed_trait) = self.sealed_traits.get_mut(trait_name) {
            sealed_trait.allowed_types.insert(type_name);
            Ok(())
        } else {
            Err(format!("Trait {} is not sealed", trait_name))
        }
    }

    pub fn remove_allowed_type(
        &mut self,
        trait_name: &str,
        type_name: &str,
    ) -> Result<(), String> {
        if let Some(sealed_trait) = self.sealed_traits.get_mut(trait_name) {
            sealed_trait.allowed_types.remove(type_name);
            Ok(())
        } else {
            Err(format!("Trait {} is not sealed", trait_name))
        }
    }

    pub fn get_sealing_module(&self, trait_name: &str) -> Option<String> {
        self.sealed_traits
            .get(trait_name)
            .map(|t| t.sealing_module.clone())
    }

    pub fn validate_implementation(
        &self,
        trait_name: &str,
        type_name: &str,
    ) -> Result<(), String> {
        if !self.can_implement(trait_name, type_name) {
            return Err(format!(
                "Type {} cannot implement sealed trait {}",
                type_name, trait_name
            ));
        }
        Ok(())
    }

    pub fn get_all_sealed_traits(&self) -> Vec<String> {
        self.sealed_traits.keys().cloned().collect()
    }

    pub fn generate_sealing_code(trait_name: &str, allowed_types: &[&str]) -> String {
        let mut code = format!(
            "// Sealed trait: {} can only be implemented by the listed types\n",
            trait_name
        );
        code.push_str("// This is enforced through a private marker type pattern\n");
        code.push_str(&format!("pub trait {} {{\n", trait_name));
        code.push_str("    fn method(&self);\n");
        code.push_str("}\n\n");

        code.push_str(&format!("mod sealed_{} {{\n", trait_name.to_lowercase()));
        code.push_str(&format!("    pub trait Sealed {{}}\n"));

        for allowed_type in allowed_types {
            code.push_str(&format!("    impl Sealed for {} {{}}\n", allowed_type));
        }

        code.push_str("}\n\n");

        for allowed_type in allowed_types {
            code.push_str(&format!(
                "impl {} for {} {{\n",
                trait_name, allowed_type
            ));
            code.push_str("    fn method(&self) {\n");
            code.push_str("        // Implementation\n");
            code.push_str("    }\n");
            code.push_str("}\n\n");
        }

        code
    }
}

impl Default for SealedTraitManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TRAIT REGISTRY
// ============================================================================

pub struct TraitRegistry {
    traits: HashMap<String, RefinedTrait>,
    sealed_traits: HashMap<String, SealedTrait>,
    implementations: HashMap<(String, String), Vec<String>>,  // (type, trait) -> [methods]
}

impl TraitRegistry {
    /// Create new trait registry
    pub fn new() -> Self {
        TraitRegistry {
            traits: HashMap::new(),
            sealed_traits: HashMap::new(),
            implementations: HashMap::new(),
        }
    }

    /// Register trait
    pub fn register_trait(&mut self, trait_def: RefinedTrait) {
        self.traits.insert(trait_def.name.clone(), trait_def);
    }

    /// Register sealed trait
    pub fn register_sealed_trait(&mut self, sealed: SealedTrait) {
        self.sealed_traits.insert(sealed.name.clone(), sealed);
    }

    /// Implement trait for type
    pub fn implement_trait(&mut self, type_name: String, trait_name: String, methods: Vec<String>) {
        self.implementations
            .insert((type_name, trait_name), methods);
    }

    /// Check if type implements trait
    pub fn implements_trait(&self, type_name: &str, trait_name: &str) -> bool {
        self.implementations
            .contains_key(&(type_name.to_string(), trait_name.to_string()))
    }

    /// Get all methods for implementation
    pub fn get_implementation(&self, type_name: &str, trait_name: &str) -> Option<&Vec<String>> {
        self.implementations
            .get(&(type_name.to_string(), trait_name.to_string()))
    }

    /// Seal a trait (make it private to module)
    pub fn seal_trait(&mut self, trait_name: String, by_module: String) {
        if let Some(trait_def) = self.traits.get_mut(&trait_name) {
            trait_def.level = RefinementLevel::Sealed(by_module);
        }
    }
}

impl Default for TraitRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ADVANCED TRAIT CHECKER
// ============================================================================

pub struct AdvancedTraitChecker {
    trait_registry: HashMap<String, TraitMetadata>,
    impl_registry: HashMap<String, Vec<GenericTraitImpl>>,
    bound_cache: HashMap<String, bool>,
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
            return if result {
                Ok(())
            } else {
                Err("Bound check failed".to_string())
            };
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
                Ok(self.trait_registry.contains_key(trait_name)
                    && self
                        .impl_registry
                        .get(trait_name)
                        .map(|impls| impls.iter().any(|i| i.impl_type == ty))
                        .unwrap_or(false))
            }
            TraitBound::Complex {
                trait_name,
                type_params,
            } => {
                let impls = self
                    .impl_registry
                    .get(trait_name)
                    .ok_or_else(|| format!("Trait {} not found", trait_name))?;
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
        self.impl_registry
            .get(trait_name)?
            .iter()
            .find(|i| i.impl_type == ty)
    }

    /// Check for trait implementation coherence
    ///
    /// Coherence rules prevent conflicting implementations:
    /// 1. Exact duplicate implementations are disallowed
    /// 2. Overlapping generic implementations are disallowed
    ///
    /// Examples of conflicts:
    /// - `impl Trait for T` + `impl Trait for T` (exact duplicate)
    /// - `impl<T> Trait for Vec<T>` + `impl<T> Trait for Vec<T>` (exact duplicate)
    /// - `impl<T> Trait for T` + `impl Trait for String` (overlap - String matches T)
    ///
    /// Examples of allowed coexistence:
    /// - `impl<T> Trait for Vec<T>` and `impl Trait for Vec<String>` (specialization)
    /// - `impl<T> Trait for Vec<T>` and `impl<T> Trait for Box<T>` (different types)
    pub fn check_impl_coherence(&self, impl_def: &GenericTraitImpl) -> Result<(), String> {
        if let Some(impls) = self.impl_registry.get(&impl_def.trait_name) {
            for existing_impl in impls.iter() {
                // Check for exact duplicates or overlapping implementations
                if self.impls_might_overlap(existing_impl, impl_def) {
                    return Err(format!(
                        "Conflicting implementations of {} detected:\n  \
                        Existing: impl{} {} for {}\n  \
                        New: impl{} {} for {}",
                        impl_def.trait_name,
                        self.format_type_params(&existing_impl.type_params),
                        impl_def.trait_name,
                        existing_impl.impl_type,
                        self.format_type_params(&impl_def.type_params),
                        impl_def.trait_name,
                        impl_def.impl_type,
                    ));
                }
            }
        }

        Ok(())
    }
    
    /// Check if two trait implementations might overlap
    /// 
    /// Two implementations overlap if:
    /// 1. They have the exact same type pattern (exact duplicate)
    /// 2. One is more general and could match the same concrete type as the other
    pub fn impls_might_overlap(
        &self,
        impl1: &GenericTraitImpl,
        impl2: &GenericTraitImpl,
    ) -> bool {
        // Exact match: same type pattern means they definitely overlap
        if impl1.impl_type == impl2.impl_type {
            // Additional check: if both have the same where clauses, they truly conflict
            return self.where_clauses_similar(&impl1.where_clauses, &impl2.where_clauses);
        }
        
        // Check for potential overlap in generic patterns
        // Example: impl<T> Trait for T and impl Trait for String overlap
        // because String matches T
        self.patterns_overlap(&impl1.impl_type, &impl2.impl_type, 
                             &impl1.type_params, &impl2.type_params)
    }
    
    /// Check if two type patterns could match the same concrete type
    fn patterns_overlap(
        &self,
        pattern1: &str,
        pattern2: &str,
        params1: &[TypeParameter],
        params2: &[TypeParameter],
    ) -> bool {
        // If patterns are identical, they overlap
        if pattern1 == pattern2 {
            return true;
        }
        
        // Check for generic parameter overlap
        // Example: "T" and "String" overlap if first impl is generic
        let pattern1_is_generic = self.is_generic_pattern(pattern1, params1);
        let pattern2_is_generic = self.is_generic_pattern(pattern2, params2);
        
        match (pattern1_is_generic, pattern2_is_generic) {
            // Both generic: "T" and "T" overlap (but caught above by equality check)
            // "Vec<T>" and "Vec<U>" might overlap depending on type params
            (true, true) => {
                // Conservative: assume potential overlap if both have generics
                // A full implementation would do unification here
                self.could_unify(pattern1, pattern2)
            }
            // One generic, one concrete: "T" overlaps with "String" 
            (true, false) => true,
            (false, true) => true,
            // Both concrete: "Vec<i32>" and "Vec<String>" don't overlap
            (false, false) => false,
        }
    }
    
    /// Check if a type pattern is generic (contains type variables)
    fn is_generic_pattern(&self, pattern: &str, _params: &[TypeParameter]) -> bool {
        // Check if pattern contains generic indicators
        // Simple heuristic: single uppercase letter or contains angle brackets
        pattern.len() == 1 && pattern.chars().next().map_or(false, |c| c.is_uppercase())
        || pattern.contains('<') || pattern.contains("T") || pattern.contains("U")
    }
    
    /// Check if two patterns could potentially unify
    fn could_unify(&self, pattern1: &str, pattern2: &str) -> bool {
        // Very conservative: if both have generic parameters, assume they might unify
        // A full implementation would attempt actual unification
        
        // Extract base types (before angle brackets)
        let base1 = pattern1.split('<').next().unwrap_or(pattern1);
        let base2 = pattern2.split('<').next().unwrap_or(pattern2);
        
        // If base types are different, patterns don't unify
        if base1 != base2 {
            return false;
        }
        
        // If bases match and either has generics, assume potential overlap
        pattern1.contains('<') || pattern2.contains('<')
    }
    
    /// Check if two where clause sets allow the implementations to coexist
    /// 
    /// Key insight: impls with different where clauses might not actually conflict
    /// because they apply to different sets of types.
    /// 
    /// Examples:
    /// - `impl<T> Trait` (no where) and `impl<T: Clone> Trait` can coexist:
    ///   the second is a specialization of the first
    /// - `impl<T: Clone> Trait` and `impl<T: Default> Trait` can coexist:
    ///   they apply to different types (not all Clone are Default, and vice versa)
    /// - `impl<T: Clone> Trait` and `impl<T: Clone> Trait` CANNOT coexist:
    ///   they're identical
    pub fn where_clauses_similar(&self, clauses1: &[TraitBound], clauses2: &[TraitBound]) -> bool {
        let constraints1 = ConstraintSet::from_clauses(clauses1);
        let constraints2 = ConstraintSet::from_clauses(clauses2);
        
        // If constraints are identical, impls with same pattern can't coexist
        if constraints1 == constraints2 {
            return true;
        }
        
        // If constraints are different:
        // - Compatible (one subset of other): might overlap depending on pattern
        // - Incompatible (no common bounds): can't overlap (different type sets)
        constraints1.compatible_with(&constraints2)
    }
    
    /// Format type parameters for display in error messages
    fn format_type_params(&self, params: &[TypeParameter]) -> String {
        if params.is_empty() {
            String::new()
        } else {
            let param_names: Vec<String> = params.iter()
                .map(|p| p.name.clone())
                .collect();
            format!("<{}>", param_names.join(", "))
        }
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
        let _metadata = self
            .trait_registry
            .get(trait_name)
            .ok_or_else(|| format!("Trait {} not found", trait_name))?;

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

        let metadata = self
            .trait_registry
            .get(trait_name)
            .ok_or_else(|| format!("Trait {} not found", trait_name))?;

        for supertrait in &metadata.supertraits {
            bounds.push(TraitBound::Simple(supertrait.clone()));
            self.collect_supertrait_bounds_rec(supertrait, bounds, seen)?;
        }

        Ok(())
    }

    pub fn validate_where_clause(&self, where_bound: &TraitBound, ty: &str) -> Result<(), String> {
        match where_bound {
            TraitBound::Simple(trait_name) => {
                if self
                    .impl_registry
                    .get(trait_name)
                    .map(|impls| impls.iter().any(|i| i.impl_type == ty))
                    .unwrap_or(false)
                {
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

// ============================================================================
// TRAIT RESOLVER
// ============================================================================

pub struct TraitResolver {
    traits: HashMap<String, TraitDef>,
    implementations: Vec<TraitImpl>,
    bounds: Vec<TraitBound>,
}

impl TraitResolver {
    /// Create new trait resolver
    pub fn new() -> Self {
        TraitResolver {
            traits: HashMap::new(),
            implementations: Vec::new(),
            bounds: Vec::new(),
        }
    }

    /// Register trait definition
    pub fn define_trait(&mut self, trait_def: TraitDef) {
        self.traits.insert(trait_def.name.clone(), trait_def);
    }

    /// Register trait implementation
    pub fn implement_trait(&mut self, impl_def: TraitImpl) {
        self.implementations.push(impl_def);
    }

    /// Add trait bound
    pub fn add_bound(&mut self, bound: TraitBound) {
        self.bounds.push(bound);
    }

    /// Check if type implements trait
    pub fn implements(&self, type_name: &str, trait_name: &str) -> bool {
        self.implementations.iter().any(|impl_def| {
            impl_def.type_name == type_name && impl_def.trait_name == trait_name
        })
    }

    /// Get all trait implementations
    pub fn get_implementations(&self, trait_name: &str) -> Vec<&TraitImpl> {
        self.implementations
            .iter()
            .filter(|impl_def| impl_def.trait_name == trait_name)
            .collect()
    }

    /// Resolve trait method for type
    pub fn resolve_method(
        &self,
        type_name: &str,
        trait_name: &str,
        method_name: &str,
    ) -> Option<&MethodImpl> {
        self.implementations
            .iter()
            .find(|impl_def| {
                impl_def.type_name == type_name && impl_def.trait_name == trait_name
            })
            .and_then(|impl_def| impl_def.methods.get(method_name))
    }

    /// Generate trait implementation code
    pub fn generate_impl_code(&self, impl_def: &TraitImpl) -> String {
        let mut code = String::new();

        let generics = if impl_def.generic_params.is_empty() {
            String::new()
        } else {
            format!("<{}>", impl_def.generic_params.join(", "))
        };

        code.push_str(&format!(
            "impl{} {} for {} {{\n",
            if generics.is_empty() {
                " ".to_string()
            } else {
                generics
            },
            impl_def.trait_name,
            impl_def.type_name
        ));

        if !impl_def.where_clauses.is_empty() {
            code.push_str("where\n");
            for clause in &impl_def.where_clauses {
                code.push_str(&format!("    {},\n", clause));
            }
        }

        code.push_str("{\n");

        for (method_name, _method_impl) in &impl_def.methods {
            code.push_str(&format!("    fn {} {{  }}\n", method_name));
        }

        code.push_str("}\n");

        code
    }

    /// Check trait bounds are satisfied
    pub fn check_bounds(&self, type_name: &str, bounds: &[TraitBound]) -> Result<(), String> {
        for bound in bounds {
            if let TraitBound::Simple(trait_name) = bound {
                if !self.implements(type_name, trait_name) {
                    return Err(format!("{} does not implement {}", type_name, trait_name));
                }
            }
        }
        Ok(())
    }
    
    /// Check if two where clause sets are "similar enough" to conflict
     /// 
     /// Returns TRUE if they have overlapping type sets:
     /// - Both empty (both blanket) - they definitely overlap
     /// - Both have constraints AND they're compatible (identical or subset)
     /// 
     /// Returns FALSE if:
     /// - One empty, one constrained (specialization, not conflict)
     /// - Constraints are completely disjoint (e.g., [Clone] vs [Default])
     pub fn where_clauses_similar(&self, clauses1: &[TraitBound], clauses2: &[TraitBound]) -> bool {
         let constraints1 = ConstraintSet::from_clauses(clauses1);
         let constraints2 = ConstraintSet::from_clauses(clauses2);
         
         // Both empty: blanket impls definitely conflict
         if constraints1.is_empty() && constraints2.is_empty() {
             return true;
         }
         
         // One empty, one constrained: not a conflict (specialization)
         if constraints1.is_empty() || constraints2.is_empty() {
             return false;
         }
         
         // Both have constraints: check if they're compatible
         // Compatible = identical or one is subset of other
         constraints1.compatible_with(&constraints2)
     }
    
    /// Check if two trait implementations might overlap
    pub fn impls_might_overlap(
        &self,
        impl1: &GenericTraitImpl,
        impl2: &GenericTraitImpl,
    ) -> bool {
        // Exact match: same type pattern means they definitely overlap
        if impl1.impl_type == impl2.impl_type {
            return self.where_clauses_similar(&impl1.where_clauses, &impl2.where_clauses);
        }
        
        // For different patterns, conservative approach: assume no overlap
        // A full implementation would check generic pattern overlap
        false
    }
}

impl Default for TraitResolver {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CODE GENERATION
// ============================================================================

/// Generate sealed trait code
pub fn generate_sealed_trait(trait_def: &RefinedTrait) -> String {
    let mut code = String::new();

    match &trait_def.level {
        RefinementLevel::Sealed(module) => {
            code.push_str(&format!("// Sealed trait in module: {}\n", module));
            code.push_str(&format!("mod {} {{\n", module));
        }
        RefinementLevel::Public => {
            code.push_str("// Public trait\n");
        }
        RefinementLevel::Private => {
            code.push_str("// Private trait\n");
        }
        RefinementLevel::Internal => {
            code.push_str("// Internal trait\n");
        }
    }

    code.push_str(&format!("pub trait {} ", trait_def.name));

    if !trait_def.super_traits.is_empty() {
        code.push_str(&format!(": {} ", trait_def.super_traits.join(" + ")));
    }

    code.push_str("{\n");

    for method in &trait_def.methods {
        code.push_str(&format!("  fn {}(", method.name));

        match method.receiver {
            MethodReceiver::Owned => code.push_str("self"),
            MethodReceiver::Immutable => code.push_str("&self"),
            MethodReceiver::Mutable => code.push_str("&mut self"),
        }

        if !method.params.is_empty() {
            code.push_str(", ");
            let params: Vec<String> = method
                .params
                .iter()
                .map(|(name, ty)| format!("{}: {:?}", name, ty))
                .collect();
            code.push_str(&params.join(", "));
        }

        code.push_str(&format!(") -> {:?};\n", method.return_type));
    }

    code.push_str("}\n");

    if matches!(trait_def.level, RefinementLevel::Sealed(_)) {
        code.push_str("}\n");
    }

    code
}

// ============================================================================
// BUILT-IN TRAITS
// ============================================================================

pub fn create_clone_trait() -> TraitDef {
    TraitDef {
        name: "Clone".to_string(),
        methods: vec![TraitMethod {
            name: "clone".to_string(),
            generic_params: vec![],
            self_type: SelfType::Ref,
            params: vec![],
            return_type: "Self".to_string(),
            has_default: false,
        }],
        associated_types: vec![],
        super_traits: vec![],
    }
}

pub fn create_copy_trait() -> TraitDef {
    TraitDef {
        name: "Copy".to_string(),
        methods: vec![],
        associated_types: vec![],
        super_traits: vec!["Clone".to_string()],
    }
}

pub fn create_iterator_trait() -> TraitDef {
    TraitDef {
        name: "Iterator".to_string(),
        methods: vec![TraitMethod {
            name: "next".to_string(),
            generic_params: vec![],
            self_type: SelfType::MutRef,
            params: vec![],
            return_type: "Option<Self::Item>".to_string(),
            has_default: false,
        }],
        associated_types: vec!["Item".to_string()],
        super_traits: vec![],
    }
}

pub fn create_default_trait() -> TraitDef {
    TraitDef {
        name: "Default".to_string(),
        methods: vec![TraitMethod {
            name: "default".to_string(),
            generic_params: vec![],
            self_type: SelfType::None,
            params: vec![],
            return_type: "Self".to_string(),
            has_default: true,
        }],
        associated_types: vec![],
        super_traits: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // SEALED TRAITS TESTS
    // ========================================================================

    #[test]
    fn test_seal_trait() {
        let mut manager = SealedTraitManager::new();
        let mut allowed = HashSet::new();
        allowed.insert("i32".to_string());
        allowed.insert("String".to_string());

        let result = manager.seal_trait(
            "MyTrait".to_string(),
            allowed,
            "my_module".to_string(),
        );
        assert!(result.is_ok());
        assert!(manager.is_sealed("MyTrait"));
    }

    #[test]
    fn test_can_implement() {
        let mut manager = SealedTraitManager::new();
        let mut allowed = HashSet::new();
        allowed.insert("Point".to_string());

        manager
            .seal_trait("Display".to_string(), allowed, "std".to_string())
            .unwrap();

        assert!(manager.can_implement("Display", "Point"));
        assert!(!manager.can_implement("Display", "OtherType"));
    }

    #[test]
    fn test_unsealed_trait() {
        let manager = SealedTraitManager::new();
        assert!(!manager.is_sealed("UnknownTrait"));
        assert!(manager.can_implement("UnknownTrait", "AnyType"));
    }

    #[test]
    fn test_get_allowed_types() {
        let mut manager = SealedTraitManager::new();
        let mut allowed = HashSet::new();
        allowed.insert("A".to_string());
        allowed.insert("B".to_string());

        manager
            .seal_trait("Trait1".to_string(), allowed, "module".to_string())
            .unwrap();

        let types = manager.get_allowed_types("Trait1");
        assert!(types.is_some());
        let types = types.unwrap();
        assert_eq!(types.len(), 2);
        assert!(types.contains(&"A".to_string()));
        assert!(types.contains(&"B".to_string()));
    }

    #[test]
    fn test_add_allowed_type() {
        let mut manager = SealedTraitManager::new();
        let allowed = HashSet::new();
        manager
            .seal_trait("Trait2".to_string(), allowed, "module".to_string())
            .unwrap();

        let result = manager.add_allowed_type("Trait2", "NewType".to_string());
        assert!(result.is_ok());
        assert!(manager.can_implement("Trait2", "NewType"));
    }

    #[test]
    fn test_validate_implementation() {
        let mut manager = SealedTraitManager::new();
        let mut allowed = HashSet::new();
        allowed.insert("AllowedType".to_string());

        manager
            .seal_trait("Trait3".to_string(), allowed, "module".to_string())
            .unwrap();

        assert!(manager
            .validate_implementation("Trait3", "AllowedType")
            .is_ok());
        assert!(manager
            .validate_implementation("Trait3", "DisallowedType")
            .is_err());
    }

    #[test]
    fn test_generate_sealing_code() {
        let code =
            SealedTraitManager::generate_sealing_code("MyTrait", &["Type1", "Type2", "Type3"]);
        assert!(code.contains("MyTrait"));
        assert!(code.contains("Type1"));
        assert!(code.contains("Type2"));
        assert!(code.contains("Type3"));
        assert!(code.contains("Sealed"));
    }

    // ========================================================================
    // REFINED TRAITS TESTS
    // ========================================================================

    #[test]
    fn test_refined_trait_creation() {
        let trait_def = RefinedTrait::new("Iterator".to_string(), RefinementLevel::Public);
        assert_eq!(trait_def.name, "Iterator");
        assert_eq!(trait_def.level, RefinementLevel::Public);
    }

    #[test]
    fn test_sealed_trait_accessibility() {
        let trait_def = RefinedTrait::new(
            "PrivateTrait".to_string(),
            RefinementLevel::Sealed("mymod".to_string()),
        );
        assert!(trait_def.is_accessible("mymod"));
        assert!(!trait_def.is_accessible("other"));
    }

    // ========================================================================
    // TRAIT REGISTRY TESTS
    // ========================================================================

    #[test]
    fn test_trait_registry() {
        let mut registry = TraitRegistry::new();
        let trait_def = RefinedTrait::new("Clone".to_string(), RefinementLevel::Public);
        registry.register_trait(trait_def);

        assert!(registry.traits.contains_key("Clone"));
    }

    #[test]
    fn test_trait_implementation() {
        let mut registry = TraitRegistry::new();
        registry.implement_trait(
            "String".to_string(),
            "Clone".to_string(),
            vec!["clone".to_string()],
        );

        assert!(registry.implements_trait("String", "Clone"));
        assert!(!registry.implements_trait("String", "Iterator"));
    }

    // ========================================================================
    // ADVANCED TRAIT CHECKER TESTS
    // ========================================================================

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

    // ========================================================================
    // TRAIT RESOLVER TESTS
    // ========================================================================

    #[test]
    fn test_trait_definition() {
        let trait_def = create_clone_trait();
        assert_eq!(trait_def.name, "Clone");
        assert_eq!(trait_def.methods.len(), 1);
    }

    #[test]
    fn test_trait_resolver_creation() {
        let resolver = TraitResolver::new();
        assert_eq!(resolver.traits.len(), 0);
        assert_eq!(resolver.implementations.len(), 0);
    }

    #[test]
    fn test_register_trait_resolver() {
        let mut resolver = TraitResolver::new();
        resolver.define_trait(create_clone_trait());
        assert!(resolver.traits.contains_key("Clone"));
    }

    #[test]
    fn test_implement_trait_resolver() {
        let mut resolver = TraitResolver::new();
        resolver.define_trait(create_clone_trait());

        let impl_def = TraitImpl {
            trait_name: "Clone".to_string(),
            type_name: "String".to_string(),
            generic_params: vec![],
            methods: HashMap::new(),
            where_clauses: vec![],
        };
        resolver.implement_trait(impl_def);

        assert!(resolver.implements("String", "Clone"));
    }

    #[test]
    fn test_iterator_trait() {
        let iter_trait = create_iterator_trait();
        assert_eq!(iter_trait.name, "Iterator");
        assert!(iter_trait
            .associated_types
            .contains(&"Item".to_string()));
    }

    #[test]
    fn test_default_trait() {
        let default_trait = create_default_trait();
        assert_eq!(default_trait.name, "Default");
        assert!(default_trait.methods[0].has_default);
    }

    #[test]
    fn test_generate_impl_code() {
        let mut resolver = TraitResolver::new();
        let mut methods = HashMap::new();
        methods.insert(
            "clone".to_string(),
            MethodImpl {
                name: "clone".to_string(),
                body: "self.clone()".to_string(),
            },
        );

        let impl_def = TraitImpl {
            trait_name: "Clone".to_string(),
            type_name: "MyType".to_string(),
            generic_params: vec!["T".to_string()],
            methods,
            where_clauses: vec!["T: Clone".to_string()],
        };

        let code = resolver.generate_impl_code(&impl_def);
        assert!(code.contains("impl<T> Clone"));
        assert!(code.contains("MyType"));
    }

    #[test]
    fn test_trait_bounds_check() {
        let mut resolver = TraitResolver::new();
        resolver.define_trait(create_clone_trait());
        resolver.implement_trait(TraitImpl {
            trait_name: "Clone".to_string(),
            type_name: "String".to_string(),
            generic_params: vec![],
            methods: HashMap::new(),
            where_clauses: vec![],
        });

        let bounds = vec![TraitBound::Simple("Clone".to_string())];

        let result = resolver.check_bounds("String", &bounds);
        assert!(result.is_ok());
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

    // ========================================================================
    // WHERE CLAUSE DISTINCTION TESTS
    // ========================================================================

    #[test]
    fn test_constraint_set_from_empty_clauses() {
        let clauses: Vec<TraitBound> = vec![];
        let cs = ConstraintSet::from_clauses(&clauses);
        assert!(cs.is_empty());
        assert_eq!(cs.bounds.len(), 0);
    }

    #[test]
    fn test_constraint_set_from_single_clause() {
        let clauses = vec![TraitBound::Simple("Clone".to_string())];
        let cs = ConstraintSet::from_clauses(&clauses);
        assert!(!cs.is_empty());
        assert_eq!(cs.bounds.len(), 1);
    }

    #[test]
    fn test_constraint_set_from_multiple_clauses() {
        let clauses = vec![
            TraitBound::Simple("Clone".to_string()),
            TraitBound::Simple("Default".to_string()),
        ];
        let cs = ConstraintSet::from_clauses(&clauses);
        assert!(!cs.is_empty());
        assert_eq!(cs.bounds.len(), 2);
    }

    #[test]
    fn test_constraint_set_sorting() {
        // Clauses added in different orders should produce same result
        let clauses1 = vec![
            TraitBound::Simple("Clone".to_string()),
            TraitBound::Simple("Debug".to_string()),
        ];
        let clauses2 = vec![
            TraitBound::Simple("Debug".to_string()),
            TraitBound::Simple("Clone".to_string()),
        ];
        
        let cs1 = ConstraintSet::from_clauses(&clauses1);
        let cs2 = ConstraintSet::from_clauses(&clauses2);
        
        assert_eq!(cs1, cs2);
    }

    #[test]
    fn test_constraint_set_is_subset_of() {
        let clauses_small = vec![TraitBound::Simple("Clone".to_string())];
        let clauses_large = vec![
            TraitBound::Simple("Clone".to_string()),
            TraitBound::Simple("Debug".to_string()),
        ];
        
        let cs_small = ConstraintSet::from_clauses(&clauses_small);
        let cs_large = ConstraintSet::from_clauses(&clauses_large);
        
        assert!(cs_small.is_subset_of(&cs_large));
        assert!(!cs_large.is_subset_of(&cs_small));
    }

    #[test]
    fn test_constraint_set_compatible_identical() {
        let clauses = vec![TraitBound::Simple("Clone".to_string())];
        let cs1 = ConstraintSet::from_clauses(&clauses);
        let cs2 = ConstraintSet::from_clauses(&clauses);
        
        assert!(cs1.compatible_with(&cs2));
    }

    #[test]
    fn test_constraint_set_compatible_subset() {
        let clauses_small = vec![TraitBound::Simple("Clone".to_string())];
        let clauses_large = vec![
            TraitBound::Simple("Clone".to_string()),
            TraitBound::Simple("Debug".to_string()),
        ];
        
        let cs_small = ConstraintSet::from_clauses(&clauses_small);
        let cs_large = ConstraintSet::from_clauses(&clauses_large);
        
        assert!(cs_small.compatible_with(&cs_large));
        assert!(cs_large.compatible_with(&cs_small));
    }

    #[test]
    fn test_constraint_set_incompatible() {
        let clauses1 = vec![TraitBound::Simple("Clone".to_string())];
        let clauses2 = vec![TraitBound::Simple("Default".to_string())];
        
        let cs1 = ConstraintSet::from_clauses(&clauses1);
        let cs2 = ConstraintSet::from_clauses(&clauses2);
        
        assert!(!cs1.compatible_with(&cs2));
    }

    #[test]
    fn test_where_clauses_similar_both_empty() {
        // Both blanket impls - they can conflict if patterns overlap
        let checker = TraitResolver::new();
        let clauses1: Vec<TraitBound> = vec![];
        let clauses2: Vec<TraitBound> = vec![];
        
        assert!(checker.where_clauses_similar(&clauses1, &clauses2));
    }

    #[test]
    fn test_where_clauses_similar_blanket_vs_constrained() {
        // Blanket impl vs constrained impl - no conflict (specialization)
        let checker = TraitResolver::new();
        let clauses1: Vec<TraitBound> = vec![];
        let clauses2 = vec![TraitBound::Simple("Clone".to_string())];
        
        // Different constraints: one is blanket, one is constrained
        // They're compatible (not similar enough to conflict)
        assert!(!checker.where_clauses_similar(&clauses1, &clauses2));
    }

    #[test]
    fn test_where_clauses_similar_different_constraints() {
        // T: Clone vs T: Default - apply to different types
        let checker = TraitResolver::new();
        let clauses1 = vec![TraitBound::Simple("Clone".to_string())];
        let clauses2 = vec![TraitBound::Simple("Default".to_string())];
        
        // Different constraints: incompatible
        assert!(!checker.where_clauses_similar(&clauses1, &clauses2));
    }

    #[test]
    fn test_where_clauses_similar_same_constraints() {
        // T: Clone vs T: Clone - identical
        let checker = TraitResolver::new();
        let clauses1 = vec![TraitBound::Simple("Clone".to_string())];
        let clauses2 = vec![TraitBound::Simple("Clone".to_string())];
        
        // Same constraints: similar enough to conflict
        assert!(checker.where_clauses_similar(&clauses1, &clauses2));
    }

    #[test]
    fn test_where_clauses_similar_subset() {
        // T: Clone, Debug vs T: Clone - subset relationship
        let checker = TraitResolver::new();
        let clauses1 = vec![
            TraitBound::Simple("Clone".to_string()),
            TraitBound::Simple("Debug".to_string()),
        ];
        let clauses2 = vec![TraitBound::Simple("Clone".to_string())];
        
        // Subset: compatible
        assert!(checker.where_clauses_similar(&clauses1, &clauses2));
    }

    #[test]
    fn test_blanket_vs_specialized_impl_no_conflict() {
        // Scenario: impl<T> Trait for T and impl<T: Clone> Trait for T
        // Should NOT be detected as overlapping
        let checker = TraitResolver::new();
        
        let blanket_impl = GenericTraitImpl {
            trait_name: "MyTrait".to_string(),
            impl_type: "T".to_string(),
            type_params: vec![TypeParameter {
                name: "T".to_string(),
                bounds: vec![],
                variance: Variance::Covariant,
                default: None,
            }],
            where_clauses: vec![],  // No constraints
            associated_types: HashMap::new(),
        };
        
        let specialized_impl = GenericTraitImpl {
            trait_name: "MyTrait".to_string(),
            impl_type: "T".to_string(),
            type_params: vec![TypeParameter {
                name: "T".to_string(),
                bounds: vec![],
                variance: Variance::Covariant,
                default: None,
            }],
            where_clauses: vec![TraitBound::Simple("Clone".to_string())],  // Constrained
            associated_types: HashMap::new(),
        };
        
        // They should NOT overlap (specialized is a specialization)
        assert!(!checker.impls_might_overlap(&blanket_impl, &specialized_impl));
    }

    #[test]
    fn test_different_constraint_impls_no_conflict() {
        // impl<T: Clone> Trait for T and impl<T: Default> Trait for T
        // Apply to different types - no conflict
        let checker = TraitResolver::new();
        
        let clone_impl = GenericTraitImpl {
            trait_name: "MyTrait".to_string(),
            impl_type: "T".to_string(),
            type_params: vec![TypeParameter {
                name: "T".to_string(),
                bounds: vec![],
                variance: Variance::Covariant,
                default: None,
            }],
            where_clauses: vec![TraitBound::Simple("Clone".to_string())],
            associated_types: HashMap::new(),
        };
        
        let default_impl = GenericTraitImpl {
            trait_name: "MyTrait".to_string(),
            impl_type: "T".to_string(),
            type_params: vec![TypeParameter {
                name: "T".to_string(),
                bounds: vec![],
                variance: Variance::Covariant,
                default: None,
            }],
            where_clauses: vec![TraitBound::Simple("Default".to_string())],
            associated_types: HashMap::new(),
        };
        
        // Different constraints - should NOT overlap
        assert!(!checker.impls_might_overlap(&clone_impl, &default_impl));
    }

    #[test]
    fn test_identical_constraint_impls_conflict() {
        // impl<T: Clone> Trait for T and impl<T: Clone> Trait for T
        // Identical - should conflict
        let checker = TraitResolver::new();
        
        let impl1 = GenericTraitImpl {
            trait_name: "MyTrait".to_string(),
            impl_type: "T".to_string(),
            type_params: vec![TypeParameter {
                name: "T".to_string(),
                bounds: vec![],
                variance: Variance::Covariant,
                default: None,
            }],
            where_clauses: vec![TraitBound::Simple("Clone".to_string())],
            associated_types: HashMap::new(),
        };
        
        let impl2 = GenericTraitImpl {
            trait_name: "MyTrait".to_string(),
            impl_type: "T".to_string(),
            type_params: vec![TypeParameter {
                name: "T".to_string(),
                bounds: vec![],
                variance: Variance::Covariant,
                default: None,
            }],
            where_clauses: vec![TraitBound::Simple("Clone".to_string())],
            associated_types: HashMap::new(),
        };
        
        // Same constraints and same pattern - should overlap
        assert!(checker.impls_might_overlap(&impl1, &impl2));
    }
}
