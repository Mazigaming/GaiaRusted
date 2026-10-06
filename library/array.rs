//! Fixed-size arrays, `[T; N]`: the traits they implement for every length,
//! and taking one apart element by element.

use std::convert::TryFrom;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::{DoubleEndedIterator, ExactSizeIterator};
use std::mem::{ManuallyDrop, MaybeUninit};

/// An array whose element at each index is `make(index)`, made in order.
pub fn from_fn<T, const N: usize, F: FnMut(usize) -> T>(mut make: F) -> [T; N] {
    let mut array: MaybeUninit<[T; N]> = MaybeUninit::uninit();
    let first = array.as_mut_ptr() as *mut T;
    let mut index = 0;
    while index < N {
        first.add(index).write(make(index));
        index += 1;
    }
    unsafe { array.assume_init() }
}

impl<T, const N: usize> [T; N] {
    /// The array of `f` applied to each element, in order.
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> [U; N] {
        let mut elements = self.into_iter();
        from_fn(|_| match elements.next() {
            Some(element) => f(element),
            None => std::intrinsics::unreachable(),
        })
    }

    pub fn as_slice(&self) -> &[T] {
        self
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }

    /// An array of references to the elements.
    pub fn each_ref(&self) -> [&T; N] {
        from_fn(|index| &self[index])
    }
}

/// Iterator that takes the elements of an array by value.
pub struct IntoIter<T, const N: usize> {
    /// The elements not handed out yet are those in `next..end`; the
    /// others have been moved out, so the array is not dropped as a whole.
    data: ManuallyDrop<[T; N]>,
    next: usize,
    end: usize,
}

impl<T, const N: usize> IntoIter<T, N> {
    fn element(&self, index: usize) -> *const T {
        (&*self.data as *const [T; N] as *const T).add(index)
    }

    /// The elements not handed out yet.
    pub fn as_slice(&self) -> &[T] {
        std::intrinsics::slice_from_raw_parts(self.element(self.next), self.end - self.next)
    }
}

impl<T, const N: usize> Iterator for IntoIter<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.next == self.end {
            return None;
        }
        let element = self.element(self.next).read();
        self.next += 1;
        Some(element)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.end - self.next;
        (len, Some(len))
    }
}

impl<T, const N: usize> DoubleEndedIterator for IntoIter<T, N> {
    fn next_back(&mut self) -> Option<T> {
        if self.next == self.end {
            return None;
        }
        self.end -= 1;
        Some(self.element(self.end).read())
    }
}

impl<T, const N: usize> ExactSizeIterator for IntoIter<T, N> {
    fn len(&self) -> usize {
        self.end - self.next
    }
}

impl<T, const N: usize> Drop for IntoIter<T, N> {
    fn drop(&mut self) {
        while self.next < self.end {
            unsafe { std::ptr::drop_in_place(self.element(self.next) as *mut T) }
            self.next += 1;
        }
    }
}

impl<T: Clone, const N: usize> Clone for IntoIter<T, N> {
    fn clone(&self) -> IntoIter<T, N> {
        let remaining = self.as_slice();
        let data = from_fn(|index| match remaining.get(index) {
            Some(element) => element.clone(),
            None => unsafe { std::intrinsics::uninit() },
        });
        IntoIter { data: ManuallyDrop::new(data), next: 0, end: remaining.len() }
    }
}

impl<T, const N: usize> IntoIterator for [T; N] {
    type Item = T;
    type IntoIter = IntoIter<T, N>;

    fn into_iter(self) -> IntoIter<T, N> {
        IntoIter { data: ManuallyDrop::new(self), next: 0, end: N }
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a [T; N] {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> std::slice::Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut [T; N] {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> std::slice::IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for [T; N] {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_slice(), f)
    }
}

impl<T: Clone, const N: usize> Clone for [T; N] {
    fn clone(&self) -> [T; N] {
        from_fn(|index| self[index].clone())
    }
}

impl<T: Default, const N: usize> Default for [T; N] {
    fn default() -> [T; N] {
        from_fn(|_| T::default())
    }
}

impl<T: PartialEq, const N: usize> PartialEq for [T; N] {
    fn eq(&self, other: &[T; N]) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: PartialEq, const N: usize> PartialEq<[T]> for [T; N] {
    fn eq(&self, other: &[T]) -> bool {
        self.as_slice() == other
    }
}

impl<T: PartialEq, const N: usize> PartialEq<[T; N]> for [T] {
    fn eq(&self, other: &[T; N]) -> bool {
        self == other.as_slice()
    }
}

impl<T: Eq, const N: usize> Eq for [T; N] {}

impl<T: PartialOrd, const N: usize> PartialOrd for [T; N] {
    fn partial_cmp(&self, other: &[T; N]) -> Option<std::cmp::Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}

impl<T: Ord, const N: usize> Ord for [T; N] {
    fn cmp(&self, other: &[T; N]) -> std::cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl<T: Hash, const N: usize> Hash for [T; N] {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state)
    }
}

impl<T, const N: usize> AsRef<[T]> for [T; N] {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T, const N: usize> AsMut<[T]> for [T; N] {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T, const N: usize> std::borrow::Borrow<[T]> for [T; N] {
    fn borrow(&self) -> &[T] {
        self
    }
}

/// The error of converting a slice to an array of a different length.
#[derive(Debug, Clone, Copy)]
pub struct TryFromSliceError(());

impl fmt::Display for TryFromSliceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("could not convert slice to array")
    }
}

impl<T: Copy, const N: usize> TryFrom<&[T]> for [T; N] {
    type Error = TryFromSliceError;

    fn try_from(slice: &[T]) -> Result<[T; N], TryFromSliceError> {
        if slice.len() != N {
            return Err(TryFromSliceError(()));
        }
        Ok(from_fn(|index| slice[index]))
    }
}

impl<'a, T, const N: usize> TryFrom<&'a [T]> for &'a [T; N] {
    type Error = TryFromSliceError;

    fn try_from(slice: &'a [T]) -> Result<&'a [T; N], TryFromSliceError> {
        if slice.len() != N {
            return Err(TryFromSliceError(()));
        }
        Ok(unsafe { &*(slice.as_ptr() as *const [T; N]) })
    }
}

impl<T, const N: usize> TryFrom<Vec<T>> for [T; N] {
    type Error = Vec<T>;

    fn try_from(vec: Vec<T>) -> Result<[T; N], Vec<T>> {
        if vec.len() != N {
            return Err(vec);
        }
        let mut elements = vec.into_iter();
        Ok(from_fn(|_| match elements.next() {
            Some(element) => element,
            None => std::intrinsics::unreachable(),
        }))
    }
}
