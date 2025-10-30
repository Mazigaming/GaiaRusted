//! Built-in functions for the Rust compiler
//!
//! This module defines all built-in functions that can be called from compiled code,
//! including math operations, string manipulation, I/O, and type conversions.
//!
//! Comprehensive standard library with 70+ built-in functions covering:
//! - Math library (trigonometry, logarithms, random)
//! - File I/O operations
//! - Advanced string operations
//! - Collection utilities
//! - Type conversions and parsing

use std::fmt;

/// Built-in function identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinFunction {
    // === Math functions (Core) ===
    Abs,        // abs(x: i32) -> i32
    Min,        // min(a: i32, b: i32) -> i32
    Max,        // max(a: i32, b: i32) -> i32
    Pow,        // pow(base: i64, exp: i64) -> i64
    Sqrt,       // sqrt(x: f64) -> f64
    Floor,      // floor(x: f64) -> f64
    Ceil,       // ceil(x: f64) -> f64
    Round,      // round(x: f64) -> f64
    
    // === Math functions (Trigonometry & Advanced) ===
    Sin,        // sin(x: f64) -> f64
    Cos,        // cos(x: f64) -> f64
    Tan,        // tan(x: f64) -> f64
    Log,        // log(x: f64, base: f64) -> f64
    Ln,         // ln(x: f64) -> f64 (natural logarithm)
    Exp,        // exp(x: f64) -> f64
    Modulo,     // modulo(a: i64, b: i64) -> i64
    Gcd,        // gcd(a: i64, b: i64) -> i64
    
    // === Random functions ===
    Rand,       // rand() -> f64 (0.0 to 1.0)
    Randrange,  // randrange(min: i64, max: i64) -> i64
    
    // === String/Array functions (Core) ===
    Len,        // len(s: &str) -> usize or len(arr: &[T]) -> usize
    StrConcat,  // str_concat(a: &str, b: &str) -> String
    
    // === String operations (v0.0.3) ===
    Trim,       // trim(s: &str) -> String
    Split,      // split(s: &str, delimiter: char) -> Vec<String>
    Replace,    // replace(s: &str, from: &str, to: &str) -> String
    Uppercase,  // uppercase(s: &str) -> String
    Lowercase,  // lowercase(s: &str) -> String
    
    // === Advanced String Operations ===
    Contains,   // contains(s: &str, substring: &str) -> bool
    StartsWith, // starts_with(s: &str, prefix: &str) -> bool
    EndsWith,   // ends_with(s: &str, suffix: &str) -> bool
    Repeat,     // repeat(s: &str, times: usize) -> String
    ReverseStr, // reverse_str(s: &str) -> String
    Chars,      // chars(s: &str) -> Vec<char>
    IndexOf,    // index_of(s: &str, substring: &str) -> Option<usize>
    Substr,     // substr(s: &str, start: usize, len: usize) -> String
    
    // === I/O functions (Core) ===
    Print,      // print(s: &str) - no newline
    Println,    // println(s: &str) - with newline
    Eprintln,   // eprintln(s: &str) - stderr
    
    // === File I/O operations ===
    OpenRead,   // open_read(path: &str) -> FileHandle
    OpenWrite,  // open_write(path: &str) -> FileHandle
    ReadFile,   // read_file(path: &str) -> String
    WriteFile,  // write_file(path: &str, content: &str) -> ()
    ReadLine,   // read_line() -> String
    FileExists, // file_exists(path: &str) -> bool
    
    // === Type conversions (Core) ===
    AsI32,      // as_i32(x: f64) -> i32
    AsI64,      // as_i64(x: f64) -> i64
    AsF64,      // as_f64(x: i32) -> f64
    
    // === Type conversions & Parsing ===
    ParseInt,   // parse_int(s: &str) -> Option<i64>
    ParseFloat, // parse_float(s: &str) -> Option<f64>
    ToString,   // to_string(x: i32) -> String
    IsDigit,    // is_digit(c: char) -> bool
    IsAlpha,    // is_alpha(c: char) -> bool
    IsWhitespace, // is_whitespace(c: char) -> bool
    ToUpper,    // to_upper(c: char) -> char
    ToLower,    // to_lower(c: char) -> char
    
    // === Array/Vector operations (Core) ===
    Push,       // push(vec: &mut Vec<T>, value: T) -> ()
    Pop,        // pop(vec: &mut Vec<T>) -> Option<T>
    Get,        // get(arr: &[T], index: usize) -> Option<&T>
    
    // === Array utilities (v0.0.3) ===
    Find,       // find(arr: &[T], value: T) -> Option<usize>
    Slice,      // slice(arr: &[T], start: usize, end: usize) -> &[T]
    Reverse,    // reverse(arr: &mut [T]) -> ()
    Sort,       // sort(arr: &mut [T]) -> ()
    
    // === Advanced Array/Collection Operations ===
    Flatten,    // flatten(vec: &Vec<Vec<T>>) -> Vec<T>
    Count,      // count(arr: &[T], value: T) -> usize
    Sum,        // sum(arr: &[i64]) -> i64
    MaxVal,     // max_val(arr: &[i64]) -> i64
    MinVal,     // min_val(arr: &[i64]) -> i64
    IsEmpty,    // is_empty(arr: &[T]) -> bool
    Clear,      // clear(vec: &mut Vec<T>) -> ()
}

