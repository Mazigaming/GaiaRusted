// Abstractions that should cost nothing: iterator chains and closures.
fn main() {
    let data: Vec<u64> = (0..5_000_000).collect();
    let mut checksum = 0u64;
    for round in 0..10u64 {
        let sum: u64 = data
            .iter()
            .map(|x| x * 3 + round)
            .filter(|x| x % 7 != 0)
            .map(|x| x ^ (x >> 3))
            .sum();
        checksum = checksum.wrapping_add(sum);
    }
    println!("{}", checksum);
    let evens = data.iter().filter(|x| *x % 2 == 0).count();
    let big = data.iter().rev().take(10).fold(0, |acc, x| acc + x);
    println!("{} {}", evens, big);
}
