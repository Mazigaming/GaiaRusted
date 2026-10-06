fn divide(a: i32, b: i32) -> Option<i32> { if b == 0 { None } else { Some(a / b) } }
fn first_even(v: &[i32]) -> Option<&i32> { v.iter().find(|x| *x % 2 == 0) }
fn chain(a: i32, b: i32, c: i32) -> Option<i32> { let x = divide(a, b)?; let y = divide(x, c)?; Some(x + y) }
fn main() {
    println!("{:?} {:?}", divide(9, 3), divide(1, 0));
    println!("{} {}", divide(9, 3).unwrap_or(-1), divide(1, 0).unwrap_or(-1));
    println!("{:?}", divide(8, 2).map(|x| x * 10));
    println!("{:?}", divide(8, 2).and_then(|x| divide(x, 0)));
    println!("{:?}", divide(8, 0).or(Some(0)));
    println!("{}", divide(8, 0).unwrap_or_else(|| 7));
    println!("{:?}", divide(8, 0).ok_or("div by zero"));
    println!("{:?}", Some(5).filter(|x| *x > 10));
    println!("{} {}", Some(3).is_some(), None::<i32>.is_none());
    println!("{:?} {:?}", chain(100, 5, 2), chain(1, 0, 2));
    println!("{:?}", first_even(&[1, 3, 6, 8]));
    let mut opt = Some(String::from("taken"));
    let t = opt.take();
    println!("{:?} {:?}", t, opt);
    let name: Option<String> = Some("x".into());
    println!("{:?}", name.as_deref());
    println!("{}", name.as_ref().map_or(0, |s| s.len()));
    println!("{:?}", Some(2).zip(Some('b')));
    println!("{:?}", Some(Some(4)).flatten());
    println!("{}", Some(10).map_or_else(|| -1, |v| v * 2));
    let mut counter = None;
    *counter.get_or_insert(0) += 5;
    println!("{:?}", counter);
    println!("{:?}", Some(4).xor(None::<i32>));
    if let Some(v) = divide(10, 2) { println!("got {}", v); }
    let vals: Option<Vec<i32>> = vec![Some(1), Some(2)].into_iter().collect();
    println!("{:?}", vals);
    println!("{}", Some(3).unwrap_or_default() + None::<i32>.unwrap_or_default());
}
