// The iterator adapters and collections in everyday use: zip, chain, flat_map,
// scan, peekable, windows, partition, unzip, BTreeMap ranges, BinaryHeap, ...
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque, BinaryHeap};
use std::cmp::Reverse;
fn main() {
    let v: Vec<i32> = (1..=10).collect();
    let evens: Vec<_> = v.iter().filter(|&&x| x % 2 == 0).map(|x| x * x).collect();
    println!("{:?}", evens);
    let z: Vec<(i32, char)> = v.iter().copied().zip("abc".chars()).collect();
    println!("{:?}", z);
    let chained: Vec<i32> = v.iter().take(2).chain(v.iter().skip(8)).cloned().collect();
    println!("{:?}", chained);
    let flat: Vec<i32> = vec![vec![1, 2], vec![], vec![3]].into_iter().flatten().collect();
    let fm: Vec<char> = vec!["ab", "cd"].iter().flat_map(|s| s.chars()).collect();
    println!("{:?} {:?}", flat, fm);
    let scanned: Vec<i32> = v.iter().scan(0, |acc, &x| { *acc += x; Some(*acc) }).collect();
    println!("{:?}", scanned);
    let tw: Vec<_> = v.iter().take_while(|&&x| x < 4).collect();
    let sw: Vec<_> = v.iter().skip_while(|&&x| x < 8).collect();
    println!("{:?} {:?}", tw, sw);
    let stepped: Vec<_> = (0..20).step_by(7).collect();
    println!("{:?} {:?}", stepped, (0..5).rev().collect::<Vec<_>>());
    println!("{:?}", v.windows(3).map(|w| w.iter().sum::<i32>()).collect::<Vec<_>>());
    println!("{:?}", v.chunks(4).map(|c| c.len()).collect::<Vec<_>>());
    println!("{:?}", v.chunks_exact(3).map(|c| c[0]).collect::<Vec<_>>());
    let mut it = v.iter().peekable();
    let mut groups = Vec::new();
    while let Some(&x) = it.next() {
        if let Some(&&next) = it.peek() { if next == x + 1 { groups.push((x, next)); it.next(); } }
    }
    println!("{:?}", groups);
    println!("{} {} {:?} {:?}", v.iter().any(|&x| x > 9), v.iter().all(|&x| x > 0), v.iter().position(|&x| x == 5), v.iter().rposition(|&x| x < 3));
    let words = ["apple", "fig", "banana", "kiwi"];
    println!("{:?} {:?}", words.iter().min_by_key(|w| w.len()), words.iter().max_by(|a, b| a.len().cmp(&b.len())));
    println!("{:?} {:?}", words.iter().max_by_key(|w| w.len()), words.iter().min());
    let total: f64 = [1.5, 2.25, 3.0].iter().sum();
    let prod: i64 = (1..=15).product();
    println!("{} {} {}", total, prod, v.iter().fold(String::new(), |acc, x| acc + &x.to_string()));
    let (small, big): (Vec<i32>, Vec<i32>) = v.iter().partition(|&&x| x < 5);
    println!("{:?} {:?}", small, big);
    let (a, b): (Vec<i32>, Vec<char>) = vec![(1, 'a'), (2, 'b')].into_iter().unzip();
    println!("{:?} {:?}", a, b);
    println!("{:?} {:?} {:?}", v.iter().last(), v.iter().nth(3), v.iter().count());
    println!("{:?}", v.iter().enumerate().filter(|(i, _)| i % 3 == 0).map(|(_, x)| x).collect::<Vec<_>>());
    println!("{:?}", v.iter().map(|x| x * 2).filter_map(|x| if x > 10 { Some(x / 2) } else { None }).collect::<Vec<_>>());
    println!("{:?}", "a,b,,c".split(',').map(String::from).collect::<Vec<String>>());
    let set: HashSet<i32> = v.iter().map(|x| x % 4).collect();
    let mut sv: Vec<_> = set.into_iter().collect(); sv.sort();
    println!("{:?}", sv);
    let bt: BTreeMap<&str, usize> = words.iter().map(|w| (*w, w.len())).collect();
    println!("{:?} {:?}", bt, bt.range("b".."g").collect::<Vec<_>>());
    let mut dq: VecDeque<i32> = (1..=5).collect();
    dq.rotate_left(2); dq.push_front(0);
    println!("{:?} {:?} {:?}", dq, dq.front(), dq.back());
    let mut heap: BinaryHeap<Reverse<i32>> = vec![5, 1, 8, 3].into_iter().map(Reverse).collect();
    let mut order = Vec::new();
    while let Some(Reverse(x)) = heap.pop() { order.push(x); }
    println!("{:?}", order);
    let counts = "mississippi".chars().fold(HashMap::new(), |mut m, c| { *m.entry(c).or_insert(0) += 1; m });
    let mut cv: Vec<_> = counts.into_iter().collect(); cv.sort();
    println!("{:?}", cv);
    let mut d = vec![1, 1, 2, 3, 3, 3, 4];
    d.dedup();
    let mut r = vec![1, 2, 3, 4, 5, 6];
    r.retain(|x| x % 2 == 0);
    let drained: Vec<i32> = r.drain(1..).collect();
    println!("{:?} {:?} {:?}", d, r, drained);
    println!("{:?}", (1..4).map(|i| (0..i).map(|j| j * i).collect::<Vec<_>>()).collect::<Vec<_>>());
    println!("{:?}", v.iter().copied().reduce(|a, b| a.max(b)));
    println!("{:?}", v.iter().rev().skip(1).step_by(3).collect::<Vec<_>>());
    println!("{}", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("-"));
    println!("{:?}", std::iter::repeat("ab").take(3).collect::<String>());
    println!("{:?}", std::iter::successors(Some(1u32), |x| if *x < 100 { Some(x * 3) } else { None }).collect::<Vec<_>>());
    println!("{:?}", std::iter::once(5).chain(std::iter::empty()).collect::<Vec<i32>>());
    let mut counter = 0;
    let from_fn: Vec<i32> = std::iter::from_fn(|| { counter += 1; if counter < 4 { Some(counter) } else { None } }).collect();
    println!("{:?}", from_fn);
    println!("{:?}", v.iter().sum::<i32>().cmp(&55));
    println!("{:?}", [3, 1, 2].iter().is_sorted());
    println!("{:?}", v.split(|x| x % 4 == 0).collect::<Vec<_>>());
    println!("{:?}", v.iter().max().map(|m| m * 2).unwrap_or_default());
}
