fn main() {
    println!("start");
    let x: Option<i32> = None;
    let v = vec![1, 2, 3];
    let i = v.len() + 2;
    if x.is_none() {
        println!("index {}", v[i]);
    }
}
