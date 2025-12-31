//! # Phase 4E: Enhanced Error Reporting for Borrowchecker
//!
//! Provides sophisticated error messages with:
//! - Location information (file, line, column)
//! - Detailed suggestions for fixing violations
//! - Code examples showing correct patterns
//! - Categorized error types with specific handling

use crate::lowering::HirType;
use crate::utilities::Span;
use std::fmt;

/// Categorized error types for borrowchecker violations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorrowErrorKind {
    /// Used after move: `error[E0382]`
    ValueUsedAfterMove { variable: String },
    /// Cannot move borrowed value: `error[E0505]`
    CannotMoveBorrowed { variable: String },
    /// Multiple mutable borrows: `error[E0499]`
    MultipleMutableBorrows { variable: String },
    /// Mutable borrow while immutable exists: `error[E0502]`
    MutableWhileImmutable { variable: String },
    /// Cannot borrow moved value: `error[E0382]`
    CannotBorrowMoved { variable: String },
    /// Undefined variable: `error[E0425]`
    UndefinedVariable { variable: String },
    /// Cannot mutate immutable binding: `error[E0017]`
    CannotMutateImmutable { variable: String },
    /// Union field access requires unsafe: custom error
    UnionFieldAccessNotUnsafe { union_type: String, field: String },
    /// Iterator consumption not tracked: custom error
    IteratorConsumptionNotTracked { variable: String },
    /// Lifetime mismatch: custom error
    LifetimeMismatch { expected: String, found: String },
}

impl fmt::Display for BorrowErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BorrowErrorKind::ValueUsedAfterMove { variable } => {
                write!(f, "value used after move: `{}`", variable)
            }
            BorrowErrorKind::CannotMoveBorrowed { variable } => {
                write!(f, "cannot move out while borrowed: `{}`", variable)
            }
            BorrowErrorKind::MultipleMutableBorrows { variable } => {
                write!(f, "cannot create multiple mutable borrows: `{}`", variable)
            }
            BorrowErrorKind::MutableWhileImmutable { variable } => {
                write!(f, "cannot mutably borrow `{}` while immutably borrowed", variable)
            }
            BorrowErrorKind::CannotBorrowMoved { variable } => {
                write!(f, "cannot borrow moved value: `{}`", variable)
            }
            BorrowErrorKind::UndefinedVariable { variable } => {
                write!(f, "undefined variable: `{}`", variable)
            }
            BorrowErrorKind::CannotMutateImmutable { variable } => {
                write!(f, "cannot mutably borrow immutable variable: `{}`", variable)
            }
            BorrowErrorKind::UnionFieldAccessNotUnsafe { union_type, field } => {
                write!(f, "union field access on `{}` requires unsafe block (field: `{}`)", union_type, field)
            }
            BorrowErrorKind::IteratorConsumptionNotTracked { variable } => {
                write!(f, "iterator consumption of `{}` not tracked", variable)
            }
            BorrowErrorKind::LifetimeMismatch { expected, found } => {
                write!(f, "lifetime mismatch: expected `{}`, found `{}`", expected, found)
            }
        }
    }
}

/// Enhanced borrow error with detailed diagnostics
#[derive(Debug, Clone)]
pub struct EnhancedBorrowError {
    /// Error kind and primary information
    pub kind: BorrowErrorKind,
    /// Error code (E0382, E0505, etc.)
    pub code: Option<String>,
    /// Location in source code
    pub location: Option<Span>,
    /// Detailed explanation of the error
    pub explanation: String,
    /// How to fix this error
    pub suggestion: String,
    /// Code example showing correct pattern
    pub example: Option<String>,
    /// Type information (if relevant)
    pub type_info: Option<String>,
}

