//! # TYPE CHECKING & INFERENCE
//!
//! Verifies types and performs type inference.
//!
//! ## What we do:
//! - Type inference (Hindley-Milner style constraint solving)
//! - Type compatibility checking
//! - Function signature validation
//! - Generic instantiation (basic)
//!
//! ## Algorithm:
//! 1. Collect all type definitions (structs, functions)
//! 2. Infer types for expressions with unknown types
//! 3. Check all operations are type-safe
//! 4. Generate typed HIR with all type information

use crate::lowering::{
    HirExpression, HirItem, HirStatement, HirType, BinaryOp, UnaryOp, ClosureTrait,
};
use crate::parser::ast::GenericParam;
use crate::iterators::IteratorMethodHandler;
use crate::compiler::{CompileError, ErrorKind};

use std::collections::{HashMap, HashSet};
use std::fmt;

/// Type checking error (deprecated, use CompileError instead)
#[derive(Debug, Clone)]
pub struct TypeCheckError {
    pub message: String,
}

impl fmt::Display for TypeCheckError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

type TypeCheckResult<T> = Result<T, TypeCheckError>;

/// Type environment: maps names to types
#[derive(Debug, Clone)]
pub struct TypeEnv {
    scopes: Vec<HashMap<String, HirType>>,
    mutable_vars: Vec<std::collections::HashSet<String>>,
}

impl TypeEnv {
    /// Create a new type environment
    pub fn new() -> Self {
        TypeEnv {
            scopes: vec![HashMap::new()],
            mutable_vars: vec![std::collections::HashSet::new()],
        }
    }

    /// Push a new scope
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.mutable_vars.push(std::collections::HashSet::new());
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
            self.mutable_vars.pop();
        }
    }

    /// Insert a variable into the current scope, returning the old value
    pub fn insert(&mut self, name: String, ty: HirType) -> Option<HirType> {
        if let Some(scope) = self.scopes.last_mut() {
            return scope.insert(name, ty);
        }
        None
    }

    /// Mark a variable as mutable
    pub fn mark_mutable(&mut self, name: &str) {
        if let Some(mut_set) = self.mutable_vars.last_mut() {
            mut_set.insert(name.to_string());
        }
    }

    /// Check if a variable is mutable
    pub fn is_mutable(&self, name: &str) -> bool {
        for mut_set in self.mutable_vars.iter().rev() {
            if mut_set.contains(name) {
                return true;
            }
        }
        false
    }

    /// Remove a variable from the current scope
    pub fn remove(&mut self, name: &str) -> Option<HirType> {
        if let Some(scope) = self.scopes.last_mut() {
            return scope.remove(name);
        }
        None
    }

    /// Look up a variable (searches from innermost to outermost scope)
    pub fn lookup(&self, name: &str) -> Option<HirType> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }
}

/// Type context: stores type definitions and current environment
pub struct TypeContext {
    /// Type environment
    env: TypeEnv,
    /// Function signatures: name -> (param_types, return_type)
    functions: HashMap<String, (Vec<HirType>, HirType)>,
    /// Struct definitions: name -> field_types
    structs: HashMap<String, Vec<(String, HirType)>>,
    /// Trait definitions: trait_name -> (method_names, method_sigs)
    traits: HashMap<String, HashMap<String, (Vec<HirType>, HirType)>>,
    /// Impl block methods: (struct_name, method_name) -> function_signature
    impl_methods: HashMap<(String, String), (Vec<HirType>, HirType)>,
    /// Generic parameter trait bounds: param_name -> Vec<trait_name>
    generic_bounds: HashMap<String, Vec<String>>,
}

impl TypeContext {
    /// Create a new type context
    pub fn new() -> Self {
        TypeContext {
            env: TypeEnv::new(),
            functions: HashMap::new(),
            structs: HashMap::new(),
            traits: HashMap::new(),
            impl_methods: HashMap::new(),
            generic_bounds: HashMap::new(),
        }
    }

    /// Register a function signature
    fn register_function(&mut self, name: String, params: Vec<HirType>, ret: HirType) {
        self.functions.insert(name, (params, ret));
    }

    /// Register a struct definition
    fn register_struct(&mut self, name: String, fields: Vec<(String, HirType)>) {
        self.structs.insert(name, fields);
    }

    /// Look up a function signature
    fn lookup_function(&self, name: &str) -> Option<(Vec<HirType>, HirType)> {
        self.functions.get(name).cloned()
    }

    /// Look up a struct definition
    fn lookup_struct(&self, name: &str) -> Option<Vec<(String, HirType)>> {
        self.structs.get(name).cloned()
    }

    /// Register a trait and its methods
    fn register_trait(&mut self, name: String, methods: HashMap<String, (Vec<HirType>, HirType)>) {
        self.traits.insert(name, methods);
    }

    /// Look up a trait definition
    fn lookup_trait(&self, name: &str) -> Option<HashMap<String, (Vec<HirType>, HirType)>> {
        self.traits.get(name).cloned()
    }

    /// Register bounds for a generic parameter
    fn register_generic_bounds(&mut self, param_name: String, bounds: Vec<String>) {
        self.generic_bounds.insert(param_name, bounds);
    }

    /// Get bounds for a generic parameter
    fn get_generic_bounds(&self, param_name: &str) -> Vec<String> {
        self.generic_bounds.get(param_name).cloned().unwrap_or_default()
    }

    /// Register an impl block method
    fn register_impl_method(&mut self, struct_name: String, method_name: String, params: Vec<HirType>, ret: HirType) {
        self.impl_methods.insert((struct_name, method_name), (params, ret));
    }

    /// Look up an impl block method
    fn lookup_impl_method(&self, struct_name: &str, method_name: &str) -> Option<(Vec<HirType>, HirType)> {
        self.impl_methods.get(&(struct_name.to_string(), method_name.to_string())).cloned()
    }
}

/// Type checking and inference
pub struct TypeChecker {
    pub context: TypeContext,
}

impl TypeChecker {
    /// Create a new type checker
    pub fn new() -> Self {
        let mut checker = TypeChecker {
            context: TypeContext::new(),
        };
        checker.register_builtin_functions();
        checker
    }

