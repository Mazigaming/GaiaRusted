use std::convert::TryFrom;
fn main() {
    let a: i64 = 300;
    println!("{} {} {}", a as u8, a as i16, a as f64);
    let b: u8 = 200;
    let c: u32 = b.into();
    let d = u32::from(b);
    println!("{} {}", c, d);
    println!("{:?} {:?}", u8::try_from(300i32), u8::try_from(30i32));
    let e: Result<i8, _> = 200u8.try_into();
    println!("{:?}", e.is_err());
    let f: i64 = i64::from(-5i32);
    println!("{}", f);
    let g = 3.7f64 as u64;
    println!("{}", g);
    let s = 42.to_string();
    let n: i32 = "  17".trim().parse().unwrap();
    println!("{} {}", s, n + 1);
    let fl: f64 = "2.5".parse().unwrap();
    println!("{}", fl * 2.0);
    println!("{:?}", "abc".parse::<i32>().is_err());
    let v: Vec<u8> = "AB".bytes().collect();
    println!("{:?}", v);
    let t = String::from_utf8(vec![104, 105]).unwrap();
    println!("{}", t);
    println!("{}", true as u8 + 1);
    println!("{}", 'Z' as u8 - b'A');
}
