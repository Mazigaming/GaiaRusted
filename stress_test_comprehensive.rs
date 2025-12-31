struct Point {
    x: i32,
    y: i32,
}

struct Counter {
    count: i32,
}

impl Counter {
    fn new(start: i32) -> Counter {
        Counter { count: start }
    }
    
    fn increment(&mut self) {
        self.count = self.count + 1;
    }
    
    fn get(&self) -> i32 {
        self.count
    }
}

fn main() {
    println!("=== ARITHMETIC TESTS ===");
    
    let a = 5;
    let b = 3;
    println!("5 + 3 = {}", a + b);
    println!("5 - 3 = {}", a - b);
    println!("5 * 3 = {}", a * b);
    println!("5 / 3 = {}", a / b);
    println!("5 % 3 = {}", a % b);
    
    println!("=== NEGATIVE NUMBERS ===");
    let neg = -10;
    println!("neg = {}", neg);
    println!("-10 + 5 = {}", neg + 5);
    println!("-10 * 2 = {}", neg * 2);
    
    println!("=== COMPARISONS ===");
    let x = 10;
    let y = 20;
    if x < y {
        println!("10 < 20: true");
    } else {
        println!("10 < 20: false");
    }
    
    if x > y {
        println!("10 > 20: true");
    } else {
        println!("10 > 20: false");
    }
    
    if x == 10 {
        println!("10 == 10: true");
    }
    
    if x != y {
        println!("10 != 20: true");
    }
    
    println!("=== FLOAT OPERATIONS ===");
    let f1: f64 = 3.14;
    let f2: f64 = 2.0;
    println!("3.14 + 2.0 = {}", f1 + f2);
    println!("3.14 * 2.0 = {}", f1 * f2);
    
    println!("=== VECTOR OPERATIONS ===");
    let mut v = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    println!("Vector len: {}", v.len());
    println!("Vector[0]: {}", v.get(0));
    println!("Vector[1]: {}", v.get(1));
    println!("Vector[2]: {}", v.get(2));
    
    println!("=== VECTOR ITERATION ===");
    let mut sum = 0;
    for item in v {
        sum = sum + item;
    }
    println!("Sum: {}", sum);
    
    println!("=== TUPLE UNPACKING ===");
    let (a, b, c) = (10, 20, 30);
    println!("Tuple unpacking: a={}, b={}, c={}", a, b, c);
    
    println!("=== STRUCT TESTS ===");
    let p = Point { x: 5, y: 10 };
    println!("Point: x={}, y={}", p.x, p.y);
    
    println!("=== BOOLEAN LOGIC ===");
    let t = true;
    let f = false;
    if t && t {
        println!("true && true = true");
    }
    if t && f {
        println!("true && false = true");
    } else {
        println!("true && false = false");
    }
    if t || f {
        println!("true || false = true");
    }
    if !f {
        println!("!false = true");
    }
    
    println!("=== STRING OPERATIONS ===");
    let s1 = "Hello";
    let s2 = "World";
    println!("Strings: {} {}", s1, s2);
    
    println!("=== MATCH STATEMENTS ===");
    let num = 2;
    match num {
        1 => println!("one"),
        2 => println!("two"),
        3 => println!("three"),
        _ => println!("other"),
    }
    
    println!("=== WHILE LOOPS ===");
    let mut i = 0;
    while i < 3 {
        println!("While iteration: {}", i);
        i = i + 1;
    }
    
    println!("=== METHOD CALLS ===");
    let mut c = Counter::new(5);
    println!("Initial count: {}", c.get());
    c.increment();
    println!("After increment: {}", c.get());
    
    println!("=== NESTED STRUCTURES ===");
    let nested = (1, (2, 3), 4);
    println!("Nested tuple test");
}
