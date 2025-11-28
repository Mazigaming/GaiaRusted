// Simple trait bounds test

use std::fmt::Display;

fn print_it<T: Display>(x: T) {
    println!("Got value");
}

fn main() {
    println!("Test");
}
