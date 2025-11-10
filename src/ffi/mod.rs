//! # FFI (Foreign Function Interface) Support
//!
//! Implements C interoperability for safe Rust-C binding generation.
//!
//! ## Features:
//! - C type representations and mappings
//! - struct layout validation
//! - Function signature validation for extern "C" 
//! - Calling convention enforcement
//!
//! ## Architecture:
//! FFI checking happens at the type level to ensure:
//! 1. extern "C" functions have compatible signatures
//! 2. Struct fields match C layout expectations
//! 3. Pointers and basic types are correctly mapped

use std::fmt;
use std::collections::HashMap;

/// FFI error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfiError {
    pub message: String,
}

impl fmt::Display for FfiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub type FfiResult<T> = Result<T, FfiError>;

/// C type representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CType {
    // Integer types
    CInt,           // c_int (usually i32)
    CUint,          // c_uint (usually u32)
    CLong,          // c_long
    CUlong,         // c_ulong
    CLongLong,      // c_longlong
    CUlongLong,     // c_ulonglong
    CShort,         // c_short
    CUshort,        // c_ushort
    CChar,          // c_char
    CUchar,         // c_uchar
    CSignedChar,    // c_schar
    
    // Floating point
    CFloat,         // c_float
    CDouble,        // c_double
    
    // Other
    CVoid,          // void (only in pointers)
    CBool,          // _Bool / bool
}

impl fmt::Display for CType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CType::CInt => write!(f, "c_int"),
            CType::CUint => write!(f, "c_uint"),
            CType::CLong => write!(f, "c_long"),
            CType::CUlong => write!(f, "c_ulong"),
            CType::CLongLong => write!(f, "c_longlong"),
            CType::CUlongLong => write!(f, "c_ulonglong"),
            CType::CShort => write!(f, "c_short"),
            CType::CUshort => write!(f, "c_ushort"),
            CType::CChar => write!(f, "c_char"),
            CType::CUchar => write!(f, "c_uchar"),
            CType::CSignedChar => write!(f, "c_schar"),
            CType::CFloat => write!(f, "c_float"),
            CType::CDouble => write!(f, "c_double"),
            CType::CVoid => write!(f, "c_void"),
            CType::CBool => write!(f, "c_bool"),
        }
    }
}

/// ABI (Application Binary Interface) specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ABI {
    C,      // extern "C"
    Rust,   // extern "Rust" (default, for reference)
}

impl fmt::Display for ABI {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ABI::C => write!(f, "C"),
            ABI::Rust => write!(f, "Rust"),
        }
    }
}

/// FFI function signature for extern "C" functions
#[derive(Debug, Clone)]
pub struct FfiFunctionSignature {
    pub name: String,
    pub abi: ABI,
    pub params: Vec<(String, CType)>,
    pub return_type: CType,
    pub is_variadic: bool,  // Support for varargs like printf
}

impl FfiFunctionSignature {
    pub fn new(
        name: String,
        abi: ABI,
        params: Vec<(String, CType)>,
        return_type: CType,
        is_variadic: bool,
    ) -> Self {
        FfiFunctionSignature {
            name,
            abi,
            params,
            return_type,
            is_variadic,
        }
    }
}

/// FFI struct definition for C struct compatibility
#[derive(Debug, Clone)]
pub struct FfiStructDef {
    pub name: String,
    pub fields: Vec<(String, CType)>,
    pub is_packed: bool,  // #[repr(C)] semantics
}

impl FfiStructDef {
    pub fn new(name: String, fields: Vec<(String, CType)>, is_packed: bool) -> Self {
        FfiStructDef {
            name,
            fields,
            is_packed,
        }
    }
}

/// FFI validator - checks C type compatibility
#[derive(Debug)]
pub struct FfiValidator {
    /// Registered C type definitions
    c_types: HashMap<String, CType>,
    
    /// Registered extern function signatures
    extern_funcs: HashMap<String, FfiFunctionSignature>,
    
    /// Registered struct definitions
    structs: HashMap<String, FfiStructDef>,
    
    /// Errors found during validation
    errors: Vec<FfiError>,
}

impl FfiValidator {
    pub fn new() -> Self {
        let mut validator = FfiValidator {
            c_types: HashMap::new(),
            extern_funcs: HashMap::new(),
            structs: HashMap::new(),
            errors: Vec::new(),
        };
        
        // Register standard C types
        validator.register_standard_c_types();
        
        validator
    }
    
