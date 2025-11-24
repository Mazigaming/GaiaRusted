enum Message {
    Text(String),
    Number(i32),
    Quit,
}

fn main() {
    let m1 = Message::Text("Hello, World!");
    let m2 = Message::Number(42);
    let m3 = Message::Quit;

    match m1 {
        Message::Text(s) => println!("Text: {}", s),
        Message::Number(n) => println!("Number: {}", n),
        Message::Quit => println!("Quit"),
    }

    match m2 {
        Message::Text(s) => println!("Text: {}", s),
        Message::Number(n) => println!("Number: {}", n),
        Message::Quit => println!("Quit"),
    }
}