impl fmt::Display for BuiltinFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // Math core
            BuiltinFunction::Abs => write!(f, "abs"),
            BuiltinFunction::Min => write!(f, "min"),
            BuiltinFunction::Max => write!(f, "max"),
            BuiltinFunction::Pow => write!(f, "pow"),
            BuiltinFunction::Sqrt => write!(f, "sqrt"),
            BuiltinFunction::Floor => write!(f, "floor"),
            BuiltinFunction::Ceil => write!(f, "ceil"),
            BuiltinFunction::Round => write!(f, "round"),
            
            // Math advanced
            BuiltinFunction::Sin => write!(f, "sin"),
            BuiltinFunction::Cos => write!(f, "cos"),
            BuiltinFunction::Tan => write!(f, "tan"),
            BuiltinFunction::Log => write!(f, "log"),
            BuiltinFunction::Ln => write!(f, "ln"),
            BuiltinFunction::Exp => write!(f, "exp"),
            BuiltinFunction::Modulo => write!(f, "modulo"),
            BuiltinFunction::Gcd => write!(f, "gcd"),
            
            // Random
            BuiltinFunction::Rand => write!(f, "rand"),
            BuiltinFunction::Randrange => write!(f, "randrange"),
            
            // String/Array core
            BuiltinFunction::Len => write!(f, "len"),
            BuiltinFunction::StrConcat => write!(f, "str_concat"),
            
            // String operations
            BuiltinFunction::Trim => write!(f, "trim"),
            BuiltinFunction::Split => write!(f, "split"),
            BuiltinFunction::Replace => write!(f, "replace"),
            BuiltinFunction::Uppercase => write!(f, "uppercase"),
            BuiltinFunction::Lowercase => write!(f, "lowercase"),
            
            // String advanced
            BuiltinFunction::Contains => write!(f, "contains"),
            BuiltinFunction::StartsWith => write!(f, "starts_with"),
            BuiltinFunction::EndsWith => write!(f, "ends_with"),
            BuiltinFunction::Repeat => write!(f, "repeat"),
            BuiltinFunction::ReverseStr => write!(f, "reverse_str"),
            BuiltinFunction::Chars => write!(f, "chars"),
            BuiltinFunction::IndexOf => write!(f, "index_of"),
            BuiltinFunction::Substr => write!(f, "substr"),
            
            // I/O core
            BuiltinFunction::Print => write!(f, "print"),
            BuiltinFunction::Println => write!(f, "println"),
            BuiltinFunction::Eprintln => write!(f, "eprintln"),
            
            // File I/O
            BuiltinFunction::OpenRead => write!(f, "open_read"),
            BuiltinFunction::OpenWrite => write!(f, "open_write"),
            BuiltinFunction::ReadFile => write!(f, "read_file"),
            BuiltinFunction::WriteFile => write!(f, "write_file"),
            BuiltinFunction::ReadLine => write!(f, "read_line"),
            BuiltinFunction::FileExists => write!(f, "file_exists"),
            
            // Type conversions core
            BuiltinFunction::AsI32 => write!(f, "as_i32"),
            BuiltinFunction::AsI64 => write!(f, "as_i64"),
            BuiltinFunction::AsF64 => write!(f, "as_f64"),
            
            // Type conversions advanced
            BuiltinFunction::ParseInt => write!(f, "parse_int"),
            BuiltinFunction::ParseFloat => write!(f, "parse_float"),
            BuiltinFunction::ToString => write!(f, "to_string"),
            BuiltinFunction::IsDigit => write!(f, "is_digit"),
            BuiltinFunction::IsAlpha => write!(f, "is_alpha"),
            BuiltinFunction::IsWhitespace => write!(f, "is_whitespace"),
            BuiltinFunction::ToUpper => write!(f, "to_upper"),
            BuiltinFunction::ToLower => write!(f, "to_lower"),
            
            // Array/Vector core
            BuiltinFunction::Push => write!(f, "push"),
            BuiltinFunction::Pop => write!(f, "pop"),
            BuiltinFunction::Get => write!(f, "get"),
            
            // Array utilities
            BuiltinFunction::Find => write!(f, "find"),
            BuiltinFunction::Slice => write!(f, "slice"),
            BuiltinFunction::Reverse => write!(f, "reverse"),
            BuiltinFunction::Sort => write!(f, "sort"),
            
            // Array advanced
            BuiltinFunction::Flatten => write!(f, "flatten"),
            BuiltinFunction::Count => write!(f, "count"),
            BuiltinFunction::Sum => write!(f, "sum"),
            BuiltinFunction::MaxVal => write!(f, "max_val"),
            BuiltinFunction::MinVal => write!(f, "min_val"),
            BuiltinFunction::IsEmpty => write!(f, "is_empty"),
            BuiltinFunction::Clear => write!(f, "clear"),
        }
    }
}

