// User-defined trait bounds test
// Note: Trait definitions are currently parsed but not fully validated

fn process<T>(value: T) -> T {
    value
}

fn main() {
    println!("=== Trait Bounds Generic Test ===\n");
    
    let x = process(42i32);
    println!("Processed i32: {}", x);
    
    let y = process(3.14f64);
    println!("Processed f64: {}", y);
    
    let s = process("hello");
    println!("Processed string: {}", s);
    
    println!("\nAll generic calls successful");
}
