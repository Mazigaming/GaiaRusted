// Loops, labels, break values, early returns and nested conditions.
fn collatz(mut n: u64) -> u32 {
    let mut steps = 0;
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        steps += 1;
    }
    steps
}

fn first_multiple(of: i32, above: i32) -> i32 {
    let mut candidate = above;
    loop {
        candidate += 1;
        if candidate % of == 0 {
            break candidate;
        }
    }
}

fn classify(n: i32) -> &'static str {
    if n < 0 {
        return "negative";
    }
    if n == 0 {
        "zero"
    } else if n < 10 {
        "small"
    } else {
        "large"
    }
}

fn main() {
    println!("{}", collatz(27));
    println!("{}", first_multiple(7, 100));
    for n in [-5, 0, 3, 42] {
        println!("{}", classify(n));
    }

    let mut found = (0, 0);
    'outer: for i in 1..10 {
        for j in 1..10 {
            if i * j == 42 {
                found = (i, j);
                break 'outer;
            }
        }
    }
    println!("{} {}", found.0, found.1);

    let mut evens = 0;
    for i in 0..=20 {
        if i % 2 != 0 {
            continue;
        }
        evens += i;
    }
    println!("{}", evens);

    let mut countdown = 3;
    while countdown > 0 {
        print!("{} ", countdown);
        countdown -= 1;
    }
    println!("liftoff");

    let value = {
        let a = 3;
        let b = 4;
        a * a + b * b
    };
    println!("{}", value);
    let mut i = 0u8;
    for n in 250..=255u8 { i = n; }
    println!("{}", i);
}
