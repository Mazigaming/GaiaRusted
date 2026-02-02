//! # Smart Pointer Type System
//!
//! Defines types and operations for smart pointers:
//! - Box<T>: Heap allocation with single ownership
//! - Rc<T>: Reference counting for shared single-threaded ownership
//! - Arc<T>: Atomic reference counting for thread-safe shared ownership

use crate::typesystem::Type;
use std::fmt;

/// Box<T> type - heap-allocated value with single ownership
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxType {
    /// The type being boxed
    pub inner: Box<Type>,
    /// Whether the box allows mutable access
    pub is_mutable: bool,
}

impl BoxType {
    /// Create a new Box type
    pub fn new(inner: Type) -> Self {
        BoxType {
            inner: Box::new(inner),
            is_mutable: false,
        }
    }

    /// Create a mutable Box type
    pub fn mutable(inner: Type) -> Self {
        BoxType {
            inner: Box::new(inner),
            is_mutable: true,
        }
    }

    /// Get the boxed type
    pub fn inner_type(&self) -> &Type {
        &self.inner
    }

    /// Check if this Box is mutable
    pub fn is_mut(&self) -> bool {
        self.is_mutable
    }

    /// Get allocation size (pointer-sized)
    pub fn allocation_size(&self) -> usize {
        8 // On 64-bit, pointer is 8 bytes
    }
}

impl fmt::Display for BoxType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_mutable {
            write!(f, "Box<&mut {}>", self.inner)
        } else {
            write!(f, "Box<{}>", self.inner)
        }
    }
}

/// Rc<T> type - reference counted shared ownership (single-threaded)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RcType {
    /// The type being reference counted
    pub inner: Box<Type>,
}

impl RcType {
    /// Create a new Rc type
    pub fn new(inner: Type) -> Self {
        RcType {
            inner: Box::new(inner),
        }
    }

    /// Get the wrapped type
    pub fn inner_type(&self) -> &Type {
        &self.inner
    }

    /// Get allocation size (refcount + data)
    pub fn allocation_size(&self) -> usize {
        4 // refcount is u32 (4 bytes), plus actual data
    }

    /// Get refcount offset (always at start)
    pub fn refcount_offset(&self) -> usize {
        0
    }

    /// Get data offset (after refcount)
    pub fn data_offset(&self) -> usize {
        4
    }
}

impl fmt::Display for RcType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rc<{}>", self.inner)
    }
}

/// Arc<T> type - atomic reference counted shared ownership (thread-safe)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArcType {
    /// The type being atomically reference counted
    pub inner: Box<Type>,
}

impl ArcType {
    /// Create a new Arc type
    pub fn new(inner: Type) -> Self {
        ArcType {
            inner: Box::new(inner),
        }
    }

    /// Get the wrapped type
    pub fn inner_type(&self) -> &Type {
        &self.inner
    }

    /// Get allocation size (atomic refcount + data)
    pub fn allocation_size(&self) -> usize {
        8 // atomic u64 for refcount
    }

    /// Get refcount offset
    pub fn refcount_offset(&self) -> usize {
        0
    }

    /// Get data offset (after atomic refcount)
    pub fn data_offset(&self) -> usize {
        8
    }

    /// Check if type is Send (can be sent across threads)
    pub fn is_send(&self) -> bool {
        true // Arc<T> is Send if T is Send
    }

    /// Check if type is Sync (can be shared across threads)
    pub fn is_sync(&self) -> bool {
        true // Arc<T> is Sync if T is Sync
    }
}

impl fmt::Display for ArcType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Arc<{}>", self.inner)
    }
}

/// Operations on smart pointers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartPointerOp {
    /// Create a new Box
    BoxNew,
    /// Dereference a Box
    BoxDeref,
    /// Mutable dereference of Box
    BoxDerefMut,
    /// Drop a Box
    BoxDrop,

    /// Create a new Rc
    RcNew,
    /// Clone an Rc (increments refcount)
    RcClone,
    /// Dereference an Rc
    RcDeref,
    /// Drop an Rc (decrements refcount)
    RcDrop,
    /// Try to unwrap an Rc to get unique ownership
    RcUnwrap,

    /// Create a new Arc
    ArcNew,
    /// Clone an Arc (atomic increment)
    ArcClone,
    /// Dereference an Arc
    ArcDeref,
    /// Drop an Arc (atomic decrement)
    ArcDrop,
    /// Try to unwrap an Arc to get unique ownership
    ArcUnwrap,
}

impl fmt::Display for SmartPointerOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SmartPointerOp::BoxNew => write!(f, "Box::new"),
            SmartPointerOp::BoxDeref => write!(f, "Box deref"),
            SmartPointerOp::BoxDerefMut => write!(f, "Box deref_mut"),
            SmartPointerOp::BoxDrop => write!(f, "Box drop"),
            SmartPointerOp::RcNew => write!(f, "Rc::new"),
            SmartPointerOp::RcClone => write!(f, "Rc clone"),
            SmartPointerOp::RcDeref => write!(f, "Rc deref"),
            SmartPointerOp::RcDrop => write!(f, "Rc drop"),
            SmartPointerOp::RcUnwrap => write!(f, "Rc unwrap"),
            SmartPointerOp::ArcNew => write!(f, "Arc::new"),
            SmartPointerOp::ArcClone => write!(f, "Arc clone"),
            SmartPointerOp::ArcDeref => write!(f, "Arc deref"),
            SmartPointerOp::ArcDrop => write!(f, "Arc drop"),
            SmartPointerOp::ArcUnwrap => write!(f, "Arc unwrap"),
        }
    }
}

