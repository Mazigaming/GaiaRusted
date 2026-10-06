//! The `Error` trait, which error types implement to be reported uniformly
//! and boxed as `Box<dyn Error>`.

use std::fmt::{self, Debug, Display};

pub trait Error: Debug + Display {
    /// The lower-level error that caused this one, if any.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl<'a, E: Error + 'a> From<E> for Box<dyn Error + 'a> {
    fn from(error: E) -> Box<dyn Error + 'a> {
        Box::new(error)
    }
}

/// The error a plain message becomes when boxed: shown as the message,
/// debugged as a string.
struct MessageError(String);

impl Debug for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self.0.as_str(), f)
    }
}

impl Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(self.0.as_str(), f)
    }
}

impl Error for MessageError {}

impl<'a> From<String> for Box<dyn Error + 'a> {
    fn from(message: String) -> Box<dyn Error + 'a> {
        Box::new(MessageError(message))
    }
}

impl<'a> From<&str> for Box<dyn Error + 'a> {
    fn from(message: &str) -> Box<dyn Error + 'a> {
        Box::new(MessageError(String::from(message)))
    }
}

impl Error for fmt::Error {}
impl Error for std::num::ParseIntError {}
impl Error for std::num::TryFromIntError {}
impl Error for std::str::ParseFloatError {}
impl Error for std::str::ParseCharError {}
impl Error for std::str::ParseBoolError {}
impl Error for std::convert::Infallible {}
impl Error for std::str::Utf8Error {}
impl Error for std::string::FromUtf8Error {}
impl Error for std::array::TryFromSliceError {}
impl Error for std::cell::BorrowError {}
impl Error for std::cell::BorrowMutError {}
impl<T: Error + ?Sized> Error for Box<T> {}
impl<T: Error + ?Sized> Error for &T {}
