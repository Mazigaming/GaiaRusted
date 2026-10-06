fn main() {
    let mut v: Vec<i32> = Vec::new();
    v.push(3); v.push(1); v.push(4); v.push(1); v.push(5);
    let popped = v.pop();
    println!("{:?} {} {:?}", v, v.len(), popped);
    v.insert(1, 9); println!("{:?}", v);
    println!("{} {:?}", v.remove(0), v);
    v.extend(vec![2, 6, 5]); println!("{:?}", v);
    v.retain(|&x| x != 5); println!("{:?}", v);
    v.sort(); v.dedup(); println!("{:?}", v);
    v.sort_by_key(|&x| std::cmp::Reverse(x)); println!("{:?}", v);
    v.truncate(3); println!("{:?}", v);
    let drained: Vec<i32> = v.drain(..1).collect(); println!("{:?} {:?}", drained, v);
    let mut w = vec![1, 2, 3, 4, 5];
    let tail = w.split_off(3); println!("{:?} {:?}", w, tail);
    w.swap(0, 2); println!("{:?}", w);
    for x in w.iter_mut() { *x += 100; } println!("{:?}", w);
    println!("{:?} {:?}", w.iter().max(), w.iter().min());
    let mut words = vec!["pear", "apple", "fig"]; words.sort_unstable(); println!("{:?}", words);
    let caps: Vec<usize> = Vec::with_capacity(10); println!("{} {}", caps.len(), caps.capacity() >= 10);
    let filled = vec![vec![0; 2]; 3]; println!("{:?}", filled);
    println!("{}", vec![1, 2] < vec![1, 3]);
    let mut stack = vec![1, 2, 3];
    while let Some(top) = stack.pop() { print!("{} ", top); }
    println!();
    let mut ints = vec![5, 2, 8];
    ints.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("{:?} {}", ints, ints.contains(&8));
    let joined = ints.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
    println!("{}", joined);
    let mut dup = vec![1, 1, 2, 3, 3, 3];
    dup.dedup_by_key(|x| *x / 2);
    println!("{:?}", dup);
    println!("{:?}", [3, 1, 2].iter().copied().rev().collect::<Vec<i32>>());
    let mut fl = vec![2.5, -1.0, 3.75];
    fl.sort_by(|a, b| b.partial_cmp(a).unwrap());
    println!("{:?}", fl);
    v.clear(); println!("{}", v.is_empty());
}
