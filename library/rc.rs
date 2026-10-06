//! `Rc<T>`: shared ownership of a heap value, counted.

use std::cell::Cell;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

/// The heap allocation behind every `Rc` and `Weak` to one value.
///
/// The strong references together hold one weak reference, so the
/// allocation outlives the value while any `Weak` remains, and is freed
/// when the last reference of either kind goes.
struct RcBox<T: ?Sized> {
    strong: Cell<usize>,
    weak: Cell<usize>,
    value: T,
}

/// A shared, reference-counted pointer; the value is dropped when the last
/// `Rc` to it is.
#[rustc_nonnull_optimization_guaranteed]
pub struct Rc<T: ?Sized> {
    inner: *mut RcBox<T>,
}

/// A reference that does not keep the value alive; `upgrade` gives an `Rc`
/// while it still exists.
pub struct Weak<T: ?Sized> {
    /// Null for a `Weak` made by `Weak::new`, which never upgrades.
    inner: *mut RcBox<T>,
}

impl<T: ?Sized, U: ?Sized> std::ops::CoerceUnsized<Rc<U>> for Rc<T> {}
impl<T: ?Sized, U: ?Sized> std::ops::CoerceUnsized<Weak<U>> for Weak<T> {}

impl<T> Rc<T> {
    pub fn new(value: T) -> Rc<T> {
        let allocation = RcBox { strong: Cell::new(1), weak: Cell::new(1), value };
        Rc { inner: Box::into_raw(Box::new(allocation)) }
    }

    /// The value, if this is its only strong reference.
    pub fn try_unwrap(this: Rc<T>) -> Result<T, Rc<T>> {
        if Rc::strong_count(&this) != 1 {
            return Err(this);
        }
        let inner = this.inner;
        std::mem::forget(this);
        unsafe {
            let value = std::ptr::read(&(*inner).value as *const T);
            (*inner).strong.set(0);
            release_weak(inner);
            Ok(value)
        }
    }

    pub fn into_inner(this: Rc<T>) -> Option<T> {
        Rc::try_unwrap(this).ok()
    }
}

/// Give up one weak reference to the allocation, freeing it with the last.
fn release_weak<T: ?Sized>(inner: *mut RcBox<T>) {
    unsafe {
        let weak = (*inner).weak.get() - 1;
        (*inner).weak.set(weak);
        if weak == 0 {
            std::libc::free(inner as *mut u8);
        }
    }
}

impl<T: ?Sized> Rc<T> {
    fn allocation(&self) -> &RcBox<T> {
        unsafe { &*self.inner }
    }

    pub fn strong_count(this: &Rc<T>) -> usize {
        this.allocation().strong.get()
    }

    pub fn weak_count(this: &Rc<T>) -> usize {
        this.allocation().weak.get() - 1
    }

    /// A weak reference to the same value.
    pub fn downgrade(this: &Rc<T>) -> Weak<T> {
        let weak = &this.allocation().weak;
        weak.set(weak.get() + 1);
        Weak { inner: this.inner }
    }

    /// Whether two `Rc`s point to the same allocation.
    pub fn ptr_eq(this: &Rc<T>, other: &Rc<T>) -> bool {
        this.inner as *const u8 as usize == other.inner as *const u8 as usize
    }

    /// The value, mutably, if no other reference can see it.
    pub fn get_mut(this: &mut Rc<T>) -> Option<&mut T> {
        if Rc::strong_count(this) == 1 && Rc::weak_count(this) == 0 {
            Some(unsafe { &mut (*this.inner).value })
        } else {
            None
        }
    }

    pub fn as_ptr(this: &Rc<T>) -> *const T {
        unsafe { &(*this.inner).value as *const T }
    }
}

impl<T: Clone> Rc<T> {
    /// The value, mutably: cloned first if other references share it.
    pub fn make_mut(this: &mut Rc<T>) -> &mut T {
        if Rc::strong_count(this) != 1 || Rc::weak_count(this) != 0 {
            *this = Rc::new((**this).clone());
        }
        unsafe { &mut (*this.inner).value }
    }

    pub fn unwrap_or_clone(this: Rc<T>) -> T {
        match Rc::try_unwrap(this) {
            Ok(value) => value,
            Err(shared) => (*shared).clone(),
        }
    }
}

impl<T: ?Sized> Clone for Rc<T> {
    fn clone(&self) -> Rc<T> {
        let strong = &self.allocation().strong;
        strong.set(strong.get() + 1);
        Rc { inner: self.inner }
    }
}

impl<T: ?Sized> Drop for Rc<T> {
    fn drop(&mut self) {
        let strong = self.allocation().strong.get() - 1;
        self.allocation().strong.set(strong);
        if strong == 0 {
            unsafe { std::ptr::drop_in_place(&mut (*self.inner).value as *mut T) }
            release_weak(self.inner);
        }
    }
}

