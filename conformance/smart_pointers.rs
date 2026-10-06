// Rc, Weak, RefCell and Cell: shared ownership, and when shared values go.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Debug)]
struct Node {
    value: i32,
    children: RefCell<Vec<Rc<Node>>>,
    parent: RefCell<Weak<Node>>,
}

fn tree() {
    let leaf = Rc::new(Node { value: 3, children: RefCell::new(vec![]), parent: RefCell::new(Weak::new()) });
    println!("leaf parent = {:?}", leaf.parent.borrow().upgrade().map(|parent| parent.value));
    {
        let branch = Rc::new(Node { value: 5, children: RefCell::new(vec![Rc::clone(&leaf)]), parent: RefCell::new(Weak::new()) });
        *leaf.parent.borrow_mut() = Rc::downgrade(&branch);
        println!("leaf parent = {:?}", leaf.parent.borrow().upgrade().map(|parent| parent.value));
        println!("branch strong {} weak {}", Rc::strong_count(&branch), Rc::weak_count(&branch));
        println!("leaf strong {} weak {}", Rc::strong_count(&leaf), Rc::weak_count(&leaf));
    }
    println!("leaf parent = {:?}", leaf.parent.borrow().upgrade().map(|parent| parent.value));
    println!("leaf strong {}", Rc::strong_count(&leaf));
}

fn drops() {
    let first = Rc::new(Noisy("shared"));
    let second = Rc::clone(&first);
    let weak = Rc::downgrade(&first);
    drop(first);
    println!("one left: {}", Rc::strong_count(&second));
    drop(second);
    println!("upgrade after: {}", weak.upgrade().is_none());
    let unique = Rc::new(Noisy("unique"));
    match Rc::try_unwrap(unique) {
        Ok(inner) => println!("unwrapped {}", inner.0),
        Err(_) => println!("still shared"),
    }
    println!("end of drops");
}

fn cells() {
    let counter = Cell::new(0);
    for _ in 0..3 {
        counter.set(counter.get() + 1);
    }
    let old = counter.replace(10);
    println!("{} {} {:?}", old, counter.get(), counter);

    let log = Rc::new(RefCell::new(Vec::new()));
    let writer = Rc::clone(&log);
    writer.borrow_mut().push("a");
    log.borrow_mut().push("b");
    {
        let reading = log.borrow();
        println!("{:?} {} {}", *reading, log.try_borrow_mut().is_err(), log.try_borrow().is_ok());
    }
    println!("{:?} {}", log, log.try_borrow_mut().is_ok());

    let mut shared = Rc::new(String::from("cow"));
    let other = Rc::clone(&shared);
    Rc::make_mut(&mut shared).push_str("boy");
    println!("{} {} {}", shared, other, Rc::ptr_eq(&shared, &other));

    let cache: RefCell<HashMap<u32, u64>> = RefCell::new(HashMap::new());
    fn fib(n: u32, cache: &RefCell<HashMap<u32, u64>>) -> u64 {
        if n < 2 {
            return n as u64;
        }
        if let Some(&known) = cache.borrow().get(&n) {
            return known;
        }
        let value = fib(n - 1, cache) + fib(n - 2, cache);
        cache.borrow_mut().insert(n, value);
        value
    }
    println!("{} {}", fib(80, &cache), cache.borrow().len());
}

fn main() {
    tree();
    drops();
    cells();
}
