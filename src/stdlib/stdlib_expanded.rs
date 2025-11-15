//! Comprehensive standard library for Gaia Rust compiler
//!
//! Includes:
//! - String operations
//! - Collections (Vec, HashMap, HashSet)
//! - File I/O
//! - Formatting
//! - Math operations
//! - Type conversions

pub mod string_ops {
    pub fn string_len(s: &str) -> usize {
        s.len()
    }

    pub fn string_is_empty(s: &str) -> bool {
        s.is_empty()
    }

    pub fn string_contains(s: &str, pattern: &str) -> bool {
        s.contains(pattern)
    }

    pub fn string_starts_with(s: &str, prefix: &str) -> bool {
        s.starts_with(prefix)
    }

    pub fn string_ends_with(s: &str, suffix: &str) -> bool {
        s.ends_with(suffix)
    }

    pub fn string_trim(s: &str) -> &str {
        s.trim()
    }

    pub fn string_trim_start(s: &str) -> &str {
        s.trim_start()
    }

    pub fn string_trim_end(s: &str) -> &str {
        s.trim_end()
    }

    pub fn string_chars_count(s: &str) -> usize {
        s.chars().count()
    }

    pub fn string_to_uppercase(s: &str) -> String {
        s.to_uppercase()
    }

    pub fn string_to_lowercase(s: &str) -> String {
        s.to_lowercase()
    }

    pub fn string_reverse(s: &str) -> String {
        s.chars().rev().collect()
    }

    pub fn string_replace(s: &str, from: &str, to: &str) -> String {
        s.replace(from, to)
    }

    pub fn string_split<'a>(s: &'a str, delimiter: &str) -> std::vec::Vec<&'a str> {
        s.split(delimiter).collect()
    }

    pub fn string_join(strings: &[&str], separator: &str) -> String {
        strings.join(separator)
    }

    pub fn string_repeat(s: &str, count: usize) -> String {
        s.repeat(count)
    }

    pub fn string_substring(s: &str, start: usize, end: usize) -> &str {
        if end <= s.len() && start <= end {
            &s[start..end]
        } else {
            ""
        }
    }
}

pub mod collections {
    use std::collections::{HashMap as StdHashMap, HashSet as StdHashSet};

    pub struct Vector<T> {
        data: std::vec::Vec<T>,
    }

    impl<T: Clone> Vector<T> {
        pub fn new() -> Self {
            Vector {
                data: std::vec::Vec::new(),
            }
        }

        pub fn with_capacity(capacity: usize) -> Self {
            Vector {
                data: std::vec::Vec::with_capacity(capacity),
            }
        }

        pub fn push(&mut self, value: T) {
            self.data.push(value);
        }

        pub fn pop(&mut self) -> Option<T> {
            self.data.pop()
        }

        pub fn insert(&mut self, index: usize, value: T) {
            if index <= self.data.len() {
                self.data.insert(index, value);
            }
        }

        pub fn remove(&mut self, index: usize) -> Option<T> {
            if index < self.data.len() {
                Some(self.data.remove(index))
            } else {
                None
            }
        }

        pub fn len(&self) -> usize {
            self.data.len()
        }

        pub fn is_empty(&self) -> bool {
            self.data.is_empty()
        }

        pub fn clear(&mut self) {
            self.data.clear();
        }

        pub fn get(&self, index: usize) -> Option<&T> {
            self.data.get(index)
        }

        pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
            self.data.get_mut(index)
        }

        pub fn first(&self) -> Option<&T> {
            self.data.first()
        }

        pub fn last(&self) -> Option<&T> {
            self.data.last()
        }

        pub fn reverse(&mut self) {
            self.data.reverse();
        }

        pub fn sort(&mut self)
        where
            T: Ord,
        {
            self.data.sort();
        }

        pub fn contains(&self, value: &T) -> bool
        where
            T: PartialEq,
        {
            self.data.contains(value)
        }

