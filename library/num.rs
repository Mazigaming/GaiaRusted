//! Methods and constants of the number types.
//!
//! Each family is defined once, by a macro instantiated for every type in
//! it, the way Rust's own library does it.

/// Why text could not be read as an integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntErrorKind {
    Empty,
    InvalidDigit,
    PosOverflow,
    NegOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseIntError {
    kind: IntErrorKind,
}

impl ParseIntError {
    pub fn kind(&self) -> &IntErrorKind {
        &self.kind
    }
}

impl std::fmt::Display for ParseIntError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.pad(match self.kind {
            IntErrorKind::Empty => "cannot parse integer from empty string",
            IntErrorKind::InvalidDigit => "invalid digit found in string",
            IntErrorKind::PosOverflow => "number too large to fit in target type",
            IntErrorKind::NegOverflow => "number too small to fit in target type",
        })
    }
}

/// A conversion between integer types lost the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TryFromIntError(());

impl std::fmt::Display for TryFromIntError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.pad("out of range integral type conversion attempted")
    }
}

/// The value of an ASCII digit in `radix`, if it is one.
fn digit_value(byte: u8, radix: u32) -> Option<u32> {
    let value = if byte >= b'0' && byte <= b'9' {
        (byte - b'0') as u32
    } else if byte >= b'a' && byte <= b'z' {
        (byte - b'a') as u32 + 10
    } else if byte >= b'A' && byte <= b'Z' {
        (byte - b'A') as u32 + 10
    } else {
        return None;
    };
    if value < radix { Some(value) } else { None }
}

