// An enum whose other variants hold no data fits them into values its
// data can never take: `Option<&T>` is one pointer, as under rustc.
use std::cmp::Ordering;
use std::mem::size_of;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    Dot,
    Line(&'static str),
    Empty,
}

#[derive(Debug)]
enum Tree {
    Leaf,
    Node(Box<Tree>, i32, Box<Tree>),
}

fn depth(tree: &Tree) -> usize {
    match tree {
        Tree::Leaf => 0,
        Tree::Node(left, _, right) => 1 + depth(left).max(depth(right)),
    }
}

static NOTHING: Option<&'static str> = None;
static GREETING: Option<&'static str> = Some("hi");

fn main() {
    println!("{} {} {}", size_of::<Option<Box<i32>>>(), size_of::<Option<&u8>>(), size_of::<Option<&[u8]>>());
    println!("{} {} {}", size_of::<Option<bool>>(), size_of::<Option<Option<bool>>>(), size_of::<Option<char>>());
    println!("{} {} {}", size_of::<Option<Ordering>>(), size_of::<Option<NonZeroU32>>(), size_of::<Option<Rc<i32>>>());
    println!("{} {} {}", size_of::<Option<Arc<String>>>(), size_of::<Option<fn()>>(), size_of::<Shape>());
    println!("{} {} {}", size_of::<Option<(u32, &i32)>>(), size_of::<Result<&i32, ()>>(), size_of::<Option<Box<dyn Fn()>>>());

    let values: Vec<Option<Box<i32>>> = vec![Some(Box::new(1)), None, Some(Box::new(3))];
    let total: i32 = values.iter().flatten().map(|boxed| **boxed).sum();
    println!("{} {:?}", total, values);

    let flags = [Some(true), None, Some(false)];
    let nested = [Some(Some(true)), Some(None), None];
    println!("{:?} {:?} {}", flags, nested, flags.iter().filter(|flag| flag.is_some()).count());
    println!("{:?}", [Some(Ordering::Less), None, Some(Ordering::Greater)]);
    for shape in [Shape::Dot, Shape::Line("ab"), Shape::Empty] {
        match shape {
            Shape::Dot => print!("dot "),
            Shape::Line(text) => print!("line:{} ", text),
            Shape::Empty => print!("empty "),
        }
    }
    println!();

    let mut tree = Tree::Leaf;
    for value in 0..5 {
        tree = Tree::Node(Box::new(tree), value, Box::new(Tree::Leaf));
    }
    println!("{}", depth(&tree));
    println!("{:?} {:?} {:?}", NOTHING, GREETING, NonZeroU32::new(0));

    let mut slot: Option<Rc<String>> = None;
    for word in ["a", "b"] {
        slot = Some(Rc::new(word.to_string()));
    }
    println!("{:?} {:?}", slot, slot.as_ref().map(Rc::strong_count));
    let chars: Vec<Option<char>> = "a\u{e9}".chars().map(Some).chain([None]).collect();
    let ok: Result<&i32, ()> = Ok(&5);
    let err: Result<&i32, ()> = Err(());
    println!("{:?} {:?} {:?}", chars, ok, err);
}
