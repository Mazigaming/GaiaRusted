//! `VecDeque<T>`: a double-ended queue in a growable ring buffer.

use std::fmt;
use std::iter::{DoubleEndedIterator, ExactSizeIterator, Extend, FromIterator};

/// A queue that grows and shrinks at both ends in constant time.
///
/// The elements occupy `len` consecutive slots of the buffer starting at
/// `head`, wrapping around its end, so they form at most two runs: one from
/// `head` to the end of the buffer and one from its start.
pub struct VecDeque<T> {
    buffer: *mut T,
    capacity: usize,
    head: usize,
    len: usize,
}

impl<T> VecDeque<T> {
    pub fn new() -> VecDeque<T> {
        VecDeque { buffer: std::ptr::null_mut(), capacity: 0, head: 0, len: 0 }
    }

    pub fn with_capacity(capacity: usize) -> VecDeque<T> {
        let mut deque = VecDeque::new();
        deque.reserve(capacity);
        deque
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

    /// The buffer position of the element `index` places from the front.
    fn position(&self, index: usize) -> usize {
        let position = self.head + index;
        if position >= self.capacity {
            position - self.capacity
        } else {
            position
        }
    }

    fn slot(&self, index: usize) -> *mut T {
        self.buffer.add(self.position(index))
    }

    /// How many elements lie in the first run, before the buffer wraps.
    fn front_len(&self) -> usize {
        let room = self.capacity - self.head;
        if self.len < room {
            self.len
        } else {
            room
        }
    }

    /// Move the elements to the start of a new buffer of `capacity` slots.
    fn relocate(&mut self, capacity: usize) {
        let fresh = unsafe { std::libc::malloc(capacity * std::intrinsics::size_of::<T>()) as *mut T };
        let front_len = self.front_len();
        unsafe {
            std::ptr::copy_nonoverlapping(self.buffer.add(self.head) as *const T, fresh, front_len);
            std::ptr::copy_nonoverlapping(self.buffer as *const T, fresh.add(front_len), self.len - front_len);
            std::libc::free(self.buffer as *mut u8);
        }
        self.buffer = fresh;
        self.capacity = capacity;
        self.head = 0;
    }

    /// Make room for at least `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        let needed = self.len + additional;
        if needed <= self.capacity {
            return;
        }
        let mut capacity = if self.capacity == 0 { 4 } else { self.capacity * 2 };
        while capacity < needed {
            capacity = capacity * 2;
        }
        self.relocate(capacity);
    }

    pub fn shrink_to_fit(&mut self) {
        if self.capacity > self.len && self.len > 0 {
            self.relocate(self.len);
        }
    }

    pub fn push_back(&mut self, value: T) {
        if self.len == self.capacity {
            self.reserve(1);
        }
        self.slot(self.len).write(value);
        self.len += 1;
    }

    pub fn push_front(&mut self, value: T) {
        if self.len == self.capacity {
            self.reserve(1);
        }
        self.head = if self.head == 0 { self.capacity - 1 } else { self.head - 1 };
        self.buffer.add(self.head).write(value);
        self.len += 1;
    }

