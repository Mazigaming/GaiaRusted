fn main() {
    let a: i8 = -100; let b: u8 = 200; let c: i16 = -30000; let d: u16 = 60000;
    let e: i32 = -2_000_000_000; let f: u32 = 4_000_000_000; let g: i64 = -9_000_000_000_000; let h: u64 = 18_000_000_000_000_000_000;
    println!("{} {} {} {} {} {} {} {}", a, b, c, d, e, f, g, h);
    println!("{} {} {}", 17 / 5, 17 % 5, -17 % 5);
    println!("{} {}", 7i32.pow(3), 2u64.pow(40));
    println!("{} {} {}", (-5i32).abs(), (-5i32).signum(), 0i32.signum());
    println!("{} {}", 3.min(9), 3.max(9));
    println!("{:?} {:?}", 250u8.checked_add(10), 250u8.checked_add(5));
    println!("{} {}", 250u8.wrapping_add(10), 250u8.saturating_add(10));
    println!("{:?}", 250u8.overflowing_add(10));
    println!("{} {}", 5u8.saturating_sub(10), i32::MAX.wrapping_add(1));
    println!("{:?} {:?}", 10i32.checked_div(0), 10i32.checked_rem(3));
    println!("{} {} {}", 40u32.leading_zeros(), 40u32.trailing_zeros(), 255u32.count_ones());
    println!("{} {}", 1u8.rotate_left(3), 0x12345678u32.swap_bytes());
    println!("{} {}", i64::MIN, u32::MAX);
    println!("{:?} {:?}", i32::from_str_radix("ff", 16), u8::from_str_radix("101", 2));
    println!("{} {}", 255u8 as i8, -1i32 as u64);
    println!("{} {}", 10i32.rem_euclid(3), (-10i32).rem_euclid(3));
    println!("{} {}", (-7i32).div_euclid(2), 7i32.div_euclid(-2));
    println!("{}", 12u64.is_power_of_two());
    println!("{}", 100u32.next_power_of_two());
    let big: u128 = 1 << 100; let neg: i128 = -(1i128 << 90);
    println!("{} {}", big, neg);
}
