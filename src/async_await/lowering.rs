//! Async function lowering to state machines
//!
//! Converts async functions and await expressions into state machines
//! that implement the Future trait.

use std::collections::{HashMap, HashSet};
use crate::parser::{self, Expression, Statement, Block, Item, MatchArm};
use crate::lowering::{HirType, HirExpression, HirStatement, HirItem, LowerError, LowerResult};

/// State machine representation for an async function
#[derive(Debug, Clone)]
pub struct AsyncStateMachine {
    /// Original function name
    pub name: String,
    
    /// Function parameters (become struct fields)
    pub params: Vec<(String, HirType)>,
    
    /// Return type of the async function
    pub return_type: HirType,
    
    /// Local variables declared in the function
    pub local_vars: Vec<(String, HirType)>,
    
    /// All await points identified
    pub await_points: Vec<AwaitPoint>,
    
    /// Code for each state
    pub state_blocks: HashMap<StateId, Vec<HirStatement>>,
    
    /// Original function item
    pub original_item: Item,
}

/// Information about an await point
#[derive(Debug, Clone)]
pub struct AwaitPoint {
    /// Sequential ID of this await point
    pub id: usize,
    
    /// The expression being awaited
    pub expr: Expression,
    
    /// Variable to store the result (if assigned)
    pub result_var: Option<String>,
}

/// State identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateId(pub usize);

impl StateId {
    pub fn start() -> Self {
        StateId(0)
    }

    pub fn next(&self) -> Self {
        StateId(self.0 + 1)
    }

    pub fn is_start(&self) -> bool {
        self.0 == 0
    }
}

/// Analyze an async function to find await points
pub fn analyze_await_points(
    body: &Block,
) -> Result<Vec<AwaitPoint>, LowerError> {
    let mut await_points = Vec::new();
    let mut next_id = 0;

    // Walk the AST to find all await expressions
    walk_block(body, &mut await_points, &mut next_id)?;

    Ok(await_points)
}

/// Recursively walk statements to find await expressions
fn walk_block(
    block: &Block,
    await_points: &mut Vec<AwaitPoint>,
    next_id: &mut usize,
) -> LowerResult<()> {
    for stmt in &block.statements {
        walk_statement(stmt, await_points, next_id)?;
    }

    Ok(())
}

/// Walk a single statement
fn walk_statement(
    stmt: &Statement,
    await_points: &mut Vec<AwaitPoint>,
    next_id: &mut usize,
) -> LowerResult<()> {
    match stmt {
        Statement::Let { initializer, .. } => {
            walk_expression(initializer, await_points, next_id)?;
        }
        Statement::Expression(expr) => {
            walk_expression(expr, await_points, next_id)?;
        }
        Statement::Return(Some(expr)) => {
            walk_expression(expr, await_points, next_id)?;
        }
        Statement::If { condition, then_body, else_body } => {
            walk_expression(condition, await_points, next_id)?;
            walk_block(then_body, await_points, next_id)?;
            if let Some(else_stmt) = else_body {
                walk_statement(else_stmt, await_points, next_id)?;
            }
        }
        Statement::While { condition, body } => {
            walk_expression(condition, await_points, next_id)?;
            walk_block(body, await_points, next_id)?;
        }
        Statement::For { iter, body, .. } => {
            walk_expression(iter, await_points, next_id)?;
            walk_block(body, await_points, next_id)?;
        }
        _ => {}
    }

    Ok(())
}

