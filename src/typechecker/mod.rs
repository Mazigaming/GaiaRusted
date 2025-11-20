//! # Phase 4: TYPE CHECKING & INFERENCE
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
}

impl TypeContext {
    /// Create a new type context
    pub fn new() -> Self {
        TypeContext {
            env: TypeEnv::new(),
            functions: HashMap::new(),
            structs: HashMap::new(),
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
}

/// Type checking and inference
pub struct TypeChecker {
    context: TypeContext,
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
    }

    /// Collect all type definitions (first pass)
    fn collect_definitions(&mut self, items: &[HirItem]) -> TypeCheckResult<()> {
        for item in items {
            match item {
                HirItem::Function {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let param_types: Vec<_> = params.iter().map(|(_, ty)| ty.clone()).collect();
                    let ret_type = return_type.clone().unwrap_or(HirType::Unknown);
                    self.context
                        .register_function(name.clone(), param_types, ret_type);
                }
                HirItem::Struct { name, fields } => {
                    self.context
                        .register_struct(name.clone(), fields.clone());
                }
                HirItem::AssociatedType { .. } => {
                }
                HirItem::Use { .. } => {
                }
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
        
        match (from, to) {
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
            // Reference/dereference coercion for string methods
            (HirType::Reference(inner_from), to_ty) => {
                self.types_compatible(inner_from, to_ty)
            }
            (from_ty, HirType::Reference(inner_to)) => {
                self.types_compatible(from_ty, inner_to)
            }
            _ => false,
        }
    }
    
    /// Infer the type of an expression
    fn infer_type(&mut self, expr: &HirExpression) -> TypeCheckResult<HirType> {
        match expr {
            HirExpression::Integer(_) => Ok(HirType::Int32), // Default to i32
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
                } else if left_ty != right_ty && left_ty != HirType::Unknown && right_ty != HirType::Unknown {
                    return Err(TypeCheckError {
                        message: format!(
                            "Type mismatch in binary operation: {} and {}",
                            left_ty, right_ty
                        ),
                    });
                } else if left_ty == HirType::Unknown && right_ty == HirType::Unknown {
                    HirType::Unknown
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
                let operand_ty = self.infer_type(operand)?;

                match op {
                    UnaryOp::Negate | UnaryOp::BitwiseNot => Ok(operand_ty),
                    UnaryOp::Not => Ok(HirType::Bool),
                    UnaryOp::Reference | UnaryOp::MutableReference => Ok(HirType::Reference(Box::new(operand_ty))),
                    UnaryOp::Dereference => {
                        match &operand_ty {
                            HirType::Reference(inner) => Ok((**inner).clone()),
                            HirType::Pointer(inner) => Ok((**inner).clone()),
                            _ => Err(TypeCheckError {
                                message: format!("Cannot dereference type: {}", operand_ty),
                            }),
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
                        // First try to look it up as a function
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
                        Err(TypeCheckError {
                            message: "Indirect function calls not yet supported".to_string(),
                        })
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
                let _array_ty = self.infer_type(array)?;
                let index_ty = self.infer_type(index)?;

                // Index must be an integer or a Range
                match index_ty {
                    HirType::Int32 | HirType::Int64 => {}
                    HirType::Range => {}  // Ranges are now valid for slicing
                    _ => {
                        return Err(TypeCheckError {
                            message: format!("Array index must be integer or range, got {}", index_ty),
                        })
                    }
                }

                // For now, just return Unknown (could infer from array type)
                Ok(HirType::Unknown)
            }

            HirExpression::StructLiteral { name, fields } => {
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

                    let actual_ty = self.infer_type(&field_value.1)?;
                    if actual_ty != *expected_ty && *expected_ty != HirType::Unknown {
                        return Err(TypeCheckError {
                            message: format!(
                                "Field {} has type {}, expected {}",
                                expected_name, actual_ty, expected_ty
                            ),
                        });
                    }
                }

                Ok(HirType::Named(name.clone()))
            }

            HirExpression::ArrayLiteral(elements) => {
                if elements.is_empty() {
                    return Ok(HirType::Array {
                        element_type: Box::new(HirType::Unknown),
                        size: Some(0),
                    });
                }

                let elem_ty = self.infer_type(&elements[0])?;

                // Check all elements have same type
                for elem in &elements[1..] {
                    let ty = self.infer_type(elem)?;
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

            HirExpression::EnumVariant { enum_name, variant_name: _ } => {
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
        }
    }

    /// Type check a statement
    fn check_statement(&mut self, stmt: &HirStatement) -> TypeCheckResult<()> {
        match stmt {
            HirStatement::Let { name, mutable, ty, init } => {
                let init_ty = self.infer_type(init)?;

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
                let _iter_ty = self.infer_type(iter)?;
                // TODO: Check that iter_ty is iterable
                
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
                HirItem::AssociatedType { .. } => {
                }
                HirItem::Use { .. } => {
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
}
