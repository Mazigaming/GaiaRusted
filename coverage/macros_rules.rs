macro_rules! square { ($x:expr) => { $x * $x }; }
macro_rules! maximum { ($x:expr) => { $x }; ($x:expr, $($rest:expr),+) => { { let a = $x; let b = maximum!($($rest),+); if a > b { a } else { b } } }; }
macro_rules! make_struct { ($name:ident { $($field:ident : $ty:ty),* }) => { #[derive(Debug, Default)] struct $name { $($field: $ty),* } }; }
macro_rules! hashmap { ($($k:expr => $v:expr),* $(,)?) => {{ let mut m = std::collections::BTreeMap::new(); $(m.insert($k, $v);)* m }}; }
macro_rules! count { () => { 0usize }; ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) }; }
make_struct!(Config { width: u32, name: String });
fn main() {
    println!("{}", square!(7));
    println!("{}", square!(2 + 3));
    println!("{}", maximum!(3, 9, 4, 1));
    println!("{:?}", Config::default());
    println!("{:?}", hashmap!{ "a" => 1, "b" => 2, });
    println!("{}", count!(a b c d));
}
