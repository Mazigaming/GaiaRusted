//! `BTreeSet<T>`: an ordered set, built on `BTreeMap`.

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::btree_map;
use std::collections::btree_map::BTreeMap;
use std::fmt;
use std::iter::{DoubleEndedIterator, ExactSizeIterator, Extend, FromIterator};
use std::ops::RangeBounds;

pub struct BTreeSet<T> {
    map: BTreeMap<T, ()>,
}

impl<T> BTreeSet<T> {
    pub fn new() -> BTreeSet<T> {
        BTreeSet { map: BTreeMap::new() }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// The values in ascending order.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { keys: self.map.keys() }
    }

    pub fn first(&self) -> Option<&T> {
        match self.map.first_key_value() {
            Some((value, _)) => Some(value),
            None => None,
        }
    }

    pub fn last(&self) -> Option<&T> {
        match self.map.last_key_value() {
            Some((value, _)) => Some(value),
            None => None,
        }
    }

    pub fn pop_first(&mut self) -> Option<T> {
        match self.map.pop_first() {
            Some((value, _)) => Some(value),
            None => None,
        }
    }

    pub fn pop_last(&mut self) -> Option<T> {
        match self.map.pop_last() {
            Some((value, _)) => Some(value),
            None => None,
        }
    }

    pub fn contains<Q: Ord + ?Sized>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
    {
        self.map.contains_key(value)
    }

    /// The stored value equal to `value`.
    pub fn get<Q: Ord + ?Sized>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
    {
        match self.map.get_key_value(value) {
            Some((stored, _)) => Some(stored),
            None => None,
        }
    }

    pub fn remove<Q: Ord + ?Sized>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
    {
        self.map.remove(value).is_some()
    }

    /// Remove the stored value equal to `value` and hand it back.
    pub fn take<Q: Ord + ?Sized>(&mut self, value: &Q) -> Option<T>
    where
        T: Borrow<Q>,
    {
        match self.map.remove_entry(value) {
            Some((stored, _)) => Some(stored),
            None => None,
        }
    }

    /// The values that fall in `range`, in ascending order.
    pub fn range<Q: Ord + ?Sized, R: RangeBounds<Q>>(&self, range: R) -> Range<'_, T>
    where
        T: Borrow<Q>,
    {
        Range { iter: self.map.range(range) }
    }
}

impl<T: Ord> BTreeSet<T> {
    /// Add a value; `true` if it was not already present.
    pub fn insert(&mut self, value: T) -> bool {
        match self.map.entry(value) {
            btree_map::Entry::Occupied(_) => false,
            btree_map::Entry::Vacant(entry) => {
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

    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut keep: F) {
        self.map.retain(|value, _| keep(value));
    }

    pub fn append(&mut self, other: &mut BTreeSet<T>) {
        self.map.append(&mut other.map);
    }

    /// The values from `value` on, as a new set; this one keeps the rest.
    pub fn split_off<Q: Ord + ?Sized>(&mut self, value: &Q) -> BTreeSet<T>
    where
        T: Borrow<Q>,
    {
        BTreeSet { map: self.map.split_off(value) }
    }

    /// The values in either set, in ascending order.
    pub fn union<'a>(&'a self, other: &'a BTreeSet<T>) -> Union<'a, T> {
        Union { a: Peeked::new(self.iter()), b: Peeked::new(other.iter()) }
    }

    /// The values in both sets, in ascending order.
    pub fn intersection<'a>(&'a self, other: &'a BTreeSet<T>) -> Intersection<'a, T> {
        Intersection { a: self.iter(), b: Peeked::new(other.iter()) }
    }

    /// The values in this set but not in `other`, in ascending order.
    pub fn difference<'a>(&'a self, other: &'a BTreeSet<T>) -> Difference<'a, T> {
        Difference { a: self.iter(), b: Peeked::new(other.iter()) }
    }

    /// The values in exactly one of the sets, in ascending order.
    pub fn symmetric_difference<'a>(&'a self, other: &'a BTreeSet<T>) -> SymmetricDifference<'a, T> {
        SymmetricDifference { a: Peeked::new(self.iter()), b: Peeked::new(other.iter()) }
    }

    pub fn is_subset(&self, other: &BTreeSet<T>) -> bool {
        self.len() <= other.len() && self.difference(other).next().is_none()
    }

    pub fn is_superset(&self, other: &BTreeSet<T>) -> bool {
        other.is_subset(self)
    }

    pub fn is_disjoint(&self, other: &BTreeSet<T>) -> bool {
        self.intersection(other).next().is_none()
    }
}

impl<T> Default for BTreeSet<T> {
    fn default() -> BTreeSet<T> {
        BTreeSet::new()
    }
}

impl<T: Clone> Clone for BTreeSet<T> {
    fn clone(&self) -> BTreeSet<T> {
        BTreeSet { map: self.map.clone() }
    }
}

impl<T: PartialEq> PartialEq for BTreeSet<T> {
    fn eq(&self, other: &BTreeSet<T>) -> bool {
        self.map == other.map
    }
}

impl<T: Eq> Eq for BTreeSet<T> {}

impl<T: fmt::Debug> fmt::Debug for BTreeSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut set = f.debug_set();
        for value in self.iter() {
            set.entry(value);
        }
        set.finish()
    }
}

