//! `Vec<T>`: a growable array on the heap.

pub struct Vec<T> {
    pointer: *mut T,
    len: usize,
    capacity: usize,
}

impl<T> Vec<T> {
    pub fn new() -> Vec<T> {
        Vec { pointer: std::ptr::null_mut(), len: 0, capacity: 0 }
    }

    pub fn with_capacity(capacity: usize) -> Vec<T> {
        let mut vec = Vec::new();
        vec.reserve(capacity);
        vec
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn as_slice(&self) -> &[T] {
        std::intrinsics::slice_from_raw_parts(self.pointer, self.len)
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        std::intrinsics::slice_from_raw_parts_mut(self.pointer, self.len)
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.pointer
    }

    /// Declare the first `len` elements initialised, without touching them.
    pub unsafe fn set_len(&mut self, len: usize) {
        self.len = len;
    }

    pub fn into_boxed_slice(self) -> Box<[T]> {
        Box::from(self)
    }

    /// Make room for at least `additional` more elements. Capacity doubles,
    /// so a sequence of pushes takes linear time overall.
    pub fn reserve(&mut self, additional: usize) {
        let needed = self.len + additional;
        if needed <= self.capacity {
            return;
        }
        let mut capacity = if self.capacity == 0 { 4 } else { self.capacity * 2 };
        while capacity < needed {
            capacity = capacity * 2;
        }
        let bytes = capacity * std::intrinsics::size_of::<T>();
        self.pointer = unsafe { std::libc::realloc(self.pointer as *mut u8, bytes) as *mut T };
        self.capacity = capacity;
    }

    pub fn push(&mut self, value: T) {
        if self.len == self.capacity {
            self.reserve(1);
        }
        self.pointer.add(self.len).write(value);
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(self.pointer.add(self.len).read())
    }

    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// Keep the first `len` elements and drop the rest, front to back.
    pub fn truncate(&mut self, len: usize) {
        let old_len = self.len;
        if len >= old_len {
            return;
        }
        // Shorten first: the elements are no longer the vector's while
        // their destructors run.
        self.len = len;
        if !std::mem::needs_drop::<T>() {
            return;
        }
        let mut index = len;
        while index < old_len {
            self.pointer.add(index).drop_in_place();
            index += 1;
        }
    }

    /// Insert `value` at `index`, shifting everything after it up by one.
    pub fn insert(&mut self, index: usize, value: T) {
        if index > self.len {
            panic!("insertion index (is {}) should be <= len (is {})", index, self.len);
        }
        self.reserve(1);
        let size = std::intrinsics::size_of::<T>();
        unsafe {
            std::libc::memmove(self.pointer.add(index + 1) as *mut u8, self.pointer.add(index) as *const u8, (self.len - index) * size);
        }
        self.pointer.add(index).write(value);
        self.len += 1;
    }

    /// Remove and return the element at `index`, shifting the rest down.
    pub fn remove(&mut self, index: usize) -> T {
        if index >= self.len {
            panic!("removal index (is {}) should be < len (is {})", index, self.len);
        }
        let removed = self.pointer.add(index).read();
        let size = std::intrinsics::size_of::<T>();
        unsafe {
            std::libc::memmove(self.pointer.add(index) as *mut u8, self.pointer.add(index + 1) as *const u8, (self.len - index - 1) * size);
        }
        self.len -= 1;
        removed
    }

    /// Remove the element at `index` by moving the last one into its place.
    pub fn swap_remove(&mut self, index: usize) -> T {
        if index >= self.len {
            panic!("swap_remove index (is {}) should be < len (is {})", index, self.len);
        }
        let removed = self.pointer.add(index).read();
        self.len -= 1;
        if index != self.len {
            self.pointer.add(index).write(self.pointer.add(self.len).read());
        }
        removed
    }

    /// Append the elements of an iterator.
    pub fn extend<I: IntoIterator>(&mut self, items: I) {
        for item in items {
            self.push(item);
        }
    }

    pub fn into_iter(self) -> IntoIter<T> {
        // The iterator takes over the buffer; the vector must not free it.
        let iter = IntoIter { pointer: self.pointer, len: self.len, next: 0 };
        std::mem::forget(self);
        iter
    }

    /// Keep only the elements for which `keep` is true, in order.
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, keep: F) {
        let mut keep = keep;
        self.retain_mut(|item| keep(item));
    }

    pub fn retain_mut<F: FnMut(&mut T) -> bool>(&mut self, keep: F) {
        let mut keep = keep;
        let len = self.len;
        // The vector holds only the kept prefix while the rest is sorted
        // out, so a panic in `keep` cannot drop anything twice.
        self.len = 0;
        let mut kept = 0;
        let mut index = 0;
        while index < len {
            let item = self.pointer.add(index);
            if keep(unsafe { &mut *item }) {
                if kept != index {
                    self.pointer.add(kept).write(item.read());
                }
                kept += 1;
            } else {
                item.drop_in_place();
            }
            index += 1;
        }
        self.len = kept;
    }

