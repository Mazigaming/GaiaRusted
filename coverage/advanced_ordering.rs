use std::cmp::Ordering;
#[derive(Debug, Eq, PartialEq)]
struct Task { pri: u8, name: String }
impl Ord for Task { fn cmp(&self, o: &Self) -> Ordering { o.pri.cmp(&self.pri).then_with(|| self.name.cmp(&o.name)) } }
impl PartialOrd for Task { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }
fn main() {
    println!("{:?} {:?} {:?}", 1.cmp(&2), "b".cmp("a"), 3.cmp(&3));
    match 5.cmp(&3) { Ordering::Less => println!("less"), Ordering::Equal => println!("eq"), Ordering::Greater => println!("greater") }
    let mut tasks = vec![Task { pri: 1, name: "low".into() }, Task { pri: 9, name: "b".into() }, Task { pri: 9, name: "a".into() }];
    tasks.sort();
    println!("{:?}", tasks.iter().map(|t| t.name.as_str()).collect::<Vec<_>>());
    println!("{:?}", 2.5f64.partial_cmp(&1.0));
    println!("{:?}", Ordering::Less.reverse());
    println!("{}", std::cmp::max(3, 8) + std::cmp::min(3, 8));
    let mut people = vec![("bob", 30), ("al", 25), ("cy", 30)];
    people.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("{:?}", people);
    println!("{:?}", [3, 1, 2].iter().max_by(|a, b| a.cmp(b)));
    println!("{}", (1, "z") < (2, "a"));
    println!("{:?}", "apple".cmp("apricot"));
}
