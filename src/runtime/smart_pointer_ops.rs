//! # Smart Pointer Runtime Operations
//!
//! Provides runtime support for smart pointer operations:
//! - Memory allocation and deallocation
//! - Reference counting operations
//! - Atomic operations for Arc<T>

use std::sync::atomic::{AtomicUsize, Ordering};
use std::ptr;

/// Allocation metadata for smart pointers
#[derive(Debug, Clone)]
pub struct AllocationMetadata {
    /// Total allocated size in bytes
    pub size: usize,
    /// Type name for debugging
    pub type_name: String,
}

/// Allocate memory on the heap for a smart pointer
/// 
/// # Arguments
/// * `size` - Number of bytes to allocate
/// * `alignment` - Alignment requirement in bytes
/// 
/// # Returns
/// Pointer to allocated memory, or null if allocation fails
#[inline]
pub fn smart_malloc(size: usize, _alignment: usize) -> *mut u8 {
    if size == 0 {
        return ptr::null_mut();
    }

    // Use Vec as a heap allocation mechanism
    let layout = std::alloc::Layout::from_size_align(size, 8)
        .unwrap_or_else(|_| std::alloc::Layout::new::<u8>());
    
    let ptr = unsafe {
        std::alloc::alloc(layout)
    };

    if ptr.is_null() {
        eprintln!("Smart pointer allocation failed for size: {}", size);
    }

    ptr
}

/// Free memory allocated by smart_malloc
///
/// # Safety
/// The pointer must be valid and previously allocated by smart_malloc
#[inline]
pub unsafe fn smart_free(ptr: *mut u8) {
    if !ptr.is_null() {
        let layout = std::alloc::Layout::from_size_align(256, 8).unwrap();
        std::alloc::dealloc(ptr, layout);
    }
}

/// Reference counter for Rc<T> (single-threaded)
pub struct SimpleRefCount {
    count: u32,
}

impl SimpleRefCount {
    /// Create a new reference counter initialized to 1
    pub fn new() -> Self {
        SimpleRefCount { count: 1 }
    }

    /// Increment the reference count
    ///
    /// # Returns
    /// The new count, or u32::MAX if overflow would occur
    pub fn increment(&mut self) -> u32 {
        if self.count == u32::MAX {
            return u32::MAX;
        }
        self.count += 1;
        self.count
    }

    /// Decrement the reference count
    ///
    /// # Returns
    /// The new count (0 if was 1)
    pub fn decrement(&mut self) -> u32 {
        if self.count > 0 {
            self.count -= 1;
        }
        self.count
    }

    /// Get the current count
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Check if this is the last reference
    pub fn is_unique(&self) -> bool {
        self.count == 1
    }
}

/// Atomic reference counter for Arc<T> (thread-safe)
pub struct AtomicRefCount {
    count: AtomicUsize,
}

impl AtomicRefCount {
    /// Create a new atomic reference counter initialized to 1
    pub fn new() -> Self {
        AtomicRefCount {
            count: AtomicUsize::new(1),
        }
    }

    /// Increment the reference count atomically
    ///
    /// # Returns
    /// The previous count value
    pub fn increment(&self) -> usize {
        // Fetch-add with relaxed ordering for performance
        self.count.fetch_add(1, Ordering::Relaxed)
    }

    /// Decrement the reference count atomically
    ///
    /// # Returns
    /// The new count value after decrement
    pub fn decrement(&self) -> usize {
        // Use release ordering for potential synchronization
        self.count.fetch_sub(1, Ordering::Release) - 1
    }

    /// Get the current count value
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Check if this is the last reference (count == 1)
    pub fn is_unique(&self) -> bool {
        self.count.load(Ordering::Relaxed) == 1
    }

