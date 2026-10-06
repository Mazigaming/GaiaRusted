//! Shareable mutable containers: values changed through a shared reference.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Deref, DerefMut};

/// The one way to change a value through a `&` reference: everything else
/// in this module is built on it.
pub struct UnsafeCell<T: ?Sized> {
    value: T,
}

impl<T> UnsafeCell<T> {
    pub fn new(value: T) -> UnsafeCell<T> {
        UnsafeCell { value }
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: ?Sized> UnsafeCell<T> {
    /// A pointer to the value, which may be written through.
    pub fn get(&self) -> *mut T {
        &self.value as *const T as *mut T
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

/// A value replaced as a whole through a shared reference.
pub struct Cell<T: ?Sized> {
    value: UnsafeCell<T>,
}

impl<T> Cell<T> {
    pub fn new(value: T) -> Cell<T> {
        Cell { value: UnsafeCell::new(value) }
    }

    pub fn set(&self, value: T) {
        let old = self.replace(value);
        drop(old);
    }

    /// Put `value` in and return what was there.
    pub fn replace(&self, value: T) -> T {
        unsafe { std::ptr::replace(self.value.get(), value) }
    }

    /// Exchange the values of two cells.
    pub fn swap(&self, other: &Cell<T>) {
        if self as *const Cell<T> as usize != other as *const Cell<T> as usize {
            unsafe { std::ptr::swap(self.value.get(), other.value.get()) }
        }
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }
}

impl<T: Copy> Cell<T> {
    pub fn get(&self) -> T {
        unsafe { *self.value.get() }
    }

    /// Replace the value with `f` of it, and return the new one.
    pub fn update<F: FnOnce(T) -> T>(&self, f: F) -> T {
        let new = f(self.get());
        self.set(new);
        new
    }
}

impl<T: Default> Cell<T> {
    pub fn take(&self) -> T {
        self.replace(T::default())
    }
}

impl<T: Copy> Clone for Cell<T> {
    fn clone(&self) -> Cell<T> {
        Cell::new(self.get())
    }
}

impl<T: Default> Default for Cell<T> {
    fn default() -> Cell<T> {
        Cell::new(T::default())
    }
}

impl<T: Copy + PartialEq> PartialEq for Cell<T> {
    fn eq(&self, other: &Cell<T>) -> bool {
        self.get() == other.get()
    }
}

impl<T: Copy + Eq> Eq for Cell<T> {}

impl<T: Copy + PartialOrd> PartialOrd for Cell<T> {
    fn partial_cmp(&self, other: &Cell<T>) -> Option<Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<T: Copy + Ord> Ord for Cell<T> {
    fn cmp(&self, other: &Cell<T>) -> Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: Copy + fmt::Debug> fmt::Debug for Cell<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Cell").field("value", &self.get()).finish()
    }
}

/// How a `RefCell` is borrowed: by this many `Ref`s, or by one `RefMut`.
const UNUSED: isize = 0;
const WRITING: isize = -1;

/// A value borrowed through a shared reference, with Rust's borrowing
/// rules checked as the program runs: any number of `Ref`s or one `RefMut`
/// at a time, or a panic.
pub struct RefCell<T: ?Sized> {
    borrow: Cell<isize>,
    value: UnsafeCell<T>,
}

/// `try_borrow` found the value mutably borrowed.
pub struct BorrowError {
    _private: (),
}

/// `try_borrow_mut` found the value already borrowed.
pub struct BorrowMutError {
    _private: (),
}

impl fmt::Debug for BorrowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("BorrowError")
    }
}

impl fmt::Display for BorrowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("already mutably borrowed")
    }
}

impl fmt::Debug for BorrowMutError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("BorrowMutError")
    }
}

impl fmt::Display for BorrowMutError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("already borrowed")
    }
}

impl<T> RefCell<T> {
    pub fn new(value: T) -> RefCell<T> {
        RefCell { borrow: Cell::new(UNUSED), value: UnsafeCell::new(value) }
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    /// Put `value` in and return what was there.
    pub fn replace(&self, value: T) -> T {
        std::mem::replace(&mut *self.borrow_mut(), value)
    }

    pub fn replace_with<F: FnOnce(&mut T) -> T>(&self, f: F) -> T {
        let mut borrowed = self.borrow_mut();
        let value = f(&mut *borrowed);
        std::mem::replace(&mut *borrowed, value)
    }

    /// Exchange the values of two cells.
    pub fn swap(&self, other: &RefCell<T>) {
        std::mem::swap(&mut *self.borrow_mut(), &mut *other.borrow_mut())
    }
}

impl<T: Default> RefCell<T> {
    pub fn take(&self) -> T {
        self.replace(T::default())
    }
}

impl<T: ?Sized> RefCell<T> {
    /// Borrow the value; panics if it is mutably borrowed.
    pub fn borrow(&self) -> Ref<'_, T> {
        match self.try_borrow() {
            Ok(borrowed) => borrowed,
            Err(error) => panic!("already mutably borrowed: {:?}", error),
        }
    }

