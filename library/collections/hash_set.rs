//! `HashSet<T>`: a set of values, each stored once, built on `HashMap`.

use std::borrow::Borrow;
use std::collections::hash_map;
use std::collections::hash_map::HashMap;
use std::fmt;
use std::hash::Hash;
use std::iter::{ExactSizeIterator, Extend, FromIterator};

pub struct HashSet<T> {
    map: HashMap<T, ()>,
}

impl<T> HashSet<T> {
    pub fn new() -> HashSet<T> {
        HashSet { map: HashMap::new() }
    }

    pub fn with_capacity(capacity: usize) -> HashSet<T> {
        HashSet { map: HashMap::with_capacity(capacity) }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter { keys: self.map.keys() }
    }

    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain { iter: self.map.drain() }
    }

    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut keep: F) {
        self.map.retain(|value, _| keep(value));
    }
}

impl<T: Hash + Eq> HashSet<T> {
    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional);
    }

    /// Add a value; `true` if it was not already present.
    pub fn insert(&mut self, value: T) -> bool {
        match self.map.entry(value) {
            hash_map::Entry::Occupied(_) => false,
            hash_map::Entry::Vacant(entry) => {
                entry.insert(());
                true
            }
        }
    }

    /// Add a value, replacing an equal one; returns the value replaced.
    pub fn replace(&mut self, value: T) -> Option<T> {
        let old = self.map.remove_entry(&value);
        self.map.insert(value, ());
        match old {
            Some((old, _)) => Some(old),
            None => None,
        }
    }

    pub fn contains<Q: Hash + Eq + ?Sized>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
    {
        self.map.contains_key(value)
    }

    /// The stored value equal to `value`.
    pub fn get<Q: Hash + Eq + ?Sized>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
    {
        match self.map.get_key_value(value) {
            Some((stored, _)) => Some(stored),
            None => None,
        }
    }

    pub fn remove<Q: Hash + Eq + ?Sized>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
    {
        self.map.remove(value).is_some()
    }

    /// Remove the stored value equal to `value` and hand it back.
    pub fn take<Q: Hash + Eq + ?Sized>(&mut self, value: &Q) -> Option<T>
    where
        T: Borrow<Q>,
    {
        match self.map.remove_entry(value) {
            Some((stored, _)) => Some(stored),
            None => None,
        }
    }

    /// The values in either set.
    pub fn union<'a>(&'a self, other: &'a HashSet<T>) -> Union<'a, T> {
        Union { first: self.iter(), rest: other.difference(self) }
    }

    /// The values in both sets.
    pub fn intersection<'a>(&'a self, other: &'a HashSet<T>) -> Intersection<'a, T> {
        Intersection { iter: self.iter(), other }
    }

    /// The values in this set but not in `other`.
    pub fn difference<'a>(&'a self, other: &'a HashSet<T>) -> Difference<'a, T> {
        Difference { iter: self.iter(), other }
    }

    /// The values in exactly one of the sets.
    pub fn symmetric_difference<'a>(&'a self, other: &'a HashSet<T>) -> SymmetricDifference<'a, T> {
        SymmetricDifference { first: self.difference(other), second: other.difference(self) }
    }

    pub fn is_subset(&self, other: &HashSet<T>) -> bool {
        self.len() <= other.len() && self.iter().all(|value| other.contains(value))
    }

    pub fn is_superset(&self, other: &HashSet<T>) -> bool {
        other.is_subset(self)
    }

    pub fn is_disjoint(&self, other: &HashSet<T>) -> bool {
        self.iter().all(|value| !other.contains(value))
    }
}

impl<T> Default for HashSet<T> {
    fn default() -> HashSet<T> {
        HashSet::new()
    }
}

impl<T: Clone> Clone for HashSet<T> {
    fn clone(&self) -> HashSet<T> {
        HashSet { map: self.map.clone() }
    }
}

