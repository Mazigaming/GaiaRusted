fn apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 { f(x) }
fn apply_mut<F: FnMut()>(mut f: F) { f(); f(); }
fn consume<F: FnOnce() -> String>(f: F) -> String { f() }
fn make_adder(n: i32) -> impl Fn(i32) -> i32 { move |x| x + n }
fn make_boxed(n: i32) -> Box<dyn Fn(i32) -> i32> { Box::new(move |x| x * n) }
fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C { move |x| g(f(x)) }
fn main() {
    let k = 10;
    println!("{}", apply(|x| x + k, 5));
    let mut count = 0;
    apply_mut(|| count += 1);
    println!("{}", count);
    let s = String::from("owned");
    println!("{}", consume(move || s + "!"));
    let add5 = make_adder(5);
    let times3 = make_boxed(3);
    println!("{} {}", add5(1), times3(4));
    let both = compose(add5, times3);
    println!("{}", both(2));
    let fs: Vec<Box<dyn Fn(i32) -> i32>> = vec![Box::new(|x| x + 1), Box::new(|x| x * x), make_boxed(-1)];
    println!("{:?}", fs.iter().map(|f| f(7)).collect::<Vec<_>>());
    let ptr: fn(i32) -> i32 = |x| x - 1;
    println!("{}", apply(ptr, 1));
    let mut v = vec![3, 1, 2];
    v.sort_by(|a, b| b.cmp(a));
    println!("{:?}", v);
}