impl EnhancedBorrowError {
    /// Create a new enhanced error from a kind
    pub fn from_kind(kind: BorrowErrorKind) -> Self {
        let (explanation, suggestion, example, code) = match &kind {
            BorrowErrorKind::ValueUsedAfterMove { variable } => (
                format!(
                    "The variable `{}` was moved to another location and cannot be used here. \
                     Once a value is moved, ownership transfers and the original binding is no longer valid.",
                    variable
                ),
                format!(
                    "Consider cloning the value before moving it, or restructure your code to use \
                     the value in a way that doesn't require multiple uses. If you need to use `{}` \
                     after moving it, clone it: `{}.clone()`",
                    variable, variable
                ),
                Some(format!(
                    "// Before (error):\nlet x = vec![1, 2, 3];\nlet y = x;  // Move\nprintln!(\"{{:?}}\", x);  // Error: x was moved\n\n\
                     // After (fixed):\nlet x = vec![1, 2, 3];\nlet y = x.clone();  // Clone instead\nprintln!(\"{{:?}}\", x);  // OK"
                )),
                Some("E0382".to_string()),
            ),
            BorrowErrorKind::CannotMoveBorrowed { variable } => (
                format!(
                    "The variable `{}` is currently borrowed and cannot be moved. \
                     You have active references to this value that must outlive any move.",
                    variable
                ),
                format!(
                    "Drop the borrow before moving the value. If you need the value and a reference, \
                     consider restructuring to avoid the simultaneous borrow and move."
                ),
                Some(format!(
                    "// Before (error):\nlet x = vec![1, 2, 3];\nlet r = &x;\nlet y = x;  // Error: x is borrowed\n\n\
                     // After (fixed):\nlet x = vec![1, 2, 3];\nlet y = x;  // Move first\nlet r = &y;  // Then borrow from y"
                )),
                Some("E0505".to_string()),
            ),
            BorrowErrorKind::MultipleMutableBorrows { variable } => (
                format!(
                    "You cannot have multiple mutable borrows of `{}` at the same time. \
                     Only one mutable reference can exist at a time to maintain memory safety.",
                    variable
                ),
                format!(
                    "Drop the first mutable borrow before creating another. Use scoping {} to limit borrow duration: {{\n  \
                     let r1 = &mut {};\n  // use r1\n}} // r1 dropped here\nlet r2 = &mut {};  // OK",
                    "{ }", variable, variable
                ),
                Some(format!(
                    "// Before (error):\nlet mut x = 5;\nlet r1 = &mut x;\nlet r2 = &mut x;  // Error: multiple mutable borrows\n\n\
                     // After (fixed):\nlet mut x = 5;\nlet r1 = &mut x;\n// use r1\ndrop(r1);  // Explicitly drop\nlet r2 = &mut x;  // OK"
                )),
                Some("E0499".to_string()),
            ),
            BorrowErrorKind::MutableWhileImmutable { variable } => (
                format!(
                    "Cannot create a mutable borrow of `{}` while an immutable borrow exists. \
                     You must drop all immutable borrows before creating a mutable one.",
                    variable
                ),
                format!(
                    "Reorder your code to drop the immutable borrow first, or use a smaller scope for it."
                ),
                Some(format!(
                    "// Before (error):\nlet mut x = 5;\nlet r1 = &x;      // immutable borrow\nlet r2 = &mut x;  // Error: mutable while immutable\n\n\
                     // After (fixed):\nlet mut x = 5;\nlet r1 = &x;\nprintln!(\"{{:?}}\", r1);  // use r1\ndrop(r1);  // drop immutable\nlet r2 = &mut x;  // OK: now mutable"
                )),
                Some("E0502".to_string()),
            ),
            BorrowErrorKind::CannotBorrowMoved { variable } => (
                format!(
                    "The variable `{}` was moved and cannot be borrowed. \
                     Once a value is moved, you cannot create references to it.",
                    variable
                ),
                format!(
                    "Either don't move the value, or use a reference instead of moving. \
                     If you need the value after using a reference, consider not moving in the first place."
                ),
                Some(format!(
                    "// Before (error):\nlet x = String::from(\"hello\");\nlet y = x;  // Move\nlet r = &x;  // Error: x was moved\n\n\
                     // After (fixed):\nlet x = String::from(\"hello\");\nlet r = &x;  // Borrow instead\nlet y = x;  // Move happens here\ndrop(y);"
                )),
                Some("E0382".to_string()),
            ),
            BorrowErrorKind::UndefinedVariable { variable } => (
                format!(
                    "The variable `{}` is not defined in this scope. \
                     Make sure it's declared with `let` or `let mut` before use.",
                    variable
                ),
                format!(
                    "Declare `{}` with `let` or `let mut` before using it.", variable
                ),
                Some(format!(
                    "// Before (error):\nprintln!(\"{{:?}}\", x);  // Error: x not defined\n\n\
                     // After (fixed):\nlet x = 42;\nprintln!(\"{{:?}}\", x);  // OK"
                )),
                Some("E0425".to_string()),
            ),
            BorrowErrorKind::CannotMutateImmutable { variable } => (
                format!(
                    "Cannot mutably borrow `{}` because it was declared as immutable. \
                     Add `mut` keyword to make it mutable.",
                    variable
                ),
                format!(
                    "Declare `{}` as mutable with `let mut` instead of `let`.", variable
                ),
                Some(format!(
                    "// Before (error):\nlet x = 5;\nlet r = &mut x;  // Error: x is not mut\n\n\
                     // After (fixed):\nlet mut x = 5;\nlet r = &mut x;  // OK"
                )),
                Some("E0017".to_string()),
            ),
            BorrowErrorKind::UnionFieldAccessNotUnsafe { union_type, field } => (
                format!(
                    "Accessing field `{}` on union type `{}` requires an `unsafe` block. \
                     Union field access is inherently unsafe because the union layout is unspecified.",
                    field, union_type
                ),
                format!(
                    "Wrap the field access in an `unsafe {{ ... }}` block. \
                     Be sure you know which variant is active before accessing the field."
                ),
                Some(format!(
                    "// Before (error):\nunion Data {{\n    i: i32,\n    f: f64,\n}}\nlet d = Data {{ i: 42 }};\nlet x = d.i;  // Error: requires unsafe\n\n\
                     // After (fixed):\nunion Data {{\n    i: i32,\n    f: f64,\n}}\nlet d = Data {{ i: 42 }};\nlet x = unsafe {{ d.i }};  // OK"
                )),
                None,
            ),
            BorrowErrorKind::IteratorConsumptionNotTracked { variable } => (
                format!(
                    "The iterator from `{}` is consumed by `.into_iter()`, moving ownership. \
                     After using `.into_iter()`, the original collection is no longer accessible.",
                    variable
                ),
                format!(
                    "Either use `.iter()` to borrow, `.iter_mut()` for mutable borrow, \
                     or accept that the collection is consumed."
                ),
                Some(format!(
                    "// Before (warning):\nlet v = vec![1, 2, 3];\nfor x in v.into_iter() {{\n    println!(\"{{:?}}\", x);\n}}\nprintln!(\"{{:?}}\", v);  // Error: v consumed\n\n\
                     // After (fixed - borrow):\nlet v = vec![1, 2, 3];\nfor x in v.iter() {{\n    println!(\"{{:?}}\", x);\n}}\nprintln!(\"{{:?}}\", v);  // OK: v still accessible"
                )),
                None,
            ),
            BorrowErrorKind::LifetimeMismatch { expected, found } => (
                format!(
                    "Lifetime mismatch: expected lifetime `{}`, but found `{}`. \
                     The lifetimes don't match what's required by the function signature.",
                    expected, found
                ),
                format!(
                    "Adjust the lifetime of your reference to match the expected lifetime. \
                     This often requires restructuring when the reference is created or used."
                ),
                None,
                None,
            ),
        };

        EnhancedBorrowError {
            kind,
            code,
            location: None,
            explanation,
            suggestion,
            example,
            type_info: None,
        }
    }

