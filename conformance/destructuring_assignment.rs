// Destructuring assignment: tuples, arrays, structs, `_` and `..` on the left of `=`.
struct Point { x: i32, y: i32 }
struct Pair(i32, i32);
fn main() {
    let (mut a, mut b) = (1, 2);
    (a, b) = (b, a);
    println!("{} {}", a, b);
    let mut fib = (0u64, 1u64);
    for _ in 0..10 {
        (fib.0, fib.1) = (fib.1, fib.0 + fib.1);
    }
    println!("{:?}", fib);
    let mut values = [0; 3];
    [values[0], _, values[2]] = [7, 8, 9];
    println!("{:?}", values);
    let (mut first, mut last) = (0, 0);
    (first, .., last) = (1, 2, 3, 4);
    println!("{} {}", first, last);
    let (mut x, mut y) = (0, 0);
    Point { x, y } = Point { x: 5, y: 6 };
    println!("{} {}", x, y);
    Pair(y, x) = Pair(x, y);
    println!("{} {}", x, y);
    let mut words = (String::new(), String::new());
    (words.1, words.0) = (String::from("left"), String::from("right"));
    println!("{} {}", words.0, words.1);
    _ = words;
}
