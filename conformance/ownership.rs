// References, mutation through them, moves, and data structures with Option<Box<..>>.
#[derive(Debug)]
struct Node {
    value: i32,
    next: Option<Box<Node>>,
}

struct List {
    head: Option<Box<Node>>,
    len: usize,
}

impl List {
    fn new() -> List {
        List { head: None, len: 0 }
    }

    fn push_front(&mut self, value: i32) {
        let old = self.head.take();
        self.head = Some(Box::new(Node { value, next: old }));
        self.len += 1;
    }

    fn pop_front(&mut self) -> Option<i32> {
        let node = self.head.take()?;
        self.head = node.next;
        self.len -= 1;
        Some(node.value)
    }

    fn sum(&self) -> i32 {
        let mut total = 0;
        let mut current = &self.head;
        while let Some(node) = current {
            total += node.value;
            current = &node.next;
        }
        total
    }

    fn increment_all(&mut self) {
        let mut current = &mut self.head;
        while let Some(node) = current {
            node.value += 1;
            current = &mut node.next;
        }
    }

    fn reverse(&mut self) {
        let mut previous = None;
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
            node.next = previous;
            previous = Some(node);
        }
        self.head = previous;
    }

    fn to_vec(&self) -> Vec<i32> {
        let mut out = Vec::new();
        let mut current = self.head.as_ref();
        while let Some(node) = current {
            out.push(node.value);
            current = node.next.as_ref();
        }
        out
    }
}

fn bump(counter: &mut i32) {
    *counter += 1;
}

fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

fn first_word(text: &str) -> &str {
    match text.find(" ") {
        Some(end) => &text[..end],
        None => text,
    }
}

fn append_all(target: &mut Vec<String>, items: &[&str]) {
    for item in items {
        target.push(item.to_string());
    }
}

fn main() {
    let mut list = List::new();
    for v in [3, 2, 1] {
        list.push_front(v);
    }
    println!("{:?} {}", list.to_vec(), list.sum());
    list.increment_all();
    list.reverse();
    println!("{:?} len {}", list.to_vec(), list.len);
    println!("{:?} {:?}", list.pop_front(), list.pop_front());
    println!("{:?}", list.head);

    let mut n = 0;
    bump(&mut n);
    bump(&mut n);
    let r = &mut n;
    *r *= 10;
    println!("{}", n);

    let a = String::from("longer one");
    let chosen;
    {
        let b = String::from("short");
        chosen = longest(&a, &b).to_string();
    }
    println!("{}", chosen);
    println!("{}", first_word("hello world"));

    let mut names = Vec::new();
    append_all(&mut names, &["x", "y"]);
    let moved = names;
    println!("{:?}", moved);

    let mut grid = vec![vec![0; 3]; 2];
    let row = &mut grid[1];
    row[2] = 9;
    println!("{:?}", grid);

    let pair = (String::from("left"), 5);
    let (text, number) = pair;
    println!("{} {}", text, number);
    let boxed = Box::new((1, 2));
    let (x, y) = *boxed;
    println!("{}", x + y);
    let mut opt = Some(String::from("inside"));
    if let Some(s) = &mut opt {
        s.push('!');
    }
    println!("{:?}", opt);
    let refs: Vec<&i32> = vec![&1, &2];
    let total: i32 = refs.iter().map(|r| **r).sum();
    println!("{}", total);
}
