fn longest<'a>(a: &'a str, b: &'a str) -> &'a str { if a.len() >= b.len() { a } else { b } }
struct Excerpt<'a> { part: &'a str }
impl<'a> Excerpt<'a> { fn level(&self) -> usize { self.part.len() } fn first_word(&self) -> &'a str { self.part.split(' ').next().unwrap() } }
fn first<'a, T>(v: &'a [T]) -> Option<&'a T> { v.first() }
fn main() {
    let s1 = String::from("long string");
    let r;
    { let s2 = String::from("xyz"); r = longest(s1.as_str(), s2.as_str()).to_string(); }
    println!("{}", r);
    let text = String::from("Call me Ishmael. Some years ago");
    let e = Excerpt { part: text.split('.').next().unwrap() };
    println!("{} {}", e.level(), e.first_word());
    println!("{:?}", first(&[7, 8]));
    let s: &'static str = "static";
    println!("{}", s);
}
