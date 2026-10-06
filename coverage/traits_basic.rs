trait Animal {
    fn name(&self) -> String;
    fn sound(&self) -> String { String::from("...") }
    fn speak(&self) -> String { format!("{} says {}", self.name(), self.sound()) }
}
struct Dog; struct Fish { size: u32 }
impl Animal for Dog { fn name(&self) -> String { "Dog".into() } fn sound(&self) -> String { "woof".into() } }
impl Animal for Fish { fn name(&self) -> String { format!("Fish{}", self.size) } }
fn introduce<A: Animal>(a: &A) -> String { a.speak() }
fn loudest(a: &impl Animal) -> usize { a.sound().len() }
trait Counter { fn count() -> u32; }
impl Counter for Dog { fn count() -> u32 { 4 } }
fn main() {
    println!("{}", introduce(&Dog));
    println!("{}", introduce(&Fish { size: 3 }));
    println!("{}", loudest(&Dog));
    println!("{}", Dog::count());
    println!("{}", <Dog as Counter>::count());
}