    pub fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let value = self.buffer.add(self.head).read();
        self.head = self.position(1);
        self.len -= 1;
        Some(value)
    }

    pub fn pop_back(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(self.slot(self.len).read())
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            Some(unsafe { &*self.slot(index) })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.len {
            Some(unsafe { &mut *self.slot(index) })
        } else {
            None
        }
    }

    pub fn front(&self) -> Option<&T> {
        self.get(0)
    }

    pub fn back(&self) -> Option<&T> {
        if self.len == 0 {
            None
        } else {
            self.get(self.len - 1)
        }
    }

    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.get_mut(0)
    }

    pub fn back_mut(&mut self) -> Option<&mut T> {
        if self.len == 0 {
            None
        } else {
            self.get_mut(self.len - 1)
        }
    }

    /// The elements as two slices: the front run, then the wrapped one.
    pub fn as_slices(&self) -> (&[T], &[T]) {
        let front_len = self.front_len();
        (
            std::intrinsics::slice_from_raw_parts(self.buffer.add(self.head) as *const T, front_len),
            std::intrinsics::slice_from_raw_parts(self.buffer as *const T, self.len - front_len),
        )
    }

    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        let front_len = self.front_len();
        (
            std::intrinsics::slice_from_raw_parts_mut(self.buffer.add(self.head), front_len),
            std::intrinsics::slice_from_raw_parts_mut(self.buffer, self.len - front_len),
        )
    }

    /// Lay the elements out in one run and return it.
    pub fn make_contiguous(&mut self) -> &mut [T] {
        if self.front_len() < self.len {
            self.relocate(self.capacity);
        }
        std::intrinsics::slice_from_raw_parts_mut(self.buffer.add(self.head), self.len)
    }

    pub fn iter(&self) -> Iter<'_, T> {
        let (front, back) = self.as_slices();
        Iter { front: front.iter(), back: back.iter() }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        let (front, back) = self.as_mut_slices();
        IterMut { front: front.iter_mut(), back: back.iter_mut() }
    }

    /// Exchange the elements at positions `a` and `b`.
    pub fn swap(&mut self, a: usize, b: usize) {
        if a >= self.len || b >= self.len {
            panic!("swap index out of bounds of a deque of length {}", self.len);
        }
        unsafe { std::ptr::swap(self.slot(a), self.slot(b)) }
    }

    /// Keep the first `len` elements and drop the rest, front to back.
    pub fn truncate(&mut self, len: usize) {
        let old_len = self.len;
        if len >= old_len {
            return;
        }
        self.len = len;
        let mut index = len;
        while index < old_len {
            self.slot(index).drop_in_place();
            index += 1;
        }
    }

    pub fn clear(&mut self) {
        self.truncate(0);
        self.head = 0;
    }

    /// Insert `value` so that it ends up at position `index`.
    pub fn insert(&mut self, index: usize, value: T) {
        if index > self.len {
            panic!("index out of bounds");
        }
        self.push_back(value);
        let mut position = self.len - 1;
        while position > index {
            self.swap(position - 1, position);
            position -= 1;
        }
    }

    /// Remove and return the element at `index`, keeping the others in order.
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.len {
            return None;
        }
        let mut position = index;
        while position + 1 < self.len {
            self.swap(position, position + 1);
            position += 1;
        }
        self.pop_back()
    }

    /// Move the first `count` elements to the back.
    pub fn rotate_left(&mut self, count: usize) {
        if count > self.len {
            panic!("rotation of {} is longer than the deque ({})", count, self.len);
        }
        let mut moved = 0;
        while moved < count {
            if let Some(value) = self.pop_front() {
                self.push_back(value);
            }
            moved += 1;
        }
    }

    /// Move the last `count` elements to the front.
    pub fn rotate_right(&mut self, count: usize) {
        if count > self.len {
            panic!("rotation of {} is longer than the deque ({})", count, self.len);
        }
        let mut moved = 0;
        while moved < count {
            if let Some(value) = self.pop_back() {
                self.push_front(value);
            }
            moved += 1;
        }
    }

    /// Keep only the elements for which `keep` returns `true`, in order.
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut keep: F) {
        self.retain_mut(|value| keep(value));
    }

    pub fn retain_mut<F: FnMut(&mut T) -> bool>(&mut self, mut keep: F) {
        let mut kept = 0;
        let mut index = 0;
        while index < self.len {
            if keep(unsafe { &mut *self.slot(index) }) {
                if kept != index {
                    self.swap(kept, index);
                }
                kept += 1;
            }
            index += 1;
        }
        self.truncate(kept);
    }

    /// Move every element of `other` to the back of this deque.
    pub fn append(&mut self, other: &mut VecDeque<T>) {
        self.reserve(other.len);
        while let Some(value) = other.pop_front() {
            self.push_back(value);
        }
    }

    /// The elements from `at` on, as a new deque; this one keeps the rest.
    pub fn split_off(&mut self, at: usize) -> VecDeque<T> {
        if at > self.len {
            panic!("`at` out of bounds");
        }
        let mut tail = VecDeque::with_capacity(self.len - at);
        while self.len > at {
            if let Some(value) = self.pop_back() {
                tail.push_front(value);
            }
        }
        tail
    }

    /// Remove the elements in `range`, handing them out front to back.
    pub fn drain<R: std::ops::RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, T> {
        let (start, end) = std::ops::range_positions(&range, self.len);
        if start > end || end > self.len {
            panic!("range {}..{} is out of bounds of a deque of length {}", start, end, self.len);
        }
        let mut tail = self.split_off(end);
        let drained = self.split_off(start);
        self.append(&mut tail);
        Drain { iter: drained.into_iter() }
    }

    pub fn resize_with<F: FnMut() -> T>(&mut self, len: usize, mut make: F) {
        self.truncate(len);
        while self.len < len {
            self.push_back(make());
        }
    }
}

