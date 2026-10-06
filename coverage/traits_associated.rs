trait Container { type Item; const KIND: &'static str; fn get(&self, i: usize) -> Option<&Self::Item>; fn first(&self) -> Option<&Self::Item> { self.get(0) } }
struct Stack { items: Vec<i32> }
impl Container for Stack { type Item = i32; const KIND: &'static str = "stack"; fn get(&self, i: usize) -> Option<&i32> { self.items.get(i) } }
trait Shape { const SIDES: u32; fn sides(&self) -> u32 { Self::SIDES } }
struct Tri; impl Shape for Tri { const SIDES: u32 = 3; }
struct Num; impl Num { const ZERO: i32 = 0; const ONE: i32 = Self::ZERO + 1; }
fn show<C: Container>(c: &C) -> String where C::Item: std::fmt::Debug { format!("{} {:?}", C::KIND, c.first()) }
fn main() {
    let s = Stack { items: vec![5, 6] };
    println!("{}", show(&s));
    println!("{} {}", Tri.sides(), Tri::SIDES);
    println!("{}", Num::ONE);
}
