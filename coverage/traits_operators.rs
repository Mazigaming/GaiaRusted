use std::ops::{Add, Sub, Mul, Neg, AddAssign, Index, IndexMut};
#[derive(Debug, Clone, Copy, PartialEq)]
struct V2 { x: i32, y: i32 }
impl Add for V2 { type Output = V2; fn add(self, o: V2) -> V2 { V2 { x: self.x + o.x, y: self.y + o.y } } }
impl Sub for V2 { type Output = V2; fn sub(self, o: V2) -> V2 { V2 { x: self.x - o.x, y: self.y - o.y } } }
impl Mul<i32> for V2 { type Output = V2; fn mul(self, k: i32) -> V2 { V2 { x: self.x * k, y: self.y * k } } }
impl Neg for V2 { type Output = V2; fn neg(self) -> V2 { V2 { x: -self.x, y: -self.y } } }
impl AddAssign for V2 { fn add_assign(&mut self, o: V2) { self.x += o.x; self.y += o.y; } }
struct Grid { cells: Vec<i32>, w: usize }
impl Index<(usize, usize)> for Grid { type Output = i32; fn index(&self, (r, c): (usize, usize)) -> &i32 { &self.cells[r * self.w + c] } }
impl IndexMut<(usize, usize)> for Grid { fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut i32 { &mut self.cells[r * self.w + c] } }
#[derive(PartialEq, PartialOrd, Debug)]
struct Version(u32, u32);
fn main() {
    let a = V2 { x: 1, y: 2 }; let b = V2 { x: 10, y: 20 };
    println!("{:?} {:?} {:?} {:?}", a + b, b - a, a * 3, -a);
    let mut c = a; c += b; c += b;
    println!("{:?} {}", c, a == V2 { x: 1, y: 2 });
    let mut g = Grid { cells: vec![0; 6], w: 3 };
    g[(1, 2)] = 7; g[(0, 0)] += 1;
    println!("{} {} {:?}", g[(1, 2)], g[(0, 0)], g.cells);
    println!("{}", Version(1, 2) < Version(1, 10));
}
