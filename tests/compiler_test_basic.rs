fn add(x: i32, y: i32) -> i32 {
    x + y
}

fn multiply(x: i32, y: i32) -> i32 {
    x * y
}

fn greet(name: &str) {
    println!("Hello, {}!", name);
}

fn main() {
    println!("=== GaiaRusted Compiler Test: Basic Features ===\n");
    
    println!("1. PRIMITIVE TYPES:");
    let i32_val: i32 = 42;
    let i64_val: i64 = 1000000i64;
    let u32_val: u32 = 100u32;
    let f64_val: f64 = 3.14;
    let bool_val: bool = true;
    
    println!("  i32: {}", i32_val);
    println!("  i64: {}", i64_val);
    println!("  u32: {}", u32_val);
    println!("  f64: {}", f64_val);
    println!("  bool: {}\n", bool_val);
    
    println!("2. VARIABLES AND BINDINGS:");
    let x = 10;
    let mut y = 20;
    y = y + 5;
    
    let (a, b, c) = (1, 2, 3);
    println!("  Immutable: x = {}", x);
    println!("  Mutable: y = {}", y);
    println!("  Tuple unpacking: a={}, b={}, c={}\n", a, b, c);
    
    println!("3. OPERATORS:");
    let p = 10;
    let q = 3;
    
    println!("  Arithmetic: {} + {} = {}", p, q, p + q);
    println!("  Arithmetic: {} - {} = {}", p, q, p - q);
    println!("  Arithmetic: {} * {} = {}", p, q, p * q);
    println!("  Arithmetic: {} / {} = {}", p, q, p / q);
    println!("  Comparison: {} > {} = {}", p, q, p > q);
    println!("  Comparison: {} == {} = {}\n", p, p, p == p);
    
    println!("4. FUNCTIONS:");
    let result = add(5, 7);
    println!("  add(5, 7) = {}", result);
    let product = multiply(4, 5);
    println!("  multiply(4, 5) = {}", product);
    greet("World");
    println!();
    
    println!("5. CONTROL FLOW:");
    let n = 5;
    if n > 0 {
        println!("  if/else: {} is positive", n);
    } else {
        println!("  if/else: {} is not positive", n);
    }
    println!();
    
    println!("6. STRUCTS:");
    struct Point {
        x: i32,
        y: i32
    }
    
    struct Person {
        name: &'static str,
        age: i32
    }
    
    let p = Point { x: 10, y: 20 };
    let person = Person { name: "Alice", age: 30 };
    
    println!("  Point: ({}, {})", p.x, p.y);
    println!("  Person: {} is {} years old", person.name, person.age);
    println!();
    
    println!("7. ARRAYS AND SLICES:");
    let arr: [i32; 5] = [1, 2, 3, 4, 5];
    let first = arr[0];
    let second = arr[1];
    
    println!("  First element: {}", first);
    println!("  Second element: {}\n", second);
    
    println!("8. CONTROL FLOW - LOOPS:");
    let mut count = 0;
    while count < 3 {
        println!("  while loop: count = {}", count);
        count = count + 1;
    }
    
    for i in 0..3 {
        println!("  for loop: i = {}", i);
    }
    println!();
}
