pub mod prelude {
    pub use crate::stdlib::collections::vec::Vec;
    pub use crate::stdlib::option::Option;
    pub use crate::stdlib::result::Result;
    pub use crate::stdlib::string::String;
}

pub mod collections {
    pub mod hashset {
        use std::collections::HashSet as StdHashSet;

        #[derive(Clone, Debug)]
        pub struct HashSet<T: std::cmp::Eq + std::hash::Hash> {
            data: StdHashSet<T>,
        }

        impl<T: std::cmp::Eq + std::hash::Hash + Clone> HashSet<T> {
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

            pub fn iter(&self) -> impl Iterator<Item = &T> {
                self.data.iter()
            }
        }

        impl<T: std::cmp::Eq + std::hash::Hash + Clone> Default for HashSet<T> {
            fn default() -> Self {
                Self::new()
            }
        }
    }

    pub mod btreemap {
        use std::collections::BTreeMap as StdBTreeMap;

        #[derive(Clone, Debug)]
        pub struct BTreeMap<K: Ord, V> {
            data: StdBTreeMap<K, V>,
        }

        impl<K: Ord + Clone, V: Clone> BTreeMap<K, V> {
            pub fn new() -> Self {
                BTreeMap {
                    data: StdBTreeMap::new(),
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

            pub fn keys(&self) -> impl Iterator<Item = &K> {
                self.data.keys()
            }

            pub fn values(&self) -> impl Iterator<Item = &V> {
                self.data.values()
            }

            pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
                self.data.iter()
            }

            pub fn range(&self, start: K, end: K) -> Vec<(&K, &V)> {
                self.data
                    .range(start..=end)
                    .map(|(k, v)| (k, v))
                    .collect()
            }
        }

        impl<K: Ord + Clone, V: Clone> Default for BTreeMap<K, V> {
            fn default() -> Self {
                Self::new()
            }
        }
    }

    pub mod vec {
        #[derive(Clone, Debug)]
        pub struct Vec<T> {
            data: std::vec::Vec<T>,
        }

        impl<T: Clone> Vec<T> {
            pub fn new() -> Self {
                Vec {
                    data: std::vec::Vec::new(),
                }
            }

            pub fn with_capacity(capacity: usize) -> Self {
                Vec {
                    data: std::vec::Vec::with_capacity(capacity),
                }
            }

            pub fn push(&mut self, value: T) {
                self.data.push(value);
            }

            pub fn pop(&mut self) -> Option<T> {
                self.data.pop()
            }

            pub fn len(&self) -> usize {
                self.data.len()
            }

            pub fn is_empty(&self) -> bool {
                self.data.is_empty()
            }

            pub fn get(&self, index: usize) -> Option<&T> {
                self.data.get(index)
            }

            pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
                self.data.get_mut(index)
            }

            pub fn clear(&mut self) {
                self.data.clear();
            }

            pub fn iter(&self) -> std::slice::Iter<'_, T> {
                self.data.iter()
            }
        }

        impl<T: Clone> Default for Vec<T> {
            fn default() -> Self {
                Self::new()
            }
        }
    }

    pub mod hashmap {
        use std::collections::HashMap as StdHashMap;

        #[derive(Clone, Debug)]
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
        }

        impl<K, V> Default for HashMap<K, V>
        where
            K: std::cmp::Eq + std::hash::Hash + Clone,
            V: Clone,
        {
            fn default() -> Self {
                Self::new()
            }
        }
    }
}

pub mod option {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Option<T> {
        Some(T),
        None,
    }

    impl<T> Option<T> {
        pub fn is_some(&self) -> bool {
            matches!(self, Option::Some(_))
        }

        pub fn is_none(&self) -> bool {
            matches!(self, Option::None)
        }

        pub fn unwrap(self) -> T {
            match self {
                Option::Some(x) => x,
                Option::None => panic!("called Option::unwrap() on a None value"),
            }
        }

        pub fn unwrap_or(self, default: T) -> T {
            match self {
                Option::Some(x) => x,
                Option::None => default,
            }
        }

        pub fn map<U, F>(self, f: F) -> Option<U>
        where
            F: FnOnce(T) -> U,
        {
            match self {
                Option::Some(x) => Option::Some(f(x)),
                Option::None => Option::None,
            }
        }

