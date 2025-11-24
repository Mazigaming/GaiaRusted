// HashMap functionality test

fn main() {
    println!("=== HashMap Test ===\n");
    
    let mut map = HashMap::new();
    println!("Created empty HashMap");
    
    map.insert("key1", 42i32);
    println!("Inserted key1 -> 42");
    
    if let Some(val) = map.get("key1") {
        println!("Retrieved key1: {}", val);
    }
    
    map.insert("key2", 100i32);
    println!("Inserted key2 -> 100");
    
    if let Some(v) = map.get("key2") {
        println!("Retrieved key2: {}", v);
    }
    
    println!("\nHashMap operations complete");
}