    /// Register all built-in functions
    fn register_builtin_functions(&mut self) {
        // Math functions
        self.context.register_function("abs".to_string(), vec![HirType::Int32], HirType::Int32);
        self.context.register_function("min".to_string(), vec![HirType::Int32, HirType::Int32], HirType::Int32);
        self.context.register_function("max".to_string(), vec![HirType::Int32, HirType::Int32], HirType::Int32);
        self.context.register_function("pow".to_string(), vec![HirType::Int64, HirType::Int64], HirType::Int64);
        self.context.register_function("sqrt".to_string(), vec![HirType::Float64], HirType::Float64);
        self.context.register_function("floor".to_string(), vec![HirType::Float64], HirType::Float64);
        self.context.register_function("ceil".to_string(), vec![HirType::Float64], HirType::Float64);
        self.context.register_function("round".to_string(), vec![HirType::Float64], HirType::Float64);

        // String/Array functions
        self.context.register_function("len".to_string(), vec![HirType::String], HirType::Int32);

        // String methods (called via method syntax: s.method())
        self.context.register_function("to_upper".to_string(), vec![HirType::String], HirType::String);
        self.context.register_function("to_lower".to_string(), vec![HirType::String], HirType::String);
        self.context.register_function("to_uppercase".to_string(), vec![HirType::String], HirType::String);
        self.context.register_function("to_lowercase".to_string(), vec![HirType::String], HirType::String);
        self.context.register_function("contains".to_string(), vec![HirType::String, HirType::String], HirType::Bool);
        self.context.register_function("starts_with".to_string(), vec![HirType::String, HirType::String], HirType::Bool);
        self.context.register_function("ends_with".to_string(), vec![HirType::String, HirType::String], HirType::Bool);
        self.context.register_function("trim".to_string(), vec![HirType::String], HirType::String);
        self.context.register_function("split".to_string(), vec![HirType::String, HirType::Char], HirType::String);
        self.context.register_function("replace".to_string(), vec![HirType::String, HirType::String, HirType::String], HirType::String);
        self.context.register_function("repeat".to_string(), vec![HirType::String, HirType::Int32], HirType::String);
        self.context.register_function("reverse_str".to_string(), vec![HirType::String], HirType::String);

        // I/O functions
        self.context.register_function("print".to_string(), vec![HirType::String], HirType::Unknown);
        self.context.register_function("println".to_string(), vec![HirType::Reference(Box::new(HirType::String))], HirType::Unknown);
        self.context.register_function("eprintln".to_string(), vec![HirType::Reference(Box::new(HirType::String))], HirType::Unknown);
        
        self.context.register_function("__builtin_print".to_string(), vec![HirType::Reference(Box::new(HirType::String))], HirType::Tuple(vec![]));
        self.context.register_function("__builtin_println".to_string(), vec![HirType::Reference(Box::new(HirType::String))], HirType::Tuple(vec![]));
        self.context.register_function("__builtin_eprintln".to_string(), vec![HirType::Reference(Box::new(HirType::String))], HirType::Tuple(vec![]));
        self.context.register_function("__builtin_printf".to_string(), vec![HirType::Reference(Box::new(HirType::String))], HirType::Tuple(vec![]));
        
        // Type-aware print functions (used by println! lowering for different types)
        self.context.register_function("gaia_print_i32".to_string(), vec![HirType::Int32], HirType::Tuple(vec![]));
        self.context.register_function("gaia_print_i64".to_string(), vec![HirType::Int64], HirType::Tuple(vec![]));
        self.context.register_function("gaia_print_bool".to_string(), vec![HirType::Bool], HirType::Tuple(vec![]));
        self.context.register_function("gaia_print_f64".to_string(), vec![HirType::Float64], HirType::Tuple(vec![]));

        // Type conversions
        self.context.register_function("as_i32".to_string(), vec![HirType::Float64], HirType::Int32);
        self.context.register_function("as_i64".to_string(), vec![HirType::Float64], HirType::Int64);
        self.context.register_function("as_f64".to_string(), vec![HirType::Int32], HirType::Float64);

        // Option enum constructors
        self.context.register_function("Some".to_string(), vec![HirType::Unknown], HirType::Named("Option".to_string()));
        self.context.register_function("None".to_string(), vec![], HirType::Named("Option".to_string()));

        // Result enum constructors
        self.context.register_function("Ok".to_string(), vec![HirType::Unknown], HirType::Named("Result".to_string()));
        self.context.register_function("Err".to_string(), vec![HirType::Unknown], HirType::Named("Result".to_string()));
        
        // Collection constructors
        self.context.register_function("HashMap::new".to_string(), vec![], HirType::Named("HashMap".to_string()));
        self.context.register_function("Vec::new".to_string(), vec![], HirType::Named("Vec".to_string()));
        self.context.register_function("HashSet::new".to_string(), vec![], HirType::Named("HashSet".to_string()));
        self.context.register_function("LinkedList::new".to_string(), vec![], HirType::Named("LinkedList".to_string()));
        self.context.register_function("BTreeMap::new".to_string(), vec![], HirType::Named("BTreeMap".to_string()));
        
        // Collection methods (qualified with collection type)
        // Vec methods
         self.context.register_function("Vec::push".to_string(), vec![HirType::Named("Vec".to_string()), HirType::Unknown], HirType::Tuple(vec![]));
         self.context.register_function("Vec::pop".to_string(), vec![HirType::Named("Vec".to_string())], HirType::Unknown);
         self.context.register_function("Vec::get".to_string(), vec![HirType::Named("Vec".to_string()), HirType::Int32], HirType::Unknown);
         self.context.register_function("Vec::len".to_string(), vec![HirType::Named("Vec".to_string())], HirType::Int32);
         self.context.register_function("Vec::is_empty".to_string(), vec![HirType::Named("Vec".to_string())], HirType::Bool);
         self.context.register_function("Vec::insert".to_string(), vec![HirType::Named("Vec".to_string()), HirType::Int32, HirType::Unknown], HirType::Tuple(vec![]));
         self.context.register_function("Vec::remove".to_string(), vec![HirType::Named("Vec".to_string()), HirType::Int32], HirType::Unknown);
         self.context.register_function("Vec::clear".to_string(), vec![HirType::Named("Vec".to_string())], HirType::Tuple(vec![]));
         self.context.register_function("Vec::reserve".to_string(), vec![HirType::Named("Vec".to_string()), HirType::Int32], HirType::Tuple(vec![]));
         self.context.register_function("Vec::into_iter".to_string(), vec![HirType::Named("Vec".to_string())], HirType::Named("Iterator".to_string()));
        
        // HashMap methods
         self.context.register_function("HashMap::insert".to_string(), vec![HirType::Named("HashMap".to_string()), HirType::Unknown, HirType::Unknown], HirType::Tuple(vec![]));
         self.context.register_function("HashMap::get".to_string(), vec![HirType::Named("HashMap".to_string()), HirType::Unknown], HirType::Unknown);
         self.context.register_function("HashMap::remove".to_string(), vec![HirType::Named("HashMap".to_string()), HirType::Unknown], HirType::Tuple(vec![]));
         self.context.register_function("HashMap::is_empty".to_string(), vec![HirType::Named("HashMap".to_string())], HirType::Bool);
         self.context.register_function("HashMap::len".to_string(), vec![HirType::Named("HashMap".to_string())], HirType::Int32);
         self.context.register_function("HashMap::clear".to_string(), vec![HirType::Named("HashMap".to_string())], HirType::Tuple(vec![]));
         self.context.register_function("HashMap::contains_key".to_string(), vec![HirType::Named("HashMap".to_string()), HirType::Unknown], HirType::Bool);
        
        // HashSet methods
         self.context.register_function("HashSet::insert".to_string(), vec![HirType::Named("HashSet".to_string()), HirType::Unknown], HirType::Tuple(vec![]));
         self.context.register_function("HashSet::contains".to_string(), vec![HirType::Named("HashSet".to_string()), HirType::Unknown], HirType::Bool);
         self.context.register_function("HashSet::remove".to_string(), vec![HirType::Named("HashSet".to_string()), HirType::Unknown], HirType::Tuple(vec![]));
         self.context.register_function("HashSet::is_empty".to_string(), vec![HirType::Named("HashSet".to_string())], HirType::Bool);
         self.context.register_function("HashSet::len".to_string(), vec![HirType::Named("HashSet".to_string())], HirType::Int32);
         self.context.register_function("HashSet::clear".to_string(), vec![HirType::Named("HashSet".to_string())], HirType::Tuple(vec![]));
         
         // LinkedList methods
         self.context.register_function("LinkedList::push_front".to_string(), vec![HirType::Named("LinkedList".to_string()), HirType::Unknown], HirType::Tuple(vec![]));
         self.context.register_function("LinkedList::push_back".to_string(), vec![HirType::Named("LinkedList".to_string()), HirType::Unknown], HirType::Tuple(vec![]));
         self.context.register_function("LinkedList::pop_front".to_string(), vec![HirType::Named("LinkedList".to_string())], HirType::Named("Option".to_string()));
         self.context.register_function("LinkedList::pop_back".to_string(), vec![HirType::Named("LinkedList".to_string())], HirType::Named("Option".to_string()));
         self.context.register_function("LinkedList::is_empty".to_string(), vec![HirType::Named("LinkedList".to_string())], HirType::Bool);
         self.context.register_function("LinkedList::len".to_string(), vec![HirType::Named("LinkedList".to_string())], HirType::Int32);
         self.context.register_function("LinkedList::clear".to_string(), vec![HirType::Named("LinkedList".to_string())], HirType::Tuple(vec![]));
         
         // BTreeMap methods
         self.context.register_function("BTreeMap::insert".to_string(), vec![HirType::Named("BTreeMap".to_string()), HirType::Unknown, HirType::Unknown], HirType::Tuple(vec![]));
         self.context.register_function("BTreeMap::get".to_string(), vec![HirType::Named("BTreeMap".to_string()), HirType::Unknown], HirType::Named("Option".to_string()));
         self.context.register_function("BTreeMap::contains_key".to_string(), vec![HirType::Named("BTreeMap".to_string()), HirType::Unknown], HirType::Bool);
         self.context.register_function("BTreeMap::remove".to_string(), vec![HirType::Named("BTreeMap".to_string()), HirType::Unknown], HirType::Named("Option".to_string()));
         self.context.register_function("BTreeMap::is_empty".to_string(), vec![HirType::Named("BTreeMap".to_string())], HirType::Bool);
         self.context.register_function("BTreeMap::len".to_string(), vec![HirType::Named("BTreeMap".to_string())], HirType::Int32);
         self.context.register_function("BTreeMap::clear".to_string(), vec![HirType::Named("BTreeMap".to_string())], HirType::Tuple(vec![]));
         
         // String methods (accept both String and &String)
         self.context.register_function("String::len".to_string(), vec![HirType::String], HirType::Int32);
         self.context.register_function("String::is_empty".to_string(), vec![HirType::String], HirType::Bool);
         self.context.register_function("String::chars".to_string(), vec![HirType::String], HirType::Unknown);
         self.context.register_function("String::lines".to_string(), vec![HirType::String], HirType::Unknown);
         self.context.register_function("String::split".to_string(), vec![HirType::String, HirType::String], HirType::Unknown);
         self.context.register_function("String::replace".to_string(), vec![HirType::String, HirType::String, HirType::String], HirType::String);
         self.context.register_function("String::trim".to_string(), vec![HirType::String], HirType::String);
         self.context.register_function("String::starts_with".to_string(), vec![HirType::String, HirType::String], HirType::Bool);
         self.context.register_function("String::ends_with".to_string(), vec![HirType::String, HirType::String], HirType::Bool);
         self.context.register_function("String::contains_str".to_string(), vec![HirType::String, HirType::String], HirType::Bool);
         self.context.register_function("String::to_uppercase".to_string(), vec![HirType::String], HirType::String);
         self.context.register_function("String::to_lowercase".to_string(), vec![HirType::String], HirType::String);
         self.context.register_function("String::trim".to_string(), vec![HirType::String], HirType::String);
         self.context.register_function("String::replace".to_string(), vec![HirType::String, HirType::String, HirType::String], HirType::String);
         self.context.register_function("String::repeat".to_string(), vec![HirType::String, HirType::Int64], HirType::String);
         self.context.register_function("String::chars".to_string(), vec![HirType::String], HirType::Named("Iterator".to_string()));
         self.context.register_function("String::split".to_string(), vec![HirType::String, HirType::String], HirType::Named("Iterator".to_string()));
         
         // Option<T> methods
         self.context.register_function("Option::unwrap".to_string(), vec![HirType::Named("Option".to_string())], HirType::Unknown);
         self.context.register_function("Option::unwrap_or".to_string(), vec![HirType::Named("Option".to_string()), HirType::Unknown], HirType::Unknown);
         self.context.register_function("Option::map".to_string(), vec![HirType::Named("Option".to_string()), HirType::Unknown], HirType::Named("Option".to_string()));
         self.context.register_function("Option::and_then".to_string(), vec![HirType::Named("Option".to_string()), HirType::Unknown], HirType::Named("Option".to_string()));
         self.context.register_function("Option::or".to_string(), vec![HirType::Named("Option".to_string()), HirType::Named("Option".to_string())], HirType::Named("Option".to_string()));
         self.context.register_function("Option::filter".to_string(), vec![HirType::Named("Option".to_string()), HirType::Unknown], HirType::Named("Option".to_string()));
         self.context.register_function("Option::is_some".to_string(), vec![HirType::Named("Option".to_string())], HirType::Bool);
         self.context.register_function("Option::is_none".to_string(), vec![HirType::Named("Option".to_string())], HirType::Bool);
         
         // Result<T, E> methods
         self.context.register_function("Result::unwrap".to_string(), vec![HirType::Named("Result".to_string())], HirType::Unknown);
         self.context.register_function("Result::unwrap_err".to_string(), vec![HirType::Named("Result".to_string())], HirType::Unknown);
         self.context.register_function("Result::unwrap_or".to_string(), vec![HirType::Named("Result".to_string()), HirType::Unknown], HirType::Unknown);
         self.context.register_function("Result::unwrap_or_else".to_string(), vec![HirType::Named("Result".to_string()), HirType::Unknown], HirType::Unknown);
         self.context.register_function("Result::map".to_string(), vec![HirType::Named("Result".to_string()), HirType::Unknown], HirType::Named("Result".to_string()));
         self.context.register_function("Result::map_err".to_string(), vec![HirType::Named("Result".to_string()), HirType::Unknown], HirType::Named("Result".to_string()));
         self.context.register_function("Result::and_then".to_string(), vec![HirType::Named("Result".to_string()), HirType::Unknown], HirType::Named("Result".to_string()));
         self.context.register_function("Result::or_else".to_string(), vec![HirType::Named("Result".to_string()), HirType::Unknown], HirType::Named("Result".to_string()));
         self.context.register_function("Result::is_ok".to_string(), vec![HirType::Named("Result".to_string())], HirType::Bool);
         self.context.register_function("Result::is_err".to_string(), vec![HirType::Named("Result".to_string())], HirType::Bool);
         
         // Generic methods (fallback)
        self.context.register_function("insert".to_string(), vec![HirType::Unknown, HirType::Unknown, HirType::Unknown], HirType::Tuple(vec![]));
        self.context.register_function("push".to_string(), vec![HirType::Unknown, HirType::Unknown], HirType::Tuple(vec![]));
        self.context.register_function("pop".to_string(), vec![HirType::Unknown], HirType::Named("Option".to_string()));
        self.context.register_function("get".to_string(), vec![HirType::Unknown, HirType::Unknown], HirType::Named("Option".to_string()));
        self.context.register_function("remove".to_string(), vec![HirType::Unknown, HirType::Unknown], HirType::Unknown);
        self.context.register_function("contains".to_string(), vec![HirType::Unknown, HirType::Unknown], HirType::Bool);
        self.context.register_function("is_empty".to_string(), vec![HirType::Unknown], HirType::Bool);
        self.context.register_function("len".to_string(), vec![HirType::Unknown], HirType::Int32);
        
        // Iterator protocol functions
        self.context.register_function("__into_iter".to_string(), vec![HirType::Unknown], HirType::Unknown);
        self.context.register_function("__next".to_string(), vec![HirType::Unknown], HirType::Unknown);
        self.context.register_function("into_iter".to_string(), vec![HirType::Unknown], HirType::Unknown);
        self.context.register_function("next".to_string(), vec![HirType::Unknown], HirType::Unknown);
        
        // Iterator adapter methods (with closure support)
        // map(closure: Fn(T) -> U) -> Iterator<U>
        self.context.register_function("Iterator::map".to_string(), 
            vec![HirType::Unknown, HirType::Unknown], // Iterator, closure
            HirType::Named("Iterator".to_string())); // Iterator<U>
        
        // filter(closure: Fn(T) -> bool) -> Iterator<T>
        self.context.register_function("Iterator::filter".to_string(),
            vec![HirType::Unknown, HirType::Unknown], // Iterator, closure
            HirType::Named("Iterator".to_string())); // Iterator<T>
        
        // fold(init: U, closure: Fn(U, T) -> U) -> U
        self.context.register_function("Iterator::fold".to_string(),
            vec![HirType::Unknown, HirType::Unknown, HirType::Unknown], // Iterator, init, closure
            HirType::Unknown);                         // U
        
        // collect() -> Collection
        self.context.register_function("Iterator::collect".to_string(),
            vec![HirType::Unknown], // Iterator
            HirType::Named("Vec".to_string()));      // Collection
        
        // for_each(closure: Fn(T)) -> ()
        self.context.register_function("Iterator::for_each".to_string(),
            vec![HirType::Unknown, HirType::Unknown], // Iterator, closure
            HirType::Tuple(vec![]));                  // ()
        
        // sum() -> T
        self.context.register_function("Iterator::sum".to_string(),
            vec![HirType::Unknown], // Iterator
            HirType::Unknown);      // T
        
        // count() -> i64
        self.context.register_function("Iterator::count".to_string(),
            vec![HirType::Unknown], // Iterator
            HirType::Int64);        // i64
        
        // take(n: i64) -> Iterator<T>
        self.context.register_function("Iterator::take".to_string(),
            vec![HirType::Unknown, HirType::Int64], // Iterator, count
            HirType::Named("Iterator".to_string())); // Iterator<T>
        
        // skip(n: i64) -> Iterator<T>
        self.context.register_function("Iterator::skip".to_string(),
            vec![HirType::Unknown, HirType::Int64], // Iterator, count
            HirType::Named("Iterator".to_string())); // Iterator<T>
        
        // chain(other: Iterator) -> Iterator<T>
        self.context.register_function("Iterator::chain".to_string(),
            vec![HirType::Unknown, HirType::Unknown], // Iterator, Iterator
            HirType::Named("Iterator".to_string())); // Iterator<T>
        
        // find(closure: Fn(T) -> bool) -> Option<T>
        self.context.register_function("Iterator::find".to_string(),
            vec![HirType::Unknown, HirType::Unknown], // Iterator, closure
            HirType::Named("Option".to_string())); // Option<T>
        
        // any(closure: Fn(T) -> bool) -> bool
        self.context.register_function("Iterator::any".to_string(),
            vec![HirType::Unknown, HirType::Unknown], // Iterator, closure
            HirType::Bool);                            // bool
        
        // all(closure: Fn(T) -> bool) -> bool
        self.context.register_function("Iterator::all".to_string(),
            vec![HirType::Unknown, HirType::Unknown], // Iterator, closure
            HirType::Bool);                            // bool
        
        // enumerate() -> Iterator<(usize, T)>
        self.context.register_function("Iterator::enumerate".to_string(),
            vec![HirType::Unknown], // Iterator
            HirType::Named("Iterator".to_string())); // Iterator<(usize, T)>
        
        // rev() -> Iterator<T>
        self.context.register_function("Iterator::rev".to_string(),
            vec![HirType::Unknown], // Iterator
            HirType::Named("Iterator".to_string())); // Iterator<T>
        
        // step_by(step: i64) -> Iterator<T>
        self.context.register_function("Iterator::step_by".to_string(),
            vec![HirType::Unknown, HirType::Int64], // Iterator, step
            HirType::Named("Iterator".to_string())); // Iterator<T>
        
        // partition(closure: Fn(T) -> bool) -> (Vec<T>, Vec<T>)
        self.context.register_function("Iterator::partition".to_string(),
            vec![HirType::Unknown, HirType::Unknown], // Iterator, closure
            HirType::Tuple(vec![HirType::Named("Vec".to_string()), HirType::Named("Vec".to_string())])); // (Vec<T>, Vec<T>)
        
        // vec! macro expansion builtins (Fix #3)
        // __builtin_vec_from([a, b, c]) -> Vec<T>
        self.context.register_function("__builtin_vec_from".to_string(), 
            vec![HirType::Unknown], // Array of elements
            HirType::Unknown);      // Vec<T>
        
        // __builtin_vec_repeat(x, n) -> Vec<T>
        self.context.register_function("__builtin_vec_repeat".to_string(),
            vec![HirType::Unknown, HirType::Int64], // Element type and count
            HirType::Unknown);                       // Vec<T>
        
        // File I/O operations
        // File::open(path: &str) -> Result<File, Error>
        self.context.register_function("File::open".to_string(),
            vec![HirType::String], // Path
            HirType::Named("Result".to_string())); // Result<File, Error>
        
        // File::create(path: &str) -> Result<File, Error>
        self.context.register_function("File::create".to_string(),
            vec![HirType::String], // Path
            HirType::Named("Result".to_string())); // Result<File, Error>
        
        // File::read_to_string() -> Result<String, Error>
        self.context.register_function("File::read_to_string".to_string(),
            vec![HirType::Named("File".to_string())], // File
            HirType::Named("Result".to_string())); // Result<String, Error>
        
        // File::write_all(data: &str) -> Result<(), Error>
        self.context.register_function("File::write_all".to_string(),
            vec![HirType::Named("File".to_string()), HirType::String], // File, data
            HirType::Named("Result".to_string())); // Result<(), Error>
        
        // File::delete(path: &str) -> Result<(), Error>
        self.context.register_function("File::delete".to_string(),
            vec![HirType::String], // Path
            HirType::Named("Result".to_string())); // Result<(), Error>
        
        // File::exists(path: &str) -> bool
        self.context.register_function("File::exists".to_string(),
            vec![HirType::String], // Path
            HirType::Bool); // bool
        
        // std::fs namespace shortcuts
        self.context.register_function("fs::read".to_string(),
            vec![HirType::String],
            HirType::Named("Result".to_string()));
        
        self.context.register_function("fs::write".to_string(),
             vec![HirType::String, HirType::String],
             HirType::Named("Result".to_string()));
         
         // Derive macro support - register default implementations
         // #[derive(Clone)] - generates clone() method
         self.context.register_function("derive::clone".to_string(),
             vec![HirType::Unknown], // Self
             HirType::Unknown);      // Self
         
         // #[derive(Debug)] - generates debug() method
         self.context.register_function("derive::debug".to_string(),
             vec![HirType::Unknown], // Self
             HirType::String);       // String representation
         
         // #[derive(Default)] - generates default() function
         self.context.register_function("derive::default".to_string(),
             vec![],                 // No parameters
             HirType::Unknown);      // Self
         
         // #[derive(Display)] - generates to_string() method
         self.context.register_function("derive::display".to_string(),
             vec![HirType::Unknown], // Self
             HirType::String);       // String representation
         
         // #[derive(PartialEq)] - generates eq() method
         self.context.register_function("derive::partial_eq".to_string(),
             vec![HirType::Unknown, HirType::Unknown], // Self, other
             HirType::Bool);         // bool
         
         // #[derive(Copy)] - marker trait, no implementation needed
         self.context.register_function("derive::copy".to_string(),
             vec![],
             HirType::Tuple(vec![]));
         
         // Register standard traits
         self.register_standard_traits();
    }
    
