//! # Vec<T> Collection Implementation
//!
//! Implements the `Vec<T>` type - a growable dynamic array that owns its elements.
//! Provides heap-allocated storage with automatic memory management.

use std::collections::HashMap;

/// Represents the Vec<T> type in the type system
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VecType;

/// Represents a Vec<T> value at runtime (generic implementation)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VecValue<T: Clone + PartialEq + Eq + std::fmt::Debug> {
    /// The actual vector data
    data: Vec<T>,
}

impl<T: Clone + PartialEq + Eq + std::fmt::Debug> VecValue<T> {
    /// Create a new empty Vec
    pub fn new() -> Self {
        VecValue { data: Vec::new() }
    }

    /// Create a Vec with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        VecValue {
            data: Vec::with_capacity(capacity),
        }
    }

    /// Get the number of elements in the vec
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the vec is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the capacity of the vec
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Push an element onto the end of the vec
    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    /// Remove and return the last element if it exists
    pub fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }

    /// Clear the vec, removing all elements
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get a reference to the element at the given index
    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    /// Get a mutable reference to the element at the given index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
    }

    /// Get a reference to the first element
    pub fn first(&self) -> Option<&T> {
        self.data.first()
    }

    /// Get a mutable reference to the first element
    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.data.first_mut()
    }

    /// Get a reference to the last element
    pub fn last(&self) -> Option<&T> {
        self.data.last()
    }

    /// Get a mutable reference to the last element
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.data.last_mut()
    }

    /// Insert an element at the given position
    pub fn insert(&mut self, index: usize, value: T) {
        if index <= self.data.len() {
            self.data.insert(index, value);
        }
    }

    /// Remove and return the element at the given position
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index < self.data.len() {
            Some(self.data.remove(index))
        } else {
            None
        }
    }

    /// Swap elements at two indices
    pub fn swap(&mut self, a: usize, b: usize) {
        if a < self.data.len() && b < self.data.len() {
            self.data.swap(a, b);
        }
    }

    /// Reserve capacity for at least n more elements
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    /// Shrink the capacity to fit the current length
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }

    /// Get a slice of the vec
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// Get a mutable slice of the vec
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// Convert to the underlying Vec
    pub fn into_vec(self) -> Vec<T> {
        self.data
    }

    /// Create from a regular Vec
    pub fn from_vec(vec: Vec<T>) -> Self {
        VecValue { data: vec }
    }
}

impl<T: Clone + PartialEq + Eq + std::fmt::Debug + std::cmp::PartialOrd> VecValue<T> {
    /// Check if the vec contains a value
    pub fn contains(&self, value: &T) -> bool {
        self.data.contains(value)
    }

    /// Get the index of a value (first occurrence)
    pub fn index_of(&self, value: &T) -> Option<usize> {
        self.data.iter().position(|x| x == value)
    }
}

impl<T: Clone + PartialEq + Eq + std::fmt::Debug + std::cmp::Ord> VecValue<T> {
    /// Sort the vec in ascending order
    pub fn sort(&mut self) {
        self.data.sort();
    }

    /// Reverse the vec in place
    pub fn reverse(&mut self) {
        self.data.reverse();
    }
}

impl<T: Clone + PartialEq + Eq + std::fmt::Debug> Default for VecValue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + PartialEq + Eq + std::fmt::Debug + std::fmt::Display> std::fmt::Display
    for VecValue<T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, item) in self.data.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", item)?;
        }
        write!(f, "]")
    }
}

/// Vec method registry for the type system
pub struct VecMethodRegistry;

impl VecMethodRegistry {
    /// Get all available methods for Vec<T> type
    pub fn get_methods() -> HashMap<String, String> {
        let mut methods = HashMap::new();

        // Constructors
        methods.insert("new".to_string(), "() -> Vec<T>".to_string());
        methods.insert("with_capacity".to_string(), "(usize) -> Vec<T>".to_string());

        // Query methods
        methods.insert("len".to_string(), "(&self) -> usize".to_string());
        methods.insert("is_empty".to_string(), "(&self) -> bool".to_string());
        methods.insert("capacity".to_string(), "(&self) -> usize".to_string());

        // Modification methods
        methods.insert("push".to_string(), "(&mut self, T)".to_string());
        methods.insert("pop".to_string(), "(&mut self) -> Option<T>".to_string());
        methods.insert("clear".to_string(), "(&mut self)".to_string());
        methods.insert("insert".to_string(), "(&mut self, usize, T)".to_string());
        methods.insert("remove".to_string(), "(&mut self, usize) -> Option<T>".to_string());
        methods.insert("swap".to_string(), "(&mut self, usize, usize)".to_string());

        // Memory methods
        methods.insert("reserve".to_string(), "(&mut self, usize)".to_string());
        methods.insert("shrink_to_fit".to_string(), "(&mut self)".to_string());

        // Access methods
        methods.insert("get".to_string(), "(&self, usize) -> Option<&T>".to_string());
        methods.insert("get_mut".to_string(), "(&mut self, usize) -> Option<&mut T>".to_string());
        methods.insert("first".to_string(), "(&self) -> Option<&T>".to_string());
        methods.insert("first_mut".to_string(), "(&mut self) -> Option<&mut T>".to_string());
        methods.insert("last".to_string(), "(&self) -> Option<&T>".to_string());
        methods.insert("last_mut".to_string(), "(&mut self) -> Option<&mut T>".to_string());

        // Search methods
        methods.insert("contains".to_string(), "(&self, &T) -> bool".to_string());
        methods.insert("index_of".to_string(), "(&self, &T) -> Option<usize>".to_string());

        // Sorting methods
        methods.insert("sort".to_string(), "(&mut self)".to_string());
        methods.insert("reverse".to_string(), "(&mut self)".to_string());

        // Slice methods
        methods.insert("as_slice".to_string(), "(&self) -> &[T]".to_string());
        methods.insert("as_mut_slice".to_string(), "(&mut self) -> &mut [T]".to_string());

        methods
    }

