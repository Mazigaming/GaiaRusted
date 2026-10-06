// Sorting and searching owned strings.
fn main() {
    let mut seed: u64 = 99;
    let mut words: Vec<String> = Vec::new();
    for _ in 0..400_000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let len = 3 + (seed >> 62) as usize;
        let word: String = (0..len).map(|i| (b'a' + ((seed >> (i * 5)) % 26) as u8) as char).collect();
        words.push(word);
    }
    words.sort();
    words.dedup();
    let hits = ["abc", "zzz", "hello", "mmmm"].iter().filter(|w| words.binary_search(&w.to_string()).is_ok()).count();
    println!("{} unique, first {} last {} hits {}", words.len(), words[0], words[words.len() - 1], hits);
    let longest = words.iter().max_by_key(|w| w.len()).unwrap();
    println!("longest {}", longest.len());
}
