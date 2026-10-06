//! Optional values.

pub enum Option<T> {
    None,
    Some(T),
}

impl<T> Option<T> {
    pub fn is_some(&self) -> bool {
        match self {
            Option::Some(_) => true,
            Option::None => false,
        }
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    pub fn unwrap(self) -> T {
        match self {
            Option::Some(value) => value,
            Option::None => panic!("called `Option::unwrap()` on a `None` value"),
        }
    }

    pub fn expect(self, message: &str) -> T {
        match self {
            Option::Some(value) => value,
            Option::None => panic!("{}", message),
        }
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Option::Some(value) => value,
            Option::None => default,
        }
    }

    pub fn unwrap_or_else<F: FnOnce() -> T>(self, default: F) -> T {
        match self {
            Option::Some(value) => value,
            Option::None => default(),
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Option<U> {
        match self {
            Option::Some(value) => Option::Some(f(value)),
            Option::None => Option::None,
        }
    }

    pub fn and_then<U, F: FnOnce(T) -> Option<U>>(self, f: F) -> Option<U> {
        match self {
            Option::Some(value) => f(value),
            Option::None => Option::None,
        }
    }

    pub fn ok_or<E>(self, error: E) -> Result<T, E> {
        match self {
            Option::Some(value) => Result::Ok(value),
            Option::None => Result::Err(error),
        }
    }

    pub fn as_ref(&self) -> Option<&T> {
        match self {
            Option::Some(value) => Option::Some(value),
            Option::None => Option::None,
        }
    }

    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            Option::Some(value) => Option::Some(value),
            Option::None => Option::None,
        }
    }

    /// Leave `None` behind and return what was there.
    pub fn take(&mut self) -> Option<T> {
        std::mem::replace(self, Option::None)
    }

    pub fn replace(&mut self, value: T) -> Option<T> {
        std::mem::replace(self, Some(value))
    }

    pub fn insert(&mut self, value: T) -> &mut T {
        *self = Some(value);
        match self {
            Some(inside) => inside,
            None => unreachable!(),
        }
    }

    pub fn get_or_insert(&mut self, value: T) -> &mut T {
        if self.is_none() {
            *self = Some(value);
        }
        match self {
            Some(inside) => inside,
            None => unreachable!(),
        }
    }

    pub fn get_or_insert_with<F: FnOnce() -> T>(&mut self, f: F) -> &mut T {
        if self.is_none() {
            *self = Some(f());
        }
        match self {
            Some(inside) => inside,
            None => unreachable!(),
        }
    }

    pub fn is_some_and<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self {
            Some(value) => f(value),
            None => false,
        }
    }

    pub fn is_none_or<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self {
            Some(value) => f(value),
            None => true,
        }
    }

    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U {
        match self {
            Some(value) => f(value),
            None => default,
        }
    }

    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, f: F) -> U {
        match self {
            Some(value) => f(value),
            None => default(),
        }
    }

    pub fn ok_or_else<E, F: FnOnce() -> E>(self, error: F) -> Result<T, E> {
        match self {
            Some(value) => Ok(value),
            None => Err(error()),
        }
    }

    pub fn filter<P: FnOnce(&T) -> bool>(self, predicate: P) -> Option<T> {
        match self {
            Some(value) => if predicate(&value) { Some(value) } else { None },
            None => None,
        }
    }

    pub fn and<U>(self, other: Option<U>) -> Option<U> {
        match self {
            Some(_) => other,
            None => None,
        }
    }

    pub fn or(self, other: Option<T>) -> Option<T> {
        match self {
            Some(value) => Some(value),
            None => other,
        }
    }

    pub fn or_else<F: FnOnce() -> Option<T>>(self, f: F) -> Option<T> {
        match self {
            Some(value) => Some(value),
            None => f(),
        }
    }

    pub fn xor(self, other: Option<T>) -> Option<T> {
        match (self, other) {
            (Some(value), None) => Some(value),
            (None, Some(value)) => Some(value),
            _ => None,
        }
    }

    pub fn zip<U>(self, other: Option<U>) -> Option<(T, U)> {
        match (self, other) {
            (Some(a), Some(b)) => Some((a, b)),
            _ => None,
        }
    }

    pub fn inspect<F: FnOnce(&T)>(self, f: F) -> Option<T> {
        if let Some(value) = &self {
            f(value);
        }
        self
    }

    pub fn iter(&self) -> IntoIter<&T> {
        IntoIter { inner: self.as_ref() }
    }

    pub fn iter_mut(&mut self) -> IntoIter<&mut T> {
        IntoIter { inner: self.as_mut() }
    }

    pub fn unwrap_or_default(self) -> T {
        match self {
            Some(value) => value,
            None => Default::default(),
        }
    }

    pub unsafe fn unwrap_unchecked(self) -> T {
        self.unwrap()
    }
}