/// What every integer type has.
macro_rules! int_impl {
    ($t:ident, $unsigned:ident, $bits:expr, $min:expr, $max:expr) => {
        impl $t {
            pub const MIN: $t = $min;
            pub const MAX: $t = $max;
            pub const BITS: u32 = $bits;

            pub fn min_value() -> $t { Self::MIN }
            pub fn max_value() -> $t { Self::MAX }

            /// The bytes of the value, least significant first.
            pub fn to_le_bytes(self) -> [u8; $bits / 8] {
                let mut bytes = [0u8; $bits / 8];
                let mut index = 0;
                while index < $bits / 8 {
                    bytes[index] = (self as $unsigned >> (index * 8)) as u8;
                    index += 1;
                }
                bytes
            }

            pub fn to_be_bytes(self) -> [u8; $bits / 8] {
                let mut bytes = self.to_le_bytes();
                bytes.reverse();
                bytes
            }

            pub fn to_ne_bytes(self) -> [u8; $bits / 8] {
                self.to_le_bytes()
            }

            /// The value of bytes stored least significant first.
            pub fn from_le_bytes(bytes: [u8; $bits / 8]) -> $t {
                let mut value: $unsigned = 0;
                let mut index = $bits / 8;
                while index > 0 {
                    index -= 1;
                    value = (value << 8) | bytes[index] as $unsigned;
                }
                value as $t
            }

            pub fn from_be_bytes(bytes: [u8; $bits / 8]) -> $t {
                let mut reversed = bytes;
                reversed.reverse();
                <$t>::from_le_bytes(reversed)
            }

            pub fn from_ne_bytes(bytes: [u8; $bits / 8]) -> $t {
                <$t>::from_le_bytes(bytes)
            }

            // Arithmetic on integers wraps around.
            pub fn wrapping_add(self, other: $t) -> $t { self + other }
            pub fn wrapping_sub(self, other: $t) -> $t { self - other }
            pub fn wrapping_mul(self, other: $t) -> $t { self * other }
            pub fn wrapping_neg(self) -> $t { (0 as $t).wrapping_sub(self) }
            pub fn wrapping_shl(self, by: u32) -> $t { self << (by % $bits) }
            pub fn wrapping_shr(self, by: u32) -> $t { self >> (by % $bits) }

            pub fn checked_div(self, other: $t) -> Option<$t> {
                if other == 0 || (self == Self::MIN && other == (0 as $t).wrapping_sub(1) && Self::MIN != 0) {
                    None
                } else {
                    Some(self / other)
                }
            }

            pub fn checked_rem(self, other: $t) -> Option<$t> {
                if other == 0 || (self == Self::MIN && other == (0 as $t).wrapping_sub(1) && Self::MIN != 0) {
                    None
                } else {
                    Some(self % other)
                }
            }

            pub fn checked_mul(self, other: $t) -> Option<$t> {
                if self == 0 || other == 0 {
                    return Some(0);
                }
                let minus_one = (0 as $t).wrapping_sub(1);
                // `MIN * -1` does not fit, and dividing to find out would trap.
                if Self::MIN != 0 && ((self == minus_one && other == Self::MIN) || (other == minus_one && self == Self::MIN)) {
                    return None;
                }
                let product = self.wrapping_mul(other);
                if product / other != self { None } else { Some(product) }
            }

            pub fn checked_pow(self, exponent: u32) -> Option<$t> {
                let mut result: $t = 1;
                let mut remaining = exponent;
                while remaining > 0 {
                    result = result.checked_mul(self)?;
                    remaining -= 1;
                }
                Some(result)
            }

            pub fn checked_shl(self, by: u32) -> Option<$t> { if by < $bits { Some(self << by) } else { None } }
            pub fn checked_shr(self, by: u32) -> Option<$t> { if by < $bits { Some(self >> by) } else { None } }

            pub fn overflowing_add(self, other: $t) -> ($t, bool) { (self.wrapping_add(other), self.checked_add(other).is_none()) }
            pub fn overflowing_sub(self, other: $t) -> ($t, bool) { (self.wrapping_sub(other), self.checked_sub(other).is_none()) }
            pub fn overflowing_mul(self, other: $t) -> ($t, bool) { (self.wrapping_mul(other), self.checked_mul(other).is_none()) }

            pub fn pow(self, exponent: u32) -> $t {
                let mut result: $t = 1;
                let mut base = self;
                let mut remaining = exponent;
                while remaining > 0 {
                    if remaining % 2 == 1 {
                        result = result.wrapping_mul(base);
                    }
                    base = base.wrapping_mul(base);
                    remaining = remaining / 2;
                }
                result
            }
            pub fn wrapping_pow(self, exponent: u32) -> $t { self.pow(exponent) }
            pub fn saturating_pow(self, exponent: u32) -> $t {
                match self.checked_pow(exponent) {
                    Some(value) => value,
                    None if self < 0 && exponent % 2 == 1 => Self::MIN,
                    None => Self::MAX,
                }
            }

            pub fn min(self, other: $t) -> $t { if other < self { other } else { self } }
            pub fn max(self, other: $t) -> $t { if other > self { other } else { self } }
            pub fn clamp(self, low: $t, high: $t) -> $t { if self < low { low } else if self > high { high } else { self } }

            pub fn count_ones(self) -> u32 {
                let mut bits = self as $unsigned;
                let mut count = 0;
                while bits != 0 {
                    count += (bits & 1) as u32;
                    bits = bits >> 1;
                }
                count
            }
            pub fn count_zeros(self) -> u32 { $bits - self.count_ones() }

            /// The integer square root, rounded down.
            pub fn isqrt(self) -> $t {
                if self < (0 as $t) {
                    panic!("argument of integer square root cannot be negative");
                }
                // Digit by digit, two bits at a time.
                let mut rest = self as $unsigned;
                let mut root: $unsigned = 0;
                let mut bit: $unsigned = 1 << ($bits - 2);
                while bit > rest {
                    bit >>= 2;
                }
                while bit != 0 {
                    if rest >= root + bit {
                        rest -= root + bit;
                        root = (root >> 1) + bit;
                    } else {
                        root >>= 1;
                    }
                    bit >>= 2;
                }
                root as $t
            }

            pub fn checked_ilog2(self) -> Option<u32> {
                if self <= (0 as $t) { None } else { Some($bits - 1 - self.leading_zeros()) }
            }

            pub fn checked_ilog(self, base: $t) -> Option<u32> {
                if self <= (0 as $t) || base < (2 as $t) {
                    return None;
                }
                let mut log = 0;
                let mut rest = self;
                while rest >= base {
                    rest /= base;
                    log += 1;
                }
                Some(log)
            }

            pub fn checked_ilog10(self) -> Option<u32> { self.checked_ilog(10 as $t) }

            pub fn ilog2(self) -> u32 {
                match self.checked_ilog2() {
                    Some(log) => log,
                    None => panic!("argument of integer logarithm must be positive"),
                }
            }

            pub fn ilog10(self) -> u32 {
                match self.checked_ilog10() {
                    Some(log) => log,
                    None => panic!("argument of integer logarithm must be positive"),
                }
            }

            pub fn ilog(self, base: $t) -> u32 {
                if base < (2 as $t) {
                    panic!("base of integer logarithm must be at least 2");
                }
                match self.checked_ilog(base) {
                    Some(log) => log,
                    None => panic!("argument of integer logarithm must be positive"),
                }
            }

            pub fn leading_zeros(self) -> u32 {
                let bits = self as $unsigned;
                let mut count = 0;
                while count < $bits && bits >> ($bits - 1 - count) & 1 == 0 {
                    count += 1;
                }
                count
            }
            pub fn trailing_zeros(self) -> u32 {
                let bits = self as $unsigned;
                let mut count = 0;
                while count < $bits && bits >> count & 1 == 0 {
                    count += 1;
                }
                count
            }
            pub fn leading_ones(self) -> u32 { (!self).leading_zeros() }
            pub fn trailing_ones(self) -> u32 { (!self).trailing_zeros() }

            pub fn rotate_left(self, by: u32) -> $t {
                let by = by % $bits;
                if by == 0 {
                    return self;
                }
                let bits = self as $unsigned;
                ((bits << by) | (bits >> ($bits - by))) as $t
            }
            pub fn rotate_right(self, by: u32) -> $t { self.rotate_left(($bits - by % $bits) % $bits) }

            pub fn swap_bytes(self) -> $t {
                let mut bits = self as $unsigned;
                let mut swapped: $unsigned = 0;
                let mut byte = 0;
                while byte < $bits / 8 {
                    swapped = (swapped << 8) | (bits & 0xff);
                    bits = bits >> 8;
                    byte += 1;
                }
                swapped as $t
            }

            pub fn reverse_bits(self) -> $t {
                let mut bits = self as $unsigned;
                let mut reversed: $unsigned = 0;
                let mut bit = 0;
                while bit < $bits {
                    reversed = (reversed << 1) | (bits & 1);
                    bits = bits >> 1;
                    bit += 1;
                }
                reversed as $t
            }

            pub fn from_str_radix(text: &str, radix: u32) -> Result<$t, ParseIntError> {
                let bytes = text.as_bytes();
                if bytes.is_empty() {
                    return Err(ParseIntError { kind: IntErrorKind::Empty });
                }
                let negative = bytes[0] == b'-' && Self::MIN != 0;
                let start = if negative || bytes[0] == b'+' { 1 } else { 0 };
                if start == bytes.len() {
                    return Err(ParseIntError { kind: IntErrorKind::InvalidDigit });
                }
                // Negative numbers are accumulated downwards, which reaches `MIN`.
                let mut value: $t = 0;
                let mut index = start;
                while index < bytes.len() {
                    let Some(digit) = digit_value(bytes[index], radix) else {
                        return Err(ParseIntError { kind: IntErrorKind::InvalidDigit });
                    };
                    let overflow = if negative { IntErrorKind::NegOverflow } else { IntErrorKind::PosOverflow };
                    let Some(scaled) = value.checked_mul(radix as $t) else {
                        return Err(ParseIntError { kind: overflow });
                    };
                    let next = if negative { scaled.checked_sub(digit as $t) } else { scaled.checked_add(digit as $t) };
                    let Some(next) = next else { return Err(ParseIntError { kind: overflow }) };
                    value = next;
                    index += 1;
                }
                Ok(value)
            }
        }

        impl std::str::FromStr for $t {
            type Err = ParseIntError;
            fn from_str(text: &str) -> Result<$t, ParseIntError> {
                $t::from_str_radix(text, 10)
            }
        }
    };
}

