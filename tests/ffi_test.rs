//! Phase 6 Week 10: FFI & extern Declarations Integration Tests
//!
//! Tests for C interoperability including:
//! - extern "C" function declarations
//! - C type mappings
//! - Function signature validation
//! - Variadic function support
//! - Struct layout matching

use gaiarusted::ffi::{
    FfiValidator, FfiFunctionSignature, FfiStructDef, CType, ABI,
};

#[test]
fn test_c_type_int_mapping() {
    let c_int = CType::CInt;
    assert_eq!(c_int.to_string(), "c_int");
}

#[test]
fn test_c_type_float_mapping() {
    let c_float = CType::CFloat;
    assert_eq!(c_float.to_string(), "c_float");
}

#[test]
fn test_c_type_void_mapping() {
    let c_void = CType::CVoid;
    assert_eq!(c_void.to_string(), "c_void");
}

#[test]
fn test_ffi_validator_initialization() {
    let validator = FfiValidator::new();
    assert_eq!(validator.errors().len(), 0);
    
    // Should have all standard C types
    assert!(validator.is_valid_c_type(CType::CInt));
    assert!(validator.is_valid_c_type(CType::CDouble));
    assert!(validator.is_valid_c_type(CType::CVoid));
}

#[test]
fn test_extern_c_function_declaration() {
    let mut validator = FfiValidator::new();
    
    let printf_sig = FfiFunctionSignature::new(
        "printf".to_string(),
        ABI::C,
        vec![("format".to_string(), CType::CInt)],
        CType::CInt,
        true, // variadic
    );
    
    assert!(validator.register_extern_function(printf_sig).is_ok());
    assert!(validator.has_function("printf"));
}

#[test]
fn test_simple_c_function_signature() {
    let mut validator = FfiValidator::new();
    
    // extern "C" { fn strlen(s: *const c_char) -> usize; }
    let strlen_sig = FfiFunctionSignature::new(
        "strlen".to_string(),
        ABI::C,
        vec![("s".to_string(), CType::CInt)], // simplified to CInt for test
        CType::CLong,
        false,
    );
    
    assert!(validator.register_extern_function(strlen_sig).is_ok());
    
    // Call with correct params
    let params = vec![("ptr".to_string(), CType::CInt)];
    assert!(validator.validate_function_params("strlen", &params).is_ok());
}

#[test]
fn test_c_function_wrong_param_count_fails() {
    let mut validator = FfiValidator::new();
    
    let add_sig = FfiFunctionSignature::new(
        "add".to_string(),
        ABI::C,
        vec![
            ("a".to_string(), CType::CInt),
            ("b".to_string(), CType::CInt),
        ],
        CType::CInt,
        false,
    );
    
    assert!(validator.register_extern_function(add_sig).is_ok());
    
    // Try calling with wrong number of args
    let params = vec![("x".to_string(), CType::CInt)];
    
    assert!(validator.validate_function_params("add", &params).is_err());
}

#[test]
fn test_c_function_wrong_param_type_fails() {
    let mut validator = FfiValidator::new();
    
    let add_sig = FfiFunctionSignature::new(
        "add".to_string(),
        ABI::C,
        vec![
            ("a".to_string(), CType::CInt),
            ("b".to_string(), CType::CInt),
        ],
        CType::CInt,
        false,
    );
    
    assert!(validator.register_extern_function(add_sig).is_ok());
    
    // Try calling with wrong type
    let params = vec![
        ("x".to_string(), CType::CInt),
        ("y".to_string(), CType::CDouble), // Wrong!
    ];
    
    assert!(validator.validate_function_params("add", &params).is_err());
}

#[test]
fn test_variadic_printf_declaration() {
    let mut validator = FfiValidator::new();
    
    let printf_sig = FfiFunctionSignature::new(
        "printf".to_string(),
        ABI::C,
        vec![("format".to_string(), CType::CInt)],
        CType::CInt,
        true, // variadic!
    );
    
    assert!(validator.register_extern_function(printf_sig).is_ok());
    
    // Can call with extra args due to variadic
    let params = vec![
        ("format".to_string(), CType::CInt),
        ("arg1".to_string(), CType::CInt),
        ("arg2".to_string(), CType::CInt),
        ("arg3".to_string(), CType::CDouble),
    ];
    
    assert!(validator.validate_function_params("printf", &params).is_ok());
}

#[test]
fn test_return_type_validation_success() {
    let mut validator = FfiValidator::new();
    
    let get_pid_sig = FfiFunctionSignature::new(
        "getpid".to_string(),
        ABI::C,
        vec![],
        CType::CInt,
        false,
    );
    
    assert!(validator.register_extern_function(get_pid_sig).is_ok());
    
    // Validate correct return type
    assert!(validator.validate_return_type("getpid", CType::CInt).is_ok());
}

#[test]
fn test_return_type_validation_failure() {
    let mut validator = FfiValidator::new();
    
    let get_pid_sig = FfiFunctionSignature::new(
        "getpid".to_string(),
        ABI::C,
        vec![],
        CType::CInt,
        false,
    );
    
    assert!(validator.register_extern_function(get_pid_sig).is_ok());
    
    // Validate wrong return type
    assert!(validator.validate_return_type("getpid", CType::CDouble).is_err());
}

