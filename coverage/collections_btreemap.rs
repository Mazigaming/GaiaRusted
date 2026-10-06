use std::collections::BTreeMap;
fn main() {
    let mut m = BTreeMap::new();
    for (k, v) in [(5, "five"), (1, "one"), (3, "three"), (9, "nine")] { m.insert(k, v); }
    println!("{:?}", m);
    for (k, v) in &m { print!("{}={} ", k, v); }
    println!();
    println!("{:?} {:?}", m.first_key_value(), m.last_key_value());
    let mid: Vec<_> = m.range(2..6).collect();
    println!("{:?}", mid);
    m.remove(&3);
    println!("{:?}", m.keys().collect::<Vec<_>>());
    let mut letters: BTreeMap<char, u32> = BTreeMap::new();
    for c in "hello world".chars().filter(|c| c.is_alphabetic()) { *letters.entry(c).or_insert(0) += 1; }
    println!("{:?}", letters);
    println!("{}", m.len());
}