/// What the signed integer types have.
macro_rules! signed_impl {
    ($t:ident, $unsigned:ident) => {
        impl $t {
            pub fn checked_add(self, other: $t) -> Option<$t> {
                let sum = self.wrapping_add(other);
                let overflowed = if other >= 0 { sum < self } else { sum > self };
                if overflowed { None } else { Some(sum) }
            }

            pub fn checked_sub(self, other: $t) -> Option<$t> {
                let difference = self.wrapping_sub(other);
                let overflowed = if other >= 0 { difference > self } else { difference < self };
                if overflowed { None } else { Some(difference) }
            }

            pub fn checked_neg(self) -> Option<$t> { if self == Self::MIN { None } else { Some(-self) } }
            pub fn checked_abs(self) -> Option<$t> { if self == Self::MIN { None } else { Some(self.abs()) } }

            pub fn saturating_add(self, other: $t) -> $t {
                match self.checked_add(other) { Some(sum) => sum, None if other > 0 => Self::MAX, None => Self::MIN }
            }
            pub fn saturating_sub(self, other: $t) -> $t {
                match self.checked_sub(other) { Some(difference) => difference, None if other > 0 => Self::MIN, None => Self::MAX }
            }
            pub fn saturating_mul(self, other: $t) -> $t {
                match self.checked_mul(other) {
                    Some(product) => product,
                    None if (self < 0) != (other < 0) => Self::MIN,
                    None => Self::MAX,
                }
            }

            pub fn abs(self) -> $t { if self < 0 { self.wrapping_neg() } else { self } }
            pub fn wrapping_abs(self) -> $t { self.abs() }
            pub fn unsigned_abs(self) -> $unsigned { self.abs() as $unsigned }
            pub fn abs_diff(self, other: $t) -> $unsigned {
                if self > other { (self as $unsigned).wrapping_sub(other as $unsigned) } else { (other as $unsigned).wrapping_sub(self as $unsigned) }
            }
            pub fn signum(self) -> $t { if self < 0 { -1 } else if self > 0 { 1 } else { 0 } }
            pub fn is_negative(self) -> bool { self < 0 }
            pub fn is_positive(self) -> bool { self > 0 }

            /// The quotient rounded so that the remainder is never negative.
            pub fn div_euclid(self, other: $t) -> $t {
                let quotient = self / other;
                if self % other < 0 {
                    if other > 0 { quotient - 1 } else { quotient + 1 }
                } else {
                    quotient
                }
            }
            pub fn rem_euclid(self, other: $t) -> $t {
                let remainder = self % other;
                if remainder < 0 { if other < 0 { remainder - other } else { remainder + other } } else { remainder }
            }
        }
    };
}