        pub fn drain(&mut self) -> std::vec::Drain<'_, T> {
            self.data.drain(..)
        }

        pub fn extend(&mut self, iter: impl IntoIterator<Item = T>) {
            self.data.extend(iter);
        }

        pub fn retain(&mut self, f: impl FnMut(&T) -> bool) {
            self.data.retain(f);
        }

        pub fn swap(&mut self, a: usize, b: usize) {
            if a < self.data.len() && b < self.data.len() {
                self.data.swap(a, b);
            }
        }

        pub fn binary_search(&self, value: &T) -> Result<usize, usize>
        where
            T: Ord,
        {
            self.data.binary_search(value)
        }
    }

    pub struct HashMap<K, V>
    where
        K: std::cmp::Eq + std::hash::Hash + Clone,
        V: Clone,
    {
        data: StdHashMap<K, V>,
    }

    impl<K, V> HashMap<K, V>
    where
        K: std::cmp::Eq + std::hash::Hash + Clone,
        V: Clone,
    {
        pub fn new() -> Self {
            HashMap {
                data: StdHashMap::new(),
            }
        }

        pub fn with_capacity(capacity: usize) -> Self {
            HashMap {
                data: StdHashMap::with_capacity(capacity),
            }
        }

        pub fn insert(&mut self, key: K, value: V) -> Option<V> {
            self.data.insert(key, value)
        }

        pub fn get(&self, key: &K) -> Option<&V> {
            self.data.get(key)
        }

        pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
            self.data.get_mut(key)
        }

        pub fn remove(&mut self, key: &K) -> Option<V> {
            self.data.remove(key)
        }

        pub fn contains_key(&self, key: &K) -> bool {
            self.data.contains_key(key)
        }

        pub fn len(&self) -> usize {
            self.data.len()
        }

        pub fn is_empty(&self) -> bool {
            self.data.is_empty()
        }

        pub fn clear(&mut self) {
            self.data.clear();
        }

        pub fn keys(&self) -> std::collections::hash_map::Keys<'_, K, V> {
            self.data.keys()
        }

        pub fn values(&self) -> std::collections::hash_map::Values<'_, K, V> {
            self.data.values()
        }

        pub fn values_mut(&mut self) -> std::collections::hash_map::ValuesMut<'_, K, V> {
            self.data.values_mut()
        }

        pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, V> {
            self.data.iter()
        }

        pub fn entry(&mut self, key: K) -> std::collections::hash_map::Entry<'_, K, V> {
            self.data.entry(key)
        }
    }

    pub struct HashSet<T>
    where
        T: std::cmp::Eq + std::hash::Hash + Clone,
    {
        data: StdHashSet<T>,
    }

    impl<T> HashSet<T>
    where
        T: std::cmp::Eq + std::hash::Hash + Clone,
    {
        pub fn new() -> Self {
            HashSet {
                data: StdHashSet::new(),
            }
        }

        pub fn insert(&mut self, value: T) -> bool {
            self.data.insert(value)
        }

        pub fn remove(&mut self, value: &T) -> bool {
            self.data.remove(value)
        }

        pub fn contains(&self, value: &T) -> bool {
            self.data.contains(value)
        }

        pub fn len(&self) -> usize {
            self.data.len()
        }

        pub fn is_empty(&self) -> bool {
            self.data.is_empty()
        }

        pub fn clear(&mut self) {
            self.data.clear();
        }

        pub fn iter(&self) -> std::collections::hash_set::Iter<'_, T> {
            self.data.iter()
        }

        pub fn union<'a>(
            &'a self,
            other: &'a HashSet<T>,
        ) -> impl Iterator<Item = &'a T> {
            self.data.union(&other.data)
        }

        pub fn intersection<'a>(
            &'a self,
            other: &'a HashSet<T>,
        ) -> impl Iterator<Item = &'a T> {
            self.data.intersection(&other.data)
        }

        pub fn difference<'a>(
            &'a self,
            other: &'a HashSet<T>,
        ) -> impl Iterator<Item = &'a T> {
            self.data.difference(&other.data)
        }
    }
}

pub mod file_io {
    use std::fs::File;
    use std::io::{Read, Write};
    use std::path::Path;

    pub struct FileHandle {
        file: File,
    }

    pub fn file_open(path: &str) -> Result<FileHandle, String> {
        File::open(path)
            .map(|file| FileHandle { file })
            .map_err(|e| e.to_string())
    }

    pub fn file_create(path: &str) -> Result<FileHandle, String> {
        File::create(path)
            .map(|file| FileHandle { file })
            .map_err(|e| e.to_string())
    }

    pub fn file_read(handle: &mut FileHandle) -> Result<String, String> {
        let mut buffer = String::new();
        handle
            .file
            .read_to_string(&mut buffer)
            .map(|_| buffer)
            .map_err(|e| e.to_string())
    }

    pub fn file_write(handle: &mut FileHandle, data: &str) -> Result<(), String> {
        handle
            .file
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())
    }

    pub fn file_exists(path: &str) -> bool {
        Path::new(path).exists()
    }

    pub fn file_delete(path: &str) -> Result<(), String> {
        std::fs::remove_file(path).map_err(|e| e.to_string())
    }

    pub fn dir_create(path: &str) -> Result<(), String> {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())
    }

    pub fn dir_read(path: &str) -> Result<Vec<String>, String> {
        std::fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .map(|entry| {
                entry
                    .map_err(|e| e.to_string())
                    .map(|e| e.file_name().to_string_lossy().to_string())
            })
            .collect()
    }
}

pub mod math {
    pub fn abs_i64(x: i64) -> i64 {
        x.abs()
    }

    pub fn abs_f64(x: f64) -> f64 {
        x.abs()
    }

    pub fn sqrt(x: f64) -> f64 {
        x.sqrt()
    }

    pub fn pow_i64(base: i64, exp: u32) -> i64 {
        base.pow(exp)
    }

    pub fn pow_f64(base: f64, exp: f64) -> f64 {
        base.powf(exp)
    }

    pub fn min_i64(a: i64, b: i64) -> i64 {
        a.min(b)
    }

    pub fn max_i64(a: i64, b: i64) -> i64 {
        a.max(b)
    }

    pub fn min_f64(a: f64, b: f64) -> f64 {
        a.min(b)
    }

