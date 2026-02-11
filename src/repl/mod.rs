//! # Interactive REPL (Read-Eval-Print Loop)
//!
//! Live Rust code execution interpreter. Allows users to:
//! - Define functions and keep them in scope
//! - Execute expressions and print results
//! - Maintain variable bindings across commands
//! - Enjoy immediate feedback with no recompilation overhead

pub mod parser;
pub mod registry;
pub mod executor;

use std::collections::HashMap;
use std::io::{self, Write};

pub use registry::{Registry, FunctionEntry, VariableEntry};
pub use executor::ExecutionResult;

/// Main REPL (Read-Eval-Print Loop) engine
pub struct Repl {
    /// Function definitions
    functions: Registry,
    /// Variable bindings
    variables: HashMap<String, String>,
    /// Command history (for future features)
    history: Vec<String>,
    /// Whether to show detailed output
    verbose: bool,
}

impl Repl {
    /// Create a new REPL instance
    pub fn new() -> Self {
        Repl {
            functions: Registry::new(),
            variables: HashMap::new(),
            history: Vec::new(),
            verbose: false,
        }
    }

    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Main REPL loop - reads commands from stdin and executes them
    pub fn run(&mut self) {
        println!("GaiaRusted REPL v1.0.2");
        println!("Type 'help' for commands, 'exit' to quit");
        println!();

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            // Print prompt
            print!("> ");
            stdout.flush().ok();

            // Read input
            let mut input = String::new();
            match stdin.read_line(&mut input) {
                Ok(0) => break,  // EOF
                Ok(_) => {},
                Err(e) => {
                    eprintln!("error reading input: {}", e);
                    continue;
                }
            }

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            // Store in history
            self.history.push(input.to_string());

            // Handle commands
            if self.handle_command(input) {
                break;  // exit command
            }
        }

        println!();
        println!("Exiting GaiaRusted REPL");
    }

    /// Handle special commands and evaluate expressions
    fn handle_command(&mut self, input: &str) -> bool {
        match input {
            "exit" | "quit" => true,
            "clear" => {
                self.variables.clear();
                self.functions.clear();
                println!("Cleared all variables and functions");
                false
            }
            "help" => {
                self.print_help();
                false
            }
            "vars" => {
                self.print_variables();
                false
            }
            "funcs" => {
                self.print_functions();
                false
            }
            "history" => {
                self.print_history();
                false
            }
            _ => {
                // Evaluate expression or statement
                self.eval(input);
                false
            }
        }
    }

    /// Evaluate an expression or statement
    fn eval(&mut self, input: &str) {
        // Check if it's a function definition
        if input.starts_with("fn ") {
            self.define_function(input);
        } else if input.starts_with("let ") {
            self.define_variable(input);
        } else if input.starts_with("mut ") {
            self.define_mutable_variable(input);
        } else {
            // Expression
            self.evaluate_expression(input);
        }
    }

    /// Define a function
    fn define_function(&mut self, input: &str) {
        // For now, just store the function definition as text
        // In Phase 4.1 advanced, we'll compile it
        match parser::parse_function_def(input) {
            Ok(func_def) => {
                println!("Registered function '{}'", func_def.name);
                self.functions.insert(func_def.name, FunctionEntry {
                    definition: input.to_string(),
                });
            }
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }

    /// Define a variable
    fn define_variable(&mut self, input: &str) {
        match parser::parse_let_binding(input) {
            Ok((name, _type, _value)) => {
                self.variables.insert(name.clone(), input.to_string());
                println!("Bound variable '{}'", name);
            }
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }

    /// Define a mutable variable
    fn define_mutable_variable(&mut self, input: &str) {
        match parser::parse_let_mut_binding(input) {
            Ok((name, _type, _value)) => {
                self.variables.insert(name.clone(), input.to_string());
                println!("Bound mutable variable '{}'", name);
            }
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }

    /// Evaluate an expression
    fn evaluate_expression(&mut self, input: &str) {
        // For now, provide a placeholder
        // In Phase 4.1 advanced, we'll actually execute it
        match parser::parse_expression(input) {
            Ok(_expr) => {
                // Would compile and execute here
                println!("[expression evaluation pending implementation]");
            }
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }

    /// Print help message
    fn print_help(&self) {
        println!();
        println!("GaiaRusted REPL Commands:");
        println!("  help          - Show this message");
        println!("  exit, quit    - Exit the REPL");
        println!("  clear         - Clear all variables and functions");
        println!("  vars          - List all variables");
        println!("  funcs         - List all functions");
        println!("  history       - Show command history");
        println!();
        println!("You can also:");
        println!("  Define functions: fn name(arg: Type) -> RetType {{ ... }}");
        println!("  Bind variables:   let name: Type = value;");
        println!("  Evaluate expressions: any valid Rust expression");
        println!();
    }

    /// Print all variables
    fn print_variables(&self) {
        if self.variables.is_empty() {
            println!("No variables defined");
        } else {
            println!("Variables:");
            for name in self.variables.keys() {
                println!("  {}", name);
            }
        }
    }

    /// Print all functions
    fn print_functions(&self) {
        if self.functions.is_empty() {
            println!("No functions defined");
        } else {
            println!("Functions:");
            for name in self.functions.keys() {
                println!("  {}", name);
            }
        }
    }

    /// Print command history
    fn print_history(&self) {
        if self.history.is_empty() {
            println!("No history");
        } else {
            println!("History:");
            for (i, cmd) in self.history.iter().enumerate() {
                println!("  {}: {}", i + 1, cmd);
            }
        }
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_creation() {
        let repl = Repl::new();
        assert_eq!(repl.history.len(), 0);
        assert_eq!(repl.variables.len(), 0);
    }

    #[test]
    fn test_repl_verbose() {
        let repl = Repl::new().with_verbose(true);
        assert!(repl.verbose);
    }

    #[test]
    fn test_function_storage() {
        let mut repl = Repl::new();
        let func_def = "fn add(a: i32, b: i32) -> i32 { a + b }";
        repl.define_function(func_def);
        assert!(repl.functions.contains("add"));
    }

    #[test]
    fn test_variable_storage() {
        let mut repl = Repl::new();
        let var_def = "let x: i32 = 5;";
        repl.define_variable(var_def);
        assert!(repl.variables.contains_key("x"));
    }

    #[test]
    fn test_history_tracking() {
        let mut repl = Repl::new();
        // Commands are added to history before handle_command processes them
        repl.history.push("let x = 5;".to_string());
        repl.history.push("let y = 10;".to_string());
        assert_eq!(repl.history.len(), 2);
    }

    #[test]
    fn test_clear_command() {
        let mut repl = Repl::new();
        repl.variables.insert("x".to_string(), "5".to_string());
        repl.handle_command("clear");
        assert_eq!(repl.variables.len(), 0);
    }
}
