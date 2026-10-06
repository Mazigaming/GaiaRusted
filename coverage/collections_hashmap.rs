use std::collections::HashMap;
fn main() {
    let text = "the quick brown fox jumps over the lazy dog the end";
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for w in text.split_whitespace() { *counts.entry(w).or_insert(0) += 1; }
    let mut pairs: Vec<_> = counts.iter().collect();
    pairs.sort();
    println!("{:?}", pairs);
    println!("{:?} {:?}", counts.get("the"), counts.get("cat"));
    println!("{}", counts.contains_key("fox"));
    counts.remove("fox");
    println!("{} {}", counts.len(), counts.contains_key("fox"));
    let mut scores: HashMap<String, Vec<u32>> = HashMap::new();
    scores.entry("ann".to_string()).or_default().push(90);
    scores.entry("ann".to_string()).or_default().push(80);
    scores.entry("bob".to_string()).or_insert_with(Vec::new).push(70);
    let mut names: Vec<&String> = scores.keys().collect(); names.sort();
    println!("{:?}", names);
    let total: u32 = scores.values().flatten().sum();
    println!("{}", total);
    if let Some(v) = scores.get_mut("bob") { v.push(1); }
    println!("{:?}", scores["bob"]);
    let m: HashMap<i32, char> = vec![(1, 'a'), (2, 'b')].into_iter().collect();
    let mut keys: Vec<_> = m.keys().copied().collect(); keys.sort();
    println!("{:?}", keys);
    let mut ages = HashMap::new();
    ages.insert("x", 1);
    let old = ages.insert("x", 2);
    println!("{:?} {}", old, ages["x"]);
    ages.entry("x").and_modify(|a| *a += 10).or_insert(0);
    println!("{}", ages["x"]);
    for (k, v) in ages.iter_mut() { *v *= 2; println!("{} {}", k, v); }
}
