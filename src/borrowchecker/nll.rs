//! Non-Lexical Lifetimes (NLL) - Flow-Sensitive Borrow Analysis
//!
//! Modern Rust uses Non-Lexical Lifetimes where borrows end at their last use,
//! not at scope exit. This enables patterns like:
//!
//! ```rust
//! let mut x = 5;
//! let r = &x;
//! println!("{}", r);  // r's borrow ends here
//! let r2 = &mut x;    // ✓ valid! r not used after this
//! ```

use crate::lowering::{HirExpression, HirStatement};
use std::collections::HashMap;

/// A position in the control flow (statement or expression index)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Location {
    pub block: usize,
    pub index: usize,
}

impl Location {
    pub fn new(block: usize, index: usize) -> Self {
        Location { block, index }
    }
}

/// Represents a borrow with its usage range
#[derive(Debug, Clone)]
pub struct BorrowInfo {
    /// Variable being borrowed
    pub variable: String,
    /// Location where borrow is created
    pub created_at: Location,
    /// Is it mutable?
    pub is_mutable: bool,
    /// Last location where borrow is used
    pub last_used_at: Option<Location>,
}

/// Flow-sensitive borrow tracker
pub struct BorrowTracker {
    /// Active borrows: variable -> BorrowInfo
    active_borrows: HashMap<String, Vec<BorrowInfo>>,
    /// Locations where each variable is used
    usage_map: HashMap<String, Vec<Location>>,
}

impl BorrowTracker {
    pub fn new() -> Self {
        BorrowTracker {
            active_borrows: HashMap::new(),
            usage_map: HashMap::new(),
        }
    }

    /// Record a borrow creation
    pub fn create_borrow(
        &mut self,
        variable: String,
        location: Location,
        is_mutable: bool,
    ) -> BorrowId {
        let borrow = BorrowInfo {
            variable: variable.clone(),
            created_at: location,
            is_mutable,
            last_used_at: None,
        };

        self.active_borrows
            .entry(variable)
            .or_insert_with(Vec::new)
            .push(borrow);

        // Return a simple ID based on position
        BorrowId(self.active_borrows.len())
    }

    /// Record a variable usage
    pub fn record_usage(&mut self, variable: &str, location: Location) {
        self.usage_map
            .entry(variable.to_string())
            .or_insert_with(Vec::new)
            .push(location);

        // Update last_used_at for all active borrows of this variable
        if let Some(borrows) = self.active_borrows.get_mut(variable) {
            for borrow in borrows {
                if borrow.last_used_at.is_none() || borrow.last_used_at < Some(location) {
                    borrow.last_used_at = Some(location);
                }
            }
        }
    }

