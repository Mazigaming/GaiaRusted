//! Success or failure.

pub enum Result<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> Result<T, E> {
    pub fn is_ok(&self) -> bool {
        match self {
            Result::Ok(_) => true,
            Result::Err(_) => false,
        }
    }

    pub fn is_err(&self) -> bool {
        !self.is_ok()
    }

    pub fn ok(self) -> Option<T> {
        match self {
            Result::Ok(value) => Option::Some(value),
            Result::Err(_) => Option::None,
        }
    }

    pub fn err(self) -> Option<E> {
        match self {
            Result::Ok(_) => Option::None,
            Result::Err(error) => Option::Some(error),
        }
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Result::Ok(value) => value,
            Result::Err(_) => default,
        }
    }

    pub fn unwrap_or_else<F: FnOnce(E) -> T>(self, default: F) -> T {
        match self {
            Result::Ok(value) => value,
            Result::Err(error) => default(error),
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Result<U, E> {
        match self {
            Result::Ok(value) => Result::Ok(f(value)),
            Result::Err(error) => Result::Err(error),
        }
    }

    pub fn map_err<G, F: FnOnce(E) -> G>(self, f: F) -> Result<T, G> {
        match self {
            Result::Ok(value) => Result::Ok(value),
            Result::Err(error) => Result::Err(f(error)),
        }
    }

    pub fn and_then<U, F: FnOnce(T) -> Result<U, E>>(self, f: F) -> Result<U, E> {
        match self {
            Result::Ok(value) => f(value),
            Result::Err(error) => Result::Err(error),
        }
    }
}

impl<T, E: std::fmt::Debug> Result<T, E> {
    pub fn is_ok_and<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self {
            Ok(value) => f(value),
            Err(_) => false,
        }
    }

    pub fn is_err_and<F: FnOnce(E) -> bool>(self, f: F) -> bool {
        match self {
            Ok(_) => false,
            Err(error) => f(error),
        }
    }

    pub fn as_ref(&self) -> Result<&T, &E> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error),
        }
    }

    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error),
        }
    }

    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U {
        match self {
            Ok(value) => f(value),
            Err(_) => default,
        }
    }

    pub fn map_or_else<U, D: FnOnce(E) -> U, F: FnOnce(T) -> U>(self, default: D, f: F) -> U {
        match self {
            Ok(value) => f(value),
            Err(error) => default(error),
        }
    }

    pub fn and<U>(self, other: Result<U, E>) -> Result<U, E> {
        match self {
            Ok(_) => other,
            Err(error) => Err(error),
        }
    }

    pub fn or<G>(self, other: Result<T, G>) -> Result<T, G> {
        match self {
            Ok(value) => Ok(value),
            Err(_) => other,
        }
    }

    pub fn or_else<G, F: FnOnce(E) -> Result<T, G>>(self, f: F) -> Result<T, G> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => f(error),
        }
    }

    pub fn inspect<F: FnOnce(&T)>(self, f: F) -> Result<T, E> {
        if let Ok(value) = &self {
            f(value);
        }
        self
    }

    pub fn inspect_err<F: FnOnce(&E)>(self, f: F) -> Result<T, E> {
        if let Err(error) = &self {
            f(error);
        }
        self
    }

    pub fn unwrap_or_default(self) -> T {
        match self {
            Ok(value) => value,
            Err(_) => Default::default(),
        }
    }

    pub fn iter(&self) -> std::option::IntoIter<&T> {
        self.as_ref().ok().into_iter()
    }

    pub fn expect_err(self, message: &str) -> E {
        match self {
            Ok(_) => panic!("{}", message),
            Err(error) => error,
        }
    }

    pub fn unwrap_err(self) -> E {
        match self {
            Ok(_) => panic!("called `Result::unwrap_err()` on an `Ok` value"),
            Err(error) => error,
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            Result::Ok(value) => value,
            Result::Err(error) => panic!("called `Result::unwrap()` on an `Err` value: {:?}", error),
        }
    }

    pub fn expect(self, message: &str) -> T {
        match self {
            Result::Ok(value) => value,
            Result::Err(error) => panic!("{}: {:?}", message, error),
        }
    }
}

impl<T: std::fmt::Debug, E: std::fmt::Debug> std::fmt::Debug for Result<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Result::Ok(value) => f.debug_tuple("Ok").field(value).finish(),
            Result::Err(error) => f.debug_tuple("Err").field(error).finish(),
        }
    }
}

impl<T: Copy, E> Result<&T, E> {
    pub fn copied(self) -> Result<T, E> {
        self.map(|value| *value)
    }
}

impl<T: Clone, E> Result<&T, E> {
    pub fn cloned(self) -> Result<T, E> {
        self.map(|value| value.clone())
    }
}

impl<T, E> Result<Option<T>, E> {
    pub fn transpose(self) -> Option<Result<T, E>> {
        match self {
            Ok(Some(value)) => Some(Ok(value)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        }
    }
}

/// Collecting results: all the values, or the first error.
impl<T, E, C: std::iter::FromIterator<T>> std::iter::FromIterator<Result<T, E>> for Result<C, E> {
    fn from_iter<I>(items: I) -> Result<C, E> {
        let mut items = items;
        let mut values = Vec::new();
        while let Some(item) = items.next() {
            match item {
                Ok(value) => values.push(value),
                Err(error) => return Err(error),
            }
        }
        Ok(values.into_iter().collect())
    }
}

impl<T: Clone, E: Clone> Clone for Result<T, E> {
    fn clone(&self) -> Result<T, E> {
        match self {
            Ok(value) => Ok(value.clone()),
            Err(error) => Err(error.clone()),
        }
    }
}

impl<T: Copy, E: Copy> Copy for Result<T, E> {}

impl<T: PartialEq, E: PartialEq> PartialEq for Result<T, E> {
    fn eq(&self, other: &Result<T, E>) -> bool {
        match (self, other) {
            (Ok(a), Ok(b)) => a == b,
            (Err(a), Err(b)) => a == b,
            _ => false,
        }
    }
}

impl<T: Eq, E: Eq> Eq for Result<T, E> {}

/// Every `Ok` comes before every `Err`.
impl<T: PartialOrd, E: PartialOrd> PartialOrd for Result<T, E> {
    fn partial_cmp(&self, other: &Result<T, E>) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Ok(a), Ok(b)) => a.partial_cmp(b),
            (Err(a), Err(b)) => a.partial_cmp(b),
            (Ok(_), Err(_)) => Some(std::cmp::Ordering::Less),
            (Err(_), Ok(_)) => Some(std::cmp::Ordering::Greater),
        }
    }
}

impl<T: Ord, E: Ord> Ord for Result<T, E> {
    fn cmp(&self, other: &Result<T, E>) -> std::cmp::Ordering {
        match (self, other) {
            (Ok(a), Ok(b)) => a.cmp(b),
            (Err(a), Err(b)) => a.cmp(b),
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        }
    }
}

impl<T: std::hash::Hash, E: std::hash::Hash> std::hash::Hash for Result<T, E> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Ok(value) => {
                state.write_isize(0);
                value.hash(state);
            }
            Err(error) => {
                state.write_isize(1);
                error.hash(state);
            }
        }
    }
}
