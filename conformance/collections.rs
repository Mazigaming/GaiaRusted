// The collections under a long run of pseudo-random operations, printed
// often enough that any divergence from the standard library shows.
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};

struct Lcg(u64);

impl Lcg {
    fn next(&mut self, bound: u64) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) % bound
    }
}

fn btree_map(rng: &mut Lcg) {
    let mut map = BTreeMap::new();
    let mut checksum = 0u64;
    for step in 0..20000u64 {
        let key = rng.next(3000);
        match rng.next(5) {
            0 | 1 => {
                if let Some(old) = map.insert(key, step) {
                    checksum = checksum.wrapping_add(old);
                }
            }
            2 => {
                if let Some(value) = map.remove(&key) {
                    checksum = checksum.wrapping_mul(31).wrapping_add(value);
                }
            }
            3 => *map.entry(key).or_insert(0) += 1,
            _ => {
                let low = key.saturating_sub(40);
                let total: u64 = map.range(low..=key).map(|(k, v)| k ^ v).sum();
                checksum = checksum.wrapping_add(total);
            }
        }
    }
    println!("btree map: {} entries, checksum {}", map.len(), checksum);
    println!("  first {:?} last {:?}", map.first_key_value(), map.last_key_value());
    let keys: Vec<u64> = map.keys().copied().collect();
    println!("  sorted {}", keys.windows(2).all(|pair| pair[0] < pair[1]));
    let backwards: Vec<u64> = map.keys().rev().copied().collect();
    println!("  reversed {}", backwards.iter().rev().eq(keys.iter()));
    let mut middle = map.range(1000..1100);
    println!("  middle {:?} {:?} {:?}", middle.next(), middle.next_back(), middle.count());
    while map.len() > 10 {
        map.pop_first();
        map.pop_last();
        if let Some(&key) = map.keys().nth(map.len() / 2) {
            map.remove(&key);
        }
    }
    println!("  drained to {:?}", map);
    let mut words: BTreeMap<String, usize> = BTreeMap::new();
    for word in "the cat and the hat and the bat".split(' ') {
        *words.entry(word.to_string()).or_default() += 1;
    }
    println!("  words {:?} {}", words, words["the"]);
    for (_, count) in words.iter_mut() {
        *count *= 10;
    }
    words.retain(|word, _| word.len() == 3 && word != "and");
    println!("  retained {:?}", words.into_iter().collect::<Vec<_>>());
}

fn btree_set(rng: &mut Lcg) {
    let a: BTreeSet<u64> = (0..300).map(|_| rng.next(500)).collect();
    let b: BTreeSet<u64> = (0..300).map(|_| rng.next(500)).collect();
    let union = a.union(&b).count();
    let both: Vec<u64> = a.intersection(&b).take(8).copied().collect();
    let only_a = a.difference(&b).count();
    let either = a.symmetric_difference(&b).count();
    println!("btree set: {} {} {} {} {} {:?}", a.len(), b.len(), union, only_a, either, both);
    let small: BTreeSet<u64> = a.iter().take(5).copied().collect();
    println!("  subset {} {} disjoint {}", small.is_subset(&a), a.is_superset(&small), small.is_disjoint(&b));
    println!("  range {:?}", a.range(100..130).collect::<Vec<_>>());
    println!("  ops {:?}", &(&small | &BTreeSet::from_iter([1000, 1001])) - &BTreeSet::from_iter([1001]));
}

fn hash_map(rng: &mut Lcg) {
    let mut map: HashMap<u64, u64> = HashMap::new();
    for step in 0..30000u64 {
        let key = rng.next(4000);
        match rng.next(4) {
            0 | 1 => {
                map.insert(key, step);
            }
            2 => {
                map.remove(&key);
            }
            _ => {
                map.entry(key).and_modify(|value| *value += 7).or_insert(1);
            }
        }
    }
    let mut entries: Vec<(u64, u64)> = map.iter().map(|(&k, &v)| (k, v)).collect();
    entries.sort();
    let checksum = entries.iter().fold(0u64, |acc, &(k, v)| acc.wrapping_mul(131).wrapping_add(k * 3 + v));
    println!("hash map: {} entries, checksum {}", map.len(), checksum);
    map.retain(|key, _| key % 3 == 0);
    for value in map.values_mut() {
        *value = 0;
    }
    println!("  retained {} all zero {}", map.len(), map.values().all(|&v| v == 0));
    let mut names: HashMap<String, Vec<u32>> = HashMap::new();
    names.entry("ann".to_string()).or_default().push(1);
    names.entry("ann".to_string()).or_insert_with(Vec::new).push(2);
    names.entry("bob".to_string()).or_insert_with_key(|key| vec![key.len() as u32]);
    let mut listed: Vec<_> = names.into_iter().collect();
    listed.sort();
    println!("  {:?}", listed);
    let set: HashSet<u64> = (0..50).map(|_| rng.next(60)).collect();
    let other: HashSet<u64> = (0..50).map(|_| rng.next(60)).collect();
    let mut common: Vec<_> = set.intersection(&other).collect();
    common.sort();
    println!("hash set: {} {} {:?}", set.len(), set.union(&other).count(), common);
}

fn deque(rng: &mut Lcg) {
    let mut deque: VecDeque<u64> = VecDeque::new();
    let mut popped = 0u64;
    for _ in 0..20000 {
        let value = rng.next(1000);
        match rng.next(6) {
            0 | 1 => deque.push_back(value),
            2 => deque.push_front(value),
            3 => popped = popped.wrapping_add(deque.pop_front().unwrap_or(0)),
            4 => popped = popped.wrapping_mul(7).wrapping_add(deque.pop_back().unwrap_or(1)),
            _ => {
                if !deque.is_empty() {
                    let index = rng.next(deque.len() as u64) as usize;
                    deque[index] += 1;
                }
            }
        }
    }
    let sum: u64 = deque.iter().sum();
    println!("deque: {} elements, sum {}, popped {}", deque.len(), sum, popped);
    deque.rotate_left(deque.len() / 3);
    deque.retain(|value| value % 4 != 0);
    let (front, back) = (deque.front().copied(), deque.back().copied());
    println!("  {:?} {:?} {:?}", front, back, deque.iter().rev().take(5).collect::<Vec<_>>());
    let mut small: VecDeque<i32> = (1..=6).collect();
    small.insert(2, 99);
    small.remove(4);
    small.make_contiguous().sort();
    let drained: Vec<i32> = small.drain(1..3).collect();
    println!("  {:?} {:?}", small, drained);
}

fn heap(rng: &mut Lcg) {
    let mut heap = BinaryHeap::new();
    for _ in 0..40 {
        heap.push(rng.next(100));
    }
    println!("heap: {:?}", heap);
    let mut taken = Vec::new();
    for _ in 0..10 {
        taken.push(heap.pop().unwrap());
    }
    println!("  {:?} {:?}", taken, heap);
    heap.extend((0..30).map(|_| rng.next(100)));
    println!("  {:?}", heap);
    let mut low = BinaryHeap::new();
    for value in [5, 1, 8, 3, 9, 2] {
        low.push(Reverse(value));
    }
    println!("  {:?} {:?}", low.pop(), low.peek());
    println!("  {:?}", BinaryHeap::from(vec![3, 1, 4, 1, 5, 9, 2, 6]).into_sorted_vec());
}

fn main() {
    let mut rng = Lcg(2024);
    btree_map(&mut rng);
    btree_set(&mut rng);
    hash_map(&mut rng);
    deque(&mut rng);
    heap(&mut rng);
}
