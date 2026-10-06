// Dynamic dispatch, supertraits, generic traits and trait objects in collections.
use std::collections::HashMap;
use std::fmt::{self, Debug, Display};

trait Shape: Debug {
    fn area(&self) -> f64;
    fn name(&self) -> String;
    fn scale(&mut self, factor: f64);
    fn summary(&self) -> String {
        format!("{} with area {:.1}", self.name(), self.area())
    }
}

#[derive(Debug)]
struct Circle {
    radius: f64,
}

#[derive(Debug)]
struct Rect {
    w: f64,
    h: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.14159 * self.radius * self.radius
    }
    fn name(&self) -> String {
        "circle".into()
    }
    fn scale(&mut self, factor: f64) {
        self.radius *= factor;
    }
}

impl Shape for Rect {
    fn area(&self) -> f64 {
        self.w * self.h
    }
    fn name(&self) -> String {
        String::from("rect")
    }
    fn scale(&mut self, factor: f64) {
        self.w *= factor;
        self.h *= factor;
    }
    fn summary(&self) -> String {
        format!("{}x{} rect", self.w, self.h)
    }
}

trait Converter<T> {
    fn convert(&self, input: &str) -> T;
}

struct Length;
struct Upper;

impl Converter<usize> for Length {
    fn convert(&self, input: &str) -> usize {
        input.len()
    }
}

impl Converter<String> for Upper {
    fn convert(&self, input: &str) -> String {
        input.to_uppercase()
    }
}

fn run<T: Display>(converter: &dyn Converter<T>, input: &str) -> String {
    format!("<{}>", converter.convert(input))
}

struct Registry {
    handlers: HashMap<String, Box<dyn Fn(i64) -> i64>>,
}

impl Registry {
    fn call(&self, name: &str, arg: i64) -> Option<i64> {
        self.handlers.get(name).map(|handler| handler(arg))
    }
}

struct Wrapper<T>(T);

impl<T: Display> fmt::Display for Wrapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}]", self.0)
    }
}

fn largest_area(shapes: &[Box<dyn Shape>]) -> &dyn Shape {
    let mut best = &shapes[0];
    for shape in shapes {
        if shape.area() > best.area() {
            best = shape;
        }
    }
    best.as_ref()
}

fn main() {
    let mut shapes: Vec<Box<dyn Shape>> = vec![Box::new(Circle { radius: 1.0 }), Box::new(Rect { w: 2.0, h: 5.0 })];
    for shape in &shapes {
        println!("{}", shape.summary());
    }
    for shape in shapes.iter_mut() {
        shape.scale(2.0);
    }
    println!("{:?}", shapes);
    println!("{}", largest_area(&shapes).name());
    let total: f64 = shapes.iter().map(|s| s.area()).sum();
    println!("{:.2}", total);

    println!("{} {}", run(&Length, "hello"), run(&Upper, "hello"));

    let mut registry = Registry { handlers: HashMap::new() };
    let offset = 100;
    registry.handlers.insert("double".to_string(), Box::new(|x| x * 2));
    registry.handlers.insert("shift".to_string(), Box::new(move |x| x + offset));
    println!("{:?} {:?} {:?}", registry.call("double", 21), registry.call("shift", 1), registry.call("nope", 0));

    println!("{} {}", Wrapper(5), Wrapper("text"));
    let printable: Vec<Box<dyn Display>> = vec![Box::new(1), Box::new("two"), Box::new(3.5)];
    for item in &printable {
        print!("{} ", item);
    }
    println!();
}