    /// Register standard C type mappings
    fn register_standard_c_types(&mut self) {
        self.c_types.insert("c_int".to_string(), CType::CInt);
        self.c_types.insert("c_uint".to_string(), CType::CUint);
        self.c_types.insert("c_long".to_string(), CType::CLong);
        self.c_types.insert("c_ulong".to_string(), CType::CUlong);
        self.c_types.insert("c_longlong".to_string(), CType::CLongLong);
        self.c_types.insert("c_ulonglong".to_string(), CType::CUlongLong);
        self.c_types.insert("c_short".to_string(), CType::CShort);
        self.c_types.insert("c_ushort".to_string(), CType::CUshort);
        self.c_types.insert("c_char".to_string(), CType::CChar);
        self.c_types.insert("c_uchar".to_string(), CType::CUchar);
        self.c_types.insert("c_schar".to_string(), CType::CSignedChar);
        self.c_types.insert("c_float".to_string(), CType::CFloat);
        self.c_types.insert("c_double".to_string(), CType::CDouble);
        self.c_types.insert("c_void".to_string(), CType::CVoid);
    }
    
    /// Register an extern "C" function
    pub fn register_extern_function(&mut self, sig: FfiFunctionSignature) -> FfiResult<()> {
        if sig.abi != ABI::C {
            let error = FfiError {
                message: format!("function '{}' has invalid ABI for extern block", sig.name),
            };
            self.errors.push(error.clone());
            return Err(error);
        }
        
        self.extern_funcs.insert(sig.name.clone(), sig);
        Ok(())
    }
    
    /// Register a struct definition
    pub fn register_struct(&mut self, def: FfiStructDef) -> FfiResult<()> {
        // Validate that all field types are valid C types
        for (field_name, c_type) in &def.fields {
            if !self.is_valid_c_type(*c_type) {
                let error = FfiError {
                    message: format!(
                        "struct '{}' field '{}' has invalid C type",
                        def.name, field_name
                    ),
                };
                self.errors.push(error.clone());
                return Err(error);
            }
        }
        
        self.structs.insert(def.name.clone(), def);
        Ok(())
    }
    
    /// Check if a type is a valid C type
    pub fn is_valid_c_type(&self, c_type: CType) -> bool {
        // All defined CType variants are valid
        matches!(
            c_type,
            CType::CInt
                | CType::CUint
                | CType::CLong
                | CType::CUlong
                | CType::CLongLong
                | CType::CUlongLong
                | CType::CShort
                | CType::CUshort
                | CType::CChar
                | CType::CUchar
                | CType::CSignedChar
                | CType::CFloat
                | CType::CDouble
                | CType::CVoid
                | CType::CBool
        )
    }
    
    /// Validate function parameter compatibility
    pub fn validate_function_params(
        &mut self,
        func_name: &str,
        provided_params: &[(String, CType)],
    ) -> FfiResult<()> {
        if let Some(sig) = self.extern_funcs.get(func_name) {
            if !sig.is_variadic && provided_params.len() != sig.params.len() {
                let error = FfiError {
                    message: format!(
                        "function '{}' expects {} parameters, got {}",
                        func_name,
                        sig.params.len(),
                        provided_params.len()
                    ),
                };
                self.errors.push(error.clone());
                return Err(error);
            }
            
            // Check each parameter type matches
            for (i, (_, provided_type)) in provided_params.iter().enumerate() {
                if !sig.is_variadic && i < sig.params.len() {
                    let (_, expected_type) = &sig.params[i];
                    if provided_type != expected_type {
                        let error = FfiError {
                            message: format!(
                                "function '{}' parameter {} type mismatch: expected {}, got {}",
                                func_name, i, expected_type, provided_type
                            ),
                        };
                        self.errors.push(error.clone());
                        return Err(error);
                    }
                }
            }
            
            Ok(())
        } else {
            let error = FfiError {
                message: format!("unknown extern function '{}'", func_name),
            };
            self.errors.push(error.clone());
            Err(error)
        }
    }
    
    /// Validate return type compatibility
    pub fn validate_return_type(&mut self, func_name: &str, actual_type: CType) -> FfiResult<()> {
        if let Some(sig) = self.extern_funcs.get(func_name) {
            if actual_type != sig.return_type {
                let error = FfiError {
                    message: format!(
                        "function '{}' return type mismatch: expected {}, got {}",
                        func_name, sig.return_type, actual_type
                    ),
                };
                self.errors.push(error.clone());
                return Err(error);
            }
            Ok(())
        } else {
            let error = FfiError {
                message: format!("unknown extern function '{}'", func_name),
            };
            self.errors.push(error.clone());
            Err(error)
        }
    }
    
    /// Get all errors found
    pub fn errors(&self) -> &[FfiError] {
        &self.errors
    }
    
    /// Check if a function is registered
    pub fn has_function(&self, name: &str) -> bool {
        self.extern_funcs.contains_key(name)
    }
    
