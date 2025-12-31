//! # Iterator Type Analysis
//!
//! Analyzes Iterator trait bounds and associated types to determine
//! loop variable ownership in for loops.
//!
//! When a for loop iterates over a collection, the loop variable's type
//! and ownership semantics depend on the Iterator::Item type:
//!
//! - `Iterator<Item=&T>`: loop variable is `&T` (borrowed immutably)
//! - `Iterator<Item=&mut T>`: loop variable is `&mut T` (borrowed mutably)
//! - `Iterator<Item=T>`: loop variable is `T` (owned, can be moved)
//!
//! This module provides infrastructure to analyze these patterns and
//! determine proper loop variable bindings.

use crate::lowering::{HirExpression, HirType};
use std::collections::HashMap;

/// Iterator trait information extracted from type analysis
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IteratorInfo {
    /// The Iterator::Item associated type
    pub item_type: HirType,
    /// Whether this is an IntoIterator (consumes) vs Iterator (borrows)
    pub is_into_iterator: bool,
}

/// Analyzes iterator expressions to determine loop variable types
#[derive(Debug)]
pub struct IteratorAnalyzer {
    /// Cached iterator type information
    /// Maps collection type names to their iterator characteristics
    /// e.g., "Vec<T>" -> IteratorInfo { item_type: Reference(T), is_into_iterator: false }
    type_iterator_cache: HashMap<String, IteratorInfo>,
}

impl IteratorAnalyzer {
    pub fn new() -> Self {
        IteratorAnalyzer {
            type_iterator_cache: HashMap::new(),
        }
    }

    /// Register iterator information for a type
    ///
    /// This is used to populate the analyzer with known iterator traits
    /// for standard collection types.
    pub fn register_iterator_type(
        &mut self,
        type_name: &str,
        item_type: HirType,
        is_into_iterator: bool,
    ) {
        self.type_iterator_cache.insert(
            type_name.to_string(),
            IteratorInfo {
                item_type,
                is_into_iterator,
            },
        );
    }

    /// Register standard collection iterator types
    ///
    /// Populates common patterns like Vec<T>, &[T], etc.
    /// In a full implementation, these would come from trait analysis.
    pub fn register_standard_types(&mut self) {
        // These are placeholder registrations. Real implementation would
        // analyze trait bounds from the type system.

        // Vec<T> implements IntoIterator<Item=T>
        // When iterated directly (by value), yields owned T
        self.register_iterator_type(
            "Vec",
            HirType::Unknown, // Would be TypeVar T
            true,
        );

        // &Vec<T> / &[T] implements Iterator<Item=&T>
        // When iterated by reference, yields borrowed &T
        self.register_iterator_type(
            "Reference",
            HirType::Unknown, // Would be Reference(T)
            false,
        );

        // &mut Vec<T> implements Iterator<Item=&mut T>
        // When iterated by mutable reference, yields &mut T
        self.register_iterator_type(
            "MutableReference",
            HirType::Unknown, // Would be MutableReference(T)
            false,
        );
    }

