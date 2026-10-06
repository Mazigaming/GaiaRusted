// Recursive enums, match guards and bindings, slice patterns, labeled
// breaks with values, let-else, ref patterns, shadowing.
#[derive(Debug)]
enum Expr { Num(i64), Add(Box<Expr>, Box<Expr>), Mul(Box<Expr>, Box<Expr>), Neg(Box<Expr>) }
fn eval(e: &Expr) -> i64 { match e { Expr::Num(n) => *n, Expr::Add(a, b) => eval(a) + eval(b), Expr::Mul(a, b) => eval(a) * eval(b), Expr::Neg(a) => -eval(a) } }
fn classify(n: i32) -> &'static str {
    match n { i32::MIN..=-1 => "neg", 0 => "zero", 1 | 2 | 3 => "small", x if x % 2 == 0 => "even", _ => "odd" }
}
struct Point { x: i32, y: i32 }
fn main() {
    let e = Expr::Add(Box::new(Expr::Num(2)), Box::new(Expr::Mul(Box::new(Expr::Num(3)), Box::new(Expr::Neg(Box::new(Expr::Num(4)))))));
    println!("{} {:?}", eval(&e), e);
    for n in [-5, 0, 2, 8, 9] { print!("{} ", classify(n)); }
    println!();
    let p = Point { x: 3, y: -3 };
    match p { Point { x, y: 0 } => println!("on x {}", x), Point { x: 0, y } => println!("on y {}", y), Point { x, y } if x == -y => println!("anti {} {}", x, y), _ => println!("else") }
    let msg = match 42u8 { n @ 0..=9 => format!("digit {}", n), n @ (10..=99) => format!("two {}", n), n => format!("big {}", n) };
    println!("{}", msg);
    let pair = (1, ("a", [1, 2, 3]));
    let (one, (letter, [first, .., last])) = pair;
    println!("{} {} {} {}", one, letter, first, last);
    let v = vec![1, 2, 3, 4, 5];
    if let [head, tail @ ..] = v.as_slice() { println!("{} {:?}", head, tail); }
    match v.as_slice() { [] => println!("empty"), [x] => println!("one {}", x), [x, y, rest @ ..] => println!("{} {} +{}", x, y, rest.len()) }
    let mut stack = vec![1, 2, 3];
    while let Some(top) = stack.pop() { print!("{} ", top); }
    println!();
    let found = 'outer: loop {
        for i in 0.. {
            for j in 0..i { if i * j == 42 { break 'outer (i, j); } }
            if i > 100 { break 'outer (0, 0); }
        }
    };
    println!("{:?}", found);
    let mut count = 0;
    'rows: for r in 0..5 { for c in 0..5 { if c > r { continue 'rows; } count += 1; } }
    println!("{}", count);
    let x = 5; let x = x * 2; { let x = "shadow"; println!("{}", x); } println!("{}", x);
    let label = if x > 5 { "big" } else { "small" };
    let value = loop { break 7; };
    println!("{} {}", label, value);
    let opt: Option<Option<i32>> = Some(None);
    match opt { Some(Some(v)) => println!("v {}", v), Some(None) => println!("inner none"), None => println!("none") }
    let r: Result<i32, String> = "12".parse::<i32>().map_err(|e| e.to_string());
    let Ok(n) = r else { panic!() };
    println!("{}", n);
    let refs = &(1, 2);
    let &(a, b) = refs;
    let (ref c, ref mut d) = (String::from("c"), 5);
    *d += 1;
    println!("{} {} {} {}", a, b, c, d);
    let nums = [1, 2, 3];
    let total: i32 = nums.iter().map(|&n| match n { 1 => 10, 2 => 20, _ => 0 }).sum();
    println!("{}", total);
    let t = matches!(e, Expr::Add(..));
    println!("{} {}", t, matches!(5, 1..=5));
    let ch = 'k';
    println!("{}", match ch { 'a'..='j' => 1, 'k'..='z' => 2, _ => 3 });
}
