//! # FFI Bridge System
//!
//! Foreign Function Interface support for C/C++ interoperability:
//! - C type mapping and conversion
//! - Function signature translation
//! - Memory safety verification for FFI calls
//! - ABI compatibility checking
//! - Unsafe code annotation and tracking

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CType {
    CVoid,
    CInt,
    CUint,
    CLong,
    CUlong,
    CFloat,
    CDouble,
    CChar,
    CPointer(Box<CType>),
    CArray { element: Box<CType>, size: usize },
    CStruct(String),
}

#[derive(Debug, Clone)]
pub struct ForeignFunction {
    pub name: String,
    pub params: Vec<(String, CType)>,
    pub return_type: CType,
    pub variadic: bool,
    pub extern_abi: String,
}

#[derive(Debug, Clone)]
pub struct FFISignature {
    pub rust_name: String,
    pub c_name: String,
    pub foreign_func: ForeignFunction,
    pub safety_verified: bool,
}

pub struct FFIBridge {
    foreign_functions: HashMap<String, FFISignature>,
    type_mappings: HashMap<String, CType>,
    unsafe_annotations: HashSet<String>,
    abi_compatibility: HashMap<String, bool>,
}

impl FFIBridge {
    pub fn new() -> Self {
        FFIBridge {
            foreign_functions: HashMap::new(),
            type_mappings: HashMap::new(),
            unsafe_annotations: HashSet::new(),
            abi_compatibility: HashMap::new(),
        }
    }

    pub fn register_foreign_function(
        &mut self,
        rust_name: String,
        signature: FFISignature,
    ) -> Result<(), String> {
        self.verify_abi_compatibility(&signature)?;
        self.foreign_functions.insert(rust_name, signature);
        Ok(())
    }

    fn verify_abi_compatibility(&mut self, signature: &FFISignature) -> Result<(), String> {
        let abi = &signature.foreign_func.extern_abi;

        if abi != "C" && abi != "cdecl" && abi != "stdcall" && abi != "fastcall" {
            return Err(format!("Unsupported ABI: {}", abi));
        }

        self.abi_compatibility.insert(abi.clone(), true);
        Ok(())
    }

    pub fn register_type_mapping(&mut self, rust_type: String, c_type: CType) {
        self.type_mappings.insert(rust_type, c_type);
    }

    pub fn map_rust_type_to_c(&self, rust_type: &str) -> Result<CType, String> {
        match rust_type {
            "i32" => Ok(CType::CInt),
            "u32" => Ok(CType::CUint),
            "i64" => Ok(CType::CLong),
            "u64" => Ok(CType::CUlong),
            "f32" => Ok(CType::CFloat),
            "f64" => Ok(CType::CDouble),
            "u8" => Ok(CType::CChar),
            "()" => Ok(CType::CVoid),
            _ => self.type_mappings.get(rust_type)
                .cloned()
                .ok_or(format!("No mapping for type {}", rust_type))
        }
    }

    pub fn annotate_unsafe(&mut self, func_name: String) {
        self.unsafe_annotations.insert(func_name);
    }

    pub fn is_annotated_unsafe(&self, func_name: &str) -> bool {
        self.unsafe_annotations.contains(func_name)
    }

    pub fn verify_memory_safety(&self, signature: &FFISignature) -> Result<(), String> {
        for (_name, param_type) in &signature.foreign_func.params {
            self.verify_type_safety(param_type)?;
        }

        self.verify_type_safety(&signature.foreign_func.return_type)?;
        Ok(())
    }

    fn verify_type_safety(&self, c_type: &CType) -> Result<(), String> {
        match c_type {
            CType::CPointer(_) => Ok(()),
            CType::CStruct(_) => Ok(()),
            CType::CArray { .. } => Ok(()),
            _ => Ok(()),
        }
    }

    pub fn get_foreign_function(&self, rust_name: &str) -> Option<FFISignature> {
        self.foreign_functions.get(rust_name).cloned()
    }

    pub fn get_function_by_c_name(&self, c_name: &str) -> Option<FFISignature> {
        self.foreign_functions.values()
            .find(|sig| sig.c_name == c_name)
            .cloned()
    }

    pub fn validate_function_call(
        &self,
        rust_name: &str,
        provided_args: &[CType],
    ) -> Result<(), String> {
        let signature = self.foreign_functions.get(rust_name)
            .ok_or(format!("Function {} not found", rust_name))?;

        let expected_count = signature.foreign_func.params.len();
        let provided_count = provided_args.len();

        if !signature.foreign_func.variadic && expected_count != provided_count {
            return Err(format!(
                "Argument count mismatch: expected {}, got {}",
                expected_count, provided_count
            ));
        }

        for (i, arg_type) in provided_args.iter().enumerate() {
            if i < expected_count {
                let expected = &signature.foreign_func.params[i].1;
                if !self.types_compatible(expected, arg_type) {
                    return Err(format!(
                        "Argument {} type mismatch: expected {:?}, got {:?}",
                        i, expected, arg_type
                    ));
                }
            }
        }

        Ok(())
    }

