// Recursion in the shapes the optimiser turns into loops, and in shapes
// it must leave alone.
fn fib(n: u64) -> u64 {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn factorial(n: u64) -> u64 {
    if n == 0 { 1 } else { n.wrapping_mul(factorial(n - 1)) }
}

fn product(n: u32) -> u32 {
    if n <= 1 { 1 } else { product(n - 1) * n }
}

fn checksum(n: u32) -> u32 {
    if n == 0 { 7 } else { n.wrapping_mul(2654435761) ^ checksum(n - 1) }
}

fn sum_digits(n: i64) -> i64 {
    if n < 10 && n > -10 { n } else { n % 10 + sum_digits(n / 10) }
}

fn collatz_steps(n: u64, steps: u32) -> u32 {
    if n == 1 {
        steps
    } else if n % 2 == 0 {
        collatz_steps(n / 2, steps + 1)
    } else {
        collatz_steps(3 * n + 1, steps + 1)
    }
}

// Subtraction is neither associative nor commutative.
fn alternating(n: i32) -> i32 {
    if n == 0 { 0 } else { n - alternating(n - 1) }
}

// Regrouping a float sum would change its rounding.
fn harmonic(n: u32) -> f64 {
    if n == 0 { 0.0 } else { 1.0 / n as f64 + harmonic(n - 1) }
}

fn count_down(n: u32, log: &mut Vec<u32>) {
    if n == 0 {
        return;
    }
    log.push(n);
    count_down(n - 1, log)
}

fn main() {
    println!("{} {} {}", fib(0), fib(1), fib(30));
    println!("{} {} {}", gcd(1071, 462), gcd(17, 5), gcd(0, 9));
    println!("{} {}", factorial(20), factorial(25));
    println!("{}", product(12));
    println!("{}", checksum(100000));
    println!("{} {}", sum_digits(987654321), sum_digits(-12345));
    println!("{}", collatz_steps(27, 0));
    println!("{} {}", alternating(10), alternating(7));
    println!("{}", harmonic(1000));
    let mut log = Vec::new();
    count_down(5, &mut log);
    println!("{:?}", log);
}
