fn sum(s: &[i32]) -> i32 { s.iter().sum() }
fn main() {
    let v = vec![10, 20, 30, 40, 50];
    let s = &v[1..4];
    println!("{:?} {} {}", s, s.len(), sum(s));
    println!("{:?} {:?}", v.first(), v.last());
    let (l, r) = v.split_at(2);
    println!("{:?} {:?}", l, r);
    for c in v.chunks(2) { print!("{:?} ", c); }
    println!();
    for w in v.windows(3) { print!("{} ", w.iter().sum::<i32>()); }
    println!();
    println!("{}", v.contains(&30));
    println!("{}", v.starts_with(&[10, 20]));
    println!("{:?} {:?}", v.binary_search(&40), v.binary_search(&35));
    println!("{:?}", [[1, 2], [3, 4]].concat());
    println!("{}", ["a", "b", "c"].join("-"));
    let mut m = [3, 1, 2];
    m.reverse();
    println!("{:?}", m);
    m.swap(0, 2);
    println!("{:?}", m);
    println!("{:?}", &v[..2]);
    println!("{:?}", &v[3..]);
    println!("{:?}", v.iter().rev().collect::<Vec<_>>());
    println!("{:?}", v.get(10));
    let mut arr = [5, 4, 3, 2, 1];
    arr[1..4].sort();
    println!("{:?}", arr);
    println!("{:?}", v.iter().position(|&x| x == 30));
    println!("{:?}", v.to_vec().len());
    println!("{}", v.is_empty());
}
