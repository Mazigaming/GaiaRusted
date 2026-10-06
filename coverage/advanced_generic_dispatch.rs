use std::collections::HashMap;
use std::hash::Hash;
fn group_by<T, K: Hash + Eq + Ord + Clone, F: Fn(&T) -> K>(items: Vec<T>, key: F) -> Vec<(K, Vec<T>)> {
    let mut m: HashMap<K, Vec<T>> = HashMap::new();
    for i in items { m.entry(key(&i)).or_default().push(i); }
    let mut out: Vec<_> = m.into_iter().collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}
struct Cache<F: Fn(u64) -> u64> { f: F, memo: HashMap<u64, u64> }
impl<F: Fn(u64) -> u64> Cache<F> { fn get(&mut self, x: u64) -> u64 { if let Some(v) = self.memo.get(&x) { return *v; } let v = (self.f)(x); self.memo.insert(x, v); v } }
trait Stack<T> { fn push_item(&mut self, t: T); fn pop_item(&mut self) -> Option<T>; fn peek_item(&self) -> Option<&T>; }
impl<T> Stack<T> for Vec<T> { fn push_item(&mut self, t: T) { self.push(t) } fn pop_item(&mut self) -> Option<T> { self.pop() } fn peek_item(&self) -> Option<&T> { self.last() } }
fn main() {
    let g = group_by(vec!["apple", "avocado", "banana", "cherry", "blueberry"], |s| s.chars().next().unwrap());
    println!("{:?}", g);
    let mut c = Cache { f: |x| x * x + 1, memo: HashMap::new() };
    println!("{} {} {}", c.get(3), c.get(3), c.memo.len());
    let mut st: Vec<i32> = Vec::new();
    st.push_item(1); st.push_item(2);
    println!("{:?}", st.peek_item());
    println!("{:?}", st.pop_item());
    println!("{:?}", st.len());
}
