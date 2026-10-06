use std::fmt::Display;
fn evens(limit: u32) -> impl Iterator<Item = u32> { (0..limit).filter(|x| x % 2 == 0) }
fn show_all(items: impl IntoIterator<Item = impl Display>) -> String { items.into_iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",") }
fn counter() -> impl FnMut() -> u32 { let mut c = 0; move || { c += 1; c } }
fn boxed_iter<'a>(v: &'a [i32], rev: bool) -> Box<dyn Iterator<Item = &'a i32> + 'a> { if rev { Box::new(v.iter().rev()) } else { Box::new(v.iter()) } }
fn main() {
    println!("{:?}", evens(10).collect::<Vec<_>>());
    println!("{}", show_all(vec![1, 2, 3]));
    println!("{}", show_all(["a", "b"]));
    let mut c = counter();
    println!("{} {} {}", c(), c(), c());
    println!("{:?}", boxed_iter(&[1, 2, 3], true).collect::<Vec<_>>());
    println!("{:?}", boxed_iter(&[1, 2, 3], false).map(|x| x * 2).collect::<Vec<_>>());
}
