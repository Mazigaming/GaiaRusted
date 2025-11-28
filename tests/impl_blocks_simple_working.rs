struct Point {
    x: i32,
}

impl Point {
    fn new(x: i32) -> Point {
        Point { x }
    }

    fn get_x(&self) -> i32 {
        self.x
    }
}

fn main() {
    let p = Point::new(42);
    println!("Success");
}
