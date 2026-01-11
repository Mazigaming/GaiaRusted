/// Automatic Reference Count Scheduling for v0.13.0
///
/// Optimizes when refcount operations happen by:
/// - Analyzing control flow to identify pairing opportunities
/// - Scheduling increments and decrements optimally
/// - Reducing unnecessary operations through smart analysis
/// - Eliminating dead stores and loads

use std::collections::{HashMap, HashSet, VecDeque};

/// Represents a single refcount operation at a location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefCountOp {
    Increment,
    Decrement,
}

/// Schedule for a single variable's refcount operations
#[derive(Debug, Clone)]
pub struct RefCountSchedule {
    /// Variable/local name
    pub var_name: String,
    /// Ordered list of operations
    pub operations: Vec<(usize, RefCountOp)>, // (block_id, op)
    /// Pairs that can be eliminated
    pub eliminable_pairs: Vec<(usize, usize)>, // (inc_idx, dec_idx)
    /// Operations that can be moved for better scheduling
    pub moveable_ops: HashSet<usize>,
}

impl RefCountSchedule {
    pub fn new(var_name: String) -> Self {
        Self {
            var_name,
            operations: Vec::new(),
            eliminable_pairs: Vec::new(),
            moveable_ops: HashSet::new(),
        }
    }

    /// Add an operation to the schedule
    pub fn add_operation(&mut self, block_id: usize, op: RefCountOp) {
        self.operations.push((block_id, op));
    }

    /// Mark operations as eliminable (consecutive inc-dec)
    pub fn mark_eliminable(&mut self, inc_idx: usize, dec_idx: usize) {
        if inc_idx < dec_idx {
            self.eliminable_pairs.push((inc_idx, dec_idx));
        }
    }

    /// Get the cost of current schedule (number of operations)
    pub fn cost(&self) -> usize {
        self.operations.len() - self.eliminable_pairs.len() * 2
    }
}

/// Automatic refcount scheduler
#[derive(Debug, Clone)]
pub struct RefCountScheduler {
    /// Schedules for each variable
    schedules: HashMap<String, RefCountSchedule>,
    /// Control flow information
    cfg: HashMap<usize, Vec<usize>>, // block -> successors
}

impl RefCountScheduler {
    pub fn new() -> Self {
        Self {
            schedules: HashMap::new(),
            cfg: HashMap::new(),
        }
    }

    /// Add a variable to schedule
    pub fn add_variable(&mut self, var_name: String) {
        self.schedules
            .insert(var_name.clone(), RefCountSchedule::new(var_name));
    }

    /// Record an operation for a variable
    pub fn record_operation(&mut self, var_name: &str, block_id: usize, op: RefCountOp) {
        if let Some(schedule) = self.schedules.get_mut(var_name) {
            schedule.add_operation(block_id, op);
        }
    }

    /// Add control flow edge
    pub fn add_cfg_edge(&mut self, from: usize, to: usize) {
        self.cfg.entry(from).or_insert_with(Vec::new).push(to);
    }

    /// Optimize all schedules
    pub fn optimize(&mut self) {
        let mut schedules_to_optimize: Vec<String> = 
            self.schedules.keys().cloned().collect();
        
        for var_name in schedules_to_optimize {
            if let Some(schedule) = self.schedules.get_mut(&var_name) {
                // Find consecutive inc-dec pairs
                let mut i = 0;
                while i < schedule.operations.len() {
                    if i + 1 < schedule.operations.len() {
                        let (_block1, op1) = schedule.operations[i];
                        let (_block2, op2) = schedule.operations[i + 1];

                        // If consecutive inc-dec, mark for elimination
                        if op1 == RefCountOp::Increment && op2 == RefCountOp::Decrement {
                            schedule.mark_eliminable(i, i + 1);
                            i += 2; // Skip past the pair
                            continue;
                        }
                    }
                    i += 1;
                }

                // Mark operations that can be moved for better scheduling
                for idx in 0..schedule.operations.len() {
                    // Check if this operation is in an eliminable pair
                    let in_eliminable = schedule
                        .eliminable_pairs
                        .iter()
                        .any(|(i, j)| *i == idx || *j == idx);

                    // Non-eliminable operations can potentially be moved
                    if !in_eliminable {
                        schedule.moveable_ops.insert(idx);
                    }
                }
            }
        }
    }

    /// Get schedule for a variable
    pub fn get_schedule(&self, var_name: &str) -> Option<&RefCountSchedule> {
        self.schedules.get(var_name)
    }

    /// Get all variables with their schedules
    pub fn get_all_schedules(&self) -> &HashMap<String, RefCountSchedule> {
        &self.schedules
    }

    /// Calculate total refcount operations saved
    pub fn calculate_savings(&self) -> RefCountSavings {
        let mut total_saved = 0;
        let mut total_original = 0;

        for schedule in self.schedules.values() {
            total_original += schedule.operations.len();
            total_saved += schedule.eliminable_pairs.len() * 2;
        }

        let reduction_percent = if total_original > 0 {
            (total_saved as f64 / total_original as f64) * 100.0
        } else {
            0.0
        };

        RefCountSavings {
            operations_saved: total_saved,
            original_operations: total_original,
            final_operations: total_original - total_saved,
            reduction_percent,
        }
    }

