//! Operators as traits, and the range types.

pub trait Add<Rhs = Self> {
    type Output;
    fn add(self, rhs: Rhs) -> Self::Output;
}

pub trait Sub<Rhs = Self> {
    type Output;
    fn sub(self, rhs: Rhs) -> Self::Output;
}

pub trait Mul<Rhs = Self> {
    type Output;
    fn mul(self, rhs: Rhs) -> Self::Output;
}

pub trait Div<Rhs = Self> {
    type Output;
    fn div(self, rhs: Rhs) -> Self::Output;
}

pub trait Rem<Rhs = Self> {
    type Output;
    fn rem(self, rhs: Rhs) -> Self::Output;
}

pub trait Neg {
    type Output;
    fn neg(self) -> Self::Output;
}

pub trait Not {
    type Output;
    fn not(self) -> Self::Output;
}

pub trait BitAnd<Rhs = Self> {
    type Output;
    fn bitand(self, rhs: Rhs) -> Self::Output;
}

pub trait BitOr<Rhs = Self> {
    type Output;
    fn bitor(self, rhs: Rhs) -> Self::Output;
}

pub trait BitXor<Rhs = Self> {
    type Output;
    fn bitxor(self, rhs: Rhs) -> Self::Output;
}

pub trait Shl<Rhs = Self> {
    type Output;
    fn shl(self, rhs: Rhs) -> Self::Output;
}

pub trait Shr<Rhs = Self> {
    type Output;
    fn shr(self, rhs: Rhs) -> Self::Output;
}

pub trait AddAssign<Rhs = Self> {
    fn add_assign(&mut self, rhs: Rhs);
}

pub trait SubAssign<Rhs = Self> {
    fn sub_assign(&mut self, rhs: Rhs);
}

pub trait MulAssign<Rhs = Self> {
    fn mul_assign(&mut self, rhs: Rhs);
}

pub trait DivAssign<Rhs = Self> {
    fn div_assign(&mut self, rhs: Rhs);
}

pub trait RemAssign<Rhs = Self> {
    fn rem_assign(&mut self, rhs: Rhs);
}

/// `*value` for smart pointers.
/// Pointer types whose pointee can be made unsized through them, as
/// `Box<T>` to `Box<dyn Trait>`, or `Rc<[T; N]>` to `Rc<[T]>`.
pub trait CoerceUnsized<Target: ?Sized> {}

pub trait Deref {
    type Target;
    fn deref(&self) -> &Self::Target;
}

pub trait DerefMut {
    fn deref_mut(&mut self) -> &mut Self::Target;
}

/// Code to run when a value goes out of scope.
pub trait Drop {
    fn drop(&mut self);
}

/// `container[index]`
pub trait Index<Idx> {
    type Output;
    fn index(&self, index: Idx) -> &Self::Output;
}

pub trait IndexMut<Idx> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output;
}

// The closure traits. A bound such as `F: Fn(i64) -> i64` is understood by
// the compiler directly; these declarations give the names something to
// resolve to.
pub trait FnOnce {}
pub trait FnMut {}
pub trait Fn {}

/// `start..end`
pub struct Range<Idx> {
    pub start: Idx,
    pub end: Idx,
}

/// `start..=end`
pub struct RangeInclusive<Idx> {
    pub start: Idx,
    pub end: Idx,
}

/// `start..`
pub struct RangeFrom<Idx> {
    pub start: Idx,
}

/// `..end`
pub struct RangeTo<Idx> {
    pub end: Idx,
}

/// `..=end`
pub struct RangeToInclusive<Idx> {
    pub end: Idx,
}

/// `..`
pub struct RangeFull;

impl<Idx> RangeInclusive<Idx> {
    pub fn new(start: Idx, end: Idx) -> RangeInclusive<Idx> {
        RangeInclusive { start, end }
    }

    pub fn start(&self) -> &Idx {
        &self.start
    }

    pub fn end(&self) -> &Idx {
        &self.end
    }

    pub fn into_inner(self) -> (Idx, Idx) {
        (self.start, self.end)
    }
}

impl<Idx: Clone> Clone for RangeInclusive<Idx> {
    fn clone(&self) -> RangeInclusive<Idx> {
        RangeInclusive { start: self.start.clone(), end: self.end.clone() }
    }
}

