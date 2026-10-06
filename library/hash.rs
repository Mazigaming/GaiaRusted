//! Hashing: turning a value into a `u64`, as hash tables need.
//!
//! The traits are Rust's. A `Hasher` consumes bytes and says what they
//! hash to; a `Hash` type feeds itself to one. Every type here feeds the
//! same bytes as under rustc, and `DefaultHasher` is SipHash-1-3 with zero
//! keys like rustc's, so a hash a program prints is the one rustc's prints.

use std::fmt;

pub trait Hasher {
    fn finish(&self) -> u64;

    fn write(&mut self, bytes: &[u8]);

    fn write_u8(&mut self, value: u8) {
        self.write(&[value]);
    }

    fn write_u16(&mut self, value: u16) {
        self.write(&value.to_ne_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.write(&value.to_ne_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.write(&value.to_ne_bytes());
    }

    fn write_u128(&mut self, value: u128) {
        self.write(&value.to_ne_bytes());
    }

    fn write_usize(&mut self, value: usize) {
        self.write(&value.to_ne_bytes());
    }

    fn write_i8(&mut self, value: i8) {
        self.write_u8(value as u8);
    }

    fn write_i16(&mut self, value: i16) {
        self.write_u16(value as u16);
    }

    fn write_i32(&mut self, value: i32) {
        self.write_u32(value as u32);
    }

    fn write_i64(&mut self, value: i64) {
        self.write_u64(value as u64);
    }

    fn write_i128(&mut self, value: i128) {
        self.write_u128(value as u128);
    }

    fn write_isize(&mut self, value: isize) {
        self.write_usize(value as usize);
    }

    /// The length of a sequence whose elements follow, so that sequences
    /// that only differ in where one ends and the next begins hash apart.
    fn write_length_prefix(&mut self, len: usize) {
        self.write_usize(len);
    }

    /// A string: its bytes, then `0xff`, which no string contains.
    fn write_str(&mut self, text: &str) {
        self.write(text.as_bytes());
        self.write_u8(0xff);
    }
}

impl<H: Hasher + ?Sized> Hasher for &mut H {
    fn finish(&self) -> u64 {
        (**self).finish()
    }

    fn write(&mut self, bytes: &[u8]) {
        (**self).write(bytes)
    }

    fn write_u8(&mut self, value: u8) {
        (**self).write_u8(value)
    }

    fn write_u16(&mut self, value: u16) {
        (**self).write_u16(value)
    }

    fn write_u32(&mut self, value: u32) {
        (**self).write_u32(value)
    }

    fn write_u64(&mut self, value: u64) {
        (**self).write_u64(value)
    }

    fn write_usize(&mut self, value: usize) {
        (**self).write_usize(value)
    }

    fn write_str(&mut self, text: &str) {
        (**self).write_str(text)
    }
}

pub trait Hash {
    fn hash<H: Hasher>(&self, state: &mut H);

    /// Feed every element of `data`, in order.
    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        for item in data {
            item.hash(state);
        }
    }
}

/// Integers feed their bytes; a slice of them feeds all its bytes at once.
macro_rules! integer_hash {
    ($($t:ident $write:ident)*) => {
        $(
            impl Hash for $t {
                fn hash<H: Hasher>(&self, state: &mut H) {
                    state.$write(*self);
                }

                fn hash_slice<H: Hasher>(data: &[$t], state: &mut H) {
                    let size = data.len() * std::intrinsics::size_of::<$t>();
                    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, size) };
                    state.write(bytes);
                }
            }
        )*
    };
}

integer_hash! {
    u8 write_u8 u16 write_u16 u32 write_u32 u64 write_u64 u128 write_u128 usize write_usize
    i8 write_i8 i16 write_i16 i32 write_i32 i64 write_i64 i128 write_i128 isize write_isize
}

impl Hash for bool {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(*self as u8);
    }
}

impl Hash for char {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(*self as u32);
    }
}

impl Hash for str {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_str(self);
    }
}

impl Hash for String {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_str(self.as_str());
    }
}

impl<T: Hash + ?Sized> Hash for &T {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hash + ?Sized> Hash for &mut T {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl Hash for () {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

impl<T: Hash> Hash for [T] {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_length_prefix(self.len());
        Hash::hash_slice(self, state);
    }
}

impl<T: Hash> Hash for Option<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            None => state.write_isize(0),
            Some(value) => {
                state.write_isize(1);
                value.hash(state);
            }
        }
    }
}

macro_rules! tuple_hash {
    ($(($($name:ident $index:tt),+))*) => {
        $(
            impl<$($name: Hash),+> Hash for ($($name,)+) {
                fn hash<H: Hasher>(&self, state: &mut H) {
                    $(self.$index.hash(state);)+
                }
            }
        )*
    };
}

tuple_hash! {
    (A 0)
    (A 0, B 1)
    (A 0, B 1, C 2)
    (A 0, B 1, C 2, D 3)
    (A 0, B 1, C 2, D 3, E 4)
    (A 0, B 1, C 2, D 3, E 4, F 5)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11)
}

/// Makes the hashers a hash table uses for its keys.
pub trait BuildHasher {
    type Hasher: Hasher;

    fn build_hasher(&self) -> Self::Hasher;

