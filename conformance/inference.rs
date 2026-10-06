// Type inference that depends on information arriving later, and on the
// type a result is expected to have.
struct Celsius(f64);
struct Fahrenheit(f64);
impl From<Celsius> for Fahrenheit {
    fn from(c: Celsius) -> Self { Fahrenheit(c.0 * 9.0 / 5.0 + 32.0) }
}

trait Convert<T> { fn convert(&self) -> T; }
impl Convert<String> for i32 { fn convert(&self) -> String { format!("#{}", self) } }
impl Convert<f64> for i32 { fn convert(&self) -> f64 { *self as f64 / 2.0 } }

fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

fn consume<F: FnOnce() -> String>(f: F) -> String { f() }

fn halve(n: usize) -> usize { n / 2 }

fn main() {
    let f: Fahrenheit = Celsius(100.0).into();
    println!("{}", f.0);
    let g = Fahrenheit::from(Celsius(-40.0));
    println!("{}", g.0);

    let text: String = 7.convert();
    let number: f64 = 7.convert();
    println!("{} {}", text, number);

    let add_then_double = compose(|x: i32| x + 1, |y: i32| y * 2);
    println!("{}", add_then_double(3));
    let describe = compose(|n: u32| n * n, |sq: u32| format!("<{}>", sq));
    println!("{}", describe(12));
    let owned = String::from("moved");
    println!("{}", consume(move || owned + "!"));

    // `n` is a `usize` because of `halve(n)`, even though casts come first.
    let n = 400;
    let squares: Vec<f64> = (0..n).map(|i| (i % 7) as f64).collect();
    println!("{} {} {}", squares.len(), squares[10], halve(n));
    let byte = 65u8;
    println!("{}", byte as char);
}
