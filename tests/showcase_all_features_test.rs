use gaiarusted::{compile_files, CompilationConfig, OutputFormat};
use std::fs;
use std::path::PathBuf;

fn create_test_file(name: &str, content: &str) -> PathBuf {
    let path = PathBuf::from(format!("/tmp/gaiarusted_showcase_{}.rs", name));
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn showcase_primitives_and_operators() {
    let code = r#"
fn main() {
    let x: i32 = 42;
    let y: i64 = 1000;
    let z: f64 = 3.14;
    let b: bool = true;
    let s: str = "hello";
    let u: usize = 100;
    let i: isize = -50;
    
    let result = x + 1;
    let mult = x * 2;
    let div = x / 2;
    let modulo = x % 5;
    
    let eq = x == 42;
    let ne = x != 0;
    let lt = x < 100;
    let le = x <= 42;
    let gt = x > 10;
    let ge = x >= 42;
    
    let and = true && false;
    let or = true || false;
    let not = !true;
}
"#;
    
    let path = create_test_file("primitives", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_control_flow() {
    let code = r#"
fn main() {
    let x = 5;
    
    if x > 3 {
        println!("x is greater than 3");
    } else if x == 3 {
        println!("x equals 3");
    } else {
        println!("x is less than 3");
    }
    
    let mut count = 0;
    while count < 10 {
        count = count + 1;
    }
    
    for i in [1, 2, 3, 4, 5] {
        println!("i");
    }
}
"#;
    
    let path = create_test_file("control_flow", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_functions_and_structs() {
    let code = r#"
struct Point {
    x: i32,
    y: i32,
}

struct Rectangle {
    top_left: Point,
    bottom_right: Point,
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn process_point(p: Point) -> i32 {
    p.x + p.y
}

fn calculate_area(rect: Rectangle) -> i32 {
    let width = rect.bottom_right.x - rect.top_left.x;
    let height = rect.bottom_right.y - rect.top_left.y;
    width * height
}

fn main() {
    let result = add(10, 20);
    
    let point = Point { x: 5, y: 10 };
    let sum = process_point(point);
    
    let rect = Rectangle {
        top_left: Point { x: 0, y: 10 },
        bottom_right: Point { x: 5, y: 0 },
    };
    let area = calculate_area(rect);
}
"#;
    
    let path = create_test_file("functions_structs", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_pattern_matching() {
    let code = r#"
enum Color {
    Red,
    Green,
    Blue,
    Rgb(i32, i32, i32),
}

fn process_color(color: Color) -> i32 {
    match color {
        Color::Red => 1,
        Color::Green => 2,
        Color::Blue => 3,
        Color::Rgb(r, g, b) => r + g + b,
    }
}

fn process_tuple(pair: (i32, i32)) -> i32 {
    match pair {
        (0, y) => y,
        (x, 0) => x,
        (x, y) if x == y => x + y,
        (x, y) => x * y,
    }
}

fn process_option(opt: Option<i32>) -> i32 {
    match opt {
        Some(n) => n,
        None => 0,
    }
}

fn main() {
    let color = Color::Rgb(255, 0, 0);
    let value = process_color(color);
    
    let pair = (5, 5);
    let result = process_tuple(pair);
    
    let opt = Some(42);
    let num = process_option(opt);
}
"#;
    
    let path = create_test_file("pattern_matching", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_option_and_result() {
    let code = r#"
fn safe_divide(a: i32, b: i32) -> Option<i32> {
    if b == 0 {
        None
    } else {
        Some(a / b)
    }
}

fn divide_with_error(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Result::Err("Division by zero".to_string())
    } else {
        Result::Ok(a / b)
    }
}

fn main() {
    let opt = safe_divide(10, 2);
    match opt {
        Some(n) => {
            let doubled = n * 2;
        },
        None => {},
    }
    
    let res = divide_with_error(20, 4);
    match res {
        Result::Ok(n) => {
            let tripled = n * 3;
        },
        Result::Err(e) => {},
    }
}
"#;
    
    let path = create_test_file("option_result", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_borrowing_and_ownership() {
    let code = r#"
fn borrow_immutable(x: &i32) -> i32 {
    let val = *x;
    val + 1
}

fn borrow_mutable(x: &mut i32) -> () {
    *x = *x + 10;
}

fn take_ownership(x: i32) -> i32 {
    x * 2
}

fn main() {
    let mut num = 5;
    let immutable_ref = &num;
    let val1 = borrow_immutable(immutable_ref);
    
    let mutable_ref = &mut num;
    borrow_mutable(mutable_ref);
    
    let owned = 42;
    let result = take_ownership(owned);
}
"#;
    
    let path = create_test_file("borrowing", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_lifetimes() {
    let code = r#"
struct RefHolder<'a> {
    value: &'a i32,
}

fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    x
}

fn create_holder<'a>(r: &'a i32) -> RefHolder<'a> {
    RefHolder { value: r }
}

fn main() {
    let num = 42;
    let holder = create_holder(&num);
    
    let s1 = "hello";
    let s2 = "world";
    let result = longest(s1, s2);
}
"#;
    
    let path = create_test_file("lifetimes", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_generics() {
    let code = r#"
fn identity<T>(x: T) -> T {
    x
}

fn swap<T, U>(pair: (T, U)) -> (U, T) {
    (pair.1, pair.0)
}

struct Container<T> {
    value: T,
}

fn get_value<T>(c: Container<T>) -> T {
    c.value
}

fn main() {
    let int_val = identity(42);
    let str_val = identity("hello");
    
    let swapped = swap((1, "test"));
    
    let int_container = Container { value: 100 };
    let val = get_value(int_container);
}
"#;
    
    let path = create_test_file("generics", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_modules() {
    let code = r#"
mod math_utils {
    pub fn add(a: i32, b: i32) -> i32 {
        a + b
    }
    
    pub fn multiply(a: i32, b: i32) -> i32 {
        a * b
    }
    
    fn private_helper() -> i32 {
        42
    }
}

mod string_utils {
    pub fn concat(s1: str, s2: str) -> str {
        s1
    }
}

use math_utils::add;
use math_utils::multiply;

fn main() {
    let sum = add(10, 20);
    let product = multiply(5, 6);
}
"#;
    
    let path = create_test_file("modules", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_arrays_and_indexing() {
    let code = r#"
fn main() {
    let arr = [1, 2, 3, 4, 5];
    
    let first = arr[0];
    let second = arr[1];
    let last = arr[4];
    
    let multi = [[1, 2], [3, 4], [5, 6]];
    let elem = multi[0][1];
    
    for i in arr {
        let doubled = i * 2;
    }
}
"#;
    
    let path = create_test_file("arrays", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_iterators() {
    let code = r#"
fn main() {
    let arr = [1, 2, 3, 4, 5];
    
    let mapped = arr.map(|x| x * 2);
    let filtered = arr.filter(|x| x > 2);
    let taken = arr.take(3);
    let skipped = arr.skip(2);
    
    let sum = arr.fold(0, |acc, x| acc + x);
    let any_match = arr.any(|x| x == 3);
    let all_match = arr.all(|x| x > 0);
    
    let found = arr.find(|x| x > 2);
    let pos = arr.position(|x| x == 3);
}
"#;
    
    let path = create_test_file("iterators", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_strings() {
    let code = r#"
fn main() {
    let s = "hello world";
    
    let len = s.len();
    let contains_l = s.contains("l");
    let starts = s.starts_with("hello");
    let ends = s.ends_with("world");
    
    let upper = s.to_upper();
    let lower = s.to_lower();
    
    let trimmed = s.trim();
    let reversed = s.reverse_str();
    
    let parts = s.split_whitespace();
    let index = s.index_of("world");
    let substring = s.substr(0, 5);
    
    let repeated = s.repeat(2);
    let with_prefix = s.strip_prefix("hello");
    let with_suffix = s.strip_suffix("world");
    
    let bytes = s.into_bytes();
    let as_str = s.to_string();
}
"#;
    
    let path = create_test_file("strings", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_numeric_suffixes() {
    let code = r#"
fn main() {
    let a = 42i32;
    let b = 1000i64;
    let c = 3.14f64;
    let d = 100usize;
    let e = -50isize;
    let f = 255u32;
    
    let x: i32 = 5;
    let y = x + 10;
}
"#;
    
    let path = create_test_file("numeric_suffixes", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_raw_and_byte_strings() {
    let code = r#"
fn main() {
    let raw1 = r"string with \"quotes\"";
    let raw2 = r#"string with "quotes" and 'apostrophes'"#;
    let raw3 = r##"complex "string" with 'quotes'"##;
    
    let bytes1 = b"hello";
    let bytes2 = b"with\nescape\tsequences";
    let byte_char = b'A';
    let byte_newline = b'\n';
}
"#;
    
    let path = create_test_file("raw_byte_strings", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}

#[test]
fn showcase_complex_program() {
    let code = r#"
enum Result<T, E> {
    Ok(T),
    Err(E),
}

struct Calculator;

impl Calculator {
    pub fn add(a: i32, b: i32) -> i32 {
        a + b
    }
    
    pub fn multiply(a: i32, b: i32) -> i32 {
        a * b
    }
}

fn process_data<T>(items: [T], transform: (T) -> T) -> [T] {
    items
}

fn filter_positives(nums: [i32]) -> [i32] {
    nums.filter(|x| x > 0)
}

fn safe_operation(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Result::Err("Cannot divide by zero".to_string())
    } else {
        Result::Ok(a / b)
    }
}

fn main() {
    let numbers = [1, 2, 3, 4, 5];
    let positives = filter_positives(numbers);
    
    let sum = positives.fold(0, |acc, n| acc + n);
    let doubled = positives.map(|x| x * 2);
    
    let result = safe_operation(10, 2);
    match result {
        Result::Ok(value) => {
            let final_result = value * 10;
        },
        Result::Err(error) => {},
    }
    
    let calc_result = Calculator::add(100, 50);
}
"#;
    
    let path = create_test_file("complex_program", code);
    let mut config = CompilationConfig::new();
    config.input_files = vec![path.clone()];
    config.output_format = OutputFormat::Assembly;
    
    let result = compile_files(&config);
    assert!(result.is_ok());
    
    fs::remove_file(&path).ok();
}
