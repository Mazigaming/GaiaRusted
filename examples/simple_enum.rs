enum Color {
    Red,
    Green,
    Blue,
}

fn main() {
    let c = Color::Red;
    
    match c {
        Color::Red => println!("Red"),
        Color::Green => println!("Green"),
        Color::Blue => println!("Blue"),
    }
}