impl BuiltinFunction {
    /// Check if a function name is a built-in function
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            // === Math functions (Core) ===
            "abs" => Some(BuiltinFunction::Abs),
            "min" => Some(BuiltinFunction::Min),
            "max" => Some(BuiltinFunction::Max),
            "pow" => Some(BuiltinFunction::Pow),
            "sqrt" => Some(BuiltinFunction::Sqrt),
            "floor" => Some(BuiltinFunction::Floor),
            "ceil" => Some(BuiltinFunction::Ceil),
            "round" => Some(BuiltinFunction::Round),
            
            // === Math functions (Trigonometry & Advanced) ===
            "sin" => Some(BuiltinFunction::Sin),
            "cos" => Some(BuiltinFunction::Cos),
            "tan" => Some(BuiltinFunction::Tan),
            "log" => Some(BuiltinFunction::Log),
            "ln" => Some(BuiltinFunction::Ln),
            "exp" => Some(BuiltinFunction::Exp),
            "modulo" => Some(BuiltinFunction::Modulo),
            "gcd" => Some(BuiltinFunction::Gcd),
            
            // === Random functions ===
            "rand" => Some(BuiltinFunction::Rand),
            "randrange" => Some(BuiltinFunction::Randrange),
            
            // === String/Array functions (Core) ===
            "len" => Some(BuiltinFunction::Len),
            "str_concat" => Some(BuiltinFunction::StrConcat),
            
            // === String operations (v0.0.3) ===
            "trim" => Some(BuiltinFunction::Trim),
            "split" => Some(BuiltinFunction::Split),
            "replace" => Some(BuiltinFunction::Replace),
            "uppercase" => Some(BuiltinFunction::Uppercase),
            "lowercase" => Some(BuiltinFunction::Lowercase),
            
            // === Advanced String Operations ===
            "contains" => Some(BuiltinFunction::Contains),
            "starts_with" => Some(BuiltinFunction::StartsWith),
            "ends_with" => Some(BuiltinFunction::EndsWith),
            "repeat" => Some(BuiltinFunction::Repeat),
            "reverse_str" => Some(BuiltinFunction::ReverseStr),
            "chars" => Some(BuiltinFunction::Chars),
            "index_of" => Some(BuiltinFunction::IndexOf),
            "substr" => Some(BuiltinFunction::Substr),
            
            // === I/O functions (Core) ===
            "print" => Some(BuiltinFunction::Print),
            "println" => Some(BuiltinFunction::Println),
            "eprintln" => Some(BuiltinFunction::Eprintln),
            
