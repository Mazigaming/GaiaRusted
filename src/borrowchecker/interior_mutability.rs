//! Interior mutability support (Cell, RefCell)
//!
//! Handles basic support for Cell<T> and RefCell<T> which allow mutation
//! through immutable references by using runtime borrow checking.

use crate::lowering::HirType;
use std::collections::HashMap;

/// Types that support interior mutability
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InteriorMutableType {
    /// Cell<T>: Single-threaded runtime borrow checking (non-Copy T only)
    Cell,
    /// RefCell<T>: Single-threaded runtime borrow checking with panics
    RefCell,
}

impl InteriorMutableType {
    /// Get display name
    pub fn as_str(&self) -> &'static str {
        match self {
            InteriorMutableType::Cell => "Cell",
            InteriorMutableType::RefCell => "RefCell",
        }
    }
}

/// Represents a binding with interior mutability
#[derive(Debug, Clone)]
pub struct InteriorMutableBinding {
    /// What type of interior mutability (Cell, RefCell)
    pub mutability_type: InteriorMutableType,
    /// Inner type (T in Cell<T>)
    pub inner_type: Box<HirType>,
    /// Can be borrowed multiple times immutably
    pub can_borrow_multiple_times: bool,
    /// Current active mutable borrows (should be 0 or 1)
    pub active_mutable_borrows: usize,
    /// Current active immutable borrows (can be any number)
    pub active_immutable_borrows: usize,
}

impl InteriorMutableBinding {
    /// Create a new Cell binding
    pub fn new_cell(inner_type: HirType) -> Self {
        InteriorMutableBinding {
            mutability_type: InteriorMutableType::Cell,
            inner_type: Box::new(inner_type),
            can_borrow_multiple_times: true,
            active_mutable_borrows: 0,
            active_immutable_borrows: 0,
        }
    }

    /// Create a new RefCell binding
    pub fn new_refcell(inner_type: HirType) -> Self {
        InteriorMutableBinding {
            mutability_type: InteriorMutableType::RefCell,
            inner_type: Box::new(inner_type),
            can_borrow_multiple_times: true,
            active_mutable_borrows: 0,
            active_immutable_borrows: 0,
        }
    }

    /// Check if we can borrow immutably
    pub fn can_borrow_immutable(&self) -> bool {
        self.active_mutable_borrows == 0
    }

    /// Check if we can borrow mutably
    pub fn can_borrow_mutable(&self) -> bool {
        self.active_mutable_borrows == 0 && self.active_immutable_borrows == 0
    }

    /// Borrow immutably (increments counter)
    pub fn borrow_immutable(&mut self) {
        self.active_immutable_borrows += 1;
    }

    /// Borrow mutably (increments counter)
    pub fn borrow_mutable(&mut self) -> Result<(), String> {
        if self.active_mutable_borrows > 0 {
            return Err("Already has active mutable borrow".to_string());
        }
        if self.active_immutable_borrows > 0 {
            return Err("Cannot mutably borrow with active immutable borrows".to_string());
        }
        self.active_mutable_borrows += 1;
        Ok(())
    }

    /// Release immutable borrow
    pub fn release_immutable(&mut self) {
        if self.active_immutable_borrows > 0 {
            self.active_immutable_borrows -= 1;
        }
    }

    /// Release mutable borrow
    pub fn release_mutable(&mut self) {
        if self.active_mutable_borrows > 0 {
            self.active_mutable_borrows -= 1;
        }
    }
}

/// Detect if a type is an interior mutable type (Cell or RefCell)
pub fn is_interior_mutable_type(ty_name: &str) -> Option<InteriorMutableType> {
    match ty_name {
        "Cell" => Some(InteriorMutableType::Cell),
        "RefCell" => Some(InteriorMutableType::RefCell),
        _ => None,
    }
}

/// Interior mutability environment
#[derive(Debug)]
pub struct InteriorMutabilityEnv {
    /// Bindings with interior mutability
    bindings: HashMap<String, InteriorMutableBinding>,
}

impl InteriorMutabilityEnv {
    /// Create new environment
    pub fn new() -> Self {
        InteriorMutabilityEnv {
            bindings: HashMap::new(),
        }
    }

    /// Register a binding with interior mutability
    pub fn register(&mut self, name: String, binding: InteriorMutableBinding) {
        self.bindings.insert(name, binding);
    }

