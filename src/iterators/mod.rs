//! # High-Performance Iterator System for GaiaRusted
//!
//! Implements zero-cost abstractions and SIMD-optimized iterators
//! for collections (Vec, HashMap, HashSet) with:
//! - Compile-time specialization through monomorphization
//! - SIMD-friendly operations for vectorizable code
//! - Lazy evaluation and iterator fusion
//! - Inline-friendly method chains
//!
//! Design principles:
//! 1. Zero-cost abstraction - no runtime overhead vs hand-written loops
//! 2. Compiler-driven optimization - MIR-level fusion and SIMD detection
//! 3. Type-driven specialization - different code for different element types
//! 4. Memory efficiency - avoid unnecessary allocations and copies

pub mod simd;
pub mod monomorphization;

use crate::lowering::HirType;

/// Handler for iterator-related method calls
/// Provides type checking and signature information for iterator methods
pub struct IteratorMethodHandler;

impl IteratorMethodHandler {
    /// Check if a method is an iterator method
    pub fn is_iterator_method(method_name: &str) -> bool {
        matches!(
            method_name,
            "next"
                | "count"
                | "for_each"
                | "map"
                | "filter"
                | "fold"
                | "sum"
                | "collect"
                | "iter"
                | "iter_mut"
                | "into_iter"
        )
    }

    /// Get the method signature for an iterator method
    /// Returns (parameter_types, return_type)
    pub fn get_method_signature(obj_ty: &HirType, method_name: &str) -> Option<(Vec<HirType>, HirType)> {
        match method_name {
            "iter" => Some((vec![], HirType::Named("Iterator".to_string()))),
            "iter_mut" => Some((vec![], HirType::Named("Iterator".to_string()))),
            "into_iter" => Some((vec![], HirType::Named("Iterator".to_string()))),
            "next" => Some((vec![], HirType::Named("Option".to_string()))),
            "count" => Some((vec![], HirType::Int64)),
            "collect" => Some((vec![], obj_ty.clone())),
            "sum" => {
                // Sum returns the element type
                get_element_type(obj_ty).map(|et| (vec![], et))
            }
            _ => None,
        }
    }
}

/// Extract the element type from a collection type
fn get_element_type(ty: &HirType) -> Option<HirType> {
    match ty {
        // For Named types like "Vec", "HashMap", "HashSet"
        // we return the element type based on the type name
        HirType::Named(name) => {
            match name.as_str() {
                name if name.contains("Vec") => Some(HirType::Int64), // Default to i64
                name if name.contains("HashMap") => Some(HirType::Int64),
                name if name.contains("HashSet") => Some(HirType::Int64),
                _ => None,
            }
        }
        // For other types, they might not support iteration
        _ => None,
    }
}

/// IteratorTrait for generic iteration over collections
/// Designed to be fully inlined and specialized at compile time
pub trait GaiaIterator {
    /// The type of element being iterated
    type Item;

    /// Advance iterator and return next element
    /// Must be inlined for zero-cost abstraction
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item>;

    /// Count remaining elements without consuming
    /// Can be specialized for exact-size iterators
    #[inline]
    fn count(mut self) -> usize
    where
        Self: Sized,
    {
        let mut n = 0;
        while self.next().is_some() {
            n += 1;
        }
        n
    }

    /// Apply a closure to each element
    /// Designed to fuse with following operations
    #[inline]
    fn for_each<F: Fn(Self::Item)>(mut self, f: F)
    where
        Self: Sized,
    {
        while let Some(item) = self.next() {
            f(item);
        }
    }

    /// Transform elements with a closure
    /// Creates a map iterator that can be further fused
    #[inline]
    fn map<F, U>(self, f: F) -> MapIterator<Self, F>
    where
        F: Fn(Self::Item) -> U,
        Self: Sized,
    {
        MapIterator {
            inner: self,
            f,
        }
    }

    /// Filter elements with a predicate
    /// Creates a filter iterator for selective iteration
    #[inline]
    fn filter<F>(self, f: F) -> FilterIterator<Self, F>
    where
        F: Fn(&Self::Item) -> bool,
        Self: Sized,
    {
        FilterIterator {
            inner: self,
            f,
        }
    }

    /// Fold/reduce elements to a single value
    /// Optimized for commutative/associative operations (SIMD-friendly)
    #[inline]
    fn fold<B, F>(mut self, init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let mut accum = init;
        while let Some(item) = self.next() {
            accum = f(accum, item);
        }
        accum
    }

