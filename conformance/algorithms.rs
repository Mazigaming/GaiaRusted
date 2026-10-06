// Classic algorithms: sorting, searching, primes, matrices, recursion.
fn quicksort(items: &mut [i32]) {
    if items.len() <= 1 {
        return;
    }
    let pivot = items[items.len() / 2];
    let (mut i, mut j) = (0, items.len() - 1);
    loop {
        while items[i] < pivot {
            i += 1;
        }
        while items[j] > pivot {
            j -= 1;
        }
        if i >= j {
            break;
        }
        items.swap(i, j);
        i += 1;
        j -= 1;
    }
    let (left, right) = items.split_at_mut(i);
    quicksort(left);
    quicksort(right);
}

fn binary_search(items: &[i32], target: i32) -> Option<usize> {
    let (mut lo, mut hi) = (0, items.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if items[mid] == target {
            return Some(mid);
        } else if items[mid] < target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    None
}

fn sieve(limit: usize) -> Vec<usize> {
    let mut is_prime = vec![true; limit + 1];
    is_prime[0] = false;
    if limit >= 1 {
        is_prime[1] = false;
    }
    let mut n = 2;
    while n * n <= limit {
        if is_prime[n] {
            let mut multiple = n * n;
            while multiple <= limit {
                is_prime[multiple] = false;
                multiple += n;
            }
        }
        n += 1;
    }
    (0..=limit).filter(|&n| is_prime[n]).collect()
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn multiply(a: &[[i64; 2]; 2], b: &[[i64; 2]; 2]) -> [[i64; 2]; 2] {
    let mut out = [[0; 2]; 2];
    for i in 0..2 {
        for j in 0..2 {
            for k in 0..2 {
                out[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    out
}

fn fib_matrix(n: u32) -> i64 {
    let mut result = [[1, 0], [0, 1]];
    let mut base = [[1, 1], [1, 0]];
    let mut power = n;
    while power > 0 {
        if power % 2 == 1 {
            result = multiply(&result, &base);
        }
        base = multiply(&base, &base);
        power /= 2;
    }
    result[0][1]
}

fn reverse_words(text: &str) -> String {
    let mut words: Vec<&str> = text.split(" ").collect();
    words.reverse();
    words.join(" ")
}

fn is_palindrome(text: &str) -> bool {
    let chars: Vec<char> = text.chars().filter(|c| c.is_alphanumeric()).collect();
    let n = chars.len();
    (0..n / 2).all(|i| chars[i] == chars[n - 1 - i])
}

fn hanoi(n: u32, from: char, to: char, via: char, moves: &mut Vec<(char, char)>) {
    if n == 0 {
        return;
    }
    hanoi(n - 1, from, via, to, moves);
    moves.push((from, to));
    hanoi(n - 1, via, to, from, moves);
}

fn main() {
    let mut data = vec![38, 27, 43, 3, 9, 82, 10, -5, 27, 0];
    quicksort(&mut data);
    println!("{:?}", data);
    println!("{:?} {:?}", binary_search(&data, 43), binary_search(&data, 4));
    println!("{:?}", sieve(50));
    println!("{}", gcd(1071, 462));
    println!("{} {}", fib_matrix(10), fib_matrix(40));
    println!("{}", reverse_words("the quick brown fox"));
    println!("{} {}", is_palindrome("racecar"), is_palindrome("rust"));
    let mut moves = Vec::new();
    hanoi(4, 'A', 'C', 'B', &mut moves);
    println!("{} {:?}", moves.len(), moves[0]);
    let mut total: u64 = 0;
    for i in 1..=100_000u64 {
        total = total.wrapping_add(i * i % 7);
    }
    println!("{}", total);
}
