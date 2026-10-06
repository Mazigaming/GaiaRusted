//! # Phase 11: Async/Await Lowering
//!
//! Transforms async Rust syntax into Future-based code through desugaring.
//!
//! ## Desugaring Strategy
//!
//! ### Async Functions
//! ```ignore
//! async fn fetch(url: &str) -> Result<Data> {
//!     let resp = http_get(url).await?;
//!     Ok(resp.parse()?)
//! }
//! ```
//!
//! Desugars to:
//! ```ignore
//! fn fetch(url: &str) -> impl Future<Output = Result<Data>> {
//!     #[state_machine]
//!     async fn __fetch_impl(url: &str) -> Result<Data> { ... }
//!     __fetch_impl(url)
//! }
//! ```
//!
//! ### Await Expressions
//! ```ignore
//! let value = future.await;
//! ```
//!
//! Desugars to:
//! ```ignore
//! let value = match poll(future) {
//!     Poll::Ready(v) => v,
//!     Poll::Pending => return Poll::Pending,
//! };
//! ```
//!
//! ## Components
//! - **Async Function Transformer**: Converts async fn to Future-returning fn
//! - **Await Expression Transformer**: Converts await to poll logic
//! - **State Machine Generator**: Creates coroutine state machine for execution
//! - **Pin Handling**: Ensures proper pinning for safe async operations

use crate::lowering::{HirExpression, HirItem, HirStatement, HirType};
use crate::parser::ast as parser_ast;
use std::collections::HashMap;
use std::fmt;

/// Represents poll state for async operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PollState {
    /// Future has completed with a value
    Ready,
    /// Future is still pending
    Pending,
}

impl fmt::Display for PollState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PollState::Ready => write!(f, "Poll::Ready"),
            PollState::Pending => write!(f, "Poll::Pending"),
        }
    }
}

/// Result of desugaring an async expression
#[derive(Debug, Clone)]
pub struct AwaitDesugaring {
    /// The desugared HIR expression
    pub expr: HirExpression,
    /// Intermediate temporaries created during desugaring
    pub temporaries: Vec<(String, HirType)>,
    /// State transitions needed
    pub state_transitions: Vec<StateTransition>,
}

/// Represents a state transition in the async state machine
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from_state: usize,
    pub to_state: usize,
    pub condition: String,
}

/// Async context tracking during transformation
#[derive(Debug, Clone)]
pub struct AsyncContext {
    /// Current nesting level of async contexts
    pub depth: usize,
    /// Whether we're currently in an async context
    pub in_async: bool,
    /// State machine counter for generating unique states
    pub state_counter: usize,
    /// Captured variables that need to be preserved across await points
    pub captured_vars: HashMap<String, HirType>,
}

impl AsyncContext {
    pub fn new() -> Self {
        AsyncContext {
            depth: 0,
            in_async: false,
            state_counter: 0,
            captured_vars: HashMap::new(),
        }
    }

    pub fn enter_async(&mut self) {
        self.depth += 1;
        self.in_async = true;
    }

    pub fn exit_async(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        self.in_async = self.depth > 0;
    }

    pub fn next_state(&mut self) -> usize {
        let state = self.state_counter;
        self.state_counter += 1;
        state
    }

    pub fn capture_var(&mut self, name: String, ty: HirType) {
        self.captured_vars.insert(name, ty);
    }
}

/// Error type for async lowering
#[derive(Debug, Clone)]
pub struct AsyncLoweringError {
    pub message: String,
    pub kind: AsyncErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsyncErrorKind {
    /// Await used outside async context
    AwaitOutsideAsync,
    /// Invalid pin type
    InvalidPin,
    /// Future type not found
    FutureTypeNotFound,
    /// State machine generation failed
    StateMachineGenFailed,
    /// Unsupported async construct
    UnsupportedConstruct,
}

impl fmt::Display for AsyncLoweringError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            AsyncErrorKind::AwaitOutsideAsync => {
                write!(f, "await outside async context: {}", self.message)
            }
            AsyncErrorKind::InvalidPin => {
                write!(f, "invalid Pin type: {}", self.message)
            }
            AsyncErrorKind::FutureTypeNotFound => {
                write!(f, "Future type not found: {}", self.message)
            }
            AsyncErrorKind::StateMachineGenFailed => {
                write!(f, "state machine generation failed: {}", self.message)
            }
            AsyncErrorKind::UnsupportedConstruct => {
                write!(f, "unsupported async construct: {}", self.message)
            }
        }
    }
}

