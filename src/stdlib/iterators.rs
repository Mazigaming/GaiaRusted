//! # Iterator Trait Implementation
//!
//! Implements the `Iterator` and `IntoIterator` traits that enable efficient
//! traversal of collections. Provides the foundation for for-loops and adapters.

use std::marker::PhantomData;

/// The Iterator trait - defines how to traverse a collection
pub trait Iterator: Sized {
    /// The type of items yielded by this iterator
    type Item;

    /// Get the next item from the iterator, or None if exhausted
    fn next(&mut self) -> Option<Self::Item>;

    /// Get the remaining count of items (may return estimate)
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    /// Count the number of items in this iterator
    fn count(mut self) -> usize {
        let mut count = 0;
        while let Some(_) = self.next() {
            count += 1;
        }
        count
    }

    /// Get the next item, or return a default
    fn next_or_default(&mut self, default: Self::Item) -> Self::Item {
        self.next().unwrap_or(default)
    }
}

/// The IntoIterator trait - converts a type into an iterator
pub trait IntoIterator: Sized {
    /// The type of items yielded by the iterator
    type Item;

    /// The type of iterator returned
    type IntoIter: Iterator<Item = Self::Item>;

    /// Convert self into an iterator
    fn into_iter(self) -> Self::IntoIter;
}

/// Reference iterator for Vec<T> - yields &T
pub struct VecIter<'a, T> {
    items: &'a [T],
    index: usize,
}

impl<'a, T> VecIter<'a, T> {
    /// Create a new vector iterator
    pub fn new(items: &'a [T]) -> Self {
        VecIter { items, index: 0 }
    }
}

impl<'a, T: 'a> Iterator for VecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.items.len() {
            let item = &self.items[self.index];
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.items.len() - self.index;
        (remaining, Some(remaining))
    }
}

/// Mutable reference iterator for Vec<T> - yields &mut T
pub struct VecIterMut<'a, T> {
    items: &'a mut [T],
    index: usize,
}

impl<'a, T> VecIterMut<'a, T> {
    /// Create a new mutable vector iterator
    pub fn new(items: &'a mut [T]) -> Self {
        VecIterMut { items, index: 0 }
    }
}

impl<'a, T: 'a> Iterator for VecIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.items.len() {
            // SAFETY: We're careful to not yield same reference twice
            let item = unsafe { &mut *(&mut self.items[self.index] as *mut T) };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.items.len() - self.index;
        (remaining, Some(remaining))
    }
}

/// Consuming iterator for Vec<T> - takes ownership and yields T
pub struct VecIntoIter<T> {
    items: Vec<T>,
    index: usize,
}

impl<T> VecIntoIter<T> {
    /// Create a new consuming vector iterator
    pub fn new(items: Vec<T>) -> Self {
        VecIntoIter { items, index: 0 }
    }
}

impl<T> Iterator for VecIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.items.len() {
            // We take ownership of the item
            let mut vec = std::mem::replace(&mut self.items, Vec::new());
            if self.index < vec.len() {
                let item = vec.remove(0);
                self.items = vec;
                self.index += 1;
                Some(item)
            } else {
                self.items = vec;
                None
            }
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.items.len() - self.index;
        (remaining, Some(remaining))
    }
}

/// Range iterator - yields values from start to end
pub struct RangeIter {
    current: i64,
    end: i64,
    inclusive: bool,
}

impl RangeIter {
    /// Create a range iterator (exclusive)
    pub fn new_exclusive(start: i64, end: i64) -> Self {
        RangeIter {
            current: start,
            end,
            inclusive: false,
        }
    }

    /// Create a range iterator (inclusive)
    pub fn new_inclusive(start: i64, end: i64) -> Self {
        RangeIter {
            current: start,
            end,
            inclusive: true,
        }
    }
}

impl Iterator for RangeIter {
    type Item = i64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.inclusive {
            if self.current <= self.end {
                let item = self.current;
                self.current += 1;
                Some(item)
            } else {
                None
            }
        } else {
            if self.current < self.end {
                let item = self.current;
                self.current += 1;
                Some(item)
            } else {
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = if self.inclusive {
            (self.end - self.current + 1).max(0) as usize
        } else {
            (self.end - self.current).max(0) as usize
        };
        (remaining, Some(remaining))
    }
}

/// String iterator - yields characters
pub struct StringIter<'a> {
    chars: std::str::Chars<'a>,
}

impl<'a> StringIter<'a> {
    /// Create a new string iterator
    pub fn new(s: &'a str) -> Self {
        StringIter { chars: s.chars() }
    }
}

impl<'a> Iterator for StringIter<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.chars.next()
    }
}

/// Map adapter - transforms items with a closure
pub struct Map<I, F>
where
    I: Iterator,
    F: FnMut(I::Item) -> I::Item,
{
    iter: I,
    f: F,
}

impl<I, F> Map<I, F>
where
    I: Iterator,
    F: FnMut(I::Item) -> I::Item,
{
    /// Create a new Map adapter
    pub fn new(iter: I, f: F) -> Self {
        Map { iter, f }
    }
}

impl<I, F> Iterator for Map<I, F>
where
    I: Iterator,
    F: FnMut(I::Item) -> I::Item,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|item| (self.f)(item))
    }
}

/// Filter adapter - keeps only items matching a predicate
pub struct Filter<I, F>
where
    I: Iterator,
    F: FnMut(&I::Item) -> bool,
{
    iter: I,
    predicate: F,
}

impl<I, F> Filter<I, F>
where
    I: Iterator,
    F: FnMut(&I::Item) -> bool,
{
    /// Create a new Filter adapter
    pub fn new(iter: I, predicate: F) -> Self {
        Filter { iter, predicate }
    }
}

impl<I, F> Iterator for Filter<I, F>
where
    I: Iterator,
    F: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.iter.next() {
            if (self.predicate)(&item) {
                return Some(item);
            }
        }
        None
    }
}

/// Collect trait - gathers iterator items into a collection
pub trait Collect<T>: Sized {
    /// Collect items from an iterator into this collection type
    fn from_iter<I: Iterator<Item = T>>(iter: I) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_iter_exclusive() {
        let mut iter = RangeIter::new_exclusive(1, 4);
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_range_iter_inclusive() {
        let mut iter = RangeIter::new_inclusive(1, 3);
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_range_iter_size_hint() {
        let iter = RangeIter::new_exclusive(1, 5);
        let (min, max) = iter.size_hint();
        assert_eq!(min, 4);
        assert_eq!(max, Some(4));
    }

    #[test]
    fn test_string_iter() {
        let mut iter = StringIter::new("abc");
        assert_eq!(iter.next(), Some('a'));
        assert_eq!(iter.next(), Some('b'));
        assert_eq!(iter.next(), Some('c'));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_iterator_count() {
        let iter = RangeIter::new_exclusive(1, 4);
        assert_eq!(iter.count(), 3);
    }

    #[test]
    fn test_map_adapter() {
        let iter = RangeIter::new_exclusive(1, 4);
        let mut map_iter = Map::new(iter, |x| x * 2);
        assert_eq!(map_iter.next(), Some(2));
        assert_eq!(map_iter.next(), Some(4));
        assert_eq!(map_iter.next(), Some(6));
        assert_eq!(map_iter.next(), None);
    }

    #[test]
    fn test_filter_adapter() {
        let iter = RangeIter::new_exclusive(1, 6);
        let mut filter_iter = Filter::new(iter, |x| x % 2 == 0);
        assert_eq!(filter_iter.next(), Some(2));
        assert_eq!(filter_iter.next(), Some(4));
        assert_eq!(filter_iter.next(), None);
    }
}
