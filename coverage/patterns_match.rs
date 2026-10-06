#[derive(Debug)]
enum Msg { Quit, Move { x: i32, y: i32 }, Write(String), Color(u8, u8, u8) }
fn handle(m: &Msg) -> String {
    match m {
        Msg::Quit => "quit".to_string(),
        Msg::Move { x: 0, y } => format!("vertical {}", y),
        Msg::Move { x, y: 0 } => format!("horizontal {}", x),
        Msg::Move { x, y } if x == y => format!("diagonal {}", x),
        Msg::Move { .. } => "move".to_string(),
        Msg::Write(s) if s.is_empty() => "empty".into(),
        Msg::Write(s) => format!("write {}", s),
        Msg::Color(r, _, b @ 200..=255) => format!("blueish {} {}", r, b),
        Msg::Color(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
    }
}
fn main() {
    let msgs = [Msg::Quit, Msg::Move { x: 0, y: 5 }, Msg::Move { x: 3, y: 0 }, Msg::Move { x: 2, y: 2 }, Msg::Move { x: 1, y: 2 },
        Msg::Write(String::new()), Msg::Write("hi".into()), Msg::Color(1, 2, 250), Msg::Color(255, 128, 0)];
    for m in &msgs { println!("{}", handle(m)); }
    let n = 7;
    let desc = match n { x @ 1..=5 => format!("small {}", x), x if x % 2 == 1 => format!("odd {}", x), _ => "other".into() };
    println!("{}", desc);
    let pair = (2, -3);
    match pair { (0, y) => println!("y {}", y), (x, 0) => println!("x {}", x), (x, y) if x + y < 0 => println!("neg sum"), _ => println!("other") }
    let point = ((1, 2), (3, 4));
    let ((a, _), (_, d)) = point;
    println!("{} {}", a, d);
    let r = &Some(5);
    match r { Some(v) => println!("ref {}", v), None => {} }
    let c = 'k';
    println!("{}", match c { 'a'..='j' => 1, 'k'..='z' => 2, _ => 3 });
    let v = vec![1, 2, 3];
    match v.as_slice() { [] => println!("empty"), [x] => println!("one {}", x), [x, rest @ ..] => println!("{} then {:?}", x, rest) }
    let num = Some(42);
    match num { Some(n @ 40..=50) => println!("in range {}", n), Some(_) | None => println!("no") }
    let b = true;
    match (b, n) { (true, 7) => println!("both"), _ => println!("nope") }
}