impl<T: Hash + Eq> PartialEq for HashSet<T> {
    fn eq(&self, other: &HashSet<T>) -> bool {
        self.len() == other.len() && self.is_subset(other)
    }
}

impl<T: Hash + Eq> Eq for HashSet<T> {}

impl<T: fmt::Debug> fmt::Debug for HashSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut set = f.debug_set();
        for value in self.iter() {
            set.entry(value);
        }
        set.finish()
    }
}

impl<T: Hash + Eq> FromIterator<T> for HashSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(values: I) -> HashSet<T> {
        let mut set = HashSet::new();
        set.extend(values);
        set
    }
}

impl<T: Hash + Eq> Extend<T> for HashSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, values: I) {
        for value in values {
            self.insert(value);
        }
    }

    fn extend_one(&mut self, value: T) {
        self.insert(value);
    }
}

impl<T: Hash + Eq + Copy> Extend<&T> for HashSet<T> {
    fn extend<I: IntoIterator<Item = &T>>(&mut self, values: I) {
        for value in values {
            self.insert(*value);
        }
    }

    fn extend_one(&mut self, value: &T) {
        self.insert(*value);
    }
}

/// Defines `&a op &b` for sets as a new set built from one of the
/// set operations.
macro_rules! set_operator {
    ($trait:ident, $method:ident, $operation:ident) => {
        impl<T: Hash + Eq + Clone> std::ops::$trait<&HashSet<T>> for &HashSet<T> {
            type Output = HashSet<T>;

            fn $method(self, other: &HashSet<T>) -> HashSet<T> {
                self.$operation(other).cloned().collect()
            }
        }
    };
}

set_operator!(BitOr, bitor, union);
set_operator!(BitAnd, bitand, intersection);
set_operator!(BitXor, bitxor, symmetric_difference);
set_operator!(Sub, sub, difference);

pub struct Iter<'a, T> {
    keys: hash_map::Keys<'a, T, ()>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        self.keys.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.keys.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.keys.len()
    }
}

pub struct IntoIter<T> {
    keys: hash_map::IntoKeys<T, ()>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.keys.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.keys.size_hint()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.keys.len()
    }
}

pub struct Drain<'a, T> {
    iter: hash_map::Drain<'a, T, ()>,
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self.iter.next() {
            Some((value, _)) => Some(value),
            None => None,
        }
    }
}

pub struct Difference<'a, T> {
    iter: Iter<'a, T>,
    other: &'a HashSet<T>,
}

impl<'a, T: Hash + Eq> Iterator for Difference<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        loop {
            let value = self.iter.next()?;
            if !self.other.contains(value) {
                return Some(value);
            }
        }
    }
}

pub struct Intersection<'a, T> {
    iter: Iter<'a, T>,
    other: &'a HashSet<T>,
}

impl<'a, T: Hash + Eq> Iterator for Intersection<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        loop {
            let value = self.iter.next()?;
            if self.other.contains(value) {
                return Some(value);
            }
        }
    }
}

pub struct Union<'a, T> {
    first: Iter<'a, T>,
    rest: Difference<'a, T>,
}

impl<'a, T: Hash + Eq> Iterator for Union<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match self.first.next() {
            Some(value) => Some(value),
            None => self.rest.next(),
        }
    }
}

pub struct SymmetricDifference<'a, T> {
    first: Difference<'a, T>,
    second: Difference<'a, T>,
}

impl<'a, T: Hash + Eq> Iterator for SymmetricDifference<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match self.first.next() {
            Some(value) => Some(value),
            None => self.second.next(),
        }
    }
}

impl<T> IntoIterator for HashSet<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> IntoIter<T> {
        IntoIter { keys: self.map.into_keys() }
    }
}

impl<'a, T> IntoIterator for &'a HashSet<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<T: Hash + Eq, const N: usize> From<[T; N]> for HashSet<T> {
    fn from(values: [T; N]) -> HashSet<T> {
        let mut set = HashSet::with_capacity(N);
        set.extend(values);
        set
    }
}
