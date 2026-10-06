// Generic functions, structs, enums and impls.
#[derive(Debug, Clone, PartialEq)]
struct Pair<A, B> {
    first: A,
    second: B,
}

impl<A: Clone, B: Clone> Pair<A, B> {
    fn new(first: A, second: B) -> Self {
        Pair { first, second }
    }
    fn swap(&self) -> Pair<B, A> {
        Pair { first: self.second.clone(), second: self.first.clone() }
    }
}

#[derive(Debug)]
enum Tree<T> {
    Leaf,
    Node(Box<Tree<T>>, T, Box<Tree<T>>),
}

impl<T: PartialOrd + Copy> Tree<T> {
    fn insert(self, value: T) -> Tree<T> {
        match self {
            Tree::Leaf => Tree::Node(Box::new(Tree::Leaf), value, Box::new(Tree::Leaf)),
            Tree::Node(left, here, right) => {
                if value < here {
                    Tree::Node(Box::new(left.insert(value)), here, right)
                } else {
                    Tree::Node(left, here, Box::new(right.insert(value)))
                }
            }
        }
    }

    fn in_order(&self, out: &mut Vec<T>) {
        if let Tree::Node(left, here, right) = self {
            left.in_order(out);
            out.push(*here);
            right.in_order(out);
        }
    }

    fn depth(&self) -> usize {
        match self {
            Tree::Leaf => 0,
            Tree::Node(left, _, right) => 1 + left.depth().max(right.depth()),
        }
    }
}

struct Stack<T> {
    items: Vec<T>,
}

impl<T> Stack<T> {
    fn new() -> Self {
        Stack { items: Vec::new() }
    }
    fn push(&mut self, item: T) -> &mut Self {
        self.items.push(item);
        self
    }
    fn peek(&self) -> Option<&T> {
        self.items.last()
    }
    fn len(&self) -> usize {
        self.items.len()
    }
}

fn identity<T>(x: T) -> T {
    x
}

fn apply_twice<T, F: Fn(T) -> T>(f: F, x: T) -> T {
    f(f(x))
}

fn swap<T>(a: &mut T, b: &mut T) {
    std::mem::swap(a, b);
}

fn first_or<T: Clone>(items: &[T], default: T) -> T {
    match items.first() {
        Some(item) => item.clone(),
        None => default,
    }
}

fn main() {
    println!("{} {} {}", identity(1), identity("two"), identity(3.5));
    let p = Pair::new(1, "one");
    println!("{:?} {:?}", p, p.swap());
    println!("{}", p == Pair::new(1, "one"));

    let mut tree = Tree::Leaf;
    for v in [5, 2, 8, 1, 9, 3] {
        tree = tree.insert(v);
    }
    let mut sorted = Vec::new();
    tree.in_order(&mut sorted);
    println!("{:?} depth {}", sorted, tree.depth());

    let mut stack = Stack::new();
    stack.push("a").push("b").push("c");
    println!("{:?} {}", stack.peek(), stack.len());

    println!("{}", apply_twice(|x| x * 3, 2));
    println!("{}", apply_twice(|s: String| s + "!", String::from("hi")));
    let (mut a, mut b) = (1, 2);
    swap(&mut a, &mut b);
    println!("{} {}", a, b);
    println!("{} {}", first_or(&[7, 8], 0), first_or(&[], "none"));
    let nested: Option<Option<i32>> = Some(None);
    println!("{:?}", nested);
    let r: Result<Vec<i32>, String> = Ok(vec![1]);
    println!("{:?}", r);
}
