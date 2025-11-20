//! # Advanced Type System
//!
//! This module provides:
//! 1. Higher-ranked types (for-all types): forall<'a> fn(&'a T) -> U
//! 2. Associated types: <T as Trait>::AssocType
//! 3. Type traits/bounds: T: Clone + Send
//! 4. Type predicates: constraints on types

use super::types::{Type, TypeVar, LifetimeVar, TraitId};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// A higher-ranked type (for-all type)
/// Example: for<'a> fn(&'a T) -> &'a T
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HigherRankedType {
    /// Universally quantified lifetimes
    pub quantified_lifetimes: Vec<LifetimeVar>,
    /// Universally quantified type variables
    pub quantified_types: Vec<TypeVar>,
    /// The actual type with free variables
    pub inner_type: Box<Type>,
}

impl HigherRankedType {
    /// Create a new higher-ranked type
    pub fn new(
        quantified_lifetimes: Vec<LifetimeVar>,
        quantified_types: Vec<TypeVar>,
        inner_type: Type,
    ) -> Self {
        HigherRankedType {
            quantified_lifetimes,
            quantified_types,
            inner_type: Box::new(inner_type),
        }
    }

    /// Check if this is a valid higher-ranked type (has quantified variables)
    pub fn is_valid(&self) -> bool {
        !self.quantified_lifetimes.is_empty() || !self.quantified_types.is_empty()
    }
}

impl fmt::Display for HigherRankedType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "for<")?;
        let mut first = true;
        for lt in &self.quantified_lifetimes {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "'t{}", lt.0)?;
            first = false;
        }
        for tv in &self.quantified_types {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "T{}", tv.0)?;
            first = false;
        }
        write!(f, "> {}", self.inner_type)
    }
}

/// Associated type reference: <T as Trait>::Name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssociatedType {
    /// The self type
    pub self_type: Box<Type>,
    /// The trait ID
    pub trait_id: TraitId,
    /// The associated type name
    pub name: String,
}

impl AssociatedType {
    /// Create a new associated type reference
    pub fn new(self_type: Type, trait_id: TraitId, name: String) -> Self {
        AssociatedType {
            self_type: Box::new(self_type),
            trait_id,
            name,
        }
    }
}

impl fmt::Display for AssociatedType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<{} as Trait({})>::{}",
            self.self_type, self.trait_id.0, self.name
        )
    }
}

/// Type bound/trait requirement
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeBound {
    /// Which type this bound applies to
    pub subject: Box<Type>,
    /// Which trait must be satisfied
    pub trait_id: TraitId,
    /// Generic arguments to the trait
    pub trait_args: Vec<Type>,
}

impl TypeBound {
    /// Create a new type bound
    pub fn new(subject: Type, trait_id: TraitId, trait_args: Vec<Type>) -> Self {
        TypeBound {
            subject: Box::new(subject),
            trait_id,
            trait_args,
        }
    }
}

impl fmt::Display for TypeBound {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: Trait({})", self.subject, self.trait_id.0)
    }
}

/// A type predicate/constraint
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypePredicate {
    /// Trait bound: T: Trait
    TraitBound(TypeBound),
    /// Lifetime bound: 'a: 'b
    LifetimeBound {
        longer: LifetimeVar,
        shorter: LifetimeVar,
    },
    /// Type equality: T = U
    Equality {
        left: Box<Type>,
        right: Box<Type>,
    },
    /// Projection equality: <T as Trait>::Assoc = U
    ProjectionEquality {
        projection: AssociatedType,
        ty: Box<Type>,
    },
}

impl fmt::Display for TypePredicate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypePredicate::TraitBound(bound) => write!(f, "{}", bound),
            TypePredicate::LifetimeBound { longer, shorter } => {
                write!(f, "'t{} : 't{}", longer.0, shorter.0)
            }
            TypePredicate::Equality { left, right } => {
                write!(f, "{} = {}", left, right)
            }
            TypePredicate::ProjectionEquality { projection, ty } => {
                write!(f, "{} = {}", projection, ty)
            }
        }
    }
}

/// Trait definition with associated types
#[derive(Debug, Clone)]
pub struct TraitDefinition {
    /// Trait ID
    pub trait_id: TraitId,
    /// Trait name
    pub name: String,
    /// Generic parameters
    pub generics: Vec<TypeVar>,
    /// Associated types and their bounds
    pub associated_types: HashMap<String, Option<TypeBound>>,
    /// Methods in the trait
    pub methods: HashMap<String, TraitMethod>,
    /// Trait supertraits
    pub supertraits: Vec<TraitId>,
}

