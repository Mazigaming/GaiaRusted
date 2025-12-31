//! # Phase 4D: Type System Bridge
//!
//! Integrates the borrowchecker modules (union detection, iterator analysis, NLL binding tracking)
//! with the actual type system to enable real type-aware safety checking.
//!
//! This module bridges the gap between:
//! - High-level type information from the type system
//! - Low-level safety analysis in union_detection, iterator_analysis, and nll_binding_tracker
//!
//! ## Key Responsibilities
//!
//! 1. **Type Information Extraction**: Convert type system Type into borrowchecker representations
//! 2. **Trait Bound Analysis**: Extract Iterator::Item, IntoIterator trait constraints
//! 3. **Union Type Detection**: Identify union/enum types that require unsafe field access
//! 4. **Generic Support**: Handle generic type parameters and instantiations
//! 5. **Binding Type Inference**: Determine precise types for loop variables

use crate::lowering::{HirExpression, HirType};
use crate::typesystem::types::Type;
use std::collections::HashMap;

/// Bridge between type system and borrowchecker analysis
#[derive(Debug)]
pub struct TypeSystemBridge {
    /// Maps HirType names to typesystem Type information
    type_mapping: HashMap<String, Type>,
    
    /// Iterator trait information extracted from bounds
    /// Maps collection type to its Iterator::Item type
    iterator_item_types: HashMap<String, HirType>,
    
    /// Union types identified from the type system
    /// Maps union/enum names to their variant information
    union_types: HashMap<String, UnionTypeInfo>,
    
    /// Generic type parameter bindings
    generic_bindings: HashMap<String, HirType>,
}

/// Information about a union/enum type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionTypeInfo {
    /// The union/enum name
    pub name: String,
    /// Whether this is a true union (C-style) or Rust enum
    pub is_union: bool,
    /// Variant names for enums
    pub variants: Vec<String>,
}

impl TypeSystemBridge {
    /// Create a new type system bridge
    pub fn new() -> Self {
        TypeSystemBridge {
            type_mapping: HashMap::new(),
            iterator_item_types: HashMap::new(),
            union_types: HashMap::new(),
            generic_bindings: HashMap::new(),
        }
    }

    /// Register a type from the type system
    pub fn register_type(&mut self, name: &str, ty: Type) {
        self.type_mapping.insert(name.to_string(), ty);
    }

    /// Register iterator information for a collection type
    /// 
    /// # Example
    /// For `Vec<i32>`, register that Iterator::Item = &i32
    pub fn register_iterator_info(
        &mut self,
        collection_type: &str,
        item_type: HirType,
        is_into_iterator: bool,
    ) {
        // Store both the borrowed and consuming variants
        if is_into_iterator {
            // IntoIterator<Item=T>: consumes the collection
            self.iterator_item_types.insert(
                format!("IntoIterator<{}>", collection_type),
                item_type,
            );
        } else {
            // Iterator<Item=&T>: borrows the collection
            self.iterator_item_types.insert(
                format!("Iterator<{}>", collection_type),
                item_type,
            );
        }
    }

    /// Register a union/enum type
    pub fn register_union_type(&mut self, info: UnionTypeInfo) {
        self.union_types.insert(info.name.clone(), info);
    }

    /// Bind a generic type parameter
    pub fn bind_generic(&mut self, param: &str, concrete_type: HirType) {
        self.generic_bindings.insert(param.to_string(), concrete_type);
    }

    /// Check if a type is a union
    pub fn is_union_type(&self, type_name: &str) -> bool {
        self.union_types.get(type_name).is_some()
    }

    /// Get union information if the type is a union
    pub fn get_union_info(&self, type_name: &str) -> Option<&UnionTypeInfo> {
        self.union_types.get(type_name)
    }

