// Strings and hashing: count words in generated text, report the most common.
use std::collections::HashMap;
fn main() {
    let syllables = ["ka", "lo", "mi", "ra", "tu", "ne", "so", "vi"];
    let mut seed: u64 = 42;
    let mut text = String::new();
    for _ in 0..400_000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let parts = 1 + (seed >> 61) as usize;
        for p in 0..parts { text.push_str(syllables[((seed >> (p * 8)) & 7) as usize]); }
        text.push(' ');
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for word in text.split_whitespace() { *counts.entry(word).or_insert(0) += 1; }
    let mut ranked: Vec<(&str, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("distinct {}", ranked.len());
    for (word, n) in ranked.iter().take(5) { println!("{} {}", word, n); }
}
