//! # Loop Variable Ownership Tracking
//!
//! Tracks ownership and borrowing of loop variables in for-in loops.
//! This module enforces proper ownership semantics for iteration.
//!
//! # Examples
//!
//! ```ignore
//! use gaiarusted::borrowchecker::loop_ownership::{LoopOwnershipTracker, LoopVariableInfo};
//!
//! let mut tracker = LoopOwnershipTracker::new();
//! tracker.enter_loop("outer");
//! tracker.bind_loop_variable("outer", "item", "i32", false)?;
//! // ... loop body ...
//! tracker.exit_loop("outer")?;
//! ```

use std::collections::{HashMap, HashSet};

/// Configuration for loop ownership tracking
#[derive(Debug, Clone)]
pub struct LoopOwnershipConfig {
    /// Enable strict loop variable tracking
    pub enable_strict: bool,
    /// Maximum loop nesting depth
    pub max_nesting_depth: usize,
    /// Allow shadowing of outer loop variables
    pub allow_shadowing: bool,
}

impl Default for LoopOwnershipConfig {
    fn default() -> Self {
        LoopOwnershipConfig {
            enable_strict: true,
            max_nesting_depth: 16,
            allow_shadowing: false,
        }
    }
}

/// Ownership state of a loop variable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OwnershipState {
    /// Variable is owned by the current scope
    Owned,
    /// Variable is borrowed immutably
    BorrowedImmut,
    /// Variable is borrowed mutably
    BorrowedMut,
    /// Variable has moved (no longer accessible)
    Moved,
}

impl OwnershipState {
    /// Check if variable can be read
    pub fn can_read(&self) -> bool {
        matches!(self, OwnershipState::Owned | OwnershipState::BorrowedImmut)
    }

    /// Check if variable can be written
    pub fn can_write(&self) -> bool {
        matches!(self, OwnershipState::Owned | OwnershipState::BorrowedMut)
    }

    /// Check if variable is accessible
    pub fn is_accessible(&self) -> bool {
        !matches!(self, OwnershipState::Moved)
    }
}

/// Information about a loop variable
#[derive(Debug, Clone)]
pub struct LoopVariableInfo {
    /// Variable name
    pub name: String,
    /// Variable type
    pub var_type: String,
    /// Is the variable mutable
    pub is_mutable: bool,
    /// Current ownership state
    pub ownership: OwnershipState,
    /// Whether variable was defined before loop
    pub pre_loop_defined: bool,
    /// Last access type (read/write)
    pub last_access: Option<AccessType>,
}

/// Type of access to a variable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Read,
    Write,
    Move,
}

impl AccessType {
    /// Get string representation
    pub fn as_str(&self) -> &str {
        match self {
            AccessType::Read => "read",
            AccessType::Write => "write",
            AccessType::Move => "move",
        }
    }
}

/// Information about a loop scope
#[derive(Debug, Clone)]
pub struct LoopScope {
    /// Loop identifier
    pub id: String,
    /// Loop variable
    pub variable: String,
    /// Variables in this scope
    pub variables: HashMap<String, LoopVariableInfo>,
    /// Depth level (0 = outermost)
    pub depth: usize,
    /// Iterator type
    pub iter_type: String,
}

/// Error in loop ownership tracking
#[derive(Debug, Clone)]
pub struct LoopOwnershipError {
    pub loop_id: String,
    pub variable: String,
    pub reason: String,
}

impl std::fmt::Display for LoopOwnershipError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Loop '{}': {}: {}",
            self.loop_id, self.variable, self.reason
        )
    }
}

/// Main loop ownership tracker
pub struct LoopOwnershipTracker {
    config: LoopOwnershipConfig,
    scopes: Vec<LoopScope>,
    errors: Vec<LoopOwnershipError>,
}

impl LoopOwnershipTracker {
    /// Create new tracker
    pub fn new() -> Self {
        Self::with_config(LoopOwnershipConfig::default())
    }

