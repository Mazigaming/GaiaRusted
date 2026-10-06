// Nested modules and re-exports, statics (also mut), type aliases, const
// generics, macro_rules! with repetition and recursion, assertions.
mod geometry {
    pub mod shapes {
        #[derive(Debug, Default)]
        pub struct Rect { pub w: u32, pub h: u32, pub id: u32 }
        impl Rect { pub fn new(w: u32, h: u32) -> Self { Rect { w, h, id: next_id() } } pub fn id(&self) -> u32 { self.id } pub fn area(&self) -> u32 { self.w * self.h } }
        fn next_id() -> u32 { super::COUNTER_START + 1 }
    }
    pub const COUNTER_START: u32 = 100;
    pub use self::shapes::Rect;
    pub fn unit() -> Rect { Rect::new(1, 1) }
}
use geometry::{shapes, unit, Rect as R};
const LIMITS: [u32; 3] = [1, 10, 100];
static GREETING: &str = "hi";
static mut COUNTER: u32 = 0;
type Pair<T> = (T, T);
fn swap<T: Clone>(p: &Pair<T>) -> Pair<T> { (p.1.clone(), p.0.clone()) }
struct Wrapper<T> { inner: T }
impl<T: std::fmt::Display> Wrapper<T> { fn show(&self) -> String { format!("[{}]", self.inner) } }
impl Wrapper<i32> { fn double(&self) -> i32 { self.inner * 2 } }
struct Stack<T, const N: usize> { items: [Option<T>; N], len: usize }
impl<T: Copy + std::fmt::Debug, const N: usize> Stack<T, N> {
    fn new() -> Self { Stack { items: [None; N], len: 0 } }
    fn push(&mut self, x: T) -> bool { if self.len == N { return false; } self.items[self.len] = Some(x); self.len += 1; true }
    fn capacity(&self) -> usize { N }
}
macro_rules! square_all { ($($x:expr),*) => { vec![$($x * $x),*] }; }
macro_rules! maximum { ($x:expr) => { $x }; ($x:expr, $($rest:expr),+) => { { let a = $x; let b = maximum!($($rest),+); if a > b { a } else { b } } }; }
macro_rules! make_struct { ($name:ident { $($field:ident : $ty:ty),* }) => { #[derive(Debug)] struct $name { $($field: $ty),* } }; }
make_struct!(Config { verbose: bool, level: u8 });
fn main() {
    let r = shapes::Rect::new(3, 4);
    println!("{:?} {} {} {}", r, r.area(), r.id(), unit().area());
    let r2: R = R { w: 2, h: 5, ..Default::default() };
    println!("{} {}", r2.area(), geometry::COUNTER_START);
    println!("{:?} {} {}", LIMITS, GREETING, LIMITS.iter().sum::<u32>());
    unsafe { COUNTER += 5; COUNTER *= 2; let c = COUNTER; println!("{}", c); }
    println!("{:?}", swap(&("a", "b")));
    let w = Wrapper { inner: 21 };
    println!("{} {} {}", w.show(), w.double(), Wrapper { inner: "s" }.show());
    let mut st: Stack<u8, 2> = Stack::new();
    println!("{} {} {} {}", st.push(1), st.push(2), st.push(3), st.capacity());
    println!("{:?} {}", square_all!(1, 2, 3), maximum!(3, 9, 4));
    println!("{:?}", Config { verbose: true, level: 3 });
    println!("{} {}", std::mem::size_of::<Config>(), std::mem::size_of::<Pair<u16>>());
    let closure_type = |x: u8| -> u16 { x as u16 * 300 };
    println!("{}", closure_type(200));
    println!("{}", i32::MAX.to_string() + &u8::MIN.to_string());
    assert!(r.area() == 12, "area was {}", r.area());
    assert_eq!(1 + 1, 2);
    assert_ne!(1, 2, "must differ");
    debug_assert!(true);
    println!("{}", concat!("a", 1, true));
    println!("{}", stringify!(x + y * 2));
    let v: Vec<u8> = Vec::from([1, 2]);
    println!("{:?} {:?}", v, <Vec<u8> as Clone>::clone(&v));
    println!("{}", <i32 as std::str::FromStr>::from_str("5").unwrap() + i32::default());
    #[allow(unused)]
    let unused = 5;
    println!("{}", line!() > 0);
}
