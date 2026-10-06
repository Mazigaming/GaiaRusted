// Operator overloading, trait objects, default and super-trait methods,
// associated types, closures returned and boxed, ordering of enums.
use std::fmt::{self, Debug, Display};
use std::ops::{Add, AddAssign, Index, IndexMut, Mul, Neg, Sub};
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
struct V2 { x: f64, y: f64 }
impl Add for V2 { type Output = V2; fn add(self, o: V2) -> V2 { V2 { x: self.x + o.x, y: self.y + o.y } } }
impl Sub for V2 { type Output = V2; fn sub(self, o: V2) -> V2 { V2 { x: self.x - o.x, y: self.y - o.y } } }
impl Mul<f64> for V2 { type Output = V2; fn mul(self, k: f64) -> V2 { V2 { x: self.x * k, y: self.y * k } } }
impl Neg for V2 { type Output = V2; fn neg(self) -> V2 { V2 { x: -self.x, y: -self.y } } }
impl AddAssign for V2 { fn add_assign(&mut self, o: V2) { self.x += o.x; self.y += o.y; } }
impl Display for V2 { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "({}, {})", self.x, self.y) } }
struct Grid { w: usize, cells: Vec<u8> }
impl Index<(usize, usize)> for Grid { type Output = u8; fn index(&self, (r, c): (usize, usize)) -> &u8 { &self.cells[r * self.w + c] } }
impl IndexMut<(usize, usize)> for Grid { fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut u8 { &mut self.cells[r * self.w + c] } }
trait Shape { fn area(&self) -> f64; fn name(&self) -> String { "shape".to_string() } fn describe(&self) -> String { format!("{} with area {:.2}", self.name(), self.area()) } }
trait Named: Shape { fn label(&self) -> String { format!("<{}>", self.name()) } }
struct Sq(f64);
struct Circ(f64);
impl Shape for Sq { fn area(&self) -> f64 { self.0 * self.0 } fn name(&self) -> String { "square".into() } }
impl Shape for Circ { fn area(&self) -> f64 { 3.14159 * self.0 * self.0 } }
impl Named for Sq {}
trait Container { type Item; fn items(&self) -> Vec<Self::Item>; fn first(&self) -> Option<Self::Item> { self.items().into_iter().next() } }
struct Words(String);
impl Container for Words { type Item = String; fn items(&self) -> Vec<String> { self.0.split(' ').map(String::from).collect() } }
fn largest<T: PartialOrd + Copy>(xs: &[T]) -> T { let mut m = xs[0]; for &x in xs { if x > m { m = x; } } m }
fn show_all<T>(xs: &[T]) -> String where T: Debug { xs.iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>().join("|") }
fn sum_all<I: IntoIterator<Item = V2>>(items: I) -> V2 { items.into_iter().fold(V2::default(), |a, b| a + b) }
struct Counter { n: u32 }
impl Iterator for Counter { type Item = u32; fn next(&mut self) -> Option<u32> { if self.n < 5 { self.n += 1; Some(self.n) } else { None } } }
fn make_adder(k: i32) -> impl Fn(i32) -> i32 { move |x| x + k }
fn make_boxed(k: i32) -> Box<dyn Fn(i32) -> i32> { if k > 0 { Box::new(move |x| x * k) } else { Box::new(|x| -x) } }
fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C { move |x| g(f(x)) }
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
enum Level { Low = 1, Mid = 5, High = 10 }
fn main() {
    let mut a = V2 { x: 1.0, y: 2.0 };
    let b = V2 { x: 0.5, y: -1.0 };
    a += b;
    println!("{} {} {} {:?} {}", a + b, a - b, -(a * 2.0), a.partial_cmp(&b), a == b);
    let mut g = Grid { w: 3, cells: vec![0; 9] };
    g[(1, 2)] = 7; g[(0, 0)] += 1;
    println!("{} {} {:?}", g[(1, 2)], g[(0, 0)], g.cells);
    let shapes: Vec<Box<dyn Shape>> = vec![Box::new(Sq(2.0)), Box::new(Circ(1.0))];
    for s in &shapes { println!("{}", s.describe()); }
    println!("{} {}", Sq(1.0).label(), shapes.iter().map(|s| s.area()).sum::<f64>() > 7.0);
    let w = Words("hello big world".into());
    println!("{:?} {:?}", w.items(), w.first());
    println!("{} {} {}", largest(&[3, 9, 2]), largest(&[1.5, -2.0]), largest(&['x', 'b']));
    println!("{}", show_all(&[Some(1), None]));
    println!("{}", sum_all(vec![a, b, V2 { x: 1.0, y: 1.0 }]));
    println!("{:?} {}", Counter { n: 0 }.collect::<Vec<_>>(), Counter { n: 2 }.map(|x| x * 10).sum::<u32>());
    let add5 = make_adder(5);
    let f = compose(add5, |x| x * 2);
    println!("{} {} {}", f(1), make_boxed(3)(4), make_boxed(-1)(4));
    let fs: Vec<Box<dyn Fn(i32) -> i32>> = vec![Box::new(|x| x + 1), make_boxed(2), Box::new(make_adder(10))];
    println!("{:?}", fs.iter().map(|f| f(1)).collect::<Vec<_>>());
    let mut levels = vec![Level::High, Level::Low, Level::Mid];
    levels.sort();
    println!("{:?} {} {:?}", levels, Level::Mid as i32, Level::Low.max(Level::High));
    let mut counter = 0;
    let mut inc = || { counter += 3; counter };
    inc(); inc();
    println!("{}", counter);
    let strs: Vec<String> = vec!["b".into(), "a".into()];
    let mut sorted = strs.clone(); sorted.sort_by(|x, y| x.cmp(y));
    println!("{:?} {:?}", strs, sorted);
}
