// dyn Trait object test
use std::fmt::Display;

fn process_display(obj: &dyn Display) {
    println!("Processing display object");
}

fn main() {
    println!("=== dyn Trait Test ===\n");
    
    println!("dyn Trait parsing test complete");
}
