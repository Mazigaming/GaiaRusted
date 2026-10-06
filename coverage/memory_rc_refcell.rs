use std::rc::{Rc, Weak};
use std::cell::RefCell;
#[derive(Debug)]
struct Node { val: i32, children: RefCell<Vec<Rc<Node>>>, parent: RefCell<Weak<Node>> }
fn main() {
    let a = Rc::new(5);
    let b = Rc::clone(&a);
    println!("{} {}", Rc::strong_count(&a), *b + 1);
    { let _c = a.clone(); println!("{}", Rc::strong_count(&a)); }
    println!("{}", Rc::strong_count(&a));
    let shared = Rc::new(RefCell::new(vec![1, 2]));
    let other = Rc::clone(&shared);
    other.borrow_mut().push(3);
    shared.borrow_mut().push(4);
    println!("{:?} {}", shared.borrow(), other.borrow().len());
    let leaf = Rc::new(Node { val: 3, children: RefCell::new(vec![]), parent: RefCell::new(Weak::new()) });
    let branch = Rc::new(Node { val: 5, children: RefCell::new(vec![Rc::clone(&leaf)]), parent: RefCell::new(Weak::new()) });
    *leaf.parent.borrow_mut() = Rc::downgrade(&branch);
    println!("{:?}", leaf.parent.borrow().upgrade().map(|p| p.val));
    println!("{} {}", Rc::strong_count(&branch), Rc::weak_count(&branch));
    println!("{}", branch.children.borrow()[0].val);
    let cell = RefCell::new(String::from("x"));
    cell.borrow_mut().push_str("yz");
    println!("{}", cell.borrow());
    println!("{}", cell.try_borrow_mut().is_ok());
}
