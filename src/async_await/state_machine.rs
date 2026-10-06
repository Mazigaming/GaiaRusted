//! State Machine Code Generation for Async Functions
//!
//! This module transforms async functions into state machines that implement
//! the Future trait. The key transformation is:
//!
//! ```rust,ignore
//! // Input: async function
//! async fn fetch(url: String) -> Response {
//!     let data = http_request(url).await;
//!     process(data).await
//! }
//!
//! // Output: state machine struct + Future impl
//! struct FetchAsyncState {
//!     state: u8,
//!     url: String,
//!     data: Option<Response>,
//! }
//!
//! impl Future for FetchAsyncState {
//!     type Output = Response;
//!     
//!     fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Response> {
//!         match self.state {
//!             0 => {
//!                 self.state = 1;
//!                 Poll::Pending
//!             }
//!             1 => {
//!                 let response = self.data.take().unwrap();
//!                 Poll::Ready(response)
//!             }
//!             _ => unreachable!()
//!         }
//!     }
//! }
//! ```

use crate::lowering::{
    HirExpression, HirItem, HirStatement, HirType, LowerError, LowerResult, MatchArm,
};
use std::collections::{HashMap, HashSet};

/// Represents a state in the state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StateId(pub usize);

impl StateId {
    /// Create the initial state (state 0)
    pub fn start() -> Self {
        StateId(0)
    }

    /// Get the next state ID
    pub fn next(&self) -> Self {
        StateId(self.0 + 1)
    }

    /// Check if this is the start state
    pub fn is_start(&self) -> bool {
        self.0 == 0
    }
}

/// Information about an await point in the function
#[derive(Debug, Clone)]
pub struct AwaitPoint {
    /// Unique ID for this await point
    pub id: usize,

    /// The state before this await point
    pub state_before: StateId,

    /// The state after this await point (when it completes)
    pub state_after: StateId,

    /// The expression being awaited
    pub expr: HirExpression,

    /// If the await is part of an assignment, the target variable
    pub result_var: Option<String>,

    /// The type of the awaited value
    pub output_type: HirType,
}

/// A complete state machine representation
#[derive(Debug, Clone)]
pub struct StateMachine {
    /// Original function name
    pub func_name: String,

    /// Function parameters (become captured fields in struct)
    pub params: Vec<(String, HirType)>,

    /// Return type of the async function
    pub return_type: HirType,

    /// All await points identified in the function
    pub await_points: Vec<AwaitPoint>,

    /// Code for each state (state_id -> statements)
    pub state_code: HashMap<StateId, Vec<HirStatement>>,

    /// Local variables (captured automatically)
    pub local_vars: HashMap<String, HirType>,

    /// Total number of states
    pub num_states: usize,
}

impl StateMachine {
    /// Create a new state machine from an async function
    pub fn new(
        func_name: String,
        params: Vec<(String, HirType)>,
        return_type: HirType,
        await_points: Vec<AwaitPoint>,
        local_vars: HashMap<String, HirType>,
    ) -> Self {
        let num_states = if await_points.is_empty() {
            1 // Just one state: execute and return
        } else {
            await_points.len() + 1 // One state per await + final state
        };

        StateMachine {
            func_name,
            params,
            return_type,
            await_points,
            state_code: HashMap::new(),
            local_vars,
            num_states,
        }
    }

    /// Get all fields that need to be captured in the state struct
    pub fn captured_fields(&self) -> Vec<(String, HirType)> {
        let mut fields = vec![];

        // State field (always present)
        fields.push(("__state".to_string(), HirType::UInt32));

        // Capture all parameters
        for (name, ty) in &self.params {
            fields.push((name.clone(), ty.clone()));
        }

        // Capture all local variables
        for (name, ty) in &self.local_vars {
            fields.push((name.clone(), ty.clone()));
        }

        // Add result field for final value
        fields.push((
            "__result".to_string(),
            HirType::Option(Box::new(self.return_type.clone())),
        ));

        fields
    }
}

/// Generate the state machine struct for an async function
///
/// This creates a struct like:
/// ```rust,ignore
/// struct FunctionAsyncState {
///     __state: u8,                    // Current state (0..N)
///     param1: Type1,                  // Captured parameters
///     param2: Type2,
///     local1: Type1,                  // Captured local variables
///     local2: Type2,
///     __result: Option<ReturnType>,   // Final result
/// }
/// ```
pub fn generate_state_struct(state_machine: &StateMachine) -> Result<HirItem, LowerError> {
    let struct_name = format!("{}AsyncState", state_machine.func_name);

    let fields = state_machine.captured_fields();

    Ok(HirItem::Struct {
        name: struct_name,
        fields,
        derives: vec![],
        is_public: false,
    })
}

