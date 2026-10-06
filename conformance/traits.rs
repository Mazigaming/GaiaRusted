// Traits: static dispatch, default methods, generics with bounds,
// associated types, operator overloading and trait objects.
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

trait Animal {
    fn name(&self) -> String;
    fn legs(&self) -> u32 {
        4
    }
    fn describe(&self) -> String {
        format!("{} with {} legs", self.name(), self.legs())
    }
}

struct Dog;
struct Bird {
    species: String,
}

impl Animal for Dog {
    fn name(&self) -> String {
        "dog".to_string()
    }
}

impl Animal for Bird {
    fn name(&self) -> String {
        self.species.clone()
    }
    fn legs(&self) -> u32 {
        2
    }
}

fn total_legs(animals: &[Box<dyn Animal>]) -> u32 {
    animals.iter().map(|a| a.legs()).sum()
}

fn loudest<T: Animal>(a: &T) -> String {
    a.describe().to_uppercase()
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct V2 {
    x: f64,
    y: f64,
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        V2 { x: self.x + o.x, y: self.y + o.y }
    }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        V2 { x: self.x - o.x, y: self.y - o.y }
    }
}
impl Mul<f64> for V2 {
    type Output = V2;
    fn mul(self, k: f64) -> V2 {
        V2 { x: self.x * k, y: self.y * k }
    }
}
impl Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        V2 { x: -self.x, y: -self.y }
    }
}
impl fmt::Display for V2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

trait Stack {
    type Item;
    fn push_item(&mut self, item: Self::Item);
    fn pop_item(&mut self) -> Option<Self::Item>;
}

struct Numbers {
    items: Vec<i32>,
}

impl Stack for Numbers {
    type Item = i32;
    fn push_item(&mut self, item: i32) {
        self.items.push(item);
    }
    fn pop_item(&mut self) -> Option<i32> {
        self.items.pop()
    }
}

fn drain<S: Stack>(stack: &mut S) -> u32 {
    let mut count = 0;
    while let Some(_) = stack.pop_item() {
        count += 1;
    }
    count
}

struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0 + 1)
        }
    }
}

fn largest<T: PartialOrd + Copy>(items: &[T]) -> T {
    let mut best = items[0];
    for &item in items {
        if item > best {
            best = item;
        }
    }
    best
}

fn main() {
    let dog = Dog;
    let bird = Bird { species: "owl".to_string() };
    println!("{}", dog.describe());
    println!("{}", bird.describe());
    println!("{}", loudest(&bird));
    let zoo: Vec<Box<dyn Animal>> = vec![Box::new(Dog), Box::new(bird), Box::new(Dog)];
    println!("{}", total_legs(&zoo));
    let any: &dyn Animal = &dog;
    println!("{}", any.name());

    let a = V2 { x: 1.0, y: 2.0 };
    let b = V2 { x: 0.5, y: -1.0 };
    println!("{}", a + b);
    println!("{}", (a - b) * 2.0);
    println!("{:?}", -a);
    println!("{}", a == a + b - b);

    let mut numbers = Numbers { items: vec![1, 2, 3] };
    numbers.push_item(4);
    println!("{:?}", numbers.pop_item());
    println!("{}", drain(&mut numbers));

    let collected: Vec<u32> = Countdown(4).collect();
    println!("{:?}", collected);
    println!("{}", Countdown(5).map(|n| n * n).filter(|n| n % 2 == 1).sum::<u32>());
    for n in Countdown(2) {
        print!("{} ", n);
    }
    println!();
    println!("{} {} {}", largest(&[3, 9, 2]), largest(&[1.5, 0.5]), largest(&['x', 'b']));
}