    /// Add location information
    pub fn with_location(mut self, location: Span) -> Self {
        self.location = Some(location);
        self
    }

    /// Add type information
    pub fn with_type_info(mut self, ty: &HirType) -> Self {
        self.type_info = Some(format!("{:?}", ty));
        self
    }

    /// Format the error for display
    pub fn format_detailed(&self) -> String {
        let mut output = String::new();

        // Error code and message
        if let Some(code) = &self.code {
            output.push_str(&format!("error[{}]: {}\n", code, self.kind));
        } else {
            output.push_str(&format!("error: {}\n", self.kind));
        }

        // Location
        if let Some(loc) = &self.location {
            output.push_str(&format!("  --> {}:{}\n", 
                loc.line,
                loc.column + 1  // Convert to 1-indexed
            ));
        }

        output.push('\n');

        // Explanation
        output.push_str("explanation:\n");
        for line in self.explanation.lines() {
            output.push_str(&format!("  {}\n", line));
        }

        output.push('\n');

        // Suggestion
        output.push_str("suggestion:\n");
        for line in self.suggestion.lines() {
            output.push_str(&format!("  {}\n", line));
        }

        // Example
        if let Some(example) = &self.example {
            output.push_str("\nexample:\n");
            for line in example.lines() {
                output.push_str(&format!("  {}\n", line));
            }
        }

        // Type info
        if let Some(info) = &self.type_info {
            output.push_str(&format!("\ntype information: {}\n", info));
        }

        output
    }
}