    pub fn max_f64(a: f64, b: f64) -> f64 {
        a.max(b)
    }

    pub fn floor(x: f64) -> f64 {
        x.floor()
    }

    pub fn ceil(x: f64) -> f64 {
        x.ceil()
    }

    pub fn round(x: f64) -> f64 {
        x.round()
    }

    pub fn sin(x: f64) -> f64 {
        x.sin()
    }

    pub fn cos(x: f64) -> f64 {
        x.cos()
    }

    pub fn tan(x: f64) -> f64 {
        x.tan()
    }

    pub fn asin(x: f64) -> f64 {
        x.asin()
    }

    pub fn acos(x: f64) -> f64 {
        x.acos()
    }

    pub fn atan(x: f64) -> f64 {
        x.atan()
    }

    pub fn log(x: f64) -> f64 {
        x.ln()
    }

    pub fn log10(x: f64) -> f64 {
        x.log10()
    }

    pub fn exp(x: f64) -> f64 {
        x.exp()
    }

    pub const PI: f64 = std::f64::consts::PI;
    pub const E: f64 = std::f64::consts::E;
}

pub mod conversion {
    pub fn i64_to_string(n: i64) -> String {
        n.to_string()
    }

    pub fn f64_to_string(n: f64) -> String {
        n.to_string()
    }

    pub fn bool_to_string(b: bool) -> String {
        b.to_string()
    }

    pub fn string_to_i64(s: &str) -> Result<i64, String> {
        s.parse::<i64>().map_err(|e| e.to_string())
    }

    pub fn string_to_f64(s: &str) -> Result<f64, String> {
        s.parse::<f64>().map_err(|e| e.to_string())
    }

    pub fn string_to_bool(s: &str) -> Result<bool, String> {
        s.parse::<bool>().map_err(|e| e.to_string())
    }

    pub fn char_to_string(c: char) -> String {
        c.to_string()
    }

    pub fn format_string(format: &str, args: &[&str]) -> String {
        let mut result = format.to_string();
        for arg in args {
            result = result.replacen("{}", arg, 1);
        }
        result
    }
}

pub mod formatting {
    pub fn pad_left(s: &str, width: usize, pad_char: char) -> String {
        if s.len() >= width {
            s.to_string()
        } else {
            let padding = pad_char.to_string().repeat(width - s.len());
            format!("{}{}", padding, s)
        }
    }

    pub fn pad_right(s: &str, width: usize, pad_char: char) -> String {
        if s.len() >= width {
            s.to_string()
        } else {
            let padding = pad_char.to_string().repeat(width - s.len());
            format!("{}{}", s, padding)
        }
    }

    pub fn truncate(s: &str, max_len: usize) -> String {
        if s.len() > max_len {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        } else {
            s.to_string()
        }
    }

    pub fn capitalize(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => {
                first.to_uppercase().collect::<String>() + chars.as_str()
            }
        }
    }
}

pub mod iterators {
    pub fn range(start: i64, end: i64) -> std::ops::Range<i64> {
        start..end
    }

    pub fn range_inclusive(start: i64, end: i64) -> std::ops::RangeInclusive<i64> {
        start..=end
    }

    pub fn repeat<T: Clone>(value: T) -> impl Iterator<Item = T> {
        std::iter::repeat(value)
    }

    pub fn zip<A, B>(
        a: impl IntoIterator<Item = A>,
        b: impl IntoIterator<Item = B>,
    ) -> impl Iterator<Item = (A, B)> {
        a.into_iter().zip(b.into_iter())
    }

    pub fn chain<A, B>(
        a: impl IntoIterator<Item = A>,
        b: impl IntoIterator<Item = A>,
    ) -> impl Iterator<Item = A>
    where
        A: 'static + std::fmt::Debug + Clone,
    {
        a.into_iter().chain(b.into_iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_operations() {
        assert_eq!(string_ops::string_len("hello"), 5);
        assert!(string_ops::string_contains("hello world", "world"));
        assert_eq!(
            string_ops::string_to_uppercase("hello"),
            "HELLO"
        );
    }

    #[test]
    fn test_math_operations() {
        assert_eq!(math::abs_i64(-42), 42);
        assert_eq!(math::max_i64(10, 20), 20);
        assert_eq!(math::min_i64(10, 20), 10);
    }

    #[test]
    fn test_conversions() {
        assert_eq!(conversion::i64_to_string(42), "42");
        assert_eq!(conversion::string_to_i64("42").unwrap(), 42);
    }

    #[test]
    fn test_vector() {
        let mut vec = collections::Vector::new();
        vec.push(1);
        vec.push(2);
        vec.push(3);
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.pop(), Some(3));
        assert_eq!(vec.len(), 2);
    }

    #[test]
    fn test_hashmap() {
        let mut map = collections::HashMap::new();
        map.insert("key1", "value1");
        map.insert("key2", "value2");
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&"key1"), Some(&"value1"));
    }

    #[test]
    fn test_hashset() {
        let mut set = collections::HashSet::new();
        assert!(set.insert(1));
        assert!(!set.insert(1));
        assert!(set.contains(&1));
        assert_eq!(set.len(), 1);
    }
}