    pub fn try_borrow(&self) -> Result<Ref<'_, T>, BorrowError> {
        let count = self.borrow.get();
        if count == WRITING {
            return Err(BorrowError { _private: () });
        }
        self.borrow.set(count + 1);
        Ok(Ref { value: self.value.get() as *const T, borrow: &self.borrow })
    }

    /// Borrow the value mutably; panics if it is borrowed at all.
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        match self.try_borrow_mut() {
            Ok(borrowed) => borrowed,
            Err(error) => panic!("already borrowed: {:?}", error),
        }
    }

    pub fn try_borrow_mut(&self) -> Result<RefMut<'_, T>, BorrowMutError> {
        if self.borrow.get() != UNUSED {
            return Err(BorrowMutError { _private: () });
        }
        self.borrow.set(WRITING);
        Ok(RefMut { value: self.value.get(), borrow: &self.borrow })
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }
}

impl<T: Clone> Clone for RefCell<T> {
    fn clone(&self) -> RefCell<T> {
        RefCell::new(self.borrow().clone())
    }
}

impl<T: Default> Default for RefCell<T> {
    fn default() -> RefCell<T> {
        RefCell::new(T::default())
    }
}

impl<T: ?Sized + PartialEq> PartialEq for RefCell<T> {
    fn eq(&self, other: &RefCell<T>) -> bool {
        *self.borrow() == *other.borrow()
    }
}

impl<T: ?Sized + Eq> Eq for RefCell<T> {}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RefCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut fields = f.debug_struct("RefCell");
        match self.try_borrow() {
            Ok(borrowed) => fields.field("value", &&*borrowed),
            Err(_) => fields.field("value", &BorrowedPlaceholder),
        };
        fields.finish()
    }
}

/// What `{:?}` shows for a `RefCell` that is mutably borrowed.
struct BorrowedPlaceholder;

impl fmt::Debug for BorrowedPlaceholder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("<borrowed>")
    }
}

/// A shared borrow of the value in a `RefCell`, released when dropped.
pub struct Ref<'b, T: ?Sized> {
    value: *const T,
    borrow: &'b Cell<isize>,
}

impl<'b, T: ?Sized> Ref<'b, T> {
    pub fn clone(original: &Ref<'b, T>) -> Ref<'b, T> {
        original.borrow.set(original.borrow.get() + 1);
        Ref { value: original.value, borrow: original.borrow }
    }

    /// A borrow of part of the value.
    pub fn map<U: ?Sized, F: FnOnce(&T) -> &U>(original: Ref<'b, T>, f: F) -> Ref<'b, U> {
        let value = f(unsafe { &*original.value }) as *const U;
        let borrow = original.borrow;
        std::mem::forget(original);
        Ref { value, borrow }
    }
}

impl<'b, T: ?Sized> Deref for Ref<'b, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.value }
    }
}

impl<'b, T: ?Sized> Drop for Ref<'b, T> {
    fn drop(&mut self) {
        self.borrow.set(self.borrow.get() - 1);
    }
}

impl<'b, T: ?Sized + fmt::Debug> fmt::Debug for Ref<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'b, T: ?Sized + fmt::Display> fmt::Display for Ref<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

/// A mutable borrow of the value in a `RefCell`, released when dropped.
pub struct RefMut<'b, T: ?Sized> {
    value: *mut T,
    borrow: &'b Cell<isize>,
}

impl<'b, T: ?Sized> RefMut<'b, T> {
    /// A borrow of part of the value.
    pub fn map<U: ?Sized, F: FnOnce(&mut T) -> &mut U>(original: RefMut<'b, T>, f: F) -> RefMut<'b, U> {
        let value = f(unsafe { &mut *original.value }) as *mut U;
        let borrow = original.borrow;
        std::mem::forget(original);
        RefMut { value, borrow }
    }
}

impl<'b, T: ?Sized> Deref for RefMut<'b, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.value }
    }
}

impl<'b, T: ?Sized> DerefMut for RefMut<'b, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value }
    }
}

impl<'b, T: ?Sized> Drop for RefMut<'b, T> {
    fn drop(&mut self) {
        self.borrow.set(UNUSED);
    }
}

impl<'b, T: ?Sized + fmt::Debug> fmt::Debug for RefMut<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'b, T: ?Sized + fmt::Display> fmt::Display for RefMut<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