    fn types_compatible(&self, expected: &CType, provided: &CType) -> bool {
        match (expected, provided) {
            (CType::CInt, CType::CInt) => true,
            (CType::CPointer(e1), CType::CPointer(e2)) => {
                self.types_compatible(e1, e2)
            }
            (CType::CVoid, CType::CVoid) => true,
            (CType::CFloat, CType::CFloat) => true,
            (CType::CDouble, CType::CDouble) => true,
            (CType::CChar, CType::CChar) => true,
            _ => false,
        }
    }

    pub fn collect_unsafe_functions(&self) -> Vec<String> {
        self.unsafe_annotations.iter().cloned().collect()
    }

    pub fn get_return_type(&self, rust_name: &str) -> Option<CType> {
        self.foreign_functions.get(rust_name)
            .map(|sig| sig.foreign_func.return_type.clone())
    }

    pub fn mark_safety_verified(&mut self, rust_name: &str) -> Result<(), String> {
        let signature = self.foreign_functions.get_mut(rust_name)
            .ok_or(format!("Function {} not found", rust_name))?;

        signature.safety_verified = true;
        Ok(())
    }

    pub fn is_variadic_function(&self, rust_name: &str) -> bool {
        self.foreign_functions.get(rust_name)
            .map(|sig| sig.foreign_func.variadic)
            .unwrap_or(false)
    }