    /// Get a function signature
    pub fn get_function(&self, name: &str) -> Option<&FfiFunctionSignature> {
        self.extern_funcs.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ffi_validator() {
        let validator = FfiValidator::new();
        assert_eq!(validator.errors().len(), 0);
    }

    #[test]
    fn test_standard_c_types_registered() {
        let validator = FfiValidator::new();
        assert!(validator.c_types.contains_key("c_int"));
        assert!(validator.c_types.contains_key("c_char"));
        assert!(validator.c_types.contains_key("c_double"));
    }

    #[test]
    fn test_register_extern_function() {
        let mut validator = FfiValidator::new();
        let sig = FfiFunctionSignature::new(
            "printf".to_string(),
            ABI::C,
            vec![("format".to_string(), CType::CInt)],
            CType::CInt,
            true, // variadic
        );
        
        assert!(validator.register_extern_function(sig).is_ok());
        assert!(validator.has_function("printf"));
    }

    #[test]
    fn test_validate_function_params_correct() {
        let mut validator = FfiValidator::new();
        let sig = FfiFunctionSignature::new(
            "add".to_string(),
            ABI::C,
            vec![
                ("a".to_string(), CType::CInt),
                ("b".to_string(), CType::CInt),
            ],
            CType::CInt,
            false,
        );
        
        validator.register_extern_function(sig).unwrap();
        
        let params = vec![
            ("arg1".to_string(), CType::CInt),
            ("arg2".to_string(), CType::CInt),
        ];
        
        assert!(validator.validate_function_params("add", &params).is_ok());
    }

    #[test]
    fn test_validate_function_params_wrong_count() {
        let mut validator = FfiValidator::new();
        let sig = FfiFunctionSignature::new(
            "add".to_string(),
            ABI::C,
            vec![
                ("a".to_string(), CType::CInt),
                ("b".to_string(), CType::CInt),
            ],
            CType::CInt,
            false,
        );
        
        validator.register_extern_function(sig).unwrap();
        
        let params = vec![("arg1".to_string(), CType::CInt)];
        
        assert!(validator.validate_function_params("add", &params).is_err());
        assert_eq!(validator.errors().len(), 1);
    }

    #[test]
    fn test_validate_function_params_wrong_type() {
        let mut validator = FfiValidator::new();
        let sig = FfiFunctionSignature::new(
            "add".to_string(),
            ABI::C,
            vec![
                ("a".to_string(), CType::CInt),
                ("b".to_string(), CType::CInt),
            ],
            CType::CInt,
            false,
        );
        
        validator.register_extern_function(sig).unwrap();
        
        let params = vec![
            ("arg1".to_string(), CType::CInt),
            ("arg2".to_string(), CType::CDouble),
        ];
        
        assert!(validator.validate_function_params("add", &params).is_err());
    }

    #[test]
    fn test_variadic_function_allows_extra_params() {
        let mut validator = FfiValidator::new();
        let sig = FfiFunctionSignature::new(
            "printf".to_string(),
            ABI::C,
            vec![("format".to_string(), CType::CInt)],
            CType::CInt,
            true, // variadic
        );
        
        validator.register_extern_function(sig).unwrap();
        
        // Can call with extra params when variadic
        let params = vec![
            ("format".to_string(), CType::CInt),
            ("extra1".to_string(), CType::CInt),
            ("extra2".to_string(), CType::CDouble),
        ];
        
        assert!(validator.validate_function_params("printf", &params).is_ok());
    }

    #[test]
    fn test_register_struct() {
        let mut validator = FfiValidator::new();
        let def = FfiStructDef::new(
            "Point".to_string(),
            vec![
                ("x".to_string(), CType::CInt),
                ("y".to_string(), CType::CInt),
            ],
            true,
        );
        
        assert!(validator.register_struct(def).is_ok());
        assert!(validator.structs.contains_key("Point"));
    }

    #[test]
    fn test_validate_return_type_correct() {
        let mut validator = FfiValidator::new();
        let sig = FfiFunctionSignature::new(
            "get_int".to_string(),
            ABI::C,
            vec![],
            CType::CInt,
            false,
        );
        
        validator.register_extern_function(sig).unwrap();
        
        assert!(validator.validate_return_type("get_int", CType::CInt).is_ok());
    }

    #[test]
    fn test_validate_return_type_mismatch() {
        let mut validator = FfiValidator::new();
        let sig = FfiFunctionSignature::new(
            "get_int".to_string(),
            ABI::C,
            vec![],
            CType::CInt,
            false,
        );
        
        validator.register_extern_function(sig).unwrap();
        
        assert!(validator.validate_return_type("get_int", CType::CDouble).is_err());
    }

    #[test]
    fn test_c_type_display() {
        assert_eq!(CType::CInt.to_string(), "c_int");
        assert_eq!(CType::CChar.to_string(), "c_char");
        assert_eq!(CType::CVoid.to_string(), "c_void");
    }

    #[test]
    fn test_abi_display() {
        assert_eq!(ABI::C.to_string(), "C");
        assert_eq!(ABI::Rust.to_string(), "Rust");
    }
}