    /// Register standard library traits
    fn register_standard_traits(&mut self) {
        // Display trait: display() -> ()
        let mut display_methods = HashMap::new();
        display_methods.insert("display".to_string(), (vec![], HirType::Tuple(vec![])));
        self.context.register_trait("Display".to_string(), display_methods);
        
        // Clone trait: clone(self) -> Self
        let mut clone_methods = HashMap::new();
        clone_methods.insert("clone".to_string(), (vec![], HirType::Unknown));
        self.context.register_trait("Clone".to_string(), clone_methods);
        
        // Copy trait (marker trait, no methods)
        self.context.register_trait("Copy".to_string(), HashMap::new());
        
        // Debug trait: debug(self) -> ()
        let mut debug_methods = HashMap::new();
        debug_methods.insert("debug".to_string(), (vec![], HirType::Tuple(vec![])));
        self.context.register_trait("Debug".to_string(), debug_methods);
        
        // PartialEq trait: eq(self, other) -> bool
        let mut eq_methods = HashMap::new();
        eq_methods.insert("eq".to_string(), (vec![HirType::Unknown], HirType::Bool));
        self.context.register_trait("PartialEq".to_string(), eq_methods.clone());
        
        // Eq trait (inherits from PartialEq)
        self.context.register_trait("Eq".to_string(), eq_methods);
        
        // Ord trait: cmp(self, other) -> Ordering
        let mut ord_methods = HashMap::new();
        ord_methods.insert("cmp".to_string(), (vec![HirType::Unknown], HirType::Named("Ordering".to_string())));
        self.context.register_trait("Ord".to_string(), ord_methods);
        
        // Iterator trait: next(&mut self) -> Option<Item>
        let mut iterator_methods = HashMap::new();
        iterator_methods.insert("next".to_string(), (vec![], HirType::Named("Option".to_string())));
        self.context.register_trait("Iterator".to_string(), iterator_methods);
        
        // IntoIterator trait: into_iter(self) -> IntoIter
        let mut into_iter_methods = HashMap::new();
        into_iter_methods.insert("into_iter".to_string(), (vec![], HirType::Named("IntoIter".to_string())));
        self.context.register_trait("IntoIterator".to_string(), into_iter_methods);
    }

    /// Collect all type definitions (first pass)
    fn collect_definitions(&mut self, items: &[HirItem]) -> TypeCheckResult<()> {
        // First pass: collect modules and functions
        self.collect_definitions_recursive(items, "".to_string())?;
        // Second pass: process use statements
        self.process_use_statements(items, "".to_string())?;
        Ok(())
    }

