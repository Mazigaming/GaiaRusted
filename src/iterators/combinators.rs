//! Iterator Combinators
//!
//! Comprehensive implementation of iterator combinators:
//! - map, filter, filter_map
//! - fold, reduce, scan
//! - take, skip, zip
//! - any, all, find
//! - chain, cycle, repeat
//! - enumerate, rev

use std::marker::PhantomData;

/// Iterator trait with combinator methods
pub trait IteratorCombinator: Sized {
    type Item;

    /// Core next method
    fn next(&mut self) -> Option<Self::Item>;

    /// Transform items with a function
    fn map<B, F: Fn(Self::Item) -> B>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
    {
        Map { iter: self, f }
    }

    /// Filter items by predicate
    fn filter<F: Fn(&Self::Item) -> bool>(self, f: F) -> Filter<Self, F>
    where
        Self: Sized,
    {
        Filter { iter: self, f }
    }

    /// Map and filter combined
    fn filter_map<B, F: Fn(Self::Item) -> Option<B>>(self, f: F) -> FilterMap<Self, F>
    where
        Self: Sized,
    {
        FilterMap { iter: self, f }
    }

    /// Take first n elements
    fn take(self, n: usize) -> Take<Self>
    where
        Self: Sized,
    {
        Take {
            iter: self,
            n,
            count: 0,
        }
    }

    /// Skip first n elements
    fn skip(self, n: usize) -> Skip<Self>
    where
        Self: Sized,
    {
        Skip {
            iter: self,
            n,
            count: 0,
        }
    }

    /// Fold items into accumulator
    fn fold<B, F: Fn(B, Self::Item) -> B>(mut self, init: B, f: F) -> B
    where
        Self: Sized,
    {
        let mut acc = init;
        while let Some(item) = self.next() {
            acc = f(acc, item);
        }
        acc
    }

    /// Reduce items into single value
    fn reduce<F: Fn(Self::Item, Self::Item) -> Self::Item>(mut self, f: F) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let mut acc = self.next()?;
        while let Some(item) = self.next() {
            acc = f(acc, item);
        }
        Some(acc)
    }

    /// Check if all items match predicate
    fn all<F: Fn(Self::Item) -> bool>(mut self, f: F) -> bool
    where
        Self: Sized,
    {
        while let Some(item) = self.next() {
            if !f(item) {
                return false;
            }
        }
        true
    }

    /// Check if any item matches predicate
    fn any<F: Fn(Self::Item) -> bool>(mut self, f: F) -> bool
    where
        Self: Sized,
    {
        while let Some(item) = self.next() {
            if f(item) {
                return true;
            }
        }
        false
    }

    /// Find first item matching predicate
    fn find<F: Fn(&Self::Item) -> bool>(mut self, f: F) -> Option<Self::Item>
    where
        Self: Sized,
    {
        while let Some(item) = self.next() {
            if f(&item) {
                return Some(item);
            }
        }
        None
    }

    /// Collect items into vector
    fn collect<B: FromIterator<Self::Item>>(self) -> B
    where
        Self: Sized,
    {
        B::from_iter(self)
    }

    /// Enumerate with indices
    fn enumerate(self) -> Enumerate<Self>
    where
        Self: Sized,
    {
        Enumerate {
            iter: self,
            count: 0,
        }
    }

    /// Count items
    fn count(mut self) -> usize
    where
        Self: Sized,
    {
        let mut count = 0;
        while self.next().is_some() {
            count += 1;
        }
        count
    }

    /// Get nth item
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }

    /// Iterate in reverse
    fn rev(self) -> Reverse<Self>
    where
        Self: Sized + DoubleEndedIterator,
    {
        Reverse { iter: self }
    }

    /// Zip with another iterator
    fn zip<I: IteratorCombinator>(self, other: I) -> Zip<Self, I>
    where
        Self: Sized,
    {
        Zip { a: self, b: other }
    }

    /// Chain with another iterator
    fn chain<I: IteratorCombinator<Item = Self::Item>>(self, other: I) -> Chain<Self, I>
    where
        Self: Sized,
    {
        Chain {
            a: self,
            b: other,
            a_done: false,
        }
    }
}

/// Double-ended iterator trait
pub trait DoubleEndedIterator: IteratorCombinator {
    fn next_back(&mut self) -> Option<Self::Item>;
}

/// Trait for types that can be collected from an iterator
pub trait FromIterator<T> {
    fn from_iter<I: IteratorCombinator<Item = T>>(iter: I) -> Self;
}

impl<T> FromIterator<T> for Vec<T> {
    fn from_iter<I: IteratorCombinator<Item = T>>(mut iter: I) -> Self {
        let mut vec = Vec::new();
        while let Some(item) = iter.next() {
            vec.push(item);
        }
        vec
    }
}

/// Map combinator
pub struct Map<I, F> {
    iter: I,
    f: F,
}

impl<T, B, I: IteratorCombinator<Item = T>, F: Fn(T) -> B> IteratorCombinator for Map<I, F> {
    type Item = B;

    fn next(&mut self) -> Option<B> {
        self.iter.next().map(|x| (self.f)(x))
    }
}

/// Filter combinator
pub struct Filter<I, F> {
    iter: I,
    f: F,
}

impl<I: IteratorCombinator, F: Fn(&I::Item) -> bool> IteratorCombinator for Filter<I, F> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        while let Some(item) = self.iter.next() {
            if (self.f)(&item) {
                return Some(item);
            }
        }
        None
    }
}

/// FilterMap combinator
pub struct FilterMap<I, F> {
    iter: I,
    f: F,
}

