fn take(v: Vec<i32>) -> usize { v.len() }
fn borrow(v: &Vec<i32>) -> i32 { v.iter().sum() }
fn modify(v: &mut Vec<i32>) { v.push(99); v[0] = -1; }
fn longest_word(s: &str) -> &str { s.split(' ').max_by_key(|w| w.len()).unwrap_or("") }
fn main() {
    let v = vec![1, 2, 3];
    let w = v.clone();
    println!("{}", take(v));
    println!("{}", borrow(&w));
    let mut m = w;
    modify(&mut m);
    println!("{:?}", m);
    let r1 = &m; let r2 = &m;
    println!("{} {}", r1.len(), r2[1]);
    let mut s = String::from("hi");
    { let r = &mut s; r.push('!'); }
    println!("{}", s);
    let s2 = s;
    println!("{}", s2);
    println!("{}", longest_word("a quick brown fox"));
    let mut nums = [1, 2, 3];
    for n in nums.iter_mut() { *n *= 10; }
    println!("{:?}", nums);
    let mut x = 5;
    let y = &mut x; *y += 1;
    println!("{}", x);
    let pair = (String::from("a"), String::from("b"));
    let (a, b) = pair;
    println!("{}{}", a, b);
}