/// What the unsigned integer types have.
macro_rules! unsigned_impl {
    ($t:ident) => {
        impl $t {
            pub fn checked_add(self, other: $t) -> Option<$t> {
                let sum = self.wrapping_add(other);
                if sum < self { None } else { Some(sum) }
            }
            pub fn checked_sub(self, other: $t) -> Option<$t> {
                if other > self { None } else { Some(self - other) }
            }
            pub fn checked_neg(self) -> Option<$t> { if self == 0 { Some(0) } else { None } }

            pub fn saturating_add(self, other: $t) -> $t { match self.checked_add(other) { Some(sum) => sum, None => Self::MAX } }
            pub fn saturating_sub(self, other: $t) -> $t { if other > self { 0 } else { self - other } }
            pub fn saturating_mul(self, other: $t) -> $t { match self.checked_mul(other) { Some(product) => product, None => Self::MAX } }

            pub fn abs_diff(self, other: $t) -> $t { if self > other { self - other } else { other - self } }
            pub fn div_euclid(self, other: $t) -> $t { self / other }
            pub fn rem_euclid(self, other: $t) -> $t { self % other }

            pub fn is_power_of_two(self) -> bool { self != 0 && self & (self - 1) == 0 }
            pub fn next_power_of_two(self) -> $t {
                let mut power: $t = 1;
                while power < self {
                    power = power << 1;
                }
                power
            }
            pub fn checked_next_power_of_two(self) -> Option<$t> {
                let mut power: $t = 1;
                while power < self {
                    power = power.checked_mul(2)?;
                }
                Some(power)
            }
        }
    };
}

