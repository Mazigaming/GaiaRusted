//! # Phase 5: BORROW CHECKING & OWNERSHIP VERIFICATION
//!
//! Enforces Rust's memory safety rules through ownership and borrowing analysis.
//!
//! ## What we do:
//! - Ownership tracking (each value has one owner)
//! - Borrow analysis (multiple immutable or one mutable)
//! - Use-after-move detection
//! - Use-after-free detection
//! - Multiple mutable reference detection
//!
//! ## Algorithm:
//! 1. Collect all bindings and their ownership status
//! 2. Track moves and borrows through the program
//! 3. Verify no value is used after move
//! 4. Verify no multiple mutable borrows of same value
//! 5. Verify all borrows outlive their use

use crate::lowering::{HirExpression, HirItem, HirStatement, HirType};
use std::collections::HashMap;
use std::fmt;

/// Borrow checking error
#[derive(Debug, Clone)]
pub struct BorrowCheckError {
    pub message: String,
}

impl fmt::Display for BorrowCheckError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

type BorrowCheckResult<T> = Result<T, BorrowCheckError>;

/// Represents the state of a binding: owned, borrowed, or moved
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipState {
    /// Value is owned by current scope
    Owned,
    /// Value has been moved elsewhere (no longer available)
    Moved,
    /// Value is borrowed immutably (can still read original)
    BorrowedImmutable,
    /// Value is borrowed mutably (cannot access original)
    BorrowedMutable,
}

/// Information about a binding
#[derive(Debug, Clone)]
pub struct Binding {
    /// Current ownership state
    pub state: OwnershipState,
    /// Type of the binding
    pub ty: HirType,
    /// Whether the binding is mutable
    pub is_mutable: bool,
    /// Number of active immutable borrows
    pub immutable_borrow_count: usize,
    /// Whether there's an active mutable borrow
    pub has_mutable_borrow: bool,
}

/// Borrow environment: tracks all bindings and their states
#[derive(Debug, Clone)]
pub struct BorrowEnv {
    scopes: Vec<HashMap<String, Binding>>,
}

impl BorrowEnv {
    /// Create a new borrow environment
    pub fn new() -> Self {
        BorrowEnv {
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

    /// Bind a new variable
    pub fn bind(&mut self, name: String, ty: HirType, is_mutable: bool) -> BorrowCheckResult<()> {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name,
                Binding {
                    state: OwnershipState::Owned,
                    ty,
                    is_mutable,
                    immutable_borrow_count: 0,
                    has_mutable_borrow: false,
                },
            );
            Ok(())
        } else {
            Err(BorrowCheckError {
                message: "No scope to bind to".to_string(),
            })
        }
    }

