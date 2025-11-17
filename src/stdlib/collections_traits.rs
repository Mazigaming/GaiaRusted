pub trait IteratorExt: std::iter::Iterator + Sized {
    fn count_items(self) -> usize {
        self.count()
    }

    fn filter_map<F, U>(self, f: F) -> std::iter::FilterMap<Self, F>
    where
        F: FnMut(Self::Item) -> Option<U>,
    {
        std::iter::Iterator::filter_map(self, f)
    }
}

impl<I: std::iter::Iterator> IteratorExt for I {}

pub trait GaiaIntoIterator {
    type Item;
    type IntoIter: std::iter::Iterator<Item = Self::Item>;

    fn into_gaia_iter(self) -> Self::IntoIter;
}

pub trait GaiaFromIterator<T>: Sized {
    fn from_gaia_iter<I: std::iter::IntoIterator<Item = T>>(iter: I) -> Self;
}

pub struct VecIterator<T> {
    data: std::vec::Vec<T>,
    index: usize,
}

impl<T> std::iter::Iterator for VecIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.data.len() {
            let item = self.data.remove(self.index);
            Some(item)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.data.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<T> std::iter::ExactSizeIterator for VecIterator<T> {
    fn len(&self) -> usize {
        self.data.len() - self.index
    }
}

pub struct HashMapIterator<K, V> {
    pairs: std::vec::Vec<(K, V)>,
    index: usize,
}

impl<K, V> std::iter::Iterator for HashMapIterator<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.pairs.len() {
            let pair = self.pairs.remove(self.index);
            Some(pair)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.pairs.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<K, V> std::iter::ExactSizeIterator for HashMapIterator<K, V> {
    fn len(&self) -> usize {
        self.pairs.len() - self.index
    }
}

pub mod vec_traits {
    use super::*;
    use crate::stdlib::collections::vec::Vec;

    impl<T: Clone + 'static> GaiaIntoIterator for Vec<T> {
        type Item = T;
        type IntoIter = VecIterator<T>;

        fn into_gaia_iter(self) -> Self::IntoIter {
            let mut data = std::vec::Vec::new();
            for item in self.iter() {
                data.push(item.clone());
            }
            VecIterator { data, index: 0 }
        }
    }

    impl<T: Clone + 'static> GaiaFromIterator<T> for Vec<T> {
        fn from_gaia_iter<I: std::iter::IntoIterator<Item = T>>(iter: I) -> Self {
            let mut vec = Vec::new();
            for item in iter {
                vec.push(item);
            }
            vec
        }
    }
}

pub mod hashmap_traits {
    use super::*;
    use crate::stdlib::collections::hashmap::HashMap;

    impl<K, V> GaiaIntoIterator for HashMap<K, V>
    where
        K: std::cmp::Eq + std::hash::Hash + Clone,
        V: Clone,
    {
        type Item = (K, V);
        type IntoIter = HashMapIterator<K, V>;

        fn into_gaia_iter(self) -> Self::IntoIter {
            let pairs = std::vec::Vec::new();
            HashMapIterator { pairs, index: 0 }
        }
    }

    impl<K, V> GaiaFromIterator<(K, V)> for HashMap<K, V>
    where
        K: std::cmp::Eq + std::hash::Hash + Clone,
        V: Clone,
    {
        fn from_gaia_iter<I: std::iter::IntoIterator<Item = (K, V)>>(iter: I) -> Self {
            let mut map = HashMap::new();
            for (key, value) in iter {
                map.insert(key, value);
            }
            map
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_iterator_basic() {
        let items: std::vec::Vec<i32> = vec![1, 2, 3];
        let iter = items.into_iter();
        assert_eq!(iter.count(), 3);
    }

    #[test]
    fn test_vec_iterator_from_vec() {
        let items: std::vec::Vec<i32> = vec![1, 2, 3, 4, 5];
        let mut it = items.into_iter();
        assert_eq!(it.next(), Some(1));
        assert_eq!(it.next(), Some(2));
        assert_eq!(it.next(), Some(3));
    }

    #[test]
    fn test_iterator_ext_count_items() {
        let items: std::vec::Vec<i32> = vec![1, 2, 3];
        let count = items.into_iter().count_items();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_map_with_standard_iterator() {
        let items: std::vec::Vec<i32> = vec![1, 2, 3, 4, 5];
        let mapped: std::vec::Vec<i32> = items
            .into_iter()
            .map(|x| x * 2)
            .collect();
        assert_eq!(mapped, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_filter_with_standard_iterator() {
        let items: std::vec::Vec<i32> = vec![1, 2, 3, 4, 5];
        let filtered: std::vec::Vec<i32> = items
            .into_iter()
            .filter(|&x| x % 2 == 0)
            .collect();
        assert_eq!(filtered, vec![2, 4]);
    }

    #[test]
    fn test_chained_operations() {
        let items: std::vec::Vec<i32> = vec![1, 2, 3, 4, 5];
        let result: std::vec::Vec<i32> = items
            .into_iter()
            .filter(|&x| x > 2)
            .map(|x| x * 10)
            .collect();
        assert_eq!(result, vec![30, 40, 50]);
    }

    #[test]
    fn test_vector_iteration_empty() {
        let items: std::vec::Vec<i32> = vec![];
        let count = items.into_iter().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_vector_iteration_single_element() {
        let items: std::vec::Vec<i32> = vec![42];
        let mut iter = items.into_iter();
        assert_eq!(iter.next(), Some(42));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_filter_all_match() {
        let items: std::vec::Vec<i32> = vec![2, 4, 6, 8];
        let filtered: std::vec::Vec<i32> = items
            .into_iter()
            .filter(|&x| x % 2 == 0)
            .collect();
        assert_eq!(filtered, vec![2, 4, 6, 8]);
    }

    #[test]
    fn test_filter_none_match() {
        let items: std::vec::Vec<i32> = vec![1, 3, 5, 7];
        let filtered: std::vec::Vec<i32> = items
            .into_iter()
            .filter(|&x| x % 2 == 0)
            .collect();
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_map_strings() {
        let items: std::vec::Vec<i32> = vec![1, 2, 3];
        let strings: std::vec::Vec<String> = items
            .into_iter()
            .map(|x| format!("num_{}", x))
            .collect();
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[0], "num_1");
    }

    #[test]
    fn test_exact_size_iterator_hint() {
        let items: std::vec::Vec<i32> = vec![1, 2, 3, 4, 5];
        let iter = items.into_iter();
        let (lower, upper) = iter.size_hint();
        assert_eq!(lower, 5);
        assert_eq!(upper, Some(5));
    }

    #[test]
    fn test_vec_iterator_size_after_next() {
        let items: std::vec::Vec<i32> = vec![1, 2, 3];
        let mut iter = items.into_iter();
        iter.next();
        let (lower, upper) = iter.size_hint();
        assert_eq!(lower, 2);
        assert_eq!(upper, Some(2));
    }

    #[test]
    fn test_complex_transformation() {
        let items: std::vec::Vec<i32> = vec![1, 2, 3, 4, 5, 6];
        let result: std::vec::Vec<i32> = items
            .into_iter()
            .filter(|&x| x > 1)
            .map(|x| x * x)
            .filter(|&x| x < 30)
            .collect();
        assert_eq!(result, vec![4, 9, 16, 25]);
    }
}
