// Paths that start from a type: `<T as Trait>::item`, `<[u8; 3]>::try_from`.
use std::convert::{TryFrom, TryInto};
trait Shape { fn name() -> String; fn area(&self) -> f64; }
trait Named { fn name() -> String; }
struct Square(f64);
impl Shape for Square { fn name() -> String { "square".into() } fn area(&self) -> f64 { self.0 * self.0 } }
impl Named for Square { fn name() -> String { "named square".into() } }
fn total<I: Iterator>(iter: I) -> usize where <I as Iterator>::Item: std::fmt::Debug {
    let mut count = 0;
    for item in iter { println!("{:?}", item); count += 1; }
    count
}
fn conversions() {
    let bytes = [1u8, 2, 3, 4, 5];
    let slice: &[u8] = &bytes[..3];
    let head = <[u8; 3]>::try_from(slice);
    let tail: [u8; 2] = bytes[3..].try_into().unwrap();
    let short: Result<[u8; 4], _> = slice.try_into();
    println!("{:?} {:?} {}", head, tail, short.is_err());
    let small = <u8 as TryFrom<i32>>::try_from(300);
    println!("{:?}", small.is_err());
}

fn main() {
    println!("{} {}", <Square as Shape>::name(), <Square as Named>::name());
    println!("{}", <Square as Shape>::area(&Square(3.0)));
    let x = <i32>::max(3, 9);
    let s = <str>::len("hello");
    let parsed = <u8 as std::str::FromStr>::from_str("42").unwrap();
    println!("{} {} {}", x, s, parsed);
    println!("{}", total(vec!["a", "b"].into_iter()));
    let v = <Vec<i32>>::with_capacity(3);
    println!("{}", v.capacity() >= 3);
    conversions();
}