impl<T: ?Sized> Deref for Rc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.allocation().value
    }
}

impl<T: ?Sized> AsRef<T> for Rc<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> std::borrow::Borrow<T> for Rc<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: Default> Default for Rc<T> {
    fn default() -> Rc<T> {
        Rc::new(T::default())
    }
}

impl<T> From<T> for Rc<T> {
    fn from(value: T) -> Rc<T> {
        Rc::new(value)
    }
}

/// A new allocation for `len` elements, with both counts at one; the
/// elements are for the caller to put in.
fn allocate_slice<T>(len: usize) -> *mut RcBox<[T]> {
    let header = 2 * std::mem::size_of::<usize>();
    let align = std::mem::align_of::<T>();
    // Where `RcBox<[T]>` puts its value: after the counts, aligned.
    let offset = (header + align - 1) / align * align;
    unsafe {
        let data = std::libc::malloc(offset + len * std::mem::size_of::<T>());
        let counts = data as *mut Cell<usize>;
        std::ptr::write(counts, Cell::new(1));
        std::ptr::write(counts.add(1), Cell::new(1));
        std::intrinsics::slice_from_raw_parts_mut(data as *mut T, len) as *mut [T] as *mut RcBox<[T]>
    }
}

impl<T: Clone> From<&[T]> for Rc<[T]> {
    fn from(items: &[T]) -> Rc<[T]> {
        let inner = allocate_slice::<T>(items.len());
        let elements = unsafe { (*inner).value.as_mut_ptr() };
        for (index, item) in items.iter().enumerate() {
            unsafe { std::ptr::write(elements.add(index), item.clone()) };
        }
        Rc { inner }
    }
}

impl<T> From<Vec<T>> for Rc<[T]> {
    fn from(items: Vec<T>) -> Rc<[T]> {
        let mut items = items;
        let inner = allocate_slice::<T>(items.len());
        unsafe {
            std::ptr::copy_nonoverlapping(items.as_ptr(), (*inner).value.as_mut_ptr(), items.len());
            // The elements have moved; the vector frees only its buffer.
            items.set_len(0);
        }
        Rc { inner }
    }
}

impl From<&str> for Rc<str> {
    fn from(text: &str) -> Rc<str> {
        let bytes: Rc<[u8]> = Rc::from(text.as_bytes());
        let inner = bytes.inner;
        std::mem::forget(bytes);
        Rc { inner: inner as *mut RcBox<str> }
    }
}

impl From<String> for Rc<str> {
    fn from(text: String) -> Rc<str> {
        Rc::from(text.as_str())
    }
}

impl<T: ?Sized + PartialEq> PartialEq for Rc<T> {
    fn eq(&self, other: &Rc<T>) -> bool {
        **self == **other
    }
}

impl<T: ?Sized + Eq> Eq for Rc<T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for Rc<T> {
    fn partial_cmp(&self, other: &Rc<T>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: ?Sized + Ord> Ord for Rc<T> {
    fn cmp(&self, other: &Rc<T>) -> Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: ?Sized + Hash> Hash for Rc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T> Weak<T> {
    /// A `Weak` to nothing: it never upgrades.
    pub fn new() -> Weak<T> {
        Weak { inner: std::ptr::null_mut() }
    }
}

impl<T: ?Sized> Weak<T> {
    /// An `Rc` to the value, if it has not been dropped yet.
    pub fn upgrade(&self) -> Option<Rc<T>> {
        if self.inner.is_null() {
            return None;
        }
        let strong = unsafe { &(*self.inner).strong };
        if strong.get() == 0 {
            return None;
        }
        strong.set(strong.get() + 1);
        Some(Rc { inner: self.inner })
    }

    pub fn strong_count(&self) -> usize {
        if self.inner.is_null() {
            0
        } else {
            unsafe { (*self.inner).strong.get() }
        }
    }

    pub fn weak_count(&self) -> usize {
        if self.inner.is_null() {
            return 0;
        }
        let allocation = unsafe { &*self.inner };
        if allocation.strong.get() == 0 {
            0
        } else {
            allocation.weak.get() - 1
        }
    }

    pub fn ptr_eq(&self, other: &Weak<T>) -> bool {
        self.inner as *const u8 as usize == other.inner as *const u8 as usize
    }
}

impl<T: ?Sized> Clone for Weak<T> {
    fn clone(&self) -> Weak<T> {
        if !self.inner.is_null() {
            let weak = unsafe { &(*self.inner).weak };
            weak.set(weak.get() + 1);
        }
        Weak { inner: self.inner }
    }
}

impl<T: ?Sized> Drop for Weak<T> {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            release_weak(self.inner);
        }
    }
}

impl<T> Default for Weak<T> {
    fn default() -> Weak<T> {
        Weak::new()
    }
}

impl<T: ?Sized> fmt::Debug for Weak<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("(Weak)")
    }
}