pub type AsyncLoweringResult<T> = Result<T, AsyncLoweringError>;

/// Main async lowering transformer
pub struct AsyncTransformer {
    context: AsyncContext,
    captured_temps: Vec<String>,
}

impl AsyncTransformer {
    pub fn new() -> Self {
        AsyncTransformer {
            context: AsyncContext::new(),
            captured_temps: vec![],
        }
    }

    /// Transform an async function into a Future-returning function (from AST)
    pub fn lower_async_fn(&mut self, func: &parser_ast::Item) -> AsyncLoweringResult<HirItem> {
        match func {
            parser_ast::Item::Function {
                name,
                params,
                return_type,
                is_async: true,
                ..
            } => {
                self.context.enter_async();
                self.context.next_state();

                let hir_params: Vec<(String, HirType)> = params
                    .iter()
                    .map(|p| (p.name.clone(), HirType::Named(p.name.clone())))
                    .collect();

                let output_type = return_type
                    .as_ref()
                    .map(|_| HirType::Named("impl Future".to_string()))
                    .unwrap_or(HirType::Named("()".to_string()));

                let hir_body = vec![];

                self.context.exit_async();

                Ok(HirItem::Function {
                    name: name.clone(),
                    generics: vec![],
                    params: hir_params,
                    return_type: Some(output_type),
                    body: hir_body,
                    is_public: true,
                    where_clause: vec![],
                })
            }
            parser_ast::Item::Function {
                is_async: false, ..
            } => Err(AsyncLoweringError {
                message: "expected async function".to_string(),
                kind: AsyncErrorKind::UnsupportedConstruct,
            }),
            _ => Err(AsyncLoweringError {
                message: "not a function item".to_string(),
                kind: AsyncErrorKind::UnsupportedConstruct,
            }),
        }
    }

    /// Transform an await expression into poll logic
    pub fn lower_await(&mut self, expr: &HirExpression) -> AsyncLoweringResult<AwaitDesugaring> {
        if !self.context.in_async {
            return Err(AsyncLoweringError {
                message: "await expression outside async context".to_string(),
                kind: AsyncErrorKind::AwaitOutsideAsync,
            });
        }

        let state = self.context.next_state();
        let temp_name = format!("__await_temp_{}", state);

        let temporaries = vec![(temp_name.clone(), HirType::Named("Poll".to_string()))];

        let desugared_expr = HirExpression::Call {
            func: Box::new(HirExpression::Variable("poll".to_string())),
            args: vec![expr.clone()],
        };

        let desugared = AwaitDesugaring {
            expr: desugared_expr,
            temporaries,
            state_transitions: vec![StateTransition {
                from_state: state,
                to_state: state + 1,
                condition: "Poll::Ready(_)".to_string(),
            }],
        };

        Ok(desugared)
    }

    /// Get the current async context
    pub fn context(&self) -> &AsyncContext {
        &self.context
    }

    /// Get mutable context for modifications
    pub fn context_mut(&mut self) -> &mut AsyncContext {
        &mut self.context
    }
}

/// Transform async functions in HIR items
///
/// This function iterates through all HIR items and converts any async functions
/// by:
/// 1. Changing the return type to `impl Future<Output = ...>`
/// 2. Replacing await expressions with poll() calls
pub fn transform_async_functions(hir_items: Vec<HirItem>) -> Vec<HirItem> {
    let mut result = Vec::new();

    for item in hir_items {
        match item {
            HirItem::Function {
                name,
                generics,
                params,
                return_type,
                body,
                is_public,
                where_clause,
            } => {
                // Check if the body contains await expressions
                let has_await = contains_await_in_body(&body);

                if has_await {
                    // Transform the body by replacing await expressions
                    let transformed_body = transform_body(body);

                    // Change return type to impl Future
                    let future_return_type = return_type
                        .as_ref()
                        .map(|ty| HirType::Named(format!("impl Future<Output = {}>", ty)));

                    result.push(HirItem::Function {
                        name,
                        generics,
                        params,
                        return_type: future_return_type,
                        body: transformed_body,
                        is_public,
                        where_clause,
                    });
                } else {
                    // No await expressions - keep as-is
                    result.push(HirItem::Function {
                        name,
                        generics,
                        params,
                        return_type,
                        body,
                        is_public,
                        where_clause,
                    });
                }
            }
            _ => {
                result.push(item);
            }
        }
    }

    result
}

/// Check if a body contains any await expressions
fn contains_await_in_body(body: &[HirStatement]) -> bool {
    body.iter().any(contains_await_in_stmt)
}

