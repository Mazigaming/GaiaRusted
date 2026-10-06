// Loops whose bodies hold computations that do not change from one
// iteration to the next, and some that only look that way.
fn nested(rows: u64, cols: u64) -> u64 {
    let mut total = 0;
    for i in 0..rows {
        for j in 0..cols {
            // `i * 7 + 3` is the same for every `j`, not for every `i`.
            let row_base = i * 7 + 3;
            let scaled = row_base * 2;
            total += scaled + j;
        }
    }
    total
}

fn maybe_never(n: u64, k: u64) -> u64 {
    let mut last = 0;
    let mut i = 0;
    while i < n {
        // Divides by a constant: fine to compute even if the loop never runs.
        last = k / 3 + i;
        i += 1;
    }
    last
}

fn conditional(values: &[i64], threshold: i64) -> i64 {
    let mut sum = 0;
    for &v in values.iter() {
        if v > threshold {
            let bonus = threshold * threshold + 1;
            sum += v + bonus;
        } else {
            sum -= v;
        }
    }
    sum
}

fn reassigned(n: u32) -> u32 {
    let mut x = 1;
    let mut acc = 0;
    for i in 0..n {
        acc += x;
        // Assigned in the loop and read before that on the next round:
        // not invariant, however constant its operands.
        x = 5 + 5;
        acc += i;
    }
    acc
}

fn matrix_trace(size: usize) -> usize {
    let mut cells = vec![0usize; size * size];
    for r in 0..size {
        for c in 0..size {
            cells[r * size + c] = r * 10 + c;
        }
    }
    let mut trace = 0;
    for d in 0..size {
        trace += cells[d * size + d];
    }
    trace
}

fn main() {
    println!("{}", nested(5, 4));
    println!("{} {}", maybe_never(0, 30), maybe_never(4, 30));
    println!("{}", conditional(&[1, 8, -3, 12, 5], 4));
    println!("{} {}", reassigned(0), reassigned(5));
    println!("{}", matrix_trace(7));
    let mut found = None;
    let limit = 50;
    for candidate in 2..limit {
        let square = candidate * candidate;
        if square > limit * 10 {
            found = Some(candidate);
            break;
        }
    }
    println!("{:?}", found);
}
