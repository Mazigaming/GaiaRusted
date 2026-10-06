//! Comparison.

pub enum Ordering {
    Less = -1,
    Equal = 0,
    Greater = 1,
}

impl Ordering {
    pub fn reverse(self) -> Ordering {
        match self {
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => Ordering::Equal,
            Ordering::Greater => Ordering::Less,
        }
    }

    /// This ordering, or `other` where this one is `Equal`.
    pub fn then(self, other: Ordering) -> Ordering {
        match self {
            Ordering::Equal => other,
            decided => decided,
        }
    }

    pub fn then_with<F: FnOnce() -> Ordering>(self, f: F) -> Ordering {
        match self {
            Ordering::Equal => f(),
            decided => decided,
        }
    }

    pub fn is_ne(self) -> bool { !matches!(self, Ordering::Equal) }
    pub fn is_lt(self) -> bool { matches!(self, Ordering::Less) }
    pub fn is_le(self) -> bool { !matches!(self, Ordering::Greater) }
    pub fn is_gt(self) -> bool { matches!(self, Ordering::Greater) }
    pub fn is_ge(self) -> bool { !matches!(self, Ordering::Less) }
    pub fn is_eq(self) -> bool { matches!(self, Ordering::Equal) }
}

impl PartialEq for Ordering {
    fn eq(&self, other: &Ordering) -> bool {
        std::intrinsics::discriminant(self) == std::intrinsics::discriminant(other)
    }
}

impl Clone for Ordering {
    fn clone(&self) -> Ordering { *self }
}
impl Copy for Ordering {}
impl Eq for Ordering {}

impl std::fmt::Debug for Ordering {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match self {
            Ordering::Less => "Less",
            Ordering::Equal => "Equal",
            Ordering::Greater => "Greater",
        })
    }
}

impl PartialOrd for Ordering {
    fn partial_cmp(&self, other: &Ordering) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl Ord for Ordering {
    fn cmp(&self, other: &Ordering) -> Ordering {
        let rank = |o: &Ordering| match o { Ordering::Less => 0, Ordering::Equal => 1, Ordering::Greater => 2 };
        rank(self).cmp(&rank(other))
    }
}

impl std::hash::Hash for Ordering {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i8(*self as i8);
    }
}

/// Sorting in reverse: `Reverse(a) < Reverse(b)` exactly when `b < a`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub struct Reverse<T>(pub T);

impl<T: PartialOrd> PartialOrd for Reverse<T> {
    fn partial_cmp(&self, other: &Reverse<T>) -> Option<Ordering> { other.0.partial_cmp(&self.0) }
}

impl<T: Ord> Ord for Reverse<T> {
    fn cmp(&self, other: &Reverse<T>) -> Ordering { other.0.cmp(&self.0) }
}

/// `==` and `!=`.
pub trait PartialEq<Rhs = Self> {
    fn eq(&self, other: &Rhs) -> bool;

    fn ne(&self, other: &Rhs) -> bool {
        !self.eq(other)
    }
}

/// Equality that holds for every value (unlike floats, where `NaN != NaN`).
pub trait Eq {}

/// `<`, `<=`, `>` and `>=`.
pub trait PartialOrd<Rhs = Self> {
    fn partial_cmp(&self, other: &Rhs) -> Option<Ordering>;

    fn lt(&self, other: &Rhs) -> bool {
        matches!(self.partial_cmp(other), Option::Some(Ordering::Less))
    }

    fn le(&self, other: &Rhs) -> bool {
        matches!(self.partial_cmp(other), Option::Some(Ordering::Less | Ordering::Equal))
    }

    fn gt(&self, other: &Rhs) -> bool {
        matches!(self.partial_cmp(other), Option::Some(Ordering::Greater))
    }

    fn ge(&self, other: &Rhs) -> bool {
        matches!(self.partial_cmp(other), Option::Some(Ordering::Greater | Ordering::Equal))
    }
}

/// A total order.
pub trait Ord {
    fn cmp(&self, other: &Self) -> Ordering;

    fn max(self, other: Self) -> Self {
        if other.cmp(&self) == Ordering::Less { self } else { other }
    }

    fn min(self, other: Self) -> Self {
        if other.cmp(&self) == Ordering::Less { other } else { self }
    }

    fn clamp(self, low: Self, high: Self) -> Self {
        if self.cmp(&low) == Ordering::Less {
            low
        } else if self.cmp(&high) == Ordering::Greater {
            high
        } else {
            self
        }
    }
}

