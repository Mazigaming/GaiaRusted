// Nested loops over flat vectors: dense matrix multiplication.
fn multiply(a: &[f64], b: &[f64], n: usize) -> Vec<f64> {
    let mut c = vec![0.0; n * n];
    for i in 0..n {
        for k in 0..n {
            let aik = a[i * n + k];
            for j in 0..n { c[i * n + j] += aik * b[k * n + j]; }
        }
    }
    c
}
fn main() {
    let n = 400;
    let a: Vec<f64> = (0..n * n).map(|i| ((i * 7) % 13) as f64 - 6.0).collect();
    let b: Vec<f64> = (0..n * n).map(|i| ((i * 5) % 11) as f64 * 0.5).collect();
    let mut c = multiply(&a, &b, n);
    for _ in 0..3 { c = multiply(&c, &b, n); for x in c.iter_mut() { *x *= 0.001; } }
    let trace: f64 = (0..n).map(|i| c[i * n + i]).sum();
    println!("{:.6}", trace);
}