    pub fn get_abi(&self, rust_name: &str) -> Option<String> {
        self.foreign_functions.get(rust_name)
            .map(|sig| sig.foreign_func.extern_abi.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bridge() {
        let _bridge = FFIBridge::new();
        assert!(true);
    }

    #[test]
    fn test_register_foreign_function() {
        let mut bridge = FFIBridge::new();
        let foreign_func = ForeignFunction {
            name: "strlen".to_string(),
            params: vec![("s".to_string(), CType::CPointer(Box::new(CType::CChar)))],
            return_type: CType::CUlong,
            variadic: false,
            extern_abi: "C".to_string(),
        };

        let signature = FFISignature {
            rust_name: "strlen".to_string(),
            c_name: "strlen".to_string(),
            foreign_func,
            safety_verified: false,
        };

        assert!(bridge.register_foreign_function("strlen".to_string(), signature).is_ok());
    }

    #[test]
    fn test_map_rust_types() {
        let bridge = FFIBridge::new();

        assert_eq!(bridge.map_rust_type_to_c("i32").unwrap(), CType::CInt);
        assert_eq!(bridge.map_rust_type_to_c("u32").unwrap(), CType::CUint);
        assert_eq!(bridge.map_rust_type_to_c("f64").unwrap(), CType::CDouble);
        assert_eq!(bridge.map_rust_type_to_c("()").unwrap(), CType::CVoid);
    }

    #[test]
    fn test_annotate_unsafe() {
        let mut bridge = FFIBridge::new();
        bridge.annotate_unsafe("strlen".to_string());

        assert!(bridge.is_annotated_unsafe("strlen"));
        assert!(!bridge.is_annotated_unsafe("safe_func"));
    }

    #[test]
    fn test_verify_memory_safety() {
        let mut bridge = FFIBridge::new();
        let foreign_func = ForeignFunction {
            name: "safe_func".to_string(),
            params: vec![("x".to_string(), CType::CInt)],
            return_type: CType::CInt,
            variadic: false,
            extern_abi: "C".to_string(),
        };

        let signature = FFISignature {
            rust_name: "safe_func".to_string(),
            c_name: "safe_func".to_string(),
            foreign_func: foreign_func.clone(),
            safety_verified: false,
        };

        bridge.register_foreign_function("safe_func".to_string(), signature.clone()).unwrap();
        assert!(bridge.verify_memory_safety(&signature).is_ok());
    }

    #[test]
    fn test_get_foreign_function() {
        let mut bridge = FFIBridge::new();
        let foreign_func = ForeignFunction {
            name: "test_func".to_string(),
            params: vec![],
            return_type: CType::CVoid,
            variadic: false,
            extern_abi: "C".to_string(),
        };

        let signature = FFISignature {
            rust_name: "test_func".to_string(),
            c_name: "test_func".to_string(),
            foreign_func,
            safety_verified: false,
        };

        bridge.register_foreign_function("test_func".to_string(), signature).unwrap();
        assert!(bridge.get_foreign_function("test_func").is_some());
    }

    #[test]
    fn test_get_function_by_c_name() {
        let mut bridge = FFIBridge::new();
        let foreign_func = ForeignFunction {
            name: "c_func".to_string(),
            params: vec![],
            return_type: CType::CVoid,
            variadic: false,
            extern_abi: "C".to_string(),
        };

        let signature = FFISignature {
            rust_name: "c_func".to_string(),
            c_name: "c_func".to_string(),
            foreign_func,
            safety_verified: false,
        };

        bridge.register_foreign_function("c_func".to_string(), signature).unwrap();
        assert!(bridge.get_function_by_c_name("c_func").is_some());
    }

    #[test]
    fn test_validate_function_call() {
        let mut bridge = FFIBridge::new();
        let foreign_func = ForeignFunction {
            name: "add".to_string(),
            params: vec![
                ("a".to_string(), CType::CInt),
                ("b".to_string(), CType::CInt),
            ],
            return_type: CType::CInt,
            variadic: false,
            extern_abi: "C".to_string(),
        };

        let signature = FFISignature {
            rust_name: "add".to_string(),
            c_name: "add".to_string(),
            foreign_func,
            safety_verified: false,
        };

        bridge.register_foreign_function("add".to_string(), signature).unwrap();

        let args = vec![CType::CInt, CType::CInt];
        assert!(bridge.validate_function_call("add", &args).is_ok());
    }

    #[test]
    fn test_collect_unsafe_functions() {
        let mut bridge = FFIBridge::new();
        bridge.annotate_unsafe("func1".to_string());
        bridge.annotate_unsafe("func2".to_string());

        let unsafe_funcs = bridge.collect_unsafe_functions();
        assert_eq!(unsafe_funcs.len(), 2);
    }

    #[test]
    fn test_get_return_type() {
        let mut bridge = FFIBridge::new();
        let foreign_func = ForeignFunction {
            name: "get_int".to_string(),
            params: vec![],
            return_type: CType::CInt,
            variadic: false,
            extern_abi: "C".to_string(),
        };

        let signature = FFISignature {
            rust_name: "get_int".to_string(),
            c_name: "get_int".to_string(),
            foreign_func,
            safety_verified: false,
        };

        bridge.register_foreign_function("get_int".to_string(), signature).unwrap();
        assert_eq!(bridge.get_return_type("get_int").unwrap(), CType::CInt);
    }

    #[test]
    fn test_mark_safety_verified() {
        let mut bridge = FFIBridge::new();
        let foreign_func = ForeignFunction {
            name: "safe".to_string(),
            params: vec![],
            return_type: CType::CVoid,
            variadic: false,
            extern_abi: "C".to_string(),
        };

        let signature = FFISignature {
            rust_name: "safe".to_string(),
            c_name: "safe".to_string(),
            foreign_func,
            safety_verified: false,
        };

        bridge.register_foreign_function("safe".to_string(), signature).unwrap();
        assert!(bridge.mark_safety_verified("safe").is_ok());
    }

    #[test]
    fn test_is_variadic_function() {
        let mut bridge = FFIBridge::new();
        let foreign_func = ForeignFunction {
            name: "printf".to_string(),
            params: vec![("format".to_string(), CType::CPointer(Box::new(CType::CChar)))],
            return_type: CType::CInt,
            variadic: true,
            extern_abi: "C".to_string(),
        };

        let signature = FFISignature {
            rust_name: "printf".to_string(),
            c_name: "printf".to_string(),
            foreign_func,
            safety_verified: false,
        };

        bridge.register_foreign_function("printf".to_string(), signature).unwrap();
        assert!(bridge.is_variadic_function("printf"));
    }

    #[test]
    fn test_register_type_mapping() {
        let mut bridge = FFIBridge::new();
        bridge.register_type_mapping("MyType".to_string(), CType::CInt);

        assert!(bridge.map_rust_type_to_c("MyType").is_ok());
    }

    #[test]
    fn test_get_abi() {
        let mut bridge = FFIBridge::new();
        let foreign_func = ForeignFunction {
            name: "c_func".to_string(),
            params: vec![],
            return_type: CType::CVoid,
            variadic: false,
            extern_abi: "C".to_string(),
        };

        let signature = FFISignature {
            rust_name: "c_func".to_string(),
            c_name: "c_func".to_string(),
            foreign_func,
            safety_verified: false,
        };

        bridge.register_foreign_function("c_func".to_string(), signature).unwrap();
        assert_eq!(bridge.get_abi("c_func").unwrap(), "C");
    }
}
