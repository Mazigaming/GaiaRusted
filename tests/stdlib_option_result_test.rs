//! Integration tests for Option and Result types (v0.0.3)

#[cfg(test)]
mod option_result_tests {
    use gaiarusted::option_result::{Option, Result};

    #[test]
    fn test_option_creation() {
        let some_val: Option<i32> = Option::Some(42);
        let none_val: Option<i32> = Option::None;

        assert!(some_val.is_some());
        assert!(!some_val.is_none());

        assert!(!none_val.is_some());
        assert!(none_val.is_none());
    }

    #[test]
    fn test_option_map_chain() {
        let val: Option<i32> = Option::Some(5);
        let result = val
            .map(|x| x * 2)
            .map(|x| x + 3);

        assert_eq!(result, Option::Some(13));
    }

    #[test]
    fn test_option_and_then() {
        let val: Option<i32> = Option::Some(10);
        let result = val.and_then(|x| {
            if x > 5 {
                Option::Some(x * 2)
            } else {
                Option::None
            }
        });

        assert_eq!(result, Option::Some(20));
    }

    #[test]
    fn test_option_or() {
        let none_val: Option<i32> = Option::None;
        let default = Option::Some(42);

        assert_eq!(none_val.or(default), Option::Some(42));
    }

    #[test]
    fn test_option_unwrap_or() {
        let some_val: Option<i32> = Option::Some(10);
        let none_val: Option<i32> = Option::None;

        assert_eq!(some_val.unwrap_or(0), 10);
        assert_eq!(none_val.unwrap_or(0), 0);
    }

    #[test]
    fn test_result_creation() {
        let ok_val: Result<i32, String> = Result::Ok(42);
        let err_val: Result<i32, String> = Result::Err("error".to_string());

        assert!(ok_val.is_ok());
        assert!(!ok_val.is_err());

        assert!(!err_val.is_ok());
        assert!(err_val.is_err());
    }

    #[test]
    fn test_result_map() {
        let val: Result<i32, String> = Result::Ok(5);
        let result = val.map(|x| x * 2);

        assert_eq!(result, Result::Ok(10));
    }

    #[test]
    fn test_result_map_err() {
        let val: Result<i32, String> = Result::Err("error".to_string());
        let result = val.map_err(|e| format!("Error: {}", e));

        assert_eq!(result, Result::Err("Error: error".to_string()));
    }

    #[test]
    fn test_result_and_then() {
        let val: Result<i32, String> = Result::Ok(10);
        let result = val.and_then(|x| {
            if x > 5 {
                Result::Ok(x * 2)
            } else {
                Result::Err("too small".to_string())
            }
        });

        assert_eq!(result, Result::Ok(20));
    }

    #[test]
    fn test_result_unwrap_or() {
        let ok_val: Result<i32, String> = Result::Ok(42);
        let err_val: Result<i32, String> = Result::Err("error".to_string());

        assert_eq!(ok_val.unwrap_or(0), 42);
        assert_eq!(err_val.unwrap_or(0), 0);
    }

    #[test]
    fn test_option_as_ref() {
        let opt: Option<i32> = Option::Some(42);
        let opt_ref = opt.as_ref();

        assert_eq!(opt_ref, Option::Some(&42));
    }

    #[test]
    fn test_result_as_ref() {
        let res: Result<i32, String> = Result::Ok(42);
        let res_ref = res.as_ref();

        assert_eq!(res_ref, Result::Ok(&42));
    }
}