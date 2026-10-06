// Hash table throughput with integer keys.
use std::collections::HashMap;
fn main() {
    let mut map: HashMap<u64, u64> = HashMap::new();
    let mut key: u64 = 1;
    for i in 0..1_000_000u64 {
        key = key.wrapping_mul(2654435761) % 4_000_037;
        *map.entry(key).or_insert(0) += i;
    }
    let mut hits = 0u64;
    let mut probe: u64 = 3;
    for _ in 0..1_000_000 {
        probe = probe.wrapping_mul(2654435761) % 4_000_037;
        if let Some(v) = map.get(&probe) { hits += v & 1; }
    }
    println!("{} entries, {} odd hits", map.len(), hits);
}
