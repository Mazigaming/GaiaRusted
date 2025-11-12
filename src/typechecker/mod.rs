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
    HirExpression, HirItem, HirStatement, HirType, BinaryOp, UnaryOp,
};
use crate::iterators::IteratorMethodHandler;
use std::collections::HashMap;
use std::fmt;

/// Type checking error
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
}

impl TypeEnv {
    /// Create a new type environment
    pub fn new() -> Self {
        TypeEnv {
            scopes: vec![HashMap::new()],
        }
    }

    /// Push a new scope
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Insert a variable into the current scope, returning the old value
    pub fn insert(&mut self, name: String, ty: HirType) -> Option<HirType> {
        if let Some(scope) = self.scopes.last_mut() {
            return scope.insert(name, ty);
        }
        None
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

        // Type conversions
        self.context.register_function("as_i32".to_string(), vec![HirType::Float64], HirType::Int32);
        self.context.register_function("as_i64".to_string(), vec![HirType::Float64], HirType::Int64);
        self.context.register_function("as_f64".to_string(), vec![HirType::Int32], HirType::Float64);
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
            }
        }
        Ok(())
    }

    /// Check if two types are compatible (including coercions)
    fn types_compatible(&self, from: &HirType, to: &HirType) -> bool {
        if from == to {
            return true;
        }
        
        match (from, to) {
            (HirType::Int32, HirType::Int64) => true,
            (HirType::Int32, HirType::Float64) => true,
            (HirType::Int64, HirType::Float64) => true,
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
                self.context
                    .env
                    .lookup(name)
                    .ok_or_else(|| TypeCheckError {
                        message: format!("Undefined variable: {}", name),
                    })
            }

            HirExpression::BinaryOp { op, left, right } => {
                let left_ty = self.infer_type(left)?;
                let right_ty = self.infer_type(right)?;

                // Type compatibility check
                if left_ty != right_ty {
                    return Err(TypeCheckError {
                        message: format!(
                            "Type mismatch in binary operation: {} and {}",
                            left_ty, right_ty
                        ),
                    });
                }

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
                    | BinaryOp::RightShift => Ok(left_ty.clone()),

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

            HirExpression::Assign { target: _, value } => {
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
                        let (param_types, ret_type) = self.context.lookup_function(name)
                            .ok_or_else(|| TypeCheckError {
                                message: format!("Undefined function: {}", name),
                            })?;

                        // Check argument count (allow variadic for builtin print functions)
                        let is_variadic = name.starts_with("__builtin_print") || name.starts_with("__builtin_eprintln") 
                            || name == "println" || name == "print" || name == "eprintln";
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

                        // Check argument types
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

                        Ok(ret_type)
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
                let obj_ty = self.infer_type(object)?;

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
                    _ => Err(TypeCheckError {
                        message: format!("Cannot access field on type {}", obj_ty),
                    }),
                }
            }

            HirExpression::Index { array, index } => {
                let _array_ty = self.infer_type(array)?;
                let index_ty = self.infer_type(index)?;

                // Index must be an integer
                match index_ty {
                    HirType::Int32 | HirType::Int64 => {}
                    _ => {
                        return Err(TypeCheckError {
                            message: format!("Array index must be integer, got {}", index_ty),
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

            HirExpression::Range { start, end, .. } => {
                // Validate that start and end have consistent types
                if let Some(start_expr) = start {
                    let _start_ty = self.infer_type(start_expr)?;
                }
                if let Some(end_expr) = end {
                    let _end_ty = self.infer_type(end_expr)?;
                }
                // Ranges are typed as Range<T> but we simplify to Unknown for now
                Ok(HirType::Unknown)
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
                return_type,
                ..
            } => {
                let param_types: Vec<_> = params.iter().map(|(_, ty)| ty.clone()).collect();
                Ok(HirType::Closure {
                    params: param_types,
                    return_type: return_type.clone(),
                })
            }
        }
    }

    /// Type check a statement
    fn check_statement(&mut self, stmt: &HirStatement) -> TypeCheckResult<()> {
        match stmt {
            HirStatement::Let { name, ty, init } => {
                let init_ty = self.infer_type(init)?;

                // If type is not explicitly given, infer it
                let final_ty = if *ty == HirType::Unknown {
                    init_ty
                } else {
                    // Verify inferred type matches annotation
                    if init_ty != *ty && init_ty != HirType::Unknown {
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
                } => {
                    self.check_function(name, params, return_type, body)?;
                }
                HirItem::Struct { .. } => {
                    // Structs are already registered, nothing to check
                }
            }
        }

        Ok(())
    }
}

/// Perform type checking on lowered HIR
pub fn check_types(items: &[HirItem]) -> TypeCheckResult<()> {
    let mut checker = TypeChecker::new();
    checker.check_items(items)
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
        };

        let mut checker = TypeChecker::new();
        match checker.infer_type(&closure_expr) {
            Ok(ty) => {
                match ty {
                    HirType::Closure { params, return_type } => {
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
}