    /// Get binding if it has interior mutability
    pub fn get_mut(&mut self, name: &str) -> Option<&mut InteriorMutableBinding> {
        self.bindings.get_mut(name)
    }

    /// Get binding immutably
    pub fn get(&self, name: &str) -> Option<&InteriorMutableBinding> {
        self.bindings.get(name)
    }

    /// Check if a binding is interior mutable
    pub fn is_interior_mutable(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_cell() {
        let mt = is_interior_mutable_type("Cell");
        assert!(mt.is_some());
        assert_eq!(mt.unwrap(), InteriorMutableType::Cell);
    }

    #[test]
    fn test_detect_refcell() {
        let mt = is_interior_mutable_type("RefCell");
        assert!(mt.is_some());
        assert_eq!(mt.unwrap(), InteriorMutableType::RefCell);
    }

    #[test]
    fn test_detect_not_interior_mutable() {
        let mt = is_interior_mutable_type("Vec");
        assert!(mt.is_none());
    }

    #[test]
    fn test_interior_mutable_type_display() {
        assert_eq!(InteriorMutableType::Cell.as_str(), "Cell");
        assert_eq!(InteriorMutableType::RefCell.as_str(), "RefCell");
    }

    #[test]
    fn test_cell_binding_creation() {
        let binding = InteriorMutableBinding::new_cell(HirType::Int32);
        assert_eq!(binding.mutability_type, InteriorMutableType::Cell);
        assert_eq!(binding.active_mutable_borrows, 0);
        assert_eq!(binding.active_immutable_borrows, 0);
    }

    #[test]
    fn test_refcell_binding_creation() {
        let binding = InteriorMutableBinding::new_refcell(HirType::Int32);
        assert_eq!(binding.mutability_type, InteriorMutableType::RefCell);
        assert_eq!(binding.active_mutable_borrows, 0);
        assert_eq!(binding.active_immutable_borrows, 0);
    }

    #[test]
    fn test_can_borrow_multiple_immutable() {
        let mut binding = InteriorMutableBinding::new_cell(HirType::Int32);
        assert!(binding.can_borrow_immutable());
        binding.borrow_immutable();
        assert!(binding.can_borrow_immutable());
        binding.borrow_immutable();
        assert!(binding.can_borrow_immutable());
        assert_eq!(binding.active_immutable_borrows, 2);
    }

    #[test]
    fn test_cannot_borrow_multiple_mutable() {
        let mut binding = InteriorMutableBinding::new_refcell(HirType::Int32);
        assert!(binding.can_borrow_mutable());
        let result = binding.borrow_mutable();
        assert!(result.is_ok());
        assert!(!binding.can_borrow_mutable());
        let result2 = binding.borrow_mutable();
        assert!(result2.is_err());
    }

    #[test]
    fn test_cannot_borrow_mutable_with_immutable() {
        let mut binding = InteriorMutableBinding::new_refcell(HirType::Int32);
        binding.borrow_immutable();
        let result = binding.borrow_mutable();
        assert!(result.is_err());
    }

    #[test]
    fn test_release_immutable_borrow() {
        let mut binding = InteriorMutableBinding::new_cell(HirType::Int32);
        binding.borrow_immutable();
        binding.borrow_immutable();
        assert_eq!(binding.active_immutable_borrows, 2);
        binding.release_immutable();
        assert_eq!(binding.active_immutable_borrows, 1);
    }

    #[test]
    fn test_release_mutable_borrow() {
        let mut binding = InteriorMutableBinding::new_refcell(HirType::Int32);
        let _ = binding.borrow_mutable();
        assert_eq!(binding.active_mutable_borrows, 1);
        binding.release_mutable();
        assert_eq!(binding.active_mutable_borrows, 0);
    }

    #[test]
    fn test_interior_mutability_env() {
        let mut env = InteriorMutabilityEnv::new();
        let binding = InteriorMutableBinding::new_cell(HirType::Int32);
        env.register("x".to_string(), binding);

        assert!(env.is_interior_mutable("x"));
        assert!(!env.is_interior_mutable("y"));

        let b = env.get("x");
        assert!(b.is_some());
    }

    #[test]
    fn test_interior_mutability_env_mutable_access() {
        let mut env = InteriorMutabilityEnv::new();
        let binding = InteriorMutableBinding::new_refcell(HirType::Int32);
        env.register("x".to_string(), binding);

        if let Some(b) = env.get_mut("x") {
            let result = b.borrow_mutable();
            assert!(result.is_ok());
        }
    }
}
