fn main() {
    let v = vec![1, 2, 3, 4, 5, 6];
    println!("{:?}", v.iter().map(|x| x * x).filter(|x| x % 2 == 0).collect::<Vec<_>>());
    println!("{:?}", v.iter().enumerate().filter(|(i, _)| i % 2 == 0).map(|(_, x)| *x).collect::<Vec<_>>());
    println!("{:?}", v.iter().zip(v.iter().skip(1)).map(|(a, b)| b - a).collect::<Vec<_>>());
    println!("{:?}", v.iter().chain([7, 8].iter()).count());
    println!("{:?}", v.iter().take(2).chain(v.iter().skip(4)).collect::<Vec<_>>());
    println!("{:?}", (0..20).step_by(5).collect::<Vec<_>>());
    println!("{:?}", v.iter().rev().take(3).collect::<Vec<_>>());
    println!("{:?}", vec![vec![1, 2], vec![3]].into_iter().flatten().collect::<Vec<_>>());
    println!("{:?}", (1..4).flat_map(|x| (0..x).map(move |y| x * 10 + y)).collect::<Vec<_>>());
    println!("{:?}", ["1", "x", "3"].iter().filter_map(|s| s.parse::<i32>().ok()).collect::<Vec<_>>());
    println!("{:?}", v.iter().take_while(|&&x| x < 4).collect::<Vec<_>>());
    println!("{:?}", v.iter().skip_while(|&&x| x < 4).collect::<Vec<_>>());
    let mut p = v.iter().peekable();
    while let Some(x) = p.next() { if let Some(&&nx) = p.peek() { print!("{}<{} ", x, nx); } }
    println!();
    println!("{:?}", v.iter().scan(0, |acc, &x| { *acc += x; Some(*acc) }).collect::<Vec<_>>());
    println!("{:?}", [1, 2, 3].iter().cycle().take(7).collect::<Vec<_>>());
    let mut seen = vec![]; let total: i32 = v.iter().inspect(|x| seen.push(**x)).sum(); println!("{} {}", total, seen.len());
    println!("{:?}", v.chunks(4).map(|c| c.iter().sum::<i32>()).collect::<Vec<_>>());
    println!("{:?}", v.windows(2).map(|w| w[0] * w[1]).collect::<Vec<_>>());
    println!("{:?}", v.iter().map(|x| x * 2).rev().collect::<Vec<_>>());
    println!("{:?}", "abc".chars().map(|c| c.to_ascii_uppercase()).collect::<String>());
    println!("{:?}", v.iter().copied().filter(|x| x % 3 == 0).last());
    println!("{:?}", (1..=3).map(|i| i.to_string()).collect::<Vec<String>>().join("+"));
    println!("{:?}", v.iter().step_by(2).zip("abc".chars()).collect::<Vec<_>>());
    println!("{:?}", v.iter().map_while(|&x| if x < 3 { Some(x * 10) } else { None }).collect::<Vec<_>>());
    let nested = vec![Some(1), None, Some(3)];
    println!("{:?}", nested.iter().flatten().collect::<Vec<_>>());
    println!("{:?}", v.rchunks(4).collect::<Vec<_>>());
}