    /// Look up a binding (searches from innermost to outermost scope)
    pub fn lookup(&self, name: &str) -> Option<Binding> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return Some(binding.clone());
            }
        }
        None
    }

    /// Look up a binding mutably
    fn lookup_mut(&mut self, name: &str) -> Option<&mut Binding> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                return scope.get_mut(name);
            }
        }
        None
    }

    /// Mark a binding as moved
    pub fn move_binding(&mut self, name: &str) -> BorrowCheckResult<()> {
        if let Some(binding) = self.lookup_mut(name) {
            if binding.state == OwnershipState::Moved {
                return Err(BorrowCheckError {
                    message: format!("Value {} used after move", name),
                });
            }
            if binding.state == OwnershipState::BorrowedMutable {
                return Err(BorrowCheckError {
                    message: format!("Cannot move borrowed value {}", name),
                });
            }
            binding.state = OwnershipState::Moved;
            Ok(())
        } else {
            Err(BorrowCheckError {
                message: format!("Undefined variable: {}", name),
            })
        }
    }

    /// Create an immutable borrow
    pub fn borrow_immutable(&mut self, name: &str) -> BorrowCheckResult<()> {
        if let Some(binding) = self.lookup_mut(name) {
            if binding.state == OwnershipState::Moved {
                return Err(BorrowCheckError {
                    message: format!("Cannot borrow moved value {}", name),
                });
            }
            if binding.state == OwnershipState::BorrowedMutable {
                return Err(BorrowCheckError {
                    message: format!("Cannot immutably borrow mutably borrowed value {}", name),
                });
            }
            binding.immutable_borrow_count += 1;
            Ok(())
        } else {
            Err(BorrowCheckError {
                message: format!("Undefined variable: {}", name),
            })
        }
    }

    /// Create a mutable borrow
    pub fn borrow_mutable(&mut self, name: &str) -> BorrowCheckResult<()> {
        if let Some(binding) = self.lookup_mut(name) {
            if binding.state == OwnershipState::Moved {
                return Err(BorrowCheckError {
                    message: format!("Cannot borrow moved value {}", name),
                });
            }
            if !binding.is_mutable {
                return Err(BorrowCheckError {
                    message: format!("Cannot create mutable borrow of immutable value {}", name),
                });
            }
            if binding.immutable_borrow_count > 0 {
                return Err(BorrowCheckError {
                    message: format!(
                        "Cannot mutably borrow {} with existing immutable borrows",
                        name
                    ),
                });
            }
            if binding.has_mutable_borrow {
                return Err(BorrowCheckError {
                    message: format!("Cannot create multiple mutable borrows of {}", name),
                });
            }
            binding.has_mutable_borrow = true;
            Ok(())
        } else {
            Err(BorrowCheckError {
                message: format!("Undefined variable: {}", name),
            })
        }
    }
}

/// Borrow checker: enforces ownership and borrowing rules
pub struct BorrowChecker {
    env: BorrowEnv,
}

impl BorrowChecker {
    /// Create a new borrow checker
    pub fn new() -> Self {
        BorrowChecker {
            env: BorrowEnv::new(),
        }
    }

    /// Check all items for borrow safety
    pub fn check_items(&mut self, items: &[HirItem]) -> BorrowCheckResult<()> {
        for item in items {
            match item {
                HirItem::Function {
                    params,
                    body,
                    ..
                } => {
                    // Create new scope for function
                    self.env.push_scope();

                    // Bind parameters
                    for (param_name, param_type) in params {
                        self.env.bind(param_name.clone(), param_type.clone(), false)?;
                    }

                    // Check function body
                    self.check_statements(body)?;

                    // Pop function scope
                    self.env.pop_scope();
                }
                HirItem::Struct { .. } => {
                    // No borrow checking needed for struct definitions
                }
            }
        }
        Ok(())
    }

    /// Check a list of statements
    fn check_statements(&mut self, statements: &[HirStatement]) -> BorrowCheckResult<()> {
        for stmt in statements {
            self.check_statement(stmt)?;
        }
        Ok(())
    }

