//! Working with values as memory.

pub fn size_of<T>() -> usize {
    std::intrinsics::size_of::<T>()
}

/// Does dropping a `T` do anything?
pub fn needs_drop<T>() -> bool {
    std::intrinsics::needs_drop::<T>()
}

/// Put `value` into `dest` and return what was there before.
pub fn replace<T>(dest: &mut T, value: T) -> T {
    unsafe {
        let old = std::intrinsics::read(dest);
        std::intrinsics::write(dest, value);
        old
    }
}

pub fn swap<T>(a: &mut T, b: &mut T) {
    unsafe {
        let saved = std::intrinsics::read(a);
        std::intrinsics::write(a, std::intrinsics::read(b));
        std::intrinsics::write(b, saved);
    }
}

/// Destroy a value now instead of at the end of its scope.
pub fn drop<T>(value: T) {}

/// Give up a value without running its destructor.
pub fn forget<T>(value: T) {
    std::intrinsics::forget(value);
}

pub fn take<T: Default>(dest: &mut T) -> T {
    replace(dest, Default::default())
}

pub fn align_of<T>() -> usize {
    std::intrinsics::align_of::<T>()
}

/// The size of the value `value` points to.
pub fn size_of_val<T>(value: &T) -> usize {
    std::intrinsics::size_of::<T>()
}

/// Reinterpret the bits of `value` as a `Dst`, which must be the same size.
pub unsafe fn transmute<Src, Dst>(value: Src) -> Dst {
    if std::intrinsics::size_of::<Src>() != std::intrinsics::size_of::<Dst>() {
        panic!("cannot transmute between types of different sizes");
    }
    let source = ManuallyDrop::new(value);
    std::intrinsics::read(&source.value as *const Src as *const Dst)
}

/// A value whose destructor does not run by itself: its owner decides
/// when, if ever, what it holds is dropped.
pub struct ManuallyDrop<T> {
    value: T,
}

impl<T> ManuallyDrop<T> {
    pub fn new(value: T) -> ManuallyDrop<T> {
        ManuallyDrop { value }
    }

    pub fn into_inner(slot: ManuallyDrop<T>) -> T {
        slot.value
    }

    /// Move the value out, leaving the slot logically uninitialised.
    pub unsafe fn take(slot: &mut ManuallyDrop<T>) -> T {
        std::intrinsics::read(&slot.value as *const T)
    }

    /// Run the value's destructor now.
    pub unsafe fn drop(slot: &mut ManuallyDrop<T>) {
        std::intrinsics::drop_in_place(&mut slot.value as *mut T)
    }
}

impl<T> std::ops::Deref for ManuallyDrop<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> std::ops::DerefMut for ManuallyDrop<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T: Clone> Clone for ManuallyDrop<T> {
    fn clone(&self) -> ManuallyDrop<T> {
        ManuallyDrop::new(self.value.clone())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for ManuallyDrop<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("ManuallyDrop").field("value", &self.value).finish()
    }
}

/// Memory for a `T` that may not hold one yet.
pub struct MaybeUninit<T> {
    value: ManuallyDrop<T>,
}

impl<T> MaybeUninit<T> {
    pub fn uninit() -> MaybeUninit<T> {
        MaybeUninit { value: ManuallyDrop::new(std::intrinsics::uninit()) }
    }

    pub fn new(value: T) -> MaybeUninit<T> {
        MaybeUninit { value: ManuallyDrop::new(value) }
    }

    /// Store a value, without dropping whatever was there.
    pub fn write(&mut self, value: T) -> &mut T {
        std::intrinsics::write(self.as_mut_ptr(), value);
        unsafe { &mut *self.as_mut_ptr() }
    }

    pub fn as_ptr(&self) -> *const T {
        &self.value.value as *const T
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        &mut self.value.value as *mut T
    }

    pub unsafe fn assume_init(self) -> T {
        ManuallyDrop::into_inner(self.value)
    }

    pub unsafe fn assume_init_read(&self) -> T {
        std::intrinsics::read(self.as_ptr())
    }

    pub unsafe fn assume_init_ref(&self) -> &T {
        &*self.as_ptr()
    }

    pub unsafe fn assume_init_mut(&mut self) -> &mut T {
        &mut *self.as_mut_ptr()
    }
}
