// The methods of the number types, at the edges of each type.
use std::convert::TryFrom;

fn main() {
    // Checked, wrapping, saturating and overflowing arithmetic.
    println!("{:?} {:?} {:?}", 250u8.checked_add(5), 250u8.checked_add(6), (-128i8).checked_sub(1));
    println!("{:?} {:?} {:?}", i32::MAX.checked_mul(2), (-5i32).checked_mul(-5), i64::MIN.checked_mul(-1));
    println!("{:?} {:?} {:?}", 10i32.checked_div(0), i32::MIN.checked_div(-1), 7u32.checked_rem(3));
    println!("{} {} {}", 250u8.wrapping_add(10), 5u8.wrapping_sub(10), 200u8.wrapping_mul(3));
    println!("{} {} {}", 250u8.saturating_add(10), 5u8.saturating_sub(10), i16::MIN.saturating_sub(1));
    println!("{} {}", i32::MAX.saturating_mul(2), (-3i32).saturating_mul(i32::MAX));
    println!("{:?} {:?}", 250u8.overflowing_add(10), i32::MAX.overflowing_add(1));
    println!("{:?} {:?} {}", 3i32.checked_pow(20), 2u64.checked_pow(63), 3i32.saturating_pow(30));
    println!("{} {} {}", 2i32.pow(10), 7u64.pow(0), (-2i64).pow(63));

    // Bits.
    println!("{} {} {}", 40u32.leading_zeros(), 40u32.trailing_zeros(), 0u32.leading_zeros());
    println!("{} {} {}", 255u8.count_ones(), (-1i64).count_ones(), 0u16.count_zeros());
    println!("{} {} {}", 1u8.rotate_left(3), 0x81u8.rotate_right(1), (-2i32).rotate_left(1));
    println!("{:#x} {:#b}", 0x12345678u32.swap_bytes(), 1u8.reverse_bits());
    println!("{} {} {}", 1000u32.ilog2(), 99999u32.ilog10(), (-16i32).leading_ones());
    println!("{} {} {}", 12u64.is_power_of_two(), 64u64.is_power_of_two(), 100u32.next_power_of_two());

    // Signs and Euclidean division.
    println!("{} {} {} {}", (-5i32).abs(), (-5i32).signum(), 0i64.signum(), i8::MIN.unsigned_abs());
    println!("{} {}", 3i32.abs_diff(-7), 10u8.abs_diff(250));
    println!("{} {} {} {}", 10i32.rem_euclid(3), (-10i32).rem_euclid(3), (-10i32).rem_euclid(-3), 10i32.div_euclid(-3));
    println!("{} {}", (-7i32).div_euclid(2), (-7.5f64).rem_euclid(2.0));
    println!("{} {} {}", 15i32.clamp(0, 10), (-3i32).min(2), 9u8.max(4));

    // Limits and parsing.
    println!("{} {} {} {}", i8::MIN, i8::MAX, u16::MAX, i64::MIN);
    println!("{} {}", u64::MAX, usize::BITS);
    println!("{:?} {:?} {:?}", i32::from_str_radix("ff", 16), u8::from_str_radix("101", 2), i64::from_str_radix("-zz", 36));
    for text in ["42", "-42", "+7", "", "-", "4x2", "300", "-129"] {
        match text.parse::<i8>() {
            Ok(value) => println!("{:?} -> {}", text, value),
            Err(error) => println!("{:?} -> {} / {:?}", text, error, error),
        }
    }
    println!("{:?} {:?}", "-1".parse::<u32>(), "18446744073709551615".parse::<u64>());
    println!("{:?} {:?} {:?}", "2.5".parse::<f64>(), "1e3".parse::<f32>(), "x".parse::<f64>().is_err());

    // Conversions.
    println!("{} {} {}", u32::from(200u8), i64::from(-5i32), f64::from(3.5f32));
    println!("{} {} {}", u32::from('A'), char::from(98u8), i32::from(true));
    println!("{:?} {:?}", u8::try_from(300i32), u8::try_from(30i32));
    println!("{:?} {:?} {:?}", i8::try_from(200u8), u32::try_from(-1i64), i64::try_from(u64::MAX));
    let small: Result<u16, _> = 70000u32.try_into();
    let fits: Result<u16, _> = 7000u32.try_into();
    println!("{:?} {:?}", small.is_err(), fits);
    match u8::try_from(-1i32) {
        Ok(value) => println!("{}", value),
        Err(error) => println!("{}", error),
    }

    // Floats.
    let x = 2.0f64;
    println!("{} {} {} {}", x.sqrt(), x.powi(10), x.powf(0.5), x.cbrt());
    println!("{:.6} {:.6} {:.6}", 1.0f64.sin(), 1.0f64.cos(), 1.0f64.tan());
    println!("{:.6} {:.6} {:.6}", 0.5f64.asin(), 0.5f64.acos(), 1.0f64.atan2(2.0));
    println!("{:.6} {:.6} {:.6} {:.6}", 1.0f64.exp(), 10.0f64.ln(), 8.0f64.log2(), 1000.0f64.log10());
    println!("{} {} {} {}", 2.7f64.floor(), 2.3f64.ceil(), 2.5f64.round(), (-2.5f64).round());
    println!("{} {} {}", (-2.7f64).trunc(), 2.75f64.fract(), (-0.0f64).abs());
    println!("{} {} {}", 3.0f64.hypot(4.0), 2.0f64.mul_add(3.0, 1.0), 4.0f64.recip());
    println!("{:.4} {:.4}", 180.0f64.to_radians(), std::f64::consts::FRAC_PI_2.to_degrees());
    println!("{:.5} {:.5} {:.5}", std::f64::consts::PI, std::f64::consts::E, std::f64::consts::SQRT_2);
    println!("{} {} {}", f64::NAN.is_nan(), f64::INFINITY.is_infinite(), (1.0f64 / 0.0).is_finite());
    println!("{} {} {}", f64::NEG_INFINITY, f64::MAX > 1e308, f64::MIN_POSITIVE > 0.0);
    println!("{} {}", f64::EPSILON, f32::EPSILON);
    println!("{} {} {}", 1.5f64.min(f64::NAN), f64::NAN.max(2.0), 5.0f64.clamp(0.0, 1.0));
    println!("{} {} {}", (-3.0f64).signum(), 0.0f64.is_sign_negative(), (-0.0f64).is_sign_negative());
    println!("{} {}", 1.5f32.sin() > 0.99, 9.0f32.sqrt());
    println!("{:.3} {:.3}", 2.0f32.powf(1.5), 100.0f32.ln());
    println!("{}", (1u64 << 53) as f64 + 1.0);
}
