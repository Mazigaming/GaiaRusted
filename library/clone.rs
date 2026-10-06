//! Explicit duplication.

pub trait Clone {
    fn clone(&self) -> Self;
}

impl Clone for i8 { fn clone(&self) -> i8 { *self } }
impl Clone for i16 { fn clone(&self) -> i16 { *self } }
impl Clone for i32 { fn clone(&self) -> i32 { *self } }
impl Clone for i64 { fn clone(&self) -> i64 { *self } }
impl Clone for isize { fn clone(&self) -> isize { *self } }
impl Clone for u8 { fn clone(&self) -> u8 { *self } }
impl Clone for u16 { fn clone(&self) -> u16 { *self } }
impl Clone for u32 { fn clone(&self) -> u32 { *self } }
impl Clone for u64 { fn clone(&self) -> u64 { *self } }
impl Clone for i128 { fn clone(&self) -> i128 { *self } }
impl Clone for u128 { fn clone(&self) -> u128 { *self } }
impl Clone for usize { fn clone(&self) -> usize { *self } }
impl Clone for f32 { fn clone(&self) -> f32 { *self } }
impl Clone for f64 { fn clone(&self) -> f64 { *self } }
impl Clone for bool { fn clone(&self) -> bool { *self } }
impl Clone for char { fn clone(&self) -> char { *self } }

impl<T: ?Sized> Clone for &T {
    fn clone(&self) -> &T {
        *self
    }
}

impl Clone for () {
    fn clone(&self) -> () {}
}

/// Defines `Clone` for tuples whose elements all have it.
macro_rules! tuple_clone {
    ($($name:ident $index:tt)+) => {
        impl<$($name: Clone),+> Clone for ($($name,)+) {
            fn clone(&self) -> ($($name,)+) {
                ($(self.$index.clone(),)+)
            }
        }
    };
}

tuple_clone!(A 0);
tuple_clone!(A 0 B 1);
tuple_clone!(A 0 B 1 C 2);
tuple_clone!(A 0 B 1 C 2 D 3);
tuple_clone!(A 0 B 1 C 2 D 3 E 4);
tuple_clone!(A 0 B 1 C 2 D 3 E 4 F 5);
tuple_clone!(A 0 B 1 C 2 D 3 E 4 F 5 G 6);
tuple_clone!(A 0 B 1 C 2 D 3 E 4 F 5 G 6 H 7);
tuple_clone!(A 0 B 1 C 2 D 3 E 4 F 5 G 6 H 7 I 8);
tuple_clone!(A 0 B 1 C 2 D 3 E 4 F 5 G 6 H 7 I 8 J 9);
tuple_clone!(A 0 B 1 C 2 D 3 E 4 F 5 G 6 H 7 I 8 J 9 K 10);
tuple_clone!(A 0 B 1 C 2 D 3 E 4 F 5 G 6 H 7 I 8 J 9 K 10 L 11);