    /// The hash of one value, from a fresh hasher.
    fn hash_one<T: Hash>(&self, value: T) -> u64 {
        let mut hasher = self.build_hasher();
        value.hash(&mut hasher);
        hasher.finish()
    }
}

/// Builds hashers of type `H` with `H::default()`.
pub struct BuildHasherDefault<H> {
    _hasher: std::marker::PhantomData<H>,
}

impl<H> BuildHasherDefault<H> {
    pub fn new() -> BuildHasherDefault<H> {
        BuildHasherDefault { _hasher: std::marker::PhantomData }
    }
}

impl<H: Default + Hasher> BuildHasher for BuildHasherDefault<H> {
    type Hasher = H;

    fn build_hasher(&self) -> H {
        H::default()
    }
}

impl<H> Default for BuildHasherDefault<H> {
    fn default() -> BuildHasherDefault<H> {
        BuildHasherDefault::new()
    }
}

impl<H> Clone for BuildHasherDefault<H> {
    fn clone(&self) -> BuildHasherDefault<H> {
        BuildHasherDefault::new()
    }
}

impl<H> fmt::Debug for BuildHasherDefault<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("BuildHasherDefault")
    }
}

/// The hash tables' default `BuildHasher`. Under rustc its keys are
/// random; nothing a program can rely on depends on them.
#[derive(Clone, Debug, Default)]
pub struct RandomState {
    k0: u64,
    k1: u64,
}

impl RandomState {
    pub fn new() -> RandomState {
        RandomState { k0: 0, k1: 0 }
    }
}

impl BuildHasher for RandomState {
    type Hasher = DefaultHasher;

    fn build_hasher(&self) -> DefaultHasher {
        DefaultHasher(SipHasher13::new_with_keys(self.k0, self.k1))
    }
}

/// The default hasher: SipHash-1-3, with zero keys from `new`.
#[derive(Clone, Debug)]
pub struct DefaultHasher(SipHasher13);

impl DefaultHasher {
    pub fn new() -> DefaultHasher {
        DefaultHasher(SipHasher13::new_with_keys(0, 0))
    }
}

impl Default for DefaultHasher {
    fn default() -> DefaultHasher {
        DefaultHasher::new()
    }
}

impl Hasher for DefaultHasher {
    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes);
    }

    fn write_str(&mut self, text: &str) {
        self.0.write_str(text);
    }

    fn finish(&self) -> u64 {
        self.0.finish()
    }
}

/// SipHash with one compression round per word and three to finish.
#[derive(Clone, Debug)]
struct SipHasher13 {
    v0: u64,
    v1: u64,
    v2: u64,
    v3: u64,
    /// Bytes written so far.
    length: usize,
    /// The bytes of the word being filled, least significant first.
    tail: u64,
    /// How many bytes `tail` holds.
    ntail: usize,
}

impl SipHasher13 {
    fn new_with_keys(k0: u64, k1: u64) -> SipHasher13 {
        SipHasher13 {
            v0: k0 ^ 0x736f6d6570736575,
            v1: k1 ^ 0x646f72616e646f6d,
            v2: k0 ^ 0x6c7967656e657261,
            v3: k1 ^ 0x7465646279746573,
            length: 0,
            tail: 0,
            ntail: 0,
        }
    }

    fn round(&mut self) {
        self.v0 = self.v0.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(13) ^ self.v0;
        self.v0 = self.v0.rotate_left(32);
        self.v2 = self.v2.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(16) ^ self.v2;
        self.v0 = self.v0.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(21) ^ self.v0;
        self.v2 = self.v2.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(17) ^ self.v2;
        self.v2 = self.v2.rotate_left(32);
    }

    fn compress(&mut self, word: u64) {
        self.v3 ^= word;
        self.round();
        self.v0 ^= word;
    }
}

/// Up to eight bytes as a little-endian word.
fn little_endian(bytes: &[u8]) -> u64 {
    let mut word = 0u64;
    for (index, &byte) in bytes.iter().enumerate() {
        word |= (byte as u64) << (8 * index);
    }
    word
}

impl Hasher for SipHasher13 {
    fn write(&mut self, bytes: &[u8]) {
        self.length += bytes.len();
        let mut start = 0;
        if self.ntail != 0 {
            let needed = 8 - self.ntail;
            let taken = needed.min(bytes.len());
            self.tail |= little_endian(&bytes[..taken]) << (8 * self.ntail);
            if bytes.len() < needed {
                self.ntail += bytes.len();
                return;
            }
            let word = self.tail;
            self.compress(word);
            self.ntail = 0;
            start = needed;
        }
        let rest = &bytes[start..];
        let whole = rest.len() / 8 * 8;
        let mut at = 0;
        while at < whole {
            let word = unsafe { std::ptr::read_unaligned(rest.as_ptr().add(at) as *const u64) };
            self.compress(word);
            at += 8;
        }
        self.tail = little_endian(&rest[whole..]);
        self.ntail = rest.len() - whole;
    }

    fn finish(&self) -> u64 {
        let mut state = self.clone();
        let last = ((self.length as u64 & 0xff) << 56) | self.tail;
        state.compress(last);
        state.v2 ^= 0xff;
        state.round();
        state.round();
        state.round();
        state.v0 ^ state.v1 ^ state.v2 ^ state.v3
    }
}