impl<T: Clone> VecDeque<T> {
    pub fn resize(&mut self, len: usize, value: T) {
        self.resize_with(len, || value.clone());
    }
}

impl<T: PartialEq> VecDeque<T> {
    pub fn contains(&self, value: &T) -> bool {
        self.iter().any(|element| element == value)
    }
}

impl<T> Drop for VecDeque<T> {
    fn drop(&mut self) {
        self.truncate(0);
        unsafe { std::libc::free(self.buffer as *mut u8) }
    }
}

impl<T> std::ops::Index<usize> for VecDeque<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        match self.get(index) {
            Some(value) => value,
            None => panic!("Out of bounds access"),
        }
    }
}

impl<T> std::ops::IndexMut<usize> for VecDeque<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        match self.get_mut(index) {
            Some(value) => value,
            None => panic!("Out of bounds access"),
        }
    }
}

impl<T> Default for VecDeque<T> {
    fn default() -> VecDeque<T> {
        VecDeque::new()
    }
}

impl<T: Clone> Clone for VecDeque<T> {
    fn clone(&self) -> VecDeque<T> {
        self.iter().cloned().collect()
    }
}

impl<T: PartialEq> PartialEq for VecDeque<T> {
    fn eq(&self, other: &VecDeque<T>) -> bool {
        self.len == other.len && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<T: Eq> Eq for VecDeque<T> {}

impl<T: fmt::Debug> fmt::Debug for VecDeque<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> FromIterator<T> for VecDeque<T> {
    fn from_iter<I: IntoIterator<Item = T>>(values: I) -> VecDeque<T> {
        let mut deque = VecDeque::new();
        deque.extend(values);
        deque
    }
}

impl<T> Extend<T> for VecDeque<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, values: I) {
        for value in values {
            self.push_back(value);
        }
    }

    fn extend_one(&mut self, value: T) {
        self.push_back(value);
    }
}

impl<T: Copy> Extend<&T> for VecDeque<T> {
    fn extend<I: IntoIterator<Item = &T>>(&mut self, values: I) {
        for value in values {
            self.push_back(*value);
        }
    }

    fn extend_one(&mut self, value: &T) {
        self.push_back(*value);
    }
}

impl<T> From<Vec<T>> for VecDeque<T> {
    fn from(values: Vec<T>) -> VecDeque<T> {
        let mut deque = VecDeque::with_capacity(values.len());
        deque.extend(values);
        deque
    }
}

impl<T> From<VecDeque<T>> for Vec<T> {
    fn from(deque: VecDeque<T>) -> Vec<T> {
        let mut values = Vec::with_capacity(deque.len());
        values.extend(deque);
        values
    }
}

/// Iterator over the elements of a deque, front to back.
pub struct Iter<'a, T> {
    front: std::slice::Iter<'a, T>,
    back: std::slice::Iter<'a, T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match self.front.next() {
            Some(value) => Some(value),
            None => self.back.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        match self.back.next_back() {
            Some(value) => Some(value),
            None => self.front.next_back(),
        }
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.front.len() + self.back.len()
    }
}

pub struct IterMut<'a, T> {
    front: std::slice::IterMut<'a, T>,
    back: std::slice::IterMut<'a, T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        match self.front.next() {
            Some(value) => Some(value),
            None => self.back.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<&'a mut T> {
        match self.back.next_back() {
            Some(value) => Some(value),
            None => self.front.next_back(),
        }
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.front.len() + self.back.len()
    }
}

/// Iterator that takes the elements of a deque by value.
pub struct IntoIter<T> {
    deque: VecDeque<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.deque.pop_front()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.deque.len, Some(self.deque.len))
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.deque.pop_back()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.deque.len
    }
}

pub struct Drain<'a, T> {
    iter: IntoIter<T>,
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for Drain<'a, T> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}

impl<T> IntoIterator for VecDeque<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> IntoIter<T> {
        IntoIter { deque: self }
    }
}

impl<'a, T> IntoIterator for &'a VecDeque<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut VecDeque<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<T, const N: usize> From<[T; N]> for VecDeque<T> {
    fn from(values: [T; N]) -> VecDeque<T> {
        let mut deque = VecDeque::with_capacity(N);
        deque.extend(values);
        deque
    }
}
