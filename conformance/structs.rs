// Structs: nesting, methods, by-value and by-reference passing, updates.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Point {
    x: i64,
    y: i64,
}

#[derive(Debug, Clone, PartialEq)]
struct Rect {
    origin: Point,
    size: Point,
    label: String,
}

struct Pair(i32, f64);
struct Marker;

impl Point {
    fn new(x: i64, y: i64) -> Point {
        Point { x, y }
    }
    fn manhattan(self) -> i64 {
        self.x.abs() + self.y.abs()
    }
    fn translate(&mut self, dx: i64, dy: i64) {
        self.x += dx;
        self.y += dy;
    }
    fn swapped(&self) -> Point {
        Point { x: self.y, y: self.x }
    }
}

impl Rect {
    fn area(&self) -> i64 {
        self.size.x * self.size.y
    }
    fn grow(&mut self, by: i64) {
        self.size.x += by;
        self.size.y += by;
    }
    fn corner(&self) -> Point {
        Point { x: self.origin.x + self.size.x, y: self.origin.y + self.size.y }
    }
}

fn make_rect(scale: i64) -> Rect {
    Rect { origin: Point::new(1, 2), size: Point::new(3 * scale, 4 * scale), label: String::from("r") }
}

fn sum_coords(p: Point) -> i64 {
    p.x + p.y
}

fn main() {
    let mut p = Point::new(3, -4);
    println!("{}", p.manhattan());
    p.translate(10, 10);
    println!("{:?}", p);
    println!("{:?}", p.swapped());
    println!("{}", sum_coords(p));
    let q = p;
    println!("{}", p == q);

    let mut r = make_rect(2);
    println!("{} {}", r.area(), r.corner().y);
    r.grow(1);
    r.origin.x = 100;
    r.origin = r.origin.swapped();
    println!("{:?}", r);
    let moved = Rect { label: String::from("copy"), ..r.clone() };
    println!("{} {}", moved.label, moved == r);

    let pair = Pair(7, 1.5);
    println!("{} {}", pair.0, pair.1);
    let _m = Marker;
    let nested = ((1, 2), (3, (4, 5)));
    println!("{}", (nested.0).1 + ((nested.1).1).0);
    let (a, (b, c)) = (1, (2.5, "three"));
    println!("{} {} {}", a, b, c);
    p = Point { x: p.y, y: p.x };
    println!("{:?}", p);
}
