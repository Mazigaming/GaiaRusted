//! # Phase 5: BORROW CHECKING & OWNERSHIP VERIFICATION
//!
//! Enforces Rust's memory safety rules through ownership and borrowing analysis.
//!
//! ## Components:
//! - **Lifetime tracking** (lifetimes.rs): Lifetime variables, constraints, elision rules
//! - **Scope management** (scopes.rs): Lexical scope tracking and binding visibility
//! - **Borrow checking** (mod.rs): Ownership, move semantics, reference validation
//!
//! ## What we do:
//! - Lifetime representation and inference
//! - Ownership tracking (each value has one owner)
//! - Borrow analysis (multiple immutable or one mutable)
//! - Use-after-move detection
//! - Use-after-free detection
//! - Multiple mutable reference detection
//! - Scope-based lifetime validation
//!
//! ## Algorithm:
//! 1. Register all lifetime parameters and create fresh lifetimes
//! 2. Build scope hierarchy as we traverse the program
//! 3. Track moves and borrows through each scope
//! 4. Verify no value is used after move
//! 5. Verify no multiple mutable borrows of same value
//! 6. Verify all borrows respect their lifetime constraints

pub mod lifetimes;
pub mod scopes;
pub mod nll;
pub mod function_lifetimes;
pub mod struct_lifetimes;
pub mod self_lifetimes;
pub mod impl_lifetimes;
pub mod interior_mutability;
pub mod smart_pointers;
pub mod reference_cycles;
pub mod lifetime_solver;
pub mod unsafe_checking;
pub mod unsafe_checking_enhanced;
pub mod lifetime_validation;

pub use lifetimes::{Lifetime, LifetimeContext, LifetimeConstraint, LifetimeId, LifetimeElision};
pub use scopes::{Scope, ScopeId, ScopeStack, ScopeBinding};
pub use nll::{BorrowTracker, Location, BorrowId, UsageAnalyzer, BorrowInfo};
pub use lifetime_validation::{LifetimeValidator, FunctionLifetimeValidator, StructLifetimeValidator};
pub use impl_lifetimes::{SelfKind, ImplMethodValidator, ImplLifetimeError, MethodLifetimeLocation};

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
/// Enhanced version that uses ScopeStack for better lifetime tracking
#[derive(Debug)]
pub struct BorrowEnv {
    /// Lexical scope hierarchy
    scopes: ScopeStack,
    /// Lifetime constraints and context
    lifetime_ctx: LifetimeContext,
    /// Ownership states for each binding (name -> state)
    ownership_states: HashMap<String, OwnershipState>,
}

impl BorrowEnv {
    /// Create a new borrow environment
    pub fn new() -> Self {
        BorrowEnv {
            scopes: ScopeStack::new(),
            lifetime_ctx: LifetimeContext::new(),
            ownership_states: HashMap::new(),
        }
    }

    /// Push a new scope
    pub fn push_scope(&mut self) {
        self.scopes.push_scope();
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) {
        self.scopes.pop_scope();
    }

    /// Get the lifetime context
    pub fn lifetime_context(&self) -> &LifetimeContext {
        &self.lifetime_ctx
    }

    /// Get the lifetime context mutably
    pub fn lifetime_context_mut(&mut self) -> &mut LifetimeContext {
        &mut self.lifetime_ctx
    }

    /// Get the scope stack
    pub fn scopes(&self) -> &ScopeStack {
        &self.scopes
    }

    /// Bind a new variable with optional lifetime
    pub fn bind(&mut self, name: String, ty: HirType, is_mutable: bool) -> BorrowCheckResult<()> {
        self.scopes
            .add_binding(name.clone(), ty, is_mutable, None)
            .map_err(|e| BorrowCheckError {
                message: e,
            })?;
        self.ownership_states.insert(name, OwnershipState::Owned);
        Ok(())
    }

    /// Bind with explicit lifetime
    pub fn bind_with_lifetime(
        &mut self,
        name: String,
        ty: HirType,
        is_mutable: bool,
        lifetime: Lifetime,
    ) -> BorrowCheckResult<()> {
        self.scopes
            .add_binding(name.clone(), ty, is_mutable, Some(lifetime))
            .map_err(|e| BorrowCheckError {
                message: e,
            })?;
        self.ownership_states.insert(name, OwnershipState::Owned);
        Ok(())
    }

    /// Look up a binding (searches from innermost to outermost scope)
    pub fn lookup(&self, name: &str) -> Option<Binding> {
        self.scopes.lookup(name).map(|scope_binding| Binding {
            state: self
                .ownership_states
                .get(name)
                .cloned()
                .unwrap_or(OwnershipState::Owned),
            ty: scope_binding.ty.clone(),
            is_mutable: scope_binding.is_mutable,
            immutable_borrow_count: 0,
            has_mutable_borrow: false,
        })
    }