/// Generate the poll() method for the state machine's Future implementation
///
/// This creates a method like:
/// ```rust,ignore
/// fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<ReturnType> {
///     match self.__state {
///         0 => {
///             // Code for state 0
///             self.__state = 1;
///             Poll::Pending
///         }
///         1 => {
///             // Code for state 1
///             self.__state = 2;
///             Poll::Pending
///         }
///         // ... more states ...
///         N => {
///             // Final state - return result
///             let result = self.__result.take();
///             Poll::Ready(result)
///         }
///         _ => unreachable!()
///     }
/// }
/// ```
pub fn generate_poll_method(state_machine: &StateMachine) -> Result<HirItem, LowerError> {
    let func_name = "poll".to_string();
    let params = vec![
        ("self".to_string(), HirType::Unknown), // Pin<&mut Self>
        ("cx".to_string(), HirType::Unknown),   // &mut Context
    ];

    let return_type = HirType::Named(format!("Poll<{}>", state_machine.return_type));

    // Build the match statement body
    let mut match_arms = vec![];

    // Create code for each state
    for state_id in 0..state_machine.num_states {
        let state = StateId(state_id);

        if state_id == state_machine.num_states - 1 {
            // Final state: return the result
            match_arms.push(create_final_state_arm(state_machine));
        } else {
            // Intermediate state: execute code and transition
            match_arms.push(create_intermediate_state_arm(state_machine, state));
        }
    }

    // Create the unreachable pattern
    match_arms.push(create_unreachable_pattern());

    // Build the match expression
    let match_expr = HirExpression::Match {
        scrutinee: Box::new(HirExpression::FieldAccess {
            object: Box::new(HirExpression::Variable("self".to_string())),
            field: "__state".to_string(),
        }),
        arms: match_arms,
    };

    let body = vec![HirStatement::Return(Some(match_expr))];

    Ok(HirItem::Function {
        name: func_name,
        generics: vec![],
        params,
        return_type: Some(return_type),
        body,
        is_public: false,
        where_clause: vec![],
    })
}

/// Create a match arm for an intermediate state
///
/// Generates something like:
/// ```rust,ignore
/// StateId(N) => {
///     // Execute state code
///     self.__state = N + 1;
///     Poll::Pending
/// }
/// ```
fn create_intermediate_state_arm(state_machine: &StateMachine, state_id: StateId) -> MatchArm {
    let mut body = vec![];

    // Execute the code for this state
    if let Some(stmts) = state_machine.state_code.get(&state_id) {
        body.extend(stmts.clone());
    }

    // Transition to next state
    let next_state = state_id.next();
    body.push(HirStatement::Let {
        name: "__state_tmp".to_string(),
        ty: HirType::UInt32,
        init: HirExpression::Integer(next_state.0 as i64),
        mutable: false,
    });

    // Return Poll::Pending
    body.push(HirStatement::Return(Some(HirExpression::Call {
        func: Box::new(HirExpression::Variable("Poll::Pending".to_string())),
        args: vec![],
    })));

    MatchArm {
        pattern: state_id.0.to_string(),
        guard: None,
        body,
    }
}

/// Create a match arm for the final state
///
/// Generates something like:
/// ```rust,ignore
/// StateId(N) => {
///     let result = self.__result.take();
///     Poll::Ready(result)
/// }
/// ```
fn create_final_state_arm(state_machine: &StateMachine) -> MatchArm {
    let mut body = vec![];

    // Get the result
    body.push(HirStatement::Let {
        name: "__final_result".to_string(),
        ty: HirType::Option(Box::new(state_machine.return_type.clone())),
        init: HirExpression::MethodCall {
            receiver: Box::new(HirExpression::FieldAccess {
                object: Box::new(HirExpression::Variable("self".to_string())),
                field: "__result".to_string(),
            }),
            method: "take".to_string(),
            args: vec![],
        },
        mutable: false,
    });

    // Return Poll::Ready(result)
    body.push(HirStatement::Return(Some(HirExpression::Call {
        func: Box::new(HirExpression::Variable("Poll::Ready".to_string())),
        args: vec![HirExpression::Variable("__final_result".to_string())],
    })));

    MatchArm {
        pattern: (state_machine.num_states - 1).to_string(),
        guard: None,
        body,
    }
}