pub fn max_by<T, F: FnOnce(&T, &T) -> Ordering>(a: T, b: T, compare: F) -> T {
    if compare(&b, &a) == Ordering::Less { a } else { b }
}

pub fn min_by<T, F: FnOnce(&T, &T) -> Ordering>(a: T, b: T, compare: F) -> T {
    if compare(&b, &a) == Ordering::Less { b } else { a }
}

pub fn max_by_key<T, K: Ord, F: FnMut(&T) -> K>(a: T, b: T, key: F) -> T {
    let mut key = key;
    if key(&b) < key(&a) { a } else { b }
}

pub fn min_by_key<T, K: Ord, F: FnMut(&T) -> K>(a: T, b: T, key: F) -> T {
    let mut key = key;
    if key(&b) < key(&a) { b } else { a }
}

pub fn min<T: PartialOrd>(a: T, b: T) -> T {
    if b < a { b } else { a }
}

pub fn max<T: PartialOrd>(a: T, b: T) -> T {
    if b > a { b } else { a }
}

fn ordering(less: bool, equal: bool) -> Ordering {
    if less {
        Ordering::Less
    } else if equal {
        Ordering::Equal
    } else {
        Ordering::Greater
    }
}

// The primitive types. Comparing them is built into the language; these
// impls make them usable where a trait is asked for by name.