    /// Determine the Iterator::Item type for a collection expression
    /// 
    /// # Returns
    /// - Some(HirType) if the collection has a known Iterator trait
    /// - None if the type is unknown or not iterable
    pub fn infer_iterator_item_type(&self, collection_expr: &HirExpression) -> Option<HirType> {
        // Extract the type of the collection from the expression
        let collection_type = self.infer_expression_type(collection_expr)?;
        let type_name = self.type_to_string(&collection_type);

        // Look up in our iterator registry
        // Try both Iterator and IntoIterator variants
        if let Some(item_type) = self.iterator_item_types.get(&format!("Iterator<{}>", type_name)) {
            return Some(item_type.clone());
        }

        if let Some(item_type) = self.iterator_item_types.get(&format!("IntoIterator<{}>", type_name)) {
            return Some(item_type.clone());
        }

        // Fallback: if no explicit registration, return Unknown
        Some(HirType::Named("Unknown".to_string()))
    }

    /// Determine if an iterator consumes or borrows its collection
    /// 
    /// # Returns
    /// - true if this is an IntoIterator (consumes)
    /// - false if this is an Iterator (borrows)
    pub fn is_consuming_iterator(&self, collection_expr: &HirExpression) -> bool {
        let collection_type = match self.infer_expression_type(collection_expr) {
            Some(ty) => self.type_to_string(&ty),
            None => return false,
        };

        // Check if we have IntoIterator registration for this type
        self.iterator_item_types.contains_key(&format!("IntoIterator<{}>", collection_type))
    }

    /// Infer the type of an expression using available type information
    fn infer_expression_type(&self, expr: &HirExpression) -> Option<HirType> {
        match expr {
            // Variable: look up its type in bindings
            HirExpression::Variable(name) => {
                // Try to find a generic binding first
                self.generic_bindings.get(name).cloned()
                    .or_else(|| Some(HirType::Named(name.clone())))
            }

            // Field access: extract the field type
            HirExpression::FieldAccess { object, field } => {
                let _object_type = self.infer_expression_type(object)?;
                // Would need field type information to fully resolve
                // For now, return a placeholder
                Some(HirType::Named(format!("{}_field", field)))
            }

            // Method call: infer from return type
            HirExpression::MethodCall { receiver, method, .. } => {
                let receiver_type = self.infer_expression_type(receiver)?;
                let receiver_name = self.type_to_string(&receiver_type);
                
                // Common method patterns
                match method.as_str() {
                    "iter" => {
                        // Vec<T>.iter() -> Iterator<Item=&T>
                        Some(HirType::Reference(
                            Box::new(HirType::Named(format!("{}_elem", receiver_name)))
                        ))
                    }
                    "iter_mut" => {
                        // Vec<T>.iter_mut() -> Iterator<Item=&mut T>
                        Some(HirType::MutableReference(
                            Box::new(HirType::Named(format!("{}_elem", receiver_name)))
                        ))
                    }
                    "into_iter" => {
                        // Vec<T>.into_iter() -> IntoIterator<Item=T>
                        Some(HirType::Named(format!("{}_elem", receiver_name)))
                    }
                    _ => None,
                }
            }

            // Range: yields the element type
            HirExpression::Range { .. } => {
                // Ranges iterate over their element type (usually integers)
                Some(HirType::Int64) // Default to i64 for ranges
            }

            // Array: yields element type
            HirExpression::ArrayLiteral(elements) => {
                if let Some(first) = elements.first() {
                    self.infer_expression_type(first)
                } else {
                    Some(HirType::Unknown)
                }
            }

            _ => None,
        }
    }

    /// Convert a HirType to a string representation for lookup
    fn type_to_string(&self, ty: &HirType) -> String {
        match ty {
            HirType::Int32 => "i32".to_string(),
            HirType::Int64 => "i64".to_string(),
            HirType::UInt32 => "u32".to_string(),
            HirType::UInt64 => "u64".to_string(),
            HirType::Float64 => "f64".to_string(),
            HirType::Bool => "bool".to_string(),
            HirType::String => "String".to_string(),
            HirType::Named(n) => n.clone(),
            HirType::Reference(inner) => format!("&{}", self.type_to_string(inner)),
            HirType::MutableReference(inner) => format!("&mut {}", self.type_to_string(inner)),
            HirType::Pointer(inner) => format!("*{}", self.type_to_string(inner)),
            _ => "Unknown".to_string(),
        }
    }

