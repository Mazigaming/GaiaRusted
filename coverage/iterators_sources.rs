use std::iter;
fn main() {
    let mut n = 0;
    let counter = iter::from_fn(|| { n += 1; if n <= 3 { Some(n * n) } else { None } });
    println!("{:?}", counter.collect::<Vec<_>>());
    println!("{:?}", iter::repeat("ab").take(3).collect::<String>());
    println!("{:?}", iter::once(1).chain(iter::once(2)).collect::<Vec<_>>());
    println!("{:?}", iter::empty::<i32>().next());
    println!("{:?}", iter::successors(Some(1u32), |x| if *x < 100 { Some(x * 3) } else { None }).collect::<Vec<_>>());
    println!("{:?}", (1..=4).map(|i| i * 2).collect::<std::collections::VecDeque<_>>());
    println!("{:?}", iter::repeat_with(|| 7).take(2).collect::<Vec<_>>());
    let v: Vec<i32> = (0..5).collect();
    let mut it = v.into_iter();
    println!("{:?} {:?} {:?}", it.next(), it.next_back(), it.len());
    let total: i32 = (1..=100).sum();
    println!("{}", total);
    println!("{:?}", (b'a'..=b'e').map(|b| b as char).collect::<String>());
    println!("{:?}", ('a'..'f').step_by(2).collect::<Vec<_>>());
}
