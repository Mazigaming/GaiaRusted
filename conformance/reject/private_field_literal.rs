// error: field `h` of struct `Rect` is private
mod shapes {
    #[derive(Default)]
    pub struct Rect { pub w: u32, h: u32 }
    pub struct Token(u8);
    pub struct Meters(pub f64);
    fn helper() {}
    const LIMIT: u32 = 10;
    mod hidden { pub fn f() {} }
    impl Rect {
        pub fn new() -> Rect { Rect { w: 1, h: 2 } }
        fn area(&self) -> u32 { self.w * self.h }
        fn make() -> Rect { Rect::default() }
    }
}
use shapes::{Meters, Rect, Token};
fn main() {
    let r = Rect { w: 1, h: 2 }; println!("{}", r.w);
}