    /// Analyze an iterator expression to determine the loop variable type
    ///
    /// # Parameters
    /// - `expr`: The iterator expression (e.g., the collection in `for x in collection`)
    ///
    /// # Returns
    /// Some(IteratorInfo) if the expression's type is recognized as iterable,
    /// None if the iterator trait cannot be determined
    pub fn analyze_iterator(&self, expr: &HirExpression) -> Option<IteratorInfo> {
        match expr {
            // Direct variable: check if it's a known iterable type
            HirExpression::Variable(name) => {
                // Try to look up the variable's type from the iterator cache
                // In a full implementation, this would check the scope's type info
                self.type_iterator_cache
                    .get(name)
                    .or_else(|| {
                        // Check if name starts with pattern (e.g., "my_vec" might be Vec)
                        if name.contains("vec") {
                            self.type_iterator_cache.get("Vec")
                        } else {
                            None
                        }
                    })
                    .cloned()
            }

            // Field access: iterate over a field
            HirExpression::FieldAccess { object, field: _ } => {
                // Recursively analyze the object
                self.analyze_iterator(object.as_ref())
            }

            // Method call: some methods return iterators
            // e.g., vec.iter(), vec.iter_mut(), vec.into_iter()
            HirExpression::MethodCall {
                receiver,
                method,
                args: _,
            } => {
                // Check method name for iterator patterns
                match method.as_str() {
                    "iter" => {
                        // iter() returns Iterator<Item=&T>
                        // Analyze the receiver to get its inner type
                        self.analyze_iterator(receiver.as_ref()).map(|_| IteratorInfo {
                            item_type: HirType::Unknown, // Would be Reference(T)
                            is_into_iterator: false,
                        })
                    }
                    "iter_mut" => {
                        // iter_mut() returns Iterator<Item=&mut T>
                        self.analyze_iterator(receiver.as_ref()).map(|_| IteratorInfo {
                            item_type: HirType::Unknown, // Would be MutableReference(T)
                            is_into_iterator: false,
                        })
                    }
                    "into_iter" => {
                        // into_iter() returns IntoIterator<Item=T>
                        // Consumes the receiver
                        self.analyze_iterator(receiver.as_ref()).map(|_| IteratorInfo {
                            item_type: HirType::Unknown, // Would be T
                            is_into_iterator: true,
                        })
                    }
                    _ => {
                        // Other method calls might return iterators
                        // For now, return None for unknown methods
                        None
                    }
                }
            }

            // Range literals: 0..10, 1..=5
            HirExpression::Range { .. } => {
                // Range<T> implements Iterator<Item=T> where T is the numeric type
                // For simplicity, we use Unknown here
                Some(IteratorInfo {
                    item_type: HirType::Int64, // Typically i64 or i32
                    is_into_iterator: true,
                })
            }

            // Array literals: [1, 2, 3]
            HirExpression::ArrayLiteral(_) => {
                // Array<T> when iterated yields &T when borrowed, T when moved
                // For now, assume borrowed iteration
                Some(IteratorInfo {
                    item_type: HirType::Unknown, // Would be Reference(T)
                    is_into_iterator: false,
                })
            }

            // Other expressions don't have obvious iterator traits
            _ => None,
        }
    }

    /// Get the loop variable type based on iterator analysis
    ///
    /// This combines iterator analysis with the collection's type to
    /// determine the actual type of the loop variable.
    pub fn get_loop_var_type(&self, iter_expr: &HirExpression) -> Option<HirType> {
        self.analyze_iterator(iter_expr)
            .map(|info| info.item_type)
    }

    /// Check if an iterator expression will consume the collection
    pub fn is_consuming_iterator(&self, iter_expr: &HirExpression) -> bool {
        self.analyze_iterator(iter_expr)
            .map(|info| info.is_into_iterator)
            .unwrap_or(false)
    }
}

/// Integration for borrow checking with iterator analysis
///
/// This trait allows the borrow checker to use iterator analysis
/// to properly determine loop variable ownership.
pub trait IteratorProvider {
    /// Analyze an iterator expression
    fn analyze_loop_iterator(&self, expr: &HirExpression) -> Option<IteratorInfo>;

    /// Get the type that the loop variable should have
    fn get_loop_variable_type(&self, expr: &HirExpression) -> Option<HirType>;

    /// Check if iterating consumes the collection
    fn consumes_collection(&self, expr: &HirExpression) -> bool;
}

impl IteratorProvider for IteratorAnalyzer {
    fn analyze_loop_iterator(&self, expr: &HirExpression) -> Option<IteratorInfo> {
        self.analyze_iterator(expr)
    }

    fn get_loop_variable_type(&self, expr: &HirExpression) -> Option<HirType> {
        self.get_loop_var_type(expr)
    }

