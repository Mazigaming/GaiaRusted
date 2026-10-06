// The built-in macros that read the program itself, and functions used as values.
mod shapes {
    pub fn here() -> &'static str {
        module_path!()
    }
}

fn double(x: i32) -> i32 {
    x * 2
}

fn main() {
    println!("{} | {}", stringify!(x + 1), stringify!(Vec::new()));
    println!("{}", concat!("v", 2, '.', 0, -1, false, 1.5));
    println!("{} {}", line!(), column!());
    println!("{} {}", module_path!(), shapes::here());
    let names: Vec<String> = vec![1, 2, 3].iter().map(ToString::to_string).collect();
    println!("{:?}", names);
    let doubled: Vec<i32> = vec![1, 2].into_iter().map(double).collect();
    let absolute: Vec<i32> = vec![-3, 4].into_iter().map(i32::abs).collect();
    let roots: Vec<f64> = vec![4.0, 9.0].into_iter().map(f64::sqrt).collect();
    println!("{:?} {:?} {:?}", doubled, absolute, roots);
    let lengths: Vec<usize> = ["ab", "c"].iter().map(|s| str::len(s)).collect();
    println!("{:?} {:?}", lengths, Some(4).map(double));
}
