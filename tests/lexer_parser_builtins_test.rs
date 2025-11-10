//! Tests for built-in functions

#[cfg(test)]
mod builtin_tests {
    use gaiarusted::builtins::BuiltinFunction;

    #[test]
    fn test_builtin_recognition() {
        assert_eq!(BuiltinFunction::from_name("abs"), Some(BuiltinFunction::Abs));
        assert_eq!(BuiltinFunction::from_name("min"), Some(BuiltinFunction::Min));
        assert_eq!(BuiltinFunction::from_name("max"), Some(BuiltinFunction::Max));
        assert_eq!(BuiltinFunction::from_name("pow"), Some(BuiltinFunction::Pow));
        assert_eq!(BuiltinFunction::from_name("sqrt"), Some(BuiltinFunction::Sqrt));
        assert_eq!(BuiltinFunction::from_name("len"), Some(BuiltinFunction::Len));
        assert_eq!(BuiltinFunction::from_name("println"), Some(BuiltinFunction::Println));
        assert_eq!(BuiltinFunction::from_name("unknown_func"), None);
    }

    #[test]
    fn test_builtin_arg_count() {
        assert_eq!(BuiltinFunction::Abs.arg_count(), Some(1));
        assert_eq!(BuiltinFunction::Min.arg_count(), Some(2));
        assert_eq!(BuiltinFunction::Max.arg_count(), Some(2));
        assert_eq!(BuiltinFunction::Pow.arg_count(), Some(2));
        assert_eq!(BuiltinFunction::Sqrt.arg_count(), Some(1));
    }

    #[test]
    fn test_builtin_display() {
        assert_eq!(BuiltinFunction::Abs.to_string(), "abs");
        assert_eq!(BuiltinFunction::Min.to_string(), "min");
        assert_eq!(BuiltinFunction::Sqrt.to_string(), "sqrt");
    }

    #[test]
    fn test_builtin_descriptions() {
        let desc = BuiltinFunction::Abs.description();
        assert!(!desc.is_empty());
        assert_eq!(desc, "Absolute value");
    }

    #[test]
    fn test_all_builtins() {
        let all = BuiltinFunction::all();
        assert!(all.len() > 0);
        assert!(all.contains(&BuiltinFunction::Abs));
        assert!(all.contains(&BuiltinFunction::Println));
    }

    #[test]
    fn test_stdlib_stubs() {
        let stubs = gaiarusted::builtins::generate_stdlib_stubs();
        assert!(stubs.contains("_builtin_abs"));
        assert!(stubs.contains("_builtin_min"));
        assert!(stubs.contains("_builtin_max"));
    }
}