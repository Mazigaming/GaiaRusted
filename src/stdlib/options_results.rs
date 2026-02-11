//! # Option<T> and Result<T, E> Methods
//!
//! Extends the Option and Result types with useful methods for composition and transformation.

/// Methods available on Option<T>
pub struct OptionMethods;

impl OptionMethods {
    /// Check if Option contains a value
    pub fn is_some<T>(opt: &Option<T>) -> bool {
        opt.is_some()
    }

    /// Check if Option is None
    pub fn is_none<T>(opt: &Option<T>) -> bool {
        opt.is_none()
    }

    /// Map the value inside Some(T) to Some(U)
    pub fn map<T, U, F: FnOnce(T) -> U>(opt: Option<T>, f: F) -> Option<U> {
        match opt {
            Some(x) => Some(f(x)),
            None => None,
        }
    }

    /// Unwrap the value or return a default
    pub fn unwrap_or<T: Clone>(opt: Option<T>, default: T) -> T {
        match opt {
            Some(x) => x,
            None => default,
        }
    }

    /// Unwrap the value or compute a default
    pub fn unwrap_or_else<T, F: FnOnce() -> T>(opt: Option<T>, f: F) -> T {
        match opt {
            Some(x) => x,
            None => f(),
        }
    }

    /// Chain operations on Option values
    pub fn and_then<T, U, F: FnOnce(T) -> Option<U>>(opt: Option<T>, f: F) -> Option<U> {
        match opt {
            Some(x) => f(x),
            None => None,
        }
    }

    /// Try to get a value, unwrap with panic
    pub fn unwrap<T: std::fmt::Debug>(opt: Option<T>) -> T {
        match opt {
            Some(x) => x,
            None => panic!("Called unwrap on None"),
        }
    }

    /// Filter based on predicate
    pub fn filter<T: Clone, F: FnOnce(&T) -> bool>(opt: Option<T>, predicate: F) -> Option<T> {
        match opt {
            Some(x) if predicate(&x) => Some(x),
            _ => None,
        }
    }

    /// Convert Some(T) to Ok(T), None to Err(E)
    pub fn ok_or<T, E>(opt: Option<T>, err: E) -> Result<T, E> {
        match opt {
            Some(x) => Ok(x),
            None => Err(err),
        }
    }
}

/// Methods available on Result<T, E>
pub struct ResultMethods;

impl ResultMethods {
    /// Check if Result is Ok
    pub fn is_ok<T, E>(res: &Result<T, E>) -> bool {
        res.is_ok()
    }

    /// Check if Result is Err
    pub fn is_err<T, E>(res: &Result<T, E>) -> bool {
        res.is_err()
    }

    /// Map the value inside Ok(T) to Ok(U)
    pub fn map<T, U, E, F: FnOnce(T) -> U>(res: Result<T, E>, f: F) -> Result<U, E> {
        match res {
            Ok(x) => Ok(f(x)),
            Err(e) => Err(e),
        }
    }

    /// Map the error inside Err(E) to Err(F)
    pub fn map_err<T, E, F, G: FnOnce(E) -> F>(res: Result<T, E>, f: G) -> Result<T, F> {
        match res {
            Ok(x) => Ok(x),
            Err(e) => Err(f(e)),
        }
    }

    /// Unwrap the value or return a default
    pub fn unwrap_or<T: Clone, E>(res: Result<T, E>, default: T) -> T {
        match res {
            Ok(x) => x,
            Err(_) => default,
        }
    }

    /// Unwrap the value or compute a default
    pub fn unwrap_or_else<T, E, F: FnOnce(E) -> T>(res: Result<T, E>, f: F) -> T {
        match res {
            Ok(x) => x,
            Err(e) => f(e),
        }
    }

    /// Chain operations on Result values
    pub fn and_then<T, U, E, F: FnOnce(T) -> Result<U, E>>(
        res: Result<T, E>,
        f: F,
    ) -> Result<U, E> {
        match res {
            Ok(x) => f(x),
            Err(e) => Err(e),
        }
    }

    /// Try to get a value, unwrap with panic
    pub fn unwrap<T: std::fmt::Debug, E: std::fmt::Debug>(res: Result<T, E>) -> T {
        match res {
            Ok(x) => x,
            Err(e) => panic!("Called unwrap on Err: {:?}", e),
        }
    }

    /// Get the error or panic
    pub fn unwrap_err<T: std::fmt::Debug, E: std::fmt::Debug>(res: Result<T, E>) -> E {
        match res {
            Ok(x) => panic!("Called unwrap_err on Ok: {:?}", x),
            Err(e) => e,
        }
    }

