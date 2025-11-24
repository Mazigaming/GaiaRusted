// Test for trait bounds in generic parameters

trait Display {
    fn display(&self);
}

struct Wrapper<T: Display> {
    value: T,
}

impl Display for i32 {
    fn display(&self) {
        println!("i32: {}", self);
    }
}

struct Number;

impl Display for Number {
    fn display(&self) {
        println!("Number");
    }
}

fn print_it<T: Display>(x: T) {
    x.display();
}

fn main() {
    println!("=== Trait Bounds Test ===\n");
    
    let num: i32 = 42;
    print_it(num);
    
    let n = Number;
    print_it(n);
    
    let wrapper = Wrapper { value: 100i32 };
    wrapper.value.display();
    
    println!("\nTest complete");
}
