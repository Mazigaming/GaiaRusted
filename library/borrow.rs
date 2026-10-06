//! Borrowing as another type.

/// A type that can be viewed as a `&Borrowed`: a `String` as a `str`.
/// Collections use it so that a lookup can be made with a borrowed key.
pub trait Borrow<Borrowed: ?Sized> {
    fn borrow(&self) -> &Borrowed;
}

impl<T: ?Sized> Borrow<T> for T {
    fn borrow(&self) -> &T {
        self
    }
}

impl Borrow<str> for String {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<T: ?Sized> Borrow<T> for &T {
    fn borrow(&self) -> &T {
        *self
    }
}

impl<T: ?Sized> Borrow<T> for &mut T {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T> Borrow<[T]> for Vec<T> {
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: ?Sized> Borrow<T> for Box<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

/// A type that has an owned form a borrowed value can be copied into:
/// `str` and `String`, `[T]` and `Vec<T>`.
pub trait ToOwned {
    type Owned;
    fn to_owned(&self) -> Self::Owned;
}

impl ToOwned for str {
    type Owned = String;
    fn to_owned(&self) -> String {
        String::from(self)
    }
}

impl<T: Clone> ToOwned for [T] {
    type Owned = Vec<T>;
    fn to_owned(&self) -> Vec<T> {
        self.to_vec()
    }
}
