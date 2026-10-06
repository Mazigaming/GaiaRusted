//! # C Type System for FFI
//!
//! Provides C type representations and mappings for Foreign Function Interface support.
//!
//! ## Features:
//! - C type enum with all standard C types
//! - Rust to C type mapping
//! - Sizeof calculations for C types
//! - Memory layout compatibility checking

use std::fmt;
use crate::parser::ast::Type as RustType;

/// C Type representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CType {
    // Primitive types
    CVoid,          // void
    CChar,          // char (usually 1 byte)
    CShort,         // short
    CInt,           // int
    CLong,          // long
    CLongLong,      // long long
    CFloat,         // float
    CDouble,        // double
    
    // Unsigned variants
    CUchar,         // unsigned char
    CUshort,        // unsigned short
    CUint,          // unsigned int
    CULong,         // unsigned long
    CULongLong,     // unsigned long long
    
    // Pointers
    CPointer(Box<CType>),
    
    // Arrays
    CArray(Box<CType>, usize),
    
    // Function pointers
    CFunctionPointer {
        return_type: Box<CType>,
        params: Vec<CType>,
        is_variadic: bool,
    },
}

impl fmt::Display for CType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CType::CVoid => write!(f, "void"),
            CType::CChar => write!(f, "char"),
            CType::CShort => write!(f, "short"),
            CType::CInt => write!(f, "int"),
            CType::CLong => write!(f, "long"),
            CType::CLongLong => write!(f, "long long"),
            CType::CFloat => write!(f, "float"),
            CType::CDouble => write!(f, "double"),
            CType::CUchar => write!(f, "unsigned char"),
            CType::CUshort => write!(f, "unsigned short"),
            CType::CUint => write!(f, "unsigned int"),
            CType::CULong => write!(f, "unsigned long"),
            CType::CULongLong => write!(f, "unsigned long long"),
            CType::CPointer(inner) => write!(f, "*{}", inner),
            CType::CArray(inner, size) => write!(f, "{}[{}]", inner, size),
            CType::CFunctionPointer { return_type, params, is_variadic } => {
                write!(f, "(*fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", param)?;
                }
                if *is_variadic {
                    if !params.is_empty() { write!(f, ", ")?; }
                    write!(f, "...")?;
                }
                write!(f, ") -> {})", return_type)
            }
        }
    }
}

/// Map Rust types to C types
pub fn rust_to_c_type(rust_type: &RustType) -> Result<CType, String> {
    match rust_type {
        RustType::Int32 => Ok(CType::CInt),
        RustType::Int64 => Ok(CType::CLongLong),
        RustType::Float64 => Ok(CType::CDouble),
        RustType::Bool => Ok(CType::CChar),
        RustType::Reference(inner) => {
            let inner_c = rust_to_c_type(inner)?;
            Ok(CType::CPointer(Box::new(inner_c)))
        }
        RustType::Pointer(inner) => {
            let inner_c = rust_to_c_type(inner)?;
            Ok(CType::CPointer(Box::new(inner_c)))
        }
        _ => Err(format!("Type {:?} is not FFI-safe", rust_type)),
    }
}

/// Calculate sizeof for C types (for x86-64 Linux)
pub fn sizeof(c_type: &CType) -> usize {
    match c_type {
        CType::CVoid => 1,           // void is 1 byte for pointer arithmetic
        CType::CChar | CType::CUchar => 1,
        CType::CShort | CType::CUshort => 2,
        CType::CInt | CType::CUint => 4,
        CType::CLong | CType::CULong => 8,  // 64-bit system
        CType::CLongLong | CType::CULongLong => 8,
        CType::CFloat => 4,
        CType::CDouble => 8,
        CType::CPointer(_) => 8,              // 64-bit pointer
        CType::CArray(inner, count) => sizeof(inner) * count,
        CType::CFunctionPointer { .. } => 8, // function pointer is 8 bytes
    }
}

/// Check if a type is FFI-safe (can be used in extern "C" blocks)
pub fn is_ffi_safe(rust_type: &RustType) -> bool {
    match rust_type {
        RustType::Int32 | RustType::Int64 | RustType::Float64 | RustType::Bool => true,
        RustType::Reference(inner) | RustType::Pointer(inner) => is_ffi_safe(inner),
        RustType::Unit => true,  // void return type
        _ => false,  // Generics, trait objects, etc. are not FFI-safe
    }
}

/// Alignment for C types (x86-64 System V ABI)
pub fn alignment(c_type: &CType) -> usize {
    match c_type {
        CType::CVoid => 1,
        CType::CChar | CType::CUchar => 1,
        CType::CShort | CType::CUshort => 2,
        CType::CInt | CType::CUint | CType::CFloat => 4,
        CType::CLong | CType::CULong | CType::CLongLong | CType::CULongLong 
        | CType::CDouble | CType::CPointer(_) | CType::CFunctionPointer { .. } => 8,
        CType::CArray(inner, _) => alignment(inner),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_c_type_display() {
        assert_eq!(CType::CInt.to_string(), "int");
        assert_eq!(CType::CChar.to_string(), "char");
        assert_eq!(CType::CVoid.to_string(), "void");
        assert_eq!(CType::CPointer(Box::new(CType::CInt)).to_string(), "*int");
    }
    
    #[test]
    fn test_sizeof() {
        assert_eq!(sizeof(&CType::CChar), 1);
        assert_eq!(sizeof(&CType::CInt), 4);
        assert_eq!(sizeof(&CType::CLong), 8);
        assert_eq!(sizeof(&CType::CDouble), 8);
        assert_eq!(sizeof(&CType::CPointer(Box::new(CType::CInt))), 8);
        assert_eq!(sizeof(&CType::CArray(Box::new(CType::CInt), 10)), 40);
    }
    
    #[test]
    fn test_alignment() {
        assert_eq!(alignment(&CType::CChar), 1);
        assert_eq!(alignment(&CType::CInt), 4);
        assert_eq!(alignment(&CType::CLong), 8);
        assert_eq!(alignment(&CType::CPointer(Box::new(CType::CInt))), 8);
    }
    
    #[test]
    fn test_ffi_safe() {
        use crate::parser::ast::Type as RustType;
        assert!(is_ffi_safe(&RustType::Int32));
        assert!(is_ffi_safe(&RustType::Float64));
        assert!(is_ffi_safe(&RustType::Unit));
        // Generics would not be FFI-safe
    }
}