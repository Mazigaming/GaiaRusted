//! Smart pointer support (Box, Rc, Arc)
//!
//! Handles basic support for smart pointers like Box<T>, Rc<T>, and Arc<T>.
//! These types manage memory automatically and allow sharing ownership.

use crate::lowering::HirType;
use std::collections::HashMap;

/// Smart pointer types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SmartPointerType {
    /// Box<T>: Unique ownership, heap allocation
    Box,
    /// Rc<T>: Shared ownership, single-threaded
    Rc,
    /// Arc<T>: Shared ownership, thread-safe
    Arc,
}

impl SmartPointerType {
    /// Get display name
    pub fn as_str(&self) -> &'static str {
        match self {
            SmartPointerType::Box => "Box",
            SmartPointerType::Rc => "Rc",
            SmartPointerType::Arc => "Arc",
        }
    }

    /// Check if this is a shared pointer type
    pub fn is_shared(&self) -> bool {
        matches!(self, SmartPointerType::Rc | SmartPointerType::Arc)
    }

    /// Check if this is thread-safe
    pub fn is_thread_safe(&self) -> bool {
        matches!(self, SmartPointerType::Arc)
    }
}

/// Represents a smart pointer binding
#[derive(Debug, Clone)]
pub struct SmartPointerBinding {
    /// Type of smart pointer
    pub pointer_type: SmartPointerType,
    /// Inner type (T in Box<T>)
    pub inner_type: Box<HirType>,
    /// Number of clones (for Rc and Arc)
    pub clone_count: usize,
    /// Whether the binding can be moved
    pub can_move: bool,
}

impl SmartPointerBinding {
    /// Create a new Box binding
    pub fn new_box(inner_type: HirType) -> Self {
        SmartPointerBinding {
            pointer_type: SmartPointerType::Box,
            inner_type: Box::new(inner_type),
            clone_count: 1,
            can_move: true,
        }
    }

    /// Create a new Rc binding
    pub fn new_rc(inner_type: HirType) -> Self {
        SmartPointerBinding {
            pointer_type: SmartPointerType::Rc,
            inner_type: Box::new(inner_type),
            clone_count: 1,
            can_move: true,
        }
    }

    /// Create a new Arc binding
    pub fn new_arc(inner_type: HirType) -> Self {
        SmartPointerBinding {
            pointer_type: SmartPointerType::Arc,
            inner_type: Box::new(inner_type),
            clone_count: 1,
            can_move: true,
        }
    }

    /// Clone the pointer (for Rc and Arc)
    pub fn clone_pointer(&mut self) -> Result<(), String> {
        if !self.pointer_type.is_shared() {
            return Err("Cannot clone Box pointers".to_string());
        }
        self.clone_count += 1;
        Ok(())
    }

    /// Drop a clone
    pub fn drop_clone(&mut self) -> Result<(), String> {
        if self.clone_count > 0 {
            self.clone_count -= 1;
            Ok(())
        } else {
            Err("No clones to drop".to_string())
        }
    }

    /// Check if we can move this binding
    pub fn can_move_binding(&self) -> bool {
        self.can_move
    }

    /// Mark binding as moved
    pub fn mark_moved(&mut self) {
        self.can_move = false;
    }

    /// Check if this is a Box pointer
    pub fn is_box(&self) -> bool {
        self.pointer_type == SmartPointerType::Box
    }

    /// Check if this is shared (Rc or Arc)
    pub fn is_shared(&self) -> bool {
        self.pointer_type.is_shared()
    }
}

/// Detect if a type is a smart pointer type
pub fn is_smart_pointer_type(ty_name: &str) -> Option<SmartPointerType> {
    match ty_name {
        "Box" => Some(SmartPointerType::Box),
        "Rc" => Some(SmartPointerType::Rc),
        "Arc" => Some(SmartPointerType::Arc),
        _ => None,
    }
}

/// Smart pointer environment
#[derive(Debug)]
pub struct SmartPointerEnv {
    /// Bindings with smart pointers
    bindings: HashMap<String, SmartPointerBinding>,
}

impl SmartPointerEnv {
    /// Create new environment
    pub fn new() -> Self {
        SmartPointerEnv {
            bindings: HashMap::new(),
        }
    }

    /// Register a binding with smart pointer
    pub fn register(&mut self, name: String, binding: SmartPointerBinding) {
        self.bindings.insert(name, binding);
    }

    /// Get binding mutably
    pub fn get_mut(&mut self, name: &str) -> Option<&mut SmartPointerBinding> {
        self.bindings.get_mut(name)
    }

    /// Get binding immutably
    pub fn get(&self, name: &str) -> Option<&SmartPointerBinding> {
        self.bindings.get(name)
    }

