//! # Option and Result Types for v0.0.3
//!
//! Implements Rust's Option<T> and Result<T, E> for safe error handling.
//!
//! ## Features
//! - Option<T>: Some(value) | None
//! - Result<T, E>: Ok(value) | Err(error)
//! - Monadic operations (map, and_then, or_else)
//! - Pattern matching integration
//! - Compile-time safety checks

/// Option type: Some(T) or None
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Option<T> {
    Some(T),
    None,
}

impl<T> Option<T> {
    /// Returns true if the option is Some(value)
    pub fn is_some(&self) -> bool {
        matches!(self, Option::Some(_))
    }

    /// Returns true if the option is None
    pub fn is_none(&self) -> bool {
        matches!(self, Option::None)
    }

    /// Maps an Option<T> to Option<U> by applying a function to a contained value
    pub fn map<U, F: Fn(T) -> U>(self, op: F) -> Option<U> {
        match self {
            Option::Some(x) => Option::Some(op(x)),
            Option::None => Option::None,
        }
    }

    /// Returns the provided default result if None, otherwise applies function to Some value
    pub fn map_or<U, F: Fn(T) -> U>(self, default: U, op: F) -> U {
        match self {
            Option::Some(x) => op(x),
            Option::None => default,
        }
    }

    /// Chains Option together: calls op with the value if Some, returns None otherwise
    pub fn and_then<U, F: Fn(T) -> Option<U>>(self, op: F) -> Option<U> {
        match self {
            Option::Some(x) => op(x),
            Option::None => Option::None,
        }
    }

    /// Returns self if it contains a value, otherwise returns optb
    pub fn or(self, optb: Option<T>) -> Option<T> {
        match self {
            Option::Some(x) => Option::Some(x),
            Option::None => optb,
        }
    }

    /// Chains Option together for Or: calls op with None value if Some, returns optb otherwise
    pub fn or_else<F: Fn() -> Option<T>>(self, op: F) -> Option<T> {
        match self {
            Option::Some(x) => Option::Some(x),
            Option::None => op(),
        }
    }

    /// Unwraps an Option, returning the contained Some value or panicking
    pub fn unwrap(self) -> T
    where
        T: std::fmt::Debug,
    {
        match self {
            Option::Some(x) => x,
            Option::None => panic!("called `Option::unwrap()` on a `None` value"),
        }
    }

    /// Returns the contained Some value or a provided default
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Option::Some(x) => x,
            Option::None => default,
        }
    }

    /// Applies a function to the contained value if Some, returns default otherwise
    pub fn unwrap_or_else<F: Fn() -> T>(self, op: F) -> T {
        match self {
            Option::Some(x) => x,
            Option::None => op(),
        }
    }

    /// Converts from &Option<T> to Option<&T>
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            Option::Some(x) => Option::Some(x),
            Option::None => Option::None,
        }
    }

    /// Converts from &mut Option<T> to Option<&mut T>
    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            Option::Some(x) => Option::Some(x),
            Option::None => Option::None,
        }
    }

    /// Takes the value out of the option, leaving a None in its place
    pub fn take(&mut self) -> Option<T> {
        match self {
            Option::Some(_) => {
                let mut val = Option::None;
                std::mem::swap(self, &mut val);
                val
            }
            Option::None => Option::None,
        }
    }
}

