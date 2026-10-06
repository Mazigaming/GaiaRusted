// Integer and float methods: checked, wrapping and saturating arithmetic, bit
// counting, euclidean division, casts that saturate, parsing, and libm.
fn main() {
    let a: i32 = i32::MAX;
    println!("{:?} {:?} {:?}", a.checked_add(1), a.checked_sub(1), 5i32.checked_div(0));
    println!("{} {} {}", a.wrapping_add(1), a.saturating_add(5), (-5i32).saturating_sub(i32::MAX));
    println!("{:?} {:?}", a.overflowing_add(1), 200u8.overflowing_mul(2));
    println!("{} {} {} {}", 2i64.pow(40), 3u32.pow(4), (-2i32).pow(3), 10u64.checked_pow(30).is_none());
    println!("{} {} {} {}", 40u32.leading_zeros(), 40u32.trailing_zeros(), 255u16.count_ones(), 0u8.count_zeros());
    println!("{} {} {}", 0x1234u16.swap_bytes(), 1u32.rotate_right(1), (-17i32).rem_euclid(5));
    println!("{} {} {} {}", (-17i32).div_euclid(5), -17 / 5, -17 % 5, 17i32.signum());
    println!("{} {} {}", (-7i32).abs(), i32::MIN.unsigned_abs(), 7u32.abs_diff(10));
    println!("{} {} {}", u64::MAX.to_string().len(), i16::MIN, u8::MAX as i8);
    println!("{} {} {}", 300i32 as u8, -1i64 as u32, 3.99f64 as i32);
    println!("{} {} {}", -3.99f64 as u8, f64::NAN as i32, 1e20f64 as i32);
    println!("{} {} {}", 255u8 as char, 'A' as u8 + 2, (b'a' + 1) as char);
    let x = 2.0f64;
    println!("{} {} {} {}", x.sqrt(), x.powi(10), x.powf(0.5), x.exp().ln());
    println!("{:.6} {:.6} {:.6} {:.6}", x.sin(), x.cos(), x.tan(), x.atan2(1.0));
    println!("{} {} {} {}", 2.5f64.round(), -2.5f64.round(), 2.7f64.floor(), -2.2f64.ceil());
    println!("{} {} {} {}", 2.7f64.trunc(), 2.75f64.fract(), (-1.5f64).abs(), 3.0f64.mul_add(2.0, 1.0));
    println!("{} {} {}", 1.0f64.max(f64::NAN), 2.0f64.min(1.0), 5.5f64.clamp(0.0, 5.0));
    println!("{} {} {}", f64::NAN.is_nan(), (1.0f64 / 0.0).is_infinite(), 0.1f64.to_bits());
    println!("{} {} {}", f64::from_bits(4611686018427387904), f32::EPSILON, f64::MAX);
    println!("{} {} {}", 10f64.log10(), 8f64.log2(), 100f64.log(10.0));
    println!("{} {} {}", (0.1f32 + 0.2f32), 1.0f32 / 3.0, 16f32.sqrt());
    println!("{:?} {:?} {:?}", "42".parse::<i32>(), "-7".parse::<i8>(), "300".parse::<u8>().is_err());
    println!("{:?} {:?} {:?}", "3.5".parse::<f64>(), "1e3".parse::<f64>(), "abc".parse::<f32>().is_err());
    println!("{:?} {:?}", i64::from_str_radix("-ff", 16), u8::from_str_radix("101", 2));
    println!("{} {} {}", 7 / 2, 7.0 / 2.0, 7 % -3);
    println!("{} {}", u32::MAX.wrapping_mul(3), (i8::MIN).wrapping_neg());
    println!("{} {} {}", 1u64 << 40, -16i32 >> 2, 0xF0u8 >> 4);
    println!("{} {}", i32::from(true), u16::try_from(70000u32).is_err());
    println!("{:?} {:?}", u8::try_from(-1i32), i8::try_from(100u64));
    println!("{} {}", (1.0f64).to_degrees(), 180f64.to_radians());
    println!("{} {} {}", 9.0f64.cbrt(), 3.0f64.hypot(4.0), 1e-3f64.exp_m1());
    println!("{} {}", isize::BITS, usize::MAX.count_ones());
    println!("{} {}", 12u32.next_power_of_two(), 16u32.is_power_of_two());
    println!("{} {}", 255u8.checked_next_power_of_two().is_none(), 10i32.isqrt());
}
