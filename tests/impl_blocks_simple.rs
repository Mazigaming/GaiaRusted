// Test simple impl blocks

fn main() {
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

        fn get_y(&self) -> i32 {
            self.y
        }

        fn translate(&mut self, dx: i32, dy: i32) {
            self.x = self.x + dx;
            self.y = self.y + dy;
        }

        fn distance_from_origin(self) -> i32 {
            self.x + self.y
        }
    }

    let mut p = Point::new(3, 4);
    let x = p.get_x();
    let y = p.get_y();
    println!("Point: ({}, {})", x, y);

    p.translate(1, 1);
    let x2 = p.get_x();
    let y2 = p.get_y();
    println!("After translate: ({}, {})", x2, y2);

    let dist = p.distance_from_origin();
    println!("Distance: {}", dist);
}