impl<T: Ord> FromIterator<T> for BTreeSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(values: I) -> BTreeSet<T> {
        let mut set = BTreeSet::new();
        set.extend(values);
        set
    }
}

impl<T: Ord> Extend<T> for BTreeSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, values: I) {
        for value in values {
            self.insert(value);
        }
    }

    fn extend_one(&mut self, value: T) {
        self.insert(value);
    }
}

impl<T: Ord + Copy> Extend<&T> for BTreeSet<T> {
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
        impl<T: Ord + Clone> std::ops::$trait<&BTreeSet<T>> for &BTreeSet<T> {
            type Output = BTreeSet<T>;

            fn $method(self, other: &BTreeSet<T>) -> BTreeSet<T> {
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
    keys: btree_map::Keys<'a, T, ()>,
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

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        self.keys.next_back()
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.keys.len()
    }
}

pub struct IntoIter<T> {
    keys: btree_map::IntoKeys<T, ()>,
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

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.keys.next_back()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.keys.len()
    }
}

pub struct Range<'a, T> {
    iter: btree_map::Range<'a, T, ()>,
}

impl<'a, T> Iterator for Range<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match self.iter.next() {
            Some((value, _)) => Some(value),
            None => None,
        }
    }
}

impl<'a, T> DoubleEndedIterator for Range<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        match self.iter.next_back() {
            Some((value, _)) => Some(value),
            None => None,
        }
    }
}

/// A set's values in order, with the next one already taken out so that
/// two sets can be walked side by side.
struct Peeked<'a, T> {
    iter: Iter<'a, T>,
    next: Option<&'a T>,
}

impl<'a, T: Ord> Peeked<'a, T> {
    fn new(iter: Iter<'a, T>) -> Peeked<'a, T> {
        let mut iter = iter;
        let next = iter.next();
        Peeked { iter, next }
    }

    fn advance(&mut self) -> Option<&'a T> {
        let current = self.next;
        self.next = self.iter.next();
        current
    }

    /// Skip the values less than `value`; whether `value` itself is next.
    fn skip_to(&mut self, value: &T) -> bool {
        while let Some(next) = self.next {
            match next.cmp(value) {
                Ordering::Less => self.next = self.iter.next(),
                Ordering::Equal => return true,
                Ordering::Greater => return false,
            }
        }
        false
    }
}

pub struct Difference<'a, T> {
    a: Iter<'a, T>,
    b: Peeked<'a, T>,
}

impl<'a, T: Ord> Iterator for Difference<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        loop {
            let value = self.a.next()?;
            if !self.b.skip_to(value) {
                return Some(value);
            }
        }
    }
}

pub struct Intersection<'a, T> {
    a: Iter<'a, T>,
    b: Peeked<'a, T>,
}

impl<'a, T: Ord> Iterator for Intersection<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        loop {
            let value = self.a.next()?;
            if self.b.skip_to(value) {
                return Some(value);
            }
        }
    }
}

pub struct Union<'a, T> {
    a: Peeked<'a, T>,
    b: Peeked<'a, T>,
}

impl<'a, T: Ord> Iterator for Union<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match (self.a.next, self.b.next) {
            (Some(a), Some(b)) => match a.cmp(b) {
                Ordering::Less => self.a.advance(),
                Ordering::Greater => self.b.advance(),
                Ordering::Equal => {
                    self.b.advance();
                    self.a.advance()
                }
            },
            (Some(_), None) => self.a.advance(),
            (None, _) => self.b.advance(),
        }
    }
}

pub struct SymmetricDifference<'a, T> {
    a: Peeked<'a, T>,
    b: Peeked<'a, T>,
}

impl<'a, T: Ord> Iterator for SymmetricDifference<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        loop {
            match (self.a.next, self.b.next) {
                (Some(a), Some(b)) => match a.cmp(b) {
                    Ordering::Less => return self.a.advance(),
                    Ordering::Greater => return self.b.advance(),
                    Ordering::Equal => {
                        self.a.advance();
                        self.b.advance();
                    }
                },
                (Some(_), None) => return self.a.advance(),
                (None, _) => return self.b.advance(),
            }
        }
    }
}

impl<T> IntoIterator for BTreeSet<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> IntoIter<T> {
        IntoIter { keys: self.map.into_keys() }
    }
}

impl<'a, T> IntoIterator for &'a BTreeSet<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<T: Ord, const N: usize> From<[T; N]> for BTreeSet<T> {
    fn from(values: [T; N]) -> BTreeSet<T> {
        values.into_iter().collect()
    }
}
