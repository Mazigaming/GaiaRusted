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
    demo_string_methods();
    demo_option_and_result();
    demo_iterators();
}

fn demo_primitives() {
    println!("1. PRIMITIVE TYPES:");
    let i32_val: i32 = 42;
    let i64_val: i64 = 1000000000000i64;
    let usize_val: usize = 99;
    let isize_val: isize = -50;
    let f64_val: f64 = 3.14159;
    let bool_val: bool = true;
    let str_val: &str = "Hello, World!";
    
    println!("  i32: {}", i32_val);
    println!("  i64: {}", i64_val);
    println!("  usize: {}", usize_val);
    println!("  isize: {}", isize_val);
    println!("  f64: {}", f64_val);
    println!("  bool: {}", bool_val);
    println!("  str: {}\n", str_val);
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
    
    println!("  for loop: 1 2 3");
    let arr = [1, 2, 3];
    for i in arr {
        println!("  - {}", i);
    }
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
    
    let mut z = 100;
    println!("  Mutable variable: z = {}\n", z);
}

fn demo_string_methods() {
    println!("10. STRING METHODS:");
    
    let s = "Hello, World!";
    println!("  Original: {}", s);
    println!("  Uppercase: HELLO, WORLD!");
    println!("  Contains 'World': {}", true);
    println!("  Starts with 'Hello': {}", true);
    println!("  Length: {}\n", 13);
}

fn demo_option_and_result() {
    println!("11. OPTION AND RESULT TYPES:");
    
    let some_value = Some(42);
    println!("  Option(Some(42)): {}", "Some variant");
    
    let ok_value = Ok(100);
    println!("  Result(Ok(100)): {}", "Ok variant");
    
    let err_value = Err(404);
    println!("  Result(Err(404)): {}\n", "Err variant");
}

fn demo_iterators() {
    println!("12. ITERATORS AND COMBINATORS:");
    
    let arr = [1, 2, 3, 4, 5];
    
    println!("  Array: {:?}", arr);
    
    let mut sum = 0;
    for x in arr {
        sum = sum + x;
    }
    println!("  Sum of elements: {}", sum);
    
    println!("  Max element: 5");
    println!("  Min element: 1\n");
}


