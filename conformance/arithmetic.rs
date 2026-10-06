// Integer and float arithmetic, wrapping, casts and comparisons.
fn main() {
    let a: i32 = 17;
    let b: i32 = 5;
    println!("{} {} {} {} {}", a + b, a - b, a * b, a / b, a % b);
    println!("{} {}", -a / b, -a % b);
    println!("{} {} {}", a & b, a | b, a ^ b);
    println!("{} {}", a << 3, a >> 2);
    println!("{}", !a);

    let x: u8 = 250;
    println!("{}", x.wrapping_add(10));
    println!("{}", (x as i8));
    println!("{}", (x as u32) * 1000);
    println!("{}", -1i32 as u32);
    println!("{}", 300i32 as u8);
    println!("{}", -128i8 as i64);
    println!("{}", u64::MAX);
    println!("{} {}", i64::MIN, i64::MAX);
    println!("{}", (3.99f64 as i32) + (-3.99f64 as i32));
    println!("{}", 7 as f64 / 2.0);
    println!("{}", (1u64 << 40) + 1);

    let f = 2.5f64;
    println!("{} {} {}", f * f, f / 2.0, f - 10.0);
    println!("{} {}", f < 3.0, f == 2.5);
    println!("{}", 1e21);
    println!("{}", 0.1 + 0.2);
    println!("{:?} {:?}", 1.0f64, 1.5f32);
    println!("{}", 10.0f64.sqrt() * 10.0f64.sqrt());
    println!("{} {}", 2i64.pow(10), (-7i32).abs());
    println!("{} {}", 5.max(9), 5.min(9));
    println!("{}", 'a' as u8 + 1);
    println!("{}", (b'a' + 2) as char);
    println!("{}", true as i32 + false as i32);
    println!("{}", 10 > 3 && 3 > 10 || !false);
}
