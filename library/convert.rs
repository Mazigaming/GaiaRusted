//! Conversions between types.

pub trait From<T> {
    fn from(value: T) -> Self;
}

pub trait Into<T> {
    fn into(self) -> T;
}

/// `value.into()` is `From::from(value)` seen from the other side.
impl<T, U: From<T>> Into<U> for T {
    fn into(self) -> U {
        U::from(self)
    }
}

/// Every type converts into itself.
impl<T> From<T> for T {
    fn from(value: T) -> T {
        value
    }
}

/// A conversion that can fail.
pub trait TryFrom<T>: Sized {
    type Error;
    fn try_from(value: T) -> Result<Self, Self::Error>;
}

pub trait TryInto<T> {
    type Error;
    fn try_into(self) -> Result<T, Self::Error>;
}

impl<T, U: TryFrom<T>> TryInto<U> for T {
    type Error = <U as TryFrom<T>>::Error;
    fn try_into(self) -> Result<U, <U as TryFrom<T>>::Error> {
        U::try_from(self)
    }
}

/// Conversions that never lose information.
macro_rules! lossless {
    ($from:ident => $($to:ident)*) => {
        $(impl From<$from> for $to { fn from(value: $from) -> $to { value as $to } })*
    };
}

lossless!(u8 => u16 u32 u64 usize i16 i32 i64 isize f32 f64);
lossless!(u16 => u32 u64 usize i32 i64 f32 f64);
lossless!(u32 => u64 i64 f64);
lossless!(u64 => i128 u128);
lossless!(u8 => i128 u128);
lossless!(u16 => i128 u128);
lossless!(u32 => i128 u128);
lossless!(i8 => i16 i32 i64 isize f32 f64);
lossless!(i16 => i32 i64 isize f32 f64);
lossless!(i32 => i64 f64);
lossless!(i8 => i128);
lossless!(i16 => i128);
lossless!(i32 => i128);
lossless!(i64 => i128);
lossless!(f32 => f64);
lossless!(bool => u8 u16 u32 u64 usize i8 i16 i32 i64 isize i128 u128);

impl From<char> for u32 { fn from(value: char) -> u32 { value as u32 } }
impl From<char> for u64 { fn from(value: char) -> u64 { value as u64 } }
impl From<u8> for char { fn from(value: u8) -> char { value as char } }

/// Integer conversions that check the value fits: it does if converting
/// it and back gives it back, with the same sign.
macro_rules! checked_int_conversions {
    ($from:ident => $($to:ident)*) => {
        $(
            impl TryFrom<$from> for $to {
                type Error = std::num::TryFromIntError;
                fn try_from(value: $from) -> Result<$to, std::num::TryFromIntError> {
                    let converted = value as $to;
                    if converted as $from == value && (converted < (0 as $to)) == (value < (0 as $from)) {
                        Ok(converted)
                    } else {
                        Err(std::num::TryFromIntError(()))
                    }
                }
            }
        )*
    };
}

checked_int_conversions!(i8 => i16 i32 i64 isize u8 u16 u32 u64 usize);
checked_int_conversions!(i16 => i8 i32 i64 isize u8 u16 u32 u64 usize);
checked_int_conversions!(i32 => i8 i16 i64 isize u8 u16 u32 u64 usize);
checked_int_conversions!(i64 => i8 i16 i32 isize u8 u16 u32 u64 usize);
checked_int_conversions!(isize => i8 i16 i32 i64 u8 u16 u32 u64 usize);
checked_int_conversions!(u8 => i8 i16 i32 i64 isize u16 u32 u64 usize);
checked_int_conversions!(u16 => i8 i16 i32 i64 isize u8 u32 u64 usize);
checked_int_conversions!(u32 => i8 i16 i32 i64 isize u8 u16 u64 usize);
checked_int_conversions!(u64 => i8 i16 i32 i64 isize u8 u16 u32 usize);
checked_int_conversions!(usize => i8 i16 i32 i64 isize u8 u16 u32 u64 i128 u128);
checked_int_conversions!(isize => i128 u128);
checked_int_conversions!(i8 => u128);
checked_int_conversions!(i16 => u128);
checked_int_conversions!(i32 => u128);
checked_int_conversions!(i64 => u128);
checked_int_conversions!(i128 => i8 i16 i32 i64 isize u8 u16 u32 u64 usize u128);
checked_int_conversions!(u128 => i8 i16 i32 i64 isize u8 u16 u32 u64 usize i128);

/// A cheap borrow of one type as another: `&String` as `&str`, `&Vec<T>`
/// as `&[T]`.
pub trait AsRef<T: ?Sized> {
    fn as_ref(&self) -> &T;
}

pub trait AsMut<T: ?Sized> {
    fn as_mut(&mut self) -> &mut T;
}

impl<T: ?Sized, U: ?Sized> AsRef<U> for &T
where
    T: AsRef<U>,
{
    fn as_ref(&self) -> &U {
        (**self).as_ref()
    }
}

impl AsRef<str> for str {
    fn as_ref(&self) -> &str {
        self
    }
}

impl AsRef<[u8]> for str {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsRef<str> for String {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for String {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<T> AsRef<[T]> for [T] {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T> AsRef<[T]> for Vec<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> AsMut<[T]> for [T] {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T> AsMut<[T]> for Vec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: ?Sized> AsRef<T> for Box<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

/// The error of a conversion that cannot fail: there is no value of it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Infallible {}

impl std::fmt::Debug for Infallible {
    fn fmt(&self, _: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {}
    }
}

impl std::fmt::Display for Infallible {
    fn fmt(&self, _: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {}
    }
}
