fn classify(n: i32) -> &'static str {
    if n < 0 { "negative" } else if n == 0 { "zero" } else { "positive" }
}
fn main() {
    println!("{} {} {}", classify(-2), classify(0), classify(5));
    let mut counter = 0;
    let found = loop { counter += 3; if counter > 10 { break counter * 2; } };
    println!("{}", found);
    let mut n = 27; let mut steps = 0;
    while n != 1 { n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 }; steps += 1; }
    println!("{}", steps);
    let mut total = 0;
    for i in 1..=10 { if i % 3 == 0 { continue; } total += i; }
    println!("{}", total);
    for i in (0..10).rev().step_by(3) { print!("{} ", i); }
    println!();
    'outer: for i in 0..5 {
        for j in 0..5 {
            if i * j == 6 { println!("found {} {}", i, j); break 'outer; }
            if j > i { continue 'outer; }
        }
    }
    let grade = |s: u32| match s { 90..=100 => 'A', 80..=89 => 'B', 0..=79 => 'C', _ => '?' };
    println!("{} {} {}", grade(95), grade(85), grade(10));
    let label = 'block: { if total > 10 { break 'block "big"; } "small" };
    println!("{}", label);
    let x = 5;
    let kind = match x { 1 | 2 => "low", 3..=5 => "mid", _ => "high" };
    println!("{}", kind);
}
