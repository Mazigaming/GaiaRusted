//! # Type System - Core Type Definitions
//!
//! This module defines the complete type representation for the Rust compiler.
//! All Rust types are represented in this enum.

use std::fmt;

/// A unique identifier for struct types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StructId(pub usize);

/// A unique identifier for enum types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EnumId(pub usize);

/// A unique identifier for trait types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TraitId(pub usize);

/// A unique identifier for generic type parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GenericId(pub usize);

/// A type variable used in type inference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeVar(pub usize);

/// A lifetime identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LifetimeName(pub usize);

/// A lifetime variable used in lifetime inference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LifetimeVar(pub usize);

/// Lifetime representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lifetime {
    /// Named lifetime: 'a, 'b, etc.
    Named(LifetimeName),
    /// Elided lifetime: '_
    Elided,
    /// Static lifetime: 'static
    Static,
    /// Variable lifetime for inference
    Variable(LifetimeVar),
}

impl fmt::Display for Lifetime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Lifetime::Named(LifetimeName(n)) => write!(f, "'lt{}", n),
            Lifetime::Elided => write!(f, "'_"),
            Lifetime::Static => write!(f, "'static"),
            Lifetime::Variable(LifetimeVar(n)) => write!(f, "'t{}", n),
        }
    }
}

/// Complete type representation for Rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    // === Primitive Types ===
    I32,
    I64,
    I16,
    I8,
    Isize,
    U32,
    U64,
    U16,
    U8,
    Usize,
    F32,
    F64,
    Bool,
    Char,
    Str,
    /// String: owned heap-allocated string (String)
    String,
    
    // === Special Types ===
    /// Never type (!): represents diverging functions
    Never,
    /// Unit type (): empty tuple
    Unit,
    
    // === Standard Library Types ===
    /// Vec<T>: growable dynamic array
    Vec(Box<Type>),
    
    // === Composite Types ===
    /// Tuple: (T1, T2, ...)
    Tuple(Vec<Type>),
    /// Array: [T; N]
    Array {
        element: Box<Type>,
        size: usize,
    },
    /// Struct reference
    Struct(StructId),
    /// Enum reference
    Enum(EnumId),
    /// Trait object
    Trait(TraitId),
    
    // === Reference Types ===
    /// Reference: &'a T or &'a mut T
    Reference {
        lifetime: Option<Lifetime>,
        mutable: bool,
        inner: Box<Type>,
    },
    
    // === Pointer Types ===
    /// Raw pointer: *const T or *mut T
    RawPointer {
        mutable: bool,
        inner: Box<Type>,
    },
    
    // === Generic Types ===
    /// Generic type parameter: T, U, etc.
    Generic(GenericId),
    
    // === Function Types ===
    /// Function type: fn(T1, T2) -> R
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    
    // === Type Inference ===
    /// Type variable: used during inference
    Variable(TypeVar),
    
    // === Fallback ===
    /// Unknown type: when we can't determine the type yet
    Unknown,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::I16 => write!(f, "i16"),
            Type::I8 => write!(f, "i8"),
            Type::Isize => write!(f, "isize"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "u64"),
            Type::U16 => write!(f, "u16"),
            Type::U8 => write!(f, "u8"),
            Type::Usize => write!(f, "usize"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
            Type::Char => write!(f, "char"),
            Type::Str => write!(f, "str"),
            Type::String => write!(f, "String"),
            Type::Never => write!(f, "!"),
            Type::Unit => write!(f, "()"),
            Type::Vec(inner) => write!(f, "Vec<{}>", inner),
            Type::Tuple(tys) => {
                write!(f, "(")?;
                for (i, ty) in tys.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            Type::Array { element, size } => write!(f, "[{}; {}]", element, size),
            Type::Struct(StructId(id)) => write!(f, "Struct({})", id),
            Type::Enum(EnumId(id)) => write!(f, "Enum({})", id),
            Type::Trait(TraitId(id)) => write!(f, "Trait({})", id),
            Type::Reference {
                lifetime,
                mutable,
                inner,
            } => {
                write!(f, "&")?;
                if let Some(lt) = lifetime {
                    write!(f, "{} ", lt)?;
                }
                if *mutable {
                    write!(f, "mut ")?;
                }
                write!(f, "{}", inner)
            }
            Type::RawPointer { mutable, inner } => {
                write!(f, "*")?;
                if *mutable {
                    write!(f, "mut")?;
                } else {
                    write!(f, "const")?;
                }
                write!(f, " {}", inner)
            }
            Type::Generic(GenericId(id)) => write!(f, "T{}", id),
            Type::Function { params, ret } => {
                write!(f, "fn(")?;
                for (i, ty) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Variable(TypeVar(id)) => write!(f, "?{}", id),
            Type::Unknown => write!(f, "?unknown"),
        }
    }
}

