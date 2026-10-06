fn main() {
    println!("before");
    let v: Vec<i32> = Vec::new();
    let r = std::panic::catch_unwind(|| 1);
    println!("{}", r.is_ok());
    if v.is_empty() {
        std::process::exit(3);
    }
    println!("unreachable");
}
