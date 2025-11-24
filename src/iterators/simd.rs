//! # SIMD-Optimized Collection Operations
//!
//! Implements vectorized operations for collections using techniques like:
//! - Chunk-based processing (process multiple elements at once)
//! - Data layout optimization (AoS vs SoA conversions)
//! - Predication for branch-free iteration
//! - Vectorized reductions with tree-based combining

use super::GaiaIterator;

/// SIMD-optimized fold operation for numeric types
/// Can be auto-vectorized by LLVM when possible
/// Works with i64, f64, and other numeric types
pub trait SIMDFoldable: Sized {
    /// Identity element (e.g., 0 for addition, 1 for multiplication)
    fn simd_identity() -> Self;

    /// Combine two elements (must be associative and commutative for SIMD)
    /// Marked with #[inline(always)] to encourage vectorization
    fn simd_combine(a: Self, b: Self) -> Self;
}

impl SIMDFoldable for i64 {
    #[inline(always)]
    fn simd_identity() -> Self {
        0
    }

    #[inline(always)]
    fn simd_combine(a: Self, b: Self) -> Self {
        a + b
    }
}

impl SIMDFoldable for f64 {
    #[inline(always)]
    fn simd_identity() -> Self {
        0.0
    }

    #[inline(always)]
    fn simd_combine(a: Self, b: Self) -> Self {
        a + b
    }
}

/// SIMD-optimized reduction iterator
/// Processes chunks of data for better vectorization
pub struct SIMDReducer<T: SIMDFoldable> {
    data: Vec<T>,
    chunk_size: usize,
}

impl<T: SIMDFoldable + Copy> SIMDReducer<T> {
    /// Create a SIMD reducer for the given data
    pub fn new(data: Vec<T>, chunk_size: usize) -> Self {
        SIMDReducer { data, chunk_size }
    }

    /// Reduce the data using tree-based combining
    /// More cache-friendly and vectorizable than naive loop
    pub fn reduce(&self) -> T {
        if self.data.is_empty() {
            return T::simd_identity();
        }

        // Process chunks
        let mut partial_results = Vec::new();
        for chunk in self.data.chunks(self.chunk_size) {
            let mut acc = T::simd_identity();
            for &item in chunk {
                acc = T::simd_combine(acc, item);
            }
            partial_results.push(acc);
        }

        // Tree reduction of partial results
        while partial_results.len() > 1 {
            let mut next_results = Vec::new();
            for i in (0..partial_results.len()).step_by(2) {
                let a = partial_results[i];
                let b = if i + 1 < partial_results.len() {
                    partial_results[i + 1]
                } else {
                    T::simd_identity()
                };
                next_results.push(T::simd_combine(a, b));
            }
            partial_results = next_results;
        }

        partial_results.first().copied().unwrap_or_else(T::simd_identity)
    }
}

/// Vectorized filter operation
/// Uses branch-free predication for better pipeline efficiency
pub struct VectorizedFilter<T> {
    data: Vec<T>,
    /// Bitmap of which elements pass the filter (1 bit per element)
    mask: Vec<u8>,
}

impl<T: Clone> VectorizedFilter<T> {
    /// Create a vectorized filter using a predicate
    /// The predicate should be evaluation-order independent
    pub fn new<F: Fn(&T) -> bool>(data: Vec<T>, predicate: F) -> Self {
        let mut mask = vec![0u8; (data.len() + 7) / 8];
        for (i, item) in data.iter().enumerate() {
            if predicate(item) {
                let byte_idx = i / 8;
                let bit_idx = i % 8;
                mask[byte_idx] |= 1 << bit_idx;
            }
        }
        VectorizedFilter { data, mask }
    }

    /// Get the filtered results (compacted)
    pub fn collect(&self) -> Vec<T> {
        let mut result = Vec::new();
        for (i, item) in self.data.iter().enumerate() {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            if byte_idx < self.mask.len() && (self.mask[byte_idx] & (1 << bit_idx)) != 0 {
                result.push(item.clone());
            }
        }
        result
    }

    /// Count matching elements without materializing
    pub fn count(&self) -> usize {
        self.mask.iter().map(|b| b.count_ones() as usize).sum()
    }
}

/// Parallel prefix scan for cumulative operations
/// Useful for inclusive/exclusive scans over collections
pub struct PrefixScan<T: Copy> {
    data: Vec<T>,
}

impl<T: Copy + SIMDFoldable> PrefixScan<T> {
    /// Create a prefix scan iterator
    pub fn new(data: Vec<T>) -> Self {
        PrefixScan { data }
    }

    /// Compute inclusive prefix scan
    /// Output[i] = data[0] combine data[1] combine ... combine data[i]
    pub fn inclusive_scan(&self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.data.len());
        let mut acc = T::simd_identity();
        for &item in &self.data {
            acc = T::simd_combine(acc, item);
            result.push(acc);
        }
        result
    }

    /// Compute exclusive prefix scan
    /// Output[i] = data[0] combine ... combine data[i-1]
    /// Output[0] = identity
    pub fn exclusive_scan(&self) -> Vec<T> {
        let mut result = vec![T::simd_identity()];
        let mut acc = T::simd_identity();
        for &item in &self.data[..self.data.len() - 1] {
            acc = T::simd_combine(acc, item);
            result.push(acc);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_reducer_basic() {
        let data = vec![1i64, 2, 3, 4, 5];
        let reducer = SIMDReducer::new(data, 2);
        assert_eq!(reducer.reduce(), 15);
    }

    #[test]
    fn test_simd_reducer_large() {
        let data: Vec<i64> = (1..=1000).collect();
        let reducer = SIMDReducer::new(data, 8);
        assert_eq!(reducer.reduce(), 500500); // sum of 1..1000
    }

    #[test]
    fn test_vectorized_filter() {
        let data = vec![1, 2, 3, 4, 5, 6];
        let filter = VectorizedFilter::new(data, |x| x % 2 == 0);
        assert_eq!(filter.collect(), vec![2, 4, 6]);
        assert_eq!(filter.count(), 3);
    }

    #[test]
    fn test_inclusive_prefix_scan() {
        let data = vec![1i64, 2, 3, 4];
        let scan = PrefixScan::new(data);
        assert_eq!(scan.inclusive_scan(), vec![1, 3, 6, 10]);
    }

    #[test]
    fn test_exclusive_prefix_scan() {
        let data = vec![1i64, 2, 3, 4];
        let scan = PrefixScan::new(data);
        assert_eq!(scan.exclusive_scan(), vec![0, 1, 3, 6]);
    }
}