impl fmt::Display for EnhancedBorrowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.format_detailed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_used_after_move_error() {
        let error = EnhancedBorrowError::from_kind(
            BorrowErrorKind::ValueUsedAfterMove {
                variable: "x".to_string(),
            }
        );

        assert_eq!(error.code, Some("E0382".to_string()));
        assert!(error.explanation.contains("moved"));
        assert!(error.suggestion.contains("clone"));
        assert!(error.example.is_some());
    }

    #[test]
    fn test_cannot_move_borrowed_error() {
        let error = EnhancedBorrowError::from_kind(
            BorrowErrorKind::CannotMoveBorrowed {
                variable: "vec".to_string(),
            }
        );

        assert_eq!(error.code, Some("E0505".to_string()));
        assert!(error.explanation.contains("borrowed"));
        assert!(error.suggestion.contains("Drop"));
    }

    #[test]
    fn test_multiple_mutable_borrows_error() {
        let error = EnhancedBorrowError::from_kind(
            BorrowErrorKind::MultipleMutableBorrows {
                variable: "data".to_string(),
            }
        );

        assert_eq!(error.code, Some("E0499".to_string()));
        assert!(error.explanation.contains("multiple"));
        assert!(error.example.is_some());
    }

    #[test]
    fn test_union_field_access_error() {
        let error = EnhancedBorrowError::from_kind(
            BorrowErrorKind::UnionFieldAccessNotUnsafe {
                union_type: "Data".to_string(),
                field: "value".to_string(),
            }
        );

        assert!(error.explanation.contains("union"));
        assert!(error.suggestion.contains("unsafe"));
        assert!(error.example.is_some());
    }

    #[test]
    fn test_error_with_location() {
        let error = EnhancedBorrowError::from_kind(
            BorrowErrorKind::UndefinedVariable {
                variable: "unknown".to_string(),
            }
        );

        assert!(error.location.is_none());
        
        // Adding location would require Span implementation
        // This is just testing the structure
    }

    #[test]
    fn test_detailed_format_output() {
        let error = EnhancedBorrowError::from_kind(
            BorrowErrorKind::CannotMutateImmutable {
                variable: "x".to_string(),
            }
        );

        let formatted = error.format_detailed();
        assert!(formatted.contains("error"));
        assert!(formatted.contains("explanation"));
        assert!(formatted.contains("suggestion"));
        assert!(formatted.contains("example"));
    }

    #[test]
    fn test_iterator_consumption_error() {
        let error = EnhancedBorrowError::from_kind(
            BorrowErrorKind::IteratorConsumptionNotTracked {
                variable: "v".to_string(),
            }
        );

        assert!(error.explanation.contains("consumed"));
        assert!(error.suggestion.contains("iter()"));
        assert!(error.example.is_some());
    }

    #[test]
    fn test_lifetime_mismatch_error() {
        let error = EnhancedBorrowError::from_kind(
            BorrowErrorKind::LifetimeMismatch {
                expected: "'a".to_string(),
                found: "'b".to_string(),
            }
        );

        assert!(error.explanation.contains("Lifetime"));
        assert!(error.suggestion.contains("lifetime"));
    }

    #[test]
    fn test_display_trait() {
        let error = EnhancedBorrowError::from_kind(
            BorrowErrorKind::ValueUsedAfterMove {
                variable: "x".to_string(),
            }
        );

        let display_str = format!("{}", error);
        assert!(display_str.contains("error"));
        assert!(display_str.contains("used after move"));
    }
}