    /// Check if a method exists
    pub fn has_method(name: &str) -> bool {
        Self::get_methods().contains_key(name)
    }

    /// Get method signature
    pub fn get_signature(name: &str) -> Option<String> {
        Self::get_methods().get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_new() {
        let v: VecValue<i32> = VecValue::new();
        assert_eq!(v.len(), 0);
        assert!(v.is_empty());
    }

    #[test]
    fn test_vec_with_capacity() {
        let v: VecValue<i32> = VecValue::with_capacity(10);
        assert!(v.capacity() >= 10);
        assert!(v.is_empty());
    }

    #[test]
    fn test_vec_push() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(1);
        v.push(2);
        v.push(3);
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn test_vec_pop() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(1);
        v.push(2);
        assert_eq!(v.pop(), Some(2));
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_vec_pop_empty() {
        let mut v: VecValue<i32> = VecValue::new();
        assert_eq!(v.pop(), None);
    }

    #[test]
    fn test_vec_clear() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(1);
        v.push(2);
        v.clear();
        assert!(v.is_empty());
    }

    #[test]
    fn test_vec_get() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(1);
        v.push(2);
        assert_eq!(v.get(0), Some(&1));
        assert_eq!(v.get(1), Some(&2));
        assert_eq!(v.get(2), None);
    }

    #[test]
    fn test_vec_first() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(1);
        v.push(2);
        assert_eq!(v.first(), Some(&1));
    }

    #[test]
    fn test_vec_last() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(1);
        v.push(2);
        assert_eq!(v.last(), Some(&2));
    }

    #[test]
    fn test_vec_insert() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(1);
        v.push(3);
        v.insert(1, 2);
        assert_eq!(v.len(), 3);
        assert_eq!(v.get(1), Some(&2));
    }

    #[test]
    fn test_vec_remove() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(1);
        v.push(2);
        v.push(3);
        assert_eq!(v.remove(1), Some(2));
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn test_vec_swap() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(1);
        v.push(2);
        v.swap(0, 1);
        assert_eq!(v.get(0), Some(&2));
        assert_eq!(v.get(1), Some(&1));
    }

    #[test]
    fn test_vec_contains() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(1);
        v.push(2);
        v.push(3);
        assert!(v.contains(&2));
        assert!(!v.contains(&4));
    }

    #[test]
    fn test_vec_index_of() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(10);
        v.push(20);
        v.push(30);
        assert_eq!(v.index_of(&20), Some(1));
        assert_eq!(v.index_of(&40), None);
    }

    #[test]
    fn test_vec_sort() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(3);
        v.push(1);
        v.push(2);
        v.sort();
        assert_eq!(v.get(0), Some(&1));
        assert_eq!(v.get(1), Some(&2));
        assert_eq!(v.get(2), Some(&3));
    }

    #[test]
    fn test_vec_reverse() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(1);
        v.push(2);
        v.push(3);
        v.reverse();
        assert_eq!(v.get(0), Some(&3));
        assert_eq!(v.get(2), Some(&1));
    }

    #[test]
    fn test_vec_method_registry() {
        assert!(VecMethodRegistry::has_method("new"));
        assert!(VecMethodRegistry::has_method("push"));
        assert!(VecMethodRegistry::has_method("pop"));
        assert!(!VecMethodRegistry::has_method("unknown"));
    }

    #[test]
    fn test_vec_method_signature() {
        let sig = VecMethodRegistry::get_signature("len");
        assert!(sig.is_some());
        assert!(sig.unwrap().contains("usize"));
    }

    #[test]
    fn test_vec_as_slice() {
        let mut v: VecValue<i32> = VecValue::new();
        v.push(1);
        v.push(2);
        let slice = v.as_slice();
        assert_eq!(slice.len(), 2);
    }
}
