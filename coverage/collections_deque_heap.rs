use std::collections::{VecDeque, BinaryHeap};
use std::cmp::Reverse;
fn main() {
    let mut q = VecDeque::new();
    q.push_back(1); q.push_back(2); q.push_front(0);
    println!("{:?} {:?} {:?}", q, q.front(), q.back());
    println!("{:?} {:?} {:?}", q.pop_front(), q.pop_back(), q);
    let mut r: VecDeque<i32> = (1..=5).collect();
    r.rotate_left(2);
    println!("{:?} {}", r, r[0]);
    let mut h = BinaryHeap::new();
    for x in [5, 1, 8, 3, 9, 2] { h.push(x); }
    println!("{:?} {}", h.peek(), h.len());
    let mut out = vec![]; while let Some(x) = h.pop() { out.push(x); }
    println!("{:?}", out);
    let mut minh = BinaryHeap::new();
    for x in [5, 1, 8] { minh.push(Reverse(x)); }
    println!("{:?}", minh.pop().map(|Reverse(x)| x));
    let sorted = BinaryHeap::from(vec![3, 1, 2]).into_sorted_vec();
    println!("{:?}", sorted);
}