    /// Remove consecutive elements that `same` says are duplicates of the
    /// one kept before them.
    pub fn dedup_by<F: FnMut(&mut T, &mut T) -> bool>(&mut self, same: F) {
        let mut same = same;
        if self.len < 2 {
            return;
        }
        let len = self.len;
        self.len = 0;
        let mut kept = 1;
        let mut index = 1;
        while index < len {
            let item = self.pointer.add(index);
            let previous = self.pointer.add(kept - 1);
            if same(unsafe { &mut *item }, unsafe { &mut *previous }) {
                item.drop_in_place();
            } else {
                if kept != index {
                    self.pointer.add(kept).write(item.read());
                }
                kept += 1;
            }
            index += 1;
        }
        self.len = kept;
    }

    pub fn dedup_by_key<K: PartialEq, F: FnMut(&mut T) -> K>(&mut self, key: F) {
        let mut key = key;
        self.dedup_by(|a, b| key(a) == key(b));
    }

    /// Take the elements in `range` out, as an iterator; the rest close up.
    pub fn drain<R: std::ops::RangeBounds<usize>>(&mut self, range: R) -> IntoIter<T> {
        let (start, end) = std::ops::range_positions(&range, self.len);
        if start > end || end > self.len {
            panic!("range {}..{} is out of bounds of a vector of length {}", start, end, self.len);
        }
        let mut drained = Vec::with_capacity(end - start);
        let size = std::intrinsics::size_of::<T>();
        unsafe {
            std::libc::memcpy(drained.pointer as *mut u8, self.pointer.add(start) as *const u8, (end - start) * size);
            std::libc::memmove(self.pointer.add(start) as *mut u8, self.pointer.add(end) as *const u8, (self.len - end) * size);
        }
        drained.len = end - start;
        self.len -= end - start;
        drained.into_iter()
    }

    /// The elements from `at` on, as a new vector; this one keeps the rest.
    pub fn split_off(&mut self, at: usize) -> Vec<T> {
        if at > self.len {
            panic!("`at` split index (is {}) should be <= len (is {})", at, self.len);
        }
        self.drain(at..).collect()
    }

    /// Move all the elements of `other` to the end of this vector.
    pub fn append(&mut self, other: &mut Vec<T>) {
        let count = other.len;
        self.reserve(count);
        let size = std::intrinsics::size_of::<T>();
        unsafe {
            std::libc::memcpy(self.pointer.add(self.len) as *mut u8, other.pointer as *const u8, count * size);
        }
        self.len += count;
        other.len = 0;
    }

    pub fn resize_with<F: FnMut() -> T>(&mut self, len: usize, f: F) {
        let mut f = f;
        if len <= self.len {
            self.truncate(len);
            return;
        }
        self.reserve(len - self.len);
        while self.len < len {
            self.push(f());
        }
    }

    pub fn first(&self) -> Option<&T> {
        self.as_slice().first()
    }

    pub fn last(&self) -> Option<&T> {
        self.as_slice().last()
    }

    pub fn shrink_to_fit(&mut self) {}

    pub fn leak(self) -> &mut [T] {
        let slice = std::intrinsics::slice_from_raw_parts_mut(self.pointer, self.len);
        std::mem::forget(self);
        slice
    }
}

impl<T: PartialEq> Vec<T> {
    /// Remove consecutive repeated elements.
    pub fn dedup(&mut self) {
        self.dedup_by(|a, b| *a == *b);
    }
}

impl<T> std::iter::Extend<T> for Vec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, items: I) {
        for item in items {
            self.push(item);
        }
    }

    fn extend_one(&mut self, item: T) {
        self.push(item);
    }
}

impl<T: Copy> std::iter::Extend<&T> for Vec<T> {
    fn extend<I: IntoIterator<Item = &T>>(&mut self, items: I) {
        for item in items {
            self.push(*item);
        }
    }

    fn extend_one(&mut self, item: &T) {
        self.push(*item);
    }
}

impl<T: PartialOrd> PartialOrd for Vec<T> {
    fn partial_cmp(&self, other: &Vec<T>) -> Option<std::cmp::Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}

impl<T: Ord> Ord for Vec<T> {
    fn cmp(&self, other: &Vec<T>) -> std::cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl<T: Eq> Eq for Vec<T> {}

impl<T: std::hash::Hash> std::hash::Hash for Vec<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(self.as_slice(), state);
    }
}

impl<T: Clone> From<&[T]> for Vec<T> {
    fn from(items: &[T]) -> Vec<T> {
        items.to_vec()
    }
}

impl<T: Clone> From<&Vec<T>> for Vec<T> {
    fn from(items: &Vec<T>) -> Vec<T> {
        items.clone()
    }
}