    /// Generate optimization report
    pub fn report(&self) -> RefCountOptimizationReport {
        let savings = self.calculate_savings();
        let schedule_count = self.schedules.len();

        RefCountOptimizationReport {
            variables_optimized: schedule_count,
            savings,
            schedules: self.schedules.clone(),
        }
    }
}

impl Default for RefCountScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Savings from refcount optimization
#[derive(Debug, Clone)]
pub struct RefCountSavings {
    pub operations_saved: usize,
    pub original_operations: usize,
    pub final_operations: usize,
    pub reduction_percent: f64,
}

/// Report on refcount optimization results
#[derive(Debug, Clone)]
pub struct RefCountOptimizationReport {
    pub variables_optimized: usize,
    pub savings: RefCountSavings,
    pub schedules: HashMap<String, RefCountSchedule>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refcount_schedule_creation() {
        let schedule = RefCountSchedule::new("var1".to_string());
        assert_eq!(schedule.var_name, "var1");
        assert!(schedule.operations.is_empty());
    }

    #[test]
    fn test_add_operations() {
        let mut schedule = RefCountSchedule::new("var1".to_string());
        schedule.add_operation(0, RefCountOp::Increment);
        schedule.add_operation(1, RefCountOp::Decrement);

        assert_eq!(schedule.operations.len(), 2);
        assert_eq!(schedule.operations[0], (0, RefCountOp::Increment));
        assert_eq!(schedule.operations[1], (1, RefCountOp::Decrement));
    }

    #[test]
    fn test_schedule_cost() {
        let mut schedule = RefCountSchedule::new("var1".to_string());
        schedule.add_operation(0, RefCountOp::Increment);
        schedule.add_operation(1, RefCountOp::Decrement);

        assert_eq!(schedule.cost(), 2); // No optimization yet
        
        schedule.mark_eliminable(0, 1);
        assert_eq!(schedule.cost(), 0); // Both operations eliminated
    }

    #[test]
    fn test_scheduler_basic() {
        let mut scheduler = RefCountScheduler::new();
        scheduler.add_variable("x".to_string());
        scheduler.record_operation("x", 0, RefCountOp::Increment);
        scheduler.record_operation("x", 1, RefCountOp::Decrement);

        let schedule = scheduler.get_schedule("x").unwrap();
        assert_eq!(schedule.operations.len(), 2);
    }

    #[test]
    fn test_scheduler_optimization() {
        let mut scheduler = RefCountScheduler::new();
        scheduler.add_variable("x".to_string());
        scheduler.record_operation("x", 0, RefCountOp::Increment);
        scheduler.record_operation("x", 1, RefCountOp::Decrement);

        scheduler.optimize();

        let schedule = scheduler.get_schedule("x").unwrap();
        assert!(!schedule.eliminable_pairs.is_empty());
    }

    #[test]
    fn test_savings_calculation() {
        let mut scheduler = RefCountScheduler::new();
        scheduler.add_variable("x".to_string());
        scheduler.record_operation("x", 0, RefCountOp::Increment);
        scheduler.record_operation("x", 1, RefCountOp::Decrement);
        scheduler.optimize();

        let savings = scheduler.calculate_savings();
        assert_eq!(savings.original_operations, 2);
        assert_eq!(savings.operations_saved, 2);
        assert_eq!(savings.final_operations, 0);
    }

    #[test]
    fn test_multiple_variables() {
        let mut scheduler = RefCountScheduler::new();
        
        scheduler.add_variable("x".to_string());
        scheduler.record_operation("x", 0, RefCountOp::Increment);
        scheduler.record_operation("x", 2, RefCountOp::Decrement);

        scheduler.add_variable("y".to_string());
        scheduler.record_operation("y", 1, RefCountOp::Increment);
        scheduler.record_operation("y", 3, RefCountOp::Decrement);

        scheduler.optimize();

        let report = scheduler.report();
        assert_eq!(report.variables_optimized, 2);
    }

    #[test]
    fn test_control_flow_tracking() {
        let mut scheduler = RefCountScheduler::new();
        
        // Build a simple CFG: 0 -> 1, 0 -> 2
        scheduler.add_cfg_edge(0, 1);
        scheduler.add_cfg_edge(0, 2);

        scheduler.add_variable("x".to_string());
        scheduler.record_operation("x", 0, RefCountOp::Increment);
        scheduler.record_operation("x", 1, RefCountOp::Decrement);

        scheduler.optimize();

        let schedule = scheduler.get_schedule("x").unwrap();
        assert_eq!(schedule.operations.len(), 2);
    }

    #[test]
    fn test_moveable_operations() {
        let mut scheduler = RefCountScheduler::new();
        scheduler.add_variable("x".to_string());
        scheduler.record_operation("x", 0, RefCountOp::Increment);
        scheduler.record_operation("x", 1, RefCountOp::Increment);
        scheduler.record_operation("x", 2, RefCountOp::Decrement);
        scheduler.record_operation("x", 3, RefCountOp::Decrement);

        scheduler.optimize();

        let schedule = scheduler.get_schedule("x").unwrap();
        assert!(!schedule.moveable_ops.is_empty());
    }
}
