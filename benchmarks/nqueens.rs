// Backtracking search: count the solutions of the n-queens problem.
fn solve(row: usize, n: usize, cols: &mut Vec<bool>, d1: &mut Vec<bool>, d2: &mut Vec<bool>) -> u64 {
    if row == n { return 1; }
    let mut count = 0;
    for c in 0..n {
        let (a, b) = (row + c, row + n - c - 1);
        if cols[c] || d1[a] || d2[b] { continue; }
        cols[c] = true; d1[a] = true; d2[b] = true;
        count += solve(row + 1, n, cols, d1, d2);
        cols[c] = false; d1[a] = false; d2[b] = false;
    }
    count
}
fn main() {
    for n in [8, 10, 12, 13] {
        let mut cols = vec![false; n];
        let mut d1 = vec![false; 2 * n];
        let mut d2 = vec![false; 2 * n];
        println!("{} queens: {} solutions", n, solve(0, n, &mut cols, &mut d1, &mut d2));
    }
}
