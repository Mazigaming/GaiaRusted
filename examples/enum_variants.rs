enum Color {
    Red,
    Green,
    Blue,
}

enum Status {
    Active,
    Inactive,
    Pending,
}

enum Value {
    Integer(i32),
    Float(i32),
    Boolean(i32),
}

fn main() {
    let color = Color::Red;
    let status = Status::Active;
    let value = Value::Integer(42);
    
    match color {
        Color::Red => println!("Red"),
        Color::Green => println!("Green"),
        Color::Blue => println!("Blue"),
    }
    
    match status {
        Status::Active => println!("Active"),
        Status::Inactive => println!("Inactive"),
        Status::Pending => println!("Pending"),
    }
    
    match value {
        Value::Integer(n) => println!("Integer: {}", n),
        Value::Float(n) => println!("Float: {}", n),
        Value::Boolean(n) => println!("Boolean: {}", n),
    }
}
