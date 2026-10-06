// Conversions between floats and integers with `as`: rounding towards zero,
// saturating at each type's limits, NaN as zero, and u64 above 2^63.
fn main() {
    let values = [0.0f64, -0.0, 0.5, -0.5, 1.9, -1.9, 127.5, 128.0, -128.7, -129.0, 255.9, 256.0, 300.0, -5.0, 1e10, -1e10, 2147483647.9, 2147483648.0, -2147483649.0, 4294967295.5, 4294967296.0, 9.2e18, 9.3e18, -9.3e18, 1.8e19, 1.9e19, f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
    for &v in values.iter() {
        println!("{} {} {} {} {} {} {} {} {}", v as i8, v as u8, v as i16, v as u16, v as i32, v as u32, v as i64, v as u64, v as usize);
    }
    let f = [0.5f32, -3.7, 300.0, 1e20, f32::NAN];
    for &v in f.iter() { println!("{} {} {}", v as u8, v as i32, v as u64); }
    for &u in [0u64, 1, 1 << 52, (1 << 53) + 1, 1 << 63, (1 << 63) + 1025, u64::MAX].iter() {
        println!("{} {}", u as f64, u as f32);
    }
}