    /// Sum all elements
    /// Specialized for numeric types with SIMD potential
    #[inline]
    fn sum(self) -> Self::Item
    where
        Self: Sized,
        Self::Item: std::ops::Add<Output = Self::Item> + Default,
    {
        self.fold(Self::Item::default(), |a, b| a + b)
    }

    /// Collect elements into a Vec
    /// Type-specialized collection
    #[inline]
    fn collect_vec(mut self) -> Vec<Self::Item>
    where
        Self: Sized,
    {
        let mut result = Vec::new();
        while let Some(item) = self.next() {
            result.push(item);
        }
        result
    }
}

/// Map iterator - transforms elements through a closure
/// Zero-cost when inlined and fused with other operations
pub struct MapIterator<I: GaiaIterator, F> {
    inner: I,
    f: F,
}

impl<I, F, U> GaiaIterator for MapIterator<I, F>
where
    I: GaiaIterator,
    F: Fn(I::Item) -> U,
{
    type Item = U;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(&self.f)
    }

    #[inline]
    fn fold<B, G>(self, init: B, mut g: G) -> B
    where
        Self: Sized,
        G: FnMut(B, Self::Item) -> B,
    {
        self.inner.fold(init, |acc, item| {
            g(acc, (self.f)(item))
        })
    }
}

/// Filter iterator - selects elements matching a predicate
/// Optimized for branch prediction and vectorization
pub struct FilterIterator<I: GaiaIterator, F> {
    inner: I,
    f: F,
}

impl<I, F> GaiaIterator for FilterIterator<I, F>
where
    I: GaiaIterator,
    F: Fn(&I::Item) -> bool,
{
    type Item = I::Item;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.inner.next() {
            if (self.f)(&item) {
                return Some(item);
            }
        }
        None
    }
}

/// VecIterator - zero-cost iterator over Vec<T>
/// Specialized for different element types
pub struct VecIterator<T> {
    ptr: *const T,
    end: *const T,
}

impl<T: Clone> VecIterator<T> {
    /// Create iterator from Vec reference
    #[inline]
    pub fn new(vec: &[T]) -> Self {
        let ptr = vec.as_ptr();
        let end = unsafe { ptr.add(vec.len()) };
        VecIterator { ptr, end }
    }
}

impl<T: Clone> GaiaIterator for VecIterator<T> {
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr < self.end {
            unsafe {
                let item = (*self.ptr).clone();
                self.ptr = self.ptr.add(1);
                Some(item)
            }
        } else {
            None
        }
    }

    #[inline]
    fn count(self) -> usize {
        (self.end as usize - self.ptr as usize) / std::mem::size_of::<T>()
    }
}

/// HashSetIterator - iterator over HashSet<T>
/// Optimized for set operations
pub struct HashSetIterator<'a, T: Clone + std::cmp::Eq + std::hash::Hash> {
    inner: std::collections::hash_set::Iter<'a, T>,
}

impl<'a, T: Clone + std::cmp::Eq + std::hash::Hash> GaiaIterator for HashSetIterator<'a, T> {
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|t| t.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_iterator_basic() {
        let vec = vec![1, 2, 3, 4, 5];
        let iter = VecIterator::new(&vec);
        let sum = iter.fold(0, |a, b| a + b);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_vec_iterator_count() {
        let vec = vec![1i64, 2, 3, 4, 5];
        let iter = VecIterator::new(&vec);
        assert_eq!(iter.count(), 5);
    }

    #[test]
    fn test_iterator_map_fusion() {
        let vec = vec![1, 2, 3, 4, 5];
        let iter = VecIterator::new(&vec);
        let sum = iter.map(|x| x * 2).fold(0, |a, b| a + b);
        assert_eq!(sum, 30); // 2 + 4 + 6 + 8 + 10
    }

    #[test]
    fn test_iterator_filter_map() {
        let vec = vec![1, 2, 3, 4, 5];
        let iter = VecIterator::new(&vec);
        let sum = iter
            .filter(|x| x % 2 == 0)
            .map(|x| x * 3)
            .fold(0, |a, b| a + b);
        assert_eq!(sum, 18); // 2*3 + 4*3 = 6 + 12
    }

    #[test]
    fn test_iterator_collect() {
        let vec = vec![1, 2, 3];
        let iter = VecIterator::new(&vec);
        let doubled: Vec<_> = iter.map(|x| x * 2).collect_vec();
        assert_eq!(doubled, vec![2, 4, 6]);
    }
}
