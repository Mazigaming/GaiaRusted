// Tight loops over a Vec: the sieve of Eratosthenes, repeated.
fn count_primes(limit: usize) -> usize {
    let mut composite = vec![false; limit + 1];
    let mut count = 0;
    let mut n = 2;
    while n <= limit {
        if !composite[n] {
            count += 1;
            let mut multiple = n * n;
            while multiple <= limit {
                composite[multiple] = true;
                multiple += n;
            }
        }
        n += 1;
    }
    count
}

fn main() {
    let mut total = 0;
    for _ in 0..20 {
        total += count_primes(2_000_000);
    }
    println!("{}", total);
}