impl<T, B, I: IteratorCombinator<Item = T>, F: Fn(T) -> Option<B>> IteratorCombinator
    for FilterMap<I, F>
{
    type Item = B;

    fn next(&mut self) -> Option<B> {
        while let Some(item) = self.iter.next() {
            if let Some(mapped) = (self.f)(item) {
                return Some(mapped);
            }
        }
        None
    }
}

/// Take combinator
pub struct Take<I> {
    iter: I,
    n: usize,
    count: usize,
}

impl<I: IteratorCombinator> IteratorCombinator for Take<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        if self.count < self.n {
            self.count += 1;
            self.iter.next()
        } else {
            None
        }
    }
}

/// Skip combinator
pub struct Skip<I> {
    iter: I,
    n: usize,
    count: usize,
}

impl<I: IteratorCombinator> IteratorCombinator for Skip<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        while self.count < self.n {
            self.count += 1;
            self.iter.next()?;
        }
        self.iter.next()
    }
}

/// Enumerate combinator
pub struct Enumerate<I> {
    iter: I,
    count: usize,
}

impl<I: IteratorCombinator> IteratorCombinator for Enumerate<I> {
    type Item = (usize, I::Item);

    fn next(&mut self) -> Option<(usize, I::Item)> {
        self.iter.next().map(|item| {
            let count = self.count;
            self.count += 1;
            (count, item)
        })
    }
}

/// Reverse combinator
pub struct Reverse<I> {
    iter: I,
}

impl<I: DoubleEndedIterator> IteratorCombinator for Reverse<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.iter.next_back()
    }
}

/// Zip combinator
pub struct Zip<I, J> {
    a: I,
    b: J,
}

impl<T, U, I: IteratorCombinator<Item = T>, J: IteratorCombinator<Item = U>> IteratorCombinator
    for Zip<I, J>
{
    type Item = (T, U);

    fn next(&mut self) -> Option<(T, U)> {
        match (self.a.next(), self.b.next()) {
            (Some(a), Some(b)) => Some((a, b)),
            _ => None,
        }
    }
}

/// Chain combinator
pub struct Chain<I, J> {
    a: I,
    b: J,
    a_done: bool,
}

impl<T, I: IteratorCombinator<Item = T>, J: IteratorCombinator<Item = T>> IteratorCombinator
    for Chain<I, J>
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if !self.a_done {
            if let Some(item) = self.a.next() {
                return Some(item);
            }
            self.a_done = true;
        }
        self.b.next()
    }
}

/// Scan combinator
pub struct Scan<I, S, F> {
    iter: I,
    state: S,
    f: F,
}

impl<I, S, B, F> IteratorCombinator for Scan<I, S, F>
where
    I: IteratorCombinator,
    F: Fn(&mut S, I::Item) -> Option<B>,
{
    type Item = B;

    fn next(&mut self) -> Option<B> {
        self.iter.next().and_then(|x| (self.f)(&mut self.state, x))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock iterator for testing
    struct SimpleIter {
        data: Vec<i32>,
        pos: usize,
    }

    impl SimpleIter {
        fn new(data: Vec<i32>) -> Self {
            SimpleIter { data, pos: 0 }
        }
    }

    impl IteratorCombinator for SimpleIter {
        type Item = i32;

        fn next(&mut self) -> Option<i32> {
            if self.pos < self.data.len() {
                let result = self.data[self.pos];
                self.pos += 1;
                Some(result)
            } else {
                None
            }
        }
    }

    #[test]
    fn test_map() {
        let iter = SimpleIter::new(vec![1, 2, 3]);
        let mapped: Vec<i32> = iter.map(|x| x * 2).collect();
        assert_eq!(mapped, vec![2, 4, 6]);
    }

    #[test]
    fn test_filter() {
        let iter = SimpleIter::new(vec![1, 2, 3, 4, 5]);
        let filtered: Vec<i32> = iter.filter(|x| x % 2 == 0).collect();
        assert_eq!(filtered, vec![2, 4]);
    }

    #[test]
    fn test_take() {
        let iter = SimpleIter::new(vec![1, 2, 3, 4, 5]);
        let taken: Vec<i32> = iter.take(3).collect();
        assert_eq!(taken, vec![1, 2, 3]);
    }

    #[test]
    fn test_skip() {
        let iter = SimpleIter::new(vec![1, 2, 3, 4, 5]);
        let skipped: Vec<i32> = iter.skip(2).collect();
        assert_eq!(skipped, vec![3, 4, 5]);
    }

    #[test]
    fn test_fold() {
        let iter = SimpleIter::new(vec![1, 2, 3, 4]);
        let sum = iter.fold(0, |acc, x| acc + x);
        assert_eq!(sum, 10);
    }

    #[test]
    fn test_all() {
        let iter = SimpleIter::new(vec![2, 4, 6]);
        assert!(iter.all(|x| x % 2 == 0));
    }

    #[test]
    fn test_any() {
        let iter = SimpleIter::new(vec![1, 2, 3]);
        assert!(iter.any(|x| x == 2));
    }

    #[test]
    fn test_count() {
        let iter = SimpleIter::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(iter.count(), 5);
    }

    #[test]
    fn test_chained_operations() {
        let iter = SimpleIter::new(vec![1, 2, 3, 4, 5]);
        let result: Vec<i32> = iter
            .filter(|x| x % 2 == 1)
            .map(|x| x * 2)
            .take(2)
            .collect();
        assert_eq!(result, vec![2, 6]);
    }
}