/// Walk an expression to find await points
fn walk_expression(
    expr: &Expression,
    await_points: &mut Vec<AwaitPoint>,
    next_id: &mut usize,
) -> LowerResult<()> {
    match expr {
        Expression::Await { value } => {
            // Found an await point
            let id = *next_id;
            *next_id += 1;

            await_points.push(AwaitPoint {
                id,
                expr: (**value).clone(),
                result_var: None,
            });

            // Recursively walk the awaited expression
            walk_expression(value, await_points, next_id)?;
        }
        Expression::AsyncBlock(_) => {
            // Async blocks are futures themselves, but don't need lowering
            // They'll be handled at codegen time
        }
        Expression::Binary { left, right, .. } => {
            walk_expression(left, await_points, next_id)?;
            walk_expression(right, await_points, next_id)?;
        }
        Expression::Unary { operand, .. } => {
            walk_expression(operand, await_points, next_id)?;
        }
        Expression::FunctionCall { args, .. } => {
            for arg in args {
                walk_expression(arg, await_points, next_id)?;
            }
        }
        Expression::MethodCall { receiver, args, .. } => {
            walk_expression(receiver, await_points, next_id)?;
            for arg in args {
                walk_expression(arg, await_points, next_id)?;
            }
        }
        Expression::FieldAccess { object, .. } => {
            walk_expression(object, await_points, next_id)?;
        }
        Expression::Index { array, index } => {
            walk_expression(array, await_points, next_id)?;
            walk_expression(index, await_points, next_id)?;
        }
        Expression::If { condition, then_body, else_body } => {
            walk_expression(condition, await_points, next_id)?;
            walk_block(then_body, await_points, next_id)?;
            if let Some(else_e) = else_body {
                walk_expression(else_e, await_points, next_id)?;
            }
        }
        Expression::Match { scrutinee, arms } => {
            walk_expression(scrutinee, await_points, next_id)?;
            for arm in arms {
                walk_expression(&arm.body, await_points, next_id)?;
            }
        }
        Expression::Block(block) => {
            walk_block(block, await_points, next_id)?;
        }
        Expression::Assign { value, .. } => {
            walk_expression(value, await_points, next_id)?;
        }
        Expression::CompoundAssign { value, .. } => {
            walk_expression(value, await_points, next_id)?;
        }
        _ => {}
    }

    Ok(())
}

/// Extract local variables from an async function
pub fn extract_local_variables(
    body: &Block,
    param_names: &HashSet<String>,
) -> Result<Vec<(String, HirType)>, LowerError> {
    let mut locals = Vec::new();
    let mut seen = HashSet::new();

    // Walk the AST to find all let bindings
    walk_block_for_locals(body, &mut locals, &mut seen, param_names)?;

    Ok(locals)
}

/// Walk block to collect local variables
fn walk_block_for_locals(
    block: &Block,
    locals: &mut Vec<(String, HirType)>,
    seen: &mut HashSet<String>,
    param_names: &HashSet<String>,
) -> LowerResult<()> {
    for stmt in &block.statements {
        walk_statement_for_locals(stmt, locals, seen, param_names)?;
    }

    Ok(())
}

/// Walk statement to collect local variables
fn walk_statement_for_locals(
    stmt: &Statement,
    locals: &mut Vec<(String, HirType)>,
    seen: &mut HashSet<String>,
    param_names: &HashSet<String>,
) -> LowerResult<()> {
    match stmt {
        Statement::Let { name, ty, initializer, .. } => {
            // Don't add parameters as local variables
            if param_names.contains(name) {
                return Ok(());
            }

            // Add if not already seen (handle shadowing)
            if !seen.contains(name) {
                seen.insert(name.clone());

                let hir_type = if let Some(t) = ty {
                    convert_type(t)?
                } else {
                    // If no explicit type, we'd need to infer it from initializer
                    // For now, use a placeholder
                    HirType::Unknown
                };

                locals.push((name.clone(), hir_type));
            }

            // Walk the initializer expression
            walk_expression_for_locals(initializer, locals, seen, param_names)?;
        }
        Statement::If { then_body, else_body, .. } => {
            walk_block_for_locals(then_body, locals, seen, param_names)?;
            if let Some(else_stmt) = else_body {
                walk_statement_for_locals(else_stmt, locals, seen, param_names)?;
            }
        }
        Statement::While { body, .. } => {
            walk_block_for_locals(body, locals, seen, param_names)?;
        }
        Statement::For { body, .. } => {
            walk_block_for_locals(body, locals, seen, param_names)?;
        }
        _ => {}
    }

    Ok(())
}

/// Walk expression to collect local variables
fn walk_expression_for_locals(
    expr: &Expression,
    locals: &mut Vec<(String, HirType)>,
    seen: &mut HashSet<String>,
    param_names: &HashSet<String>,
) -> LowerResult<()> {
    match expr {
        Expression::Binary { left, right, .. } => {
            walk_expression_for_locals(left, locals, seen, param_names)?;
            walk_expression_for_locals(right, locals, seen, param_names)?;
        }
        Expression::Unary { operand, .. } => {
            walk_expression_for_locals(operand, locals, seen, param_names)?;
        }
        Expression::FunctionCall { args, .. } => {
            for arg in args {
                walk_expression_for_locals(arg, locals, seen, param_names)?;
            }
        }
        Expression::MethodCall { receiver, args, .. } => {
            walk_expression_for_locals(receiver, locals, seen, param_names)?;
            for arg in args {
                walk_expression_for_locals(arg, locals, seen, param_names)?;
            }
        }
        Expression::FieldAccess { object, .. } => {
            walk_expression_for_locals(object, locals, seen, param_names)?;
        }
        Expression::Block(block) => {
            walk_block_for_locals(block, locals, seen, param_names)?;
        }
        Expression::If { then_body, else_body, .. } => {
            walk_block_for_locals(then_body, locals, seen, param_names)?;
            if let Some(else_e) = else_body {
                walk_expression_for_locals(else_e, locals, seen, param_names)?;
            }
        }
        Expression::Match { arms, .. } => {
            for arm in arms {
                walk_expression_for_locals(&arm.body, locals, seen, param_names)?;
            }
        }
        Expression::Await { value } => {
            walk_expression_for_locals(value, locals, seen, param_names)?;
        }
        _ => {}
    }

    Ok(())
}