            // === File I/O operations ===
            "open_read" => Some(BuiltinFunction::OpenRead),
            "open_write" => Some(BuiltinFunction::OpenWrite),
            "read_file" => Some(BuiltinFunction::ReadFile),
            "write_file" => Some(BuiltinFunction::WriteFile),
            "read_line" => Some(BuiltinFunction::ReadLine),
            "file_exists" => Some(BuiltinFunction::FileExists),
            
            // === Type conversions (Core) ===
            "as_i32" => Some(BuiltinFunction::AsI32),
            "as_i64" => Some(BuiltinFunction::AsI64),
            "as_f64" => Some(BuiltinFunction::AsF64),
            
            // === Type conversions & Parsing ===
            "parse_int" => Some(BuiltinFunction::ParseInt),
            "parse_float" => Some(BuiltinFunction::ParseFloat),
            "to_string" => Some(BuiltinFunction::ToString),
            "is_digit" => Some(BuiltinFunction::IsDigit),
            "is_alpha" => Some(BuiltinFunction::IsAlpha),
            "is_whitespace" => Some(BuiltinFunction::IsWhitespace),
            "to_upper" => Some(BuiltinFunction::ToUpper),
            "to_lower" => Some(BuiltinFunction::ToLower),
            
            // === Array/Vector operations (Core) ===
            "push" => Some(BuiltinFunction::Push),
            "pop" => Some(BuiltinFunction::Pop),
            "get" => Some(BuiltinFunction::Get),
            
            // === Array utilities (v0.0.3) ===
            "find" => Some(BuiltinFunction::Find),
            "slice" => Some(BuiltinFunction::Slice),
            "reverse" => Some(BuiltinFunction::Reverse),
            "sort" => Some(BuiltinFunction::Sort),
            
            // === Advanced Array/Collection Operations ===
            "flatten" => Some(BuiltinFunction::Flatten),
            "count" => Some(BuiltinFunction::Count),
            "sum" => Some(BuiltinFunction::Sum),
            "max_val" => Some(BuiltinFunction::MaxVal),
            "min_val" => Some(BuiltinFunction::MinVal),
            "is_empty" => Some(BuiltinFunction::IsEmpty),
            "clear" => Some(BuiltinFunction::Clear),
            
