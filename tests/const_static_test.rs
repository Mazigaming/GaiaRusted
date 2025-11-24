// Const and static variable test

const ANSWER: i32 = 42;
static VERSION: &str = "1.0.0";

fn main() {
    println!("=== Const and Static Test ===\n");
    
    println!("ANSWER constant: {}", ANSWER);
    println!("VERSION static: {}", VERSION);
    
    const PI: f64 = 3.14159;
    println!("PI constant: {}", PI);
    
    println!("\nConst/static test complete");
}