impl PartialEq for i8 { fn eq(&self, other: &i8) -> bool { *self == *other } }
impl Eq for i8 {}
impl PartialOrd for i8 {
    fn partial_cmp(&self, other: &i8) -> Option<Ordering> { Option::Some(ordering(*self < *other, *self == *other)) }
}
impl Ord for i8 { fn cmp(&self, other: &i8) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for i16 { fn eq(&self, other: &i16) -> bool { *self == *other } }
impl Eq for i16 {}
impl PartialOrd for i16 {
    fn partial_cmp(&self, other: &i16) -> Option<Ordering> { Option::Some(ordering(*self < *other, *self == *other)) }
}
impl Ord for i16 { fn cmp(&self, other: &i16) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for i32 { fn eq(&self, other: &i32) -> bool { *self == *other } }
impl Eq for i32 {}
impl PartialOrd for i32 {
    fn partial_cmp(&self, other: &i32) -> Option<Ordering> { Option::Some(ordering(*self < *other, *self == *other)) }
}
impl Ord for i32 { fn cmp(&self, other: &i32) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for i64 { fn eq(&self, other: &i64) -> bool { *self == *other } }
impl Eq for i64 {}
impl PartialOrd for i64 {
    fn partial_cmp(&self, other: &i64) -> Option<Ordering> { Option::Some(ordering(*self < *other, *self == *other)) }
}
impl Ord for i64 { fn cmp(&self, other: &i64) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for isize { fn eq(&self, other: &isize) -> bool { *self == *other } }
impl Eq for isize {}
impl PartialOrd for isize {
    fn partial_cmp(&self, other: &isize) -> Option<Ordering> { Option::Some(ordering(*self < *other, *self == *other)) }
}
impl Ord for isize { fn cmp(&self, other: &isize) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for u8 { fn eq(&self, other: &u8) -> bool { *self == *other } }
impl Eq for u8 {}
impl PartialOrd for u8 {
    fn partial_cmp(&self, other: &u8) -> Option<Ordering> { Option::Some(ordering(*self < *other, *self == *other)) }
}
impl Ord for u8 { fn cmp(&self, other: &u8) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for u16 { fn eq(&self, other: &u16) -> bool { *self == *other } }
impl Eq for u16 {}
impl PartialOrd for u16 {
    fn partial_cmp(&self, other: &u16) -> Option<Ordering> { Option::Some(ordering(*self < *other, *self == *other)) }
}
impl Ord for u16 { fn cmp(&self, other: &u16) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for u32 { fn eq(&self, other: &u32) -> bool { *self == *other } }
impl Eq for u32 {}
impl PartialOrd for u32 {
    fn partial_cmp(&self, other: &u32) -> Option<Ordering> { Option::Some(ordering(*self < *other, *self == *other)) }
}
impl Ord for u32 { fn cmp(&self, other: &u32) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for u64 { fn eq(&self, other: &u64) -> bool { *self == *other } }
impl Eq for u64 {}
impl PartialOrd for u64 {
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> { Option::Some(ordering(*self < *other, *self == *other)) }
}
impl Ord for u64 { fn cmp(&self, other: &u64) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for i128 { fn eq(&self, other: &i128) -> bool { *self == *other } }
impl Eq for i128 {}
impl PartialOrd for i128 {
    fn partial_cmp(&self, other: &i128) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for i128 { fn cmp(&self, other: &i128) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for u128 { fn eq(&self, other: &u128) -> bool { *self == *other } }
impl Eq for u128 {}
impl PartialOrd for u128 {
    fn partial_cmp(&self, other: &u128) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for u128 { fn cmp(&self, other: &u128) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for usize { fn eq(&self, other: &usize) -> bool { *self == *other } }
impl Eq for usize {}
impl PartialOrd for usize {
    fn partial_cmp(&self, other: &usize) -> Option<Ordering> { Option::Some(ordering(*self < *other, *self == *other)) }
}
impl Ord for usize { fn cmp(&self, other: &usize) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for char { fn eq(&self, other: &char) -> bool { *self == *other } }
impl Eq for char {}
impl PartialOrd for char {
    fn partial_cmp(&self, other: &char) -> Option<Ordering> { Option::Some(ordering(*self < *other, *self == *other)) }
}
impl Ord for char { fn cmp(&self, other: &char) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for bool { fn eq(&self, other: &bool) -> bool { *self == *other } }
impl Eq for bool {}
impl PartialOrd for bool {
    fn partial_cmp(&self, other: &bool) -> Option<Ordering> { Option::Some(ordering(*self < *other, *self == *other)) }
}
impl Ord for bool { fn cmp(&self, other: &bool) -> Ordering { ordering(*self < *other, *self == *other) } }

impl PartialEq for f32 { fn eq(&self, other: &f32) -> bool { *self == *other } }
impl PartialOrd for f32 {
    fn partial_cmp(&self, other: &f32) -> Option<Ordering> {
        if *self < *other {
            Option::Some(Ordering::Less)
        } else if *self == *other {
            Option::Some(Ordering::Equal)
        } else if *self > *other {
            Option::Some(Ordering::Greater)
        } else {
            Option::None
        }
    }
}

impl PartialEq for f64 { fn eq(&self, other: &f64) -> bool { *self == *other } }
impl PartialOrd for f64 {
    fn partial_cmp(&self, other: &f64) -> Option<Ordering> {
        if *self < *other {
            Option::Some(Ordering::Less)
        } else if *self == *other {
            Option::Some(Ordering::Equal)
        } else if *self > *other {
            Option::Some(Ordering::Greater)
        } else {
            Option::None
        }
    }
}

impl<T: PartialEq> PartialEq for &T {
    fn eq(&self, other: &&T) -> bool { **self == **other }
}

impl<T: PartialOrd> PartialOrd for &T {
    fn partial_cmp(&self, other: &&T) -> Option<Ordering> { PartialOrd::partial_cmp(*self, *other) }
}

impl<T: Eq> Eq for &T {}
impl<T: Eq> Eq for &mut T {}

impl<T: PartialEq> PartialEq for &mut T {
    fn eq(&self, other: &&mut T) -> bool { **self == **other }
}

impl<T: Ord> Ord for &T {
    fn cmp(&self, other: &&T) -> Ordering { Ord::cmp(*self, *other) }
}

// Tuples compare element by element.

impl PartialEq for () { fn eq(&self, other: &()) -> bool { true } }
impl Eq for () {}
impl PartialOrd for () { fn partial_cmp(&self, other: &()) -> Option<Ordering> { Some(Ordering::Equal) } }
impl Ord for () { fn cmp(&self, other: &()) -> Ordering { Ordering::Equal } }

macro_rules! tuple_comparisons {
    ($(($($name:ident $index:tt),+))*) => {
        $(
            impl<$($name: PartialEq),+> PartialEq for ($($name,)+) {
                fn eq(&self, other: &($($name,)+)) -> bool {
                    $(self.$index == other.$index)&&+
                }
            }

            impl<$($name: Eq),+> Eq for ($($name,)+) {}

            impl<$($name: PartialOrd),+> PartialOrd for ($($name,)+) {
                fn partial_cmp(&self, other: &($($name,)+)) -> Option<Ordering> {
                    $(
                        match self.$index.partial_cmp(&other.$index) {
                            Some(Ordering::Equal) => {}
                            decided => return decided,
                        }
                    )+
                    Some(Ordering::Equal)
                }
            }

            impl<$($name: Ord),+> Ord for ($($name,)+) {
                fn cmp(&self, other: &($($name,)+)) -> Ordering {
                    $(
                        match self.$index.cmp(&other.$index) {
                            Ordering::Equal => {}
                            decided => return decided,
                        }
                    )+
                    Ordering::Equal
                }
            }
        )*
    };
}

tuple_comparisons! {
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
