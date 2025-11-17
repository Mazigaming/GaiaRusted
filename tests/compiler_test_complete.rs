pub use std::collections::HashMap;

extern "C" fn c_compatible(x: i32) -> i32 {
    x * 2
}

trait Shape {
    fn area(&self) -> f64;
    fn name(&self) -> &str;
}

struct Circle {
    radius: f64
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.14159 * self.radius * self.radius
    }
    
    fn name(&self) -> &str {
        "Circle"
    }
}

struct Rectangle {
    width: f64,
    height: f64
}

impl Shape for Rectangle {
    fn area(&self) -> f64 {
        self.width * self.height
    }
    
    fn name(&self) -> &str {
        "Rectangle"
    }
}

mod math {
    pub fn add(a: i32, b: i32) -> i32 {
        a + b
    }
    
    pub fn multiply(a: i32, b: i32) -> i32 {
        a * b
    }
    
    pub fn divide(a: i32, b: i32) -> Result<i32, &'static str> {
        if b == 0 {
            Err("Division by zero")
        } else {
            Ok(a / b)
        }
    }
}

fn main() {
    println!("=== GaiaRusted Compiler Test: Complete v0.5.0 Features ===\n");
    
    println!(">>> FEATURE COVERAGE <<<\n");
    println!("1. Pub/Use Statements");
    println!("2. Raw Pointers");
    println!("3. FFI Support");
    println!("4. Slice Patterns");
    println!("5. Error Handling");
    println!("6. Traits and Implementations");
    println!("7. Module System");
    println!("8. Type System");
    println!("9. Complex Types");
    println!("10. Memory Management");
    println!("11. Advanced Pattern Matching\n");
    
    test_pub_use();
    test_raw_pointers();
    test_ffi_support();
    test_slice_patterns();
    test_error_handling();
    test_traits();
    test_modules();
    test_type_system();
    test_complex_types();
    test_memory();
}

fn test_pub_use() {
    println!("1. PUB/USE STATEMENTS:");
    
    let mut map = HashMap::new();
    map.insert("key", "value");
    
    println!("  pub use std::collections::HashMap;");
    println!("  Created HashMap with pub use\n");
}

fn test_raw_pointers() {
    println!("2. RAW POINTERS:");
    
    let x = 42;
    let ptr: *const i32 = &x;
    
    println!("  Created raw const pointer: *const i32");
    println!("  Raw pointers supported\n");
}

fn test_ffi_support() {
    println!("3. FFI SUPPORT:");
    
    let result = c_compatible(21);
    println!("  FFI function result: {}", result);
    println!("  C-compatible ABI supported\n");
}

fn test_slice_patterns() {
    println!("4. SLICE PATTERNS:");
    
    let arr = [1, 2, 3, 4, 5];
    let slice: &[i32] = &arr[1..4];
    
    println!("  Array: [1, 2, 3, 4, 5]");
    println!("  Slice [1..4] created");
    
    let tail: &[i32] = &arr[2..];
    let head: &[i32] = &arr[..2];
    
    println!("  Head and tail slices created\n");
}

fn test_error_handling() {
    println!("5. ERROR HANDLING:");
    
    match math::divide(10, 2) {
        Ok(result) => println!("  Division succeeded: {}", result),
        Err(e) => println!("  Error: {}", e)
    }
    
    match math::divide(10, 0) {
        Ok(_) => println!("  Unexpected success"),
        Err(e) => println!("  Expected error caught: {}", e)
    }
    println!();
}

fn test_traits() {
    println!("6. TRAITS AND IMPLEMENTATIONS:");
    
    let circle = Circle { radius: 5.0 };
    let rectangle = Rectangle { width: 4.0, height: 3.0 };
    
    println!("  Shape: {}", circle.name());
    println!("  Area: {:.2}", circle.area());
    
    println!("  Shape: {}", rectangle.name());
    println!("  Area: {:.2}\n", rectangle.area());
}

fn test_modules() {
    println!("7. MODULE SYSTEM:");
    
    let add_result = math::add(5, 3);
    let mult_result = math::multiply(5, 3);
    
    println!("  Module: math");
    println!("  5 + 3 = {}", add_result);
    println!("  5 * 3 = {}\n", mult_result);
}

fn test_type_system() {
    println!("8. TYPE SYSTEM:");
    
    let i32_val: i32 = 42;
    let f64_val: f64 = 3.14159;
    let str_val: &str = "hello";
    let bool_val: bool = true;
    
    println!("  i32: {}", i32_val);
    println!("  f64: {}", f64_val);
    println!("  &str: {}", str_val);
    println!("  bool: {}\n", bool_val);
}

fn test_complex_types() {
    println!("9. COMPLEX TYPES:");
    
    let tuple: (i32, &str, bool) = (42, "hello", true);
    let arr: [i32; 3] = [1, 2, 3];
    
    println!("  Tuple created");
    println!("  Array created");
    
    let (x, y, z) = tuple;
    println!("  Tuple unpacking: x={}, y={}, z={}\n", x, y, z);
}

fn test_memory() {
    println!("10. MEMORY MANAGEMENT:");
    
    let mut x = 10;
    let r1 = &x;
    let r2 = &x;
    
    println!("  Immutable borrow 1: {}", r1);
    println!("  Immutable borrow 2: {}", r2);
    
    let r3 = &mut x;
    println!("  Mutable borrow: {}", r3);
    
    x = 20;
    println!("  Modified value: {}\n", x);
}
