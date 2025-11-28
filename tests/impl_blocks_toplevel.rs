// Test top-level impl blocks

struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    fn get_x(&self) -> i32 {
        self.x
    }
}

fn main() {
    let p = Point::new(3, 4);
    let x = p.get_x();
    println!("Point x: {}", x);
}
