// Closures: capture by reference, by mutable reference and by move;
// closures as arguments, return values and struct fields.
fn apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 {
    f(x)
}

fn call_n<F: FnMut()>(mut f: F, n: usize) {
    for _ in 0..n {
        f();
    }
}

fn make_adder(k: i32) -> Box<dyn Fn(i32) -> i32> {
    Box::new(move |x| x + k)
}

fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

struct Counter<F: Fn(u32) -> u32> {
    step: F,
    value: u32,
}

impl<F: Fn(u32) -> u32> Counter<F> {
    fn advance(&mut self) -> u32 {
        self.value = (self.step)(self.value);
        self.value
    }
}

fn double(x: i32) -> i32 {
    x * 2
}

fn main() {
    let offset = 10;
    println!("{}", apply(|x| x + offset, 5));
    println!("{}", apply(double, 21));

    let mut total = 0;
    call_n(|| total += 3, 4);
    println!("{}", total);

    let mut log = Vec::new();
    let mut record = |s: &str| log.push(s.to_string());
    record("a");
    record("b");
    println!("{:?}", log);

    let name = String::from("gaia");
    let greet = move |greeting: &str| format!("{}, {}", greeting, name);
    println!("{}", greet("hello"));

    let add5 = make_adder(5);
    println!("{}", add5(1) + add5(2));

    let mut counter = Counter { step: |v| v * 2 + 1, value: 0 };
    counter.advance();
    counter.advance();
    println!("{}", counter.advance());

    let fp: fn(i32) -> i32 = double;
    println!("{}", fp(4));
    let table: Vec<fn(i32) -> i32> = vec![double, |x| x - 1];
    println!("{}", table.iter().map(|f| f(10)).sum::<i32>());

    let squares: Vec<i32> = (1..=5).map(|n| n * n).collect();
    let evens: Vec<&i32> = squares.iter().filter(|n| *n % 2 == 0).collect();
    println!("{:?} {:?}", squares, evens);
    let lengths: Vec<usize> = vec!["a", "bbb"].iter().map(|s| s.len()).collect();
    println!("{:?}", lengths);
}