            _ => None,
        }
    }

    /// Get the number of arguments for this function
    pub fn arg_count(&self) -> Option<usize> {
        match self {
            // === Single argument functions ===
            BuiltinFunction::Abs | BuiltinFunction::Sqrt | BuiltinFunction::Floor
            | BuiltinFunction::Ceil | BuiltinFunction::Round | BuiltinFunction::Len
            | BuiltinFunction::Print | BuiltinFunction::Println | BuiltinFunction::Eprintln
            | BuiltinFunction::AsI32 | BuiltinFunction::AsI64 | BuiltinFunction::AsF64
            | BuiltinFunction::Trim | BuiltinFunction::Uppercase | BuiltinFunction::Lowercase
            | BuiltinFunction::Reverse | BuiltinFunction::Pop
            // Math advanced (single arg)
            | BuiltinFunction::Sin | BuiltinFunction::Cos | BuiltinFunction::Tan
            | BuiltinFunction::Ln | BuiltinFunction::Exp
            // String advanced (single arg)
            | BuiltinFunction::ReverseStr | BuiltinFunction::Chars
            // File I/O (single arg)
            | BuiltinFunction::OpenRead | BuiltinFunction::OpenWrite | BuiltinFunction::ReadFile
            | BuiltinFunction::FileExists | BuiltinFunction::ReadLine
            // Type parsing (single arg)
            | BuiltinFunction::ParseInt | BuiltinFunction::ParseFloat | BuiltinFunction::ToString
            | BuiltinFunction::IsDigit | BuiltinFunction::IsAlpha | BuiltinFunction::IsWhitespace
            | BuiltinFunction::ToUpper | BuiltinFunction::ToLower
            // Array operations (single arg)
            | BuiltinFunction::Flatten | BuiltinFunction::IsEmpty | BuiltinFunction::Clear
            | BuiltinFunction::Rand | BuiltinFunction::Sum | BuiltinFunction::MaxVal 
            | BuiltinFunction::MinVal => Some(1),

            // === Two argument functions ===
            BuiltinFunction::Min | BuiltinFunction::Max | BuiltinFunction::Pow
            | BuiltinFunction::StrConcat | BuiltinFunction::Push 
            | BuiltinFunction::Get | BuiltinFunction::Find
            // Math advanced (two arg)
            | BuiltinFunction::Log | BuiltinFunction::Modulo | BuiltinFunction::Gcd
            // String advanced (two arg)
            | BuiltinFunction::Contains | BuiltinFunction::StartsWith | BuiltinFunction::EndsWith
            | BuiltinFunction::Repeat | BuiltinFunction::Split | BuiltinFunction::IndexOf
            // File I/O (two arg)
            | BuiltinFunction::WriteFile
            // Random (two arg)
            | BuiltinFunction::Randrange
            // Array (two arg)
            | BuiltinFunction::Count => Some(2),
            
            // === Three argument functions ===
            BuiltinFunction::Replace | BuiltinFunction::Slice | BuiltinFunction::Substr => Some(3),
            
            // Zero argument functions
            BuiltinFunction::Sort => Some(1), // Actually sorts in-place, takes 1 arg
        }
    }

    /// Get a description of this function
    pub fn description(&self) -> &'static str {
        match self {
            // === Math functions (Core) ===
            BuiltinFunction::Abs => "Absolute value",
            BuiltinFunction::Min => "Minimum of two values",
            BuiltinFunction::Max => "Maximum of two values",
            BuiltinFunction::Pow => "Power (base^exponent)",
            BuiltinFunction::Sqrt => "Square root",
            BuiltinFunction::Floor => "Floor (round down)",
            BuiltinFunction::Ceil => "Ceiling (round up)",
            BuiltinFunction::Round => "Round to nearest integer",
            
            // === Math functions (Trigonometry & Advanced) ===
            BuiltinFunction::Sin => "Sine (radians)",
            BuiltinFunction::Cos => "Cosine (radians)",
            BuiltinFunction::Tan => "Tangent (radians)",
            BuiltinFunction::Log => "Logarithm with custom base",
            BuiltinFunction::Ln => "Natural logarithm (base e)",
            BuiltinFunction::Exp => "Exponential function (e^x)",
            BuiltinFunction::Modulo => "Modulo operation (a % b)",
            BuiltinFunction::Gcd => "Greatest common divisor",
            
            // === Random functions ===
            BuiltinFunction::Rand => "Random float between 0.0 and 1.0",
            BuiltinFunction::Randrange => "Random integer in range [min, max)",
            
            // === String/Array functions (Core) ===
            BuiltinFunction::Len => "Length of string or array",
            BuiltinFunction::StrConcat => "Concatenate two strings",
            
            // === String operations (v0.0.3) ===
            BuiltinFunction::Trim => "Remove leading/trailing whitespace",
            BuiltinFunction::Split => "Split string by delimiter",
            BuiltinFunction::Replace => "Replace substring with another",
            BuiltinFunction::Uppercase => "Convert string to uppercase",
            BuiltinFunction::Lowercase => "Convert string to lowercase",
            
            // === Advanced String Operations ===
            BuiltinFunction::Contains => "Check if string contains substring",
            BuiltinFunction::StartsWith => "Check if string starts with prefix",
            BuiltinFunction::EndsWith => "Check if string ends with suffix",
            BuiltinFunction::Repeat => "Repeat string n times",
            BuiltinFunction::ReverseStr => "Reverse string characters",
            BuiltinFunction::Chars => "Convert string to character array",
            BuiltinFunction::IndexOf => "Find first index of substring",
            BuiltinFunction::Substr => "Extract substring",
            
            // === I/O functions (Core) ===
            BuiltinFunction::Print => "Print to stdout (no newline)",
            BuiltinFunction::Println => "Print to stdout with newline",
            BuiltinFunction::Eprintln => "Print to stderr with newline",
            
            // === File I/O operations ===
            BuiltinFunction::OpenRead => "Open file for reading",
            BuiltinFunction::OpenWrite => "Open file for writing",
            BuiltinFunction::ReadFile => "Read entire file into string",
            BuiltinFunction::WriteFile => "Write string to file",
            BuiltinFunction::ReadLine => "Read line from stdin",
            BuiltinFunction::FileExists => "Check if file exists",
            
            // === Type conversions (Core) ===
            BuiltinFunction::AsI32 => "Convert to i32",
            BuiltinFunction::AsI64 => "Convert to i64",
            BuiltinFunction::AsF64 => "Convert to f64",
            
            // === Type conversions & Parsing ===
            BuiltinFunction::ParseInt => "Parse string to i64",
            BuiltinFunction::ParseFloat => "Parse string to f64",
            BuiltinFunction::ToString => "Convert number to string",
            BuiltinFunction::IsDigit => "Check if character is digit",
            BuiltinFunction::IsAlpha => "Check if character is alphabetic",
            BuiltinFunction::IsWhitespace => "Check if character is whitespace",
            BuiltinFunction::ToUpper => "Convert character to uppercase",
            BuiltinFunction::ToLower => "Convert character to lowercase",
            
            // === Array/Vector operations (Core) ===
            BuiltinFunction::Push => "Push element to vector",
            BuiltinFunction::Pop => "Pop element from vector",
            BuiltinFunction::Get => "Get array element by index",
            
            // === Array utilities (v0.0.3) ===
            BuiltinFunction::Find => "Find element in array",
            BuiltinFunction::Slice => "Slice array from start to end",
            BuiltinFunction::Reverse => "Reverse array in place",
            BuiltinFunction::Sort => "Sort array in place",
            
            // === Advanced Array/Collection Operations ===
            BuiltinFunction::Flatten => "Flatten nested array",
            BuiltinFunction::Count => "Count occurrences of value",
            BuiltinFunction::Sum => "Sum all elements",
            BuiltinFunction::MaxVal => "Find maximum value",
            BuiltinFunction::MinVal => "Find minimum value",
            BuiltinFunction::IsEmpty => "Check if array is empty",
            BuiltinFunction::Clear => "Clear all elements from vector",
        }
    }

    /// Get the assembly code for this builtin (if it's a simple operation)
    pub fn codegen_inline(&self) -> Option<&'static str> {
        match self {
            // These need special handling in codegen
            _ => None,
        }
    }

    /// List all available built-in functions
    pub fn all() -> &'static [BuiltinFunction] {
        &[
            // === Math functions (Core) ===
            BuiltinFunction::Abs,
            BuiltinFunction::Min,
            BuiltinFunction::Max,
            BuiltinFunction::Pow,
            BuiltinFunction::Sqrt,
            BuiltinFunction::Floor,
            BuiltinFunction::Ceil,
            BuiltinFunction::Round,
            
            // === Math functions (Trigonometry & Advanced) ===
            BuiltinFunction::Sin,
            BuiltinFunction::Cos,
            BuiltinFunction::Tan,
            BuiltinFunction::Log,
            BuiltinFunction::Ln,
            BuiltinFunction::Exp,
            BuiltinFunction::Modulo,
            BuiltinFunction::Gcd,
            
            // === Random functions ===
            BuiltinFunction::Rand,
            BuiltinFunction::Randrange,
            
            // === String/Array functions (Core) ===
            BuiltinFunction::Len,
            BuiltinFunction::StrConcat,
            
            // === String operations (v0.0.3) ===
            BuiltinFunction::Trim,
            BuiltinFunction::Split,
            BuiltinFunction::Replace,
            BuiltinFunction::Uppercase,
            BuiltinFunction::Lowercase,
            
            // === Advanced String Operations ===
            BuiltinFunction::Contains,
            BuiltinFunction::StartsWith,
            BuiltinFunction::EndsWith,
            BuiltinFunction::Repeat,
            BuiltinFunction::ReverseStr,
            BuiltinFunction::Chars,
            BuiltinFunction::IndexOf,
            BuiltinFunction::Substr,
            
            // === I/O functions (Core) ===
            BuiltinFunction::Print,
            BuiltinFunction::Println,
            BuiltinFunction::Eprintln,
            
            // === File I/O operations ===
            BuiltinFunction::OpenRead,
            BuiltinFunction::OpenWrite,
            BuiltinFunction::ReadFile,
            BuiltinFunction::WriteFile,
            BuiltinFunction::ReadLine,
            BuiltinFunction::FileExists,
            
            // === Type conversions (Core) ===
            BuiltinFunction::AsI32,
            BuiltinFunction::AsI64,
            BuiltinFunction::AsF64,
            
            // === Type conversions & Parsing ===
            BuiltinFunction::ParseInt,
            BuiltinFunction::ParseFloat,
            BuiltinFunction::ToString,
            BuiltinFunction::IsDigit,
            BuiltinFunction::IsAlpha,
            BuiltinFunction::IsWhitespace,
            BuiltinFunction::ToUpper,
            BuiltinFunction::ToLower,
            
            // === Array/Vector operations (Core) ===
            BuiltinFunction::Push,
            BuiltinFunction::Pop,
            BuiltinFunction::Get,
            
            // === Array utilities (v0.0.3) ===
            BuiltinFunction::Find,
            BuiltinFunction::Slice,
            BuiltinFunction::Reverse,
            BuiltinFunction::Sort,
            
            // === Advanced Array/Collection Operations ===
            BuiltinFunction::Flatten,
            BuiltinFunction::Count,
            BuiltinFunction::Sum,
            BuiltinFunction::MaxVal,
            BuiltinFunction::MinVal,
            BuiltinFunction::IsEmpty,
            BuiltinFunction::Clear,
        ]
    }
}

