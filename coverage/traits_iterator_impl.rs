struct Fib { a: u64, b: u64 }
impl Iterator for Fib { type Item = u64; fn next(&mut self) -> Option<u64> { let r = self.a; self.a = self.b; self.b += r; Some(r) } }
struct Countdown(u32);
impl Iterator for Countdown { type Item = u32; fn next(&mut self) -> Option<u32> { if self.0 == 0 { None } else { self.0 -= 1; Some(self.0 + 1) } } }
struct Bag { items: Vec<String> }
impl<'a> IntoIterator for &'a Bag { type Item = &'a String; type IntoIter = std::slice::Iter<'a, String>; fn into_iter(self) -> Self::IntoIter { self.items.iter() } }
fn main() {
    let f: Vec<u64> = Fib { a: 0, b: 1 }.take(10).collect();
    println!("{:?}", f);
    println!("{}", Fib { a: 0, b: 1 }.skip_while(|&x| x < 100).next().unwrap());
    println!("{:?}", Countdown(5).collect::<Vec<_>>());
    println!("{}", Countdown(4).map(|x| x * x).sum::<u32>());
    for (i, x) in Countdown(3).enumerate() { print!("{}:{} ", i, x); }
    println!();
    let bag = Bag { items: vec!["x".into(), "y".into()] };
    for s in &bag { print!("{} ", s); }
    println!();
    let mut it = Countdown(2);
    println!("{:?} {:?} {:?}", it.next(), it.next(), it.next());
}
