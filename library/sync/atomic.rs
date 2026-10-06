//! Values shared between threads and changed indivisibly.
//!
//! Every operation is sequentially consistent, which satisfies whatever
//! `Ordering` is asked for: the machine routines behind them are full
//! barriers.

use std::cell::UnsafeCell;
use std::fmt;

/// How an atomic operation is ordered with the memory accesses around it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ordering {
    Relaxed,
    Release,
    Acquire,
    AcqRel,
    SeqCst,
}

/// A barrier between the memory accesses before and after it.
pub fn fence(order: Ordering) {
    let barrier = AtomicUsize::new(0);
    barrier.swap(0, order);
}

pub fn compiler_fence(order: Ordering) {}

/// Defines an atomic integer type over the machine routines for its size,
/// whose operands are of the unsigned type `$bits`.
macro_rules! atomic_int {
    ($name:ident, $t:ident, $bits:ident, $load:ident, $store:ident, $swap:ident, $add:ident, $exchange:ident) => {
        /// An integer shared between threads, changed with atomic operations.
        pub struct $name {
            value: UnsafeCell<$t>,
        }

        impl $name {
            pub const fn new(value: $t) -> $name {
                $name { value: UnsafeCell::new(value) }
            }

            fn address(&self) -> *mut $bits {
                self.value.get() as *mut $bits
            }

            pub fn load(&self, order: Ordering) -> $t {
                unsafe { std::libc::$load(self.address()) as $t }
            }

            pub fn store(&self, value: $t, order: Ordering) {
                unsafe { std::libc::$store(self.address(), value as $bits) }
            }

            /// Put `value` in and return what was there.
            pub fn swap(&self, value: $t, order: Ordering) -> $t {
                unsafe { std::libc::$swap(self.address(), value as $bits) as $t }
            }

            /// Add `value`, wrapping around; returns the previous value.
            pub fn fetch_add(&self, value: $t, order: Ordering) -> $t {
                unsafe { std::libc::$add(self.address(), value as $bits) as $t }
            }

            pub fn fetch_sub(&self, value: $t, order: Ordering) -> $t {
                self.fetch_add((0 as $t).wrapping_sub(value), order)
            }

            /// Store `new` if the value is `current`: `Ok` with the
            /// previous value if it was, `Err` with it otherwise.
            pub fn compare_exchange(&self, current: $t, new: $t, success: Ordering, failure: Ordering) -> Result<$t, $t> {
                let previous = unsafe { std::libc::$exchange(self.address(), current as $bits, new as $bits) as $t };
                if previous == current { Ok(previous) } else { Err(previous) }
            }

            pub fn compare_exchange_weak(&self, current: $t, new: $t, success: Ordering, failure: Ordering) -> Result<$t, $t> {
                self.compare_exchange(current, new, success, failure)
            }

            /// Replace the value by `update` of it, retrying until no other
            /// thread changed it in between; returns the previous value.
            pub fn fetch_update<F: FnMut($t) -> Option<$t>>(&self, set: Ordering, fetch: Ordering, mut update: F) -> Result<$t, $t> {
                let mut previous = self.load(fetch);
                loop {
                    let Some(next) = update(previous) else { return Err(previous) };
                    match self.compare_exchange(previous, next, set, fetch) {
                        Ok(value) => return Ok(value),
                        Err(value) => previous = value,
                    }
                }
            }

            pub fn fetch_and(&self, value: $t, order: Ordering) -> $t {
                self.fetch_update(order, order, |old| Some(old & value)).unwrap_or(0 as $t)
            }

            pub fn fetch_or(&self, value: $t, order: Ordering) -> $t {
                self.fetch_update(order, order, |old| Some(old | value)).unwrap_or(0 as $t)
            }

            pub fn fetch_xor(&self, value: $t, order: Ordering) -> $t {
                self.fetch_update(order, order, |old| Some(old ^ value)).unwrap_or(0 as $t)
            }

            pub fn fetch_max(&self, value: $t, order: Ordering) -> $t {
                self.fetch_update(order, order, |old| Some(old.max(value))).unwrap_or(0 as $t)
            }

            pub fn fetch_min(&self, value: $t, order: Ordering) -> $t {
                self.fetch_update(order, order, |old| Some(old.min(value))).unwrap_or(0 as $t)
            }

            pub fn get_mut(&mut self) -> &mut $t {
                self.value.get_mut()
            }

            pub fn into_inner(self) -> $t {
                self.value.into_inner()
            }

            pub fn as_ptr(&self) -> *mut $t {
                self.value.get()
            }
        }

        impl Default for $name {
            fn default() -> $name {
                $name::new(0 as $t)
            }
        }

        impl From<$t> for $name {
            fn from(value: $t) -> $name {
                $name::new(value)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Debug::fmt(&self.load(Ordering::SeqCst), f)
            }
        }
    };
}