    fn consumes_collection(&self, expr: &HirExpression) -> bool {
        self.is_consuming_iterator(expr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_iterator_analyzer() {
        let analyzer = IteratorAnalyzer::new();
        assert_eq!(analyzer.type_iterator_cache.len(), 0);
    }

    #[test]
    fn test_register_iterator_type() {
        let mut analyzer = IteratorAnalyzer::new();
        analyzer.register_iterator_type("Vec", HirType::Unknown, true);

        assert_eq!(analyzer.type_iterator_cache.len(), 1);
    }

    #[test]
    fn test_analyze_simple_variable() {
        let mut analyzer = IteratorAnalyzer::new();
        analyzer.register_iterator_type("my_vec", HirType::Int64, true);

        let expr = HirExpression::Variable("my_vec".to_string());
        let info = analyzer.analyze_iterator(&expr);

        assert!(info.is_some());
        assert!(info.unwrap().is_into_iterator);
    }

    #[test]
    fn test_analyze_unregistered_variable() {
        let analyzer = IteratorAnalyzer::new();
        let expr = HirExpression::Variable("unknown".to_string());
        let info = analyzer.analyze_iterator(&expr);

        assert!(info.is_none());
    }

    #[test]
    fn test_analyze_range_literal() {
        let analyzer = IteratorAnalyzer::new();
        let expr = HirExpression::Range {
            start: Some(Box::new(HirExpression::Integer(0))),
            end: Some(Box::new(HirExpression::Integer(10))),
            inclusive: false,
        };

        let info = analyzer.analyze_iterator(&expr);
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.item_type, HirType::Int64);
        assert!(info.is_into_iterator);
    }

    #[test]
    fn test_analyze_array_literal() {
        let analyzer = IteratorAnalyzer::new();
        let expr = HirExpression::ArrayLiteral(vec![
            HirExpression::Integer(1),
            HirExpression::Integer(2),
            HirExpression::Integer(3),
        ]);

        let info = analyzer.analyze_iterator(&expr);
        assert!(info.is_some());
    }

    #[test]
    fn test_iter_method_borrowed() {
        let mut analyzer = IteratorAnalyzer::new();
        analyzer.register_iterator_type("Vec", HirType::Int64, true);

        let expr = HirExpression::MethodCall {
            receiver: Box::new(HirExpression::Variable("my_vec".to_string())),
            method: "iter".to_string(),
            args: vec![],
        };

        let info = analyzer.analyze_iterator(&expr);
        assert!(info.is_some());
        assert!(!info.unwrap().is_into_iterator);
    }

    #[test]
    fn test_iter_mut_method_mutable() {
        let mut analyzer = IteratorAnalyzer::new();
        analyzer.register_iterator_type("Vec", HirType::Int64, true);

        let expr = HirExpression::MethodCall {
            receiver: Box::new(HirExpression::Variable("my_vec".to_string())),
            method: "iter_mut".to_string(),
            args: vec![],
        };

        let info = analyzer.analyze_iterator(&expr);
        assert!(info.is_some());
        assert!(!info.unwrap().is_into_iterator);
    }

    #[test]
    fn test_into_iter_method_consuming() {
        let mut analyzer = IteratorAnalyzer::new();
        analyzer.register_iterator_type("Vec", HirType::Int64, true);

        let expr = HirExpression::MethodCall {
            receiver: Box::new(HirExpression::Variable("my_vec".to_string())),
            method: "into_iter".to_string(),
            args: vec![],
        };

        let info = analyzer.analyze_iterator(&expr);
        assert!(info.is_some());
        assert!(info.unwrap().is_into_iterator);
    }

    #[test]
    fn test_field_access_iterator() {
        let mut analyzer = IteratorAnalyzer::new();
        // Register a field that itself is iterable
        analyzer.register_iterator_type("my_struct", HirType::Int64, false);

        let expr = HirExpression::FieldAccess {
            object: Box::new(HirExpression::Variable("my_struct".to_string())),
            field: "data".to_string(),
        };

        // This will recursively check the object (my_struct), which is registered
        let info = analyzer.analyze_iterator(&expr);
        assert!(info.is_some());
    }

    #[test]
    fn test_get_loop_variable_type() {
        let analyzer = IteratorAnalyzer::new();
        let expr = HirExpression::Range {
            start: Some(Box::new(HirExpression::Integer(0))),
            end: Some(Box::new(HirExpression::Integer(10))),
            inclusive: false,
        };

        let var_type = analyzer.get_loop_var_type(&expr);
        assert!(var_type.is_some());
        assert_eq!(var_type.unwrap(), HirType::Int64);
    }

    #[test]
    fn test_is_consuming_iterator_range() {
        let analyzer = IteratorAnalyzer::new();
        let expr = HirExpression::Range {
            start: Some(Box::new(HirExpression::Integer(0))),
            end: Some(Box::new(HirExpression::Integer(10))),
            inclusive: false,
        };

        assert!(analyzer.is_consuming_iterator(&expr));
    }

    #[test]
    fn test_iterator_provider_trait() {
        let mut analyzer = IteratorAnalyzer::new();
        analyzer.register_iterator_type("Vec", HirType::Int64, true);

        let expr = HirExpression::Variable("Vec".to_string());

        // Test through trait interface
        assert!(analyzer.analyze_loop_iterator(&expr).is_some());
        assert!(analyzer.get_loop_variable_type(&expr).is_some());
        assert!(analyzer.consumes_collection(&expr));
    }

    #[test]
    fn test_standard_types_registration() {
        let mut analyzer = IteratorAnalyzer::new();
        analyzer.register_standard_types();

        // Should have registered common types
        assert!(analyzer.type_iterator_cache.len() > 0);
    }
}
