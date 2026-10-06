// Const generic parameters, and the arrays of every length they give traits to.
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::convert::TryFrom;

const SIZE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Buffer<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> Buffer<N> {
    fn new() -> Self {
        Buffer { data: [0; N], len: 0 }
    }

    fn capacity(&self) -> usize {
        N
    }

    fn push(&mut self, byte: u8) -> bool {
        if self.len == N {
            return false;
        }
        self.data[self.len] = byte;
        self.len += 1;
        true
    }
}

fn sum<const N: usize>(values: [i32; N]) -> i32 {
    let mut total = 0;
    for i in 0..N {
        total += values[i];
    }
    total
}

fn first_n<T: Copy + Default, const N: usize>(items: &[T]) -> [T; N] {
    let mut out = [T::default(); N];
    for (slot, item) in out.iter_mut().zip(items) {
        *slot = *item;
    }
    out
}

struct Matrix<const R: usize, const C: usize> {
    cells: [[f64; C]; R],
}

impl<const R: usize, const C: usize> Matrix<R, C> {
    fn transpose(&self) -> Matrix<C, R> {
        let mut cells = [[0.0; R]; C];
        for r in 0..R {
            for c in 0..C {
                cells[c][r] = self.cells[r][c];
            }
        }
        Matrix { cells }
    }

    fn dims(&self) -> (usize, usize) {
        (R, C)
    }
}

fn arrays() {
    let words = [String::from("one"), String::from("two"), String::from("three")];
    let lengths = words.clone().map(|word| word.len());
    let mut joined = String::new();
    for word in words {
        joined.push_str(&word);
    }
    println!("{:?} {}", lengths, joined);
    let squares: [u32; 5] = std::array::from_fn(|i| (i * i) as u32);
    let mut backwards = squares.into_iter().rev();
    println!("{:?} {:?} {:?}", squares, backwards.next(), backwards.len());
    let bytes = [1u8, 2, 3, 4, 5];
    let head = <[u8; 3]>::try_from(&bytes[..3]).unwrap();
    let wrong = <[u8; 3]>::try_from(&bytes[..2]);
    println!("{:?} {}", head, wrong.is_err());
    let grid: [[char; 2]; 2] = Default::default();
    println!("{:?} {}", grid, [1, 2, 3] < [1, 3, 0]);
    let v = Vec::from([3, 1, 2]);
    println!("{} {}", v == [3, 1, 2], v == vec![3, 1, 2]);
    let map = HashMap::from([("a", 1), ("b", 2)]);
    let ordered = BTreeMap::from([(2, 'b'), (1, 'a')]);
    let set = HashSet::from([1, 2, 2, 3]);
    let deque = VecDeque::from([1, 2, 3]);
    println!("{} {:?} {} {:?}", map["b"], ordered, set.len(), deque);
    let pairs: Vec<(i32, char)> = [1, 2].into_iter().zip(['x', 'y']).collect();
    println!("{:?}", pairs);
}

fn main() {
    let mut buffer: Buffer<3> = Buffer::new();
    println!("{} {} {} {}", buffer.push(7), buffer.push(8), buffer.push(9), buffer.push(10));
    println!("{:?} {}", buffer, buffer.capacity());
    let other = Buffer::<SIZE>::new();
    println!("{} {:?}", other.capacity(), other.data);
    println!("{} {}", sum([1, 2, 3]), sum([10; 5]));
    let firsts: [i64; 2] = first_n(&[5, 6, 7]);
    println!("{:?} {:?}", firsts, first_n::<char, 3>(&['a', 'b']));
    let m = Matrix { cells: [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]] };
    let t = m.transpose();
    println!("{:?} {:?} {:?}", m.dims(), t.dims(), t.cells);
    let copy = buffer;
    println!("{}", copy == buffer);
    arrays();
}