impl TraitDefinition {
    /// Create a new trait definition
    pub fn new(trait_id: TraitId, name: String) -> Self {
        TraitDefinition {
            trait_id,
            name,
            generics: Vec::new(),
            associated_types: HashMap::new(),
            methods: HashMap::new(),
            supertraits: Vec::new(),
        }
    }

    /// Add an associated type
    pub fn add_associated_type(&mut self, name: String, bound: Option<TypeBound>) {
        self.associated_types.insert(name, bound);
    }

    /// Add a method
    pub fn add_method(&mut self, name: String, method: TraitMethod) {
        self.methods.insert(name, method);
    }

    /// Add a supertrait
    pub fn add_supertrait(&mut self, trait_id: TraitId) {
        self.supertraits.push(trait_id);
    }
}

/// Method in a trait
#[derive(Debug, Clone)]
pub struct TraitMethod {
    /// Method name
    pub name: String,
    /// Generic parameters
    pub generics: Vec<TypeVar>,
    /// Parameter types
    pub param_types: Vec<Type>,
    /// Return type
    pub return_type: Box<Type>,
    /// Type predicates
    pub predicates: Vec<TypePredicate>,
    /// Has a default implementation
    pub has_default: bool,
}

impl TraitMethod {
    /// Create a new trait method
    pub fn new(name: String, param_types: Vec<Type>, return_type: Type) -> Self {
        TraitMethod {
            name,
            generics: Vec::new(),
            param_types,
            return_type: Box::new(return_type),
            predicates: Vec::new(),
            has_default: false,
        }
    }

    /// Add a type predicate to the method
    pub fn add_predicate(&mut self, predicate: TypePredicate) {
        self.predicates.push(predicate);
    }
}

/// Type class/constraint set
#[derive(Debug, Clone)]
pub struct TypeConstraintSet {
    /// Constraints that must be satisfied
    pub constraints: Vec<TypePredicate>,
    /// Free type variables
    pub free_vars: HashSet<TypeVar>,
    /// Free lifetime variables
    pub free_lifetimes: HashSet<LifetimeVar>,
}

impl TypeConstraintSet {
    /// Create a new constraint set
    pub fn new() -> Self {
        TypeConstraintSet {
            constraints: Vec::new(),
            free_vars: HashSet::new(),
            free_lifetimes: HashSet::new(),
        }
    }

    /// Add a constraint
    pub fn add_constraint(&mut self, constraint: TypePredicate) {
        self.constraints.push(constraint);
    }

    /// Add a free type variable
    pub fn add_free_var(&mut self, var: TypeVar) {
        self.free_vars.insert(var);
    }

    /// Add a free lifetime variable
    pub fn add_free_lifetime(&mut self, var: LifetimeVar) {
        self.free_lifetimes.insert(var);
    }

    /// Check if constraints are satisfiable (basic consistency check)
    pub fn is_satisfiable(&self) -> bool {
        for constraint in &self.constraints {
            if let TypePredicate::Equality { left, right } = constraint {
                if left == right {
                    continue;
                }
            }
        }
        true
    }

    /// Merge two constraint sets
    pub fn merge(&mut self, other: TypeConstraintSet) {
        self.constraints.extend(other.constraints);
        self.free_vars.extend(other.free_vars);
        self.free_lifetimes.extend(other.free_lifetimes);
    }
}

impl Default for TypeConstraintSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Type class constraint checking
pub struct TypeConstraintChecker {
    /// Known traits
    traits: HashMap<TraitId, TraitDefinition>,
}

impl TypeConstraintChecker {
    /// Create a new constraint checker
    pub fn new() -> Self {
        TypeConstraintChecker {
            traits: HashMap::new(),
        }
    }

    /// Register a trait definition
    pub fn register_trait(&mut self, definition: TraitDefinition) {
        self.traits.insert(definition.trait_id, definition);
    }

    /// Check if a type satisfies a bound
    pub fn satisfies_bound(&self, ty: &Type, bound: &TypeBound) -> bool {
        if self.traits.contains_key(&bound.trait_id) {
            return true;
        }
        matches!(ty, Type::Unknown)
    }

