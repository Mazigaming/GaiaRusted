// `{:?}` and `{:#?}` of nested values, and `write!` inside `Display`.
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
struct Inner { id: u8, tags: Vec<&'static str> }
#[derive(Debug)]
struct Outer { name: String, inner: Inner, pair: (i32, f64), maybe: Option<Box<i32>>, empty: Vec<u8> }
#[derive(Debug)]
enum Shape { Dot, Circle(f64), Rect { w: u32, h: u32 }, Group(Vec<Shape>) }
#[derive(Debug)]
struct Unit;
#[derive(Debug)]
struct Wrapper(i32, &'static str);

struct Money(i64);
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "${}.{:02}", self.0 / 100, self.0 % 100)
    }
}

fn main() {
    let outer = Outer {
        name: "o".to_string(),
        inner: Inner { id: 1, tags: vec!["a", "b"] },
        pair: (3, 1.5),
        maybe: Some(Box::new(9)),
        empty: Vec::new(),
    };
    println!("{:?}", outer);
    println!("{:#?}", outer);
    let shapes = vec![Shape::Dot, Shape::Circle(2.5), Shape::Rect { w: 2, h: 3 }, Shape::Group(vec![Shape::Dot])];
    println!("{:?}", shapes);
    println!("{:#?}", shapes);
    println!("{:?} {:?} {:#?}", Unit, Wrapper(4, "x"), Wrapper(5, "y"));
    println!("{:#?}", (1, "two"));
    println!("{:?} {:?}", (7,), Some(Some(())));
    let mut map = HashMap::new();
    map.insert("key", vec![1, 2]);
    println!("{:?}", map);
    println!("{:#?}", map);
    let none: Option<i32> = None;
    println!("{:#?} {:#?}", none, Ok::<i32, String>(3));
    println!("[{:>10}] [{:<8}] [{}]", Money(5), Money(1234), Money(99));
    println!("[{:>6?}] [{:<8?}]", Some(1), "ab");
}