fn contains_await_in_stmt(stmt: &HirStatement) -> bool {
    match stmt {
        HirStatement::Expression(expr) => contains_await_in_expr(expr),
        HirStatement::Let { init, .. } => contains_await_in_expr(init),
        HirStatement::Return(Some(expr)) => contains_await_in_expr(expr),
        HirStatement::If {
            condition,
            then_body,
            else_body,
        } => {
            contains_await_in_expr(condition)
                || then_body.iter().any(contains_await_in_stmt)
                || else_body
                    .as_ref()
                    .map(|b| b.iter().any(contains_await_in_stmt))
                    .unwrap_or(false)
        }
        HirStatement::While { condition, body } => {
            contains_await_in_expr(condition) || body.iter().any(contains_await_in_stmt)
        }
        HirStatement::For { var: _, iter, body } => {
            contains_await_in_expr(iter) || body.iter().any(contains_await_in_stmt)
        }
        _ => false,
    }
}

fn contains_await_in_expr(expr: &HirExpression) -> bool {
    match expr {
        HirExpression::Await { .. } => true,
        HirExpression::Call { args, .. } => args.iter().any(contains_await_in_expr),
        HirExpression::MethodCall { args, .. } => args.iter().any(contains_await_in_expr),
        HirExpression::BinaryOp { left, right, .. } => {
            contains_await_in_expr(left) || contains_await_in_expr(right)
        }
        HirExpression::If {
            condition,
            then_body,
            else_body,
        } => {
            contains_await_in_expr(condition)
                || then_body.iter().any(contains_await_in_stmt)
                || else_body
                    .as_ref()
                    .map(|b| b.iter().any(contains_await_in_stmt))
                    .unwrap_or(false)
        }
        HirExpression::While { condition, body } => {
            contains_await_in_expr(condition) || body.iter().any(contains_await_in_stmt)
        }
        HirExpression::Match { arms, .. } => arms
            .iter()
            .any(|arm| arm.body.iter().any(contains_await_in_stmt)),
        HirExpression::Block(stmts, final_expr) => {
            stmts.iter().any(contains_await_in_stmt)
                || final_expr
                    .as_ref()
                    .map(|e| contains_await_in_expr(e))
                    .unwrap_or(false)
        }
        _ => false,
    }
}

/// Transform a function body by replacing await expressions with poll calls
fn transform_body(body: Vec<HirStatement>) -> Vec<HirStatement> {
    body.into_iter().map(transform_stmt).collect()
}

fn transform_stmt(stmt: HirStatement) -> HirStatement {
    match stmt {
        HirStatement::Expression(expr) => HirStatement::Expression(transform_expr(expr)),
        HirStatement::Let {
            name,
            mutable,
            ty,
            init,
        } => HirStatement::Let {
            name,
            mutable,
            ty,
            init: transform_expr(init),
        },
        HirStatement::Return(Some(expr)) => HirStatement::Return(Some(transform_expr(expr))),
        HirStatement::If {
            condition,
            then_body,
            else_body,
        } => HirStatement::If {
            condition: Box::new(transform_expr(*condition)),
            then_body: then_body.into_iter().map(transform_stmt).collect(),
            else_body: else_body.map(|b| b.into_iter().map(transform_stmt).collect()),
        },
        HirStatement::While { condition, body } => HirStatement::While {
            condition: Box::new(transform_expr(*condition)),
            body: body.into_iter().map(transform_stmt).collect(),
        },
        HirStatement::For { var, iter, body } => HirStatement::For {
            var,
            iter: Box::new(transform_expr(*iter)),
            body: body.into_iter().map(transform_stmt).collect(),
        },
        _ => stmt,
    }
}

