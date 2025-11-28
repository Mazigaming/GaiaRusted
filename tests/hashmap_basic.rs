// Basic HashMap test

use std::collections::HashMap;

fn main() {
    println!("=== HashMap Test ===\n");
    
    let mut map = HashMap::new();
    println!("Created empty HashMap");
    
    map.insert("key1", 42i32);
    println!("Inserted key1 -> 42");
    
    let val = map.get("key1");
    println!("Retrieved key1");
    
    println!("\nHashMap operations complete");
}