impl From<&str> for Vec<u8> {
    fn from(text: &str) -> Vec<u8> {
        text.as_bytes().to_vec()
    }
}

impl From<String> for Vec<u8> {
    fn from(text: String) -> Vec<u8> {
        text.into_bytes()
    }
}

impl<T: Clone> Vec<T> {
    pub fn resize(&mut self, len: usize, value: T) {
        self.resize_with(len, || value.clone());
    }

    pub fn extend_from_slice(&mut self, other: &[T]) {
        self.reserve(other.len());
        let mut index = 0;
        while index < other.len() {
            self.push(other[index].clone());
            index += 1;
        }
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        self.truncate(0);
        unsafe { std::libc::free(self.pointer as *mut u8) }
    }
}

impl<T> std::ops::Deref for Vec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> std::ops::DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: Clone> Clone for Vec<T> {
    fn clone(&self) -> Vec<T> {
        let mut copy = Vec::with_capacity(self.len);
        let mut index = 0;
        while index < self.len {
            copy.push(self[index].clone());
            index += 1;
        }
        copy
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Vec<T> {
        Vec::new()
    }
}

impl<T: PartialEq> PartialEq for Vec<T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: PartialEq> PartialEq<[T]> for Vec<T> {
    fn eq(&self, other: &[T]) -> bool {
        self.as_slice() == other
    }
}

impl<T: PartialEq> PartialEq<&[T]> for Vec<T> {
    fn eq(&self, other: &&[T]) -> bool {
        self.as_slice() == *other
    }
}

impl<T: PartialEq, const N: usize> PartialEq<[T; N]> for Vec<T> {
    fn eq(&self, other: &[T; N]) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: PartialEq, const N: usize> PartialEq<&[T; N]> for Vec<T> {
    fn eq(&self, other: &&[T; N]) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: PartialEq> PartialEq<Vec<T>> for [T] {
    fn eq(&self, other: &Vec<T>) -> bool {
        self == other.as_slice()
    }
}

impl<T: PartialEq> PartialEq<Vec<T>> for &[T] {
    fn eq(&self, other: &Vec<T>) -> bool {
        *self == other.as_slice()
    }
}

impl<T: PartialEq, const N: usize> PartialEq<Vec<T>> for [T; N] {
    fn eq(&self, other: &Vec<T>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T, const N: usize> From<[T; N]> for Vec<T> {
    fn from(array: [T; N]) -> Vec<T> {
        let mut vec = Vec::with_capacity(N);
        vec.extend(array);
        vec
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Vec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_slice(), f)
    }
}

/// `vec![value; count]`
pub fn from_elem<T: Clone>(value: T, count: usize) -> Vec<T> {
    let mut vec = Vec::with_capacity(count);
    let mut made = 0;
    while made < count {
        vec.push(value.clone());
        made += 1;
    }
    vec
}

/// Iterator that takes the elements out of a vector.
pub struct IntoIter<T> {
    pointer: *mut T,
    len: usize,
    next: usize,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.next >= self.len {
            return None;
        }
        let item = self.pointer.add(self.next).read();
        self.next += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len - self.next, Some(self.len - self.next))
    }
}

impl<T> std::iter::DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        if self.next >= self.len {
            return None;
        }
        self.len -= 1;
        Some(self.pointer.add(self.len).read())
    }
}

impl<T> std::iter::ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.len - self.next
    }
}

impl<T> IntoIter<T> {
    pub fn as_slice(&self) -> &[T] {
        std::intrinsics::slice_from_raw_parts(self.pointer.add(self.next), self.len - self.next)
    }
}

impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        // Elements not handed out yet are still ours to destroy.
        while std::mem::needs_drop::<T>() && self.next < self.len {
            self.pointer.add(self.next).drop_in_place();
            self.next += 1;
        }
        unsafe { std::libc::free(self.pointer as *mut u8) }
    }
}

impl<T> IntoIterator for Vec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> IntoIter<T> {
        Vec::into_iter(self)
    }
}

impl<T> IntoIterator for &Vec<T> {
    type Item = &T;
    type IntoIter = std::slice::Iter<T>;

    fn into_iter(self) -> std::slice::Iter<T> {
        self.as_slice().iter()
    }
}

impl<T> IntoIterator for &mut Vec<T> {
    type Item = &mut T;
    type IntoIter = std::slice::IterMut<T>;

    fn into_iter(self) -> std::slice::IterMut<T> {
        self.as_mut_slice().iter_mut()
    }
}

impl<T> std::iter::FromIterator<T> for Vec<T> {
    fn from_iter<I>(items: I) -> Vec<T> {
        let mut vec = Vec::new();
        let mut items = items;
        while let Some(item) = items.next() {
            vec.push(item);
        }
        vec
    }
}
