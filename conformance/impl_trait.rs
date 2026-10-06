// impl Trait in argument and return position, boxed trait objects with
// associated types, and closures that capture, are boxed or composed.
use std::fmt::Display;
fn evens(limit: u32) -> impl Iterator<Item = u32> { (0..limit).filter(|x| x % 2 == 0) }
fn show_all(items: impl IntoIterator<Item = impl Display>) -> String { items.into_iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",") }
fn counter() -> impl FnMut() -> u32 { let mut c = 0; move || { c += 1; c } }
fn boxed_iter<'a>(v: &'a [i32], rev: bool) -> Box<dyn Iterator<Item = &'a i32> + 'a> { if rev { Box::new(v.iter().rev()) } else { Box::new(v.iter()) } }
fn impl_traits() {
    println!("{:?}", evens(10).collect::<Vec<_>>());
    println!("{}", show_all(vec![1, 2, 3]));
    println!("{}", show_all(["a", "b"]));
    let mut c = counter();
    println!("{} {} {}", c(), c(), c());
    println!("{:?}", boxed_iter(&[1, 2, 3], true).collect::<Vec<_>>());
    println!("{:?}", boxed_iter(&[1, 2, 3], false).map(|x| x * 2).collect::<Vec<_>>());
}

fn apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 { f(x) }
fn apply_mut<F: FnMut()>(mut f: F) { f(); f(); }
fn consume<F: FnOnce() -> String>(f: F) -> String { f() }
fn make_adder(n: i32) -> impl Fn(i32) -> i32 { move |x| x + n }
fn make_boxed(n: i32) -> Box<dyn Fn(i32) -> i32> { Box::new(move |x| x * n) }
fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C { move |x| g(f(x)) }
fn closures() {
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

fn main() {
    impl_traits();
    closures();
}