impl<T: Clone> Clone for Option<T> {
    fn clone(&self) -> Option<T> {
        match self {
            Option::Some(value) => Option::Some(value.clone()),
            Option::None => Option::None,
        }
    }
}

impl<T: Copy> Copy for Option<T> {}

impl<T: PartialEq> PartialEq for Option<T> {
    fn eq(&self, other: &Option<T>) -> bool {
        match (self, other) {
            (Option::Some(a), Option::Some(b)) => a == b,
            (Option::None, Option::None) => true,
            _ => false,
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Option<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Option::Some(value) => f.debug_tuple("Some").field(value).finish(),
            Option::None => f.write_str("None"),
        }
    }
}

impl<T> Default for Option<T> {
    fn default() -> Option<T> {
        Option::None
    }
}

impl<T: Copy> Option<&T> {
    pub fn copied(self) -> Option<T> {
        match self {
            Some(value) => Some(*value),
            None => None,
        }
    }
}

impl<T: Clone> Option<&T> {
    pub fn cloned(self) -> Option<T> {
        match self {
            Some(value) => Some(value.clone()),
            None => None,
        }
    }
}

impl<T> Option<Option<T>> {
    pub fn flatten(self) -> Option<T> {
        match self {
            Some(inner) => inner,
            None => None,
        }
    }
}

impl<T: std::ops::Deref> Option<T> {
    /// `Option<String>` to `Option<&str>`, and the like.
    pub fn as_deref(&self) -> Option<&T::Target> {
        match self {
            Some(value) => Some(value.deref()),
            None => None,
        }
    }
}

impl<T, E> Option<Result<T, E>> {
    pub fn transpose(self) -> Result<Option<T>, E> {
        match self {
            Some(Ok(value)) => Ok(Some(value)),
            Some(Err(error)) => Err(error),
            None => Ok(None),
        }
    }
}

/// The value of an option, if it has one, as an iterator.
pub struct IntoIter<T> {
    inner: Option<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.inner.take()
    }
}

impl<T> std::iter::DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.inner.take()
    }
}

impl<T> IntoIterator for Option<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> IntoIter<T> {
        IntoIter { inner: self }
    }
}

impl<T> IntoIterator for &Option<T> {
    type Item = &T;
    type IntoIter = IntoIter<&T>;

    fn into_iter(self) -> IntoIter<&T> {
        IntoIter { inner: self.as_ref() }
    }
}

/// Collecting options: all the values if every one is `Some`, else `None`.
impl<T, C: std::iter::FromIterator<T>> std::iter::FromIterator<Option<T>> for Option<C> {
    fn from_iter<I>(items: I) -> Option<C> {
        let mut items = items;
        let mut values = Vec::new();
        while let Some(item) = items.next() {
            match item {
                Some(value) => values.push(value),
                None => return None,
            }
        }
        Some(values.into_iter().collect())
    }
}

impl<T: Eq> Eq for Option<T> {}

/// `None` comes before every `Some`.
impl<T: PartialOrd> PartialOrd for Option<T> {
    fn partial_cmp(&self, other: &Option<T>) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Some(a), Some(b)) => a.partial_cmp(b),
            (None, None) => Some(std::cmp::Ordering::Equal),
            (None, Some(_)) => Some(std::cmp::Ordering::Less),
            (Some(_), None) => Some(std::cmp::Ordering::Greater),
        }
    }
}

impl<T: Ord> Ord for Option<T> {
    fn cmp(&self, other: &Option<T>) -> std::cmp::Ordering {
        match (self, other) {
            (Some(a), Some(b)) => a.cmp(b),
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
        }
    }
}

impl<T> From<T> for Option<T> {
    fn from(value: T) -> Option<T> {
        Some(value)
    }
}
