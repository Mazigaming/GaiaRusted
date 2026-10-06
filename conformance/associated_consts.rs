// Associated constants: required, defaulted, and defaults built on others.
trait Shape {
    const SIDES: u32;
    const NAME: &'static str = "shape";
    const DOUBLE: u32 = Self::SIDES * 2;
    fn describe(&self) -> String { format!("{} with {} sides ({})", Self::NAME, Self::SIDES, Self::DOUBLE) }
}
struct Tri;
struct Square;
impl Shape for Tri { const SIDES: u32 = 3; }
impl Shape for Square { const SIDES: u32 = 4; const NAME: &'static str = "square"; }
fn total<T: Shape, U: Shape>() -> u32 { T::SIDES + U::SIDES + <T as Shape>::DOUBLE }
fn main() {
    println!("{}", Tri.describe());
    println!("{}", Square.describe());
    println!("{} {} {}", Tri::NAME, <Square as Shape>::NAME, total::<Tri, Square>());
}
