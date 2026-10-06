//! Raw pointers.

pub fn null<T>() -> *const T {
    0usize as *const T
}

pub fn null_mut<T>() -> *mut T {
    0usize as *mut T
}

/// Copy `count` values from `source` to `dest`. The two ranges may overlap;
/// the values are moved bitwise, so `source` is left logically uninitialised.
pub unsafe fn copy<T>(source: *const T, dest: *mut T, count: usize) {
    std::libc::memmove(dest as *mut u8, source as *const u8, count * std::intrinsics::size_of::<T>());
}

/// Like [`copy`], for ranges known not to overlap.
pub unsafe fn copy_nonoverlapping<T>(source: *const T, dest: *mut T, count: usize) {
    std::libc::memcpy(dest as *mut u8, source as *const u8, count * std::intrinsics::size_of::<T>());
}

pub unsafe fn read<T>(source: *const T) -> T {
    std::intrinsics::read(source)
}

/// Like [`read`], from an address that need not be aligned for `T`: the
/// processor this compiles for reads any address.
pub unsafe fn read_unaligned<T>(source: *const T) -> T {
    std::intrinsics::read(source)
}

pub unsafe fn write<T>(dest: *mut T, value: T) {
    std::intrinsics::write(dest, value)
}

/// Put `value` at `*dest` and return what was there.
pub unsafe fn replace<T>(dest: *mut T, value: T) -> T {
    let old = std::intrinsics::read(dest);
    std::intrinsics::write(dest, value);
    old
}

/// Exchange the values at two places, which may be the same.
pub unsafe fn swap<T>(a: *mut T, b: *mut T) {
    let saved = std::intrinsics::read(a);
    copy(b as *const T, a, 1);
    std::intrinsics::write(b, saved);
}

pub unsafe fn drop_in_place<T>(object: *mut T) {
    std::intrinsics::drop_in_place(object)
}

/// Whether two pointers hold the same address.
pub fn eq<T>(a: *const T, b: *const T) -> bool {
    a as usize == b as usize
}

impl<T> *const T {
    pub fn is_null(self) -> bool {
        self as usize == 0
    }

    /// The pointer `count` elements further on.
    pub fn add(self, count: usize) -> *const T {
        (self as usize + count * std::intrinsics::size_of::<T>()) as *const T
    }

    pub fn sub(self, count: usize) -> *const T {
        (self as usize - count * std::intrinsics::size_of::<T>()) as *const T
    }

    /// Copy the value out, leaving ownership of the original untouched.
    pub fn read(self) -> T {
        unsafe { std::intrinsics::read(self) }
    }
}

impl<T> *mut T {
    pub fn is_null(self) -> bool {
        self as usize == 0
    }

    pub fn add(self, count: usize) -> *mut T {
        (self as usize + count * std::intrinsics::size_of::<T>()) as *mut T
    }

    pub fn sub(self, count: usize) -> *mut T {
        (self as usize - count * std::intrinsics::size_of::<T>()) as *mut T
    }

    /// Copy the value out, leaving ownership of the original untouched.
    pub fn read(self) -> T {
        unsafe { std::intrinsics::read(self) }
    }

    /// Store a value without dropping what was there before.
    pub fn write(self, value: T) {
        unsafe { std::intrinsics::write(self, value) }
    }

    /// Run the destructor of the value pointed to.
    pub fn drop_in_place(self) {
        unsafe { std::intrinsics::drop_in_place(self) }
    }
}
