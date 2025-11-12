fn main() {
    let x = 42;
    let y = 10;
    let z = x + y;
    
    let arr = [1, 2, 3];
    let first = arr[0];
    
    if x > y {
        println!("x is greater than y");
    } else {
        println!("y is greater than or equal to x");
    }
    
    let mut count = 0;
    while count < 5 {
        println!("Count: {}", count);
        count = count + 1;
    }
    
    for i in [1, 2, 3, 4, 5] {
        let doubled = i * 2;
    }
}
