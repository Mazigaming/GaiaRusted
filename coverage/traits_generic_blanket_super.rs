use std::fmt::{Debug, Display};
trait Describe { fn describe(&self) -> String; }
impl<T: Debug> Describe for T { fn describe(&self) -> String { format!("<{:?}>", self) } }
trait Named { fn name(&self) -> String; }
trait Greeter: Named { fn greet(&self) -> String { format!("Hello, {}", self.name()) } }
struct P;
impl Named for P { fn name(&self) -> String { "P".into() } }
impl Greeter for P {}
trait Converter<T> { fn convert(&self) -> T; }
impl Converter<String> for i32 { fn convert(&self) -> String { format!("#{}", self) } }
impl Converter<f64> for i32 { fn convert(&self) -> f64 { *self as f64 / 2.0 } }
fn pair_up<T: Display, U: Display>(t: T, u: U) -> String { format!("{}{}", t, u) }
trait Zero { fn zero() -> Self; }
impl Zero for i32 { fn zero() -> Self { 0 } }
impl Zero for f64 { fn zero() -> Self { 0.0 } }
fn sum_all<T: Zero + std::ops::Add<Output = T> + Copy>(xs: &[T]) -> T { xs.iter().fold(T::zero(), |a, &b| a + b) }
fn main() {
    println!("{} {}", 5.describe(), "s".describe());
    println!("{}", vec![1, 2].describe());
    println!("{}", P.greet());
    let s: String = 7.convert();
    let f: f64 = 7.convert();
    println!("{} {}", s, f);
    println!("{}", pair_up(1, 'z'));
    println!("{} {}", sum_all(&[1, 2, 3]), sum_all(&[0.5, 0.25]));
}
