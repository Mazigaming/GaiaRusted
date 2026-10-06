trait Shape { fn area(&self) -> f64; fn name(&self) -> &str { "shape" } }
struct Circle(f64); struct Square(f64);
impl Shape for Circle { fn area(&self) -> f64 { 3.14 * self.0 * self.0 } fn name(&self) -> &str { "circle" } }
impl Shape for Square { fn area(&self) -> f64 { self.0 * self.0 } }
fn total(shapes: &[Box<dyn Shape>]) -> f64 { shapes.iter().map(|s| s.area()).sum() }
fn describe(s: &dyn Shape) -> String { format!("{} {:.2}", s.name(), s.area()) }
fn make(kind: u8) -> Box<dyn Shape> { if kind == 0 { Box::new(Circle(1.0)) } else { Box::new(Square(2.0)) } }
fn main() {
    let shapes: Vec<Box<dyn Shape>> = vec![Box::new(Circle(2.0)), Box::new(Square(3.0)), make(0)];
    println!("{:.2}", total(&shapes));
    for s in &shapes { println!("{}", describe(s.as_ref())); }
    let refs: Vec<&dyn Shape> = vec![&Circle(1.0), &Square(1.0)];
    println!("{}", refs.len());
    let biggest = shapes.iter().max_by(|a, b| a.area().partial_cmp(&b.area()).unwrap()).unwrap();
    println!("{}", biggest.name());
}
