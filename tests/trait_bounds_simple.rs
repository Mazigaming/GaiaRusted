// Simple trait bounds test

fn print_it<T: Display>(x: T) {
    println!("Got value");
}

fn main() {
    println!("Test");
}