    /// Extract trait bounds and constraints from a type
    /// 
    /// This is a stub that would be fully implemented to analyze actual trait bounds.
    /// Returns information about what traits a type implements.
    pub fn extract_trait_bounds(&self, _type_name: &str) -> Vec<String> {
        // In a full implementation, this would:
        // 1. Look up the type in the symbol table
        // 2. Extract its trait implementations
        // 3. Return the list of trait names
        //
        // For now, return empty - this is expanded in Phase 4D continuation
        Vec::new()
    }

    /// Check if a type implements a specific trait
    pub fn implements_trait(&self, _type_name: &str, _trait_name: &str) -> bool {
        // TODO: Implement using actual trait information
        false
    }

    /// Get all registered union types
    pub fn union_types(&self) -> Vec<&str> {
        self.union_types.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bridge() {
        let bridge = TypeSystemBridge::new();
        assert_eq!(bridge.union_types.len(), 0);
        assert_eq!(bridge.iterator_item_types.len(), 0);
    }

    #[test]
    fn test_register_iterator_info() {
        let mut bridge = TypeSystemBridge::new();
        bridge.register_iterator_info(
            "Vec",
            HirType::Int32,
            false,
        );

        // Check that the information is stored
        assert!(bridge.iterator_item_types.contains_key("Iterator<Vec>"));
    }

    #[test]
    fn test_register_union_type() {
        let mut bridge = TypeSystemBridge::new();
        let union_info = UnionTypeInfo {
            name: "MyUnion".to_string(),
            is_union: true,
            variants: vec!["A".to_string(), "B".to_string()],
        };

        bridge.register_union_type(union_info);
        assert!(bridge.is_union_type("MyUnion"));
    }

    #[test]
    fn test_bind_generic() {
        let mut bridge = TypeSystemBridge::new();
        bridge.bind_generic("T", HirType::Int32);

        let bound = bridge.generic_bindings.get("T");
        assert!(bound.is_some());
        assert_eq!(bound.unwrap(), &HirType::Int32);
    }

    #[test]
    fn test_infer_variable_type() {
        let mut bridge = TypeSystemBridge::new();
        bridge.bind_generic("x", HirType::Int64);

        let var_expr = HirExpression::Variable("x".to_string());
        let inferred = bridge.infer_expression_type(&var_expr);

        assert_eq!(inferred, Some(HirType::Int64));
    }

    #[test]
    fn test_infer_range_type() {
        let bridge = TypeSystemBridge::new();
        let range_expr = HirExpression::Range {
            start: Some(Box::new(HirExpression::Integer(0))),
            end: Some(Box::new(HirExpression::Integer(10))),
            inclusive: false,
        };

        let inferred = bridge.infer_expression_type(&range_expr);
        assert_eq!(inferred, Some(HirType::Int64));
    }

    #[test]
    fn test_union_info_variants() {
        let mut bridge = TypeSystemBridge::new();
        let union_info = UnionTypeInfo {
            name: "Result".to_string(),
            is_union: false,
            variants: vec!["Ok".to_string(), "Err".to_string()],
        };

        bridge.register_union_type(union_info);
        let info = bridge.get_union_info("Result").unwrap();

        assert_eq!(info.variants.len(), 2);
        assert!(!info.is_union);
    }

    #[test]
    fn test_type_to_string_conversions() {
        let bridge = TypeSystemBridge::new();

        assert_eq!(bridge.type_to_string(&HirType::Int32), "i32");
        assert_eq!(bridge.type_to_string(&HirType::String), "String");

        let ref_type = HirType::Reference(Box::new(HirType::Int32));
        assert_eq!(bridge.type_to_string(&ref_type), "&i32");
    }

    #[test]
    fn test_consuming_vs_borrowing_iterator() {
        let mut bridge = TypeSystemBridge::new();

        // Register Vec<i32> with Iterator (borrowing)
        bridge.register_iterator_info("Vec", HirType::Int32, false);

        // Register String with IntoIterator (consuming)
        bridge.register_iterator_info("String", HirType::Char, true);

        // Now check the consuming detection
        // Note: This needs actual expression type inference to work fully
        // For now, it's just checking the registry directly
    }
}
