// Formatting: {:#?} of nested data, widths, fills, precision from arguments,
// signs, radixes, exponents, and Display impls of one's own.
use std::fmt;
#[derive(Debug, Clone, Default, PartialEq)]
struct Inner { name: String, values: Vec<u8>, flag: Option<bool> }
#[derive(Debug)]
enum Shape { Circle { r: f64 }, Rect(f64, f64), Empty }
#[derive(Debug)]
struct Outer { inner: Inner, shapes: Vec<Shape>, pair: (i32, &'static str) }
struct Money(i64);
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        write!(f, "{}${}.{:02}", sign, self.0.abs() / 100, self.0.abs() % 100)
    }
}
fn main() {
    let o = Outer { inner: Inner { name: "in".into(), values: vec![1, 2], flag: Some(true) }, shapes: vec![Shape::Circle { r: 1.5 }, Shape::Rect(2.0, 3.0), Shape::Empty], pair: (7, "seven") };
    println!("{:?}", o);
    println!("{:#?}", o);
    println!("[{:>8.3}] [{:<8.2}] [{:^9.1}] [{:+}] [{:08.2}] [{:e}] [{:E}]", 3.14159, 2.5f32, -1.25, 42, -3.5, 1234.5, 0.00012);
    println!("[{:#x}] [{:#X}] [{:#o}] [{:#b}] [{:08b}] [{:#010x}] [{:x}]", 255, 255, 8, 5, 5, 255, -1i8);
    println!("[{:>5}] [{:<5}] [{:^5}] [{:*^7}] [{:-<4}]", "ab", "ab", "ab", "mid", 1);
    let w = 10; let p = 3;
    println!("[{:w$.p$}] [{:>1$}] [{:.*}] [{:>width$}]", 2.0f64.sqrt(), 6, 2, 1.23456, "r", width = w);
    println!("{0} {1} {0} {name}", "a", "b", name = "n");
    println!("{:?} {:?} {:?} {:?}", f64::NAN, f64::INFINITY, -0.0f64, 1e21);
    println!("{} {} {} {}", 1e15, 1e16, 0.1 + 0.2, 100000000.0f32);
    println!("{:?} {:?} {}", 1.0f64, 0.1f32, 1.0e-7);
    println!("{} {}", Money(-12345), Money(5));
    println!("{:>10}|{:<10}|", Money(250), format!("{}", Money(1)));
    println!("{:?} {:?}", Inner::default(), (1, "x", [1.5, 2.0], Some(()), None::<i32>));
    println!("{:5}|{:<5}|{:^5}|{:>5}|", true, 'c', "é", 1u8);
    println!("{:?}", "tab\t\"quote\"");
    println!("{:#?}", (1, vec![(2, 3)], Shape::Empty));
    eprintln!("to stderr");
    print!("no newline ");
    println!("{}", u128::MAX);
    println!("{:?} {}", i128::MIN, -170141183460469231731687303715884105728i128);
}
