mod geometry {
    pub mod shapes {
        #[derive(Debug)]
        pub struct Square { pub side: f64, secret: u8 }
        impl Square { pub fn new(side: f64) -> Self { Square { side, secret: 7 } } pub fn secret(&self) -> u8 { self.secret } }
        pub fn unit() -> Square { Square::new(1.0) }
    }
    pub fn area(s: &shapes::Square) -> f64 { s.side * s.side }
    pub(crate) fn helper() -> &'static str { super::top_level() }
    pub const SCALE: f64 = 2.0;
}
mod util {
    pub fn double(x: i32) -> i32 { x * 2 }
    pub mod inner { pub fn triple(x: i32) -> i32 { super::double(x) + x } }
}
fn top_level() -> &'static str { "top" }
use geometry::shapes::{self, Square};
use util::inner::triple as thrice;
fn main() {
    let s = Square::new(3.0);
    println!("{} {} {}", geometry::area(&s), s.secret(), s.side);
    println!("{:?}", shapes::unit());
    println!("{} {}", geometry::helper(), geometry::SCALE);
    println!("{} {}", util::double(4), thrice(5));
    println!("{}", crate::util::double(1));
}