        pub fn and_then<U, F>(self, f: F) -> Option<U>
        where
            F: FnOnce(T) -> Option<U>,
        {
            match self {
                Option::Some(x) => f(x),
                Option::None => Option::None,
            }
        }

        pub fn or(self, optb: Option<T>) -> Option<T> {
            match self {
                Option::Some(x) => Option::Some(x),
                Option::None => optb,
            }
        }
    }
}

pub mod result {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Result<T, E> {
        Ok(T),
        Err(E),
    }

    impl<T, E> Result<T, E> {
        pub fn is_ok(&self) -> bool {
            matches!(self, Result::Ok(_))
        }

        pub fn is_err(&self) -> bool {
            matches!(self, Result::Err(_))
        }

        pub fn unwrap(self) -> T {
            match self {
                Result::Ok(x) => x,
                Result::Err(_) => panic!("called Result::unwrap() on an Err value"),
            }
        }

        pub fn unwrap_or(self, default: T) -> T {
            match self {
                Result::Ok(x) => x,
                Result::Err(_) => default,
            }
        }

        pub fn map<U, F>(self, f: F) -> Result<U, E>
        where
            F: FnOnce(T) -> U,
        {
            match self {
                Result::Ok(x) => Result::Ok(f(x)),
                Result::Err(e) => Result::Err(e),
            }
        }

        pub fn and_then<U, F>(self, f: F) -> Result<U, E>
        where
            F: FnOnce(T) -> Result<U, E>,
        {
            match self {
                Result::Ok(x) => f(x),
                Result::Err(e) => Result::Err(e),
            }
        }

        pub fn or<F>(self, resb: Result<T, F>) -> Result<T, F> {
            match self {
                Result::Ok(x) => Result::Ok(x),
                Result::Err(_) => resb,
            }
        }

        pub fn map_err<F, O>(self, f: F) -> Result<T, O>
        where
            F: FnOnce(E) -> O,
        {
            match self {
                Result::Ok(x) => Result::Ok(x),
                Result::Err(e) => Result::Err(f(e)),
            }
        }
    }
}

pub mod string {
    use std::fmt;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct String {
        data: std::string::String,
    }

    impl String {
        pub fn new() -> Self {
            String {
                data: std::string::String::new(),
            }
        }

        pub fn from(s: &str) -> Self {
            String {
                data: std::string::String::from(s),
            }
        }

        pub fn with_capacity(capacity: usize) -> Self {
            String {
                data: std::string::String::with_capacity(capacity),
            }
        }

        pub fn push(&mut self, ch: char) {
            self.data.push(ch);
        }

        pub fn push_str(&mut self, s: &str) {
            self.data.push_str(s);
        }

        pub fn len(&self) -> usize {
            self.data.len()
        }

        pub fn is_empty(&self) -> bool {
            self.data.is_empty()
        }

        pub fn as_str(&self) -> &str {
            &self.data
        }

        pub fn as_bytes(&self) -> &[u8] {
            self.data.as_bytes()
        }

        pub fn to_uppercase(&self) -> Self {
            String {
                data: self.data.to_uppercase(),
            }
        }

        pub fn to_lowercase(&self) -> Self {
            String {
                data: self.data.to_lowercase(),
            }
        }

        pub fn trim(&self) -> &str {
            self.data.trim()
        }

        pub fn trim_start(&self) -> &str {
            self.data.trim_start()
        }

        pub fn trim_end(&self) -> &str {
            self.data.trim_end()
        }

        pub fn replace(&self, from: &str, to: &str) -> Self {
            String {
                data: self.data.replace(from, to),
            }
        }

        pub fn split(&self, delim: char) -> std::vec::Vec<&str> {
            self.data.split(delim).collect()
        }

        pub fn contains(&self, pat: &str) -> bool {
            self.data.contains(pat)
        }

        pub fn starts_with(&self, prefix: &str) -> bool {
            self.data.starts_with(prefix)
        }

        pub fn ends_with(&self, suffix: &str) -> bool {
            self.data.ends_with(suffix)
        }

        pub fn find(&self, pat: &str) -> Option<usize> {
            self.data.find(pat)
        }

