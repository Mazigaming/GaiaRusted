//! GiaRusted Compiler Showcase
//! Demonstrates major compiler features:
//! - String operations
//! - Collections (Vec, HashMap)
//! - Pattern matching
//! - Generics
//! - Functions and closures

fn main() {
    println_welcome();
    demo_strings();
    demo_collections();
    demo_generics();
    demo_pattern_matching();
}

fn println_welcome() {
    let name = "GiaRusted";
    let version = "0.0.3";
    println!("Welcome to {} v{}", name, version);
}

fn demo_strings() {
    let mut message = String::from("Hello");
    message.push_str(", World!");
    
    let upper = message.to_uppercase();
    let lower = message.to_lowercase();
    
    println!("Original: {}", message);
    println!("Uppercase: {}", upper);
    println!("Lowercase: {}", lower);
    println!("Length: {}", message.len());
}

fn demo_collections() {
    let mut numbers = vec![1, 2, 3, 4, 5];
    
    for num in &numbers {
        println!("Number: {}", num);
    }
    
    numbers.push(6);
    numbers.push(7);
    
    println!("Sum: {}", sum_vec(&numbers));
    println!("Count: {}", numbers.len());
}

fn sum_vec(numbers: &Vec<i64>) -> i64 {
    let mut sum = 0;
    for &num in numbers {
        sum = sum + num;
    }
    sum
}

fn demo_generics() {
    let int_result = max(10, 20);
    let float_result = max_float(3.14, 2.71);
    
    println!("Max of 10 and 20: {}", int_result);
    println!("Max of 3.14 and 2.71: {}", float_result);
}

fn max(a: i64, b: i64) -> i64 {
    if a > b { a } else { b }
}

fn max_float(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

fn demo_pattern_matching() {
    let value = 42;
    
    match value {
        0 => println!("Zero"),
        1 | 2 | 3 => println!("One, Two, or Three"),
        4..=10 => println!("Four to Ten"),
        _ => println!("Other: {}", value),
    }
    
    let option_val: Option<i64> = Some(100);
    match option_val {
        Some(x) => println!("Got value: {}", x),
        None => println!("No value"),
    }
}

fn some_function(x: i64) -> i64 {
    let y = x + 10;
    y * 2
}
