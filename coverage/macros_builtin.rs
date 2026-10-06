use std::fmt::Write as FmtWrite;
fn main() {
    let v = vec![1, 2, 3];
    assert!(v.len() == 3);
    assert_eq!(v[0], 1, "first element");
    assert_ne!(v[1], 5);
    debug_assert!(true);
    let mut s = String::new();
    write!(s, "{}-{}", 1, 2).unwrap();
    writeln!(s, "!").unwrap();
    print!("{}", s);
    println!("{}", stringify!(a + b * c));
    println!("{}", concat!("ab", 1, 'c', true));
    println!("{}", matches!(Some(3), Some(x) if x > 2));
    let file = file!();
    println!("{}", file.ends_with(".rs"));
    println!("{}", column!() > 0);
    let r: Result<i32, ()> = Ok(1);
    println!("{:?}", r);
    let x = dbg!(2 * 3);
    println!("{}", x);
    if v.is_empty() { unreachable!(); }
    if false { todo!(); }
    if false { unimplemented!("later"); }
    println!("{}", format!("{v:?}"));
    println!("{}", vec![0u8; 3].len());
}