impl Type {
    /// Check if this is a primitive type
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            Type::I32 | Type::I64 | Type::I16 | Type::I8
                | Type::U32 | Type::U64 | Type::U16 | Type::U8
                | Type::F32 | Type::F64 | Type::Bool | Type::Char
        )
    }

    /// Check if this is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::I32 | Type::I64 | Type::I16 | Type::I8
                | Type::U32 | Type::U64 | Type::U16 | Type::U8
                | Type::F32 | Type::F64
        )
    }

    /// Check if this is an integer type
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Type::I32 | Type::I64 | Type::I16 | Type::I8
                | Type::U32 | Type::U64 | Type::U16 | Type::U8
        )
    }

    /// Check if this is a floating-point type
    pub fn is_float(&self) -> bool {
        matches!(self, Type::F32 | Type::F64)
    }

    /// Get the inner type of a reference or pointer
    pub fn inner_type(&self) -> Option<&Type> {
        match self {
            Type::Reference { inner, .. } => Some(inner),
            Type::RawPointer { inner, .. } => Some(inner),
            Type::Array { element, .. } => Some(element),
            _ => None,
        }
    }

    /// Get the mutability of a reference
    pub fn is_mutable_ref(&self) -> bool {
        matches!(
            self,
            Type::Reference {
                mutable: true,
                ..
            }
        )
    }

    /// Check if this is a String type
    pub fn is_string(&self) -> bool {
        matches!(self, Type::String)
    }

    /// Check if this is a Vec<T> type
    pub fn is_vec(&self) -> bool {
        matches!(self, Type::Vec(_))
    }

    /// Get the element type of Vec<T>
    pub fn vec_element_type(&self) -> Option<&Type> {
        match self {
            Type::Vec(inner) => Some(inner),
            _ => None,
        }
    }

    /// Check if this is a collection type (String or Vec)
    pub fn is_collection(&self) -> bool {
        matches!(self, Type::String | Type::Vec(_))
    }
}

/// Generates fresh type variables for inference
#[derive(Debug, Clone)]
pub struct TypeVarGenerator {
    next_var: usize,
}

impl TypeVarGenerator {
    /// Create a new type variable generator
    pub fn new() -> Self {
        Self { next_var: 0 }
    }

    /// Generate a fresh type variable
    pub fn fresh(&mut self) -> TypeVar {
        let var = TypeVar(self.next_var);
        self.next_var += 1;
        var
    }

    /// Reset the generator
    pub fn reset(&mut self) {
        self.next_var = 0;
    }
}

impl Default for TypeVarGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Generates fresh lifetime variables
#[derive(Debug, Clone)]
pub struct LifetimeVarGenerator {
    next_var: usize,
}

impl LifetimeVarGenerator {
    /// Create a new lifetime variable generator
    pub fn new() -> Self {
        Self { next_var: 0 }
    }

    /// Generate a fresh lifetime variable
    pub fn fresh(&mut self) -> LifetimeVar {
        let var = LifetimeVar(self.next_var);
        self.next_var += 1;
        var
    }

    /// Reset the generator
    pub fn reset(&mut self) {
        self.next_var = 0;
    }
}