    /// Create with config
    pub fn with_config(config: LoopOwnershipConfig) -> Self {
        LoopOwnershipTracker {
            config,
            scopes: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Enter a new loop scope
    pub fn enter_loop(&mut self, loop_id: &str, iter_type: &str) -> Result<(), String> {
        if loop_id.is_empty() {
            return Err("Loop ID cannot be empty".to_string());
        }

        if self.scopes.len() >= self.config.max_nesting_depth {
            return Err(format!(
                "Maximum loop nesting depth {} exceeded",
                self.config.max_nesting_depth
            ));
        }

        let scope = LoopScope {
            id: loop_id.to_string(),
            variable: String::new(),
            variables: HashMap::new(),
            depth: self.scopes.len(),
            iter_type: iter_type.to_string(),
        };

        self.scopes.push(scope);
        Ok(())
    }

    /// Exit the current loop scope
    pub fn exit_loop(&mut self, loop_id: &str) -> Result<(), String> {
        if self.scopes.is_empty() {
            return Err("No active loop scope".to_string());
        }

        let current = self.scopes.last().ok_or("No active scope")?;
        if current.id != loop_id {
            return Err(format!(
                "Mismatched loop exit: expected '{}', got '{}'",
                current.id, loop_id
            ));
        }

        self.scopes.pop();
        Ok(())
    }

    /// Bind a loop variable
    pub fn bind_loop_variable(
        &mut self,
        loop_id: &str,
        var_name: &str,
        var_type: &str,
        is_mutable: bool,
    ) -> Result<(), String> {
        if var_name.is_empty() || var_type.is_empty() {
            return Err("Variable name and type cannot be empty".to_string());
        }

        // Check for shadowing before getting mutable access
        if !self.config.allow_shadowing && self.variable_in_outer_scope(var_name) {
            return Err(format!(
                "Loop variable '{}' shadows an outer scope variable",
                var_name
            ));
        }

        let scope = self
            .scopes
            .iter_mut()
            .rev()
            .find(|s| s.id == loop_id)
            .ok_or_else(|| format!("Loop scope {} not found", loop_id))?;

        let info = LoopVariableInfo {
            name: var_name.to_string(),
            var_type: var_type.to_string(),
            is_mutable,
            ownership: OwnershipState::Owned,
            pre_loop_defined: false,
            last_access: None,
        };

        scope.variables.insert(var_name.to_string(), info);
        scope.variable = var_name.to_string();

        Ok(())
    }

    /// Access a loop variable (read)
    pub fn access_read(&mut self, loop_id: &str, var_name: &str) -> Result<(), String> {
        self.access_variable(loop_id, var_name, AccessType::Read)
    }

    /// Access a loop variable (write)
    pub fn access_write(&mut self, loop_id: &str, var_name: &str) -> Result<(), String> {
        self.access_variable(loop_id, var_name, AccessType::Write)
    }

    /// Access a loop variable (move)
    pub fn access_move(&mut self, loop_id: &str, var_name: &str) -> Result<(), String> {
        self.access_variable(loop_id, var_name, AccessType::Move)
    }

    /// Internal: Access a variable with specified access type
    fn access_variable(
        &mut self,
        loop_id: &str,
        var_name: &str,
        access_type: AccessType,
    ) -> Result<(), String> {
        let scope = self
            .scopes
            .iter_mut()
            .rev()
            .find(|s| s.id == loop_id)
            .ok_or_else(|| format!("Loop scope {} not found", loop_id))?;

        let var_info = scope.variables.get_mut(var_name).ok_or_else(|| {
            format!("Variable '{}' not found in loop scope '{}'", var_name, loop_id)
        })?;

        // Check accessibility
        if !var_info.ownership.is_accessible() {
            return Err(format!(
                "Variable '{}' has been moved and is no longer accessible",
                var_name
            ));
        }

        // Check readability for reads
        if matches!(access_type, AccessType::Read) && !var_info.ownership.can_read() {
            return Err(format!("Cannot read borrowed variable '{}'", var_name));
        }

        // Check writeability for writes
        if matches!(access_type, AccessType::Write) {
            if !var_info.is_mutable {
                return Err(format!("Cannot write to immutable variable '{}'", var_name));
            }
            if !var_info.ownership.can_write() {
                return Err(format!(
                    "Cannot write to variable '{}' with current borrow state",
                    var_name
                ));
            }
        }

        // Update last access
        var_info.last_access = Some(access_type);

        // Handle moves
        if matches!(access_type, AccessType::Move) {
            var_info.ownership = OwnershipState::Moved;
        }

        Ok(())
    }

    /// Check if variable exists in outer scope
    fn variable_in_outer_scope(&self, var_name: &str) -> bool {
        let current_depth = self.scopes.len();
        self.scopes.iter().any(|s| {
            s.depth < current_depth && s.variables.contains_key(var_name)
        })
    }

    /// Get current scope
    pub fn current_scope(&self) -> Option<&LoopScope> {
        self.scopes.last()
    }

    /// Get variable info
    pub fn get_variable_info(
        &self,
        loop_id: &str,
        var_name: &str,
    ) -> Option<LoopVariableInfo> {
        self.scopes
            .iter()
            .rev()
            .find(|s| s.id == loop_id)
            .and_then(|s| s.variables.get(var_name).cloned())
    }

    /// Check if variable is accessible
    pub fn is_accessible(&self, loop_id: &str, var_name: &str) -> bool {
        self.get_variable_info(loop_id, var_name)
            .map(|info| info.ownership.is_accessible())
            .unwrap_or(false)
    }

    /// Generate tracking report
    pub fn generate_report(&self) -> LoopOwnershipReport {
        let mut total_scopes = 0;
        let mut total_variables = 0;
        let mut moved_variables = 0;

        for scope in &self.scopes {
            total_scopes += 1;
            for var_info in scope.variables.values() {
                total_variables += 1;
                if !var_info.ownership.is_accessible() {
                    moved_variables += 1;
                }
            }
        }

        LoopOwnershipReport {
            total_scopes,
            total_variables,
            moved_variables,
            max_depth: self.scopes.iter().map(|s| s.depth).max().unwrap_or(0),
            errors: self.errors.clone(),
        }
    }

    /// Add error
    pub fn add_error(&mut self, loop_id: String, variable: String, reason: String) {
        self.errors.push(LoopOwnershipError {
            loop_id,
            variable,
            reason,
        });
    }
}

impl Default for LoopOwnershipTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Report of loop ownership tracking
#[derive(Debug, Clone)]
pub struct LoopOwnershipReport {
    /// Total loop scopes tracked
    pub total_scopes: usize,
    /// Total variables tracked
    pub total_variables: usize,
    /// Variables that have been moved
    pub moved_variables: usize,
    /// Maximum nesting depth
    pub max_depth: usize,
    /// Errors found
    pub errors: Vec<LoopOwnershipError>,
}

impl LoopOwnershipReport {
    /// Check if all variables are properly managed
    pub fn all_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        format!(
            "Loop ownership: {} scopes, {} variables ({} moved), max depth: {}",
            self.total_scopes, self.total_variables, self.moved_variables, self.max_depth
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tracker() {
        let tracker = LoopOwnershipTracker::new();
        assert_eq!(tracker.scopes.len(), 0);
    }

    #[test]
    fn test_enter_loop() {
        let mut tracker = LoopOwnershipTracker::new();
        let result = tracker.enter_loop("loop1", "Vec<i32>");
        assert!(result.is_ok());
        assert_eq!(tracker.scopes.len(), 1);
    }

    #[test]
    fn test_exit_loop() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("loop1", "Vec<i32>").ok();
        let result = tracker.exit_loop("loop1");
        assert!(result.is_ok());
        assert_eq!(tracker.scopes.len(), 0);
    }

    #[test]
    fn test_bind_loop_variable() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("loop1", "Vec<i32>").ok();
        let result = tracker.bind_loop_variable("loop1", "item", "i32", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_access_read() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("loop1", "Vec<i32>").ok();
        tracker.bind_loop_variable("loop1", "item", "i32", false).ok();
        let result = tracker.access_read("loop1", "item");
        assert!(result.is_ok());
    }

    #[test]
    fn test_access_write_immutable_fails() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("loop1", "Vec<i32>").ok();
        tracker.bind_loop_variable("loop1", "item", "i32", false).ok();
        let result = tracker.access_write("loop1", "item");
        assert!(result.is_err());
    }

    #[test]
    fn test_access_write_mutable() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("loop1", "Vec<i32>").ok();
        tracker.bind_loop_variable("loop1", "item", "i32", true).ok();
        let result = tracker.access_write("loop1", "item");
        assert!(result.is_ok());
    }