        pub fn repeat(&self, n: usize) -> Self {
            let mut result = String::new();
            for _ in 0..n {
                result.push_str(self.as_str());
            }
            result
        }

        pub fn chars_vec(&self) -> std::vec::Vec<char> {
            self.data.chars().collect()
        }

        pub fn lines_vec(&self) -> std::vec::Vec<&str> {
            self.data.lines().collect()
        }

        pub fn clear(&mut self) {
            self.data.clear();
        }

        pub fn capacity(&self) -> usize {
            self.data.capacity()
        }

        pub fn reserve(&mut self, additional: usize) {
            self.data.reserve(additional);
        }

        pub fn split_whitespace(&self) -> std::vec::Vec<&str> {
            self.data.split_whitespace().collect()
        }

        pub fn strip_prefix(&self, prefix: &str) -> Option<&str> {
            self.data.strip_prefix(prefix)
        }

        pub fn strip_suffix(&self, suffix: &str) -> Option<&str> {
            self.data.strip_suffix(suffix)
        }

        pub fn remove(&mut self, idx: usize) -> char {
            self.data.remove(idx)
        }

        pub fn insert(&mut self, idx: usize, ch: char) {
            self.data.insert(idx, ch)
        }

        pub fn truncate(&mut self, new_len: usize) {
            self.data.truncate(new_len)
        }

        pub fn split_once(&self, delim: &str) -> Option<(&str, &str)> {
            self.data.split_once(delim)
        }

        pub fn rsplit_once(&self, delim: &str) -> Option<(&str, &str)> {
            self.data.rsplit_once(delim)
        }

        pub fn to_string(&self) -> Self {
            String {
                data: self.data.clone(),
            }
        }

        pub fn into_bytes(self) -> std::vec::Vec<u8> {
            self.data.into_bytes()
        }

        pub fn is_numeric(&self) -> bool {
            !self.data.is_empty() && self.data.chars().all(|c| c.is_numeric())
        }

        pub fn is_alphabetic(&self) -> bool {
            !self.data.is_empty() && self.data.chars().all(|c| c.is_alphabetic())
        }
    }

    impl Default for String {
        fn default() -> Self {
            Self::new()
        }
    }

    impl fmt::Display for String {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.data)
        }
    }



    impl From<&str> for String {
        fn from(s: &str) -> Self {
            String::from(s)
        }
    }

    impl AsRef<str> for String {
        fn as_ref(&self) -> &str {
            self.as_str()
        }
    }
}

pub mod io {
    pub fn println(s: &str) {
        println!("{}", s);
    }

    pub fn print(s: &str) {
        print!("{}", s);
    }

    pub fn eprintln(s: &str) {
        eprintln!("{}", s);
    }

    pub struct Reader {
        inner: std::io::Stdin,
    }

    impl Reader {
        pub fn new() -> Self {
            Reader {
                inner: std::io::stdin(),
            }
        }

        pub fn read_line(&mut self) -> std::io::Result<String> {
            let mut buf = std::string::String::new();
            self.inner.read_line(&mut buf)?;
            Ok(buf)
        }
    }

    pub fn read_file(path: &str) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    pub fn write_file(path: &str, contents: &str) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }
}

pub mod iterators {
    pub trait Iterator {
        type Item;