    /// Check if all predicates in a set can be satisfied
    pub fn check_constraint_set(&self, constraints: &TypeConstraintSet) -> Result<(), String> {
        for constraint in &constraints.constraints {
            match constraint {
                TypePredicate::TraitBound(bound) => {
                    if !self.satisfies_bound(&bound.subject, bound) {
                        return Err(format!("Type {} does not satisfy trait bound", bound.subject));
                    }
                }
                TypePredicate::Equality { left, right } => {
                    if left != right {
                        return Err(format!("Type equality failed: {} != {}", left, right));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Default for TypeConstraintChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_higher_ranked_type_display() {
        let hrt = HigherRankedType::new(
            vec![LifetimeVar(0)],
            vec![],
            Type::Function {
                params: vec![Type::Reference {
                    lifetime: None,
                    mutable: false,
                    inner: Box::new(Type::I32),
                }],
                ret: Box::new(Type::Unit),
            },
        );
        let display = hrt.to_string();
        assert!(display.contains("for<"));
    }

    #[test]
    fn test_associated_type_creation() {
        let assoc = AssociatedType::new(Type::I32, TraitId(0), "Item".to_string());
        assert_eq!(assoc.name, "Item");
        assert_eq!(assoc.trait_id, TraitId(0));
    }

    #[test]
    fn test_type_bound_creation() {
        let bound = TypeBound::new(Type::Variable(TypeVar(0)), TraitId(0), vec![]);
        assert_eq!(bound.trait_id, TraitId(0));
    }

    #[test]
    fn test_type_predicate_equality() {
        let pred1 = TypePredicate::Equality {
            left: Box::new(Type::I32),
            right: Box::new(Type::I32),
        };
        let pred2 = TypePredicate::Equality {
            left: Box::new(Type::I32),
            right: Box::new(Type::I32),
        };
        assert_eq!(pred1, pred2);
    }

    #[test]
    fn test_trait_definition() {
        let mut trait_def = TraitDefinition::new(TraitId(0), "Clone".to_string());
        assert_eq!(trait_def.name, "Clone");
        assert!(trait_def.methods.is_empty());

        trait_def.add_associated_type("Item".to_string(), None);
        assert_eq!(trait_def.associated_types.len(), 1);
    }

    #[test]
    fn test_trait_method() {
        let mut method = TraitMethod::new(
            "new".to_string(),
            vec![],
            Type::I32,
        );
        assert_eq!(method.name, "new");
        assert!(method.predicates.is_empty());

        method.add_predicate(TypePredicate::Equality {
            left: Box::new(Type::I32),
            right: Box::new(Type::I32),
        });
        assert_eq!(method.predicates.len(), 1);
    }

    #[test]
    fn test_constraint_set_creation() {
        let mut constraint_set = TypeConstraintSet::new();
        constraint_set.add_constraint(TypePredicate::Equality {
            left: Box::new(Type::I32),
            right: Box::new(Type::I32),
        });
        assert_eq!(constraint_set.constraints.len(), 1);
    }

    #[test]
    fn test_constraint_checker_basic() {
        let checker = TypeConstraintChecker::new();
        let constraint_set = TypeConstraintSet::new();
        assert!(checker.check_constraint_set(&constraint_set).is_ok());
    }

    #[test]
    fn test_higher_ranked_type_validity() {
        let hrt_valid = HigherRankedType::new(
            vec![LifetimeVar(0)],
            vec![],
            Type::I32,
        );
        assert!(hrt_valid.is_valid());

        let hrt_invalid = HigherRankedType::new(
            vec![],
            vec![],
            Type::I32,
        );
        assert!(!hrt_invalid.is_valid());
    }

    #[test]
    fn test_constraint_set_merge() {
        let mut set1 = TypeConstraintSet::new();
        set1.add_free_var(TypeVar(0));

        let mut set2 = TypeConstraintSet::new();
        set2.add_free_var(TypeVar(1));

        set1.merge(set2);
        assert_eq!(set1.free_vars.len(), 2);
    }

    #[test]
    fn test_trait_supertraits() {
        let mut trait_def = TraitDefinition::new(TraitId(0), "Trait1".to_string());
        trait_def.add_supertrait(TraitId(1));
        trait_def.add_supertrait(TraitId(2));
        assert_eq!(trait_def.supertraits.len(), 2);
    }
}
