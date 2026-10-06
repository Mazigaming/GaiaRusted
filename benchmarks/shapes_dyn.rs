// Dynamic dispatch over a heterogeneous collection of trait objects.
trait Shape { fn area(&self) -> f64; fn scale(&mut self, k: f64); }
struct Circle { r: f64 }
struct Rect { w: f64, h: f64 }
struct Tri { a: f64, b: f64, c: f64 }
impl Shape for Circle { fn area(&self) -> f64 { 3.141592653589793 * self.r * self.r } fn scale(&mut self, k: f64) { self.r *= k; } }
impl Shape for Rect { fn area(&self) -> f64 { self.w * self.h } fn scale(&mut self, k: f64) { self.w *= k; self.h *= k; } }
impl Shape for Tri {
    fn area(&self) -> f64 { let s = (self.a + self.b + self.c) / 2.0; (s * (s - self.a) * (s - self.b) * (s - self.c)).sqrt() }
    fn scale(&mut self, k: f64) { self.a *= k; self.b *= k; self.c *= k; }
}
fn main() {
    let mut shapes: Vec<Box<dyn Shape>> = Vec::new();
    for i in 0..30_000 {
        let x = (i % 17) as f64 + 1.0;
        match i % 3 { 0 => shapes.push(Box::new(Circle { r: x })), 1 => shapes.push(Box::new(Rect { w: x, h: x + 1.0 })), _ => shapes.push(Box::new(Tri { a: x + 2.0, b: x + 3.0, c: x + 4.0 })) }
    }
    let mut total = 0.0;
    for round in 0..1000 {
        let k = if round % 2 == 0 { 1.01 } else { 1.0 / 1.01 };
        for s in shapes.iter_mut() { s.scale(k); }
        total += shapes.iter().map(|s| s.area()).sum::<f64>();
    }
    println!("{:.3}", total);
}
