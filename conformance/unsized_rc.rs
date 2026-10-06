// Rc and Arc pointing to trait objects, str and slices: unsizing through
// smart pointers, and values after a header aligned by their vtable.
use std::rc::Rc;
use std::sync::Arc;
trait Speak { fn speak(&self) -> String; }
struct Dog(u8);
impl Speak for Dog { fn speak(&self) -> String { format!("woof{}", self.0) } }
fn main() {
    let d: Rc<dyn Speak> = Rc::new(Dog(1));
    let e = Rc::clone(&d);
    println!("{} {} {}", d.speak(), e.speak(), Rc::strong_count(&d));
    let a: Arc<dyn Speak + Send + Sync> = Arc::new(Dog(2));
    println!("{}", a.speak());
    let v: Vec<Rc<dyn Speak>> = vec![Rc::new(Dog(3)), d];
    println!("{}", v.iter().map(|s| s.speak()).collect::<Vec<_>>().join(","));
    unsized_contents();
}
fn unsized_contents() {
    let s: Rc<str> = Rc::from("shared str");
    let t = Rc::clone(&s);
    println!("{} {} {} {}", s, t.len(), Rc::strong_count(&s), &s[2..5]);
    let owned: Rc<str> = Rc::from(String::from("owned"));
    println!("{:?} {}", owned, owned.to_uppercase());
    let nums: Rc<[i32]> = Rc::from(vec![1, 2, 3]);
    let refs: Rc<[String]> = Rc::from(&[String::from("a"), String::from("bc")][..]);
    println!("{:?} {} {:?} {}", nums, nums.iter().sum::<i32>(), refs, refs[1].len());
    let arr: Rc<[u8]> = Rc::new([1u8, 2, 3]);
    println!("{:?} {}", arr, arr.len());
    let words: Vec<Rc<str>> = "x y x".split(' ').map(Rc::from).collect();
    println!("{:?} {}", words, words[0] == words[2]);
}
