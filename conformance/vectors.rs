// Vec, arrays and slices.
fn total(values: &[i64]) -> i64 {
    let mut sum = 0;
    for v in values {
        sum += v;
    }
    sum
}

fn double_all(values: &mut Vec<i64>) {
    for v in values.iter_mut() {
        *v *= 2;
    }
}

fn largest(values: &[i32]) -> Option<i32> {
    let mut best = *values.first()?;
    for &v in values {
        if v > best {
            best = v;
        }
    }
    Some(best)
}

fn main() {
    let mut v: Vec<i64> = Vec::new();
    for i in 0..100 {
        v.push(i * i);
    }
    println!("{} {} {}", v.len(), v[10], v[99]);
    println!("{}", total(&v));
    double_all(&mut v);
    println!("{}", v[3]);
    println!("{:?}", v.pop());
    v.truncate(5);
    println!("{:?}", v);
    v.insert(1, -1);
    v.remove(0);
    println!("{:?}", v);
    v.reverse();
    println!("{:?}", v);
    v.sort();
    println!("{:?}", v);
    println!("{} {}", v.contains(&8), v.contains(&9));
    println!("{:?} {:?}", v.first(), v.last());
    println!("{:?}", &v[1..3]);
    v.clear();
    println!("{} {:?}", v.is_empty(), v.pop());

    let arr = [5, 3, 8, 1];
    println!("{} {}", arr.len(), arr[2]);
    println!("{:?}", largest(&arr));
    println!("{:?}", largest(&[]));
    let grid = [[1, 2, 3], [4, 5, 6]];
    println!("{}", grid[1][2] + grid[0][0]);
    let mut zeros = [0u8; 10];
    zeros[9] = 7;
    println!("{:?}", zeros);

    let mut nested: Vec<Vec<i32>> = Vec::new();
    for i in 0..3 {
        nested.push(vec![i; i as usize + 1]);
    }
    println!("{:?}", nested);
    nested[2][0] = 99;
    println!("{}", nested[2].iter().sum::<i32>());

    let mut big: Vec<i32> = (0..50).rev().collect();
    big.sort();
    println!("{} {} {}", big[0], big[25], big[49]);
    let strings = vec![String::from("b"), String::from("a")];
    let mut sorted = strings.clone();
    sorted.sort();
    println!("{:?} {:?}", strings, sorted);
    let v2: Vec<(i32, char)> = vec![(1, 'a'), (2, 'b')];
    for (n, c) in &v2 {
        println!("{} {}", n, c);
    }
}
