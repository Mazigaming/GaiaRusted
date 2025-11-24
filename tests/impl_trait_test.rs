// impl Trait return type test

fn get_iterator() -> impl Iterator {
    println!("Getting iterator");
    0
}

fn combine_iterators() -> impl Iterator + Clone {
    println!("Getting combined iterator");
    0
}

fn main() {
    println!("=== impl Trait Test ===\n");
    
    let it = get_iterator();
    println!("Got iterator: {}", it);
    
    let combined = combine_iterators();
    println!("Got combined iterator: {}", combined);
    
    println!("\nimpl Trait parsing test complete");
}