int_impl!(i8, u8, 8, -128, 127);
int_impl!(i16, u16, 16, -32768, 32767);
int_impl!(i32, u32, 32, -2147483648, 2147483647);
int_impl!(i64, u64, 64, -9223372036854775808, 9223372036854775807);
int_impl!(isize, usize, 64, -9223372036854775808, 9223372036854775807);
int_impl!(u8, u8, 8, 0, 255);
int_impl!(u16, u16, 16, 0, 65535);
int_impl!(u32, u32, 32, 0, 4294967295);
int_impl!(u64, u64, 64, 0, 18446744073709551615);
int_impl!(usize, usize, 64, 0, 18446744073709551615);
int_impl!(i128, u128, 128, -170141183460469231731687303715884105728, 170141183460469231731687303715884105727);
int_impl!(u128, u128, 128, 0, 340282366920938463463374607431768211455);

signed_impl!(i8, u8);
signed_impl!(i16, u16);
signed_impl!(i32, u32);
signed_impl!(i64, u64);
signed_impl!(isize, usize);
unsigned_impl!(u8);
unsigned_impl!(u16);
unsigned_impl!(u32);
unsigned_impl!(u64);
unsigned_impl!(usize);
signed_impl!(i128, u128);
unsigned_impl!(u128);

/// What the floating-point types have. Elementary functions come from the
/// C library's `libm`, in its single- and double-precision versions.
macro_rules! float_impl {
    ($t:ident, $bits:ident, $epsilon:expr, $max:expr, $min_positive:expr,
     $sin:ident, $cos:ident, $tan:ident, $asin:ident, $acos:ident, $atan:ident, $atan2:ident,
     $sinh:ident, $cosh:ident, $tanh:ident, $exp:ident, $exp2:ident, $log:ident, $log2:ident, $log10:ident,
     $pow:ident, $hypot:ident, $floor:ident, $ceil:ident, $round:ident, $trunc:ident, $fma:ident,
     $expm1:ident, $log1p:ident, $uint:ident, $sint:ident) => {
        impl $t {
            pub const NAN: $t = 0.0 / 0.0;
            pub const INFINITY: $t = 1.0 / 0.0;
            pub const NEG_INFINITY: $t = -1.0 / 0.0;
            pub const EPSILON: $t = $epsilon;
            pub const MAX: $t = $max;
            pub const MIN: $t = -$max;
            pub const MIN_POSITIVE: $t = $min_positive;

            pub fn sqrt(self) -> $t { std::intrinsics::sqrt(self as f64) as $t }
            pub fn abs(self) -> $t { if self < 0.0 || (self == 0.0 && self.is_sign_negative()) { -self } else { self } }
            pub fn signum(self) -> $t { if self.is_nan() { self } else if self.is_sign_negative() { -1.0 } else { 1.0 } }
            pub fn copysign(self, sign: $t) -> $t { if sign.is_sign_negative() == self.is_sign_negative() { self } else { -self } }

            pub fn min(self, other: $t) -> $t { if self.is_nan() || other < self { other } else { self } }
            pub fn max(self, other: $t) -> $t { if self.is_nan() || other > self { other } else { self } }
            pub fn clamp(self, low: $t, high: $t) -> $t { if self < low { low } else if self > high { high } else { self } }

            pub fn is_nan(self) -> bool { self != self }
            pub fn is_infinite(self) -> bool { self == Self::INFINITY || self == Self::NEG_INFINITY }
            pub fn is_finite(self) -> bool { !self.is_nan() && !self.is_infinite() }
            pub fn is_normal(self) -> bool { self.is_finite() && self.abs() >= Self::MIN_POSITIVE }
            /// `-0.0` is negative too; it is the one value that `1 / x` tells apart.
            pub fn is_sign_negative(self) -> bool { self < 0.0 || (self == 0.0 && 1.0 / self < 0.0) }
            pub fn is_sign_positive(self) -> bool { !self.is_nan() && !self.is_sign_negative() }

            pub fn floor(self) -> $t { unsafe { std::libc::$floor(self) } }
            pub fn ceil(self) -> $t { unsafe { std::libc::$ceil(self) } }
            pub fn round(self) -> $t { unsafe { std::libc::$round(self) } }
            pub fn trunc(self) -> $t { unsafe { std::libc::$trunc(self) } }
            pub fn fract(self) -> $t { self - self.trunc() }
            pub fn mul_add(self, a: $t, b: $t) -> $t { unsafe { std::libc::$fma(self, a, b) } }
            pub fn recip(self) -> $t { 1.0 / self }
            pub fn rem_euclid(self, other: $t) -> $t {
                let remainder = self % other;
                if remainder < 0.0 { remainder + other.abs() } else { remainder }
            }
            pub fn div_euclid(self, other: $t) -> $t {
                let quotient = (self / other).trunc();
                if self % other < 0.0 { if other > 0.0 { quotient - 1.0 } else { quotient + 1.0 } } else { quotient }
            }

            pub fn powi(self, exponent: i32) -> $t {
                let mut result: $t = 1.0;
                let mut base = self;
                let mut remaining = if exponent < 0 { (exponent as i64).abs() } else { exponent as i64 };
                while remaining > 0 {
                    if remaining % 2 == 1 {
                        result = result * base;
                    }
                    base = base * base;
                    remaining = remaining / 2;
                }
                if exponent < 0 { 1.0 / result } else { result }
            }
            pub fn powf(self, exponent: $t) -> $t { unsafe { std::libc::$pow(self, exponent) } }
            pub fn exp(self) -> $t { unsafe { std::libc::$exp(self) } }
            pub fn exp2(self) -> $t { unsafe { std::libc::$exp2(self) } }
            pub fn exp_m1(self) -> $t { unsafe { std::libc::$expm1(self) } }
            pub fn ln(self) -> $t { unsafe { std::libc::$log(self) } }
            pub fn ln_1p(self) -> $t { unsafe { std::libc::$log1p(self) } }
            pub fn log(self, base: $t) -> $t { self.ln() / base.ln() }
            pub fn log2(self) -> $t { unsafe { std::libc::$log2(self) } }
            pub fn log10(self) -> $t { unsafe { std::libc::$log10(self) } }
            pub fn hypot(self, other: $t) -> $t { unsafe { std::libc::$hypot(self, other) } }

            pub fn sin(self) -> $t { unsafe { std::libc::$sin(self) } }
            pub fn cos(self) -> $t { unsafe { std::libc::$cos(self) } }
            pub fn tan(self) -> $t { unsafe { std::libc::$tan(self) } }
            pub fn sin_cos(self) -> ($t, $t) { (self.sin(), self.cos()) }
            pub fn asin(self) -> $t { unsafe { std::libc::$asin(self) } }
            pub fn acos(self) -> $t { unsafe { std::libc::$acos(self) } }
            pub fn atan(self) -> $t { unsafe { std::libc::$atan(self) } }
            pub fn atan2(self, other: $t) -> $t { unsafe { std::libc::$atan2(self, other) } }
            pub fn sinh(self) -> $t { unsafe { std::libc::$sinh(self) } }
            pub fn cosh(self) -> $t { unsafe { std::libc::$cosh(self) } }
            pub fn tanh(self) -> $t { unsafe { std::libc::$tanh(self) } }

            pub fn to_degrees(self) -> $t { self * (180.0 / std::$bits::consts::PI) }
            pub fn to_radians(self) -> $t { self * (std::$bits::consts::PI / 180.0) }

            /// The bits of the IEEE 754 encoding.
            pub fn to_bits(self) -> $uint { unsafe { std::mem::transmute::<$t, $uint>(self) } }
            pub fn from_bits(bits: $uint) -> $t { unsafe { std::mem::transmute::<$uint, $t>(bits) } }
            pub fn to_le_bytes(self) -> [u8; std::mem::size_of::<$t>()] { self.to_bits().to_le_bytes() }
            pub fn to_be_bytes(self) -> [u8; std::mem::size_of::<$t>()] { self.to_bits().to_be_bytes() }
            pub fn to_ne_bytes(self) -> [u8; std::mem::size_of::<$t>()] { self.to_bits().to_ne_bytes() }
            pub fn from_le_bytes(bytes: [u8; std::mem::size_of::<$t>()]) -> $t { Self::from_bits($uint::from_le_bytes(bytes)) }
            pub fn from_be_bytes(bytes: [u8; std::mem::size_of::<$t>()]) -> $t { Self::from_bits($uint::from_be_bytes(bytes)) }
            pub fn from_ne_bytes(bytes: [u8; std::mem::size_of::<$t>()]) -> $t { Self::from_bits($uint::from_ne_bytes(bytes)) }

            /// IEEE 754's total order: `-NaN < -inf < ... < -0 < +0 < ... < inf < NaN`.
            pub fn total_cmp(&self, other: &$t) -> std::cmp::Ordering {
                // Flipping all but the sign bit of negative numbers makes
                // their bits compare as integers in the right order.
                let mut left = self.to_bits() as $sint;
                let mut right = other.to_bits() as $sint;
                left ^= (((left >> ($sint::BITS - 1)) as $uint) >> 1) as $sint;
                right ^= (((right >> ($sint::BITS - 1)) as $uint) >> 1) as $sint;
                left.cmp(&right)
            }
        }
    };
}

