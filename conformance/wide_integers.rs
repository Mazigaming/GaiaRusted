// 128-bit integers: arithmetic, shifts, comparisons, conversions and formatting.
fn fib(n: u32) -> u128 {
    let (mut a, mut b) = (0u128, 1u128);
    for _ in 0..n {
        let next = a + b;
        a = b;
        b = next;
    }
    a
}

fn mul_mod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

fn main() {
    println!("{} {}", fib(150), fib(180));
    println!("{} {}", u128::MAX, i128::MIN);
    println!("{} {}", i128::MAX, -170141183460469231731687303715884105727i128 - 1);
    let x: i128 = -12345678901234567890123456789;
    let y: i128 = 98765432109876543210;
    println!("{} {} {} {}", x + y, x - y, x * 3, y * 1000);
    println!("{} {} {} {}", x / 7, x % 7, y / -13, y % -13);
    println!("{} {} {}", x < y, x > y, x == x);
    let big: u128 = 1 << 100;
    println!("{} {} {} {}", big, big >> 37, big << 20, (big - 1).count_ones());
    println!("{:x} {:#b} {:o}", big + 255, 5u128, u128::MAX);
    println!("{}", mul_mod(u64::MAX - 1, u64::MAX - 2, 1_000_000_007));
    let shifted = -1i128 >> 100;
    let v: Vec<u128> = (1..=5u128).map(|k| k * k * 1_000_000_000_000_000_000_000).collect();
    println!("{} {:?}", shifted, v);
    println!("{} {} {}", (x as f64), (1e30 as u128), (-2.5e20f64 as i128));
    println!("{:?} {:?}", "340282366920938463463374607431768211455".parse::<u128>(), "-5".parse::<i128>());
    println!("{:?} {:?}", 300u128.checked_mul(u128::MAX / 2), (u128::MAX).checked_add(1));
    let bytes = 0x0102030405060708090a0b0c0d0e0f10u128.to_be_bytes();
    println!("{:?} {}", bytes, u128::from_le_bytes(bytes) == 0x100f0e0d0c0b0a090807060504030201);
    println!("{} {}", u64::MAX as i128 * -1, (u64::MAX as u128).pow(2));
    let total: u128 = (1..=30u128).product();
    println!("{} {}", total, (1..=30u128).sum::<u128>());
}
