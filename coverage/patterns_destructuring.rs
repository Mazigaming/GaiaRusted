struct Point3 { x: i32, y: i32, z: i32 }
struct Rect { tl: (i32, i32), br: (i32, i32) }
fn area(Rect { tl: (x1, y1), br: (x2, y2) }: &Rect) -> i32 { (x2 - x1) * (y2 - y1) }
fn sum_pair(&(a, b): &(i32, i32)) -> i32 { a + b }
fn main() {
    let p = Point3 { x: 1, y: 2, z: 3 };
    let Point3 { x, y: why, .. } = p;
    println!("{} {} {}", x, why, p.z);
    println!("{}", area(&Rect { tl: (0, 0), br: (3, 4) }));
    println!("{}", sum_pair(&(4, 5)));
    let pts = vec![(1, 'a'), (2, 'b')];
    for &(n, c) in &pts { print!("{}{} ", n, c); }
    println!();
    for (i, (n, c)) in pts.iter().enumerate() { print!("{}:{}{} ", i, n, c); }
    println!();
    let (mut a, mut b) = (1, 2);
    std::mem::swap(&mut a, &mut b);
    (a, b) = (b * 10, a * 10);
    println!("{} {}", a, b);
    let [x0, _, x2] = [7, 8, 9];
    println!("{} {}", x0, x2);
    let nested = Some((1, Some("deep")));
    if let Some((n, Some(s))) = nested { println!("{} {}", n, s); }
    let ref r = 5;
    println!("{}", *r + 1);
}
