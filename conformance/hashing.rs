// Hashing as rustc does it: DefaultHasher is SipHash-1-3 with zero keys, and
// every type feeds it the same bytes, so the printed hashes are rustc's.
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasher, Hash, Hasher};

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
struct Point { x: i32, y: i32 }

#[derive(Hash)]
enum Shape { Dot, Circle(u8), Named { name: String, sides: u16 } }

fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

struct Counting { bytes: usize, calls: usize }
impl Hasher for Counting {
    fn finish(&self) -> u64 { (self.bytes * 1000 + self.calls) as u64 }
    fn write(&mut self, bytes: &[u8]) { self.bytes += bytes.len(); self.calls += 1; }
}

fn main() {
    println!("{} {} {}", hash_of(&0u64), hash_of(&42i32), hash_of(&u128::MAX));
    println!("{} {} {}", hash_of("hello"), hash_of(&String::from("hello world, longer than 8")), hash_of(""));
    println!("{} {} {}", hash_of(&true), hash_of(&'x'), hash_of(&(1u8, 2u16, 3u32)));
    println!("{} {}", hash_of(&vec![1i32, 2, 3]), hash_of(&[1u64, 2, 3][..]));
    println!("{} {}", hash_of(&Some(5i64)), hash_of(&None::<i64>));
    println!("{} {}", hash_of(&Point { x: 1, y: -2 }), hash_of(&vec!["a", "bc"]));
    println!("{} {} {}", hash_of(&Shape::Dot), hash_of(&Shape::Circle(3)), hash_of(&Shape::Named { name: "tri".into(), sides: 3 }));
    println!("{} {}", hash_of(&std::cmp::Ordering::Less), hash_of(&Ok::<u8, ()>(1)));
    println!("{} {}", std::cmp::Ordering::Less as i32, std::cmp::Ordering::Greater as i8);
    let mut h = DefaultHasher::new();
    h.write(b"abc"); h.write_u8(1); h.write(b"defghijklmnop"); h.write_usize(9);
    println!("{}", h.finish());
    let mut counting = Counting { bytes: 0, calls: 0 };
    "abc".hash(&mut counting);
    vec![1u32, 2, 3].hash(&mut counting);
    (1u8, 'c').hash(&mut counting);
    println!("{}", counting.finish());
    let state = std::hash::BuildHasherDefault::<DefaultHasher>::default();
    println!("{}", state.hash_one(77u32));
    let mut map: HashMap<Point, u32> = HashMap::new();
    for i in 0..1000 { *map.entry(Point { x: i % 37, y: i % 11 }).or_insert(0) += 1; }
    let mut counts: Vec<u32> = map.values().copied().collect();
    counts.sort();
    println!("{} {:?}", map.len(), &counts[..5]);
    let words: HashSet<&str> = "the cat saw the other cat".split(' ').collect();
    let mut sorted: Vec<_> = words.into_iter().collect();
    sorted.sort();
    println!("{:?}", sorted);
}
