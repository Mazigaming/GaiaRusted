fn add_one(x: i32) -> i32 { x + 1 }
fn double(x: i32) -> i32 { x * 2 }
fn apply_n(f: fn(i32) -> i32, n: usize, x: i32) -> i32 { (0..n).fold(x, |acc, _| f(acc)) }
fn pick(op: char) -> fn(i32, i32) -> i32 { match op { '+' => |a, b| a + b, '*' => |a, b| a * b, _ => |a, _| a } }
struct Op { name: &'static str, f: fn(f64) -> f64 }
fn main() {
    let table: [fn(i32) -> i32; 2] = [add_one, double];
    println!("{:?}", table.iter().map(|f| f(10)).collect::<Vec<_>>());
    println!("{}", apply_n(double, 5, 1));
    println!("{} {}", pick('+')(3, 4), pick('*')(3, 4));
    let ops = [Op { name: "sqrt", f: f64::sqrt }, Op { name: "abs", f: f64::abs }];
    for op in &ops { println!("{} {}", op.name, (op.f)(-16f64.abs())); }
    let strs: Vec<String> = vec![1, 2].iter().map(ToString::to_string).collect();
    println!("{:?}", strs);
    let lens: Vec<usize> = ["ab", "c"].iter().map(|s| str::len(s)).collect();
    println!("{:?}", lens);
    println!("{:?}", Some(4).map(double));
}
