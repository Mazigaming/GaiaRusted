#[derive(Debug, Clone)]
struct Point { x: f64, y: f64 }
struct Pair(i32, i32);
struct Unit;
impl Point {
    fn new(x: f64, y: f64) -> Self { Point { x, y } }
    fn origin() -> Point { Point { x: 0.0, y: 0.0 } }
    fn dist(&self, o: &Point) -> f64 { ((self.x - o.x).powi(2) + (self.y - o.y).powi(2)).sqrt() }
    fn translate(&mut self, dx: f64) { self.x += dx; }
    fn into_tuple(self) -> (f64, f64) { (self.x, self.y) }
}
impl Point { fn scale(&self, k: f64) -> Point { Point { x: self.x * k, ..self.clone() } } }
impl Unit { fn name(&self) -> &str { "unit" } }
fn main() {
    let mut p = Point::new(3.0, 4.0);
    println!("{}", p.dist(&Point::origin()));
    p.translate(1.0);
    println!("{:?}", p);
    println!("{:?}", p.scale(2.0));
    let q = Point { y: 9.0, ..p.clone() };
    println!("{:?}", q);
    println!("{:?}", p.into_tuple());
    let pr = Pair(1, 2);
    let Pair(a, b) = pr;
    println!("{} {} {}", a, b, pr.0 + pr.1);
    println!("{}", Unit.name());
    println!("{}", std::mem::size_of::<Point>());
}
