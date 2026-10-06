// A hash map's entries are dropped exactly once, whichever way they leave it:
// removed, replaced, retained away, drained, cloned, or never taken.
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

fn main() {
    let token = Rc::new(());
    let mut map: HashMap<u32, (Rc<()>, String)> = HashMap::new();
    for i in 0..2000 {
        map.insert(i, (Rc::clone(&token), format!("v{}", i)));
    }
    println!("{} {}", map.len(), Rc::strong_count(&token));
    for i in (0..2000).step_by(3) {
        map.remove(&i);
    }
    for i in (0..2000).step_by(6) {
        map.insert(i, (Rc::clone(&token), format!("again{}", i)));
    }
    println!("{} {}", map.len(), Rc::strong_count(&token));
    map.insert(5, (Rc::clone(&token), "replaced".to_string()));
    println!("{:?} {}", map.get(&5).map(|v| &v.1), Rc::strong_count(&token));
    map.retain(|k, _| k % 5 != 0);
    println!("{} {}", map.len(), Rc::strong_count(&token));
    let copy = map.clone();
    println!("{} {} {}", copy.len(), Rc::strong_count(&token), copy.get(&7).map(|v| v.1.as_str()).unwrap_or("-"));
    let mut taken = 0;
    for (k, v) in copy.into_iter().take(10) {
        taken += k as usize + v.1.len();
    }
    println!("{} {}", taken > 0, Rc::strong_count(&token));
    let drained: Vec<(u32, (Rc<()>, String))> = map.drain().collect();
    println!("{} {} {}", drained.len(), map.len(), Rc::strong_count(&token));
    drop(drained);
    println!("{}", Rc::strong_count(&token));
    for i in 0..100 {
        map.insert(i, (Rc::clone(&token), String::new()));
    }
    map.clear();
    println!("{} {} {}", map.len(), map.is_empty(), Rc::strong_count(&token));

    let mut set: HashSet<String> = HashSet::new();
    for word in "a b c a b d e f g h i j k l m n o p".split(' ') {
        set.insert(word.to_string());
    }
    set.remove("c");
    let mut items: Vec<&String> = set.iter().collect();
    items.sort();
    println!("{} {:?}", set.len(), &items[..4]);
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for word in "x y z x y x".split(' ') {
        *counts.entry(word).or_default() += 1;
    }
    let mut pairs: Vec<_> = counts.into_iter().collect();
    pairs.sort();
    println!("{:?}", pairs);
    let mut big: HashMap<u64, u64> = HashMap::with_capacity(10);
    for i in 0..100_000u64 { big.insert(i * 7919, i); }
    for i in 0..100_000u64 { if i % 2 == 0 { big.remove(&(i * 7919)); } }
    let sum: u64 = (0..100_000u64).filter_map(|i| big.get(&(i * 7919))).sum();
    println!("{} {} {}", big.len(), sum, big.capacity() >= big.len());
}
