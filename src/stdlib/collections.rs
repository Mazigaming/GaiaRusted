//! Collections: Vec, HashMap, HashSet

use std::collections::HashMap as StdHashMap;
use std::collections::HashSet as StdHashSet;

/// Vector (dynamic array)
#[derive(Debug, Clone)]
pub struct Vec<T> {
    data: std::vec::Vec<T>,
}

impl<T> Vec<T> {
    /// Create new empty vector
    pub fn new() -> Self {
        Vec {
            data: std::vec::Vec::new(),
        }
    }

    /// Create vector with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Vec {
            data: std::vec::Vec::with_capacity(capacity),
        }
    }

    /// Push element
    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    /// Pop element
    pub fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }

    /// Get element by index
    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    /// Get mutable element
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
    }

    /// Length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clear vector
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Iterate over references
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    /// Reserve capacity
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    /// Remove element at index
    pub fn remove(&mut self, index: usize) -> T {
        self.data.remove(index)
    }

    /// Insert at index
    pub fn insert(&mut self, index: usize, value: T) {
        self.data.insert(index, value);
    }
}

/// Hash Map
#[derive(Debug, Clone)]
pub struct HashMap<K: std::hash::Hash + Eq, V> {
    data: StdHashMap<K, V>,
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> HashMap<K, V> {
    /// Create new empty hashmap
    pub fn new() -> Self {
        HashMap {
            data: StdHashMap::new(),
        }
    }

    /// Insert key-value pair
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.data.insert(key, value)
    }

    /// Get value by key
    pub fn get(&self, key: &K) -> Option<&V> {
        self.data.get(key)
    }

    /// Get mutable value
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.data.get_mut(key)
    }

    /// Remove entry
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.data.remove(key)
    }

    /// Contains key
    pub fn contains_key(&self, key: &K) -> bool {
        self.data.contains_key(key)
    }

    /// Length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clear map
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get all keys
    pub fn keys(&self) -> std::vec::Vec<K> {
        self.data.keys().cloned().collect()
    }

    /// Get all values
    pub fn values(&self) -> std::vec::Vec<V> {
        self.data.values().cloned().collect()
    }
}

/// Hash Set
#[derive(Debug, Clone)]
pub struct HashSet<T: std::hash::Hash + Eq> {
    data: StdHashSet<T>,
}

impl<T: std::hash::Hash + Eq + Clone> HashSet<T> {
    /// Create new empty hashset
    pub fn new() -> Self {
        HashSet {
            data: StdHashSet::new(),
        }
    }

    /// Insert element
    pub fn insert(&mut self, value: T) -> bool {
        self.data.insert(value)
    }

    /// Remove element
    pub fn remove(&mut self, value: &T) -> bool {
        self.data.remove(value)
    }

    /// Contains element
    pub fn contains(&self, value: &T) -> bool {
        self.data.contains(value)
    }

    /// Length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clear set
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Union
    pub fn union(&self, other: &HashSet<T>) -> HashSet<T> {
        let mut result = self.clone();
        for item in &other.data {
            result.insert(item.clone());
        }
        result
    }

    /// Intersection
    pub fn intersection(&self, other: &HashSet<T>) -> HashSet<T> {
        let mut result = HashSet::new();
        for item in &self.data {
            if other.contains(item) {
                result.insert(item.clone());
            }
        }
        result
    }

    /// Difference
    pub fn difference(&self, other: &HashSet<T>) -> HashSet<T> {
        let mut result = self.clone();
        for item in &other.data {
            result.remove(item);
        }
        result
    }

    /// Check if self is a subset of other
    /// All elements in self must be in other
    pub fn is_subset(&self, other: &HashSet<T>) -> bool {
        for item in &self.data {
            if !other.contains(item) {
                return false;
            }
        }
        true
    }

    /// Check if self is a superset of other
    /// All elements in other must be in self
    pub fn is_superset(&self, other: &HashSet<T>) -> bool {
        other.is_subset(self)
    }

    /// Check if self and other have no elements in common
    pub fn is_disjoint(&self, other: &HashSet<T>) -> bool {
        for item in &self.data {
            if other.contains(item) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_creation() {
        let vec: Vec<i32> = Vec::new();
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
    }

    #[test]
    fn test_vec_push_pop() {
        let mut vec = Vec::new();
        vec.push(1);
        vec.push(2);
        assert_eq!(vec.len(), 2);
        assert_eq!(vec.pop(), Some(2));
    }

    #[test]
    fn test_vec_get() {
        let mut vec = Vec::new();
        vec.push(10);
        vec.push(20);
        assert_eq!(vec.get(0), Some(&10));
        assert_eq!(vec.get(5), None);
    }

    #[test]
    fn test_hashmap_creation() {
        let map: HashMap<String, i32> = HashMap::new();
        assert!(map.is_empty());
    }

    #[test]
    fn test_hashmap_insert_get() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), 42);
        assert_eq!(map.get(&"key".to_string()), Some(&42));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_hashmap_contains_key() {
        let mut map = HashMap::new();
        map.insert("exists".to_string(), 1);
        assert!(map.contains_key(&"exists".to_string()));
        assert!(!map.contains_key(&"missing".to_string()));
    }

    #[test]
    fn test_hashset_creation() {
        let set: HashSet<i32> = HashSet::new();
        assert!(set.is_empty());
    }

    #[test]
    fn test_hashset_insert_contains() {
        let mut set = HashSet::new();
        set.insert(42);
        assert!(set.contains(&42));
        assert!(!set.contains(&99));
    }

    #[test]
    fn test_hashset_union() {
        let mut set1 = HashSet::new();
        set1.insert(1);
        set1.insert(2);

        let mut set2 = HashSet::new();
        set2.insert(2);
        set2.insert(3);

        let union = set1.union(&set2);
        assert_eq!(union.len(), 3);
    }

    #[test]
    fn test_hashset_intersection() {
        let mut set1 = HashSet::new();
        set1.insert(1);
        set1.insert(2);

        let mut set2 = HashSet::new();
        set2.insert(2);
        set2.insert(3);

        let inter = set1.intersection(&set2);
        assert_eq!(inter.len(), 1);
        assert!(inter.contains(&2));
    }

    #[test]
    fn test_hashset_difference() {
        let mut set1 = HashSet::new();
        set1.insert(1);
        set1.insert(2);

        let mut set2 = HashSet::new();
        set2.insert(2);

        let diff = set1.difference(&set2);
        assert_eq!(diff.len(), 1);
        assert!(diff.contains(&1));
    }
}
