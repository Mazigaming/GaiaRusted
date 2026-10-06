use std::sync::{Arc, Mutex, mpsc};
use std::thread;
fn main() {
    let handles: Vec<_> = (0..4).map(|i| thread::spawn(move || i * i)).collect();
    let results: Vec<i32> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    println!("{:?}", results);
    let counter = Arc::new(Mutex::new(0));
    let mut hs = vec![];
    for _ in 0..8 { let c = Arc::clone(&counter); hs.push(thread::spawn(move || { for _ in 0..1000 { *c.lock().unwrap() += 1; } })); }
    for h in hs { h.join().unwrap(); }
    println!("{}", *counter.lock().unwrap());
    let (tx, rx) = mpsc::channel();
    for id in 0..3 { let tx = tx.clone(); thread::spawn(move || { tx.send(id * 10).unwrap(); }); }
    drop(tx);
    let mut got: Vec<i32> = rx.iter().collect(); got.sort();
    println!("{:?}", got);
    let data = Arc::new(vec![1, 2, 3]);
    let d2 = Arc::clone(&data);
    let sum = thread::spawn(move || d2.iter().sum::<i32>()).join().unwrap();
    println!("{} {}", sum, Arc::strong_count(&data));
    let scoped_total = thread::scope(|s| { let h = s.spawn(|| data.len()); h.join().unwrap() });
    println!("{}", scoped_total);
}
