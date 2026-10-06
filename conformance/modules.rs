// Inline modules, nested paths, constants, statics and shadowing.
mod geometry {
    pub const UNIT: f64 = 1.0;

    pub mod shapes {
        pub struct Square(pub f64);

        impl Square {
            pub fn area(&self) -> f64 {
                self.0 * self.0
            }
        }

        pub fn unit() -> Square {
            Square(super::UNIT)
        }
    }

    pub fn scale(by: f64) -> shapes::Square {
        shapes::Square(UNIT * by)
    }
}

mod counters {
    pub static START: i64 = 100;
    const STEP: i64 = 5;

    pub fn after(n: i64) -> i64 {
        START + STEP * n
    }
}

use geometry::shapes::{self, Square};

const LIMIT: usize = 3;
const DOUBLE: usize = LIMIT * 2;
static GREETING: &str = "hi";

fn main() {
    println!("{}", shapes::unit().area());
    println!("{}", geometry::scale(3.0).area());
    let s: Square = Square(2.0);
    println!("{}", s.area());
    println!("{} {}", counters::after(2), counters::START);
    println!("{} {} {}", LIMIT, DOUBLE, GREETING);
    let table = [0u8; DOUBLE];
    println!("{}", table.len());

    let x = 1;
    let x = x + 1;
    {
        let x = x * 10;
        println!("{}", x);
    }
    println!("{}", x);

    fn helper(n: i32) -> i32 {
        n + 1
    }
    struct Local {
        v: i32,
    }
    let l = Local { v: helper(1) };
    println!("{}", l.v);
}
