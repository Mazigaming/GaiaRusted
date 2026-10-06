//! `BinaryHeap<T>`: a priority queue that hands out its greatest element.

use std::fmt;
use std::iter::{ExactSizeIterator, Extend, FromIterator};

/// A max-heap in a vector: every element is at least as great as its
/// children, which sit at positions `2i + 1` and `2i + 2`.
///
/// The sifting follows the standard library's choices step for step, so
/// the layout `{:?}` shows matches what a program built with rustc prints.
pub struct BinaryHeap<T> {
    data: Vec<T>,
}

impl<T> BinaryHeap<T> {
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// The greatest element.
    pub fn peek(&self) -> Option<&T> {
        self.data.first()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// The elements in heap order.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }

    pub fn as_slice(&self) -> &[T] {
        self.data.as_slice()
    }

    /// The underlying vector, in heap order.
    pub fn into_vec(self) -> Vec<T> {
        self.data
    }

    /// Remove every element, in heap order.
    pub fn drain(&mut self) -> std::vec::IntoIter<T> {
        self.data.drain(..)
    }
}

impl<T: Ord> BinaryHeap<T> {
    pub fn new() -> BinaryHeap<T> {
        BinaryHeap { data: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> BinaryHeap<T> {
        BinaryHeap { data: Vec::with_capacity(capacity) }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    pub fn push(&mut self, value: T) {
        self.data.push(value);
        let last = self.data.len() - 1;
        self.sift_up(0, last);
    }

    /// Remove the greatest element.
    pub fn pop(&mut self) -> Option<T> {
        let mut item = self.data.pop()?;
        if !self.data.is_empty() {
            std::mem::swap(&mut item, &mut self.data[0]);
            self.sift_down_to_bottom(0);
        }
        Some(item)
    }

    /// The elements in ascending order.
    pub fn into_sorted_vec(mut self) -> Vec<T> {
        let mut end = self.data.len();
        while end > 1 {
            end -= 1;
            self.data.swap(0, end);
            self.sift_down_range(0, end);
        }
        self.data
    }

    /// Move every element of `other` into this heap.
    pub fn append(&mut self, other: &mut BinaryHeap<T>) {
        if self.data.len() < other.data.len() {
            std::mem::swap(self, other);
        }
        let start = self.data.len();
        self.data.append(&mut other.data);
        self.rebuild_tail(start);
    }

    /// Keep only the elements for which `keep` returns `true`.
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut keep: F) {
        let mut first_removed = self.data.len();
        let mut index = 0;
        self.data.retain(|value| {
            let kept = keep(value);
            if !kept && index < first_removed {
                first_removed = index;
            }
            index += 1;
            kept
        });
        self.rebuild_tail(first_removed);
    }

    /// Move the element at `position` toward the root while it is greater
    /// than its parent, stopping at `start`. Returns where it ends up.
    fn sift_up(&mut self, start: usize, position: usize) -> usize {
        let mut position = position;
        while position > start {
            let parent = (position - 1) / 2;
            if self.data[position] <= self.data[parent] {
                break;
            }
            self.data.swap(position, parent);
            position = parent;
        }
        position
    }

    /// Move the element at `position` down while a child within the first
    /// `end` elements is greater.
    fn sift_down_range(&mut self, position: usize, end: usize) {
        let mut position = position;
        let mut child = 2 * position + 1;
        while end >= 2 && child <= end - 2 {
            if self.data[child] <= self.data[child + 1] {
                child += 1;
            }
            if self.data[position] >= self.data[child] {
                return;
            }
            self.data.swap(position, child);
            position = child;
            child = 2 * position + 1;
        }
        if child + 1 == end && self.data[position] < self.data[child] {
            self.data.swap(position, child);
        }
    }

    /// Move the element at `position` all the way down along the greater
    /// children, then back up to its place. Removing the root this way
    /// takes fewer comparisons, as the moved element is usually small.
    fn sift_down_to_bottom(&mut self, position: usize) {
        let start = position;
        let end = self.data.len();
        let mut position = position;
        let mut child = 2 * position + 1;
        while end >= 2 && child <= end - 2 {
            if self.data[child] <= self.data[child + 1] {
                child += 1;
            }
            self.data.swap(position, child);
            position = child;
            child = 2 * position + 1;
        }
        if child + 1 == end {
            self.data.swap(position, child);
            position = child;
        }
        self.sift_up(start, position);
    }

    /// Restore the heap property over the whole vector.
    fn rebuild(&mut self) {
        let mut position = self.data.len() / 2;
        while position > 0 {
            position -= 1;
            let end = self.data.len();
            self.sift_down_range(position, end);
        }
    }

    /// Restore the heap property after elements were added from `start` on,
    /// by sifting each up or by rebuilding, whichever is cheaper.
    fn rebuild_tail(&mut self, start: usize) {
        let len = self.data.len();
        if start == len {
            return;
        }
        let tail_len = len - start;
        let rebuild = if start < tail_len {
            true
        } else if len <= 2048 {
            2 * len < tail_len * log2(start)
        } else {
            2 * len < tail_len * 11
        };
        if rebuild {
            self.rebuild();
        } else {
            let mut position = start;
            while position < len {
                self.sift_up(0, position);
                position += 1;
            }
        }
    }
}

/// The floor of the base-2 logarithm of `value`, which is not zero.
fn log2(value: usize) -> usize {
    (usize::BITS - value.leading_zeros() - 1) as usize
}

impl<T: Ord> Default for BinaryHeap<T> {
    fn default() -> BinaryHeap<T> {
        BinaryHeap::new()
    }
}

impl<T: Clone> Clone for BinaryHeap<T> {
    fn clone(&self) -> BinaryHeap<T> {
        BinaryHeap { data: self.data.clone() }
    }
}

impl<T: fmt::Debug> fmt::Debug for BinaryHeap<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.data.iter()).finish()
    }
}

impl<T: Ord> From<Vec<T>> for BinaryHeap<T> {
    fn from(data: Vec<T>) -> BinaryHeap<T> {
        let mut heap = BinaryHeap { data };
        heap.rebuild();
        heap
    }
}

impl<T> From<BinaryHeap<T>> for Vec<T> {
    fn from(heap: BinaryHeap<T>) -> Vec<T> {
        heap.data
    }
}

impl<T: Ord> FromIterator<T> for BinaryHeap<T> {
    fn from_iter<I: IntoIterator<Item = T>>(values: I) -> BinaryHeap<T> {
        BinaryHeap::from(values.into_iter().collect::<Vec<T>>())
    }
}

impl<T: Ord> Extend<T> for BinaryHeap<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, values: I) {
        let start = self.data.len();
        self.data.extend(values);
        self.rebuild_tail(start);
    }

    fn extend_one(&mut self, value: T) {
        self.push(value);
    }
}

impl<T> IntoIterator for BinaryHeap<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    /// The elements in heap order, not sorted.
    fn into_iter(self) -> std::vec::IntoIter<T> {
        self.data.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a BinaryHeap<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> std::slice::Iter<'a, T> {
        self.data.iter()
    }
}

impl<T: Ord, const N: usize> From<[T; N]> for BinaryHeap<T> {
    fn from(values: [T; N]) -> BinaryHeap<T> {
        BinaryHeap::from_iter(values)
    }
}
