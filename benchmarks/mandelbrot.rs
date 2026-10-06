// Floating point in tight loops: escape times over the Mandelbrot set.
fn escape(cr: f64, ci: f64, limit: u32) -> u32 {
    let (mut zr, mut zi) = (0.0f64, 0.0f64);
    for i in 0..limit {
        let (zr2, zi2) = (zr * zr, zi * zi);
        if zr2 + zi2 > 4.0 { return i; }
        zi = 2.0 * zr * zi + ci;
        zr = zr2 - zi2 + cr;
    }
    limit
}
fn main() {
    let (width, height, limit) = (1200, 800, 400);
    let mut histogram = vec![0u64; 8];
    let mut total: u64 = 0;
    for y in 0..height {
        for x in 0..width {
            let cr = -2.2 + 3.2 * x as f64 / width as f64;
            let ci = -1.2 + 2.4 * y as f64 / height as f64;
            let n = escape(cr, ci, limit);
            total += n as u64;
            histogram[(n as usize * 8 / (limit as usize + 1)).min(7)] += 1;
        }
    }
    println!("total {} histogram {:?}", total, histogram);
}