    /// Determine if a borrow is still live at a location
    pub fn is_borrow_live(&self, variable: &str, location: Location) -> bool {
        if let Some(borrows) = self.active_borrows.get(variable) {
            for borrow in borrows {
                if borrow.created_at <= location {
                    // Borrow started before or at this location
                    if let Some(last_used) = borrow.last_used_at {
                        if last_used >= location {
                            // Borrow is still live at this location
                            return true;
                        }
                    } else {
                        // Never used, still live
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a mutable borrow can be created at this location
    pub fn can_create_mutable_borrow(
        &self,
        variable: &str,
        location: Location,
    ) -> Result<(), String> {
        if let Some(borrows) = self.active_borrows.get(variable) {
            for borrow in borrows {
                if self.is_borrow_live(variable, location) {
                    if borrow.is_mutable {
                        return Err(format!(
                            "Cannot create mutable borrow of {} with existing mutable borrow",
                            variable
                        ));
                    } else {
                        return Err(format!(
                            "Cannot create mutable borrow of {} with existing immutable borrow",
                            variable
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if an immutable borrow can be created at this location
    pub fn can_create_immutable_borrow(
        &self,
        variable: &str,
        location: Location,
    ) -> Result<(), String> {
        if let Some(borrows) = self.active_borrows.get(variable) {
            for borrow in borrows {
                if self.is_borrow_live(variable, location) && borrow.is_mutable {
                    return Err(format!(
                        "Cannot create immutable borrow of {} while mutable borrow is active",
                        variable
                    ));
                }
            }
        }
        Ok(())
    }

    /// Get summary of all active borrows
    pub fn active_borrows(&self) -> &HashMap<String, Vec<BorrowInfo>> {
        &self.active_borrows
    }
}

impl Default for BorrowTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple borrow ID for tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BorrowId(usize);

/// Tracks which variables are used where in the program
pub struct UsageAnalyzer {
    /// For each variable, record all locations where it's used
    usage_locations: HashMap<String, Vec<Location>>,
}

impl UsageAnalyzer {
    pub fn new() -> Self {
        UsageAnalyzer {
            usage_locations: HashMap::new(),
        }
    }

    /// Analyze statements to find variable usages
    pub fn analyze_statements(&mut self, stmts: &[HirStatement], block_id: usize) {
        for (idx, stmt) in stmts.iter().enumerate() {
            self.analyze_statement(stmt, Location::new(block_id, idx));
        }
    }

    fn analyze_statement(&mut self, stmt: &HirStatement, location: Location) {
        match stmt {
            HirStatement::Let { name, init, .. } => {
                self.analyze_expression(init, location);
                self.record_usage(name, location);
            }
            HirStatement::Expression(expr) => {
                self.analyze_expression(expr, location);
            }
            HirStatement::Return(Some(expr)) => {
                self.analyze_expression(expr, location);
            }
            HirStatement::For { var, iter, body } => {
                self.analyze_expression(iter, location);
                self.record_usage(var, location);
                self.analyze_statements(body, location.block);
            }
            HirStatement::While { condition, body } => {
                self.analyze_expression(condition, location);
                self.analyze_statements(body, location.block);
            }
            HirStatement::If {
                condition,
                then_body,
                else_body,
            } => {
                self.analyze_expression(condition, location);
                self.analyze_statements(then_body, location.block);
                if let Some(else_stmts) = else_body {
                    self.analyze_statements(else_stmts, location.block);
                }
            }
            _ => {}
        }
    }

    fn analyze_expression(&mut self, expr: &HirExpression, location: Location) {
        match expr {
            HirExpression::Variable(name) => {
                self.record_usage(name, location);
            }
            HirExpression::BinaryOp { left, right, .. } => {
                self.analyze_expression(left, location);
                self.analyze_expression(right, location);
            }
            HirExpression::UnaryOp { operand, .. } => {
                self.analyze_expression(operand, location);
            }
            HirExpression::Call { func, args } => {
                self.analyze_expression(func, location);
                for arg in args {
                    self.analyze_expression(arg, location);
                }
            }
            _ => {}
        }
    }

    fn record_usage(&mut self, var: &str, location: Location) {
        self.usage_locations
            .entry(var.to_string())
            .or_insert_with(Vec::new)
            .push(location);
    }

    /// Get all usage locations for a variable
    pub fn get_usages(&self, var: &str) -> Option<&[Location]> {
        self.usage_locations.get(var).map(|v| v.as_slice())
    }
}

impl Default for UsageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_borrow_tracker_creation() {
        let tracker = BorrowTracker::new();
        assert_eq!(tracker.active_borrows.len(), 0);
    }

    #[test]
    fn test_create_borrow() {
        let mut tracker = BorrowTracker::new();
        let loc = Location::new(0, 0);
        let bid = tracker.create_borrow("x".to_string(), loc, false);
        assert_eq!(tracker.active_borrows.len(), 1);
        assert_ne!(bid.0, 0); // Should have some ID
    }

    #[test]
    fn test_borrow_liveness() {
        let mut tracker = BorrowTracker::new();
        let loc_create = Location::new(0, 0);
        let loc_use = Location::new(0, 1);
        let loc_after = Location::new(0, 2);

        tracker.create_borrow("x".to_string(), loc_create, false);
        tracker.record_usage("x", loc_use);

        // Borrow should be live at use point
        assert!(tracker.is_borrow_live("x", loc_use));

        // Borrow should be dead after last use
        assert!(!tracker.is_borrow_live("x", loc_after));
    }

    #[test]
    fn test_immutable_then_mutable_conflict() {
        let mut tracker = BorrowTracker::new();
        let loc1 = Location::new(0, 0);
        let loc2 = Location::new(0, 1);
        let loc3 = Location::new(0, 2);

        tracker.create_borrow("x".to_string(), loc1, false);
        tracker.record_usage("x", loc2);

        // Cannot create mutable borrow while immutable is live
        let result = tracker.can_create_mutable_borrow("x", loc2);
        assert!(result.is_err());

        // But can create after last use
        let result2 = tracker.can_create_mutable_borrow("x", loc3);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_mutable_borrow_conflict() {
        let mut tracker = BorrowTracker::new();
        let loc1 = Location::new(0, 0);
        let loc2 = Location::new(0, 1);

        tracker.create_borrow("x".to_string(), loc1, true);
        tracker.record_usage("x", loc2);

        // Cannot create another mutable borrow while first is live
        let result = tracker.can_create_mutable_borrow("x", loc2);
        assert!(result.is_err());
    }

    #[test]
    fn test_usage_analyzer_creation() {
        let analyzer = UsageAnalyzer::new();
        assert_eq!(analyzer.usage_locations.len(), 0);
    }
}