float_impl!(f64, f64, 2.220446049250313e-16, 1.7976931348623157e308, 2.2250738585072014e-308,
    sin, cos, tan, asin, acos, atan, atan2, sinh, cosh, tanh, exp, exp2, log, log2, log10,
    pow, hypot, floor, ceil, round, trunc, fma, expm1, log1p, u64, i64);
float_impl!(f32, f32, 1.1920929e-7, 3.40282347e38, 1.17549435e-38,
    sinf, cosf, tanf, asinf, acosf, atanf, atan2f, sinhf, coshf, tanhf, expf, exp2f, logf, log2f, log10f,
    powf, hypotf, floorf, ceilf, roundf, truncf, fmaf, expm1f, log1pf, u32, i32);

impl f64 {
    /// The cube root, correctly rounded, as Rust's own library computes it.
    /// The C library's is a few units in the last place off. One Newton
    /// step, with the error of the cube computed exactly (fused
    /// multiply-adds catch the rounding of each product), brings it within
    /// one unit; of that and its neighbours, the one whose cube comes
    /// closest to `self` is the answer.
    pub fn cbrt(self) -> f64 {
        let estimate = unsafe { std::libc::cbrt(self) };
        if !estimate.is_finite() || estimate == 0.0 {
            return estimate;
        }
        // `root³ - self`, computed without rounding error of its own.
        let excess = |root: f64| {
            let square = root * root;
            let square_error = root.mul_add(root, -square);
            let cube = square * root;
            let cube_error = square.mul_add(root, -cube);
            (cube - self) + cube_error + square_error * root
        };
        let refined = estimate - excess(estimate) / (3.0 * estimate * estimate);
        let mut best = refined;
        for candidate in [unsafe { std::libc::nextafter(refined, f64::NEG_INFINITY) }, unsafe { std::libc::nextafter(refined, f64::INFINITY) }] {
            if excess(candidate).abs() < excess(best).abs() {
                best = candidate;
            }
        }
        best
    }
}

