// Rc and Weak trees, RefCell and Cell, drop order, lifetimes in structs,
// mem::swap/replace/take, Rc<str>, Rc::make_mut, boxed slices.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
#[derive(Debug)]
struct Node { val: i32, children: RefCell<Vec<Rc<Node>>>, parent: RefCell<Weak<Node>> }
struct Logger { lines: RefCell<Vec<String>>, count: Cell<u32> }
impl Logger { fn log(&self, s: &str) { self.lines.borrow_mut().push(s.to_string()); self.count.set(self.count.get() + 1); } }
struct Droppy(&'static str);
impl Drop for Droppy { fn drop(&mut self) { println!("drop {}", self.0); } }
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str { if a.len() >= b.len() { a } else { b } }
struct Parser<'s> { text: &'s str, pos: usize }
impl<'s> Parser<'s> { fn next_word(&mut self) -> Option<&'s str> { let rest = &self.text[self.pos..]; let rest = rest.trim_start(); if rest.is_empty() { return None; } let start = self.text.len() - rest.len(); let end = rest.find(' ').map(|i| start + i).unwrap_or(self.text.len()); self.pos = end; Some(&self.text[start..end]) } }
fn main() {
    let leaf = Rc::new(Node { val: 3, children: RefCell::new(vec![]), parent: RefCell::new(Weak::new()) });
    let branch = Rc::new(Node { val: 5, children: RefCell::new(vec![Rc::clone(&leaf)]), parent: RefCell::new(Weak::new()) });
    *leaf.parent.borrow_mut() = Rc::downgrade(&branch);
    println!("{:?} {} {}", leaf.parent.borrow().upgrade().map(|p| p.val), Rc::strong_count(&leaf), Rc::weak_count(&branch));
    println!("{}", branch.children.borrow().iter().map(|c| c.val).sum::<i32>());
    let log = Logger { lines: RefCell::new(vec![]), count: Cell::new(0) };
    log.log("a"); log.log("b");
    println!("{:?} {}", log.lines.borrow(), log.count.get());
    let shared = Rc::new(RefCell::new(HashMap::new()));
    let s2 = Rc::clone(&shared);
    s2.borrow_mut().insert("k", 1);
    shared.borrow_mut().entry("k").and_modify(|v| *v += 10);
    println!("{:?} {}", shared.borrow().get("k"), Rc::ptr_eq(&shared, &s2));
    let r = RefCell::new(5);
    { let b1 = r.borrow(); let b2 = r.borrow(); println!("{} {}", *b1 + *b2, r.try_borrow_mut().is_err()); }
    *r.borrow_mut() += 1;
    println!("{}", r.into_inner());
    {
        let _a = Droppy("a");
        let b = Droppy("b");
        let v = vec![Droppy("v1"), Droppy("v2")];
        std::mem::drop(b);
        let boxed = Box::new(Droppy("boxed"));
        let moved = v;
        println!("end of scope {}", moved.len() + boxed.0.len());
    }
    println!("{}", longest("abc", "de"));
    let mut p = Parser { text: "  alpha beta  gamma", pos: 0 };
    while let Some(w) = p.next_word() { print!("[{}]", w); }
    println!();
    let mut a = vec![1, 2]; let mut b = vec![3];
    std::mem::swap(&mut a, &mut b);
    let old = std::mem::replace(&mut a, vec![9]);
    let taken = std::mem::take(&mut b);
    println!("{:?} {:?} {:?} {:?}", a, b, old, taken);
    let cell = Cell::new(1); let old = cell.replace(2); println!("{} {}", old, cell.take());
    let rc_s: Rc<str> = Rc::from("shared str");
    let cloned = Rc::clone(&rc_s);
    println!("{} {}", cloned, Rc::strong_count(&rc_s));
    let mut rc_v = Rc::new(vec![1]);
    Rc::make_mut(&mut rc_v).push(2);
    println!("{:?} {:?}", rc_v, Rc::try_unwrap(rc_v.clone()).is_err());
    let b: Box<[i32]> = vec![1, 2, 3].into_boxed_slice();
    println!("{:?} {}", b, b.len());
}
