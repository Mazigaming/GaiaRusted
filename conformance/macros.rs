// `macro_rules!`: matching, repetition, recursion and expansion in every
// position.
macro_rules! square {
    ($x:expr) => { $x * $x };
}

macro_rules! maximum {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => {{
        let a = $x;
        let b = maximum!($($rest),+);
        if a > b { a } else { b }
    }};
}

macro_rules! count {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) };
}

macro_rules! make_point {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Copy, Default)]
        struct $name { $($field: $ty),* }
        impl $name {
            fn sum(&self) -> f64 { 0.0 $(+ self.$field as f64)* }
        }
    };
}

make_point!(Point3 { x: f64, y: f64, z: i32 });

macro_rules! impl_describe {
    ($($t:ty => $label:literal),* $(,)?) => {
        $(impl Describe for $t { fn describe(&self) -> String { format!("{} {}", $label, self) } })*
    };
}

trait Describe { fn describe(&self) -> String; }
impl_describe! { i32 => "int", f64 => "float", bool => "flag", }

macro_rules! pairs {
    ($($a:expr => $b:expr);*) => { vec![$(($a, $b)),*] };
}

macro_rules! either {
    (left $e:expr) => { format!("L:{}", $e) };
    (right $e:expr) => { format!("R:{}", $e) };
    ($other:ident $e:expr) => { format!("{}?{}", stringify_ident!($other), $e) };
}

macro_rules! stringify_ident {
    ($i:ident) => { "other" };
}

macro_rules! repeat_twice {
    ($($s:stmt;)*) => { $($s;)* $($s;)* };
}

macro_rules! matrix {
    ($([$($x:expr),*]),*) => { vec![$(vec![$($x),*]),*] };
}

macro_rules! with_block {
    ($b:block) => { { let r = $b; r * 10 } };
}

macro_rules! check_pattern {
    ($value:expr, $p:pat) => { match $value { $p => true, _ => false } };
}

fn main() {
    println!("{} {}", square!(7), square!(2 + 3));
    println!("{}", 100 - square!(3));
    println!("{}", maximum!(3, 9, 4, 1));
    println!("{}", maximum!(5));
    println!("{}", count!(a b c d e));
    let p = Point3 { x: 1.5, y: 2.5, z: 3 };
    println!("{:?} {}", p, p.sum());
    println!("{:?}", Point3::default());
    println!("{} {} {}", 5.describe(), 2.5.describe(), true.describe());
    println!("{:?}", pairs!(1 => 'a'; 2 => 'b'; 3 => 'c'));
    println!("{} {} {}", either!(left 1), either!(right 2), either!(middle 3));
    let mut total = 0;
    repeat_twice!(total += 1; total *= 3;);
    println!("{}", total);
    println!("{:?}", matrix!([1, 2], [3, 4], [5]));
    println!("{}", with_block!({ 4 + 1 }));
    println!("{} {}", check_pattern!(Some(3), Some(1..=5)), check_pattern!(None::<i32>, Some(_)));

    macro_rules! local_add {
        ($a:expr, $b:expr) => { $a + $b };
    }
    println!("{}", local_add!(2, 3) * 2);
    let squares: Vec<i32> = (1..=4).map(|n| square!(n)).collect();
    println!("{:?}", squares);
}