#[test]
fn test_struct_with_c_fields() {
    let mut validator = FfiValidator::new();
    
    // struct Point { x: c_int, y: c_int }
    let point_def = FfiStructDef::new(
        "Point".to_string(),
        vec![
            ("x".to_string(), CType::CInt),
            ("y".to_string(), CType::CInt),
        ],
        true,
    );
    
    assert!(validator.register_struct(point_def).is_ok());
}

#[test]
fn test_struct_with_multiple_field_types() {
    let mut validator = FfiValidator::new();
    
    // Mimicking a complex C struct
    let person_def = FfiStructDef::new(
        "Person".to_string(),
        vec![
            ("age".to_string(), CType::CInt),
            ("height".to_string(), CType::CDouble),
            ("name_len".to_string(), CType::CLong),
        ],
        true,
    );
    
    assert!(validator.register_struct(person_def).is_ok());
}

#[test]
fn test_abi_c_required_for_extern() {
    let mut validator = FfiValidator::new();
    
    // Trying to register with non-C ABI should fail
    let bad_sig = FfiFunctionSignature::new(
        "bad_func".to_string(),
        ABI::Rust, // Wrong ABI!
        vec![],
        CType::CVoid,
        false,
    );
    
    assert!(validator.register_extern_function(bad_sig).is_err());
    assert_eq!(validator.errors().len(), 1);
}

#[test]
fn test_multiple_extern_functions() {
    let mut validator = FfiValidator::new();
    
    let funcs = vec![
        FfiFunctionSignature::new(
            "malloc".to_string(),
            ABI::C,
            vec![("size".to_string(), CType::CLong)],
            CType::CInt, // Simplified pointer as CInt
            false,
        ),
        FfiFunctionSignature::new(
            "free".to_string(),
            ABI::C,
            vec![("ptr".to_string(), CType::CInt)],
            CType::CVoid,
            false,
        ),
        FfiFunctionSignature::new(
            "strlen".to_string(),
            ABI::C,
            vec![("s".to_string(), CType::CInt)],
            CType::CLong,
            false,
        ),
    ];
    
    for sig in funcs {
        assert!(validator.register_extern_function(sig).is_ok());
    }
    
    assert!(validator.has_function("malloc"));
    assert!(validator.has_function("free"));
    assert!(validator.has_function("strlen"));
}

#[test]
fn test_c_integer_type_family() {
    // Test all C integer types
    let int_types = vec![
        CType::CInt,
        CType::CUint,
        CType::CLong,
        CType::CUlong,
        CType::CLongLong,
        CType::CUlongLong,
        CType::CShort,
        CType::CUshort,
        CType::CChar,
        CType::CUchar,
        CType::CSignedChar,
    ];
    
    let validator = FfiValidator::new();
    
    for c_type in int_types {
        assert!(validator.is_valid_c_type(c_type));
    }
}

#[test]
fn test_c_float_type_family() {
    // Test all C float types
    let float_types = vec![CType::CFloat, CType::CDouble];
    
    let validator = FfiValidator::new();
    
    for c_type in float_types {
        assert!(validator.is_valid_c_type(c_type));
    }
}

#[test]
fn test_error_accumulation() {
    let mut validator = FfiValidator::new();
    
    let sig1 = FfiFunctionSignature::new(
        "func1".to_string(),
        ABI::C,
        vec![("a".to_string(), CType::CInt)],
        CType::CInt,
        false,
    );
    validator.register_extern_function(sig1).unwrap();
    
    // Try calling with wrong params
    let _ = validator.validate_function_params("func1", &vec![]);
    assert_eq!(validator.errors().len(), 1);
    
    // Try returning wrong type
    let _ = validator.validate_return_type("func1", CType::CDouble);
    assert_eq!(validator.errors().len(), 2);
}

#[test]
fn test_unknown_function_error() {
    let mut validator = FfiValidator::new();
    
    let result = validator.validate_function_params("unknown_func", &vec![]);
    assert!(result.is_err());
    
    if let Err(e) = result {
        assert!(e.message.contains("unknown"));
    }
}

#[test]
fn test_capability_summary() {
    let mut validator = FfiValidator::new();
    
    // Register standard C library functions
    let functions = vec![
        FfiFunctionSignature::new(
            "malloc".to_string(),
            ABI::C,
            vec![("size".to_string(), CType::CLong)],
            CType::CInt,
            false,
        ),
        FfiFunctionSignature::new(
            "free".to_string(),
            ABI::C,
            vec![("ptr".to_string(), CType::CInt)],
            CType::CVoid,
            false,
        ),
        FfiFunctionSignature::new(
            "memcpy".to_string(),
            ABI::C,
            vec![
                ("dest".to_string(), CType::CInt),
                ("src".to_string(), CType::CInt),
                ("n".to_string(), CType::CLong),
            ],
            CType::CInt,
            false,
        ),
    ];
    
    for func in functions {
        assert!(validator.register_extern_function(func).is_ok());
    }
    
    // Register struct
    let point = FfiStructDef::new(
        "Point".to_string(),
        vec![
            ("x".to_string(), CType::CInt),
            ("y".to_string(), CType::CInt),
        ],
        true,
    );
    assert!(validator.register_struct(point).is_ok());
    
    // Verify all registered
    assert!(validator.has_function("malloc"));
    assert!(validator.has_function("free"));
    assert!(validator.has_function("memcpy"));
    
    // Verify function signatures
    let malloc_func = validator.get_function("malloc").unwrap();
    assert_eq!(malloc_func.abi, ABI::C);
    assert_eq!(malloc_func.params.len(), 1);
}
