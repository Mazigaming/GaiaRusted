//! # Phase 3: State Machine Code Generation for Async Functions
//!
//! Generates state machine implementations for async functions.
//!
//! ## Overview
//!
//! Async functions are transformed into state machines that can be
//! polled to progress execution:
//!
//! ```ignore
//! async fn fetch_data(url: &str) -> i32 {
//!     let resp = http_get(url).await;
//!     resp.len() as i32
//! }
//! ```
//!
//! Becomes:
//!
//! ```ignore
//! enum FetchDataState {
//!     Start { url: &str },
//!     AwaitingResponse { url: &str },
//!     Done,
//! }
//!
//! struct FetchDataFuture {
//!     state: FetchDataState,
//! }
//!
//! impl Future for FetchDataFuture {
//!     type Output = i32;
//!     fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<i32> {
//!         match &mut self.state {
//!             FetchDataState::Start { url } => {
//!                 // Call http_get, transition to AwaitingResponse
//!                 Poll::Pending
//!             }
//!             FetchDataState::AwaitingResponse { url } => {
//!                 // Check response status
//!                 if done { return Poll::Ready(len); }
//!                 Poll::Pending
//!             }
//!             FetchDataState::Done => unreachable!(),
//!         }
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::fmt;

/// Represents a single state in the state machine
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDefinition {
    pub name: String,
    pub index: usize,
    pub fields: Vec<(String, String)>, // (field_name, field_type)
    pub description: String,
}

impl StateDefinition {
    pub fn new(name: String, index: usize) -> Self {
        StateDefinition {
            name,
            index,
            fields: vec![],
            description: String::new(),
        }
    }

    pub fn add_field(&mut self, name: String, ty: String) {
        self.fields.push((name, ty));
    }

    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }
}

/// Represents a transition between states
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransition {
    pub from_state: usize,
    pub to_state: usize,
    pub condition: String,
    pub action: String,
}

impl StateTransition {
    pub fn new(
        from_state: usize,
        to_state: usize,
        condition: String,
        action: String,
    ) -> Self {
        StateTransition {
            from_state,
            to_state,
            condition,
            action,
        }
    }
}

/// Configuration for state machine generation
#[derive(Debug, Clone)]
pub struct StateMachineConfig {
    pub async_fn_name: String,
    pub output_type: String,
    pub input_params: Vec<(String, String)>, // (param_name, param_type)
    pub await_points: usize,
}

impl StateMachineConfig {
    pub fn new(async_fn_name: String, output_type: String) -> Self {
        StateMachineConfig {
            async_fn_name,
            output_type,
            input_params: vec![],
            await_points: 0,
        }
    }

    pub fn add_param(&mut self, name: String, ty: String) {
        self.input_params.push((name, ty));
    }

    pub fn set_await_points(&mut self, count: usize) {
        self.await_points = count;
    }
}

/// Generated state machine code
#[derive(Debug, Clone)]
pub struct GeneratedStateMachine {
    pub struct_name: String,
    pub enum_name: String,
    pub states: Vec<StateDefinition>,
    pub transitions: Vec<StateTransition>,
    pub generated_enum: String,
    pub generated_struct: String,
    pub generated_impl: String,
}

/// State machine code generator for async functions
pub struct StateMachineCodegen;

impl StateMachineCodegen {
    /// Generate the enum that represents all states
    pub fn generate_state_enum(
        struct_name: &str,
        states: &[StateDefinition],
    ) -> String {
        let mut code = String::new();
        let enum_name = format!("{}State", struct_name);

        code.push_str(&format!("enum {} {{\n", enum_name));

        for state in states {
            if state.fields.is_empty() {
                code.push_str(&format!("    {},\n", state.name));
            } else {
                code.push_str(&format!("    {} {{\n", state.name));
                for (field_name, field_type) in &state.fields {
                    code.push_str(&format!("        {}: {},\n", field_name, field_type));
                }
                code.push_str("    },\n");
            }
        }

        code.push_str("}\n");
        code
    }