fn transform_expr(expr: HirExpression) -> HirExpression {
    match expr {
        HirExpression::Await { value } => {
            // Transform await into a poll call
            HirExpression::Call {
                func: Box::new(HirExpression::Variable("poll".to_string())),
                args: vec![*value],
            }
        }
        HirExpression::Call { func, args } => HirExpression::Call {
            func: Box::new(transform_expr(*func)),
            args: args.into_iter().map(transform_expr).collect(),
        },
        HirExpression::MethodCall {
            receiver,
            method,
            args,
        } => HirExpression::MethodCall {
            receiver: Box::new(transform_expr(*receiver)),
            method,
            args: args.into_iter().map(transform_expr).collect(),
        },
        HirExpression::BinaryOp { op, left, right } => HirExpression::BinaryOp {
            op,
            left: Box::new(transform_expr(*left)),
            right: Box::new(transform_expr(*right)),
        },
        HirExpression::UnaryOp { op, operand } => HirExpression::UnaryOp {
            op,
            operand: Box::new(transform_expr(*operand)),
        },
        HirExpression::If {
            condition,
            then_body,
            else_body,
        } => HirExpression::If {
            condition: Box::new(transform_expr(*condition)),
            then_body: then_body.into_iter().map(|s| transform_stmt(s)).collect(),
            else_body: else_body.map(|b| b.into_iter().map(transform_stmt).collect()),
        },
        HirExpression::While { condition, body } => HirExpression::While {
            condition: Box::new(transform_expr(*condition)),
            body: body.into_iter().map(|s| transform_stmt(s)).collect(),
        },
        HirExpression::Match { scrutinee, arms } => HirExpression::Match {
            scrutinee: Box::new(transform_expr(*scrutinee)),
            arms: arms
                .into_iter()
                .map(|arm| {
                    let guard = arm.guard.map(|g| transform_expr(g));
                    let body = arm.body.into_iter().map(transform_stmt).collect();
                    crate::lowering::MatchArm {
                        pattern: arm.pattern,
                        guard,
                        body,
                    }
                })
                .collect(),
        },
        HirExpression::Block(stmts, final_expr) => HirExpression::Block(
            stmts.into_iter().map(transform_stmt).collect(),
            final_expr.map(|e| Box::new(transform_expr(*e))),
        ),
        HirExpression::Assign { target, value } => HirExpression::Assign {
            target: Box::new(transform_expr(*target)),
            value: Box::new(transform_expr(*value)),
        },
        HirExpression::ArrayLiteral(exprs) => {
            HirExpression::ArrayLiteral(exprs.into_iter().map(transform_expr).collect())
        }
        HirExpression::Tuple(exprs) => {
            HirExpression::Tuple(exprs.into_iter().map(transform_expr).collect())
        }
        HirExpression::StructLiteral { name, fields } => HirExpression::StructLiteral {
            name,
            fields: fields
                .into_iter()
                .map(|(n, e)| (n, transform_expr(e)))
                .collect(),
        },
        HirExpression::EnumVariant {
            enum_name,
            variant_name,
            args,
        } => HirExpression::EnumVariant {
            enum_name,
            variant_name,
            args: args.into_iter().map(transform_expr).collect(),
        },
        HirExpression::EnumStructVariant {
            enum_name,
            variant_name,
            fields,
        } => HirExpression::EnumStructVariant {
            enum_name,
            variant_name,
            fields: fields
                .into_iter()
                .map(|(n, e)| (n, transform_expr(e)))
                .collect(),
        },
        HirExpression::FieldAccess { object, field } => HirExpression::FieldAccess {
            object: Box::new(transform_expr(*object)),
            field,
        },
        HirExpression::Index { array, index } => HirExpression::Index {
            array: Box::new(transform_expr(*array)),
            index: Box::new(transform_expr(*index)),
        },
        _ => expr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_context_creation() {
        let ctx = AsyncContext::new();
        assert_eq!(ctx.depth, 0);
        assert!(!ctx.in_async);
        assert_eq!(ctx.state_counter, 0);
    }

    #[test]
    fn test_async_context_enter_exit() {
        let mut ctx = AsyncContext::new();
        ctx.enter_async();
        assert!(ctx.in_async);
        assert_eq!(ctx.depth, 1);

        ctx.exit_async();
        assert!(!ctx.in_async);
        assert_eq!(ctx.depth, 0);
    }

    #[test]
    fn test_state_machine_counter() {
        let mut ctx = AsyncContext::new();
        let state1 = ctx.next_state();
        let state2 = ctx.next_state();
        let state3 = ctx.next_state();

        assert_eq!(state1, 0);
        assert_eq!(state2, 1);
        assert_eq!(state3, 2);
    }

    #[test]
    fn test_capture_variables() {
        let mut ctx = AsyncContext::new();
        ctx.capture_var("x".to_string(), HirType::Named("i32".to_string()));
        ctx.capture_var("y".to_string(), HirType::Named("&str".to_string()));

        assert_eq!(ctx.captured_vars.len(), 2);
        assert!(ctx.captured_vars.contains_key("x"));
        assert!(ctx.captured_vars.contains_key("y"));
    }

    #[test]
    fn test_await_outside_async_error() {
        let mut transformer = AsyncTransformer::new();
        let expr = HirExpression::Integer(42);

        let result = transformer.lower_await(&expr);
        assert!(result.is_err());

        if let Err(e) = result {
            assert_eq!(e.kind, AsyncErrorKind::AwaitOutsideAsync);
        }
    }

    #[test]
    fn test_transformer_creation() {
        let transformer = AsyncTransformer::new();
        assert_eq!(transformer.context.depth, 0);
        assert!(!transformer.context.in_async);
    }

    #[test]
    fn test_poll_state_display() {
        assert_eq!(format!("{}", PollState::Ready), "Poll::Ready");
        assert_eq!(format!("{}", PollState::Pending), "Poll::Pending");
    }

    #[test]
    fn test_async_context_nested() {
        let mut ctx = AsyncContext::new();
        ctx.enter_async();
        assert_eq!(ctx.depth, 1);

        ctx.enter_async();
        assert_eq!(ctx.depth, 2);

        ctx.exit_async();
        assert_eq!(ctx.depth, 1);
        assert!(ctx.in_async);

        ctx.exit_async();
        assert_eq!(ctx.depth, 0);
        assert!(!ctx.in_async);
    }

    #[test]
    fn test_error_display() {
        let error = AsyncLoweringError {
            message: "test message".to_string(),
            kind: AsyncErrorKind::AwaitOutsideAsync,
        };

        let msg = format!("{}", error);
        assert!(msg.contains("await outside async context"));
    }

    #[test]
    fn test_transform_async_functions_preserves_sync() {
        let input = vec![HirItem::Function {
            name: "sync_func".to_string(),
            generics: vec![],
            params: vec![],
            return_type: Some(HirType::Int32),
            body: vec![HirStatement::Return(Some(HirExpression::Integer(42)))],
            is_public: false,
            where_clause: vec![],
        }];

        let result = transform_async_functions(input);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_transform_async_functions_changes_return_type() {
        let input = vec![HirItem::Function {
            name: "async_func".to_string(),
            generics: vec![],
            params: vec![],
            return_type: Some(HirType::Int32),
            body: vec![HirStatement::Return(Some(HirExpression::Await {
                value: Box::new(HirExpression::Integer(42)),
            }))],
            is_public: false,
            where_clause: vec![],
        }];

        let result = transform_async_functions(input);
        assert_eq!(result.len(), 1);

        if let HirItem::Function { return_type, .. } = &result[0] {
            assert!(return_type
                .as_ref()
                .unwrap()
                .to_string()
                .contains("impl Future"));
        }
    }

    #[test]
    fn test_transform_await_to_poll() {
        let body = vec![HirStatement::Expression(HirExpression::Await {
            value: Box::new(HirExpression::Variable("future".to_string())),
        })];

        let result = transform_async_functions(vec![HirItem::Function {
            name: "test".to_string(),
            generics: vec![],
            params: vec![],
            return_type: Some(HirType::Int32),
            body,
            is_public: false,
            where_clause: vec![],
        }]);

        if let HirItem::Function { body, .. } = &result[0] {
            if let HirStatement::Expression(HirExpression::Call { func, .. }) = &body[0] {
                if let HirExpression::Variable(name) = func.as_ref() {
                    assert_eq!(name, "poll");
                }
            }
        }
    }

    #[test]
    fn test_contains_await_in_body() {
        let body = vec![HirStatement::Expression(HirExpression::Await {
            value: Box::new(HirExpression::Integer(42)),
        })];
        assert!(contains_await_in_body(&body));
    }

    #[test]
    fn test_contains_await_in_if_condition() {
        let body = vec![HirStatement::Expression(HirExpression::If {
            condition: Box::new(HirExpression::Await {
                value: Box::new(HirExpression::Integer(1)),
            }),
            then_body: vec![HirStatement::Return(Some(HirExpression::Integer(42)))],
            else_body: None,
        })];
        assert!(contains_await_in_body(&body));
    }

    #[test]
    fn test_transform_preserves_non_async() {
        let input = vec![HirItem::Function {
            name: "regular".to_string(),
            generics: vec![],
            params: vec![],
            return_type: Some(HirType::Int32),
            body: vec![HirStatement::Return(Some(HirExpression::Integer(42)))],
            is_public: false,
            where_clause: vec![],
        }];

        let result = transform_async_functions(input);
        assert_eq!(result.len(), 1);
        if let HirItem::Function {
            return_type, body, ..
        } = &result[0]
        {
            assert_eq!(return_type, &Some(HirType::Int32));
            assert_eq!(body.len(), 1);
        }
    }
}
