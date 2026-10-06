fn describe(s: &[i32]) -> String {
    match s {
        [] => "empty".into(),
        [one] => format!("one {}", one),
        [first, second] => format!("two {} {}", first, second),
        [first, .., last] if first == last => format!("same ends {}", first),
        [first, middle @ .., last] => format!("{} {:?} {}", first, middle, last),
    }
}
fn parse_kv(s: &str) -> Option<(String, i32)> {
    let Some((k, v)) = s.split_once(':') else { return None; };
    let Ok(n) = v.trim().parse::<i32>() else { return None; };
    Some((k.to_string(), n))
}
fn main() {
    for s in [&[][..], &[1], &[1, 2], &[4, 5, 4], &[1, 2, 3, 4]] { println!("{}", describe(s)); }
    println!("{:?} {:?} {:?}", parse_kv("a: 5"), parse_kv("nocolon"), parse_kv("b:x"));
    let mut stack = vec![1, 2, 3];
    while let Some(top) = stack.pop() { if top == 2 { continue; } print!("{} ", top); }
    println!();
    let v = Some(3);
    if let Some(x) = v { if x > 2 { println!("big {}", x); } } else { println!("none"); }
    println!("{}", matches!(v, Some(1..=5)));
    let arr = [1, 2, 3, 4, 5];
    if let [a, b, rest @ ..] = arr { println!("{} {} {:?}", a, b, rest); }
    let opt: Option<Option<i32>> = Some(None);
    if let Some(None) = opt { println!("inner none"); }
    let words = ["x", "y"];
    if let [w1, w2] = words { println!("{}{}", w1, w2); }
}