    /// Generate the Future struct wrapper
    pub fn generate_future_struct(
        struct_name: &str,
        config: &StateMachineConfig,
    ) -> String {
        let mut code = String::new();
        let enum_name = format!("{}State", struct_name);

        code.push_str(&format!("struct {} {{\n", struct_name));
        code.push_str(&format!("    state: {},\n", enum_name));
        code.push_str("}\n\n");

        code.push_str(&format!("impl {} {{\n", struct_name));
        code.push_str(&format!("    fn new(", ));
        for (i, (param_name, param_type)) in config.input_params.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(&format!("{}: {}", param_name, param_type));
        }
        code.push_str(&format!(") -> Self {{\n"));
        code.push_str(&format!("        {} {{\n", struct_name));
        code.push_str(&format!("            state: {}::Start {{\n", enum_name));
        for (param_name, _) in &config.input_params {
            code.push_str(&format!("                {},\n", param_name));
        }
        code.push_str("            },\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n");

        code
    }

    /// Generate the Future trait implementation
    pub fn generate_future_impl(
        struct_name: &str,
        config: &StateMachineConfig,
        transitions: &[StateTransition],
    ) -> String {
        let mut code = String::new();
        let enum_name = format!("{}State", struct_name);

        code.push_str(&format!(
            "impl Future for {} {{\n",
            struct_name
        ));
        code.push_str(&format!("    type Output = {};\n\n", config.output_type));
        code.push_str(&format!("    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<{}> {{\n", config.output_type));
        code.push_str("        match &mut self.state {\n");

        // Generate match arms for each state
        for (i, transition) in transitions.iter().enumerate() {
            code.push_str(&format!("            // State {} -> {}\n", transition.from_state, transition.to_state));
            code.push_str(&format!("            {} => {{\n", format!("{}::State{}", enum_name, transition.from_state)));
            code.push_str("                // TODO: Implement state-specific logic\n");
            code.push_str("                Poll::Pending\n");
            code.push_str("            }\n");
        }

        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n");

        code
    }

    /// Generate complete state machine for an async function
    pub fn generate_state_machine(config: &StateMachineConfig) -> GeneratedStateMachine {
        let struct_name = format!("{}Future", config.async_fn_name);
        let enum_name = format!("{}State", struct_name);

        // Create states
        let mut states = vec![];

        // Start state with input parameters
        let mut start_state = StateDefinition::new("Start".to_string(), 0)
            .with_description("Initial state with function parameters".to_string());
        for (param_name, param_type) in &config.input_params {
            start_state.add_field(param_name.clone(), param_type.clone());
        }
        states.push(start_state);

        // Create await point states
        for i in 0..config.await_points {
            let state_name = format!("AwaitPoint{}", i);
            let state = StateDefinition::new(state_name, i + 1)
                .with_description(format!("Awaiting future at point {}", i));
            states.push(state);
        }

        // Done state
        let done_state = StateDefinition::new("Done".to_string(), config.await_points + 1)
            .with_description("State machine completed".to_string());
        states.push(done_state);

        // Create transitions
        let mut transitions = vec![];
        for i in 0..states.len().saturating_sub(1) {
            transitions.push(StateTransition::new(
                i,
                i + 1,
                format!("state == State::{}", states[i].name),
                format!("transition to state {}", i + 1),
            ));
        }

        // Generate code
        let generated_enum = Self::generate_state_enum(&struct_name, &states);
        let generated_struct = Self::generate_future_struct(&struct_name, config);
        let generated_impl = Self::generate_future_impl(&struct_name, config, &transitions);

        GeneratedStateMachine {
            struct_name,
            enum_name,
            states,
            transitions,
            generated_enum,
            generated_struct,
            generated_impl,
        }
    }

    /// Generate x86-64 assembly for polling a state machine
    pub fn generate_poll_assembly(state_machine: &GeneratedStateMachine) -> String {
        let mut asm = String::new();

        asm.push_str(&format!(
            "// Assembly for {}: poll implementation\n",
            state_machine.struct_name
        ));
        asm.push_str(".globl _poll_");
        asm.push_str(&state_machine.struct_name);
        asm.push_str("\n");
        asm.push_str("_poll_");
        asm.push_str(&state_machine.struct_name);
        asm.push_str(":\n");

        asm.push_str("    // Load state from future struct (rdi = &mut self)\n");
        asm.push_str("    mov rax, [rdi]        // Load current state\n");
        asm.push_str("    cmp rax, 0            // Check if state == Start\n");
        asm.push_str("    je .start_state\n");
        asm.push_str("    cmp rax, 1            // Check if state == Awaiting\n");
        asm.push_str("    je .await_state\n");
        asm.push_str("    cmp rax, 2            // Check if state == Done\n");
        asm.push_str("    je .done_state\n");
        asm.push_str("\n");
        asm.push_str(".start_state:\n");
        asm.push_str("    // Initialize and transition\n");
        asm.push_str("    mov qword [rdi], 1   // Set state to Awaiting\n");
        asm.push_str("    mov rax, 0           // Poll::Pending = 0\n");
        asm.push_str("    ret\n");
        asm.push_str("\n");
        asm.push_str(".await_state:\n");
        asm.push_str("    // Check if future is ready\n");
        asm.push_str("    mov rax, 0           // Poll::Pending = 0\n");
        asm.push_str("    ret\n");
        asm.push_str("\n");
        asm.push_str(".done_state:\n");
        asm.push_str("    mov rax, 1           // Poll::Ready = 1\n");
        asm.push_str("    ret\n");

        asm
    }
}