        fn next(&mut self) -> Option<Self::Item>;

        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, None)
        }

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

        fn map<U, F>(self, f: F) -> Map<Self, F>
        where
            F: FnMut(Self::Item) -> U,
            Self: Sized,
        {
            Map { iter: self, f }
        }

        fn filter<F>(self, predicate: F) -> Filter<Self, F>
        where
            F: FnMut(&Self::Item) -> bool,
            Self: Sized,
        {
            Filter {
                iter: self,
                predicate,
            }
        }

        fn take(self, n: usize) -> Take<Self>
        where
            Self: Sized,
        {
            Take {
                iter: self,
                n,
                taken: 0,
            }
        }

        fn skip(self, n: usize) -> Skip<Self>
        where
            Self: Sized,
        {
            Skip {
                iter: self,
                n,
                skipped: 0,
            }
        }

        fn find<P>(&mut self, mut predicate: P) -> Option<Self::Item>
        where
            P: FnMut(&Self::Item) -> bool,
        {
            while let Some(item) = self.next() {
                if predicate(&item) {
                    return Some(item);
                }
            }
            None
        }

        fn position<P>(&mut self, mut predicate: P) -> Option<usize>
        where
            P: FnMut(Self::Item) -> bool,
        {
            let mut pos = 0;
            while let Some(item) = self.next() {
                if predicate(item) {
                    return Some(pos);
                }
                pos += 1;
            }
            None
        }

        fn fold<B, F>(mut self, init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
            Self: Sized,
        {
            let mut accum = init;
            while let Some(item) = self.next() {
                accum = f(accum, item);
            }
            accum
        }

        fn any<F>(&mut self, mut f: F) -> bool
        where
            F: FnMut(Self::Item) -> bool,
        {
            while let Some(item) = self.next() {
                if f(item) {
                    return true;
                }
            }
            false
        }

        fn all<F>(&mut self, mut f: F) -> bool
        where
            F: FnMut(Self::Item) -> bool,
        {
            while let Some(item) = self.next() {
                if !f(item) {
                    return false;
                }
            }
            true
        }
    }

    pub struct Map<I, F> {
        iter: I,
        f: F,
    }

    pub struct Filter<I, F> {
        iter: I,
        predicate: F,
    }

    pub struct Take<I> {
        iter: I,
        n: usize,
        taken: usize,
    }

    pub struct Skip<I> {
        iter: I,
        n: usize,
        skipped: usize,
    }
}

pub mod smart_pointers {
    use std::rc::Rc as StdRc;
    use std::sync::Arc as StdArc;
    use std::sync::Mutex as StdMutex;

    pub struct Box<T> {
        data: std::boxed::Box<T>,
    }

    impl<T> Box<T> {
        pub fn new(value: T) -> Self {
            Box {
                data: std::boxed::Box::new(value),
            }
        }

        pub fn deref(&self) -> &T {
            &self.data
        }

        pub fn deref_mut(&mut self) -> &mut T {
            &mut self.data
        }

        pub fn into_inner(self) -> T {
            *self.data
        }
    }

    impl<T: Clone> Clone for Box<T> {
        fn clone(&self) -> Self {
            Box::new((*self.data).clone())
        }
    }

    pub struct Rc<T> {
        data: StdRc<T>,
    }

    impl<T> Rc<T> {
        pub fn new(value: T) -> Self {
            Rc {
                data: StdRc::new(value),
            }
        }

        pub fn deref(&self) -> &T {
            &self.data
        }

        pub fn strong_count(&self) -> usize {
            StdRc::strong_count(&self.data)
        }

        pub fn weak_count(&self) -> usize {
            StdRc::weak_count(&self.data)
        }
    }

    impl<T: Clone> Clone for Rc<T> {
        fn clone(&self) -> Self {
            Rc {
                data: self.data.clone(),
            }
        }
    }

    pub struct Arc<T> {
        data: StdArc<T>,
    }

    impl<T> Arc<T> {
        pub fn new(value: T) -> Self {
            Arc {
                data: StdArc::new(value),
            }
        }

        pub fn deref(&self) -> &T {
            &self.data
        }

        pub fn strong_count(&self) -> usize {
            StdArc::strong_count(&self.data)
        }

        pub fn weak_count(&self) -> usize {
            StdArc::weak_count(&self.data)
        }
    }

    impl<T> Clone for Arc<T> {
        fn clone(&self) -> Self {
            Arc {
                data: self.data.clone(),
            }
        }
    }

    pub struct Mutex<T> {
        data: StdMutex<T>,
    }

    impl<T> Mutex<T> {
        pub fn new(value: T) -> Self {
            Mutex {
                data: StdMutex::new(value),
            }
        }

        pub fn lock(&self) -> Result<std::sync::MutexGuard<T>, String> {
            self.data.lock().map_err(|e| e.to_string())
        }

        pub fn unlock(&self) -> Result<(), String> {
            Ok(())
        }

        pub fn is_poisoned(&self) -> bool {
            self.data.is_poisoned()
        }
    }

    impl<T: Clone> Clone for Mutex<T> {
        fn clone(&self) -> Self {
            if let Ok(guard) = self.data.lock() {
                Mutex::new((*guard).clone())
            } else {
                Mutex::new(std::panic::panic_any("Mutex poisoned"))
            }
        }
    }
}
