// Dynamic programming over a 2-D table: longest common subsequence.
fn lcs(a: &[u8], b: &[u8]) -> usize {
    let mut table = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            table[i][j] = if a[i - 1] == b[j - 1] { table[i - 1][j - 1] + 1 } else { table[i - 1][j].max(table[i][j - 1]) };
        }
    }
    table[a.len()][b.len()] as usize
}
fn generate(n: usize, mut seed: u64) -> Vec<u8> {
    (0..n).map(|_| { seed = seed.wrapping_mul(2862933555777941757).wrapping_add(3037000493); b'a' + (seed >> 60) as u8 % 4 }).collect()
}
fn main() {
    let a = generate(6000, 1);
    let b = generate(6000, 2);
    println!("{}", lcs(&a, &b));
}
