// Valid generic function test

fn identity<T>(x: T) -> T {
    x
}

fn main() {
    let x = identity(42);
    println!("Result: {}", x);
}
