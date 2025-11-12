fn main() {
    println!("=== GaiaRusted Compiler Showcase ===\n");
    
    demo_primitives();
    demo_variables();
    demo_operators();
    demo_control_flow();
    demo_functions();
    demo_structs();
    demo_arrays();
    demo_pattern_matching();
    demo_ownership_and_borrowing();
    demo_option_and_result();
    demo_iterators();
    demo_string_methods();
}

fn demo_primitives() {
    println!("1. PRIMITIVE TYPES:");
    let i32_val: i32 = 42;
    let i64_val: i64 = 1000000000000i64;
    let f64_val: f64 = 3.14159;
    let bool_val: bool = true;
    let str_val: &str = "Hello, World!";
    let usize_val: usize = 10usize;
    let isize_val: isize = -5isize;
    
    println!("  i32: {}", i32_val);
    println!("  i64: {}", i64_val);
    println!("  f64: {}", f64_val);
    println!("  bool: {}", bool_val);
    println!("  str: {}", str_val);
    println!("  usize: {}", usize_val);
    println!("  isize: {}\n", isize_val);
}

fn demo_variables() {
    println!("2. VARIABLES AND ASSIGNMENTS:");
    let x = 10;
    let mut y = 20;
    y = y + 5;
    
    let (a, b) = (1, 2);
    println!("  x = {}, y = {}", x, y);
    println!("  Tuple unpacking: a = {}, b = {}\n", a, b);
}

fn demo_operators() {
    println!("3. OPERATORS:");
    let a = 10;
    let b = 3;
    
    println!("  Arithmetic: {} + {} = {}", a, b, a + b);
    println!("  Arithmetic: {} - {} = {}", a, b, a - b);
    println!("  Arithmetic: {} * {} = {}", a, b, a * b);
    println!("  Arithmetic: {} / {} = {}", a, b, a / b);
    println!("  Arithmetic: {} % {} = {}", a, b, a % b);
    
    println!("  Comparison: {} > {} = {}", a, b, a > b);
    println!("  Comparison: {} == {} = {}", a, a, a == a);
    println!("  Logical: true && false = {}", true && false);
    println!("  Logical: true || false = {}\n", true || false);
}

fn demo_control_flow() {
    println!("4. CONTROL FLOW:");
    
    let n = 5;
    if n > 0 {
        println!("  if/else: {} is positive", n);
    } else {
        println!("  if/else: {} is not positive", n);
    }
    
    let mut count = 0;
    while count < 3 {
        println!("  while loop: count = {}", count);
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
    println!("5. FUNCTIONS:");
    
    fn add(x: i32, y: i32) -> i32 {
        x + y
    }
    
    fn greet(name: &str) {
        println!("  Hello, {}!", name);
    }
    
    fn fibonacci(n: i32) -> i32 {
        if n <= 1 {
            return n;
        }
        fibonacci(n - 1) + fibonacci(n - 2)
    }
    
    println!("  add(3, 7) = {}", add(3, 7));
    greet("World");
    println!("  fibonacci(6) = {}\n", fibonacci(6));
}

fn demo_structs() {
    println!("6. STRUCTS:");
    
    struct Point {
        x: i32,
        y: i32,
    }
    
    struct Person {
        name: &'static str,
        age: i32,
    }
    
    let p = Point { x: 10, y: 20 };
    println!("  Point: ({}, {})", p.x, p.y);
    
    let person = Person { name: "Alice", age: 30 };
    println!("  Person: {} is {} years old\n", person.name, person.age);
}

fn demo_arrays() {
    println!("7. ARRAYS AND INDEXING:");
    let arr = [10, 20, 30, 40, 50];
    println!("  Array: {:?}", arr);
    println!("  arr[0] = {}", arr[0]);
    println!("  arr[2] = {}", arr[2]);
    println!("  arr[4] = {}\n", arr[4]);
}

fn demo_pattern_matching() {
    println!("8. PATTERN MATCHING:");
    
    let x = 3;
    match x {
        1 => println!("  match: x is one"),
        2 => println!("  match: x is two"),
        3 => println!("  match: x is three"),
        _ => println!("  match: x is something else"),
    }
    
    match x {
        1..=2 => println!("  range pattern: x is 1 or 2"),
        3..=5 => println!("  range pattern: x is 3, 4, or 5"),
        _ => println!("  range pattern: x is outside range"),
    }
    
    let (a, b) = (5, 10);
    match (a, b) {
        (5, 10) => println!("  tuple pattern: matched (5, 10)"),
        _ => println!("  tuple pattern: no match"),
    }
    
    println!();
}

fn demo_ownership_and_borrowing() {
    println!("9. OWNERSHIP AND BORROWING:");
    
    let s1 = "hello";
    let s2 = s1;
    println!("  Copy semantics: s1 = {}, s2 = {}", s1, s2);
    
    let mut x = 42;
    let y = &x;
    println!("  Immutable borrow: x = {}, *y = {}", x, *y);
    
    let z = &mut x;
    *z = 100;
    println!("  Mutable borrow: x = {}\n", x);
}

fn demo_option_and_result() {
    println!("10. OPTION AND RESULT TYPES:");
    
    let some_value: Option<i32> = Some(42);
    match some_value {
        Some(v) => println!("  Option::Some({})", v),
        None => println!("  Option::None"),
    }
    
    let none_value: Option<i32> = None;
    match none_value {
        Some(v) => println!("  Option::Some({})", v),
        None => println!("  Option::None"),
    }
    
    let ok_result: Result<i32, &str> = Ok(42);
    match ok_result {
        Ok(v) => println!("  Result::Ok({})", v),
        Err(e) => println!("  Result::Err({})", e),
    }
    
    let err_result: Result<i32, &str> = Err("Something went wrong");
    match err_result {
        Ok(v) => println!("  Result::Ok({})", v),
        Err(e) => println!("  Result::Err({})\n", e),
    }
}

fn demo_iterators() {
    println!("11. ITERATORS AND COMBINATORS:");
    
    let arr = [1, 2, 3, 4, 5];
    
    println!("  Array: {:?}", arr);
    
    let mut sum = 0;
    for x in arr {
        sum = sum + x;
    }
    println!("  Sum of elements: {}", sum);
    
    let mut max = arr[0];
    for x in arr {
        if x > max {
            max = x;
        }
    }
    println!("  Max element: {}", max);
    
    let mut min = arr[0];
    for x in arr {
        if x < min {
            min = x;
        }
    }
    println!("  Min element: {}\n", min);
}

fn demo_string_methods() {
    println!("12. STRING METHODS:");
    
    let s = "Hello, World!";
    println!("  Original string: {}", s);
    println!("  Length: {}", s.len());
    
    let upper = s.to_uppercase();
    println!("  to_uppercase(): {}", upper);
    
    let lower = s.to_lowercase();
    println!("  to_lowercase(): {}", lower);
    
    let contains_result = s.contains("World");
    println!("  contains(\"World\"): {}", contains_result);
    
    let starts = s.starts_with("Hello");
    println!("  starts_with(\"Hello\"): {}", starts);
    
    let ends = s.ends_with("!");
    println!("  ends_with(\"!\"): {}\n", ends);
}