    /// Try to increment, returns false if count is 0 (being dropped)
    pub fn try_increment(&self) -> bool {
        let mut current = self.count.load(Ordering::Acquire);
        loop {
            if current == 0 {
                return false; // Already dropped
            }
            match self.count.compare_exchange(
                current,
                current + 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }
}

/// Operations on Box<T> pointers
pub mod box_ops {
    use super::*;

    /// Allocate memory for a boxed value
    pub fn allocate(size: usize) -> *mut u8 {
        smart_malloc(size, 8) // 8-byte alignment
    }

    /// Deallocate memory from a box
    pub unsafe fn deallocate(ptr: *mut u8) {
        smart_free(ptr);
    }

    /// Get the data pointer (box itself is the pointer)
    pub fn data_ptr(box_ptr: *mut u8) -> *mut u8 {
        box_ptr
    }
}

/// Operations on Rc<T> pointers
pub mod rc_ops {
    use super::*;

    /// Memory layout: [refcount: u32] [data: T]
    const REFCOUNT_SIZE: usize = 4;
    const REFCOUNT_OFFSET: usize = 0;
    const DATA_OFFSET: usize = 4;

    /// Allocate memory for an Rc including refcount header
    pub fn allocate(data_size: usize) -> *mut u8 {
        let total_size = REFCOUNT_SIZE + data_size;
        let ptr = smart_malloc(total_size, 8);
        
        if !ptr.is_null() {
            // Initialize refcount to 1
            unsafe {
                *(ptr as *mut u32) = 1;
            }
        }
        
        ptr
    }

    /// Get the refcount header pointer
    pub unsafe fn refcount_ptr(rc_ptr: *mut u8) -> *mut u32 {
        rc_ptr.add(REFCOUNT_OFFSET) as *mut u32
    }

    /// Get the data pointer from an Rc
    pub unsafe fn data_ptr(rc_ptr: *mut u8) -> *mut u8 {
        rc_ptr.add(DATA_OFFSET)
    }

    /// Increment the reference count
    pub unsafe fn increment(rc_ptr: *mut u8) -> u32 {
        let refcount_ptr = refcount_ptr(rc_ptr);
        let mut old_count = *refcount_ptr;
        old_count = old_count.saturating_add(1);
        *refcount_ptr = old_count;
        old_count
    }

    /// Decrement the reference count
    ///
    /// # Returns
    /// true if count reached 0 (should deallocate)
    pub unsafe fn decrement(rc_ptr: *mut u8) -> bool {
        let refcount_ptr = refcount_ptr(rc_ptr);
        let count = (*refcount_ptr).saturating_sub(1);
        *refcount_ptr = count;
        count == 0
    }

    /// Get the current reference count
    pub unsafe fn count(rc_ptr: *mut u8) -> u32 {
        *(refcount_ptr(rc_ptr))
    }

    /// Deallocate an Rc (including header)
    pub unsafe fn deallocate(rc_ptr: *mut u8) {
        smart_free(rc_ptr);
    }
}

/// Operations on Arc<T> pointers
pub mod arc_ops {
    use super::*;

    /// Memory layout: [refcount: AtomicUsize] [data: T]
    const REFCOUNT_SIZE: usize = 8;
    const REFCOUNT_OFFSET: usize = 0;
    const DATA_OFFSET: usize = 8;

    /// Allocate memory for an Arc including atomic refcount header
    pub fn allocate(data_size: usize) -> *mut u8 {
        let total_size = REFCOUNT_SIZE + data_size;
        let ptr = smart_malloc(total_size, 8);
        
        if !ptr.is_null() {
            // Initialize atomic refcount to 1
            unsafe {
                let refcount_ptr = ptr as *mut AtomicUsize;
                *refcount_ptr = AtomicUsize::new(1);
            }
        }
        
        ptr
    }

    /// Get the refcount header pointer
    pub unsafe fn refcount_ptr(arc_ptr: *mut u8) -> *mut AtomicUsize {
        arc_ptr.add(REFCOUNT_OFFSET) as *mut AtomicUsize
    }

    /// Get the data pointer from an Arc
    pub unsafe fn data_ptr(arc_ptr: *mut u8) -> *mut u8 {
        arc_ptr.add(DATA_OFFSET)
    }

    /// Increment the reference count (atomic)
    pub unsafe fn increment(arc_ptr: *mut u8) -> usize {
        let refcount_ptr = refcount_ptr(arc_ptr);
        (*refcount_ptr).fetch_add(1, Ordering::Relaxed)
    }

    /// Decrement the reference count (atomic)
    ///
    /// # Returns
    /// true if count reached 0 (should deallocate)
    pub unsafe fn decrement(arc_ptr: *mut u8) -> bool {
        let refcount_ptr = refcount_ptr(arc_ptr);
        let old_count = (*refcount_ptr).fetch_sub(1, Ordering::Release);
        old_count == 1
    }

    /// Get the current reference count
    pub unsafe fn count(arc_ptr: *mut u8) -> usize {
        let refcount_ptr = refcount_ptr(arc_ptr);
        (*refcount_ptr).load(Ordering::Relaxed)
    }

    /// Deallocate an Arc (including header)
    pub unsafe fn deallocate(arc_ptr: *mut u8) {
        smart_free(arc_ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_refcount_new() {
        let rc = SimpleRefCount::new();
        assert_eq!(rc.count(), 1);
        assert!(rc.is_unique());
    }

    #[test]
    fn test_simple_refcount_increment() {
        let mut rc = SimpleRefCount::new();
        let new_count = rc.increment();
        assert_eq!(new_count, 2);
        assert!(!rc.is_unique());
    }

    #[test]
    fn test_simple_refcount_decrement() {
        let mut rc = SimpleRefCount::new();
        rc.increment();
        let new_count = rc.decrement();
        assert_eq!(new_count, 1);
        assert!(rc.is_unique());
    }

    #[test]
    fn test_atomic_refcount_new() {
        let arc = AtomicRefCount::new();
        assert_eq!(arc.count(), 1);
        assert!(arc.is_unique());
    }

    #[test]
    fn test_atomic_refcount_increment() {
        let arc = AtomicRefCount::new();
        let old_count = arc.increment();
        assert_eq!(old_count, 1);
        assert_eq!(arc.count(), 2);
    }

    #[test]
    fn test_atomic_refcount_decrement() {
        let arc = AtomicRefCount::new();
        arc.increment();
        let new_count = arc.decrement();
        assert_eq!(new_count, 1);
        assert!(arc.is_unique());
    }

    #[test]
    fn test_atomic_refcount_try_increment() {
        let arc = AtomicRefCount::new();
        assert!(arc.try_increment());
        assert_eq!(arc.count(), 2);
    }

    #[test]
    fn test_box_allocation() {
        let ptr = box_ops::allocate(64);
        assert!(!ptr.is_null());
        unsafe {
            box_ops::deallocate(ptr);
        }
    }

    #[test]
    fn test_rc_allocation() {
        let ptr = rc_ops::allocate(32);
        assert!(!ptr.is_null());
        unsafe {
            // Verify initial refcount is 1
            assert_eq!(rc_ops::count(ptr), 1);
        }
        // Note: actual increment/decrement tests deferred until malloc/free properly linked
    }

    #[test]
    fn test_arc_allocation() {
        let ptr = arc_ops::allocate(32);
        assert!(!ptr.is_null());
        unsafe {
            assert_eq!(arc_ops::count(ptr), 1);
            let old_count = arc_ops::increment(ptr);
            assert_eq!(old_count, 1);
            assert_eq!(arc_ops::count(ptr), 2);
            let should_free = arc_ops::decrement(ptr);
            assert!(!should_free);
            let should_free = arc_ops::decrement(ptr);
            assert!(should_free);
            arc_ops::deallocate(ptr);
        }
    }

    #[test]
    fn test_rc_multiple_owners() {
        let ptr = rc_ops::allocate(16);
        unsafe {
            assert_eq!(rc_ops::count(ptr), 1);
            rc_ops::increment(ptr);
            rc_ops::increment(ptr);
            assert_eq!(rc_ops::count(ptr), 3);
            
            // Decrement to 0
            rc_ops::decrement(ptr);
            assert_eq!(rc_ops::count(ptr), 2);
            rc_ops::decrement(ptr);
            assert_eq!(rc_ops::count(ptr), 1);
            let should_free = rc_ops::decrement(ptr);
            assert!(should_free);
            
            rc_ops::deallocate(ptr);
        }
    }
}