impl Default for LifetimeVarGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
        assert!(Type::I32.is_primitive());
        assert!(Type::F64.is_primitive());
        assert!(Type::Bool.is_primitive());
        assert!(!Type::Unit.is_primitive());
    }

    #[test]
    fn test_numeric_types() {
        assert!(Type::I32.is_numeric());
        assert!(Type::U64.is_numeric());
        assert!(Type::F32.is_numeric());
        assert!(!Type::Bool.is_numeric());
    }

    #[test]
    fn test_integer_types() {
        assert!(Type::I32.is_integer());
        assert!(Type::U64.is_integer());
        assert!(!Type::F32.is_integer());
    }

    #[test]
    fn test_float_types() {
        assert!(Type::F32.is_float());
        assert!(Type::F64.is_float());
        assert!(!Type::I32.is_float());
    }

    #[test]
    fn test_type_display() {
        assert_eq!(Type::I32.to_string(), "i32");
        assert_eq!(Type::Bool.to_string(), "bool");
        assert_eq!(Type::Str.to_string(), "str");
        assert_eq!(Type::Unit.to_string(), "()");
        assert_eq!(Type::Never.to_string(), "!");
    }

    #[test]
    fn test_tuple_display() {
        let tuple = Type::Tuple(vec![Type::I32, Type::Bool, Type::Str]);
        assert_eq!(tuple.to_string(), "(i32, bool, str)");
    }

    #[test]
    fn test_array_display() {
        let array = Type::Array {
            element: Box::new(Type::I32),
            size: 10,
        };
        assert_eq!(array.to_string(), "[i32; 10]");
    }

    #[test]
    fn test_reference_display() {
        let reference = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };
        assert_eq!(reference.to_string(), "&i32");
    }

    #[test]
    fn test_mutable_reference_display() {
        let reference = Type::Reference {
            lifetime: None,
            mutable: true,
            inner: Box::new(Type::I32),
        };
        assert_eq!(reference.to_string(), "&mut i32");
    }

    #[test]
    fn test_reference_with_lifetime() {
        let reference = Type::Reference {
            lifetime: Some(Lifetime::Named(LifetimeName(0))),
            mutable: false,
            inner: Box::new(Type::I32),
        };
        assert!(reference.to_string().contains("'lt0"));
    }

    #[test]
    fn test_raw_pointer_const() {
        let ptr = Type::RawPointer {
            mutable: false,
            inner: Box::new(Type::I32),
        };
        assert_eq!(ptr.to_string(), "*const i32");
    }

    #[test]
    fn test_raw_pointer_mut() {
        let ptr = Type::RawPointer {
            mutable: true,
            inner: Box::new(Type::I32),
        };
        assert_eq!(ptr.to_string(), "*mut i32");
    }

    #[test]
    fn test_function_type_display() {
        let func = Type::Function {
            params: vec![Type::I32, Type::Bool],
            ret: Box::new(Type::Str),
        };
        assert_eq!(func.to_string(), "fn(i32, bool) -> str");
    }

    #[test]
    fn test_type_variable_display() {
        let var = Type::Variable(TypeVar(0));
        assert_eq!(var.to_string(), "?0");

        let var2 = Type::Variable(TypeVar(42));
        assert_eq!(var2.to_string(), "?42");
    }

    #[test]
    fn test_generic_type_display() {
        let generic = Type::Generic(GenericId(0));
        assert_eq!(generic.to_string(), "T0");
    }

    #[test]
    fn test_type_equality() {
        assert_eq!(Type::I32, Type::I32);
        assert_ne!(Type::I32, Type::I64);
        assert_eq!(Type::Bool, Type::Bool);
    }

    #[test]
    fn test_tuple_equality() {
        let tuple1 = Type::Tuple(vec![Type::I32, Type::Bool]);
        let tuple2 = Type::Tuple(vec![Type::I32, Type::Bool]);
        let tuple3 = Type::Tuple(vec![Type::I32, Type::I32]);

        assert_eq!(tuple1, tuple2);
        assert_ne!(tuple1, tuple3);
    }

    #[test]
    fn test_reference_equality() {
        let ref1 = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };
        let ref2 = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };
        let ref3 = Type::Reference {
            lifetime: None,
            mutable: true,
            inner: Box::new(Type::I32),
        };

        assert_eq!(ref1, ref2);
        assert_ne!(ref1, ref3);
    }

    #[test]
    fn test_type_var_generator() {
        let mut gen = TypeVarGenerator::new();
        let v1 = gen.fresh();
        let v2 = gen.fresh();
        let v3 = gen.fresh();

        assert_eq!(v1, TypeVar(0));
        assert_eq!(v2, TypeVar(1));
        assert_eq!(v3, TypeVar(2));
    }

    #[test]
    fn test_lifetime_var_generator() {
        let mut gen = LifetimeVarGenerator::new();
        let lt1 = gen.fresh();
        let lt2 = gen.fresh();

        assert_eq!(lt1, LifetimeVar(0));
        assert_eq!(lt2, LifetimeVar(1));
    }

    #[test]
    fn test_type_var_clone_equality() {
        let var1 = TypeVar(42);
        let var2 = var1;
        assert_eq!(var1, var2);
    }

    #[test]
    fn test_inner_type() {
        let ref_type = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };
        assert_eq!(ref_type.inner_type(), Some(&Type::I32));
    }

    #[test]
    fn test_is_mutable_ref() {
        let mut_ref = Type::Reference {
            lifetime: None,
            mutable: true,
            inner: Box::new(Type::I32),
        };
        let immut_ref = Type::Reference {
            lifetime: None,
            mutable: false,
            inner: Box::new(Type::I32),
        };

        assert!(mut_ref.is_mutable_ref());
        assert!(!immut_ref.is_mutable_ref());
    }

    #[test]
    fn test_complex_nested_type() {
        let complex = Type::Array {
            element: Box::new(Type::Reference {
                lifetime: None,
                mutable: false,
                inner: Box::new(Type::Tuple(vec![Type::I32, Type::Bool])),
            }),
            size: 5,
        };
        let display = complex.to_string();
        assert!(display.contains("&(i32, bool)"));
        assert!(display.contains("["));
        assert!(display.contains("; 5]"));
    }

    #[test]
    fn test_lifetime_display() {
        assert_eq!(Lifetime::Static.to_string(), "'static");
        assert_eq!(Lifetime::Elided.to_string(), "'_");
    }

    #[test]
    fn test_type_cloning() {
        let original = Type::Tuple(vec![Type::I32, Type::Bool, Type::Str]);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}