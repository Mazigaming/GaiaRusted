//! Traits that describe a type without adding behaviour.

/// Values of the type can be duplicated by copying their bits.
pub trait Copy {}

impl Copy for i8 {}
impl Copy for i16 {}
impl Copy for i32 {}
impl Copy for i64 {}
impl Copy for isize {}
impl Copy for u8 {}
impl Copy for u16 {}
impl Copy for u32 {}
impl Copy for u64 {}
impl Copy for i128 {}
impl Copy for u128 {}
impl Copy for usize {}
impl Copy for f32 {}
impl Copy for f64 {}
impl Copy for bool {}
impl Copy for char {}

pub trait Sized {}
pub trait Send {}
pub trait Sync {}

/// A type that acts as if it held a `T` without holding one: it is zero
/// sized, and tells the type system what a pointer or index stands for.
pub struct PhantomData<T: ?Sized>;

impl<T: ?Sized> Clone for PhantomData<T> {
    fn clone(&self) -> PhantomData<T> {
        PhantomData
    }
}

impl<T: ?Sized> Copy for PhantomData<T> {}

impl<T: ?Sized> Default for PhantomData<T> {
    fn default() -> PhantomData<T> {
        PhantomData
    }
}

impl<T: ?Sized> PartialEq for PhantomData<T> {
    fn eq(&self, other: &PhantomData<T>) -> bool {
        true
    }
}

impl<T: ?Sized> Eq for PhantomData<T> {}

impl<T: ?Sized> PartialOrd for PhantomData<T> {
    fn partial_cmp(&self, other: &PhantomData<T>) -> Option<std::cmp::Ordering> {
        Some(std::cmp::Ordering::Equal)
    }
}

impl<T: ?Sized> Ord for PhantomData<T> {
    fn cmp(&self, other: &PhantomData<T>) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

impl<T: ?Sized> std::hash::Hash for PhantomData<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {}
}

impl<T: ?Sized> std::fmt::Debug for PhantomData<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("PhantomData<")?;
        f.write_str(std::any::type_name::<T>())?;
        f.write_str(">")
    }
}
