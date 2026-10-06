//! Types with a sensible "empty" value.

pub trait Default {
    fn default() -> Self;
}

impl Default for i8 { fn default() -> i8 { 0 } }
impl Default for i16 { fn default() -> i16 { 0 } }
impl Default for i32 { fn default() -> i32 { 0 } }
impl Default for i64 { fn default() -> i64 { 0 } }
impl Default for isize { fn default() -> isize { 0 } }
impl Default for u8 { fn default() -> u8 { 0 } }
impl Default for u16 { fn default() -> u16 { 0 } }
impl Default for u32 { fn default() -> u32 { 0 } }
impl Default for u64 { fn default() -> u64 { 0 } }
impl Default for i128 { fn default() -> i128 { 0 } }
impl Default for u128 { fn default() -> u128 { 0 } }
impl Default for usize { fn default() -> usize { 0 } }
impl Default for f32 { fn default() -> f32 { 0.0 } }
impl Default for f64 { fn default() -> f64 { 0.0 } }
impl Default for bool { fn default() -> bool { false } }
impl Default for char { fn default() -> char { '\0' } }
impl Default for () { fn default() -> () {} }

impl Default for &str {
    fn default() -> &str {
        ""
    }
}

impl<T> Default for &[T] {
    fn default() -> &[T] {
        std::intrinsics::slice_from_raw_parts(std::ptr::null(), 0)
    }
}

impl<T: Default> Default for Box<T> {
    fn default() -> Box<T> {
        Box::new(T::default())
    }
}

/// Defines `Default` for tuples whose elements all have it.
macro_rules! tuple_default {
    ($($name:ident)+) => {
        impl<$($name: Default),+> Default for ($($name,)+) {
            fn default() -> ($($name,)+) {
                ($($name::default(),)+)
            }
        }
    };
}

tuple_default!(A);
tuple_default!(A B);
tuple_default!(A B C);
tuple_default!(A B C D);
tuple_default!(A B C D E);
tuple_default!(A B C D E F);
tuple_default!(A B C D E F G);
tuple_default!(A B C D E F G H);
tuple_default!(A B C D E F G H I);
tuple_default!(A B C D E F G H I J);
tuple_default!(A B C D E F G H I J K);
tuple_default!(A B C D E F G H I J K L);
