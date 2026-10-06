// Iterator adapters, consumers and format specifications.
fn main() {
    let v = vec![1_i64, 2, 3, 4, 5];
    let sum: i64 = v.iter().map(|x| x * 2).fold(0, |acc, x| acc + x);
    println!("{}", sum);
    let evens: Vec<i64> = v.iter().filter(|x| **x % 2 == 0).cloned().collect();
    println!("{:?}", evens);
    let total: i64 = v.iter().sum();
    println!("{}", total);
    for (i, x) in v.iter().enumerate() {
        if i < 2 { println!("{} {}", i, x); }
    }
    let squares = (1..4).map(|n| n * n).collect::<Vec<i32>>();
    println!("{:?}", squares);
    let mut count = 0;
    for n in (0..10).rev() { if n % 3 == 0 { count += n; } }
    println!("{}", count);
    println!("{}", v.iter().any(|x| *x > 4));
    println!("{:?}", v.iter().max());
    let names = vec!["a", "b"];
    for (n, x) in names.iter().zip(v.iter()) { println!("{}={}", n, x); }
    let strs: Vec<String> = v.iter().map(|x| x.to_string()).collect();
    println!("{}", strs.len());
    println!("{:>5}|{:<5}|{:^5}|{:05}|{:.2}|{:x}", 42, "ab", "c", 7, 3.14159, 255);

    // The literal's type comes from what the result is collected into.
    let wide: Vec<u64> = (0..5).collect();
    println!("{:?} {}", wide, wide.iter().map(|x| x << 40).max().unwrap());
    let bytes: Vec<u8> = (250..=255).rev().collect();
    println!("{:?}", bytes);
    let halves: Vec<f64> = (1..4).map(|n| n as f64 / 2.0).collect();
    println!("{:?}", halves);
    let total: i64 = (1..=10).filter(|n| n % 2 == 1).map(|n| n * 1_000_000_000).sum();
    println!("{}", total);
}
