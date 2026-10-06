// Division and remainder by constants, which the optimiser turns into
// shifts, masks and multiplications, checked at the edges of each type.
fn mix(d: u64, n: u64) -> u64 {
    (n / d).wrapping_mul(31).wrapping_add(n % d)
}

fn main() {
    let numbers = [
        0u64, 1, 2, 3, 5, 6, 7, 8, 9, 10, 99, 100, 101, 255, 256, 1000, 65535, 65536,
        4294967295, 4294967296, 4294967297, 1 << 63, (1 << 63) - 1, (1 << 63) + 1,
        u64::MAX, u64::MAX - 1, 12345678901234567, 9876543210987654321,
    ];
    let mut digest: u64 = 0;
    for &n in numbers.iter() {
        let results = [
            mix(3, n), mix(5, n), mix(6, n), mix(7, n), mix(10, n), mix(16, n), mix(641, n),
            mix(1000000007, n), mix(4294967295, n), mix(4294967297, n), mix((1 << 63) + 1, n),
            mix(u64::MAX, n), mix(u64::MAX - 2, n),
        ];
        for r in results.iter() {
            digest = digest.wrapping_mul(1000003) ^ r;
        }
        let narrow = n as u32;
        digest = digest.wrapping_mul(1000003) ^ (narrow / 7) as u64 ^ (((narrow % 10) as u64) << 40);
        let byte = n as u8;
        digest = digest.wrapping_mul(1000003) ^ (byte / 3) as u64 ^ (((byte % 5) as u64) << 20);
    }
    println!("{}", digest);

    // Signed division rounds towards zero; the remainder takes the
    // dividend's sign.
    for n in [-9i64, -8, -7, -1, 0, 1, 7, 8, 9, i64::MIN + 1, i64::MAX].iter() {
        println!("{} {} {} {} {} {}", n / 2, n % 2, n / 8, n % 8, n / 7, n % 7);
    }
    for n in [-128i8, -127, -5, -1, 0, 1, 5, 127].iter() {
        println!("{} {} {} {}", n / 4, n % 4, n / 3, n % 3);
    }
    println!("{} {}", 12345u32.wrapping_mul(4096), 7i32 * 8 / 4);
}
