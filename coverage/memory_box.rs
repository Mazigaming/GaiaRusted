#[derive(Debug)]
enum List { Cons(i32, Box<List>), Nil }
use List::{Cons, Nil};
fn sum(l: &List) -> i32 { match l { Cons(v, rest) => v + sum(rest), Nil => 0 } }
#[derive(Debug)]
struct Tree { val: i32, left: Option<Box<Tree>>, right: Option<Box<Tree>> }
impl Tree {
    fn leaf(v: i32) -> Self { Tree { val: v, left: None, right: None } }
    fn insert(&mut self, v: i32) {
        let slot = if v < self.val { &mut self.left } else { &mut self.right };
        match slot { Some(n) => n.insert(v), None => *slot = Some(Box::new(Tree::leaf(v))) }
    }
    fn inorder(&self, out: &mut Vec<i32>) {
        if let Some(l) = &self.left { l.inorder(out); }
        out.push(self.val);
        if let Some(r) = &self.right { r.inorder(out); }
    }
    fn depth(&self) -> usize { 1 + self.left.as_ref().map_or(0, |l| l.depth()).max(self.right.as_ref().map_or(0, |r| r.depth())) }
}
fn main() {
    let l = Cons(1, Box::new(Cons(2, Box::new(Cons(3, Box::new(Nil))))));
    println!("{} {:?}", sum(&l), l);
    let mut t = Tree::leaf(50);
    for v in [30, 70, 20, 40, 60, 80, 35] { t.insert(v); }
    let mut out = Vec::new(); t.inorder(&mut out);
    println!("{:?} {}", out, t.depth());
    let b = Box::new(41);
    println!("{}", *b + 1);
    let mut bx = Box::new([0u8; 4]);
    bx[2] = 9;
    println!("{:?}", bx);
}