/// Result type for smart pointer operations
pub type SmartPointerResult<T> = Result<T, SmartPointerError>;

/// Errors that can occur with smart pointer operations
#[derive(Debug, Clone)]
pub enum SmartPointerError {
    /// Type mismatch in smart pointer operation
    TypeMismatch(String),
    /// Invalid dereference
    InvalidDeref(String),
    /// Memory allocation failed
    AllocationFailed,
    /// Refcount overflow
    RefcountOverflow,
    /// Invalid unwrap (not unique owner)
    CannotUnwrap(String),
    /// Generic smart pointer error
    Other(String),
}

impl fmt::Display for SmartPointerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SmartPointerError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            SmartPointerError::InvalidDeref(msg) => write!(f, "Invalid dereference: {}", msg),
            SmartPointerError::AllocationFailed => write!(f, "Memory allocation failed"),
            SmartPointerError::RefcountOverflow => write!(f, "Reference count overflow"),
            SmartPointerError::CannotUnwrap(msg) => write!(f, "Cannot unwrap: {}", msg),
            SmartPointerError::Other(msg) => write!(f, "Smart pointer error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_type_creation() {
        let box_type = BoxType::new(Type::I32);
        assert_eq!(box_type.inner_type(), &Type::I32);
        assert!(!box_type.is_mut());
    }

    #[test]
    fn test_box_type_mutable() {
        let box_type = BoxType::mutable(Type::I32);
        assert!(box_type.is_mut());
    }

    #[test]
    fn test_box_allocation_size() {
        let box_type = BoxType::new(Type::I32);
        assert_eq!(box_type.allocation_size(), 8);
    }

    #[test]
    fn test_box_display() {
        let box_type = BoxType::new(Type::I32);
        let display = format!("{}", box_type);
        assert!(display.contains("Box"));
        assert!(display.contains("i32"));
    }

    #[test]
    fn test_rc_type_creation() {
        let rc_type = RcType::new(Type::Str);
        assert_eq!(rc_type.inner_type(), &Type::Str);
    }

    #[test]
    fn test_rc_offsets() {
        let rc_type = RcType::new(Type::I32);
        assert_eq!(rc_type.refcount_offset(), 0);
        assert_eq!(rc_type.data_offset(), 4);
    }

    #[test]
    fn test_rc_display() {
        let rc_type = RcType::new(Type::Bool);
        let display = format!("{}", rc_type);
        assert!(display.contains("Rc"));
        assert!(display.contains("bool"));
    }

    #[test]
    fn test_arc_type_creation() {
        let arc_type = ArcType::new(Type::I32);
        assert_eq!(arc_type.inner_type(), &Type::I32);
    }

    #[test]
    fn test_arc_thread_safety() {
        let arc_type = ArcType::new(Type::Str);
        assert!(arc_type.is_send());
        assert!(arc_type.is_sync());
    }

    #[test]
    fn test_arc_offsets() {
        let arc_type = ArcType::new(Type::I32);
        assert_eq!(arc_type.refcount_offset(), 0);
        assert_eq!(arc_type.data_offset(), 8);
    }

    #[test]
    fn test_arc_display() {
        let arc_type = ArcType::new(Type::Bool);
        let display = format!("{}", arc_type);
        assert!(display.contains("Arc"));
    }

    #[test]
    fn test_smart_pointer_op_display() {
        assert_eq!(format!("{}", SmartPointerOp::BoxNew), "Box::new");
        assert_eq!(format!("{}", SmartPointerOp::RcClone), "Rc clone");
        assert_eq!(format!("{}", SmartPointerOp::ArcDrop), "Arc drop");
    }

    #[test]
    fn test_smart_pointer_error_display() {
        let error = SmartPointerError::TypeMismatch("i32 vs String".to_string());
        let msg = format!("{}", error);
        assert!(msg.contains("Type mismatch"));
    }

    #[test]
    fn test_nested_smart_pointers() {
        let inner_box = BoxType::new(Type::I32);
        // Box has 8-byte allocation size (pointer)
        assert_eq!(inner_box.allocation_size(), 8);
    }

    #[test]
    fn test_rc_of_different_types() {
        let rc_type = RcType::new(Type::Str);
        // RcType should work with different base types
        assert_eq!(rc_type.refcount_offset(), 0);
    }

    #[test]
    fn test_box_type_equality() {
        let box1 = BoxType::new(Type::I32);
        let box2 = BoxType::new(Type::I32);
        let box3 = BoxType::mutable(Type::I32);

        assert_eq!(box1, box2);
        assert_ne!(box1, box3);
    }

    #[test]
    fn test_rc_type_equality() {
        let rc1 = RcType::new(Type::Str);
        let rc2 = RcType::new(Type::Str);
        let rc3 = RcType::new(Type::I32);

        assert_eq!(rc1, rc2);
        assert_ne!(rc1, rc3);
    }

    #[test]
    fn test_arc_type_equality() {
        let arc1 = ArcType::new(Type::Bool);
        let arc2 = ArcType::new(Type::Bool);
        let arc3 = ArcType::new(Type::I32);

        assert_eq!(arc1, arc2);
        assert_ne!(arc1, arc3);
    }
}
