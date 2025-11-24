// Vec<T> basic operations test

fn main() {
    println!("=== Vec<T> Test ===\n");
    
    let mut v = Vec::new();
    println!("Created empty Vec");
    
    v.push(1);
    v.push(2);
    v.push(3);
    println!("Pushed 3 elements");
    
    let len = v.len();
    println!("Vector length: {}", len);
    
    let first = v.get(0);
    println!("First element retrieved");
    
    let popped = v.pop();
    println!("Popped element");
    
    println!("\nVec operations complete");
}
