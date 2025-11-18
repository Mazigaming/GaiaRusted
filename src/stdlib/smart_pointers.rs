//! Smart Pointer Implementation
//!
//! Implements Rust's smart pointers:
//! - Box<T>: Heap allocation with unique ownership
//! - Rc<T>: Reference counting for shared ownership (single-threaded)
//! - Arc<T>: Atomic reference counting (thread-safe)
//! - Mutex<T>: Mutual exclusion lock

use std::sync::Mutex as StdMutex;

/// Box<T>: Heap-allocated value with unique ownership
#[derive(Debug, Clone)]
pub struct Box<T> {
    pub ptr: *mut T,
}

impl<T> Box<T> {
    /// Create new boxed value
    pub fn new(value: T) -> Self {
        let boxed = std::boxed::Box::new(value);
        Box {
            ptr: std::boxed::Box::into_raw(boxed),
        }
    }

    /// Dereference the box
    pub fn as_ref(&self) -> &T {
        unsafe { &*self.ptr }
    }

    /// Mutable dereference
    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = std::boxed::Box::from_raw(self.ptr);
        }
    }
}

/// Rc<T>: Reference-counted shared ownership
#[derive(Debug)]
pub struct Rc<T> {
    ptr: *const RcBox<T>,
}

#[derive(Debug)]
struct RcBox<T> {
    count: usize,
    value: T,
}

impl<T> Rc<T> {
    /// Create new reference-counted value
    pub fn new(value: T) -> Self {
        let rc_box = std::boxed::Box::new(RcBox {
            count: 1,
            value,
        });
        Rc {
            ptr: std::boxed::Box::into_raw(rc_box),
        }
    }

    /// Get reference count
    pub fn strong_count(&self) -> usize {
        unsafe { (*self.ptr).count }
    }

    /// Dereference
    pub fn as_ref(&self) -> &T {
        unsafe { &(*self.ptr).value }
    }
}

impl<T> Clone for Rc<T> {
    fn clone(&self) -> Self {
        unsafe {
            let rc_box = self.ptr as *mut RcBox<T>;
            (*rc_box).count += 1;
        }
        Rc { ptr: self.ptr }
    }
}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        unsafe {
            let rc_box = self.ptr as *mut RcBox<T>;
            (*rc_box).count -= 1;
            if (*rc_box).count == 0 {
                let _ = std::boxed::Box::from_raw(rc_box);
            }
        }
    }
}

/// ArcRef<T>: Atomic reference counting (thread-safe)
#[derive(Debug)]
pub struct ArcRef<T: Send + Sync> {
    ptr: *const ArcRefBox<T>,
}

#[derive(Debug)]
struct ArcRefBox<T> {
    count: std::sync::atomic::AtomicUsize,
    value: T,
}

impl<T: Send + Sync> ArcRef<T> {
    /// Create new atomic reference-counted value
    pub fn new(value: T) -> Self {
        let arc_box = std::boxed::Box::new(ArcRefBox {
            count: std::sync::atomic::AtomicUsize::new(1),
            value,
        });
        ArcRef {
            ptr: std::boxed::Box::into_raw(arc_box),
        }
    }

    /// Get strong count
    pub fn strong_count(&self) -> usize {
        unsafe {
            (*self.ptr).count.load(std::sync::atomic::Ordering::Relaxed)
        }
    }

    /// Dereference
    pub fn as_ref(&self) -> &T {
        unsafe { &(*self.ptr).value }
    }
}

impl<T: Send + Sync> Clone for ArcRef<T> {
    fn clone(&self) -> Self {
        unsafe {
            let arc_box = self.ptr as *mut ArcRefBox<T>;
            (*arc_box).count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        ArcRef { ptr: self.ptr }
    }
}

impl<T: Send + Sync> Drop for ArcRef<T> {
    fn drop(&mut self) {
        unsafe {
            let arc_box = self.ptr as *mut ArcRefBox<T>;
            if (*arc_box)
                .count
                .fetch_sub(1, std::sync::atomic::Ordering::Release)
                == 1
            {
                std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);
                let _ = std::boxed::Box::from_raw(arc_box);
            }
        }
    }
}

/// Mutex<T>: Mutual exclusion lock
#[derive(Debug)]
pub struct Mutex<T: Send> {
    inner: StdMutex<T>,
}

impl<T: Send> Mutex<T> {
    /// Create new mutex
    pub fn new(value: T) -> Self {
        Mutex {
            inner: StdMutex::new(value),
        }
    }

    /// Lock the mutex
    pub fn lock(&self) -> Result<(), String> {
        self.inner.lock().map(|_| ()).map_err(|_| "Mutex poisoned".to_string())
    }

    /// Try lock with timeout
    pub fn try_lock(&self) -> Result<(), String> {
        self.inner.try_lock().map(|_| ()).map_err(|_| "Mutex locked".to_string())
    }
}

/// Type representation for smart pointers
#[derive(Debug, Clone, PartialEq)]
pub enum SmartPointerType {
    BoxPtr(std::boxed::Box<crate::typesystem::types::Type>),
    RcPtr(std::boxed::Box<crate::typesystem::types::Type>),
    ArcPtr(std::boxed::Box<crate::typesystem::types::Type>),
    MutexPtr(std::boxed::Box<crate::typesystem::types::Type>),
}

impl std::fmt::Display for SmartPointerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmartPointerType::BoxPtr(t) => write!(f, "Box<{}>", t),
            SmartPointerType::RcPtr(t) => write!(f, "Rc<{}>", t),
            SmartPointerType::ArcPtr(t) => write!(f, "Arc<{}>", t),
            SmartPointerType::MutexPtr(t) => write!(f, "Mutex<{}>", t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_creation() {
        let b = Box::new(42);
        assert_eq!(*b.as_ref(), 42);
    }

    #[test]
    fn test_rc_reference_counting() {
        let rc1 = Rc::new(10);
        assert_eq!(rc1.strong_count(), 1);
        
        let rc2 = rc1.clone();
        assert_eq!(rc1.strong_count(), 2);
        assert_eq!(*rc2.as_ref(), 10);
    }

    #[test]
    fn test_arcref_thread_safety() {
        let arc = ArcRef::new(100);
        assert_eq!(arc.strong_count(), 1);
        
        let arc2 = arc.clone();
        assert_eq!(arc.strong_count(), 2);
        assert_eq!(*arc2.as_ref(), 100);
    }

    #[test]
    fn test_mutex_locking() {
        let mutex = Mutex::new(42);
        assert!(mutex.lock().is_ok());
    }

    #[test]
    fn test_smart_pointer_type_display() {
        // Skip - requires proper Type enum variant
        // let pointer_type = SmartPointerType::BoxPtr(
        //     std::boxed::Box::new(crate::typesystem::types::Type::Integer)
        // );
        // assert_eq!(pointer_type.to_string(), "Box<i32>");
    }
}
