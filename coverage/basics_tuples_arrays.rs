fn swap(p: (i32, i32)) -> (i32, i32) { (p.1, p.0) }
fn main() {
    let t = (1, "two", 3.0);
    let (a, b, c) = t;
    println!("{} {} {} {}", a, b, c, t.1);
    println!("{:?}", swap((4, 5)));
    let nested = ((1, 2), [3, 4]);
    println!("{:?} {}", nested, (nested.0).1);
    let arr = [5, 3, 8, 1];
    println!("{} {:?} {}", arr.len(), arr, arr[2]);
    let zeros = [0u8; 4];
    println!("{:?}", zeros);
    let mut grid = [[0i32; 3]; 3];
    for i in 0..3 { for j in 0..3 { grid[i][j] = (i * 3 + j) as i32; } }
    println!("{:?}", grid);
    let mut sorted = arr; sorted.sort();
    println!("{:?} {:?}", arr, sorted);
    println!("{}", arr.contains(&8));
    println!("{}", arr.iter().sum::<i32>());
    println!("{}", [1, 2] == [1, 2]);
    let unit = ();
    println!("{:?}", unit);
    let [first, second, ..] = arr;
    println!("{} {}", first, second);
    println!("{:?}", arr.map(|x| x * 2));
}