    /// Check if a binding is a smart pointer
    pub fn is_smart_pointer(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Get total clone count for a binding
    pub fn get_clone_count(&self, name: &str) -> Option<usize> {
        self.bindings.get(name).map(|b| b.clone_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_box() {
        let sp = is_smart_pointer_type("Box");
        assert!(sp.is_some());
        assert_eq!(sp.unwrap(), SmartPointerType::Box);
    }

    #[test]
    fn test_detect_rc() {
        let sp = is_smart_pointer_type("Rc");
        assert!(sp.is_some());
        assert_eq!(sp.unwrap(), SmartPointerType::Rc);
    }

    #[test]
    fn test_detect_arc() {
        let sp = is_smart_pointer_type("Arc");
        assert!(sp.is_some());
        assert_eq!(sp.unwrap(), SmartPointerType::Arc);
    }

    #[test]
    fn test_detect_not_smart_pointer() {
        let sp = is_smart_pointer_type("Vec");
        assert!(sp.is_none());
    }

    #[test]
    fn test_smart_pointer_type_display() {
        assert_eq!(SmartPointerType::Box.as_str(), "Box");
        assert_eq!(SmartPointerType::Rc.as_str(), "Rc");
        assert_eq!(SmartPointerType::Arc.as_str(), "Arc");
    }

    #[test]
    fn test_is_shared() {
        assert!(!SmartPointerType::Box.is_shared());
        assert!(SmartPointerType::Rc.is_shared());
        assert!(SmartPointerType::Arc.is_shared());
    }

    #[test]
    fn test_is_thread_safe() {
        assert!(!SmartPointerType::Box.is_thread_safe());
        assert!(!SmartPointerType::Rc.is_thread_safe());
        assert!(SmartPointerType::Arc.is_thread_safe());
    }

    #[test]
    fn test_box_binding_creation() {
        let binding = SmartPointerBinding::new_box(HirType::Int32);
        assert_eq!(binding.pointer_type, SmartPointerType::Box);
        assert_eq!(binding.clone_count, 1);
        assert!(binding.can_move);
    }

    #[test]
    fn test_rc_binding_creation() {
        let binding = SmartPointerBinding::new_rc(HirType::Int32);
        assert_eq!(binding.pointer_type, SmartPointerType::Rc);
        assert_eq!(binding.clone_count, 1);
        assert!(binding.can_move);
    }

    #[test]
    fn test_arc_binding_creation() {
        let binding = SmartPointerBinding::new_arc(HirType::Int32);
        assert_eq!(binding.pointer_type, SmartPointerType::Arc);
        assert_eq!(binding.clone_count, 1);
        assert!(binding.can_move);
    }

    #[test]
    fn test_cannot_clone_box() {
        let mut binding = SmartPointerBinding::new_box(HirType::Int32);
        let result = binding.clone_pointer();
        assert!(result.is_err());
    }

    #[test]
    fn test_can_clone_rc() {
        let mut binding = SmartPointerBinding::new_rc(HirType::Int32);
        let result = binding.clone_pointer();
        assert!(result.is_ok());
        assert_eq!(binding.clone_count, 2);
    }

    #[test]
    fn test_can_clone_arc() {
        let mut binding = SmartPointerBinding::new_arc(HirType::Int32);
        let result = binding.clone_pointer();
        assert!(result.is_ok());
        assert_eq!(binding.clone_count, 2);
    }

    #[test]
    fn test_drop_clone() {
        let mut binding = SmartPointerBinding::new_rc(HirType::Int32);
        binding.clone_pointer().unwrap();
        assert_eq!(binding.clone_count, 2);
        let result = binding.drop_clone();
        assert!(result.is_ok());
        assert_eq!(binding.clone_count, 1);
    }

    #[test]
    fn test_mark_moved() {
        let mut binding = SmartPointerBinding::new_box(HirType::Int32);
        assert!(binding.can_move_binding());
        binding.mark_moved();
        assert!(!binding.can_move_binding());
    }

    #[test]
    fn test_is_box() {
        let box_binding = SmartPointerBinding::new_box(HirType::Int32);
        let rc_binding = SmartPointerBinding::new_rc(HirType::Int32);
        assert!(box_binding.is_box());
        assert!(!rc_binding.is_box());
    }

    #[test]
    fn test_smart_pointer_env() {
        let mut env = SmartPointerEnv::new();
        let binding = SmartPointerBinding::new_box(HirType::Int32);
        env.register("b".to_string(), binding);

        assert!(env.is_smart_pointer("b"));
        assert!(!env.is_smart_pointer("x"));

        let b = env.get("b");
        assert!(b.is_some());
    }

    #[test]
    fn test_smart_pointer_env_clone_count() {
        let mut env = SmartPointerEnv::new();
        let binding = SmartPointerBinding::new_rc(HirType::Int32);
        env.register("r".to_string(), binding);

        assert_eq!(env.get_clone_count("r"), Some(1));

        if let Some(b) = env.get_mut("r") {
            let _ = b.clone_pointer();
        }

        assert_eq!(env.get_clone_count("r"), Some(2));
    }
}
