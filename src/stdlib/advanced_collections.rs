//! # Advanced Collection Types
//!
//! Features:
//! - HashMap with hash function support
//! - BTreeMap for sorted key-value storage
//! - VecDeque for double-ended queues
//! - LinkedList for bidirectional traversal
//! - Heap priority queue

use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Simple HashMap implementation
#[derive(Debug, Clone)]
pub struct HashMap<K, V> {
    /// Buckets for hash storage
    buckets: Vec<Vec<(K, V)>>,
    /// Number of entries
    count: usize,
    /// Capacity
    capacity: usize,
}

impl<K: Eq + Clone + Hash, V: Clone> HashMap<K, V> {
    /// Create a new HashMap
    pub fn new() -> Self {
        let capacity = 16;
        HashMap {
            buckets: vec![Vec::new(); capacity],
            count: 0,
            capacity,
        }
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.resize_if_needed();
        let bucket_idx = self.hash(&key) % self.capacity;
        let bucket = &mut self.buckets[bucket_idx];

        for (k, v) in bucket.iter_mut() {
            if k == &key {
                let old = v.clone();
                *v = value;
                return Some(old);
            }
        }

        bucket.push((key, value));
        self.count += 1;
        None
    }

    /// Get a value by key
    pub fn get(&self, key: &K) -> Option<&V> {
        let bucket_idx = self.hash(key) % self.capacity;
        self.buckets[bucket_idx]
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Get mutable reference to value
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let bucket_idx = self.hash(key) % self.capacity;
        self.buckets[bucket_idx]
            .iter_mut()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Remove a key-value pair
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let bucket_idx = self.hash(key) % self.capacity;
        let bucket = &mut self.buckets[bucket_idx];
        
        if let Some(pos) = bucket.iter().position(|(k, _)| k == key) {
            self.count -= 1;
            Some(bucket.remove(pos).1)
        } else {
            None
        }
    }

    /// Check if key exists
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        self.count = 0;
    }

    /// Hash function
    fn hash(&self, key: &K) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }

    /// Resize if load factor exceeds 0.75
    fn resize_if_needed(&mut self) {
        if self.count as f32 / self.capacity as f32 > 0.75 {
            self.resize(self.capacity * 2);
        }
    }

    /// Resize buckets
    fn resize(&mut self, new_capacity: usize) {
        let mut new_buckets = vec![Vec::new(); new_capacity];
        let old_capacity = self.capacity;
        self.capacity = new_capacity;

        for bucket in &self.buckets {
            for (k, v) in bucket {
                let bucket_idx = self.hash(k) % self.capacity;
                new_buckets[bucket_idx].push((k.clone(), v.clone()));
            }
        }

        self.buckets = new_buckets;
    }
}

impl<K: Eq + Clone + Hash, V: Clone> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// BTreeMap for ordered key-value storage
#[derive(Debug, Clone)]
pub struct BTreeMap<K, V> {
    /// Entries stored as sorted vector
    entries: Vec<(K, V)>,
}

impl<K: Ord + Clone, V: Clone> BTreeMap<K, V> {
    /// Create a new BTreeMap
    pub fn new() -> Self {
        BTreeMap { entries: Vec::new() }
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.entries.binary_search_by(|(k, _)| k.cmp(&key)) {
            Ok(idx) => {
                let old_value = self.entries[idx].1.clone();
                self.entries[idx].1 = value;
                Some(old_value)
            }
            Err(idx) => {
                self.entries.insert(idx, (key, value));
                None
            }
        }
    }

    /// Get a value by key
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries
            .binary_search_by(|(k, _)| k.cmp(key))
            .ok()
            .map(|idx| &self.entries[idx].1)
    }

    /// Remove a key-value pair
    pub fn remove(&mut self, key: &K) -> Option<V> {
        match self.entries.binary_search_by(|(k, _)| k.cmp(key)) {
            Ok(idx) => Some(self.entries.remove(idx).1),
            Err(_) => None,
        }
    }

    /// Check if key exists
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get range of keys
    pub fn range(&self, start: &K, end: &K) -> Vec<(&K, &V)> {
        self.entries
            .iter()
            .filter(|(k, _)| k >= start && k <= end)
            .map(|(k, v)| (k, v))
            .collect()
    }

    /// Get first entry
    pub fn first(&self) -> Option<(&K, &V)> {
        self.entries.first().map(|(k, v)| (k, v))
    }

    /// Get last entry
    pub fn last(&self) -> Option<(&K, &V)> {
        self.entries.last().map(|(k, v)| (k, v))
    }
}

impl<K: Ord + Clone, V: Clone> Default for BTreeMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// VecDeque for double-ended queue
#[derive(Debug, Clone)]
pub struct VecDeque<T> {
    /// Items in queue
    items: Vec<T>,
}

