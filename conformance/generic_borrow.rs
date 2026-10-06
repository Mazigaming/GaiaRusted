// Generic code over K: Borrow<Q> calls borrow() through the bound, whatever
// K is: a reference, an owned String, a Box<str>, a number.
use std::borrow::Borrow;
fn find<K: Borrow<Q>, Q: Eq + ?Sized>(items: &[K], key: &Q) -> Option<usize> {
    for (index, item) in items.iter().enumerate() {
        if key.eq(item.borrow()) {
            return Some(index);
        }
    }
    None
}
fn contains<T: Borrow<str>>(items: &[T], wanted: &str) -> bool {
    items.iter().any(|item| item.borrow() == wanted)
}
fn main() {
    let words = ["alpha", "beta"];
    println!("{:?} {:?}", find(&words, "beta"), find(&words, "gamma"));
    let owned = vec![String::from("x"), String::from("y")];
    println!("{:?} {}", find(&owned, "y"), contains(&owned, "x"));
    println!("{}", contains(&words, "beta"));
    let numbers = vec![1, 2, 3];
    println!("{:?}", find(&numbers, &3));
    let boxed: Vec<Box<str>> = vec!["p".into(), "q".into()];
    println!("{:?}", find(&boxed, "q"));
}