    /// Look up a binding mutably (for ownership state changes)
    fn lookup_mut(&mut self, name: &str) -> Option<Binding> {
        self.scopes.lookup(name).map(|scope_binding| Binding {
            state: self
                .ownership_states
                .get(name)
                .cloned()
                .unwrap_or(OwnershipState::Owned),
            ty: scope_binding.ty.clone(),
            is_mutable: scope_binding.is_mutable,
            immutable_borrow_count: 0,
            has_mutable_borrow: false,
        })
    }

    /// Mark a binding as moved
    pub fn move_binding(&mut self, name: &str) -> BorrowCheckResult<()> {
        // Check if binding exists
        if !self.scopes.contains(name) {
            return Err(BorrowCheckError {
                message: format!("Undefined variable: {}", name),
            });
        }

        // Check current state
        let current_state = self
            .ownership_states
            .get(name)
            .cloned()
            .unwrap_or(OwnershipState::Owned);

        match current_state {
            OwnershipState::Moved => {
                return Err(BorrowCheckError {
                    message: format!("Value {} used after move", name),
                });
            }
            OwnershipState::BorrowedMutable => {
                return Err(BorrowCheckError {
                    message: format!("Cannot move borrowed value {}", name),
                });
            }
            OwnershipState::BorrowedImmutable => {
                return Err(BorrowCheckError {
                    message: format!("Cannot move borrowed value {}", name),
                });
            }
            _ => {}
        }

        self.ownership_states.insert(name.to_string(), OwnershipState::Moved);
        Ok(())
    }

    /// Create an immutable borrow
    pub fn borrow_immutable(&mut self, name: &str) -> BorrowCheckResult<()> {
        // Check if binding exists
        if !self.scopes.contains(name) {
            return Err(BorrowCheckError {
                message: format!("Undefined variable: {}", name),
            });
        }

        // Check current state
        let current_state = self
            .ownership_states
            .get(name)
            .cloned()
            .unwrap_or(OwnershipState::Owned);

        match current_state {
            OwnershipState::Moved => {
                return Err(BorrowCheckError {
                    message: format!("Cannot borrow moved value {}", name),
                });
            }
            OwnershipState::BorrowedMutable => {
                return Err(BorrowCheckError {
                    message: format!("Cannot immutably borrow mutably borrowed value {}", name),
                });
            }
            _ => {}
        }

        self.ownership_states
            .insert(name.to_string(), OwnershipState::BorrowedImmutable);
        Ok(())
    }

    /// Create a mutable borrow
    pub fn borrow_mutable(&mut self, name: &str) -> BorrowCheckResult<()> {
        // Check if binding exists
        if !self.scopes.contains(name) {
            return Err(BorrowCheckError {
                message: format!("Undefined variable: {}", name),
            });
        }

        // Get the binding info
        let scope_binding = self.scopes.lookup(name).ok_or_else(|| BorrowCheckError {
            message: format!("Undefined variable: {}", name),
        })?;

        if !scope_binding.is_mutable {
            return Err(BorrowCheckError {
                message: format!("Cannot create mutable borrow of immutable value {}", name),
            });
        }

        // Check current state
        let current_state = self
            .ownership_states
            .get(name)
            .cloned()
            .unwrap_or(OwnershipState::Owned);

        match current_state {
            OwnershipState::Moved => {
                return Err(BorrowCheckError {
                    message: format!("Cannot borrow moved value {}", name),
                });
            }
            OwnershipState::BorrowedImmutable => {
                return Err(BorrowCheckError {
                    message: format!(
                        "Cannot mutably borrow {} with existing immutable borrows",
                        name
                    ),
                });
            }
            OwnershipState::BorrowedMutable => {
                return Err(BorrowCheckError {
                    message: format!("Cannot create multiple mutable borrows of {}", name),
                });
            }
            _ => {}
        }

        self.ownership_states
            .insert(name.to_string(), OwnershipState::BorrowedMutable);
        Ok(())
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
                }
                HirItem::AssociatedType { .. } => {
                }
                HirItem::Use { .. } => {
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
            HirStatement::Let { name, mutable, ty, init } => {
                // Check the right-hand side expression
                self.check_expression(init)?;

                self.env.bind(name.clone(), ty.clone(), *mutable)?;
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
                var: _,
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

            HirStatement::UnsafeBlock(stmts) => {
                // Unsafe blocks bypass borrow checking
                // We don't need to check borrows inside unsafe blocks
                // But we should still recurse for consistency
                for stmt in stmts {
                    self.check_statement(stmt)?;
                }
            }

            HirStatement::Item(_) => {
                // Nested items don't need borrow checking at this level
                // They will be checked separately
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

            HirExpression::TupleAccess { object, .. } => {
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

            HirExpression::Closure { body, .. } => {
                self.env.push_scope();
                self.check_statements(body)?;
                self.env.pop_scope();
                Ok(())
            }

            HirExpression::Try { value } => {
                self.check_expression(value)?;
                Ok(())
            }
        }
    }
}

/// Public API: Check borrow safety for all items
pub fn check_borrows(items: &[HirItem]) -> Result<(), BorrowCheckError> {
    let mut checker = BorrowChecker::new();
    checker.check_items(items)
}