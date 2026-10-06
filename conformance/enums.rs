// Enums with every kind of variant, matching, and methods.
#[derive(Debug, Clone, PartialEq)]
enum Shape {
    Circle(f64),
    Rect { w: f64, h: f64 },
    Triangle(f64, f64, f64),
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
enum Level {
    Low = 1,
    Mid = 5,
    High = 10,
}

impl Shape {
    fn area(&self) -> f64 {
        match self {
            Shape::Circle(r) => 3.0 * r * r,
            Shape::Rect { w, h } => w * h,
            Shape::Triangle(a, b, c) => {
                let s = (a + b + c) / 2.0;
                (s * (s - a) * (s - b) * (s - c)).sqrt()
            }
            Shape::Empty => 0.0,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Shape::Circle(_) => "circle",
            Shape::Rect { .. } => "rect",
            Shape::Triangle(..) => "triangle",
            Shape::Empty => "empty",
        }
    }
}

fn describe(n: i32) -> &'static str {
    match n {
        i32::MIN..=-1 => "negative",
        0 => "zero",
        1 | 2 | 3 => "few",
        x if x % 2 == 0 => "even",
        _ => "odd",
    }
}

fn main() {
    let shapes = vec![
        Shape::Circle(1.0),
        Shape::Rect { w: 2.0, h: 3.5 },
        Shape::Triangle(3.0, 4.0, 5.0),
        Shape::Empty,
    ];
    for shape in &shapes {
        println!("{} {}", shape.name(), shape.area());
    }
    println!("{:?}", shapes[1]);
    println!("{}", shapes[0] == Shape::Circle(1.0));

    println!("{} {}", Level::Mid as i32, Level::High as i32);
    println!("{}", Level::Low < Level::High);
    println!("{:?}", Level::Mid);

    for n in [-3, 0, 2, 8, 9] {
        println!("{}", describe(n));
    }

    let maybe: Option<Shape> = Some(Shape::Empty);
    if let Some(Shape::Empty) = maybe {
        println!("empty inside");
    }
    let pair = (Level::Low, Some(3));
    match pair {
        (Level::Low, Some(n)) if n > 2 => println!("low {}", n),
        (_, None) => println!("none"),
        _ => println!("other"),
    }
    let code = match Level::High {
        Level::Low => 'l',
        Level::Mid => 'm',
        Level::High => 'h',
    };
    println!("{}", code);
    let n @ 1..=9 = 5 else { panic!() };
    println!("{}", n);
}