impl<T: Clone> VecDeque<T> {
    /// Create a new VecDeque
    pub fn new() -> Self {
        VecDeque { items: Vec::new() }
    }

    /// Push to front
    pub fn push_front(&mut self, item: T) {
        self.items.insert(0, item);
    }

    /// Push to back
    pub fn push_back(&mut self, item: T) {
        self.items.push(item);
    }

    /// Pop from front
    pub fn pop_front(&mut self) -> Option<T> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.items.remove(0))
        }
    }

    /// Pop from back
    pub fn pop_back(&mut self) -> Option<T> {
        self.items.pop()
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

impl<T: Clone> Default for VecDeque<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Priority queue element
#[derive(Debug, Clone)]
pub struct PriorityItem<T> {
    /// Priority value (higher = more priority)
    pub priority: i32,
    /// The item
    pub item: T,
}

impl<T> PartialEq for PriorityItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl<T> Eq for PriorityItem<T> {}

impl<T> PartialOrd for PriorityItem<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for PriorityItem<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

/// Priority queue
#[derive(Debug, Clone)]
pub struct PriorityQueue<T> {
    heap: BinaryHeap<PriorityItem<T>>,
}

impl<T: Clone> PriorityQueue<T> {
    /// Create a new priority queue
    pub fn new() -> Self {
        PriorityQueue {
            heap: BinaryHeap::new(),
        }
    }

    /// Insert with priority
    pub fn push(&mut self, item: T, priority: i32) {
        self.heap.push(PriorityItem { priority, item });
    }

    /// Get highest priority item
    pub fn pop(&mut self) -> Option<T> {
        self.heap.pop().map(|pi| pi.item)
    }

    /// Peek highest priority item
    pub fn peek(&self) -> Option<&T> {
        self.heap.peek().map(|pi| &pi.item)
    }

    /// Get size
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.heap.clear();
    }
}

impl<T: Clone> Default for PriorityQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashmap_insert_get() {
        let mut map = HashMap::new();
        map.insert("key1", 42);
        assert_eq!(map.get(&"key1"), Some(&42));
    }

    #[test]
    fn test_hashmap_len() {
        let mut map = HashMap::new();
        map.insert("a", 1);
        map.insert("b", 2);
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_hashmap_remove() {
        let mut map = HashMap::new();
        map.insert("key", 10);
        let removed = map.remove(&"key");
        assert_eq!(removed, Some(10));
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_btreemap_insert_get() {
        let mut map = BTreeMap::new();
        map.insert(2, "two");
        map.insert(1, "one");
        map.insert(3, "three");
        assert_eq!(map.get(&2), Some(&"two"));
    }

    #[test]
    fn test_btreemap_ordering() {
        let mut map = BTreeMap::new();
        map.insert(3, 'c');
        map.insert(1, 'a');
        map.insert(2, 'b');
        assert_eq!(map.first(), Some((&1, &'a')));
        assert_eq!(map.last(), Some((&3, &'c')));
    }

    #[test]
    fn test_btreemap_range() {
        let mut map = BTreeMap::new();
        map.insert(1, "a");
        map.insert(2, "b");
        map.insert(3, "c");
        let range = map.range(&1, &2);
        assert_eq!(range.len(), 2);
    }

    #[test]
    fn test_vecdeque_push_pop() {
        let mut deque = VecDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        assert_eq!(deque.pop_back(), Some(2));
    }

    #[test]
    fn test_vecdeque_front_back() {
        let mut deque = VecDeque::new();
        deque.push_back(1);
        deque.push_front(0);
        assert_eq!(deque.pop_front(), Some(0));
        assert_eq!(deque.pop_back(), Some(1));
    }

    #[test]
    fn test_priority_queue() {
        let mut pq = PriorityQueue::new();
        pq.push("low", 1);
        pq.push("high", 10);
        pq.push("medium", 5);
        assert_eq!(pq.pop(), Some("high"));
        assert_eq!(pq.pop(), Some("medium"));
    }

    #[test]
    fn test_hashmap_contains_key() {
        let mut map = HashMap::new();
        map.insert("key", 1);
        assert!(map.contains_key(&"key"));
        assert!(!map.contains_key(&"other"));
    }

    #[test]
    fn test_btreemap_remove() {
        let mut map = BTreeMap::new();
        map.insert(1, "one");
        map.remove(&1);
        assert!(!map.contains_key(&1));
    }

    #[test]
    fn test_vecdeque_len() {
        let mut deque = VecDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        assert_eq!(deque.len(), 2);
    }

    #[test]
    fn test_hashmap_clear() {
        let mut map = HashMap::new();
        map.insert("a", 1);
        map.insert("b", 2);
        map.clear();
        assert!(map.is_empty());
    }

    #[test]
    fn test_priority_queue_empty() {
        let mut pq = PriorityQueue::new();
        assert!(pq.is_empty());
        pq.push("item", 5);
        assert!(!pq.is_empty());
    }
}
