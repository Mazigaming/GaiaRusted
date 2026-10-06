use std::collections::{HashSet, BTreeSet};
fn main() {
    let a: HashSet<i32> = [1, 2, 3, 4].into_iter().collect();
    let b: HashSet<i32> = [3, 4, 5].iter().cloned().collect();
    let mut u: Vec<_> = a.union(&b).collect(); u.sort();
    let mut i: Vec<_> = a.intersection(&b).collect(); i.sort();
    let mut d: Vec<_> = a.difference(&b).collect(); d.sort();
    println!("{:?} {:?} {:?}", u, i, d);
    let mut s = HashSet::new();
    println!("{} {} {}", s.insert("x"), s.insert("x"), s.len());
    println!("{} {}", s.contains("x"), s.remove("y"));
    let bt: BTreeSet<char> = "mississippi".chars().collect();
    println!("{:?}", bt);
    println!("{:?} {:?}", bt.iter().next(), bt.iter().next_back());
    println!("{}", a.is_subset(&[1, 2, 3, 4, 5].into_iter().collect()));
    let words = ["b", "a", "b", "c", "a"];
    let unique: BTreeSet<&str> = words.iter().copied().collect();
    println!("{:?}", unique);
}
