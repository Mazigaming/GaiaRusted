// impl Trait return type test

use std::fmt::Display;

fn get_iterator() -> impl Iterator<Item = i32> {
    println!("Getting iterator");
    vec![1, 2, 3].into_iter()
}

fn combine_iterators() -> impl Iterator<Item = i32> {
    println!("Getting combined iterator");
    vec![4, 5, 6].into_iter()
}

fn display_value<T: Display>(val: T) {
    println!("Value: {}", val);
}

fn main() {
    println!("=== impl Trait Test ===\n");
    
    let it = get_iterator();
    println!("Got iterator");
    
    let combined = combine_iterators();
    println!("Got combined iterator");
    
    display_value(42);
    
    println!("\nimpl Trait parsing test complete");
}