    #[test]
    fn test_access_moved_variable_fails() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("loop1", "Vec<i32>").ok();
        tracker.bind_loop_variable("loop1", "item", "i32", false).ok();
        tracker.access_move("loop1", "item").ok();
        let result = tracker.access_read("loop1", "item");
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_loops() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("outer", "Vec<Vec<i32>>").ok();
        tracker.bind_loop_variable("outer", "row", "Vec<i32>", false).ok();
        tracker.enter_loop("inner", "Vec<i32>").ok();
        tracker.bind_loop_variable("inner", "item", "i32", false).ok();

        assert_eq!(tracker.scopes.len(), 2);
        assert_eq!(tracker.current_scope().unwrap().id, "inner");
    }

    #[test]
    fn test_nesting_depth_limit() {
        let config = LoopOwnershipConfig {
            max_nesting_depth: 2,
            ..Default::default()
        };
        let mut tracker = LoopOwnershipTracker::with_config(config);
        tracker.enter_loop("loop1", "Vec").ok();
        tracker.enter_loop("loop2", "Vec").ok();
        let result = tracker.enter_loop("loop3", "Vec");
        assert!(result.is_err());
    }

    #[test]
    fn test_shadowing_prevention() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("loop1", "Vec<i32>").ok();
        tracker.bind_loop_variable("loop1", "x", "i32", false).ok();
        tracker.enter_loop("loop2", "Vec<i32>").ok();
        let result = tracker.bind_loop_variable("loop2", "x", "i32", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_variable_info() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("loop1", "Vec<i32>").ok();
        tracker.bind_loop_variable("loop1", "item", "i32", false).ok();

        let info = tracker.get_variable_info("loop1", "item");
        assert!(info.is_some());
        assert_eq!(info.unwrap().var_type, "i32");
    }

    #[test]
    fn test_is_accessible() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("loop1", "Vec<i32>").ok();
        tracker.bind_loop_variable("loop1", "item", "i32", false).ok();

        assert!(tracker.is_accessible("loop1", "item"));

        tracker.access_move("loop1", "item").ok();
        assert!(!tracker.is_accessible("loop1", "item"));
    }

    #[test]
    fn test_generate_report() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("loop1", "Vec<i32>").ok();
        tracker.bind_loop_variable("loop1", "item", "i32", false).ok();

        let report = tracker.generate_report();
        assert_eq!(report.total_scopes, 1);
        assert_eq!(report.total_variables, 1);
    }

    #[test]
    fn test_multiple_variables() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("loop1", "Vec<(i32, i32)>").ok();
        tracker.bind_loop_variable("loop1", "x", "i32", false).ok();
        tracker.bind_loop_variable("loop1", "y", "i32", true).ok();

        assert!(tracker.is_accessible("loop1", "x"));
        assert!(tracker.is_accessible("loop1", "y"));
    }

    #[test]
    fn test_ownership_state_operations() {
        let mut state = OwnershipState::Owned;
        assert!(state.can_read());
        assert!(state.can_write());
        assert!(state.is_accessible());

        state = OwnershipState::Moved;
        assert!(!state.is_accessible());
    }

    #[test]
    fn test_access_type_to_string() {
        assert_eq!(AccessType::Read.as_str(), "read");
        assert_eq!(AccessType::Write.as_str(), "write");
        assert_eq!(AccessType::Move.as_str(), "move");
    }

    #[test]
    fn test_tracker_with_custom_config() {
        let config = LoopOwnershipConfig {
            enable_strict: false,
            max_nesting_depth: 32,
            allow_shadowing: true,
        };
        let mut tracker = LoopOwnershipTracker::with_config(config);
        tracker.enter_loop("loop1", "Vec").ok();
        tracker.bind_loop_variable("loop1", "x", "i32", false).ok();
        tracker.enter_loop("loop2", "Vec").ok();
        // This should succeed with allow_shadowing=true
        let result = tracker.bind_loop_variable("loop2", "x", "i32", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mismatch_loop_exit() {
        let mut tracker = LoopOwnershipTracker::new();
        tracker.enter_loop("loop1", "Vec").ok();
        let result = tracker.exit_loop("loop2");
        assert!(result.is_err());
    }
}