atomic_int!(AtomicU8, u8, u8, __gaia_atomic_load_1, __gaia_atomic_store_1, __gaia_atomic_swap_1, __gaia_atomic_fetch_add_1, __gaia_atomic_compare_exchange_1);
atomic_int!(AtomicI8, i8, u8, __gaia_atomic_load_1, __gaia_atomic_store_1, __gaia_atomic_swap_1, __gaia_atomic_fetch_add_1, __gaia_atomic_compare_exchange_1);
atomic_int!(AtomicU32, u32, u32, __gaia_atomic_load_4, __gaia_atomic_store_4, __gaia_atomic_swap_4, __gaia_atomic_fetch_add_4, __gaia_atomic_compare_exchange_4);
atomic_int!(AtomicI32, i32, u32, __gaia_atomic_load_4, __gaia_atomic_store_4, __gaia_atomic_swap_4, __gaia_atomic_fetch_add_4, __gaia_atomic_compare_exchange_4);
atomic_int!(AtomicU64, u64, u64, __gaia_atomic_load_8, __gaia_atomic_store_8, __gaia_atomic_swap_8, __gaia_atomic_fetch_add_8, __gaia_atomic_compare_exchange_8);
atomic_int!(AtomicI64, i64, u64, __gaia_atomic_load_8, __gaia_atomic_store_8, __gaia_atomic_swap_8, __gaia_atomic_fetch_add_8, __gaia_atomic_compare_exchange_8);
atomic_int!(AtomicUsize, usize, u64, __gaia_atomic_load_8, __gaia_atomic_store_8, __gaia_atomic_swap_8, __gaia_atomic_fetch_add_8, __gaia_atomic_compare_exchange_8);
atomic_int!(AtomicIsize, isize, u64, __gaia_atomic_load_8, __gaia_atomic_store_8, __gaia_atomic_swap_8, __gaia_atomic_fetch_add_8, __gaia_atomic_compare_exchange_8);

/// A `bool` shared between threads.
pub struct AtomicBool {
    value: AtomicU8,
}

impl AtomicBool {
    pub const fn new(value: bool) -> AtomicBool {
        AtomicBool { value: AtomicU8::new(value as u8) }
    }

    pub fn load(&self, order: Ordering) -> bool {
        self.value.load(order) != 0
    }

    pub fn store(&self, value: bool, order: Ordering) {
        self.value.store(value as u8, order)
    }

    pub fn swap(&self, value: bool, order: Ordering) -> bool {
        self.value.swap(value as u8, order) != 0
    }

    pub fn compare_exchange(&self, current: bool, new: bool, success: Ordering, failure: Ordering) -> Result<bool, bool> {
        match self.value.compare_exchange(current as u8, new as u8, success, failure) {
            Ok(previous) => Ok(previous != 0),
            Err(previous) => Err(previous != 0),
        }
    }

    pub fn compare_exchange_weak(&self, current: bool, new: bool, success: Ordering, failure: Ordering) -> Result<bool, bool> {
        self.compare_exchange(current, new, success, failure)
    }

    pub fn fetch_and(&self, value: bool, order: Ordering) -> bool {
        self.value.fetch_and(value as u8, order) != 0
    }

    pub fn fetch_or(&self, value: bool, order: Ordering) -> bool {
        self.value.fetch_or(value as u8, order) != 0
    }

    pub fn fetch_xor(&self, value: bool, order: Ordering) -> bool {
        self.value.fetch_xor(value as u8, order) != 0
    }

    pub fn fetch_not(&self, order: Ordering) -> bool {
        self.fetch_xor(true, order)
    }

    pub fn get_mut(&mut self) -> &mut bool {
        unsafe { &mut *(self.value.as_ptr() as *mut bool) }
    }

    pub fn into_inner(self) -> bool {
        self.value.into_inner() != 0
    }
}

impl Default for AtomicBool {
    fn default() -> AtomicBool {
        AtomicBool::new(false)
    }
}

impl From<bool> for AtomicBool {
    fn from(value: bool) -> AtomicBool {
        AtomicBool::new(value)
    }
}

impl fmt::Debug for AtomicBool {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.load(Ordering::SeqCst), f)
    }
}

/// A raw pointer shared between threads.
pub struct AtomicPtr<T> {
    value: AtomicUsize,
    _points_to: std::marker::PhantomData<T>,
}

impl<T> AtomicPtr<T> {
    pub const fn new(pointer: *mut T) -> AtomicPtr<T> {
        AtomicPtr { value: AtomicUsize::new(pointer as usize), _points_to: std::marker::PhantomData }
    }

    pub fn load(&self, order: Ordering) -> *mut T {
        self.value.load(order) as *mut T
    }

    pub fn store(&self, pointer: *mut T, order: Ordering) {
        self.value.store(pointer as usize, order)
    }

    pub fn swap(&self, pointer: *mut T, order: Ordering) -> *mut T {
        self.value.swap(pointer as usize, order) as *mut T
    }

    pub fn compare_exchange(&self, current: *mut T, new: *mut T, success: Ordering, failure: Ordering) -> Result<*mut T, *mut T> {
        match self.value.compare_exchange(current as usize, new as usize, success, failure) {
            Ok(previous) => Ok(previous as *mut T),
            Err(previous) => Err(previous as *mut T),
        }
    }
}