    /// Convert Ok(T) to Some(T), Err(E) to None
    pub fn ok<T, E>(res: Result<T, E>) -> Option<T> {
        match res {
            Ok(x) => Some(x),
            Err(_) => None,
        }
    }

    /// Convert Err(E) to Some(E), Ok(T) to None
    pub fn err<T, E>(res: Result<T, E>) -> Option<E> {
        match res {
            Ok(_) => None,
            Err(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_is_some() {
        assert!(OptionMethods::is_some(&Some(5)));
        assert!(!OptionMethods::is_some(&(None::<i32>)));
    }

    #[test]
    fn test_option_is_none() {
        assert!(!OptionMethods::is_none(&Some(5)));
        assert!(OptionMethods::is_none(&(None::<i32>)));
    }

    #[test]
    fn test_option_map() {
        let opt = Some(5);
        let mapped = OptionMethods::map(opt, |x| x * 2);
        assert_eq!(mapped, Some(10));
    }

    #[test]
    fn test_option_map_none() {
        let opt: Option<i32> = None;
        let mapped = OptionMethods::map(opt, |x| x * 2);
        assert_eq!(mapped, None);
    }

    #[test]
    fn test_option_unwrap_or() {
        assert_eq!(OptionMethods::unwrap_or(Some(5), 0), 5);
        assert_eq!(OptionMethods::unwrap_or(None::<i32>, 0), 0);
    }

    #[test]
    fn test_option_unwrap_or_else() {
        assert_eq!(OptionMethods::unwrap_or_else(Some(5), || 0), 5);
        assert_eq!(OptionMethods::unwrap_or_else(None::<i32>, || 10), 10);
    }

    #[test]
    fn test_option_and_then() {
        let opt = Some(5);
        let result = OptionMethods::and_then(opt, |x| {
            if x > 0 {
                Some(x * 2)
            } else {
                None
            }
        });
        assert_eq!(result, Some(10));
    }

    #[test]
    fn test_option_filter() {
        let opt = Some(5);
        let filtered = OptionMethods::filter(opt.clone(), |x| x > &3);
        assert_eq!(filtered, Some(5));

        let filtered2 = OptionMethods::filter(opt, |x| x > &10);
        assert_eq!(filtered2, None);
    }

    #[test]
    fn test_option_ok_or() {
        let opt: Option<i32> = Some(5);
        let res = OptionMethods::ok_or(opt, "error");
        assert_eq!(res, Ok(5));

        let opt_none: Option<i32> = None;
        let res_err = OptionMethods::ok_or(opt_none, "error");
        assert_eq!(res_err, Err("error"));
    }

    #[test]
    fn test_result_is_ok() {
        assert!(ResultMethods::is_ok::<i32, String>(&Ok(5)));
        assert!(!ResultMethods::is_ok::<i32, String>(&Err("error".to_string())));
    }

    #[test]
    fn test_result_is_err() {
        assert!(!ResultMethods::is_err::<i32, String>(&Ok(5)));
        assert!(ResultMethods::is_err::<i32, String>(&Err("error".to_string())));
    }

    #[test]
    fn test_result_map() {
        let res: Result<i32, String> = Ok(5);
        let mapped = ResultMethods::map(res, |x| x * 2);
        assert_eq!(mapped, Ok(10));
    }

    #[test]
    fn test_result_map_err() {
        let res: Result<i32, String> = Err("error".to_string());
        let mapped = ResultMethods::map_err(res, |_| "new_error".to_string());
        assert_eq!(mapped, Err("new_error".to_string()));
    }

    #[test]
    fn test_result_unwrap_or() {
        assert_eq!(ResultMethods::unwrap_or(Ok::<i32, String>(5), 0), 5);
        assert_eq!(
            ResultMethods::unwrap_or(Err::<i32, String>("error".to_string()), 0),
            0
        );
    }

    #[test]
    fn test_result_and_then() {
        let res: Result<i32, String> = Ok(5);
        let result = ResultMethods::and_then(res, |x| {
            if x > 0 {
                Ok(x * 2)
            } else {
                Err("negative".to_string())
            }
        });
        assert_eq!(result, Ok(10));
    }

    #[test]
    fn test_result_ok() {
        let res: Result<i32, String> = Ok(5);
        assert_eq!(ResultMethods::ok(res), Some(5));

        let res_err: Result<i32, String> = Err("error".to_string());
        assert_eq!(ResultMethods::ok(res_err), None);
    }

    #[test]
    fn test_result_err() {
        let res: Result<i32, String> = Ok(5);
        assert_eq!(ResultMethods::err(res), None);

        let res_err: Result<i32, String> = Err("error".to_string());
        assert_eq!(ResultMethods::err(res_err), Some("error".to_string()));
    }
}