impl f32 {
    pub fn cbrt(self) -> f32 {
        (self as f64).cbrt() as f32
    }
}

/// `T: Add<Output = T>` and the like hold for the number types too, so
/// that generic arithmetic works on them. (Where the types are known, the
/// compiler does the arithmetic itself.)
macro_rules! arithmetic_traits {
    ($($t:ident)*) => {
        $(
            impl std::ops::Add for $t { type Output = $t; fn add(self, other: $t) -> $t { self + other } }
            impl std::ops::Sub for $t { type Output = $t; fn sub(self, other: $t) -> $t { self - other } }
            impl std::ops::Mul for $t { type Output = $t; fn mul(self, other: $t) -> $t { self * other } }
            impl std::ops::Div for $t { type Output = $t; fn div(self, other: $t) -> $t { self / other } }
            impl std::ops::Rem for $t { type Output = $t; fn rem(self, other: $t) -> $t { self % other } }
            impl std::ops::Add<&$t> for $t { type Output = $t; fn add(self, other: &$t) -> $t { self + *other } }
            impl std::ops::Sub<&$t> for $t { type Output = $t; fn sub(self, other: &$t) -> $t { self - *other } }
            impl std::ops::Mul<&$t> for $t { type Output = $t; fn mul(self, other: &$t) -> $t { self * *other } }
            impl std::ops::AddAssign for $t { fn add_assign(&mut self, other: $t) { *self = *self + other; } }
            impl std::ops::SubAssign for $t { fn sub_assign(&mut self, other: $t) { *self = *self - other; } }
            impl std::ops::MulAssign for $t { fn mul_assign(&mut self, other: $t) { *self = *self * other; } }
            impl std::ops::DivAssign for $t { fn div_assign(&mut self, other: $t) { *self = *self / other; } }
            impl std::ops::RemAssign for $t { fn rem_assign(&mut self, other: $t) { *self = *self % other; } }
        )*
    };
}

