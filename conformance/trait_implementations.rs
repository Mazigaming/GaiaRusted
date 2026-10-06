// Implementing the standard traits for one's own types: Sum, FromIterator,
// Extend, IntoIterator for references, DoubleEndedIterator, Deref, derived
// Default with #[default], blanket impls, Any, const fn, field drop order.
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::iter::{FromIterator, Sum};
use std::ops::{Deref, DerefMut, Index, Mul};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Money {
    cents: i64,
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        Money { cents: iter.map(|m| m.cents).sum() }
    }
}

impl<'a> Sum<&'a Money> for Money {
    fn sum<I: Iterator<Item = &'a Money>>(iter: I) -> Money {
        iter.copied().sum()
    }
}

impl Mul<i64> for Money {
    type Output = Money;
    fn mul(self, k: i64) -> Money {
        Money { cents: self.cents * k }
    }
}

impl Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "${}.{:02}", self.cents / 100, self.cents % 100)
    }
}

#[derive(Debug, Default)]
struct Bag {
    items: Vec<String>,
}

impl FromIterator<String> for Bag {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Bag {
        Bag { items: iter.into_iter().collect() }
    }
}

impl Extend<String> for Bag {
    fn extend<I: IntoIterator<Item = String>>(&mut self, iter: I) {
        for s in iter {
            self.items.push(s);
        }
    }
}

impl<'a> IntoIterator for &'a Bag {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl Index<usize> for Bag {
    type Output = str;
    fn index(&self, i: usize) -> &str {
        &self.items[i]
    }
}

struct Countdown {
    from: u32,
    to: u32,
}

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.from > self.to {
            self.from -= 1;
            Some(self.from + 1)
        } else {
            None
        }
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        if self.from > self.to {
            self.to += 1;
            Some(self.to)
        } else {
            None
        }
    }
}

struct Stack(Vec<i32>);

impl Deref for Stack {
    type Target = Vec<i32>;
    fn deref(&self) -> &Vec<i32> {
        &self.0
    }
}

impl DerefMut for Stack {
    fn deref_mut(&mut self) -> &mut Vec<i32> {
        &mut self.0
    }
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
enum Mode {
    Fast,
    #[default]
    Normal,
    Slow,
}

trait Describe {
    fn describe(&self) -> String;
}

impl<T: Display> Describe for T {
    fn describe(&self) -> String {
        format!("[{}]", self)
    }
}

#[derive(Debug, PartialEq)]
struct Id(u32);

impl PartialEq<u32> for Id {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}

const fn square(x: u32) -> u32 {
    x * x
}

const AREA: u32 = square(12);

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Pair {
    _first: Noisy,
    _second: Noisy,
}

struct Cached<F: Fn(u64) -> u64> {
    f: F,
    memo: HashMap<u64, u64>,
}

impl<F: Fn(u64) -> u64> Cached<F> {
    fn get(&mut self, x: u64) -> u64 {
        if let Some(&v) = self.memo.get(&x) {
            return v;
        }
        let v = (self.f)(x);
        self.memo.insert(x, v);
        v
    }
}

fn inspect(value: &dyn Any) -> String {
    if let Some(i) = value.downcast_ref::<i32>() {
        format!("i32 {}", i)
    } else if let Some(s) = value.downcast_ref::<String>() {
        format!("string {}", s)
    } else {
        "unknown".to_string()
    }
}

fn main() {
    let wallet = vec![Money { cents: 150 }, Money { cents: 275 }];
    let total: Money = wallet.iter().sum();
    let doubled: Money = wallet.into_iter().map(|m| m * 2).sum();
    println!("{} {} {}", total, doubled, Money::default());

    let mut bag: Bag = vec!["a".to_string(), "b".to_string()].into_iter().collect();
    bag.extend(vec!["c".to_string()]);
    for item in &bag {
        print!("{} ", item);
    }
    println!("{} {:?}", &bag[2], bag);

    println!("{:?} {:?}", Countdown { from: 5, to: 2 }.collect::<Vec<_>>(), Countdown { from: 5, to: 2 }.rev().collect::<Vec<_>>());
    let mut both = Countdown { from: 6, to: 0 };
    println!("{:?} {:?} {:?}", both.next(), both.next_back(), both.collect::<Vec<_>>());

    let mut st = Stack(vec![1]);
    st.push(2);
    st.extend([3, 4]);
    println!("{} {:?} {:?} {}", st.len(), st.last(), st.iter().rev().collect::<Vec<_>>(), st.contains(&3));

    println!("{:?} {:?}", Mode::default(), [Mode::Fast, Mode::Slow]);
    println!("{} {} {}", 42.describe(), "hi".describe(), Money { cents: 5 }.describe());
    println!("{} {} {}", Id(3) == 3, Id(3) == Id(4), AREA);
    {
        let _pair = Pair { _first: Noisy("first"), _second: Noisy("second") };
        println!("pair built");
    }

    let mut cached = Cached { f: |x| x * x + 1, memo: HashMap::new() };
    println!("{} {} {}", cached.get(3), cached.get(3), cached.memo.len());
    println!("{} {} {}", inspect(&5i32), inspect(&String::from("s")), inspect(&1.5f64));
    let boxed: Box<dyn Any> = Box::new(7u8);
    println!("{:?}", boxed.downcast::<u8>().map(|b| *b + 1).ok());
}