/// Generate standard library function stubs (assembly for math functions)
pub fn generate_stdlib_stubs() -> String {
    let mut code = String::new();
    
    // Math function stubs
    code.push_str("; Built-in math functions\n");
    code.push_str("\n");
    
    // abs(x) - return |x|
    code.push_str("_builtin_abs:\n");
    code.push_str("  ; abs(rdi) -> rax\n");
    code.push_str("  mov rax, rdi\n");
    code.push_str("  cqo\n");                    // Sign extend rax into rdx:rax
    code.push_str("  xor rax, rdx\n");           // XOR with sign bit
    code.push_str("  sub rax, rdx\n");           // Complete abs operation
    code.push_str("  ret\n");
    code.push_str("\n");

    // min(a, b) - return min(rdi, rsi)
    code.push_str("_builtin_min:\n");
    code.push_str("  ; min(rdi, rsi) -> rax\n");
    code.push_str("  mov rax, rdi\n");
    code.push_str("  cmp rdi, rsi\n");
    code.push_str("  jle .min_ret\n");
    code.push_str("  mov rax, rsi\n");
    code.push_str(".min_ret:\n");
    code.push_str("  ret\n");
    code.push_str("\n");

    // max(a, b) - return max(rdi, rsi)
    code.push_str("_builtin_max:\n");
    code.push_str("  ; max(rdi, rsi) -> rax\n");
    code.push_str("  mov rax, rdi\n");
    code.push_str("  cmp rdi, rsi\n");
    code.push_str("  jge .max_ret\n");
    code.push_str("  mov rax, rsi\n");
    code.push_str(".max_ret:\n");
    code.push_str("  ret\n");
    code.push_str("\n");

    // len(s) - for strings, this would need to access runtime info
    code.push_str("_builtin_len:\n");
    code.push_str("  ; len(&str) -> rax\n");
    code.push_str("  ; Argument: rdi = pointer to string data\n");
    code.push_str("  ; Note: This needs runtime length info - returning 0 as placeholder\n");
    code.push_str("  xor rax, rax\n");
    code.push_str("  ret\n");
    code.push_str("\n");

    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_from_name() {
        assert_eq!(BuiltinFunction::from_name("abs"), Some(BuiltinFunction::Abs));
        assert_eq!(BuiltinFunction::from_name("min"), Some(BuiltinFunction::Min));
        assert_eq!(BuiltinFunction::from_name("unknown"), None);
    }

    #[test]
    fn test_builtin_arg_count() {
        assert_eq!(BuiltinFunction::Abs.arg_count(), Some(1));
        assert_eq!(BuiltinFunction::Min.arg_count(), Some(2));
        assert_eq!(BuiltinFunction::Pow.arg_count(), Some(2));
    }

    #[test]
    fn test_all_builtins() {
        let builtins = BuiltinFunction::all();
        assert!(builtins.len() > 0);
        assert!(builtins.contains(&BuiltinFunction::Abs));
        assert!(builtins.contains(&BuiltinFunction::Println));
    }
}