macro_rules! signed_traits {
    ($($t:ident)*) => {
        $(
            impl std::ops::Neg for $t { type Output = $t; fn neg(self) -> $t { -self } }
        )*
    };
}

macro_rules! bit_traits {
    ($($t:ident)*) => {
        $(
            impl std::ops::Not for $t { type Output = $t; fn not(self) -> $t { !self } }
            impl std::ops::BitAnd for $t { type Output = $t; fn bitand(self, other: $t) -> $t { self & other } }
            impl std::ops::BitOr for $t { type Output = $t; fn bitor(self, other: $t) -> $t { self | other } }
            impl std::ops::BitXor for $t { type Output = $t; fn bitxor(self, other: $t) -> $t { self ^ other } }
            impl std::ops::Shl<u32> for $t { type Output = $t; fn shl(self, by: u32) -> $t { self << by } }
            impl std::ops::Shr<u32> for $t { type Output = $t; fn shr(self, by: u32) -> $t { self >> by } }
        )*
    };
}

arithmetic_traits!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64);
signed_traits!(i8 i16 i32 i64 i128 isize f32 f64);
bit_traits!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize);

/// Defines integer types whose values are never zero.
macro_rules! non_zero {
    ($($name:ident $t:ident)*) => {
        $(
            /// An integer known not to be zero.
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[rustc_nonnull_optimization_guaranteed]
            pub struct $name($t);

            impl $name {
                pub fn new(value: $t) -> Option<$name> {
                    if value == 0 { None } else { Some($name(value)) }
                }

                pub unsafe fn new_unchecked(value: $t) -> $name {
                    $name(value)
                }

                pub fn get(self) -> $t {
                    self.0
                }
            }

            impl std::fmt::Debug for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    std::fmt::Debug::fmt(&self.0, f)
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    std::fmt::Display::fmt(&self.0, f)
                }
            }

            impl From<$name> for $t {
                fn from(value: $name) -> $t {
                    value.0
                }
            }
        )*
    };
}

non_zero!(NonZeroU8 u8 NonZeroU16 u16 NonZeroU32 u32 NonZeroU64 u64 NonZeroUsize usize NonZeroI32 i32 NonZeroI64 i64);

/// A byte as an ASCII character.
impl u8 {
    pub fn is_ascii(&self) -> bool { *self < 0x80 }
    pub fn is_ascii_alphabetic(&self) -> bool { self.is_ascii_uppercase() || self.is_ascii_lowercase() }
    pub fn is_ascii_uppercase(&self) -> bool { matches!(*self, b'A'..=b'Z') }
    pub fn is_ascii_lowercase(&self) -> bool { matches!(*self, b'a'..=b'z') }
    pub fn is_ascii_digit(&self) -> bool { matches!(*self, b'0'..=b'9') }
    pub fn is_ascii_alphanumeric(&self) -> bool { self.is_ascii_alphabetic() || self.is_ascii_digit() }
    pub fn is_ascii_hexdigit(&self) -> bool { matches!(*self, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F') }
    pub fn is_ascii_punctuation(&self) -> bool { self.is_ascii_graphic() && !self.is_ascii_alphanumeric() }
    pub fn is_ascii_graphic(&self) -> bool { matches!(*self, b'!'..=b'~') }
    pub fn is_ascii_whitespace(&self) -> bool { matches!(*self, b' ' | b'\t' | b'\n' | b'\x0c' | b'\r') }
    pub fn is_ascii_control(&self) -> bool { *self < 0x20 || *self == 0x7f }
    pub fn to_ascii_uppercase(&self) -> u8 { if self.is_ascii_lowercase() { *self - 32 } else { *self } }
    pub fn to_ascii_lowercase(&self) -> u8 { if self.is_ascii_uppercase() { *self + 32 } else { *self } }
    pub fn make_ascii_uppercase(&mut self) { *self = self.to_ascii_uppercase(); }
    pub fn make_ascii_lowercase(&mut self) { *self = self.to_ascii_lowercase(); }
    pub fn eq_ignore_ascii_case(&self, other: &u8) -> bool { self.to_ascii_lowercase() == other.to_ascii_lowercase() }
}
