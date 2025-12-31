// Comprehensive test suite for impl blocks

struct Counter {
    value: i32,
}

impl Counter {
    fn new() -> Counter {
        Counter { value: 0 }
    }

    fn increment(&mut self) {
        self.value = self.value + 1;
    }

    fn get_value(&self) -> i32 {
        self.value
    }
}

struct Rectangle {
    width: i32,
    height: i32,
}

impl Rectangle {
    fn area(&self) -> i32 {
        self.width * self.height
    }

    fn perimeter(&self) -> i32 {
        2 * (self.width + self.height)
    }

    fn scale(&mut self, factor: i32) {
        self.width = self.width * factor;
        self.height = self.height * factor;
    }

    fn is_square(&self) -> bool {
        self.width == self.height
    }
}

struct MyBox {
    value: i32,
}

impl MyBox {
    fn new(v: i32) -> MyBox {
        MyBox { value: v }
    }

    fn get(&self) -> i32 {
        self.value
    }

    fn set(&mut self, v: i32) {
        self.value = v;
    }

    fn replace(&mut self, v: i32) -> i32 {
        let old = self.value;
        self.value = v;
        old
    }
}

struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    fn distance(&self, other: &Point) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    fn midpoint(&self, other: &Point) -> Point {
        Point {
            x: (self.x + other.x) / 2,
            y: (self.y + other.y) / 2,
        }
    }
}

fn main() {
    test_basic_impl_block();
    test_multiple_methods();
    test_generic_impl_block();
    test_trait_impl();
}

fn test_basic_impl_block() {
    let mut c = Counter::new();
    c.increment();
    c.increment();
    let val = c.get_value();
    println!("Counter value: {}", val);
}

fn test_multiple_methods() {
    let mut rect = Rectangle { width: 5, height: 3 };
    println!("Area: {}", rect.area());
    println!("Perimeter: {}", rect.perimeter());
    println!("Is square: {}", rect.is_square());

    rect.scale(2);
    println!("Area after scale: {}", rect.area());
}

fn test_generic_impl_block() {
    let mut b = MyBox::new(42);
    let val = b.get();
    println!("Box value: {}", val);

    b.set(100);
    let val2 = b.get();
    println!("Box value after set: {}", val2);

    let old = b.replace(200);
    println!("Old value: {}, new value: {}", old, b.get());
}

fn test_trait_impl() {
    let p1 = Point::new(0, 0);
    let p2 = Point::new(10, 10);
    let dist = p1.distance(&p2);
    println!("Distance: {}", dist);

    let mid = p1.midpoint(&p2);
    println!("Midpoint: ({}, {})", mid.x, mid.y);
}
