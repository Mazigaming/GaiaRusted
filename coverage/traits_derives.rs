use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
struct Card { rank: u8, suit: u8 }
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Level { Low, Mid(u8), High { v: i32 } }
#[derive(Debug, Default)]
struct Config { name: String, retries: u32, verbose: bool, ratio: f64, tags: Vec<String> }
fn main() {
    let mut cards = vec![Card { rank: 3, suit: 1 }, Card { rank: 1, suit: 2 }, Card { rank: 3, suit: 0 }];
    cards.sort();
    println!("{:?}", cards);
    let c = cards[0]; let d = c;
    println!("{} {}", c == d, c < cards[1]);
    let mut set = HashSet::new();
    for c in &cards { set.insert(*c); }
    set.insert(Card { rank: 1, suit: 2 });
    println!("{}", set.len());
    let mut counts = HashMap::new();
    *counts.entry(Level::Mid(2)).or_insert(0) += 1;
    *counts.entry(Level::Mid(2)).or_insert(0) += 1;
    println!("{:?}", counts.get(&Level::Mid(2)));
    let mut levels = vec![Level::High { v: -1 }, Level::Low, Level::Mid(5), Level::Mid(1)];
    levels.sort();
    println!("{:?}", levels);
    println!("{:?}", Config::default());
    println!("{:?}", Card::default());
    println!("{:?}", Level::Mid(3).clone());
    println!("{:?}", Card { rank: 2, suit: 2 }.cmp(&Card { rank: 2, suit: 1 }));
}
