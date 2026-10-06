// Allocation, hashing and sorting: Vec, String and HashMap under load.
use std::collections::HashMap;

fn main() {
    let mut counts: HashMap<u64, u64> = HashMap::new();
    let mut state: u64 = 12345;
    for _ in 0..2_000_000 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let key = (state >> 33) % 50_000;
        match counts.get_mut(&key) {
            Some(count) => *count += 1,
            None => {
                counts.insert(key, 1);
            }
        }
    }
    let mut values: Vec<u64> = counts.values().map(|v| *v).collect();
    values.sort();
    println!("{} {} {}", counts.len(), values[0], values[values.len() - 1]);

    let mut words: Vec<String> = Vec::new();
    for i in 0..200_000 {
        words.push(format!("word{}", (i * 7919) % 100_003));
    }
    words.sort();
    let total: usize = words.iter().map(|w| w.len()).sum();
    println!("{} {} {}", words.len(), total, words[100_000]);
}
