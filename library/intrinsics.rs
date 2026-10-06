//! Operations the language cannot express: the compiler generates code for
//! each of these directly at the place it is called.

pub fn size_of<T>() -> usize;

/// The name of `T`, written out in full the way Rust's library does.
pub fn type_name<T: ?Sized>() -> &'static str;
pub fn align_of<T>() -> usize;

pub fn slice_from_raw_parts<T>(data: *const T, len: usize) -> &[T];
pub fn slice_from_raw_parts_mut<T>(data: *mut T, len: usize) -> &mut [T];
pub fn slice_len<T>(slice: &[T]) -> usize;
pub fn slice_as_ptr<T>(slice: &[T]) -> *const T;

pub fn str_from_raw_parts(data: *const u8, len: usize) -> &str;
pub fn str_len(text: &str) -> usize;
pub fn str_as_ptr(text: &str) -> *const u8;

pub fn sqrt(value: f64) -> f64;

/// Which variant an enum value holds, as its position among the variants.
pub fn discriminant<T>(value: &T) -> isize;

/// Copy the value out of `*source` without affecting who owns it.
pub fn read<T>(source: *const T) -> T;

/// Store `value` at `*dest` without dropping what was there.
pub fn write<T>(dest: *mut T, value: T);

/// Run the destructor of the value at `*object`.
pub fn drop_in_place<T>(object: *mut T);

/// Whether dropping a `T` does anything.
pub fn needs_drop<T>() -> bool;

/// Take ownership of a value and never drop it.
pub fn forget<T>(value: T);

/// A value of type `T` whose bits are whatever its memory held.
pub fn uninit<T>() -> T;

/// Tells the compiler this point is never reached.
pub fn unreachable() -> !;