impl fmt::Display for GeneratedStateMachine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.struct_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_definition_creation() {
        let state = StateDefinition::new("Start".to_string(), 0);
        assert_eq!(state.name, "Start");
        assert_eq!(state.index, 0);
        assert_eq!(state.fields.len(), 0);
    }

    #[test]
    fn test_state_definition_add_field() {
        let mut state = StateDefinition::new("Start".to_string(), 0);
        state.add_field("url".to_string(), "&str".to_string());
        state.add_field("timeout".to_string(), "u32".to_string());

        assert_eq!(state.fields.len(), 2);
        assert_eq!(state.fields[0], ("url".to_string(), "&str".to_string()));
    }

    #[test]
    fn test_state_transition_creation() {
        let trans = StateTransition::new(
            0,
            1,
            "is_ready".to_string(),
            "continue".to_string(),
        );

        assert_eq!(trans.from_state, 0);
        assert_eq!(trans.to_state, 1);
        assert_eq!(trans.condition, "is_ready");
    }

    #[test]
    fn test_state_machine_config_creation() {
        let config = StateMachineConfig::new("fetch".to_string(), "i32".to_string());
        assert_eq!(config.async_fn_name, "fetch");
        assert_eq!(config.output_type, "i32");
        assert_eq!(config.input_params.len(), 0);
    }

    #[test]
    fn test_state_machine_config_add_param() {
        let mut config = StateMachineConfig::new("fetch".to_string(), "Result<Data>".to_string());
        config.add_param("url".to_string(), "&str".to_string());
        config.add_param("timeout".to_string(), "u32".to_string());

        assert_eq!(config.input_params.len(), 2);
    }

    #[test]
    fn test_generate_state_enum_simple() {
        let states = vec![
            StateDefinition::new("Start".to_string(), 0),
            StateDefinition::new("Done".to_string(), 1),
        ];

        let code = StateMachineCodegen::generate_state_enum("Fetch", &states);
        assert!(code.contains("enum FetchState"));
        assert!(code.contains("Start"));
        assert!(code.contains("Done"));
    }

    #[test]
    fn test_generate_state_enum_with_fields() {
        let mut state = StateDefinition::new("Start".to_string(), 0);
        state.add_field("url".to_string(), "&str".to_string());
        state.add_field("timeout".to_string(), "u32".to_string());

        let states = vec![state];

        let code = StateMachineCodegen::generate_state_enum("Fetch", &states);
        assert!(code.contains("Start"));
        assert!(code.contains("url: &str"));
        assert!(code.contains("timeout: u32"));
    }

    #[test]
    fn test_generate_future_struct() {
        let config = StateMachineConfig::new("fetch".to_string(), "i32".to_string());
        let code = StateMachineCodegen::generate_future_struct("FetchFuture", &config);

        assert!(code.contains("struct FetchFuture"));
        assert!(code.contains("state"));
        assert!(code.contains("impl FetchFuture"));
        assert!(code.contains("fn new"));
    }

    #[test]
    fn test_generate_future_impl() {
        let config = StateMachineConfig::new("fetch".to_string(), "i32".to_string());
        let transitions = vec![];

        let code = StateMachineCodegen::generate_future_impl("FetchFuture", &config, &transitions);
        assert!(code.contains("impl Future for FetchFuture"));
        assert!(code.contains("type Output = i32"));
        assert!(code.contains("fn poll"));
        assert!(code.contains("Pin<&mut Self>"));
    }

    #[test]
    fn test_generate_complete_state_machine() {
        let mut config = StateMachineConfig::new("fetch_data".to_string(), "i32".to_string());
        config.add_param("url".to_string(), "&str".to_string());
        config.set_await_points(1);

        let sm = StateMachineCodegen::generate_state_machine(&config);

        assert!(sm.struct_name.contains("Future"));
        assert_eq!(sm.struct_name, "fetch_dataFuture");
        assert!(sm.states.len() > 0);
        assert!(sm.generated_enum.contains("enum"));
        assert!(sm.generated_struct.contains("struct"));
        assert!(sm.generated_impl.contains("impl"));
    }

    #[test]
    fn test_generate_poll_assembly() {
        let mut config = StateMachineConfig::new("fetch".to_string(), "i32".to_string());
        config.set_await_points(1);

        let sm = StateMachineCodegen::generate_state_machine(&config);
        let asm = StateMachineCodegen::generate_poll_assembly(&sm);

        assert!(asm.contains("// Assembly for"));
        assert!(asm.contains(".globl"));
        assert!(asm.contains("_poll_"));
        assert!(asm.contains("mov"));
        assert!(asm.contains("cmp"));
        assert!(asm.contains(".start_state"));
        assert!(asm.contains(".done_state"));
    }

    #[test]
    fn test_state_machine_display() {
        let mut config = StateMachineConfig::new("test".to_string(), "i32".to_string());
        config.set_await_points(1);

        let sm = StateMachineCodegen::generate_state_machine(&config);
        let display = format!("{}", sm);

        assert!(display.contains("testFuture"));
    }

    #[test]
    fn test_state_machine_multiple_await_points() {
        let mut config = StateMachineConfig::new("complex".to_string(), "String".to_string());
        config.add_param("x".to_string(), "i32".to_string());
        config.add_param("y".to_string(), "i32".to_string());
        config.set_await_points(3);

        let sm = StateMachineCodegen::generate_state_machine(&config);

        // Should have Start, AwaitPoint0, AwaitPoint1, AwaitPoint2, Done = 5 states
        assert_eq!(sm.states.len(), 5);

        // Verify state names
        assert_eq!(sm.states[0].name, "Start");
        assert_eq!(sm.states[1].name, "AwaitPoint0");
        assert_eq!(sm.states[2].name, "AwaitPoint1");
        assert_eq!(sm.states[3].name, "AwaitPoint2");
        assert_eq!(sm.states[4].name, "Done");
    }

    #[test]
    fn test_generated_state_machine_transitions() {
        let mut config = StateMachineConfig::new("test".to_string(), "i32".to_string());
        config.set_await_points(2);

        let sm = StateMachineCodegen::generate_state_machine(&config);

        // Should have transitions for each state -> next
        assert!(sm.transitions.len() >= 2);

        // Check transition structure
        assert_eq!(sm.transitions[0].from_state, 0);
        assert_eq!(sm.transitions[0].to_state, 1);
    }
}