/// Create a match arm for the unreachable pattern
fn create_unreachable_pattern() -> MatchArm {
    MatchArm {
        pattern: "_".to_string(),
        guard: None,
        body: vec![HirStatement::Return(Some(HirExpression::Call {
            func: Box::new(HirExpression::Variable("unreachable".to_string())),
            args: vec![],
        }))],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_id_creation() {
        let start = StateId::start();
        assert_eq!(start.0, 0);
        assert!(start.is_start());
    }

    #[test]
    fn test_state_id_next() {
        let s0 = StateId::start();
        let s1 = s0.next();
        let s2 = s1.next();

        assert_eq!(s0.0, 0);
        assert_eq!(s1.0, 1);
        assert_eq!(s2.0, 2);
    }

    #[test]
    fn test_simple_state_machine_creation() {
        let sm = StateMachine::new(
            "test_func".to_string(),
            vec![("x".to_string(), HirType::Int32)],
            HirType::Int32,
            vec![],
            HashMap::new(),
        );

        assert_eq!(sm.func_name, "test_func");
        assert_eq!(sm.num_states, 1);
        assert_eq!(sm.params.len(), 1);
    }

    #[test]
    fn test_state_machine_with_awaits() {
        let mut await_points = vec![];
        await_points.push(AwaitPoint {
            id: 0,
            state_before: StateId::start(),
            state_after: StateId(1),
            expr: HirExpression::Variable("future1".to_string()),
            result_var: Some("result1".to_string()),
            output_type: HirType::Int32,
        });

        let sm = StateMachine::new(
            "test_func".to_string(),
            vec![],
            HirType::Int32,
            await_points,
            HashMap::new(),
        );

        assert_eq!(sm.num_states, 2); // 1 await + final state
        assert_eq!(sm.await_points.len(), 1);
    }

    #[test]
    fn test_captured_fields_includes_state() {
        let sm = StateMachine::new(
            "test".to_string(),
            vec![],
            HirType::Int32,
            vec![],
            HashMap::new(),
        );

        let fields = sm.captured_fields();

        // Should have: state, result
        assert!(fields.iter().any(|(name, _)| name == "__state"));
        assert!(fields.iter().any(|(name, _)| name == "__result"));
    }

    #[test]
    fn test_captured_fields_includes_params() {
        let sm = StateMachine::new(
            "test".to_string(),
            vec![
                ("param1".to_string(), HirType::Int32),
                ("param2".to_string(), HirType::String),
            ],
            HirType::Int32,
            vec![],
            HashMap::new(),
        );

        let fields = sm.captured_fields();

        // Should include all parameters
        assert!(fields.iter().any(|(name, _)| name == "param1"));
        assert!(fields.iter().any(|(name, _)| name == "param2"));
    }

    #[test]
    fn test_captured_fields_includes_local_vars() {
        let mut local_vars = HashMap::new();
        local_vars.insert("local1".to_string(), HirType::Int32);
        local_vars.insert("local2".to_string(), HirType::String);

        let sm = StateMachine::new(
            "test".to_string(),
            vec![],
            HirType::Int32,
            vec![],
            local_vars,
        );

        let fields = sm.captured_fields();

        // Should include local variables
        assert!(fields.iter().any(|(name, _)| name == "local1"));
        assert!(fields.iter().any(|(name, _)| name == "local2"));
    }

    #[test]
    fn test_generate_state_struct_creates_struct_item() {
        let sm = StateMachine::new(
            "my_func".to_string(),
            vec![("x".to_string(), HirType::Int32)],
            HirType::String,
            vec![],
            HashMap::new(),
        );

        let result = generate_state_struct(&sm);

        assert!(result.is_ok());
        if let Ok(HirItem::Struct { name, fields, .. }) = result {
            assert_eq!(name, "my_funcAsyncState");

            // Should have: state, param, result
            assert!(fields.iter().any(|(n, _)| n == "__state"));
            assert!(fields.iter().any(|(n, _)| n == "x"));
            assert!(fields.iter().any(|(n, _)| n == "__result"));
        } else {
            panic!("Expected Struct item");
        }
    }

    #[test]
    fn test_generate_poll_method_creates_function() {
        let sm = StateMachine::new(
            "test".to_string(),
            vec![],
            HirType::Int32,
            vec![],
            HashMap::new(),
        );

        let result = generate_poll_method(&sm);

        assert!(result.is_ok());
        if let Ok(HirItem::Function { name, params, .. }) = result {
            assert_eq!(name, "poll");
            assert_eq!(params.len(), 2); // self, cx
        } else {
            panic!("Expected HirFunction");
        }
    }
}
