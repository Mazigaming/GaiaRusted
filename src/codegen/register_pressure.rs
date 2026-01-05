//! # Register Pressure Analysis
//!
//! Tracks live ranges and calculates register needs for better allocation decisions.
//! Determines when spilling is necessary and selects optimal spill candidates.

use std::collections::{HashMap, HashSet, BTreeMap};

/// A live range for a variable
#[derive(Debug, Clone)]
pub struct LiveRange {
    /// Variable name
    pub name: String,
    /// First use (instruction number)
    pub start: usize,
    /// Last use (instruction number)
    pub end: usize,
    /// Value size in bytes
    pub size: usize,
    /// Is this a spilled variable?
    pub spilled: bool,
}

impl LiveRange {
    /// Check if this range is alive at a given instruction
    pub fn is_alive_at(&self, instr: usize) -> bool {
        instr >= self.start && instr <= self.end
    }

    /// Get range length
    pub fn length(&self) -> usize {
        self.end - self.start + 1
    }
}

/// Register allocation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterState {
    /// Available for allocation
    Available,
    /// Currently in use
    InUse,
    /// Caller-saved (volatile)
    CallerSaved,
    /// Callee-saved (non-volatile)
    CalleeSaved,
}

/// Physical register
#[derive(Debug, Clone)]
pub struct PhysicalRegister {
    /// Register name (rax, rbx, etc.)
    pub name: String,
    /// Current state
    pub state: RegisterState,
    /// Value currently in register (if any)
    pub current_value: Option<String>,
    /// State (caller vs callee saved)
    pub saved_type: SavedType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedType {
    CallerSaved,
    CalleeSaved,
}

impl PhysicalRegister {
    /// Create new physical register
    pub fn new(name: &str, saved_type: SavedType) -> Self {
        PhysicalRegister {
            name: name.to_string(),
            state: RegisterState::Available,
            current_value: None,
            saved_type,
        }
    }

    /// Allocate this register for a value
    pub fn allocate(&mut self, value: String) {
        self.state = RegisterState::InUse;
        self.current_value = Some(value);
    }

    /// Free this register
    pub fn free(&mut self) {
        self.state = RegisterState::Available;
        self.current_value = None;
    }
}

/// Register pressure analyzer
pub struct RegisterPressureAnalyzer {
    /// Live ranges for all variables
    pub live_ranges: HashMap<String, LiveRange>,
    /// Physical registers
    pub registers: Vec<PhysicalRegister>,
    /// Current instruction index
    pub current_instr: usize,
    /// Stack slots for spills
    pub stack_slots: HashMap<String, i64>,
    /// Next available stack slot
    next_stack_slot: i64,
}

impl RegisterPressureAnalyzer {
    /// Create new register pressure analyzer
    pub fn new() -> Self {
        // Create standard x86-64 registers (16 total, ignoring RSP/RBP)
        let mut registers = Vec::new();

        // Caller-saved (volatile)
        let caller_saved = ["rax", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10", "r11"];
        for reg in &caller_saved {
            registers.push(PhysicalRegister::new(reg, SavedType::CallerSaved));
        }

        // Callee-saved (non-volatile)
        let callee_saved = ["rbx", "r12", "r13", "r14", "r15"];
        for reg in &callee_saved {
            registers.push(PhysicalRegister::new(reg, SavedType::CalleeSaved));
        }

        RegisterPressureAnalyzer {
            live_ranges: HashMap::new(),
            registers,
            current_instr: 0,
            stack_slots: HashMap::new(),
            next_stack_slot: -8, // Start from rbp - 8
        }
    }

    /// Record a variable's live range
    pub fn add_live_range(&mut self, name: String, start: usize, end: usize, size: usize) {
        self.live_ranges.insert(
            name.clone(),
            LiveRange {
                name,
                start,
                end,
                size,
                spilled: false,
            },
        );
    }

    /// Get variables live at a given instruction
    pub fn live_at(&self, instr: usize) -> Vec<String> {
        self.live_ranges.iter()
            .filter(|(_, range)| range.is_alive_at(instr))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Calculate register pressure at a given instruction
    pub fn pressure_at(&self, instr: usize) -> usize {
        self.live_at(instr).len()
    }

    /// Get peak register pressure
    pub fn peak_pressure(&self) -> usize {
        let max_instr = self.live_ranges.values()
            .map(|r| r.end)
            .max()
            .unwrap_or(0);

        (0..=max_instr)
            .map(|i| self.pressure_at(i))
            .max()
            .unwrap_or(0)
    }

    /// Allocate a variable to a register
    pub fn allocate_to_register(&mut self, var: &str) -> Result<usize, String> {
        // Find first available register
        for (idx, reg) in self.registers.iter_mut().enumerate() {
            if reg.state == RegisterState::Available {
                reg.allocate(var.to_string());
                return Ok(idx);
            }
        }

        // No register available - need to spill
        Err(format!("No available register for {}", var))
    }

    /// Allocate a variable to stack (spill)
    pub fn allocate_to_stack(&mut self, var: &str) -> i64 {
        let slot = self.next_stack_slot;
        self.stack_slots.insert(var.to_string(), slot);
        self.next_stack_slot -= 8; // Next slot 8 bytes down
        slot
    }

    /// Select a variable to spill
    pub fn select_spill_candidate(&self) -> Option<String> {
        // Use "furthest use" heuristic: spill the variable with furthest next use
        let mut candidates = Vec::new();

        for (name, range) in &self.live_ranges {
            if !range.spilled {
                candidates.push((name.clone(), range.end));
            }
        }

        candidates.sort_by_key(|(_, end)| std::cmp::Reverse(*end));
        candidates.first().map(|(name, _)| name.clone())
    }

    /// Estimate code size from register pressure
    pub fn estimate_spill_code(&self) -> usize {
        let peak = self.peak_pressure();
        let available = self.registers.len();

        if peak <= available {
            0 // No spills needed
        } else {
            // Estimate: ~4 extra instructions per spilled variable
            (peak - available) * 4
        }
    }

    /// Get register allocation report
    pub fn get_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("Register Pressure Analysis\n"));
        report.push_str(&format!("=========================\n"));
        report.push_str(&format!("Peak pressure: {} registers\n", self.peak_pressure()));
        report.push_str(&format!("Available: {} registers\n", self.registers.len()));
        report.push_str(&format!("Live ranges: {} variables\n", self.live_ranges.len()));
        report.push_str(&format!("Spilled: {} variables\n", self.stack_slots.len()));
        report.push_str(&format!("Estimated spill code size: {} bytes\n", self.estimate_spill_code()));

        report
    }
}

/// Live range calculation algorithm
pub struct LiveRangeCalculator {
    /// Function body (instruction sequence)
    pub instructions: Vec<String>,
    /// Variable definitions (name -> instruction indices)
    pub definitions: HashMap<String, Vec<usize>>,
    /// Variable uses (name -> instruction indices)
    pub uses: HashMap<String, Vec<usize>>,
}

impl LiveRangeCalculator {
    /// Create new live range calculator
    pub fn new(instructions: Vec<String>) -> Self {
        LiveRangeCalculator {
            instructions,
            definitions: HashMap::new(),
            uses: HashMap::new(),
        }
    }

