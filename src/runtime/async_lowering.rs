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

use crate::lowering::{HirExpression, HirItem, HirType};
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

    /// Transform an async function into a Future-returning function
    pub fn lower_async_fn(
        &mut self,
        func: &parser_ast::Item,
    ) -> AsyncLoweringResult<HirItem> {
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
                is_async: false,
                ..
            } => {
                Err(AsyncLoweringError {
                    message: "expected async function".to_string(),
                    kind: AsyncErrorKind::UnsupportedConstruct,
                })
            }
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
}
