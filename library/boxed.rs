//! `Box<T>`: a value on the heap, owned through a pointer.
//!
//! The contents may be unsized (`Box<str>`, `Box<[T]>`, `Box<dyn Trait>`):
//! the pointer then carries their length or vtable.

#[rustc_nonnull_optimization_guaranteed]
pub struct Box<T: ?Sized> {
    pointer: *mut T,
}

impl<T> Box<T> {
    pub fn new(value: T) -> Box<T> {
        unsafe {
            let pointer = std::libc::malloc(std::intrinsics::size_of::<T>()) as *mut T;
            pointer.write(value);
            Box { pointer }
        }
    }

    /// Give up ownership: the caller now owns the heap value behind the
    /// pointer, and frees it by turning it back with [`Box::from_raw`].
    pub fn into_raw(boxed: Box<T>) -> *mut T {
        let pointer = boxed.pointer;
        std::mem::forget(boxed);
        pointer
    }

    /// Take ownership of a pointer [`Box::into_raw`] gave out.
    pub unsafe fn from_raw(pointer: *mut T) -> Box<T> {
        Box { pointer }
    }

    /// The contents, for the rest of the program: they are never freed.
    pub fn leak(boxed: Box<T>) -> &'static mut T {
        unsafe { &mut *Box::into_raw(boxed) }
    }
}

/// A heap copy of `bytes` bytes starting at `source`.
fn copy_to_heap(source: *const u8, bytes: usize) -> *mut u8 {
    unsafe {
        let data = std::libc::malloc(bytes);
        std::libc::memcpy(data, source, bytes);
        data
    }
}

impl From<&str> for Box<str> {
    fn from(text: &str) -> Box<str> {
        let data = copy_to_heap(text.as_ptr(), text.len());
        let copy = std::intrinsics::str_from_raw_parts(data as *const u8, text.len());
        Box { pointer: copy as *const str as *mut str }
    }
}

impl From<String> for Box<str> {
    fn from(text: String) -> Box<str> {
        Box::from(text.as_str())
    }
}

impl<T: Copy> From<&[T]> for Box<[T]> {
    fn from(items: &[T]) -> Box<[T]> {
        let bytes = items.len() * std::intrinsics::size_of::<T>();
        let data = copy_to_heap(items.as_ptr() as *const u8, bytes);
        let copy = std::intrinsics::slice_from_raw_parts_mut(data as *mut T, items.len());
        Box { pointer: copy as *mut [T] }
    }
}

impl<T> From<Vec<T>> for Box<[T]> {
    fn from(items: Vec<T>) -> Box<[T]> {
        let len = items.len();
        let bytes = len * std::intrinsics::size_of::<T>();
        let data = copy_to_heap(items.as_ptr() as *const u8, bytes);
        // The elements now live in the copy; the vector frees only its buffer.
        let mut items = items;
        unsafe { items.set_len(0) };
        let copy = std::intrinsics::slice_from_raw_parts_mut(data as *mut T, len);
        Box { pointer: copy as *mut [T] }
    }
}

impl<T> From<T> for Box<T> {
    fn from(value: T) -> Box<T> {
        Box::new(value)
    }
}

impl<T: ?Sized, U: ?Sized> std::ops::CoerceUnsized<Box<U>> for Box<T> {}

impl Box<dyn std::any::Any> {
    /// The box as a box of `T`, if that is the type it holds; else the box
    /// itself back.
    pub fn downcast<T: std::any::Any>(self) -> Result<Box<T>, Box<dyn std::any::Any>> {
        if !(*self).is::<T>() {
            return Err(self);
        }
        let pointer = self.pointer;
        std::mem::forget(self);
        Ok(Box { pointer: pointer as *mut T })
    }
}

impl<T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            std::intrinsics::drop_in_place(self.pointer);
            std::libc::free(self.pointer as *mut u8);
        }
    }
}

impl<T: ?Sized> std::ops::Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.pointer }
    }
}

impl<T: ?Sized> std::ops::DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.pointer }
    }
}

impl<T: Clone> Clone for Box<T> {
    fn clone(&self) -> Box<T> {
        Box::new((**self).clone())
    }
}

impl<T: std::fmt::Display + ?Sized> std::fmt::Display for Box<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&**self, f)
    }
}

impl<T: std::fmt::Debug + ?Sized> std::fmt::Debug for Box<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: PartialEq + ?Sized> PartialEq for Box<T> {
    fn eq(&self, other: &Box<T>) -> bool {
        **self == **other
    }
}

impl<T: Eq + ?Sized> Eq for Box<T> {}

impl<T: PartialOrd + ?Sized> PartialOrd for Box<T> {
    fn partial_cmp(&self, other: &Box<T>) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: Ord + ?Sized> Ord for Box<T> {
    fn cmp(&self, other: &Box<T>) -> std::cmp::Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: std::hash::Hash + ?Sized> std::hash::Hash for Box<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl<T: ?Sized> AsMut<T> for Box<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

/// A boxed iterator is an iterator, trait objects included.
impl<I: Iterator + ?Sized> Iterator for Box<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        (**self).next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (**self).size_hint()
    }
}

impl<I: std::iter::DoubleEndedIterator + ?Sized> std::iter::DoubleEndedIterator for Box<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        (**self).next_back()
    }
}

impl<I: std::iter::ExactSizeIterator + ?Sized> std::iter::ExactSizeIterator for Box<I> {
    fn len(&self) -> usize {
        (**self).len()
    }
}