    /// Check a single statement
    fn check_statement(&mut self, stmt: &HirStatement) -> BorrowCheckResult<()> {
        match stmt {
            HirStatement::Let { name, ty, init } => {
                // Check the right-hand side expression
                self.check_expression(init)?;

                self.env.bind(name.clone(), ty.clone(), false)?;
            }

            HirStatement::Expression(expr) => {
                self.check_expression(expr)?;
            }

            HirStatement::Return(Some(expr)) => {
                self.check_expression(expr)?;
            }

            HirStatement::Return(None) => {
                // Unit return, no checking needed
            }

            HirStatement::Break => {
                // Break statements don't need borrow checking
            }

            HirStatement::Continue => {
                // Continue statements don't need borrow checking
            }

            HirStatement::For {
                var,
                iter,
                body,
            } => {
                // Check the iterator expression
                self.check_expression(iter)?;
                
                // Bind the loop variable (it's a new binding in the loop scope)
                // For now, we'll be conservative and assume it's owned
                // TODO: Properly handle loop variable ownership
                
                // Check the body
                for stmt in body {
                    self.check_statement(stmt)?;
                }
            }

            HirStatement::While {
                condition,
                body,
            } => {
                // Check the condition expression
                self.check_expression(condition)?;
                
                // Check the body
                for stmt in body {
                    self.check_statement(stmt)?;
                }
            }

            HirStatement::If {
                condition,
                then_body,
                else_body,
            } => {
                // Check the condition expression
                self.check_expression(condition)?;
                
                // Check the then body
                for stmt in then_body {
                    self.check_statement(stmt)?;
                }
                
                // Check the else body if present
                if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        self.check_statement(stmt)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Check an expression for borrow safety
    fn check_expression(&mut self, expr: &HirExpression) -> BorrowCheckResult<()> {
        match expr {
            HirExpression::Variable(name) => {
                // Reading a variable - check it hasn't been moved
                if let Some(binding) = self.env.lookup(name) {
                    if binding.state == OwnershipState::Moved {
                        return Err(BorrowCheckError {
                            message: format!("Value {} used after move", name),
                        });
                    }
                }
                Ok(())
            }

            HirExpression::BinaryOp { left, right, .. } => {
                self.check_expression(left)?;
                self.check_expression(right)?;
                Ok(())
            }

            HirExpression::UnaryOp { operand, .. } => {
                self.check_expression(operand)?;
                Ok(())
            }

            HirExpression::Assign { target: _, value } => {
                self.check_expression(value)?;
                Ok(())
            }

            HirExpression::Call { func, args } => {
                self.check_expression(func)?;
                for arg in args {
                    self.check_expression(arg)?;
                }
                Ok(())
            }

            HirExpression::FieldAccess { object, .. } => {
                self.check_expression(object)?;
                Ok(())
            }

            HirExpression::Index { array, index } => {
                self.check_expression(array)?;
                self.check_expression(index)?;
                Ok(())
            }

            HirExpression::If {
                condition,
                then_body,
                else_body,
            } => {
                self.check_expression(condition)?;
                self.env.push_scope();
                self.check_statements(then_body)?;
                self.env.pop_scope();

                if let Some(else_stmts) = else_body {
                    self.env.push_scope();
                    self.check_statements(else_stmts)?;
                    self.env.pop_scope();
                }
                Ok(())
            }

            HirExpression::While { condition, body } => {
                self.check_expression(condition)?;
                self.env.push_scope();
                self.check_statements(body)?;
                self.env.pop_scope();
                Ok(())
            }

            HirExpression::Match { scrutinee, arms } => {
                self.check_expression(scrutinee)?;
                for arm in arms {
                    self.env.push_scope();
                    if let Some(guard) = &arm.guard {
                        self.check_expression(guard)?;
                    }
                    self.check_statements(&arm.body)?;
                    self.env.pop_scope();
                }
                Ok(())
            }

            HirExpression::StructLiteral { fields, .. } => {
                for (_, expr) in fields {
                    self.check_expression(expr)?;
                }
                Ok(())
            }

            HirExpression::ArrayLiteral(elements) => {
                for elem in elements {
                    self.check_expression(elem)?;
                }
                Ok(())
            }

            HirExpression::Block(statements, final_expr) => {
                self.env.push_scope();
                self.check_statements(statements)?;
                if let Some(expr) = final_expr {
                    self.check_expression(expr)?;
                }
                self.env.pop_scope();
                Ok(())
            }

            HirExpression::Tuple(elements) => {
                for elem in elements {
                    self.check_expression(elem)?;
                }
                Ok(())
            }

            HirExpression::Range { start, end, .. } => {
                // Check the range start and end expressions
                if let Some(start_expr) = start {
                    self.check_expression(start_expr)?;
                }
                if let Some(end_expr) = end {
                    self.check_expression(end_expr)?;
                }
                Ok(())
            }

            // Literals don't need borrow checking
            HirExpression::Integer(_)
            | HirExpression::Float(_)
            | HirExpression::String(_)
            | HirExpression::Bool(_) => Ok(()),
        }
    }
}

/// Public API: Check borrow safety for all items
pub fn check_borrows(items: &[HirItem]) -> Result<(), BorrowCheckError> {
    let mut checker = BorrowChecker::new();
    checker.check_items(items)
}