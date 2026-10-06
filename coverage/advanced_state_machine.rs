#[derive(Debug, Clone, Copy, PartialEq)]
enum State { Idle, Running { ticks: u32 }, Done(u32) }
#[derive(Debug, Clone, Copy)]
enum Event { Start, Tick, Stop }
fn step(s: State, e: Event) -> State {
    match (s, e) {
        (State::Idle, Event::Start) => State::Running { ticks: 0 },
        (State::Running { ticks }, Event::Tick) => State::Running { ticks: ticks + 1 },
        (State::Running { ticks }, Event::Stop) => State::Done(ticks),
        (s, _) => s,
    }
}
trait Visitor { fn visit(&mut self, n: &Node); }
enum Node { Num(i64), Add(Box<Node>, Box<Node>), Mul(Box<Node>, Box<Node>) }
struct Eval { stack: Vec<i64> }
impl Visitor for Eval {
    fn visit(&mut self, n: &Node) {
        match n {
            Node::Num(v) => self.stack.push(*v),
            Node::Add(a, b) | Node::Mul(a, b) => {
                self.visit(a); self.visit(b);
                let (y, x) = (self.stack.pop().unwrap(), self.stack.pop().unwrap());
                self.stack.push(if matches!(n, Node::Add(..)) { x + y } else { x * y });
            }
        }
    }
}
fn main() {
    let mut s = State::Idle;
    for e in [Event::Tick, Event::Start, Event::Tick, Event::Tick, Event::Stop, Event::Tick] { s = step(s, e); println!("{:?}", s); }
    let tree = Node::Add(Box::new(Node::Num(2)), Box::new(Node::Mul(Box::new(Node::Num(3)), Box::new(Node::Num(4)))));
    let mut e = Eval { stack: vec![] }; e.visit(&tree);
    println!("{:?}", e.stack);
}