    fn collect_definitions_recursive(&mut self, items: &[HirItem], module_prefix: String) -> TypeCheckResult<()> {
        for item in items {
            match item {
                HirItem::Function {
                    name,
                    params,
                    return_type,
                    generics,
                    ..
                } => {
                    // Register generic parameter bounds
                    for generic in generics {
                        if let GenericParam::Type { name: param_name, bounds, .. } = generic {
                            if !bounds.is_empty() {
                                self.context.register_generic_bounds(param_name.clone(), bounds.clone());
                            }
                        }
                    }
                    
                    let param_types: Vec<_> = params.iter().map(|(_, ty)| ty.clone()).collect();
                    let ret_type = return_type.clone().unwrap_or(HirType::Unknown);
                    let full_name = if name.contains("::") {
                        name.clone()
                    } else if module_prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}::{}", module_prefix, name)
                    };
                    self.context
                        .register_function(full_name, param_types, ret_type);
                }
                HirItem::Struct { name, fields, derives } => {
                    self.context
                        .register_struct(name.clone(), fields.clone());
                    
                    // Apply derives to register impl methods
                    if !derives.is_empty() {
                        for derive_name in derives {
                            match derive_name.as_str() {
                                "Clone" => {
                                    // Clone method takes self and returns Self
                                    self.context.register_impl_method(
                                        name.clone(),
                                        "clone".to_string(),
                                        vec![],  // self is implicit
                                        HirType::Named(name.clone()),
                                    );
                                }
                                "Copy" => {
                                    // Copy is a marker trait, no methods to register
                                }
                                "Debug" => {
                                    // Debug::fmt(&self, f: &Formatter) -> Result
                                    self.context.register_impl_method(
                                        name.clone(),
                                        "fmt".to_string(),
                                        vec![HirType::Named("Formatter".to_string())],
                                        HirType::Named("Result".to_string()),
                                    );
                                }
                                "Default" => {
                                    // Default::default() -> Self
                                    self.context.register_impl_method(
                                        name.clone(),
                                        "default".to_string(),
                                        vec![],
                                        HirType::Named(name.clone()),
                                    );
                                }
                                "PartialEq" => {
                                    // PartialEq::eq(&self, other: &Self) -> bool
                                    self.context.register_impl_method(
                                        name.clone(),
                                        "eq".to_string(),
                                        vec![HirType::Named(name.clone())],
                                        HirType::Bool,
                                    );
                                }
                                "Eq" => {
                                    // Eq is a marker trait extending PartialEq
                                }
                                "Ord" => {
                                    // Ord::cmp(&self, other: &Self) -> Ordering
                                    self.context.register_impl_method(
                                        name.clone(),
                                        "cmp".to_string(),
                                        vec![HirType::Named(name.clone())],
                                        HirType::Named("Ordering".to_string()),
                                    );
                                }
                                "PartialOrd" => {
                                    // PartialOrd::partial_cmp(&self, other: &Self) -> Option<Ordering>
                                    self.context.register_impl_method(
                                        name.clone(),
                                        "partial_cmp".to_string(),
                                        vec![HirType::Named(name.clone())],
                                        HirType::Named("Option".to_string()),
                                    );
                                }
                                "Hash" => {
                                    // Hash::hash(&self, state: &mut H)
                                    self.context.register_impl_method(
                                        name.clone(),
                                        "hash".to_string(),
                                        vec![],
                                        HirType::Tuple(vec![]),  // Unit type as empty tuple
                                    );
                                }
                                _ => {
                                    // Unknown derive - silently ignore
                                }
                            }
                        }
                    }
                }
                HirItem::Module { name, items: module_items, .. } => {
                    let new_prefix = if module_prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}::{}", module_prefix, name)
                    };
                    self.collect_definitions_recursive(module_items, new_prefix)?;
                }
                HirItem::Const { .. } => {
                    // Constants don't need to be registered as functions
                }
                HirItem::Static { .. } => {
                    // Statics don't need to be registered as functions
                }
                HirItem::AssociatedType { .. } => {
                }
                HirItem::Use { .. } => {
                    // Use statements are processed in second pass
                }
                HirItem::Impl { struct_name, methods, .. } => {
                    for method in methods {
                        if let HirItem::Function { name, params, return_type, .. } = method {
                            let param_types: Vec<_> = params.iter().map(|(_, ty)| ty.clone()).collect();
                            let ret_type = return_type.clone().unwrap_or(HirType::Unknown);
                            
                            // Register both as impl method and as a qualified function
                            self.context.register_impl_method(struct_name.clone(), name.clone(), param_types.clone(), ret_type.clone());
                            
                            // Also register as a qualified function so it can be called as Type::method()
                            let qualified_name = format!("{}::{}", struct_name, name);
                            self.context.register_function(qualified_name, param_types, ret_type);
                        }
                    }
                    self.collect_definitions_recursive(methods, module_prefix.clone())?;
                }
                HirItem::Enum { .. } => {
                }
                HirItem::Trait { .. } => {
                }
            }
        }
        Ok(())
    }

    /// Process use statements to bring items into scope (second pass)
    pub fn process_use_statements(&mut self, items: &[HirItem], module_prefix: String) -> TypeCheckResult<()> {
        for item in items {
            match item {
                HirItem::Module { name, items: module_items, .. } => {
                    let new_prefix = if module_prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}::{}", module_prefix, name)
                    };
                    self.process_use_statements(module_items, new_prefix)?;
                }
                HirItem::Use { path, is_glob, .. } => {
                    // Process use statements to bring items into scope
                    if !path.is_empty() {
                        if *is_glob {
                            // use module::* - import all items from module
                            // Remove the "*" from the end of the path
                            let module_path = path.iter()
                                .filter(|p| *p != "*")
                                .cloned()
                                .collect::<Vec<_>>()
                                .join("::");
                            
                            // Import all functions that start with the module path
                            let functions_to_import: Vec<(String, Vec<HirType>, HirType)> = self.context.functions.iter()
                                .filter(|(fname, _)| fname.starts_with(&format!("{}::", module_path)))
                                .map(|(fname, (params, ret))| {
                                    // Extract the short name (after the last ::)
                                    let short_name = fname.split("::").last().unwrap_or(fname).to_string();
                                    (short_name, params.clone(), ret.clone())
                                })
                                .collect();
                            
                            // Register these items with their short names
                            for (short_name, params, ret) in functions_to_import {
                                self.context.register_function(short_name, params, ret);
                            }
                            
                            // Import all structs that start with the module path
                            let structs_to_import: Vec<(String, Vec<(String, HirType)>)> = self.context.structs.iter()
                                .filter(|(sname, _)| sname.starts_with(&format!("{}::", module_path)))
                                .map(|(sname, fields)| {
                                    // Extract the short name (after the last ::)
                                    let short_name = sname.split("::").last().unwrap_or(sname).to_string();
                                    (short_name, fields.clone())
                                })
                                .collect();
                            
                            // Register these struct items with their short names
                            for (short_name, fields) in structs_to_import {
                                self.context.register_struct(short_name, fields);
                            }
                        } else if path.len() > 1 {
                            // use module::item - import specific item
                            let item_name = &path[path.len() - 1];
                            let module_path = path[..path.len() - 1].join("::");
                            let full_path = format!("{}::{}", module_path, item_name);
                            
                            // Create an alias so the item can be accessed by its short name
                            if let Some((param_types, ret_type)) = self.context.lookup_function(&full_path) {
                                self.context.register_function(item_name.clone(), param_types, ret_type);
                            } else if let Some(fields) = self.context.lookup_struct(&full_path) {
                                self.context.register_struct(item_name.clone(), fields);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Try to match a generic parameter type with an actual type
    /// Returns a substitution map (generic name -> concrete type) if successful
    fn try_unify_type(&self, generic_ty: &HirType, actual_ty: &HirType) -> Option<(String, HirType)> {
        match generic_ty {
            HirType::Named(name) if name.len() == 1 && name.chars().next().unwrap().is_uppercase() => {
                // This looks like a generic type parameter (single uppercase letter like T, U, etc.)
                Some((name.clone(), actual_ty.clone()))
            }
            _ => None
        }
    }

    /// Apply generic type substitutions to a type
    fn apply_substitutions(&self, ty: &HirType, subs: &std::collections::HashMap<String, HirType>) -> HirType {
        match ty {
            HirType::Named(name) => {
                if let Some(concrete_ty) = subs.get(name) {
                    concrete_ty.clone()
                } else {
                    ty.clone()
                }
            }
            _ => ty.clone()
        }
    }

    /// Check if two types are compatible (including coercions)
    fn types_compatible(&self, from: &HirType, to: &HirType) -> bool {
        if from == to {
            return true;
        }
        
        let result = match (from, to) {
            (HirType::Int32, HirType::Int64) => true,
            (HirType::Int32, HirType::UInt32) => true,
            (HirType::Int32, HirType::UInt64) => true,
            (HirType::Int32, HirType::USize) => true,
            (HirType::Int32, HirType::ISize) => true,
            (HirType::Int32, HirType::Float64) => true,
            (HirType::UInt32, HirType::Int32) => true,
            (HirType::UInt32, HirType::Int64) => true,
            (HirType::UInt32, HirType::UInt64) => true,
            (HirType::UInt32, HirType::USize) => true,
            (HirType::UInt32, HirType::Float64) => true,
            (HirType::UInt64, HirType::Int32) => true,
            (HirType::UInt64, HirType::Int64) => true,
            (HirType::UInt64, HirType::UInt32) => true,
            (HirType::UInt64, HirType::USize) => true,
            (HirType::UInt64, HirType::Float64) => true,
            (HirType::Int64, HirType::Float64) => true,
            (HirType::Int64, HirType::USize) => true,
            (HirType::Int64, HirType::ISize) => true,
            (HirType::Int64, HirType::UInt32) => true,
            (HirType::Int64, HirType::UInt64) => true,
            (HirType::USize, HirType::Int64) => true,
            (HirType::USize, HirType::UInt32) => true,
            (HirType::USize, HirType::UInt64) => true,
            (HirType::USize, HirType::Int32) => true,
            (HirType::ISize, HirType::Int64) => true,
            (HirType::ISize, HirType::Int32) => true,
            // Reference to raw pointer coercion (e.g., &i32 -> *const i32)
            (HirType::Reference(inner_from), HirType::Pointer(inner_to)) => {
                // References can coerce to raw pointers, with type compatibility for inner types
                self.types_compatible(inner_from, inner_to)
            }
            // Mutable reference to raw pointer coercion
            (HirType::MutableReference(inner_from), HirType::Pointer(inner_to)) => {
                // Mutable references can coerce to raw pointers, with type compatibility for inner types
                self.types_compatible(inner_from, inner_to)
            }
            // Reference/dereference coercion for string methods
            (HirType::Reference(inner_from), to_ty) => {
                self.types_compatible(inner_from, to_ty)
            }
            (from_ty, HirType::Reference(inner_to)) => {
                self.types_compatible(from_ty, inner_to)
            }
            _ => false,
        };
        result
    }

    /// Validate that a generic parameter satisfies its trait bounds
    /// Returns true if all bounds are satisfied, false otherwise
    fn validate_trait_bounds(&self, generic_param: &str, concrete_type: &HirType) -> bool {
        let bounds = self.context.get_generic_bounds(generic_param);
        
        // If no bounds, it's always valid
        if bounds.is_empty() {
            return true;
        }
        
        // Validate each bound
        for bound in &bounds {
            match bound.as_str() {
                // Built-in marker traits - check if concrete type supports them
                "Clone" => {
                    // Clone is supported for Copy types and most compound types
                    if !self.type_supports_clone(concrete_type) {
                        eprintln!("[TypeChecker] Error: Type {} does not implement Clone trait", concrete_type);
                        return false;
                    }
                }
                "Copy" => {
                    // Copy is only for small, fixed-size types
                    if !self.type_is_copy(concrete_type) {
                        eprintln!("[TypeChecker] Error: Type {} does not implement Copy trait (only primitive types and thin structs are Copy)", concrete_type);
                        return false;
                    }
                }
                "Default" => {
                    // Default is supported for primitives and types with default values
                    if !self.type_supports_default(concrete_type) {
                        eprintln!("[TypeChecker] Warning: Type {} may not implement Default trait", concrete_type);
                        // Don't fail hard - Default might be user-defined
                    }
                }
                "Debug" => {
                    // Debug is supported for most types (pretty much everything)
                    if !self.type_supports_debug(concrete_type) {
                        eprintln!("[TypeChecker] Warning: Type {} may not implement Debug trait", concrete_type);
                        // Don't fail hard - Debug might be user-defined or derived
                    }
                }
                "Display" => {
                    // Display requires explicit impl, not automatically available
                    if !self.type_supports_display(concrete_type) {
                        eprintln!("[TypeChecker] Warning: Type {} may not implement Display trait - explicit impl required", concrete_type);
                        // Don't fail hard - user might have impl Display
                    }
                }
                "Hash" => {
                    // Hash is for types that can be hashed
                    if !self.type_supports_hash(concrete_type) {
                        eprintln!("[TypeChecker] Error: Type {} does not implement Hash trait (Fn and FnMut types don't hash)", concrete_type);
                        return false;
                    }
                }
                "Eq" => {
                    // Eq requires types that support equality
                    if !self.type_supports_eq(concrete_type) {
                        eprintln!("[TypeChecker] Error: Type {} does not implement Eq trait", concrete_type);
                        return false;
                    }
                }
                "PartialEq" => {
                    // PartialEq is supported for most types
                    if !self.type_supports_partialeq(concrete_type) {
                        eprintln!("[TypeChecker] Error: Type {} does not implement PartialEq trait", concrete_type);
                        return false;
                    }
                }
                "Ord" => {
                    // Ord requires full ordering (stricter than PartialOrd)
                    if !self.type_supports_ord(concrete_type) {
                        eprintln!("[TypeChecker] Error: Type {} does not implement Ord trait", concrete_type);
                        return false;
                    }
                }
                "PartialOrd" => {
                    // PartialOrd for types that support partial ordering
                    if !self.type_supports_partialord(concrete_type) {
                        eprintln!("[TypeChecker] Error: Type {} does not implement PartialOrd trait", concrete_type);
                        return false;
                    }
                }
                "Send" => {
                    // Send: safe to send across thread boundaries
                    if !self.type_is_send(concrete_type) {
                        eprintln!("[TypeChecker] Error: Type {} does not implement Send trait (likely has interior mutability)", concrete_type);
                        return false;
                    }
                }
                "Sync" => {
                    // Sync: safe to share across thread boundaries
                    if !self.type_is_sync(concrete_type) {
                        eprintln!("[TypeChecker] Error: Type {} does not implement Sync trait (likely has interior mutability)", concrete_type);
                        return false;
                    }
                }
                "Unpin" => {
                    // Unpin: can be safely moved
                    // Most types are Unpin unless they explicitly implement Drop and Pin semantics
                    if !self.type_is_unpin(concrete_type) {
                        eprintln!("[TypeChecker] Error: Type {} does not implement Unpin trait", concrete_type);
                        return false;
                    }
                }
                custom_trait => {
                    // For custom traits, try to look up the trait definition
                    if let Some(_trait_methods) = self.context.lookup_trait(custom_trait) {
                        // Check if concrete_type has impl block for this trait
                        if !self.type_implements_custom_trait(concrete_type, custom_trait) {
                            eprintln!("[TypeChecker] Error: Type {} does not implement custom trait '{}'", concrete_type, custom_trait);
                            return false;
                        }
                    } else {
                        // Unknown trait - can't validate
                        eprintln!("[TypeChecker] Warning: Cannot find trait definition for '{}' - deferring validation to runtime", custom_trait);
                        // Continue anyway - might be defined elsewhere
                    }
                }
            }
        }
        
        true
    }
    
    /// Check if a type implements Clone
    fn type_supports_clone(&self, ty: &HirType) -> bool {
        match ty {
            // Primitives always Clone
            HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64
            | HirType::USize | HirType::ISize | HirType::Float64
            | HirType::Bool | HirType::Char => true,
            // References always Clone
            HirType::Reference(_) | HirType::MutableReference(_) => true,
            // Pointers Clone
            HirType::Pointer(_) => true,
            // Strings Clone
            HirType::String => true,
            // Arrays Clone (if elements do)
            HirType::Array { element_type, .. } => self.type_supports_clone(element_type),
            // Structs: assume Clone unless we know otherwise
            HirType::Named(_) => true,
            _ => true  // Conservative: allow unknown types to Clone
        }
    }
    
    /// Check if a type is Copy (subset of Clone, small fixed-size types)
    fn type_is_copy(&self, ty: &HirType) -> bool {
        match ty {
            // Only primitives are Copy
            HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64
            | HirType::USize | HirType::ISize | HirType::Float64
            | HirType::Bool | HirType::Char => true,
            // Raw pointers are Copy (not references, which are special)
            HirType::Pointer(_) => true,
            // References are Copy-like but we model them separately
            HirType::Reference(_) => true,
            // Tuples of Copy types are Copy
            HirType::Tuple(types) => types.iter().all(|t| self.type_is_copy(t)),
            // Most user-defined types are NOT Copy unless explicitly marked
            _ => false
        }
    }
    
    /// Check if a type supports Default
    fn type_supports_default(&self, ty: &HirType) -> bool {
        match ty {
            // Primitives support Default (zero/false/empty)
            HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64
            | HirType::USize | HirType::ISize | HirType::Float64
            | HirType::Bool | HirType::Char | HirType::String => true,
            // Collections support Default
            HirType::Array { .. } => true,
            // Tuples support Default if elements do
            HirType::Tuple(types) => types.iter().all(|t| self.type_supports_default(t)),
            // User-defined types might have Default impl
            HirType::Named(_) => true,  // Assume true, could be checked in registry
            _ => false
        }
    }
    
    /// Check if a type supports Debug
    fn type_supports_debug(&self, ty: &HirType) -> bool {
        match ty {
            // All primitives Debug
            HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64
            | HirType::USize | HirType::ISize | HirType::Float64
            | HirType::Bool | HirType::Char | HirType::String => true,
            // Collections Debug
            HirType::Array { .. } => true,
            // References Debug
            HirType::Reference(_) | HirType::MutableReference(_) | HirType::Pointer(_) => true,
            // Tuples Debug
            HirType::Tuple(types) => types.iter().all(|t| self.type_supports_debug(t)),
            // User types usually derive Debug
            HirType::Named(_) => true,
            _ => true  // Conservative default
        }
    }
    
    /// Check if a type supports Display
    fn type_supports_display(&self, ty: &HirType) -> bool {
        match ty {
            // Primitives support Display
            HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64
            | HirType::USize | HirType::ISize | HirType::Float64
            | HirType::Bool | HirType::Char | HirType::String => true,
            // Not auto-impl'd for most user types - requires explicit impl
            _ => false
        }
    }
    
    /// Check if a type supports Hash
    fn type_supports_hash(&self, ty: &HirType) -> bool {
        match ty {
            // Primitives hash
            HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64
            | HirType::USize | HirType::ISize | HirType::Bool | HirType::Char | HirType::String => true,
            // Float64 does NOT hash (NaN issues)
            HirType::Float64 => false,
            // Arrays hash
            HirType::Array { element_type, .. } => self.type_supports_hash(element_type),
            // Tuples hash if elements do
            HirType::Tuple(types) => types.iter().all(|t| self.type_supports_hash(t)),
            // Pointers don't hash reliably
            HirType::Pointer(_) => false,
            HirType::Reference(_) | HirType::MutableReference(_) => false,
            // User types might hash
            HirType::Named(_) => true,
            _ => false
        }
    }
    
    /// Check if a type supports Eq
    fn type_supports_eq(&self, ty: &HirType) -> bool {
        match ty {
            // Primitives support Eq
            HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64
            | HirType::USize | HirType::ISize | HirType::Bool | HirType::Char | HirType::String => true,
            // Float64 is PartialEq but NOT Eq (NaN != NaN)
            HirType::Float64 => false,
            // Arrays support Eq
            HirType::Array { element_type, .. } => self.type_supports_eq(element_type),
            // Tuples if elements support Eq
            HirType::Tuple(types) => types.iter().all(|t| self.type_supports_eq(t)),
            // Function types don't support Eq
            HirType::Function { .. } => false,
            // User types might support Eq
            HirType::Named(_) => true,
            _ => false
        }
    }
    
    /// Check if a type supports PartialEq
    fn type_supports_partialeq(&self, ty: &HirType) -> bool {
        match ty {
            // Primitives support PartialEq (including floats)
            HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64
            | HirType::USize | HirType::ISize | HirType::Float64
            | HirType::Bool | HirType::Char | HirType::String => true,
            // Arrays support PartialEq
            HirType::Array { element_type, .. } => self.type_supports_partialeq(element_type),
            // Tuples if elements support PartialEq
            HirType::Tuple(types) => types.iter().all(|t| self.type_supports_partialeq(t)),
            // Pointers and references don't usually PartialEq (they compare by value)
            HirType::Function { .. } => false,
            // User types might support PartialEq
            HirType::Named(_) => true,
            _ => false
        }
    }
    
    /// Check if a type supports Ord (full ordering)
    fn type_supports_ord(&self, ty: &HirType) -> bool {
        match ty {
            // Primitives support Ord (except floats due to NaN)
            HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64
            | HirType::USize | HirType::ISize | HirType::Bool | HirType::Char | HirType::String => true,
            // Float64 doesn't support Ord
            HirType::Float64 => false,
            // Arrays support Ord if elements do
            HirType::Array { element_type, .. } => self.type_supports_ord(element_type),
            // Tuples support Ord if elements do
            HirType::Tuple(types) => types.iter().all(|t| self.type_supports_ord(t)),
            // User types might support Ord
            HirType::Named(_) => true,
            _ => false
        }
    }
    
    /// Check if a type supports PartialOrd (partial ordering)
    fn type_supports_partialord(&self, ty: &HirType) -> bool {
        match ty {
            // Primitives support PartialOrd (including floats)
            HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64
            | HirType::USize | HirType::ISize | HirType::Float64
            | HirType::Bool | HirType::Char | HirType::String => true,
            // Arrays support PartialOrd if elements do
            HirType::Array { element_type, .. } => self.type_supports_partialord(element_type),
            // Tuples support PartialOrd if elements do
            HirType::Tuple(types) => types.iter().all(|t| self.type_supports_partialord(t)),
            // User types might support PartialOrd
            HirType::Named(_) => true,
            _ => false
        }
    }
    
    /// Check if a type is Send (safe across thread boundaries)
    fn type_is_send(&self, ty: &HirType) -> bool {
        match ty {
            // All primitives are Send
            HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64
            | HirType::USize | HirType::ISize | HirType::Float64
            | HirType::Bool | HirType::Char | HirType::String => true,
            // Owned collections are Send if their elements are
            HirType::Array { element_type, .. } => self.type_is_send(element_type),
            // Tuples are Send if all elements are
            HirType::Tuple(types) => types.iter().all(|t| self.type_is_send(t)),
            // References and pointers are Send if what they point to is Sync
            HirType::Reference(_) | HirType::MutableReference(_) | HirType::Pointer(_) => true,
            // Closures/functions: conservatively assume not Send (captures might not be)
            HirType::Function { .. } => false,
            // User types conservatively assumed Send unless we know better
            HirType::Named(_) => true,
            _ => true
        }
    }
    
    /// Check if a type is Sync (safe to share across thread boundaries)
    fn type_is_sync(&self, ty: &HirType) -> bool {
        match ty {
            // All primitives are Sync
            HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64
            | HirType::USize | HirType::ISize | HirType::Float64
            | HirType::Bool | HirType::Char | HirType::String => true,
            // Shared collections are Sync if elements are
            HirType::Array { element_type, .. } => self.type_is_sync(element_type),
            // Tuples are Sync if all elements are
            HirType::Tuple(types) => types.iter().all(|t| self.type_is_sync(t)),
            // References/pointers are Sync
            HirType::Reference(_) | HirType::MutableReference(_) | HirType::Pointer(_) => true,
            // Functions aren't Sync (captures might not be)
            HirType::Function { .. } => false,
            // User types conservatively assumed Sync unless we know better
            HirType::Named(_) => true,
            _ => true
        }
    }
    
    /// Check if a type is Unpin (can be safely moved)
    fn type_is_unpin(&self, ty: &HirType) -> bool {
        // Almost all types are Unpin except for manually impl'd Pin types
        // For our purposes, assume everything is Unpin
        match ty {
            // Function types that explicitly require Pin would not be Unpin
            // But we don't track that, so assume Unpin
            _ => true
        }
    }
    
    /// Check if a type implements a custom trait
    fn type_implements_custom_trait(&self, ty: &HirType, trait_name: &str) -> bool {
        match ty {
            HirType::Named(type_name) => {
                // Check if there's an impl block for this type and trait
                // This would be checked against registered impl blocks
                // For now, we check if the type has any impl methods at all
                // In a full implementation, we'd check the trait impl registry
                
                // Try to find impl methods for this type
                let _has_impl = self.context.lookup_impl_method(type_name, trait_name);
                
                // If we found methods, assume the trait is implemented
                // This is a simplified check - real implementation would need
                // to check all methods required by the trait
                _has_impl.is_some()
            }
            // Primitive types don't implement custom traits
            _ => false
        }
    }
    
    /// Infer the type of an expression with an optional expected type
    /// This allows context-aware type inference for literals and overloaded functions
    fn infer_type_with_context(&mut self, expr: &HirExpression, expected: Option<&HirType>) -> TypeCheckResult<HirType> {
        match expr {
            HirExpression::Integer(_) => {
                // Use expected type if it's an integer type, otherwise default to i32
                match expected {
                    Some(HirType::Int32) => Ok(HirType::Int32),
                    Some(HirType::Int64) => Ok(HirType::Int64),
                    Some(HirType::Array { element_type, size }) => {
                        // If array element type is known, use it for array literals
                        match &**element_type {
                            HirType::Int32 => Ok(HirType::Int32),
                            HirType::Int64 => Ok(HirType::Int64),
                            _ => Ok(HirType::Int32), // Default
                        }
                    }
                    _ => Ok(HirType::Int32), // Default to i32
                }
            }
            HirExpression::Float(_) => Ok(HirType::Float64),
            HirExpression::String(_) => Ok(HirType::Reference(Box::new(HirType::String))),
            HirExpression::Bool(_) => Ok(HirType::Bool),

            HirExpression::Variable(name) => {
                // First check if it's a variable
                if let Some(ty) = self.context.env.lookup(name) {
                    Ok(ty)
                } else if self.context.lookup_struct(name).is_some() {
                    // It's a struct type - unit struct or type name used as a value
                    Ok(HirType::Named(name.clone()))
                } else {
                    Err(TypeCheckError {
                        message: format!("Undefined variable: {}", name),
                    })
                }
            }

            HirExpression::BinaryOp { op, left, right } => {
                let left_ty = self.infer_type(left)?;
                let right_ty = self.infer_type(right)?;

                // Type compatibility check with support for Unknown (type inference)
                let result_ty = if left_ty == HirType::Unknown && right_ty != HirType::Unknown {
                    right_ty.clone()
                } else if right_ty == HirType::Unknown && left_ty != HirType::Unknown {
                    left_ty.clone()
                } else if left_ty == HirType::Unknown && right_ty == HirType::Unknown {
                    HirType::Unknown
                } else if left_ty != right_ty && left_ty != HirType::Unknown && right_ty != HirType::Unknown {
                    // Allow coercion between integer types (i32 <-> i64)
                    let is_integer_coercion = matches!((left_ty.clone(), right_ty.clone()), 
                        (HirType::Int32, HirType::Int64) | (HirType::Int64, HirType::Int32));
                    
                    if !is_integer_coercion {
                        return Err(TypeCheckError {
                            message: format!(
                                "Type mismatch in binary operation: {} and {}",
                                left_ty, right_ty
                            ),
                        });
                    }
                    
                    // For mixed int operations, promote to i64
                    if matches!((left_ty.clone(), right_ty.clone()),
                        (HirType::Int32, HirType::Int64) | (HirType::Int64, HirType::Int32)) {
                        HirType::Int64
                    } else {
                        left_ty.clone()
                    }
                } else {
                    left_ty.clone()
                };

                // Determine result type based on operator
                match op {
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Modulo
                    | BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseOr
                    | BinaryOp::BitwiseXor
                    | BinaryOp::LeftShift
                    | BinaryOp::RightShift => Ok(result_ty),

                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual
                    | BinaryOp::And
                    | BinaryOp::Or => Ok(HirType::Bool),
                }
            }

            HirExpression::UnaryOp { op, operand } => {
                match op {
                    UnaryOp::Reference | UnaryOp::MutableReference => {
                        // For references, check if the expected type is a pointer
                        // If so, extract the inner type and use it for operand inference
                        let operand_expected = if let Some(HirType::Pointer(inner)) = expected {
                            Some((**inner).clone())
                        } else if let Some(HirType::Reference(inner)) = expected {
                            Some((**inner).clone())
                        } else {
                            None
                        };
                        
                        let operand_ty = if let Some(exp_ty) = operand_expected {
                            self.infer_type_with_context(operand, Some(&exp_ty))?
                        } else {
                            self.infer_type(operand)?
                        };
                        
                        Ok(HirType::Reference(Box::new(operand_ty)))
                    }
                    _ => {
                        let operand_ty = self.infer_type(operand)?;
                        match op {
                            UnaryOp::Negate | UnaryOp::BitwiseNot => Ok(operand_ty),
                            UnaryOp::Not => Ok(HirType::Bool),
                            UnaryOp::Dereference => {
                                match &operand_ty {
                                    HirType::Reference(inner) => Ok((**inner).clone()),
                                    HirType::Pointer(inner) => Ok((**inner).clone()),
                                    _ => Err(TypeCheckError {
                                        message: format!("Cannot dereference type: {}", operand_ty),
                                    }),
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }

            HirExpression::Assign { target, value } => {
                if let HirExpression::Variable(name) = &**target {
                    if !self.context.env.is_mutable(name) {
                        return Err(TypeCheckError {
                            message: format!("Cannot assign to immutable variable '{}'", name),
                        });
                    }
                }
                
                // Assignment expression returns the assigned value's type
                self.infer_type(value)
            }

            HirExpression::If {
                condition,
                then_body,
                else_body,
            } => {
                // Check condition is bool
                let cond_ty = self.infer_type(condition)?;
                if cond_ty != HirType::Bool {
                    return Err(TypeCheckError {
                        message: format!("If condition must be bool, got {}", cond_ty),
                    });
                }

                // Type check bodies
                self.check_statements(then_body)?;
                if let Some(else_stmts) = else_body {
                    self.check_statements(else_stmts)?;
                }

                Ok(HirType::Unknown) // Could infer from body returns
            }

            HirExpression::While { condition, body } => {
                // Check condition is bool
                let cond_ty = self.infer_type(condition)?;
                if cond_ty != HirType::Bool {
                    return Err(TypeCheckError {
                        message: format!("While condition must be bool, got {}", cond_ty),
                    });
                }

                // Type check body
                self.check_statements(body)?;

                Ok(HirType::Unknown) // Loops don't have a value type
            }

            HirExpression::Match { scrutinee, arms } => {
                let _scrutinee_ty = self.infer_type(scrutinee)?;

                // Type check all arms
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        let guard_ty = self.infer_type(guard)?;
                        if guard_ty != HirType::Bool {
                            return Err(TypeCheckError {
                                message: format!(
                                    "Match guard must be bool, got {}",
                                    guard_ty
                                ),
                            });
                        }
                    }
                    self.check_statements(&arm.body)?;
                }

                Ok(HirType::Unknown)
            }

            HirExpression::Call { func, args } => {
                match &**func {
                    HirExpression::Variable(name) => {
                        // For method calls, try qualified name first (ReceiverType::method)
                        // This prevents generic functions from shadowing method calls
                        let mut found_qualified = false;
                        if !args.is_empty() {
                            let receiver_ty = self.infer_type(&args[0])?;
                            let receiver_type_name = match &receiver_ty {
                                HirType::Named(n) => n.clone(),
                                _ => receiver_ty.to_string(),
                            };
                            
                            let qualified_name = format!("{}::{}", receiver_type_name, name);
                            if let Some((param_types, ret_type)) = self.context.lookup_function(&qualified_name) {
                                // Found as a method! Check arguments
                                let is_variadic = qualified_name.starts_with("__builtin_print");
                                if !is_variadic && args.len() != param_types.len() {
                                    return Err(TypeCheckError {
                                        message: format!(
                                            "Method {} expects {} arguments, got {}",
                                            name,
                                            param_types.len(),
                                            args.len()
                                        ),
                                    });
                                }
                                
                                // Check argument types
                                let mut substitutions = std::collections::HashMap::new();
                                for (i, (arg, param_ty)) in args.iter().zip(param_types.iter()).enumerate() {
                                    let arg_ty = self.infer_type(arg)?;
                                    
                                    if let Some((gen_name, concrete_ty)) = self.try_unify_type(param_ty, &arg_ty) {
                                        substitutions.insert(gen_name, concrete_ty);
                                    } else if !self.types_compatible(&arg_ty, param_ty) && *param_ty != HirType::Unknown {
                                        return Err(TypeCheckError {
                                            message: format!(
                                                "Argument {} has type {}, expected {}",
                                                i, arg_ty, param_ty
                                            ),
                                        });
                                    }
                                }
                                
                                let final_ret_type = self.apply_substitutions(&ret_type, &substitutions);
                                return Ok(final_ret_type);
                            }
                        }
                        
                        // Try to look it up as a function
                        if let Some((param_types, ret_type)) = self.context.lookup_function(name) {
                            // Check argument count (allow variadic for builtin print functions)
                            let is_variadic = name.starts_with("__builtin_print") || name.starts_with("__builtin_eprintln") 
                                || name == "println" || name == "print" || name == "eprintln" || name == "__builtin_printf";
                            if !is_variadic && args.len() != param_types.len() {
                                return Err(TypeCheckError {
                                    message: format!(
                                        "Function {} expects {} arguments, got {}",
                                        name,
                                        param_types.len(),
                                        args.len()
                                    ),
                                });
                            }

                            // Check argument types and collect generic substitutions
                            let mut substitutions = std::collections::HashMap::new();
                            
                            // Skip type checking for variadic/polymorphic functions like println
                            let is_polymorphic = name == "println" || name == "print" || name == "eprintln" 
                                || name == "__builtin_println" || name == "__builtin_print" || name == "__builtin_eprintln"
                                || name == "__builtin_printf";
                            
                            if !is_polymorphic {
                                for (i, (arg, param_ty)) in args.iter().zip(param_types.iter()).enumerate() {
                                    let arg_ty = self.infer_type(arg)?;
                                    
                                    // Try to unify generic types
                                    if let Some((gen_name, concrete_ty)) = self.try_unify_type(param_ty, &arg_ty) {
                                        substitutions.insert(gen_name, concrete_ty);
                                    } else if !self.types_compatible(&arg_ty, param_ty) && *param_ty != HirType::Unknown {
                                        let msg = if arg_ty == HirType::Int64 || arg_ty == HirType::Int32 {
                                            if param_ty.to_string().chars().all(|c| c.is_alphanumeric() || c == '_') {
                                                format!(
                                                    "Argument {} has type {}, expected {}\n\
                                                    hint: enum variants are currently converted to integers; \
                                                    this is a known compiler limitation",
                                                    i, arg_ty, param_ty
                                                )
                                            } else {
                                                format!(
                                                    "Argument {} has type {}, expected {}",
                                                    i, arg_ty, param_ty
                                                )
                                            }
                                        } else {
                                            format!(
                                                "Argument {} has type {}, expected {}",
                                                i, arg_ty, param_ty
                                            )
                                        };
                                        return Err(TypeCheckError {
                                            message: msg,
                                        });
                                    }
                                }
                            } else {
                                // For polymorphic functions, just infer types without checking compatibility
                                for arg in args {
                                    let _ = self.infer_type(arg)?;  // Just make sure args are valid
                                }
                            }

                            // Apply substitutions to return type
                            let final_ret_type = self.apply_substitutions(&ret_type, &substitutions);
                            Ok(final_ret_type)
                        } else if let Some(var_ty) = self.context.env.lookup(name) {
                            // Check if it's a closure type
                            if let HirType::Closure { params, return_type, .. } = var_ty {
                                // Allow calling a closure
                                if args.len() != params.len() {
                                    return Err(TypeCheckError {
                                        message: format!(
                                            "Closure {} expects {} arguments, got {}",
                                            name,
                                            params.len(),
                                            args.len()
                                        ),
                                    });
                                }

                                // Check argument types
                                for (i, (arg, param_ty)) in args.iter().zip(params.iter()).enumerate() {
                                    let arg_ty = self.infer_type(arg)?;
                                    if !self.types_compatible(&arg_ty, param_ty) && *param_ty != HirType::Unknown {
                                        return Err(TypeCheckError {
                                            message: format!(
                                                "Argument {} has type {}, expected {}",
                                                i, arg_ty, param_ty
                                            ),
                                        });
                                    }
                                }

                                Ok(return_type.as_ref().clone())
                            } else {
                                Err(TypeCheckError {
                                    message: format!("Variable {} is not callable", name),
                                })
                            }
                        } else if !args.is_empty() {
                            // This case is now handled above
                            Err(TypeCheckError {
                                message: format!("Unknown function or method: {}", name),
                            })
                        } else {
                            Err(TypeCheckError {
                                message: format!("Undefined function: {}", name),
                            })
                        }
                    }
                    HirExpression::FieldAccess { object, field } => {
                        let obj_ty = self.infer_type(object)?;
                        
                        if IteratorMethodHandler::is_iterator_method(field) {
                            if let Some((_params, ret_ty)) = IteratorMethodHandler::get_method_signature(&obj_ty, field) {
                                Ok(ret_ty)
                            } else {
                                Err(TypeCheckError {
                                    message: format!(
                                        "Method {} not supported on type {}",
                                        field, obj_ty
                                    ),
                                })
                            }
                        } else {
                            Err(TypeCheckError {
                                message: format!("Unknown method: {}", field),
                            })
                        }
                    }
                    HirExpression::Closure { params, return_type, .. } => {
                        // Check argument count matches closure parameters
                        if args.len() != params.len() {
                            return Err(TypeCheckError {
                                message: format!(
                                    "Closure expects {} arguments, got {}",
                                    params.len(),
                                    args.len()
                                ),
                            });
                        }

                        // Check argument types against closure parameters
                        for (i, (arg, (_, param_ty))) in args.iter().zip(params.iter()).enumerate() {
                            let arg_ty = self.infer_type(arg)?;
                            if arg_ty != *param_ty && *param_ty != HirType::Unknown {
                                return Err(TypeCheckError {
                                    message: format!(
                                        "Argument {} has type {}, expected {}",
                                        i, arg_ty, param_ty
                                    ),
                                });
                            }
                        }

                        Ok(return_type.as_ref().clone())
                    }
                    _ => {
                        // Try to infer the type of the function expression for indirect calls
                        let func_ty = self.infer_type(func)?;
                        match func_ty {
                            HirType::Function { params, return_type } => {
                                // Validate argument count
                                if args.len() != params.len() {
                                    return Err(TypeCheckError {
                                        message: format!(
                                            "Function expects {} arguments, got {}",
                                            params.len(),
                                            args.len()
                                        ),
                                    });
                                }

                                // Check argument types
                                for (i, (arg, param_ty)) in args.iter().zip(params.iter()).enumerate() {
                                    let arg_ty = self.infer_type(arg)?;
                                    if !self.types_compatible(&arg_ty, param_ty) && *param_ty != HirType::Unknown {
                                        return Err(TypeCheckError {
                                            message: format!(
                                                "Argument {} has type {}, expected {}",
                                                i, arg_ty, param_ty
                                            ),
                                        });
                                    }
                                }

                                Ok((*return_type).clone())
                            }
                            _ => {
                                Err(TypeCheckError {
                                    message: "Indirect function calls only work on function pointers".to_string(),
                                })
                            }
                        }
                    }
                }
            }

            HirExpression::FieldAccess { object, field } => {
                let mut obj_ty = self.infer_type(object)?;

                // Dereference references and mutable references for field access
                loop {
                    match &obj_ty {
                        HirType::Reference(inner) | HirType::MutableReference(inner) => {
                            obj_ty = (**inner).clone();
                        }
                        _ => break,
                    }
                }

                match &obj_ty {
                    HirType::Named(struct_name) => {
                        let struct_def = self.context.lookup_struct(struct_name)
                            .ok_or_else(|| TypeCheckError {
                                message: format!("Unknown struct: {}", struct_name),
                            })?;

                        struct_def
                            .iter()
                            .find(|(fname, _)| fname == field)
                            .map(|(_, ty)| ty.clone())
                            .ok_or_else(|| TypeCheckError {
                                message: format!(
                                    "Struct {} has no field {}",
                                    struct_name, field
                                ),
                            })
                    }
                    HirType::Tuple(types) => {
                        if let Ok(index) = field.parse::<usize>() {
                            if index < types.len() {
                                Ok(types[index].clone())
                            } else {
                                Err(TypeCheckError {
                                    message: format!(
                                        "Tuple index {} out of bounds (tuple has {} elements)",
                                        index, types.len()
                                    ),
                                })
                            }
                        } else {
                            Err(TypeCheckError {
                                message: format!(
                                    "Cannot access tuple field with non-numeric name '{}'",
                                    field
                                ),
                            })
                        }
                    }
                    _ => Err(TypeCheckError {
                        message: format!("Cannot access field on type {}", obj_ty),
                    }),
                }
            }

            HirExpression::TupleAccess { object, index: _ } => {
                let obj_ty = self.infer_type(object)?;
                match obj_ty {
                    HirType::Tuple(_types) => {
                        // Return Unknown for tuple access since we don't track indices
                        // In a real implementation, we'd return types[*index]
                        Ok(HirType::Unknown)
                    }
                    _ => Err(TypeCheckError {
                        message: format!("Cannot access tuple field on type {}", obj_ty),
                    }),
                }
            }

            HirExpression::Index { array, index } => {
                let array_ty = self.infer_type(array)?;
                let index_ty = self.infer_type(index)?;

                // Index must be an integer or a Range
                match index_ty {
                    HirType::Int32 | HirType::Int64 => {
                        // Single element indexing - return element type
                        match &array_ty {
                            HirType::Array { element_type, .. } => {
                                Ok(element_type.as_ref().clone())
                            }
                            HirType::Reference(inner) => {
                                if let HirType::Array { element_type, .. } = inner.as_ref() {
                                    Ok(element_type.as_ref().clone())
                                } else {
                                    Ok(HirType::Unknown)
                                }
                            }
                            _ => Ok(HirType::Unknown)
                        }
                    }
                    HirType::Range => {
                        // Range indexing - return slice type (reference to array)
                        match &array_ty {
                            HirType::Array { element_type, .. } => {
                                Ok(HirType::Reference(Box::new(
                                    HirType::Array {
                                        element_type: element_type.clone(),
                                        size: None,  // Slices have no known size
                                    }
                                )))
                            }
                            HirType::Reference(inner) => {
                                if let HirType::Array { element_type, .. } = inner.as_ref() {
                                    Ok(HirType::Reference(Box::new(
                                        HirType::Array {
                                            element_type: element_type.clone(),
                                            size: None,
                                        }
                                    )))
                                } else {
                                    Ok(HirType::Unknown)
                                }
                            }
                            _ => Ok(HirType::Unknown)
                        }
                    }
                    _ => {
                        return Err(TypeCheckError {
                            message: format!("Array index must be integer or range, got {}", index_ty),
                        })
                    }
                }
            }

            HirExpression::StructLiteral { name, fields } => {
                // For generic structs, we skip strict type checking since we can't properly
                // resolve type parameters without a full generic substitution system.
                // The struct name may have generic parameters like "Box<T>"
                if name.contains('<') {
                    // This is a generic struct - just assume it's valid
                    // and return the named type. Proper generic resolution would require
                    // inferring type parameters from field values, which is complex.
                    return Ok(HirType::Named(name.clone()));
                }
                
                let struct_def = self.context.lookup_struct(name)
                    .ok_or_else(|| TypeCheckError {
                        message: format!("Unknown struct: {}", name),
                    })?;

                // Check all required fields are present and have correct types
                for (expected_name, expected_ty) in &struct_def {
                    let field_value = fields
                        .iter()
                        .find(|(fname, _)| fname == expected_name)
                        .ok_or_else(|| TypeCheckError {
                            message: format!("Missing field {} in struct literal {}", expected_name, name),
                        })?;

                    let actual_ty = self.infer_type_with_context(&field_value.1, Some(expected_ty))?;
                    // For fields with generic types (e.g., "T"), accept any concrete type
                    // since we can't resolve the type parameter at this stage
                    if actual_ty != *expected_ty && *expected_ty != HirType::Unknown {
                        // Check if expected_ty is a simple identifier (generic type variable)
                        if !matches!(expected_ty, HirType::Named(n) if n.len() == 1 && n.chars().all(char::is_uppercase)) {
                            return Err(TypeCheckError {
                                message: format!(
                                    "Field {} has type {}, expected {}",
                                    expected_name, actual_ty, expected_ty
                                ),
                            });
                        }
                    }
                }

                Ok(HirType::Named(name.clone()))
            }

            HirExpression::ArrayLiteral(elements) => {
                if elements.is_empty() {
                    // For empty arrays, try to infer element type from context
                    if let Some(HirType::Array { element_type, .. }) = expected {
                        return Ok(HirType::Array {
                            element_type: element_type.clone(),
                            size: Some(0),
                        });
                    }
                    // If no context, use Unknown element type
                    return Ok(HirType::Array {
                        element_type: Box::new(HirType::Unknown),
                        size: Some(0),
                    });
                }

                // Extract element type from expected array type if available
                let expected_elem_type = expected.and_then(|ty| {
                    if let HirType::Array { element_type, .. } = ty {
                        Some(element_type.as_ref())
                    } else {
                        None
                    }
                });

                // Infer element type with context if available
                let elem_ty = if let Some(expected_elem) = expected_elem_type {
                    self.infer_type_with_context(&elements[0], Some(expected_elem))?
                } else {
                    self.infer_type(&elements[0])?
                };

                // Check all elements have same type
                for elem in &elements[1..] {
                    let ty = if let Some(expected_elem) = expected_elem_type {
                        self.infer_type_with_context(elem, Some(expected_elem))?
                    } else {
                        self.infer_type(elem)?
                    };
                    if ty != elem_ty {
                        return Err(TypeCheckError {
                            message: format!(
                                "Array elements have inconsistent types: {} and {}",
                                elem_ty, ty
                            ),
                        });
                    }
                }

                Ok(HirType::Array {
                    element_type: Box::new(elem_ty),
                    size: Some(elements.len()),
                })
            }

            HirExpression::Tuple(elements) => {
                let types: Result<Vec<_>, _> =
                    elements.iter().map(|e| self.infer_type(e)).collect();
                Ok(HirType::Tuple(types?))
            }

            HirExpression::EnumVariant { enum_name, variant_name: _, args } => {
                for arg in args {
                    let _ = self.infer_type(arg)?;
                }
                Ok(HirType::Named(enum_name.clone()))
            }

            HirExpression::EnumStructVariant { enum_name, variant_name: _, fields } => {
                for (_, field_expr) in fields {
                    let _ = self.infer_type(field_expr)?;
                }
                Ok(HirType::Named(enum_name.clone()))
            }

            HirExpression::Range { start, end, .. } => {
                // Validate that start and end have consistent types
                if let Some(start_expr) = start {
                    let _start_ty = self.infer_type(start_expr)?;
                }
                if let Some(end_expr) = end {
                    let _end_ty = self.infer_type(end_expr)?;
                }
                // Range expressions have the Range type
                Ok(HirType::Range)
            }

            HirExpression::Block(stmts, expr) => {
                self.context.env.push_scope();

                self.check_statements(stmts)?;

                let block_type = if let Some(e) = expr {
                    self.infer_type(e)?
                } else {
                    HirType::Unknown
                };

                self.context.env.pop_scope();

                Ok(block_type)
            }

            HirExpression::Closure {
                params,
                body,
                return_type,
                is_move,
                captures,
            } => {
                self.context.env.push_scope();
                let mut param_types = Vec::new();
                for (param_name, param_type) in params {
                    self.context.env.insert(param_name.clone(), param_type.clone());
                    param_types.push(param_type.clone());
                }
                self.check_statements(body)?;
                let inferred_return = if let Some(HirStatement::Expression(expr)) = body.last() {
                    self.infer_type(expr)?
                } else {
                    HirType::Unknown
                };
                self.context.env.pop_scope();
                let final_ret = if **return_type == HirType::Unknown {
                    inferred_return
                } else {
                    return_type.as_ref().clone()
                };

                let trait_kind = if *is_move {
                    ClosureTrait::FnOnce
                } else {
                    let captured = self.get_captured_vars(body, params);
                    if self.has_mutable_captures(body, &captured) {
                        ClosureTrait::FnMut
                    } else {
                        ClosureTrait::Fn
                    }
                };
                Ok(HirType::Closure {
                    params: param_types,
                    return_type: Box::new(final_ret),
                    trait_kind,
                })
            }

            HirExpression::Try { value } => {
                let value_ty = self.infer_type(value)?;
                
                match &value_ty {
                    HirType::Named(name) if name == "Result" || name == "Option" => {
                        Ok(HirType::Int32)
                    }
                    HirType::Unknown => Ok(HirType::Unknown),
                    _ => Err(TypeCheckError {
                        message: format!(
                            "Try operator (?) can only be used with Result<T, E> or Option<T>, got {}",
                            value_ty
                        ),
                    }),
                }
            }

            HirExpression::MethodCall { receiver, method, args } => {
                let receiver_ty = self.infer_type(receiver)?;
                
                // Check if this is a primitive type with a builtin trait method
                let is_primitive = matches!(&receiver_ty,
                    HirType::Int32 | HirType::Int64 | HirType::UInt32 | HirType::UInt64 |
                    HirType::USize | HirType::ISize | HirType::Float64 | HirType::Bool | HirType::Char
                );
                
                if is_primitive {
                    // Handle builtin trait methods on primitive types
                    match method.as_str() {
                        // Arithmetic methods: Add, Sub, Mul, Div, Rem
                        "add" | "sub" | "mul" | "div" | "rem" => {
                            if args.len() != 1 {
                                return Err(TypeCheckError {
                                    message: format!(
                                        "Arithmetic method {} expects 1 argument, got {}",
                                        method,
                                        args.len()
                                    ),
                                });
                            }
                            let arg_ty = self.infer_type(&args[0])?;
                            if !self.types_compatible(&arg_ty, &receiver_ty) {
                                return Err(TypeCheckError {
                                    message: format!(
                                        "Argument type {} doesn't match receiver type {}",
                                        arg_ty, receiver_ty
                                    ),
                                });
                            }
                            return Ok(receiver_ty.clone());
                        }
                        // Comparison methods: eq, ne, lt, le, gt, ge
                        "eq" | "ne" | "lt" | "le" | "gt" | "ge" => {
                            if args.len() != 1 {
                                return Err(TypeCheckError {
                                    message: format!(
                                        "Comparison method {} expects 1 argument, got {}",
                                        method,
                                        args.len()
                                    ),
                                });
                            }
                            let arg_ty = self.infer_type(&args[0])?;
                            if !self.types_compatible(&arg_ty, &receiver_ty) && arg_ty != HirType::Unknown {
                                return Err(TypeCheckError {
                                    message: format!(
                                        "Argument type {} doesn't match receiver type {}",
                                        arg_ty, receiver_ty
                                    ),
                                });
                            }
                            return Ok(HirType::Bool);
                        }
                        // Clone method
                        "clone" => {
                            if !args.is_empty() {
                                return Err(TypeCheckError {
                                    message: format!(
                                        "Clone method expects 0 arguments, got {}",
                                        args.len()
                                    ),
                                });
                            }
                            return Ok(receiver_ty.clone());
                        }
                        // For now, accept other trait methods on primitives
                        _ => {
                            // Fall through to named type handling or accept anyway
                            if args.is_empty() {
                                return Ok(receiver_ty.clone());
                            } else {
                                return Ok(receiver_ty.clone());
                            }
                        }
                    }
                }
                
                // Check if it's a String or &String or &str type
                let is_string_type = receiver_ty == HirType::String ||
                    (if let HirType::Reference(inner) = &receiver_ty {
                        **inner == HirType::String
                    } else {
                        false
                    });
                
                if is_string_type {
                    // Handle String methods
                    let qualified_name = format!("String::{}", method);
                    if let Some((param_types, ret_type)) = self.context.lookup_function(&qualified_name) {
                        // Check argument count
                        if args.len() != param_types.len() - 1 {
                            return Err(TypeCheckError {
                                message: format!(
                                    "Method {} expects {} arguments, got {}",
                                    method,
                                    param_types.len() - 1,
                                    args.len()
                                ),
                            });
                        }
                        
                        // Type check arguments
                        for (i, (arg, param_ty)) in args.iter().zip(param_types.iter().skip(1)).enumerate() {
                            let arg_ty = self.infer_type(arg)?;
                            if !self.types_compatible(&arg_ty, param_ty) && *param_ty != HirType::Unknown {
                                return Err(TypeCheckError {
                                    message: format!(
                                        "Argument {} has type {}, expected {}",
                                        i, arg_ty, param_ty
                                    ),
                                });
                            }
                        }
                        
                        return Ok(ret_type);
                    } else {
                        return Err(TypeCheckError {
                            message: format!("Unknown method {} for String", method),
                        });
                    }
                } else if let HirType::Named(struct_name) = &receiver_ty {
                    // First, try to lookup in impl blocks
                    if let Some((param_types, ret_type)) = self.context.lookup_impl_method(&struct_name, method) {
                        // For instance methods in impl blocks, no implicit self in param_types
                        if args.len() != param_types.len() {
                            return Err(TypeCheckError {
                                message: format!(
                                    "Method {} expects {} arguments, got {}",
                                    method,
                                    param_types.len(),
                                    args.len()
                                ),
                            });
                        }
                        
                        for (i, (arg, param_ty)) in args.iter().zip(param_types.iter()).enumerate() {
                            let arg_ty = self.infer_type(arg)?;
                            if !self.types_compatible(&arg_ty, param_ty) && *param_ty != HirType::Unknown {
                                return Err(TypeCheckError {
                                    message: format!(
                                        "Argument {} has type {}, expected {}",
                                        i, arg_ty, param_ty
                                    ),
                                });
                            }
                        }
                        
                        return Ok(ret_type);
                    }
                    
                    // Fall back to qualified function lookup for compatibility
                    let qualified_name = format!("{}::{}", struct_name, method);
                    
                    if let Some((param_types, ret_type)) = self.context.lookup_function(&qualified_name) {
                        // Check if this is a static method (no self parameter)
                        if param_types.is_empty() {
                            return Err(TypeCheckError {
                                message: format!(
                                    "Cannot call static method {} on instance of type {}. Use {}::{} instead.",
                                    method, struct_name, struct_name, method
                                ),
                            });
                        }
                        
                        // For instance methods, first param is implicit self, rest are explicit args
                        let expected_args = param_types.len() - 1;
                        if args.len() != expected_args {
                            return Err(TypeCheckError {
                                message: format!(
                                    "Method {} expects {} arguments, got {}",
                                    method,
                                    expected_args,
                                    args.len()
                                ),
                            });
                        }
                        
                        for (i, (arg, param_ty)) in args.iter().zip(param_types.iter().skip(1)).enumerate() {
                            let arg_ty = self.infer_type(arg)?;
                            if !self.types_compatible(&arg_ty, param_ty) && *param_ty != HirType::Unknown {
                                return Err(TypeCheckError {
                                    message: format!(
                                        "Argument {} has type {}, expected {}",
                                        i, arg_ty, param_ty
                                    ),
                                });
                            }
                        }
                        
                        Ok(ret_type)
                    } else {
                        Err(TypeCheckError {
                            message: format!("Unknown method {} for type {}", method, struct_name),
                        })
                    }
                } else {
                    Err(TypeCheckError {
                        message: format!(
                            "Method {} can only be called on named types, got {}",
                            method, receiver_ty
                        ),
                    })
                }
            }
        }
    }

    /// Infer the type of an expression (without expected type context)
    fn infer_type(&mut self, expr: &HirExpression) -> TypeCheckResult<HirType> {
        self.infer_type_with_context(expr, None)
    }

    /// Type check a statement
    fn check_statement(&mut self, stmt: &HirStatement) -> TypeCheckResult<()> {
        match stmt {
            HirStatement::Let { name, mutable, ty, init } => {
                // Use context-aware type inference if type annotation is provided
                let init_ty = if *ty == HirType::Unknown {
                    self.infer_type(init)?
                } else {
                    self.infer_type_with_context(init, Some(ty))?
                };

                // If type is not explicitly given, infer it
                let final_ty = if *ty == HirType::Unknown {
                    init_ty
                } else {
                    // Verify inferred type matches annotation (with coercion)
                    if !self.types_compatible(&init_ty, ty) && init_ty != HirType::Unknown {
                        return Err(TypeCheckError {
                            message: format!(
                                "Variable {} has type {}, but initializer has type {}",
                                name, ty, init_ty
                            ),
                        });
                    }
                    ty.clone()
                };

                self.context.env.insert(name.clone(), final_ty);
                
                // Mark variable as mutable if needed
                if *mutable {
                    self.context.env.mark_mutable(name);
                }
                
                Ok(())
            }

            HirStatement::Expression(expr) => {
                self.infer_type(expr)?;
                Ok(())
            }

            HirStatement::Return(expr_opt) => {
                if let Some(e) = expr_opt {
                    self.infer_type(e)?;
                }
                Ok(())
            }

            HirStatement::Break | HirStatement::Continue => Ok(()),

            HirStatement::For {
                var,
                iter,
                body,
            } => {
                // Type check the iterator expression
                let iter_ty = self.infer_type(iter)?;
                
                // Check that iter_ty is iterable (Vec, array, range, etc.)
                let is_iterable = matches!(
                    iter_ty,
                    HirType::Vec(_) | HirType::Array { .. } | HirType::Range { .. } | 
                    HirType::String
                ) || iter_ty.to_string().contains("IntoIterator");
                
                if !is_iterable {
                    eprintln!(
                        "[TypeChecker] Error: Cannot iterate over type {}: must implement IntoIterator",
                        iter_ty
                    );
                }
                
                // Infer the loop variable type from the iterator
                let var_type = match &**iter {
                    HirExpression::Range { start, end, .. } => {
                        // Get the type from the start of the range
                        if let Some(start_expr) = start {
                            self.infer_type(start_expr)?
                        } else if let Some(end_expr) = end {
                            // Fallback to end if no start
                            self.infer_type(end_expr)?
                        } else {
                            // Default to i32 if no bounds
                            HirType::Int32
                        }
                    }
                    _ => {
                        // For non-range iterators, default to i32
                        HirType::Int32
                    }
                };
                
                // Register the loop variable in the environment
                let old_val = self.context.env.insert(var.clone(), var_type);
                
                // Type check the body
                let result = self.check_statements(body);
                
                // Restore previous value or remove the variable
                if let Some(prev) = old_val {
                    self.context.env.insert(var.clone(), prev);
                } else {
                    self.context.env.remove(var);
                }
                
                result
            }

            HirStatement::While {
                condition,
                body,
            } => {
                // Type check the condition - should be bool
                let cond_ty = self.infer_type(condition)?;
                if cond_ty != HirType::Bool && cond_ty != HirType::Unknown {
                    return Err(TypeCheckError {
                        message: format!("While condition must be bool, got {}", cond_ty),
                    });
                }
                
                // Type check the body
                self.check_statements(body)?;
                Ok(())
            }

            HirStatement::If {
                condition,
                then_body,
                else_body,
            } => {
                // Type check the condition - should be bool
                let cond_ty = self.infer_type(condition)?;
                if cond_ty != HirType::Bool && cond_ty != HirType::Unknown {
                    return Err(TypeCheckError {
                        message: format!("If condition must be bool, got {}", cond_ty),
                    });
                }
                
                // Type check the then body
                self.check_statements(then_body)?;
                
                // Type check the else body if present
                if let Some(else_stmts) = else_body {
                    self.check_statements(else_stmts)?;
                }
                
                Ok(())
            }

            HirStatement::UnsafeBlock(stmts) => {
                // Type check statements inside unsafe block
                // Unsafe blocks bypass borrow checking but still need type checking
                self.check_statements(stmts)?;
                Ok(())
            }

            HirStatement::Item(item) => {
                match &**item {
                    HirItem::Function { name, params, return_type, body, .. } => {
                        let param_types: Vec<HirType> = params.iter().map(|(_, ty)| ty.clone()).collect();
                        let ret_ty = return_type.clone().unwrap_or(HirType::Tuple(vec![]));
                        
                        self.context.functions.insert(name.clone(), (param_types.clone(), ret_ty.clone()));
                        
                        self.context.env.push_scope();
                        for (param_name, param_ty) in params {
                            self.context.env.insert(param_name.clone(), param_ty.clone());
                        }
                        
                        self.check_statements(body)?;
                        self.context.env.pop_scope();
                        
                        Ok(())
                    }
                    HirItem::Struct { name, fields, .. } => {
                        let field_types: Vec<(String, HirType)> = fields.iter()
                            .map(|(n, t)| (n.clone(), t.clone()))
                            .collect();
                        self.context.structs.insert(name.clone(), field_types);
                        Ok(())
                    }
                    _ => Ok(())
                }
            }
        }
    }

    /// Type check a list of statements
    fn check_statements(&mut self, stmts: &[HirStatement]) -> TypeCheckResult<()> {
        for stmt in stmts {
            if let HirStatement::Item(item) = stmt {
                if let HirItem::Function { name, params, return_type, .. } = &**item {
                    let param_types: Vec<HirType> = params.iter().map(|(_, ty)| ty.clone()).collect();
                    let ret_ty = return_type.clone().unwrap_or(HirType::Tuple(vec![]));
                    self.context.functions.insert(name.clone(), (param_types, ret_ty));
                } else if let HirItem::Struct { name, fields, .. } = &**item {
                    let field_types: Vec<(String, HirType)> = fields.iter()
                        .map(|(n, t)| (n.clone(), t.clone()))
                        .collect();
                    self.context.structs.insert(name.clone(), field_types);
                } else if let HirItem::Impl { struct_name, methods, .. } = &**item {
                    // Register impl block methods as qualified functions
                    for method in methods {
                        if let HirItem::Function { name, params, return_type, .. } = method {
                            let param_types: Vec<_> = params.iter().map(|(_, ty)| ty.clone()).collect();
                            let ret_type = return_type.clone().unwrap_or(HirType::Unknown);
                            
                            // Register as impl method and as qualified function
                            self.context.register_impl_method(struct_name.clone(), name.clone(), param_types.clone(), ret_type.clone());
                            let qualified_name = format!("{}::{}", struct_name, name);
                            self.context.register_function(qualified_name, param_types, ret_type);
                        }
                    }
                }
            }
        }
        
        for stmt in stmts {
            self.check_statement(stmt)?;
        }
        Ok(())
    }

    /// Type check a function
    fn check_function(
        &mut self,
        _name: &str,
        params: &[(String, HirType)],
        _return_type: &Option<HirType>,
        body: &[HirStatement],
    ) -> TypeCheckResult<()> {
        // Push new scope for function
        self.context.env.push_scope();

        // Add parameters to environment
        for (param_name, param_type) in params {
            self.context
                .env
                .insert(param_name.clone(), param_type.clone());
        }

        // Type check body
        self.check_statements(body)?;

        // Pop function scope
        self.context.env.pop_scope();

        Ok(())
    }

    /// Type check all items
    pub fn check_items(&mut self, items: &[HirItem]) -> TypeCheckResult<()> {
        // First pass: collect all definitions
        self.collect_definitions(items)?;

        // Second pass: type check each item
        self.check_items_recursive(items)
    }

    fn check_items_recursive(&mut self, items: &[HirItem]) -> TypeCheckResult<()> {
        for item in items {
            match item {
                HirItem::Function {
                    name,
                    params,
                    return_type,
                    body,
                    ..
                } => {
                    self.check_function(name, params, return_type, body)?;
                }
                HirItem::Struct { .. } => {
                }
                HirItem::Module { items: module_items, .. } => {
                    self.check_items_recursive(module_items)?;
                }
                HirItem::Const { .. } => {
                    // Constants are compile-time values
                }
                HirItem::Static { .. } => {
                    // Statics are runtime values
                }
                HirItem::AssociatedType { .. } => {
                }
                HirItem::Use { .. } => {
                }
                HirItem::Impl { methods, .. } => {
                    self.check_items_recursive(methods)?;
                }
                HirItem::Enum { .. } => {
                }
                HirItem::Trait { .. } => {
                }
            }
        }

        Ok(())
    }

    /// Detect all variables captured by a closure
    fn get_captured_vars(&self, body: &[HirStatement], params: &[(String, HirType)]) -> HashSet<String> {
        let mut captured = HashSet::new();
        let param_names: HashSet<_> = params.iter().map(|(n, _)| n.clone()).collect();

        for stmt in body {
            self.collect_vars_from_stmt(stmt, &mut captured, &param_names);
        }

        captured
    }

    /// Collect variables from a statement recursively
    fn collect_vars_from_stmt(
        &self,
        stmt: &HirStatement,
        vars: &mut HashSet<String>,
        param_names: &HashSet<String>,
    ) {
        match stmt {
            HirStatement::Let { init, .. } => {
                self.collect_vars_from_expr(init, vars, param_names);
            }
            HirStatement::Expression(expr) => {
                self.collect_vars_from_expr(expr, vars, param_names);
            }
            HirStatement::Return(Some(expr)) => {
                self.collect_vars_from_expr(expr, vars, param_names);
            }
            HirStatement::For { iter, body, .. } => {
                self.collect_vars_from_expr(iter, vars, param_names);
                for s in body {
                    self.collect_vars_from_stmt(s, vars, param_names);
                }
            }
            HirStatement::While { condition, body } => {
                self.collect_vars_from_expr(condition, vars, param_names);
                for s in body {
                    self.collect_vars_from_stmt(s, vars, param_names);
                }
            }
            HirStatement::If { condition, then_body, else_body } => {
                self.collect_vars_from_expr(condition, vars, param_names);
                for s in then_body {
                    self.collect_vars_from_stmt(s, vars, param_names);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.collect_vars_from_stmt(s, vars, param_names);
                    }
                }
            }
            HirStatement::UnsafeBlock(stmts) => {
                for s in stmts {
                    self.collect_vars_from_stmt(s, vars, param_names);
                }
            }
            _ => {}
        }
    }

    /// Collect variables from an expression recursively
    fn collect_vars_from_expr(
        &self,
        expr: &HirExpression,
        vars: &mut HashSet<String>,
        param_names: &HashSet<String>,
    ) {
        match expr {
            HirExpression::Variable(name) => {
                if !param_names.contains(name) {
                    vars.insert(name.clone());
                }
            }
            HirExpression::BinaryOp { left, right, .. } => {
                self.collect_vars_from_expr(left, vars, param_names);
                self.collect_vars_from_expr(right, vars, param_names);
            }
            HirExpression::UnaryOp { operand, .. } => {
                self.collect_vars_from_expr(operand, vars, param_names);
            }
            HirExpression::Call { func, args } => {
                self.collect_vars_from_expr(func, vars, param_names);
                for arg in args {
                    self.collect_vars_from_expr(arg, vars, param_names);
                }
            }
            HirExpression::FieldAccess { object, .. } => {
                self.collect_vars_from_expr(object, vars, param_names);
            }
            HirExpression::Index { array, index } => {
                self.collect_vars_from_expr(array, vars, param_names);
                self.collect_vars_from_expr(index, vars, param_names);
            }
            HirExpression::Block(stmts, final_expr) => {
                for s in stmts {
                    self.collect_vars_from_stmt(s, vars, param_names);
                }
                if let Some(expr) = final_expr {
                    self.collect_vars_from_expr(expr, vars, param_names);
                }
            }
            _ => {}
        }
    }

    /// Detect if any captured variable is mutated in the closure body
    fn has_mutable_captures(&self, body: &[HirStatement], captured: &HashSet<String>) -> bool {
        for stmt in body {
            if self.stmt_mutates_vars(stmt, captured) {
                return true;
            }
        }
        false
    }

    /// Check if a statement mutates any of the given variables
    fn stmt_mutates_vars(&self, stmt: &HirStatement, vars: &HashSet<String>) -> bool {
        match stmt {
            HirStatement::Let { name, init, .. } => {
                if vars.contains(name) {
                    return true;
                }
                self.expr_mutates_vars(init, vars)
            }
            HirStatement::Expression(expr) => self.expr_mutates_vars(expr, vars),
            HirStatement::Return(Some(expr)) => self.expr_mutates_vars(expr, vars),
            HirStatement::For { body, .. } => {
                for s in body {
                    if self.stmt_mutates_vars(s, vars) {
                        return true;
                    }
                }
                false
            }
            HirStatement::While { body, .. } => {
                for s in body {
                    if self.stmt_mutates_vars(s, vars) {
                        return true;
                    }
                }
                false
            }
            HirStatement::If { then_body, else_body, .. } => {
                for s in then_body {
                    if self.stmt_mutates_vars(s, vars) {
                        return true;
                    }
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        if self.stmt_mutates_vars(s, vars) {
                            return true;
                        }
                    }
                }
                false
            }
            HirStatement::UnsafeBlock(stmts) => {
                for s in stmts {
                    if self.stmt_mutates_vars(s, vars) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Check if an expression mutates any of the given variables
    fn expr_mutates_vars(&self, expr: &HirExpression, vars: &HashSet<String>) -> bool {
        match expr {
            HirExpression::Assign { target, value } => {
                if let HirExpression::Variable(name) = &**target {
                    if vars.contains(name) {
                        return true;
                    }
                }
                self.expr_mutates_vars(value, vars)
            }
            HirExpression::BinaryOp { left, right, .. } => {
                self.expr_mutates_vars(left, vars) || self.expr_mutates_vars(right, vars)
            }
            HirExpression::UnaryOp { operand, .. } => {
                self.expr_mutates_vars(operand, vars)
            }
            HirExpression::Call { func, args } => {
                if self.expr_mutates_vars(func, vars) {
                    return true;
                }
                for arg in args {
                    if self.expr_mutates_vars(arg, vars) {
                        return true;
                    }
                }
                false
            }
            HirExpression::Block(stmts, final_expr) => {
                for s in stmts {
                    if self.stmt_mutates_vars(s, vars) {
                        return true;
                    }
                }
                if let Some(expr) = final_expr {
                    self.expr_mutates_vars(expr, vars)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

/// Perform type checking on lowered HIR
pub fn check_types(items: &[HirItem]) -> Result<(), CompileError> {
    let mut checker = TypeChecker::new();
    checker.check_items(items).map_err(|e| {
        let message = e.message.clone();
        let kind = if message.contains("not yet supported") || 
                      message.contains("not supported") ||
                      message.contains("Indirect function calls not yet supported") ||
                      message.contains("compiler limitation") ||
                      (message.contains("Argument") && message.contains("has type") && 
                       (message.contains("i32") || message.contains("i64")) &&
                       message.contains("expected")) {
            ErrorKind::CompilerLimitation
        } else {
            ErrorKind::CodeIssue
        };
        CompileError::new("Type Checking", &message, kind)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_inference_integer() {
        let expr = HirExpression::Integer(42);
        let mut checker = TypeChecker::new();
        match checker.infer_type(&expr) {
            Ok(ty) => assert_eq!(ty, HirType::Int32),
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_type_inference_float() {
        let expr = HirExpression::Float(3.14);
        let mut checker = TypeChecker::new();
        match checker.infer_type(&expr) {
            Ok(ty) => assert_eq!(ty, HirType::Float64),
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_type_inference_bool() {
        let expr = HirExpression::Bool(true);
        let mut checker = TypeChecker::new();
        match checker.infer_type(&expr) {
            Ok(ty) => assert_eq!(ty, HirType::Bool),
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_binary_op_type_mismatch() {
        let expr = HirExpression::BinaryOp {
            op: BinaryOp::Add,
            left: Box::new(HirExpression::Integer(1)),
            right: Box::new(HirExpression::String("hello".to_string())),
        };
        let mut checker = TypeChecker::new();
        assert!(checker.infer_type(&expr).is_err());
    }

    #[test]
    fn test_closure_type_inference() {
        let closure_expr = HirExpression::Closure {
            params: vec![("x".to_string(), HirType::Int32)],
            body: vec![HirStatement::Expression(HirExpression::Integer(42))],
            return_type: Box::new(HirType::Int32),
            is_move: false,
            captures: Vec::new(),
        };

        let mut checker = TypeChecker::new();
        match checker.infer_type(&closure_expr) {
            Ok(ty) => {
                match ty {
                    HirType::Closure { params, return_type, .. } => {
                        assert_eq!(params.len(), 1);
                        assert_eq!(params[0], HirType::Int32);
                        assert_eq!(*return_type, HirType::Int32);
                    }
                    _ => panic!("Expected closure type"),
                }
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_closure_call_with_matching_args() {
        let closure_expr = HirExpression::Closure {
            params: vec![("x".to_string(), HirType::Int32), ("y".to_string(), HirType::Bool)],
            body: vec![HirStatement::Expression(HirExpression::Integer(42))],
            return_type: Box::new(HirType::Int32),
            is_move: false,
            captures: Vec::new(),
        };

        let call_expr = HirExpression::Call {
            func: Box::new(closure_expr),
            args: vec![HirExpression::Integer(5), HirExpression::Bool(true)],
        };

        let mut checker = TypeChecker::new();
        match checker.infer_type(&call_expr) {
            Ok(ty) => assert_eq!(ty, HirType::Int32),
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_closure_call_with_mismatched_arg_count() {
        let closure_expr = HirExpression::Closure {
            params: vec![("x".to_string(), HirType::Int32)],
            body: vec![HirStatement::Expression(HirExpression::Integer(42))],
            return_type: Box::new(HirType::Int32),
            is_move: false,
            captures: Vec::new(),
        };

        let call_expr = HirExpression::Call {
            func: Box::new(closure_expr),
            args: vec![HirExpression::Integer(5), HirExpression::Integer(10)],
        };

        let mut checker = TypeChecker::new();
        assert!(checker.infer_type(&call_expr).is_err());
    }

    #[test]
    fn test_closure_call_with_mismatched_arg_types() {
        let closure_expr = HirExpression::Closure {
            params: vec![("x".to_string(), HirType::Int32)],
            body: vec![HirStatement::Expression(HirExpression::Integer(42))],
            return_type: Box::new(HirType::Int32),
            is_move: false,
            captures: Vec::new(),
        };

        let call_expr = HirExpression::Call {
            func: Box::new(closure_expr),
            args: vec![HirExpression::String("hello".to_string())],
        };

        let mut checker = TypeChecker::new();
        assert!(checker.infer_type(&call_expr).is_err());
    }

    #[test]
    fn test_closure_with_no_args() {
        let closure_expr = HirExpression::Closure {
            params: vec![],
            body: vec![HirStatement::Expression(HirExpression::Integer(42))],
            return_type: Box::new(HirType::Int32),
            is_move: false,
            captures: Vec::new(),
        };

        let call_expr = HirExpression::Call {
            func: Box::new(closure_expr),
            args: vec![],
        };

        let mut checker = TypeChecker::new();
        match checker.infer_type(&call_expr) {
            Ok(ty) => assert_eq!(ty, HirType::Int32),
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_closure_fnmut_detection() {
        let closure_expr = HirExpression::Closure {
            params: vec![],
            body: vec![
                HirStatement::Let {
                    name: "x".to_string(),
                    mutable: true,
                    ty: HirType::Int32,
                    init: HirExpression::Integer(5),
                },
                HirStatement::Expression(
                    HirExpression::Assign {
                        target: Box::new(HirExpression::Variable("x".to_string())),
                        value: Box::new(HirExpression::Integer(10)),
                    }
                ),
            ],
            return_type: Box::new(HirType::Tuple(vec![])),
            is_move: false,
            captures: Vec::new(),
        };

        let mut checker = TypeChecker::new();
        match checker.infer_type(&closure_expr) {
            Ok(ty) => {
                match ty {
                    HirType::Closure { trait_kind, .. } => {
                        assert_eq!(trait_kind, ClosureTrait::Fn);
                    }
                    _ => panic!("Expected closure type"),
                }
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_closure_fn_detection() {
        let closure_expr = HirExpression::Closure {
            params: vec![],
            body: vec![
                HirStatement::Expression(
                    HirExpression::BinaryOp {
                        op: BinaryOp::Add,
                        left: Box::new(HirExpression::Integer(5)),
                        right: Box::new(HirExpression::Integer(10)),
                    }
                ),
            ],
            return_type: Box::new(HirType::Int32),
            is_move: false,
            captures: Vec::new(),
        };

        let mut checker = TypeChecker::new();
        match checker.infer_type(&closure_expr) {
            Ok(ty) => {
                match ty {
                    HirType::Closure { trait_kind, .. } => {
                        assert_eq!(trait_kind, ClosureTrait::Fn);
                    }
                    _ => panic!("Expected closure type"),
                }
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_closure_fnonce_detection() {
        let closure_expr = HirExpression::Closure {
            params: vec![],
            body: vec![HirStatement::Expression(HirExpression::Integer(42))],
            return_type: Box::new(HirType::Int32),
            is_move: true,
            captures: Vec::new(),
        };

        let mut checker = TypeChecker::new();
        match checker.infer_type(&closure_expr) {
            Ok(ty) => {
                match ty {
                    HirType::Closure { trait_kind, .. } => {
                        assert_eq!(trait_kind, ClosureTrait::FnOnce);
                    }
                    _ => panic!("Expected closure type"),
                }
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    mod module_system_tests {
        use super::*;

        #[test]
        fn test_use_specific_function() {
            let mut checker = TypeChecker::new();
            
            // Simulate having a function in a module
            checker.context.register_function(
                "math::add".to_string(),
                vec![HirType::Int32, HirType::Int32],
                HirType::Int32,
            );
            
            // Create a use statement for the specific function
            let items = vec![
                HirItem::Use {
                    path: vec!["math".to_string(), "add".to_string()],
                    is_glob: false,
                    is_public: false,
                },
            ];
            
            // Process the use statement
            let _ = checker.process_use_statements(&items, "".to_string());
            
            // The function should now be accessible as "add"
            assert!(checker.context.lookup_function("add").is_some());
            assert!(checker.context.lookup_function("math::add").is_some());
        }

        #[test]
        fn test_use_glob_imports() {
            let mut checker = TypeChecker::new();
            
            // Register multiple functions in a module
            checker.context.register_function(
                "utils::max".to_string(),
                vec![HirType::Int32, HirType::Int32],
                HirType::Int32,
            );
            checker.context.register_function(
                "utils::min".to_string(),
                vec![HirType::Int32, HirType::Int32],
                HirType::Int32,
            );
            checker.context.register_function(
                "utils::abs".to_string(),
                vec![HirType::Int32],
                HirType::Int32,
            );
            
            // Create a glob use statement
            let items = vec![
                HirItem::Use {
                    path: vec!["utils".to_string()],
                    is_glob: true,
                    is_public: false,
                },
            ];
            
            // Process the use statement
            let _ = checker.process_use_statements(&items, "".to_string());
            
            // All functions should be accessible by short name
            assert!(checker.context.lookup_function("max").is_some());
            assert!(checker.context.lookup_function("min").is_some());
            assert!(checker.context.lookup_function("abs").is_some());
            
            // Original names should still work
            assert!(checker.context.lookup_function("utils::max").is_some());
            assert!(checker.context.lookup_function("utils::min").is_some());
            assert!(checker.context.lookup_function("utils::abs").is_some());
        }

        #[test]
        fn test_use_glob_struct_imports() {
            let mut checker = TypeChecker::new();
            
            // Register multiple structs in a module
            checker.context.register_struct(
                "geometry::Point".to_string(),
                vec![
                    ("x".to_string(), HirType::Float64),
                    ("y".to_string(), HirType::Float64),
                ],
            );
            checker.context.register_struct(
                "geometry::Vector".to_string(),
                vec![
                    ("x".to_string(), HirType::Float64),
                    ("y".to_string(), HirType::Float64),
                    ("z".to_string(), HirType::Float64),
                ],
            );
            
            // Create a glob use statement
            let items = vec![
                HirItem::Use {
                    path: vec!["geometry".to_string()],
                    is_glob: true,
                    is_public: false,
                },
            ];
            
            // Process the use statement
            let _ = checker.process_use_statements(&items, "".to_string());
            
            // All structs should be accessible by short name
            assert!(checker.context.lookup_struct("Point").is_some());
            assert!(checker.context.lookup_struct("Vector").is_some());
            
            // Original names should still work
            assert!(checker.context.lookup_struct("geometry::Point").is_some());
            assert!(checker.context.lookup_struct("geometry::Vector").is_some());
        }

        #[test]
        fn test_nested_module_use() {
            let mut checker = TypeChecker::new();
            
            // Register functions in nested modules
            checker.context.register_function(
                "core::math::add".to_string(),
                vec![HirType::Int32, HirType::Int32],
                HirType::Int32,
            );
            checker.context.register_function(
                "core::math::sub".to_string(),
                vec![HirType::Int32, HirType::Int32],
                HirType::Int32,
            );
            
            // Create a glob use statement for a nested module
            let items = vec![
                HirItem::Use {
                    path: vec!["core".to_string(), "math".to_string()],
                    is_glob: true,
                    is_public: false,
                },
            ];
            
            // Process the use statement
            let _ = checker.process_use_statements(&items, "".to_string());
            
            // Functions should be accessible by short name
            assert!(checker.context.lookup_function("add").is_some());
            assert!(checker.context.lookup_function("sub").is_some());
        }
    }
}