impl<Idx: Clone> Clone for RangeFrom<Idx> {
    fn clone(&self) -> RangeFrom<Idx> {
        RangeFrom { start: self.start.clone() }
    }
}

impl<Idx: PartialEq> PartialEq for Range<Idx> {
    fn eq(&self, other: &Range<Idx>) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl<Idx: PartialEq> PartialEq for RangeInclusive<Idx> {
    fn eq(&self, other: &RangeInclusive<Idx>) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl<Idx: std::fmt::Debug> std::fmt::Debug for Range<Idx> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.start.fmt(f);
        f.write_str("..");
        self.end.fmt(f)
    }
}

impl<Idx: std::fmt::Debug> std::fmt::Debug for RangeInclusive<Idx> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.start.fmt(f);
        f.write_str("..=");
        self.end.fmt(f)
    }
}

impl<Idx: std::fmt::Debug> std::fmt::Debug for RangeFrom<Idx> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.start.fmt(f);
        f.write_str("..")
    }
}

impl<Idx: std::fmt::Debug> std::fmt::Debug for RangeTo<Idx> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("..");
        self.end.fmt(f)
    }
}

impl std::fmt::Debug for RangeFull {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("..")
    }
}

impl<Idx: Clone> Clone for Range<Idx> {
    fn clone(&self) -> Range<Idx> {
        Range { start: self.start.clone(), end: self.end.clone() }
    }
}

impl<Idx: PartialOrd> Range<Idx> {
    pub fn contains(&self, item: &Idx) -> bool {
        *item >= self.start && *item < self.end
    }

    pub fn is_empty(&self) -> bool {
        !(self.start < self.end)
    }
}

impl<Idx: PartialOrd> RangeInclusive<Idx> {
    pub fn contains(&self, item: &Idx) -> bool {
        *item >= self.start && *item <= self.end
    }

    pub fn is_empty(&self) -> bool {
        !(self.start <= self.end)
    }
}

impl<Idx: PartialOrd> RangeFrom<Idx> {
    pub fn contains(&self, item: &Idx) -> bool {
        *item >= self.start
    }
}

impl<Idx: PartialOrd> RangeTo<Idx> {
    pub fn contains(&self, item: &Idx) -> bool {
        *item < self.end
    }
}

/// One end of a range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bound<T> {
    Included(T),
    Excluded(T),
    Unbounded,
}

/// The ranges of every kind: what `drain(..)` and `get(1..=3)` accept.
pub trait RangeBounds<T> {
    fn start_bound(&self) -> Bound<&T>;
    fn end_bound(&self) -> Bound<&T>;
}

impl<T> RangeBounds<T> for Range<T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Included(&self.start) }
    fn end_bound(&self) -> Bound<&T> { Bound::Excluded(&self.end) }
}

impl<T> RangeBounds<T> for RangeInclusive<T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Included(&self.start) }
    fn end_bound(&self) -> Bound<&T> { Bound::Included(&self.end) }
}

impl<T> RangeBounds<T> for RangeFrom<T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Included(&self.start) }
    fn end_bound(&self) -> Bound<&T> { Bound::Unbounded }
}

impl<T> RangeBounds<T> for RangeTo<T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Unbounded }
    fn end_bound(&self) -> Bound<&T> { Bound::Excluded(&self.end) }
}

impl<T> RangeBounds<T> for RangeToInclusive<T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Unbounded }
    fn end_bound(&self) -> Bound<&T> { Bound::Included(&self.end) }
}

impl<T> RangeBounds<T> for RangeFull {
    fn start_bound(&self) -> Bound<&T> { Bound::Unbounded }
    fn end_bound(&self) -> Bound<&T> { Bound::Unbounded }
}

/// The start and end, as positions, of a range over something `len` long.
pub fn range_positions<R: RangeBounds<usize>>(range: &R, len: usize) -> (usize, usize) {
    let start = match range.start_bound() {
        Bound::Included(start) => *start,
        Bound::Excluded(start) => *start + 1,
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(end) => *end + 1,
        Bound::Excluded(end) => *end,
        Bound::Unbounded => len,
    };
    (start, end)
}