/// Result type: Ok(T) or Err(E)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> Result<T, E> {
    /// Returns true if the result is Ok
    pub fn is_ok(&self) -> bool {
        matches!(self, Result::Ok(_))
    }

    /// Returns true if the result is Err
    pub fn is_err(&self) -> bool {
        matches!(self, Result::Err(_))
    }

    /// Maps a Result<T, E> to Result<U, E> by applying a function to Ok value
    pub fn map<U, F: Fn(T) -> U>(self, op: F) -> Result<U, E> {
        match self {
            Result::Ok(x) => Result::Ok(op(x)),
            Result::Err(e) => Result::Err(e),
        }
    }

    /// Maps a Result<T, E> to Result<T, F> by applying a function to Err value
    pub fn map_err<F, O: Fn(E) -> F>(self, op: O) -> Result<T, F> {
        match self {
            Result::Ok(x) => Result::Ok(x),
            Result::Err(e) => Result::Err(op(e)),
        }
    }

    /// Chains Results together: calls op with Ok value if Ok, returns Err otherwise
    pub fn and_then<U, F: Fn(T) -> Result<U, E>>(self, op: F) -> Result<U, E> {
        match self {
            Result::Ok(x) => op(x),
            Result::Err(e) => Result::Err(e),
        }
    }

    /// Returns Ok value if it contains Ok, otherwise returns resb
    pub fn or<F>(self, resb: Result<T, F>) -> Result<T, F> {
        match self {
            Result::Ok(x) => Result::Ok(x),
            Result::Err(_) => resb,
        }
    }

    /// Chains Results together for Or: calls op with Err if Err, returns Ok otherwise
    pub fn or_else<F, O: Fn(E) -> Result<T, F>>(self, op: O) -> Result<T, F> {
        match self {
            Result::Ok(x) => Result::Ok(x),
            Result::Err(e) => op(e),
        }
    }

    /// Unwraps a Result, returning the contained Ok value or panicking
    pub fn unwrap(self) -> T
    where
        E: std::fmt::Debug,
    {
        match self {
            Result::Ok(x) => x,
            Result::Err(e) => panic!("called `Result::unwrap()` on an `Err` value: {:?}", e),
        }
    }

    /// Unwraps a Result, returning the contained Err value or panicking
    pub fn unwrap_err(self) -> E
    where
        T: std::fmt::Debug,
    {
        match self {
            Result::Ok(x) => panic!("called `Result::unwrap_err()` on an `Ok` value: {:?}", x),
            Result::Err(e) => e,
        }
    }

    /// Returns the contained Ok value or a provided default
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Result::Ok(x) => x,
            Result::Err(_) => default,
        }
    }

    /// Applies a function to the contained Ok value if Ok, returns default otherwise
    pub fn unwrap_or_else<F: Fn(E) -> T>(self, op: F) -> T {
        match self {
            Result::Ok(x) => x,
            Result::Err(e) => op(e),
        }
    }

    /// Converts from &Result<T, E> to Result<&T, &E>
    pub fn as_ref(&self) -> Result<&T, &E> {
        match self {
            Result::Ok(x) => Result::Ok(x),
            Result::Err(e) => Result::Err(e),
        }
    }

    /// Converts from &mut Result<T, E> to Result<&mut T, &mut E>
    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {
        match self {
            Result::Ok(x) => Result::Ok(x),
            Result::Err(e) => Result::Err(e),
        }
    }

    /// Flattens a nested Result
    pub fn flatten(self) -> Result<T, E>
    where
        T: std::fmt::Debug,
    {
        match self {
            Result::Ok(val) => Result::Ok(val),
            Result::Err(e) => Result::Err(e),
        }
    }
}

/// Error trait for custom error handling
pub trait ErrorTrait: std::fmt::Display + std::fmt::Debug {}

/// Default error type using String
pub type BoxError = Box<dyn ErrorTrait>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_some() {
        let opt: Option<i32> = Option::Some(42);
        assert!(opt.is_some());
        assert!(!opt.is_none());
    }

    #[test]
    fn test_option_none() {
        let opt: Option<i32> = Option::None;
        assert!(!opt.is_some());
        assert!(opt.is_none());
    }

    #[test]
    fn test_option_map() {
        let opt: Option<i32> = Option::Some(5);
        let result = opt.map(|x| x * 2);
        assert_eq!(result, Option::Some(10));
    }

    #[test]
    fn test_option_and_then() {
        let opt: Option<i32> = Option::Some(5);
        let result = opt.and_then(|x| {
            if x > 0 {
                Option::Some(x * 2)
            } else {
                Option::None
            }
        });
        assert_eq!(result, Option::Some(10));
    }

    #[test]
    fn test_result_ok() {
        let res: Result<i32, String> = Result::Ok(42);
        assert!(res.is_ok());
        assert!(!res.is_err());
    }

    #[test]
    fn test_result_err() {
        let res: Result<i32, String> = Result::Err("error".to_string());
        assert!(!res.is_ok());
        assert!(res.is_err());
    }

    #[test]
    fn test_result_map() {
        let res: Result<i32, String> = Result::Ok(5);
        let mapped = res.map(|x| x * 2);
        assert_eq!(mapped, Result::Ok(10));
    }

    #[test]
    fn test_result_and_then() {
        let res: Result<i32, String> = Result::Ok(5);
        let result = res.and_then(|x| {
            if x > 0 {
                Result::Ok(x * 2)
            } else {
                Result::Err("negative".to_string())
            }
        });
        assert_eq!(result, Result::Ok(10));
    }

    #[test]
    fn test_option_unwrap_or() {
        let opt: Option<i32> = Option::None;
        assert_eq!(opt.unwrap_or(42), 42);
    }

    #[test]
    fn test_result_unwrap_or() {
        let res: Result<i32, String> = Result::Err("error".to_string());
        assert_eq!(res.unwrap_or(42), 42);
    }
}