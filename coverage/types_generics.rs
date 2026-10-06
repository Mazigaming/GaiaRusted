use std::fmt::Display;
fn largest<T: PartialOrd + Copy>(xs: &[T]) -> T { let mut m = xs[0]; for &x in xs { if x > m { m = x; } } m }
struct Wrapper<T> { value: T }
impl<T: Display> Wrapper<T> { fn show(&self) -> String { format!("[{}]", self.value) } }
impl<T> Wrapper<T> { fn map<U, F: Fn(T) -> U>(self, f: F) -> Wrapper<U> { Wrapper { value: f(self.value) } } }
struct Pair<A, B> { a: A, b: B }
impl<A: Display, B: Display> Pair<A, B> { fn describe(&self) -> String where A: Clone { format!("{}+{}", self.a, self.b) } }
fn print_all<T>(items: &[T]) where T: Display { for i in items { print!("{} ", i); } println!(); }
fn main() {
    println!("{} {} {}", largest(&[3, 9, 2]), largest(&[1.5, 0.2]), largest(&['x', 'b']));
    let w = Wrapper { value: 21 };
    println!("{}", w.show());
    let w2 = w.map(|v| v as f64 * 2.0);
    println!("{}", w2.show());
    println!("{}", Pair { a: "x", b: 7 }.describe());
    print_all(&["a", "b"]);
    print_all(&[1, 2, 3]);
    let v: Vec<Option<i32>> = vec![Some(1), None];
    println!("{:?}", v);
    println!("{}", std::any::type_name::<Wrapper<i32>>().contains("Wrapper"));
}