/// Convert parser Type to HirType
fn convert_type(typ: &parser::Type) -> LowerResult<HirType> {
    match typ {
        parser::Type::Named(name) => {
            // Try to match common Rust types
            match name.as_str() {
                "i32" => Ok(HirType::Int32),
                "i64" => Ok(HirType::Int64),
                "u32" => Ok(HirType::UInt32),
                "u64" => Ok(HirType::UInt64),
                "usize" => Ok(HirType::USize),
                "isize" => Ok(HirType::ISize),
                "f64" => Ok(HirType::Float64),
                "bool" => Ok(HirType::Bool),
                "char" => Ok(HirType::Char),
                "String" | "str" => Ok(HirType::String),
                _ => Ok(HirType::Named(name.clone())),
            }
        }
        parser::Type::Array { element, size } => {
            let elem_hir = convert_type(element)?;
            // For now, we don't handle dynamic sizes in lowering
            Ok(HirType::Array {
                element_type: Box::new(elem_hir),
                size: None,
            })
        }
        parser::Type::Reference { mutable, inner, .. } => {
            let inner_hir = convert_type(inner)?;
            if *mutable {
                Ok(HirType::MutableReference(Box::new(inner_hir)))
            } else {
                Ok(HirType::Reference(Box::new(inner_hir)))
            }
        }
        parser::Type::Pointer { mutable: _mutable, inner } => {
            let inner_hir = convert_type(inner)?;
            Ok(HirType::Pointer(Box::new(inner_hir)))
        }
        parser::Type::Generic { name, type_args } => {
            // Handle generic types like Vec<T>, Option<T>, etc.
            match name.as_str() {
                "Vec" if type_args.len() == 1 => {
                    let elem = convert_type(&type_args[0])?;
                    Ok(HirType::Vec(Box::new(elem)))
                }
                "Option" if type_args.len() == 1 => {
                    let elem = convert_type(&type_args[0])?;
                    Ok(HirType::Option(Box::new(elem)))
                }
                "Result" if type_args.len() == 2 => {
                    let ok_type = convert_type(&type_args[0])?;
                    let err_type = convert_type(&type_args[1])?;
                    Ok(HirType::Result {
                        ok_type: Box::new(ok_type),
                        err_type: Box::new(err_type),
                    })
                }
                "Box" if type_args.len() == 1 => {
                    let elem = convert_type(&type_args[0])?;
                    Ok(HirType::Box(Box::new(elem)))
                }
                _ => Ok(HirType::Named(name.clone())),
            }
        }
        parser::Type::Tuple(types) => {
            let elem_types: Result<Vec<_>, _> = types
                .iter()
                .map(|t| convert_type(t))
                .collect();
            Ok(HirType::Tuple(elem_types?))
        }
        _ => {
            Ok(HirType::Unknown)
        }
    }
}

/// Create an async state machine from an async function
pub fn create_state_machine(
    name: String,
    params: Vec<(String, parser::Type)>,
    return_type: parser::Type,
    body: &Block,
    original_item: Item,
) -> LowerResult<AsyncStateMachine> {
    // Convert parameter types
    let mut hir_params = Vec::new();
    let mut param_names = HashSet::new();

    for (param_name, param_type) in params {
        param_names.insert(param_name.clone());
        let hir_type = convert_type(&param_type)?;
        hir_params.push((param_name, hir_type));
    }

    // Convert return type
    let hir_return_type = convert_type(&return_type)?;

    // Find await points
    let await_points = analyze_await_points(body)?;

    // Extract local variables
    let local_vars = extract_local_variables(body, &param_names)?;

    // Build state blocks (for now, empty - will be filled in code generation)
    let state_blocks = HashMap::new();

    Ok(AsyncStateMachine {
        name,
        params: hir_params,
        return_type: hir_return_type,
        local_vars,
        await_points,
        state_blocks,
        original_item,
    })
}
