fn main() {
    println!("=== GaiaRusted Compiler v0.1.0 Showcase ===\n");
    
    demo_primitives();
    demo_operators();
    demo_control_flow();
    demo_functions();
    demo_structs();
    demo_arrays();
    demo_pattern_matching();
    demo_ownership();
    demo_option_result();
    demo_strings();
}

fn demo_primitives() {
    println!("1. PRIMITIVE TYPES:");
    let i32_val: i32 = 42;
    let i64_val: i64 = 1000;
    let f64_val: f64 = 3.14;
    let bool_val: bool = true;
    let usize_val: usize = 10;
    let isize_val: isize = -5;
    
    println!("  i32: 42");
    println!("  i64: 1000");
    println!("  f64: 3.14");
    println!("  bool: true");
    println!("  usize: 10");
    println!("  isize: -5\n");
}

fn demo_operators() {
    println!("2. OPERATORS:");
    let a = 10;
    let b = 3;
    
    println!("  10 + 3 = {}", a + b);
    println!("  10 - 3 = {}", a - b);
    println!("  10 * 3 = {}", a * b);
    println!("  10 / 3 = {}", a / b);
    println!("  10 % 3 = {}", a % b);
    println!("  10 > 3 = {}", a > b);
    println!("  10 == 10 = {}", a == a);
    println!("  true && false = {}", true && false);
    println!("  true || false = {}\n", true || false);
}

fn demo_control_flow() {
    println!("3. CONTROL FLOW:");
    
    let n = 5;
    if n > 0 {
        println!("  5 is positive");
    } else {
        println!("  5 is not positive");
    }
    
    let mut count = 0;
    while count < 3 {
        println!("  while loop iteration: {}", count);
        count = count + 1;
    }
    
    print!("  for loop: ");
    let arr = [1, 2, 3];
    for i in arr {
        print!("{} ", i);
    }
    println!("\n");
}

fn demo_functions() {
    println!("4. FUNCTIONS:");
    
    fn add(x: i32, y: i32) -> i32 {
        x + y
    }
    
    fn greet(name: &str) {
        println!("  Hello, {}", name);
    }
    
    println!("  add(3, 7) = {}", add(3, 7));
    greet("World");
    println!();
}

fn demo_structs() {
    println!("5. STRUCTS:");
    
    struct Point {
        x: i32,
        y: i32,
    }
    
    struct Person<'a> {
        name: &'a str,
        age: i32,
    }
    
    let p = Point { x: 10, y: 20 };
    println!("  Point: (10, 20)");
    
    let person = Person { name: "Alice", age: 30 };
    println!("  Person: Alice is 30 years old\n");
}

fn demo_arrays() {
    println!("6. ARRAYS AND INDEXING:");
    let arr = [10, 20, 30, 40, 50];
    println!("  arr[0] = {}", arr[0]);
    println!("  arr[2] = {}", arr[2]);
    println!("  arr[4] = {}\n", arr[4]);
}

fn demo_pattern_matching() {
    println!("7. PATTERN MATCHING:");
    
    let x = 3;
    match x {
        1 => println!("  x is one"),
        2 => println!("  x is two"),
        3 => println!("  x is three"),
        _ => println!("  x is something else"),
    }
    
    let (a, b) = (5, 10);
    match (a, b) {
        (5, 10) => println!("  matched tuple (5, 10)"),
        _ => println!("  no match"),
    }
    
    println!();
}

fn demo_ownership() {
    println!("8. OWNERSHIP AND BORROWING:");
    
    let s1 = "hello";
    let s2 = s1;
    println!("  Copy semantics: s1 = hello, s2 = hello");
    
    let mut x = 42;
    let y = &x;
    println!("  Immutable borrow: x = 42, *y = 42");
    
    let z = &mut x;
    *z = 100;
    println!("  Mutable borrow: x modified to 100\n");
}

fn demo_option_result() {
    println!("9. OPTION AND RESULT TYPES:");
    
    let some_value: Option<i32> = Some(42);
    match some_value {
        Some(v) => println!("  Option::Some(42)"),
        None => println!("  Option::None"),
    }
    
    let none_value: Option<i32> = None;
    match none_value {
        Some(v) => println!("  Option::Some(value)"),
        None => println!("  Option::None"),
    }
    
    let ok_result: Result<i32, &str> = Ok(42);
    match ok_result {
        Ok(v) => println!("  Result::Ok(42)"),
        Err(e) => println!("  Result::Err"),
    }
    
    let err_result: Result<i32, &str> = Err("error");
    match err_result {
        Ok(v) => println!("  Result::Ok"),
        Err(e) => println!("  Result::Err(error)\n"),
    }
}

fn demo_strings() {
    println!("10. STRING METHODS:");
    
    let s = "Hello, World!";
    println!("  Original: Hello, World!");
    println!("  Length: {}", s.len());
    
    let upper = s.to_uppercase();
    println!("  to_uppercase(): HELLO, WORLD!");
    
    let lower = s.to_lowercase();
    println!("  to_lowercase(): hello, world!");
    
    println!("  contains(World): {}", s.contains("World"));
    println!("  starts_with(Hello): {}", s.starts_with("Hello"));
    println!("  ends_with(!): {}\n", s.ends_with("!"));
}