    /// Record a definition
    pub fn add_definition(&mut self, var: &str, instr_idx: usize) {
        self.definitions.entry(var.to_string())
            .or_insert_with(Vec::new)
            .push(instr_idx);
    }

    /// Record a use
    pub fn add_use(&mut self, var: &str, instr_idx: usize) {
        self.uses.entry(var.to_string())
            .or_insert_with(Vec::new)
            .push(instr_idx);
    }

    /// Calculate live ranges
    pub fn calculate(&self) -> HashMap<String, LiveRange> {
        let mut ranges = HashMap::new();

        let all_vars: HashSet<_> = self.definitions.keys()
            .chain(self.uses.keys())
            .cloned()
            .collect();

        for var in all_vars {
            let def_indices = self.definitions.get(&var).cloned().unwrap_or_default();
            let use_indices = self.uses.get(&var).cloned().unwrap_or_default();

            if !def_indices.is_empty() || !use_indices.is_empty() {
                let start = def_indices.iter().chain(use_indices.iter())
                    .min()
                    .copied()
                    .unwrap_or(0);
                let end = def_indices.iter().chain(use_indices.iter())
                    .max()
                    .copied()
                    .unwrap_or(0);

                ranges.insert(
                    var.clone(),
                    LiveRange {
                        name: var,
                        start,
                        end,
                        size: 8, // Assume 8 bytes per variable
                        spilled: false,
                    },
                );
            }
        }

        ranges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_range_alive_at() {
        let range = LiveRange {
            name: "x".to_string(),
            start: 5,
            end: 15,
            size: 8,
            spilled: false,
        };

        assert!(range.is_alive_at(5));
        assert!(range.is_alive_at(10));
        assert!(range.is_alive_at(15));
        assert!(!range.is_alive_at(4));
        assert!(!range.is_alive_at(16));
    }

    #[test]
    fn test_register_allocation() {
        let mut analyzer = RegisterPressureAnalyzer::new();
        analyzer.add_live_range("x".to_string(), 0, 10, 8);
        analyzer.add_live_range("y".to_string(), 5, 15, 8);
        analyzer.add_live_range("z".to_string(), 10, 20, 8);

        assert_eq!(analyzer.pressure_at(0), 1); // only x
        assert_eq!(analyzer.pressure_at(10), 3); // x, y, z all live
        assert!(analyzer.peak_pressure() >= 3);
    }

    #[test]
    fn test_live_range_calculation() {
        let instrs = vec!["mov x, 1".to_string(), "add x, y".to_string()];
        let mut calc = LiveRangeCalculator::new(instrs);

        calc.add_definition("x", 0);
        calc.add_use("x", 1);
        calc.add_use("y", 1);

        let ranges = calc.calculate();
        assert!(ranges.contains_key("x"));
        assert!(ranges.contains_key("y"));
    }

    #[test]
    fn test_spill_selection() {
        let mut analyzer = RegisterPressureAnalyzer::new();
        analyzer.add_live_range("x".to_string(), 0, 10, 8);
        analyzer.add_live_range("y".to_string(), 0, 20, 8);
        analyzer.add_live_range("z".to_string(), 0, 5, 8);

        let spill = analyzer.select_spill_candidate();
        // Should spill y (furthest use at 20)
        assert_eq!(spill, Some("y".to_string()));
    }

    #[test]
    fn test_stack_allocation() {
        let mut analyzer = RegisterPressureAnalyzer::new();
        let slot1 = analyzer.allocate_to_stack("x");
        let slot2 = analyzer.allocate_to_stack("y");

        assert_eq!(slot1, -8);
        assert_eq!(slot2, -16);
        assert!(slot2 < slot1); // Stack grows downward
    }
}
