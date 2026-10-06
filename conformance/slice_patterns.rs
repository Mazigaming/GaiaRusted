// Slice patterns on arrays and slices: prefixes, suffixes and the rest.
fn describe(values: &[i32]) -> String {
    match values {
        [] => "empty".to_string(),
        [one] => format!("one {}", one),
        [first, second] => format!("two {} {}", first, second),
        [first, .., last] if first == last => format!("bookends {}", first),
        [first, middle @ .., last] => format!("{} {:?} {}", first, middle, last),
    }
}

fn sum(values: &[i64]) -> i64 {
    match values {
        [] => 0,
        [head, tail @ ..] => head + sum(tail),
    }
}

fn main() {
    for v in [vec![], vec![1], vec![1, 2], vec![3, 9, 3], vec![1, 2, 3, 4, 5]] {
        println!("{}", describe(&v));
    }
    println!("{}", sum(&[1, 2, 3, 4]));
    let array = [10, 20, 30, 40];
    let [a, b, rest @ ..] = array;
    println!("{} {} {:?}", a, b, rest);
    let [.., penultimate, _] = array;
    println!("{}", penultimate);
    if let [first, .., last] = &array {
        println!("{} {}", first, last);
    }
    let mut grid = [[0u8; 3]; 2];
    if let [row, ..] = &mut grid {
        row[1] = 7;
    }
    println!("{:?}", grid);
    let words = vec![String::from("alpha"), String::from("beta"), String::from("gamma")];
    if let [first, rest @ ..] = words.as_slice() {
        println!("{} then {}", first, rest.join(","));
    }
    match words.as_slice() {
        [.., last] => println!("last {}", last),
        [] => println!("none"),
    }
    let bytes = b"key=value";
    match bytes {
        [b'k', b'e', b'y', b'=', value @ ..] => println!("{}", String::from_utf8_lossy(value)),
        _ => println!("no key"),
    }
    let pair = [String::from("x"), String::from("y")];
    let [x, y] = pair;
    println!("{}{}", y, x);
}
