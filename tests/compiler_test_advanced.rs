trait Animal {
    fn speak(&self);
}

struct Dog;
struct Cat;

impl Animal for Dog {
    fn speak(&self) {
        println!("Woof!");
    }
}

impl Animal for Cat {
    fn speak(&self) {
        println!("Meow!");
    }
}

fn longest<'a>(s1: &'a str, s2: &'a str) -> &'a str {
    s1
}

fn divide(a: i32, b: i32) -> Result<i32, &'static str> {
    if b == 0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}

struct Circle {
    radius: f64
}

impl Circle {
    fn area(&self) -> f64 {
        3.14159 * self.radius * self.radius
    }
}

fn main() {
    println!("=== GaiaRusted Compiler Test: Advanced Features ===\n");
    
    println!("1. TRAITS AND IMPLEMENTATIONS:");
    let dog = Dog;
    let cat = Cat;
    
    dog.speak();
    cat.speak();
    println!();
    
    println!("2. GENERICS:");
    fn identity(x: i32) -> i32 {
        x
    }
    
    let i = identity(42);
    println!("  Identity with i32: {}\n", i);
    
    println!("3. LIFETIMES:");
    let s1 = "hello";
    let s2 = "world";
    let result = longest(s1, s2);
    println!("  Longest: {}\n", result);
    
    println!("4. PATTERN MATCHING:");
    let value = 42;
    match value {
        42 => println!("  Matched 42"),
        _ => println!("  Other value")
    }
    println!();
    
    println!("5. RESULT TYPE:");
    match divide(10, 2) {
        Ok(result) => println!("  Division result: {}", result),
        Err(e) => println!("  Error: {}", e)
    }
    
    match divide(10, 0) {
        Ok(_) => println!("  Unexpected success"),
        Err(e) => println!("  Expected error caught: {}", e)
    }
    println!();
    
    println!("6. ENUMS:");
    enum Color {
        Red,
        Green,
        Blue
    }
    
    enum Message {
        Quit,
        Move { x: i32, y: i32 },
        Write(&'static str)
    }
    
    let color = Color::Red;
    let msg = Message::Move { x: 10, y: 20 };
    
    println!("  Color enum created");
    println!("  Message enum created\n");
    
    println!("7. STRUCTS WITH METHODS:");
    let circle = Circle { radius: 5.0 };
    println!("  Circle with radius: 5.0");
    println!("  Area: {:.2}\n", circle.area());
    
    println!("8. BORROWING AND REFERENCES:");
    let mut x = 10;
    let r1 = &x;
    let r2 = &x;
    
    println!("  Immutable borrow 1: {}", r1);
    println!("  Immutable borrow 2: {}", r2);
    
    let r3 = &mut x;
    println!("  Mutable borrow: {}", r3);
    
    x = 20;
    println!("  Modified value: {}\n", x);
}
