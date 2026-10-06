use std::cell::Cell;
use std::mem;
struct Counter { hits: Cell<u32> }
impl Counter { fn hit(&self) { self.hits.set(self.hits.get() + 1); } }
fn main() {
    let c = Counter { hits: Cell::new(0) };
    c.hit(); c.hit();
    println!("{}", c.hits.get());
    let mut a = vec![1]; let mut b = vec![2, 3];
    mem::swap(&mut a, &mut b);
    println!("{:?} {:?}", a, b);
    let old = mem::replace(&mut a, vec![9]);
    println!("{:?} {:?}", old, a);
    let taken = mem::take(&mut b);
    println!("{:?} {:?}", taken, b);
    println!("{} {} {}", mem::size_of::<u64>(), mem::size_of::<(u8, u32)>(), mem::size_of::<Option<Box<i32>>>());
    let x = Cell::new(5);
    let y = x.replace(6);
    println!("{} {}", y, x.get());
}
