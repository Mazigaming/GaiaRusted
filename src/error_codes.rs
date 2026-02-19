//! GaiaRusted Error Codes (E001-E100)
//! 
//! Each error in the compiler is assigned a unique error code that:
//! - Identifies the error category and severity
//! - Provides a stable reference for documentation
//! - Enables programmatic error handling
//! - Links to helpful suggestions and examples

use std::collections::HashMap;
use std::sync::OnceLock;

/// Error code structure
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ErrorCode {
    /// Unique code (E001-E100)
    pub code: String,
    /// Error category (Syntax, Type, Borrow, etc)
    pub category: ErrorCategory,
    /// Severity level
    pub severity: Severity,
    /// Short title
    pub title: &'static str,
    /// Default error message
    pub message: &'static str,
    /// Helpful suggestion
    pub suggestion: &'static str,
    /// Example of the error
    pub example: &'static str,
    /// How to fix it
    pub fix: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    Syntax,           // E001-E010
    Parsing,          // E011-E020
    Type,             // E021-E040
    Borrow,           // E041-E060
    Trait,            // E061-E070
    Lifetime,         // E071-E080
    Unimplemented,    // E081-E090
    Internal,         // E091-E100
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

// ============================================================================
// ERROR CODE DEFINITIONS (E001-E100)
// ============================================================================

static ERROR_CODES: OnceLock<HashMap<&'static str, ErrorCode>> = OnceLock::new();

fn get_error_codes_map() -> &'static HashMap<&'static str, ErrorCode> {
    ERROR_CODES.get_or_init(|| {
        let mut m = HashMap::new();

        // ========== SYNTAX ERRORS (E001-E010) ==========
        m.insert("E001", ErrorCode {
            code: "E001".to_string(),
            category: ErrorCategory::Syntax,
            severity: Severity::Error,
            title: "Unexpected token",
            message: "Found an unexpected token in the source code",
            suggestion: "Check the syntax around this location. Did you forget a semicolon, comma, or bracket?",
            example: "let x = 5  // Missing semicolon\nlet y = 10;",
            fix: "let x = 5;  // Add semicolon\nlet y = 10;",
        });

        m.insert("E002", ErrorCode {
            code: "E002".to_string(),
            category: ErrorCategory::Syntax,
            severity: Severity::Error,
            title: "Unmatched bracket",
            message: "Unmatched opening bracket - closing bracket not found",
            suggestion: "Check that all opening brackets '(', '[', '{' have matching closing brackets",
            example: "fn foo() {\n    let x = [1, 2, 3;\n}  // Missing ]",
            fix: "fn foo() {\n    let x = [1, 2, 3];\n}",
        });

        m.insert("E003", ErrorCode {
            code: "E003".to_string(),
            category: ErrorCategory::Syntax,
            severity: Severity::Error,
            title: "Expected expression",
            message: "Expected an expression but found something else",
            suggestion: "Make sure you have a valid expression in this context",
            example: "let x =;  // Missing value",
            fix: "let x = 5;  // Provide a value",
        });

        m.insert("E004", ErrorCode {
            code: "E004".to_string(),
            category: ErrorCategory::Syntax,
            severity: Severity::Error,
            title: "Invalid assignment",
            message: "Invalid assignment target",
            suggestion: "You can only assign to variables, fields, or array elements. Try assigning to a valid location.",
            example: "5 = x;  // Cannot assign to literal",
            fix: "x = 5;  // Assign to variable instead",
        });

        m.insert("E005", ErrorCode {
            code: "E005".to_string(),
            category: ErrorCategory::Syntax,
            severity: Severity::Error,
            title: "Missing function body",
            message: "Function declaration missing body",
            suggestion: "All functions must have a body enclosed in braces '{ }'",
            example: "fn add(x: i32, y: i32) -> i32;",
            fix: "fn add(x: i32, y: i32) -> i32 {\n    x + y\n}",
        });

        m.insert("E006", ErrorCode {
            code: "E006".to_string(),
            category: ErrorCategory::Syntax,
            severity: Severity::Error,
            title: "Invalid type annotation",
            message: "Invalid type annotation syntax",
            suggestion: "Use ':' to annotate types, like 'let x: i32 = 5;'",
            example: "let x i32 = 5;  // Missing :",
            fix: "let x: i32 = 5;",
        });

        m.insert("E007", ErrorCode {
            code: "E007".to_string(),
            category: ErrorCategory::Syntax,
            severity: Severity::Error,
            title: "Missing comma in list",
            message: "Missing comma between list elements",
            suggestion: "Separate items in arrays, tuples, or function arguments with commas",
            example: "let x = vec![1 2 3];  // Missing commas",
            fix: "let x = vec![1, 2, 3];",
        });

        m.insert("E008", ErrorCode {
            code: "E008".to_string(),
            category: ErrorCategory::Syntax,
            severity: Severity::Error,
            title: "Invalid struct literal",
            message: "Invalid syntax in struct literal",
            suggestion: "Use 'Name { field: value, ... }' syntax for struct literals",
            example: "let p = Point(1, 2);  // Missing field names",
            fix: "let p = Point { x: 1, y: 2 };",
        });

        m.insert("E009", ErrorCode {
            code: "E009".to_string(),
            category: ErrorCategory::Syntax,
            severity: Severity::Error,
            title: "Invalid pattern",
            message: "Invalid pattern in match or destructuring",
            suggestion: "Patterns must be valid expressions that can be destructured",
            example: "match x {\n    5 + 1 => { }  // Cannot add in pattern\n}",
            fix: "match x {\n    6 => { }\n}",
        });

        m.insert("E010", ErrorCode {
            code: "E010".to_string(),
            category: ErrorCategory::Syntax,
            severity: Severity::Error,
            title: "Unexpected end of file",
            message: "Unexpected end of file - missing closing bracket or statement",
            suggestion: "Check that all blocks are properly closed with '}'",
            example: "fn foo() {\n    let x = 5;\n// File ends here, missing }",
            fix: "fn foo() {\n    let x = 5;\n}  // Add closing brace",
        });

        // ========== PARSING ERRORS (E011-E020) ==========
        m.insert("E011", ErrorCode {
            code: "E011".to_string(),
            category: ErrorCategory::Parsing,
            severity: Severity::Error,
            title: "Invalid identifier",
            message: "Invalid identifier name",
            suggestion: "Identifiers must start with a letter or underscore, followed by letters, digits, or underscores",
            example: "let 2x = 5;  // Cannot start with digit",
            fix: "let x2 = 5;",
        });

        m.insert("E012", ErrorCode {
            code: "E012".to_string(),
            category: ErrorCategory::Parsing,
            severity: Severity::Error,
            title: "Invalid number literal",
            message: "Invalid number literal format",
            suggestion: "Numbers must be properly formatted: integers (42), floats (3.14), hex (0xFF), binary (0b1010)",
            example: "let x = 42.42.42;  // Multiple decimal points",
            fix: "let x = 42.42;",
        });

        m.insert("E013", ErrorCode {
            code: "E013".to_string(),
            category: ErrorCategory::Parsing,
            severity: Severity::Error,
            title: "Invalid string literal",
            message: "Invalid string literal - unterminated or invalid escape sequence",
            suggestion: "String literals must be enclosed in double quotes and properly escaped",
            example: "let s = \"hello\\nworld;  // Missing closing quote",
            fix: "let s = \"hello\\nworld\";",
        });

        m.insert("E014", ErrorCode {
            code: "E014".to_string(),
            category: ErrorCategory::Parsing,
            severity: Severity::Error,
            title: "Invalid character literal",
            message: "Invalid character literal - must be single character",
            suggestion: "Character literals use single quotes and must contain exactly one character",
            example: "let c = 'hello';  // Too many characters",
            fix: "let c = 'h';",
        });

        m.insert("E015", ErrorCode {
            code: "E015".to_string(),
            category: ErrorCategory::Parsing,
            severity: Severity::Error,
            title: "Invalid operator",
            message: "Invalid or unexpected operator",
            suggestion: "Use valid Rust operators: +, -, *, /, %, ==, !=, <, >, <=, >=, &&, ||, &, |, ^, <<, >>",
            example: "let x = 5 @@ 3;  // Invalid operator",
            fix: "let x = 5 + 3;",
        });

        m.insert("E016", ErrorCode {
            code: "E016".to_string(),
            category: ErrorCategory::Parsing,
            severity: Severity::Error,
            title: "Missing type parameter",
            message: "Generic type parameter is missing",
            suggestion: "Provide type parameters for generic types: Vec<i32>, Option<String>, etc.",
            example: "let x: Vec = vec![1, 2, 3];  // Missing type parameter",
            fix: "let x: Vec<i32> = vec![1, 2, 3];",
        });

        m.insert("E017", ErrorCode {
            code: "E017".to_string(),
            category: ErrorCategory::Parsing,
            severity: Severity::Error,
            title: "Invalid lifetime syntax",
            message: "Invalid lifetime parameter syntax",
            suggestion: "Lifetimes use single quotes: 'a, 'static, 'static",
            example: "fn foo<a>(x: &a i32) { }  // Missing quote before lifetime",
            fix: "fn foo<'a>(x: &'a i32) { }",
        });

        m.insert("E018", ErrorCode {
            code: "E018".to_string(),
            category: ErrorCategory::Parsing,
            severity: Severity::Error,
            title: "Invalid attribute syntax",
            message: "Invalid attribute or decorator syntax",
            suggestion: "Attributes use #[...] syntax: #[derive(Debug)], #[test], etc.",
            example: "#derive(Debug) struct Point { }  // Missing brackets",
            fix: "#[derive(Debug)] struct Point { }",
        });

        m.insert("E019", ErrorCode {
            code: "E019".to_string(),
            category: ErrorCategory::Parsing,
            severity: Severity::Error,
            title: "Invalid macro invocation",
            message: "Invalid macro invocation syntax",
            suggestion: "Macros are invoked with '!' after the name: println!(\"...\"), vec![...]",
            example: "println \"hello\";  // Missing !",
            fix: "println!(\"hello\");",
        });

        m.insert("E020", ErrorCode {
            code: "E020".to_string(),
            category: ErrorCategory::Parsing,
            severity: Severity::Error,
            title: "Invalid visibility modifier",
            message: "Invalid visibility modifier",
            suggestion: "Use 'pub' for public, 'pub(crate)' for crate-private, or nothing for private",
            example: "private fn foo() { }  // Invalid modifier",
            fix: "fn foo() { }  // Private by default",
        });

        // ========== TYPE ERRORS (E021-E040) ==========
        m.insert("E021", ErrorCode {
            code: "E021".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Type mismatch",
            message: "Type mismatch in assignment or function argument",
            suggestion: "The value type doesn't match the expected type. Check the types involved.",
            example: "let x: i32 = \"hello\";  // String cannot be assigned to i32",
            fix: "let x: String = \"hello\";  // Match types",
        });

        m.insert("E022", ErrorCode {
            code: "E022".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Undefined variable",
            message: "Variable is not defined in this scope",
            suggestion: "Make sure the variable is declared before use, and you're using the correct name",
            example: "println!(\"{}\", x);  // x not defined",
            fix: "let x = 5;\nprintln!(\"{}\", x);",
        });

        m.insert("E023", ErrorCode {
            code: "E023".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Undefined type",
            message: "Type is not defined in this scope",
            suggestion: "Make sure the type is imported or defined. Use 'use' to import types.",
            example: "let x: HashMap<i32, String>;  // HashMap not imported",
            fix: "use std::collections::HashMap;\nlet x: HashMap<i32, String>;",
        });

        m.insert("E024", ErrorCode {
            code: "E024".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Wrong number of arguments",
            message: "Function called with wrong number of arguments",
            suggestion: "Check the function signature and provide the correct number of arguments",
            example: "fn add(x: i32, y: i32) -> i32 { x + y }\nadd(5);  // Missing argument",
            fix: "add(5, 3);",
        });

        m.insert("E025", ErrorCode {
            code: "E025".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "No such method",
            message: "Type doesn't have the requested method",
            suggestion: "Check the type and available methods. Use .method() syntax correctly.",
            example: "let x = 5;\nx.push(10);  // i32 doesn't have push method",
            fix: "let mut v = vec![5];\nv.push(10);",
        });

        m.insert("E026", ErrorCode {
            code: "E026".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Incompatible types",
            message: "Types cannot be used together in this operation",
            suggestion: "The types are not compatible for this operation. Convert one to match the other.",
            example: "let x = 5 + \"hello\";  // Cannot add i32 and String",
            fix: "let x = 5 + 3;  // Use compatible types",
        });

        m.insert("E027", ErrorCode {
            code: "E027".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Not callable",
            message: "Value is not callable as a function",
            suggestion: "Only functions and function pointers can be called. Try using a different value.",
            example: "let x = 5;\nx();  // Cannot call integer",
            fix: "fn foo() { }\nfoo();  // Call a function instead",
        });

        m.insert("E028", ErrorCode {
            code: "E028".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Not indexable",
            message: "Type cannot be indexed",
            suggestion: "Only arrays, slices, strings, and maps can be indexed. Use bracket notation [].",
            example: "let x = 5;\nlet y = x[0];  // Cannot index i32",
            fix: "let arr = [1, 2, 3];\nlet y = arr[0];",
        });

        m.insert("E029", ErrorCode {
            code: "E029".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Missing field in struct",
            message: "Struct field is missing in literal",
            suggestion: "All non-defaulted fields must be provided in struct literals",
            example: "struct Point { x: i32, y: i32 }\nlet p = Point { x: 1 };  // Missing y",
            fix: "let p = Point { x: 1, y: 2 };",
        });

        m.insert("E030", ErrorCode {
            code: "E030".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Unknown field",
            message: "Struct doesn't have the specified field",
            suggestion: "Check the struct definition for available fields",
            example: "struct Point { x: i32, y: i32 }\nlet p = Point { x: 1, z: 2 };  // No z field",
            fix: "let p = Point { x: 1, y: 2 };",
        });

        m.insert("E031", ErrorCode {
            code: "E031".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Type parameter not found",
            message: "Generic type parameter is not in scope",
            suggestion: "Declare the type parameter in angle brackets: <T>, <T, U>",
            example: "fn foo(x: T) { }  // T not declared",
            fix: "fn foo<T>(x: T) { }",
        });

        m.insert("E032", ErrorCode {
            code: "E032".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Type instantiation error",
            message: "Error instantiating generic type",
            suggestion: "Provide all required type arguments or let type inference fill them in",
            example: "let x: Vec = vec![1, 2, 3];  // Need type parameter",
            fix: "let x: Vec<i32> = vec![1, 2, 3];  // Provide <i32>",
        });

        m.insert("E033", ErrorCode {
            code: "E033".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Trait not implemented",
            message: "Type doesn't implement required trait",
            suggestion: "Implement the trait for the type, or use a different type",
            example: "fn print<T: std::fmt::Display>(x: T) { }\nprint(vec![1, 2]);  // Vec doesn't implement Display",
            fix: "print(5);  // i32 implements Display",
        });

        m.insert("E034", ErrorCode {
            code: "E034".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Unsupported type operation",
            message: "This operation is not supported for this type",
            suggestion: "Try using a type that supports this operation",
            example: "let b = true;\nlet c = !b && 5;  // Cannot mix bool and int in &&",
            fix: "let c = !b;  // Just negate the bool",
        });

        m.insert("E035", ErrorCode {
            code: "E035".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Invalid cast",
            message: "Invalid type cast",
            suggestion: "Check that the types can be cast. Use 'as' for valid casts.",
            example: "let x = \"hello\" as i32;  // Cannot cast String to i32",
            fix: "let x = 5 as i32;  // Valid cast",
        });

        m.insert("E036", ErrorCode {
            code: "E036".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Condition must be bool",
            message: "Condition must be boolean type",
            suggestion: "Use a boolean expression in if/while conditions",
            example: "if 5 { }  // Should be boolean",
            fix: "if 5 > 0 { }  // Use boolean comparison",
        });

        m.insert("E037", ErrorCode {
            code: "E037".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Missing return type",
            message: "Function is missing a return type annotation",
            suggestion: "Specify return type with -> syntax: fn foo() -> i32 { }",
            example: "fn foo() { 42 }  // Missing return type",
            fix: "fn foo() -> i32 { 42 }",
        });

        m.insert("E038", ErrorCode {
            code: "E038".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Unexpected return value",
            message: "Function returns a value but shouldn't, or vice versa",
            suggestion: "Check the function signature. Remove the return value or add a return type.",
            example: "fn foo() {\n    42  // Returns value but fn foo() has no return type\n}",
            fix: "fn foo() -> i32 {\n    42\n}",
        });

        m.insert("E039", ErrorCode {
            code: "E039".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Pattern doesn't cover all cases",
            message: "Match pattern is not exhaustive",
            suggestion: "Add missing pattern cases or use _ to catch all remaining cases",
            example: "match x {\n    1 => { }\n    2 => { }\n}  // Doesn't cover other values",
            fix: "match x {\n    1 => { }\n    2 => { }\n    _ => { }\n}",
        });

        m.insert("E040", ErrorCode {
            code: "E040".to_string(),
            category: ErrorCategory::Type,
            severity: Severity::Error,
            title: "Array length mismatch",
            message: "Array has wrong number of elements",
            suggestion: "Provide the correct number of elements or use a Vec instead",
            example: "let arr: [i32; 3] = [1, 2];  // Expected 3 elements, got 2",
            fix: "let arr: [i32; 3] = [1, 2, 3];",
        });

        // ========== BORROW CHECKING ERRORS (E041-E060) ==========
        m.insert("E041", ErrorCode {
            code: "E041".to_string(),
            category: ErrorCategory::Borrow,
            severity: Severity::Error,
            title: "Value moved",
            message: "Value was moved and can no longer be used",
            suggestion: "The value has been moved to another variable. Either use it before the move or clone it.",
            example: "let x = vec![1, 2, 3];\nlet y = x;\nprintln!(\"{:?}\", x);  // Error: x was moved",
            fix: "let x = vec![1, 2, 3];\nlet y = &x;  // Borrow instead of move\nprintln!(\"{:?}\", x);",
        });

        m.insert("E042", ErrorCode {
            code: "E042".to_string(),
            category: ErrorCategory::Borrow,
            severity: Severity::Error,
            title: "Already borrowed",
            message: "Value is already borrowed",
            suggestion: "You have an existing borrow of this value. Drop that borrow or use a shared borrow.",
            example: "let mut x = 5;\nlet r1 = &mut x;\nlet r2 = &mut x;  // Error: already borrowed",
            fix: "let mut x = 5;\nlet r1 = &mut x;\n// Use r1 then let it go out of scope\nlet r2 = &mut x;",
        });

        m.insert("E043", ErrorCode {
            code: "E043".to_string(),
            category: ErrorCategory::Borrow,
            severity: Severity::Error,
            title: "Multiple mutable borrows",
            message: "Cannot have multiple mutable borrows of the same value",
            suggestion: "Only one mutable borrow is allowed at a time. Use immutable borrows or sequential borrows.",
            example: "let mut x = 5;\nlet r1 = &mut x;\nlet r2 = &mut x;  // Error",
            fix: "let mut x = 5;\nlet r1 = &mut x;\nlet r2 = &mut x;  // r1 goes out of scope first",
        });

        m.insert("E044", ErrorCode {
            code: "E044".to_string(),
            category: ErrorCategory::Borrow,
            severity: Severity::Error,
            title: "Cannot mutate through immutable borrow",
            message: "Cannot modify a value through an immutable borrow",
            suggestion: "Either use a mutable borrow from the start, or drop the immutable borrow",
            example: "let mut x = 5;\nlet r = &x;\n*r = 10;  // Error: r is immutable",
            fix: "let mut x = 5;\nlet r = &mut x;\n*r = 10;",
        });

        m.insert("E045", ErrorCode {
            code: "E045".to_string(),
            category: ErrorCategory::Borrow,
            severity: Severity::Error,
            title: "Mutable borrow while immutable exists",
            message: "Cannot create mutable borrow while immutable borrow exists",
            suggestion: "Drop the immutable borrow before creating a mutable one",
            example: "let mut x = 5;\nlet r1 = &x;\nlet r2 = &mut x;  // Error",
            fix: "let mut x = 5;\nlet r1 = &x;\n// r1 scope ends\nlet r2 = &mut x;",
        });

        m.insert("E046", ErrorCode {
            code: "E046".to_string(),
            category: ErrorCategory::Borrow,
            severity: Severity::Error,
            title: "Cannot borrow as mutable",
            message: "Cannot create mutable borrow of non-mutable binding",
            suggestion: "Declare the binding as 'mut' to allow mutable borrowing",
            example: "let x = 5;\nlet r = &mut x;  // Error: x is not mut",
            fix: "let mut x = 5;\nlet r = &mut x;",
        });

        m.insert("E047", ErrorCode {
            code: "E047".to_string(),
            category: ErrorCategory::Borrow,
            severity: Severity::Error,
            title: "Dangling reference",
            message: "Reference outlives the value it borrows",
            suggestion: "Ensure the value lives as long as the reference. Check lifetime bounds.",
            example: "fn foo<'a>() -> &'a i32 {\n    let x = 5;\n    &x  // Error: x doesn't live long enough\n}",
            fix: "fn foo() -> i32 {\n    let x = 5;\n    x  // Return by value\n}",
        });

        m.insert("E048", ErrorCode {
            code: "E048".to_string(),
            category: ErrorCategory::Borrow,
            severity: Severity::Error,
            title: "Borrow after move",
            message: "Cannot borrow value after it has been moved",
            suggestion: "The value has been consumed. Try borrowing before the move instead.",
            example: "let x = vec![1, 2];\nlet y = x;\nlet r = &x;  // Error: x moved to y",
            fix: "let x = vec![1, 2];\nlet r = &x;\nlet y = x;",
        });

        m.insert("E049", ErrorCode {
            code: "E049".to_string(),
            category: ErrorCategory::Borrow,
            severity: Severity::Error,
            title: "Use after drop",
            message: "Value used after being dropped",
            suggestion: "The value has been consumed/dropped. Use it before the drop.",
            example: "let x = String::from(\"hi\");\nlet y = x;\ndrop(y);\nprintln!(\"{}\", x);  // Error",
            fix: "let x = String::from(\"hi\");\nprintln!(\"{}\", x);  // Use before drop",
        });

        m.insert("E050", ErrorCode {
            code: "E050".to_string(),
            category: ErrorCategory::Borrow,
            severity: Severity::Error,
            title: "Illegal move",
            message: "Value cannot be moved because it's borrowed",
            suggestion: "Drop the borrow first, or use references instead",
            example: "let mut x = 5;\nlet r = &mut x;\nlet y = x;  // Error: x is borrowed",
            fix: "let mut x = 5;\nlet r = &mut x;\n// r scope ends\nlet y = x;",
        });

        // ... Continue with more error codes (E051-E100 for space)
        // For brevity, I'll add placeholders for the rest

        m.insert("E051", ErrorCode {
            code: "E051".to_string(),
            category: ErrorCategory::Borrow,
            severity: Severity::Error,
            title: "Invalid lifetime",
            message: "Invalid lifetime reference",
            suggestion: "Check the lifetime is properly declared and used",
            example: "fn foo<'a>(&self, x: &'b i32) { }  // 'b not declared",
            fix: "fn foo<'a, 'b>(&'a self, x: &'b i32) { }",
        });

        m.insert("E052", ErrorCode {
            code: "E052".to_string(),
            category: ErrorCategory::Borrow,
            severity: Severity::Error,
            title: "Lifetime mismatch",
            message: "Lifetime mismatch in return type",
            suggestion: "Ensure return lifetime matches the inputs",
            example: "fn foo<'a, 'b>(x: &'a i32, y: &'b i32) -> &'a i32 { y }  // Wrong lifetime",
            fix: "fn foo<'a>(x: &'a i32, y: &'a i32) -> &'a i32 { x }",
        });

        // ========== TRAIT ERRORS (E061-E070) ==========
        m.insert("E061", ErrorCode {
            code: "E061".to_string(),
            category: ErrorCategory::Trait,
            severity: Severity::Error,
            title: "Trait not in scope",
            message: "Trait method not found because trait is not imported",
            suggestion: "Import the trait with 'use' statement",
            example: "vec![1, 2].into_iter();  // IntoIterator not imported",
            fix: "use std::iter::IntoIterator;\nvec![1, 2].into_iter();",
        });

        m.insert("E062", ErrorCode {
            code: "E062".to_string(),
            category: ErrorCategory::Trait,
            severity: Severity::Error,
            title: "Missing trait method",
            message: "Trait implementation missing required method",
            suggestion: "Implement all required methods from the trait",
            example: "impl Iterator for MyType {\n    // Missing next() method\n}",
            fix: "impl Iterator for MyType {\n    type Item = i32;\n    fn next(&mut self) -> Option<i32> { None }\n}",
        });

        m.insert("E063", ErrorCode {
            code: "E063".to_string(),
            category: ErrorCategory::Trait,
            severity: Severity::Error,
            title: "Invalid associated type",
            message: "Invalid associated type in trait implementation",
            suggestion: "Check the associated type matches the trait definition",
            example: "impl Iterator for MyType {\n    type Item = u32;  // Should match declared type\n}",
            fix: "Ensure Item type matches what the trait expects",
        });

        m.insert("E064", ErrorCode {
            code: "E064".to_string(),
            category: ErrorCategory::Trait,
            severity: Severity::Error,
            title: "Conflicting trait impls",
            message: "Conflicting trait implementations",
            suggestion: "You can't implement the same trait twice for the same type",
            example: "impl From<i32> for MyType { }\nimpl From<i32> for MyType { }  // Conflict",
            fix: "impl From<i32> for MyType { }\nimpl From<String> for MyType { }  // Different impl",
        });

        m.insert("E065", ErrorCode {
            code: "E065".to_string(),
            category: ErrorCategory::Trait,
            severity: Severity::Error,
            title: "Unsafe trait without unsafe impl",
            message: "Unsafe trait requires 'unsafe impl'",
            suggestion: "Add 'unsafe' before 'impl' when implementing unsafe traits",
            example: "impl Drop for MyType { }  // Drop is unsafe",
            fix: "unsafe impl Drop for MyType { }",
        });

        // ========== UNIMPLEMENTED FEATURES (E081-E090) ==========
        m.insert("E081", ErrorCode {
            code: "E081".to_string(),
            category: ErrorCategory::Unimplemented,
            severity: Severity::Error,
            title: "Feature not yet supported",
            message: "This Rust feature is not yet implemented in GaiaRusted",
            suggestion: "Check the documentation for workarounds or limitations",
            example: "async fn foo() { }  // Async not yet implemented",
            fix: "Use callbacks or futures crate instead (planned for future release)",
        });

        m.insert("E082", ErrorCode {
            code: "E082".to_string(),
            category: ErrorCategory::Unimplemented,
            severity: Severity::Error,
            title: "Unsafe blocks not supported",
            message: "Unsafe blocks are not yet fully supported",
            suggestion: "Avoid unsafe code for now or use safe alternatives",
            example: "unsafe { let x = 5 as *const i32; }",
            fix: "Use safe Rust constructs instead",
        });

        m.insert("E083", ErrorCode {
            code: "E083".to_string(),
            category: ErrorCategory::Unimplemented,
            severity: Severity::Error,
            title: "Macros not fully supported",
            message: "Advanced macro features are not yet implemented",
            suggestion: "Use simple macros or find workarounds",
            example: "#[proc_macro]\npub fn my_macro(input: TokenStream) -> TokenStream { }",
            fix: "Use declarative macros (macro_rules!) instead",
        });

        m.insert("E084", ErrorCode {
            code: "E084".to_string(),
            category: ErrorCategory::Unimplemented,
            severity: Severity::Error,
            title: "Trait objects not supported",
            message: "Dynamic trait objects (dyn Trait) are not yet fully supported",
            suggestion: "Use concrete types or generic parameters instead of trait objects",
            example: "fn process(item: &dyn Display) { }  // Trait objects not yet supported",
            fix: "Use generics: fn process<T: Display>(item: &T) { }",
        });

        m.insert("E085", ErrorCode {
            code: "E085".to_string(),
            category: ErrorCategory::Unimplemented,
            severity: Severity::Error,
            title: "Impl trait not supported",
            message: "impl Trait return types are not yet implemented",
            suggestion: "Use concrete return types or trait objects instead",
            example: "fn create_iterator() -> impl Iterator<Item = i32> { }  // Not yet supported",
            fix: "Return a concrete iterator type or use Box<dyn Iterator>",
        });

        m.insert("E086", ErrorCode {
            code: "E086".to_string(),
            category: ErrorCategory::Unimplemented,
            severity: Severity::Error,
            title: "Complex assignment targets not supported",
            message: "Complex patterns on the left side of assignment are not yet fully supported",
            suggestion: "Use simpler assignment patterns or split into multiple statements",
            example: "let [x, y] = [1, 2];  // Complex destructuring may not work",
            fix: "Use simpler patterns: let x = arr[0];",
        });

        m.insert("E087", ErrorCode {
            code: "E087".to_string(),
            category: ErrorCategory::Unimplemented,
            severity: Severity::Error,
            title: "Expression type not supported",
            message: "This expression type is not yet implemented",
            suggestion: "Try using a simpler or different expression pattern",
            example: "let x = complex_expr_not_yet_supported;",
            fix: "Check the documentation for supported expressions",
        });

        // ========== INTERNAL ERRORS (E091-E100) ==========
        m.insert("E091", ErrorCode {
            code: "E091".to_string(),
            category: ErrorCategory::Internal,
            severity: Severity::Error,
            title: "Compiler bug",
            message: "This is a compiler bug - please report it",
            suggestion: "File a bug report with the error message and minimal reproduction",
            example: "Internal compiler error in codegen",
            fix: "This should not happen. Please report the issue.",
        });

        m.insert("E092", ErrorCode {
            code: "E092".to_string(),
            category: ErrorCategory::Internal,
            severity: Severity::Error,
            title: "Symbol table corruption",
            message: "Internal symbol table is corrupted",
            suggestion: "This indicates a compiler bug. Try recompiling.",
            example: "Variable lookup failed in symbol table",
            fix: "Report this error with your source code",
        });

        m.insert("E093", ErrorCode {
            code: "E093".to_string(),
            category: ErrorCategory::Internal,
            severity: Severity::Error,
            title: "Type system inconsistency",
            message: "Type system detected an internal inconsistency",
            suggestion: "This is likely a compiler bug",
            example: "Type unification failed unexpectedly",
            fix: "Please report this with your code",
        });

        m.insert("E094", ErrorCode {
            code: "E094".to_string(),
            category: ErrorCategory::Internal,
            severity: Severity::Error,
            title: "Code generation error",
            message: "Error during assembly code generation",
            suggestion: "Try simplifying your code or splitting into smaller functions",
            example: "Failed to generate assembly",
            fix: "Break complex code into simpler pieces",
        });

        m.insert("E095", ErrorCode {
             code: "E095".to_string(),
             category: ErrorCategory::Internal,
             severity: Severity::Error,
             title: "Out of resources",
             message: "Compiler ran out of memory or resources",
             suggestion: "Your code may be too complex. Try breaking it into smaller pieces.",
             example: "Stack overflow in recursive compilation",
             fix: "Simplify the code structure",
         });

         // ========== VISIBILITY ERRORS (E201-E210) ==========
         m.insert("E201", ErrorCode {
             code: "E201".to_string(),
             category: ErrorCategory::Type,
             severity: Severity::Error,
             title: "Private function not accessible",
             message: "Cannot access private function from another module",
             suggestion: "Make the function public by adding 'pub' keyword, or access it from within the same module",
             example: "mod utils {\n    fn private_fn() {}\n}\nfn main() {\n    utils::private_fn();  // Error!\n}",
             fix: "mod utils {\n    pub fn private_fn() {}  // Add pub\n}\nfn main() {\n    utils::private_fn();  // OK\n}",
         });

         m.insert("E202", ErrorCode {
             code: "E202".to_string(),
             category: ErrorCategory::Type,
             severity: Severity::Error,
             title: "Private struct not accessible",
             message: "Cannot access private struct from another module",
             suggestion: "Make the struct public by adding 'pub' keyword, or use it from within the same module",
             example: "mod types {\n    struct MyStruct { x: i32 }\n}\nfn main() {\n    let s = types::MyStruct { x: 5 };  // Error!\n}",
             fix: "mod types {\n    pub struct MyStruct { x: i32 }  // Add pub\n}\nfn main() {\n    let s = types::MyStruct { x: 5 };  // OK\n}",
         });

         m.insert("E203", ErrorCode {
             code: "E203".to_string(),
             category: ErrorCategory::Type,
             severity: Severity::Error,
             title: "Private enum not accessible",
             message: "Cannot access private enum from another module",
             suggestion: "Make the enum public by adding 'pub' keyword, or use it from within the same module",
             example: "mod colors {\n    enum Color { Red, Blue }\n}\nfn main() {\n    let c = colors::Color::Red;  // Error!\n}",
             fix: "mod colors {\n    pub enum Color { Red, Blue }  // Add pub\n}\nfn main() {\n    let c = colors::Color::Red;  // OK\n}",
         });

         m.insert("E204", ErrorCode {
             code: "E204".to_string(),
             category: ErrorCategory::Type,
             severity: Severity::Error,
             title: "Private method not accessible",
             message: "Cannot access private method from another module",
             suggestion: "Make the method public by adding 'pub' keyword to its impl block",
             example: "impl MyStruct {\n    fn private_method(&self) {}\n}\nfn main() {\n    let s = MyStruct {};    s.private_method();  // Error!\n}",
             fix: "impl MyStruct {\n    pub fn private_method(&self) {}  // Add pub\n}\nfn main() {\n    let s = MyStruct {};    s.private_method();  // OK\n}",
         });

         m.insert("E205", ErrorCode {
             code: "E205".to_string(),
             category: ErrorCategory::Type,
             severity: Severity::Error,
             title: "Private trait not accessible",
             message: "Cannot access private trait from another module",
             suggestion: "Make the trait public by adding 'pub' keyword, or use it from within the same module",
             example: "mod traits {\n    trait MyTrait { fn method(&self); }\n}\nfn main() {\n    // Cannot use traits::MyTrait\n}",
             fix: "mod traits {\n    pub trait MyTrait { fn method(&self); }  // Add pub\n}\nfn main() {\n    // Now can implement traits::MyTrait\n}",
         });

        m
    })
}

/// Get error code by code string
pub fn get_error_code(code: &str) -> Option<ErrorCode> {
    get_error_codes_map().get(code).cloned()
}

/// Map error message to appropriate error code
pub fn get_error_code_for_message(message: &str) -> Option<String> {
     let msg_lower = message.to_lowercase();
     
     // Visibility errors (E201-E205)
     if msg_lower.contains("private") && msg_lower.contains("function") {
         Some("E201".to_string())
     } else if msg_lower.contains("private") && msg_lower.contains("struct") {
         Some("E202".to_string())
     } else if msg_lower.contains("private") && msg_lower.contains("enum") {
         Some("E203".to_string())
     } else if msg_lower.contains("private") && msg_lower.contains("method") {
         Some("E204".to_string())
     } else if msg_lower.contains("private") && msg_lower.contains("trait") {
         Some("E205".to_string())
     } else if msg_lower.contains("not accessible") {
         Some("E201".to_string())
     } else if msg_lower.contains("undefined") && msg_lower.contains("variable") {
         Some("E022".to_string())
     } else if msg_lower.contains("undefined") && msg_lower.contains("type") {
         Some("E023".to_string())
     } else if msg_lower.contains("type mismatch") {
         Some("E021".to_string())
     } else if msg_lower.contains("wrong number") || msg_lower.contains("arguments") {
         Some("E024".to_string())
     } else if msg_lower.contains("unknown method") || (msg_lower.contains("method") && msg_lower.contains("not")) {
         Some("E025".to_string())
     } else if msg_lower.contains("incompatible") {
         Some("E026".to_string())
     } else if msg_lower.contains("not callable") {
         Some("E027".to_string())
     } else if msg_lower.contains("not") && msg_lower.contains("indexed") {
         Some("E028".to_string())
     } else if msg_lower.contains("missing") && msg_lower.contains("field") {
         Some("E029".to_string())
     } else if msg_lower.contains("unknown") && msg_lower.contains("field") {
         Some("E030".to_string())
     } else if msg_lower.contains("bool") && (msg_lower.contains("condition") || msg_lower.contains("expected")) {
         Some("E036".to_string())
     } else if msg_lower.contains("return") && msg_lower.contains("type") {
         Some("E037".to_string())
     } else if msg_lower.contains("exhaustive") || msg_lower.contains("pattern") {
         Some("E039".to_string())
     } else if msg_lower.contains("moved") {
         Some("E041".to_string())
     } else if msg_lower.contains("borrowed") && msg_lower.contains("already") {
         Some("E042".to_string())
     } else if msg_lower.contains("multiple") && msg_lower.contains("mutable") && msg_lower.contains("borrow") {
         Some("E043".to_string())
     } else if msg_lower.contains("immutable") && msg_lower.contains("borrow") {
         Some("E044".to_string())
     } else if msg_lower.contains("mutable") && msg_lower.contains("immutable") && msg_lower.contains("borrow") {
         Some("E045".to_string())
     } else if msg_lower.contains("not") && msg_lower.contains("mutable") && msg_lower.contains("borrow") {
         Some("E046".to_string())
     } else if msg_lower.contains("dangling") {
         Some("E047".to_string())
     } else if msg_lower.contains("unsafe") {
         Some("E082".to_string())
     } else if msg_lower.contains("trait object") || msg_lower.contains("dyn trait") {
         Some("E084".to_string())
     } else if msg_lower.contains("impl trait") {
         Some("E085".to_string())
     } else if msg_lower.contains("complex assignment") || msg_lower.contains("destructuring") {
         Some("E086".to_string())
     } else if msg_lower.contains("expression type") || msg_lower.contains("not yet supported") {
         Some("E087".to_string())
     } else {
         None
     }
 }

/// Get suggestion for error message
pub fn get_suggestion_for_message(message: &str) -> Option<String> {
    // Match error messages to suggestions
    if message.contains("undefined") || message.contains("not found") {
        Some("Check that the item is imported or in scope. Use 'use' to import items.".to_string())
    } else if message.contains("type mismatch") {
        Some("Ensure both sides of the operation have compatible types.".to_string())
    } else if message.contains("moved") {
        Some("Use a reference (&x) instead of moving, or reclone the value.".to_string())
    } else if message.contains("borrow") {
        Some("Check borrowing rules: at most one &mut or any number of &.".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_exists() {
        assert!(get_error_code("E001").is_some());
        assert!(get_error_code("E050").is_some());
    }

    #[test]
    fn test_error_code_invalid() {
        assert!(get_error_code("E999").is_none());
    }

    #[test]
    fn test_suggestion_generation() {
        assert!(get_suggestion_for_message("undefined variable").is_some());
        assert!(get_suggestion_for_message("type mismatch").is_some());
    }
}
