//! Strings as the operating system takes them. On Linux they are bytes,
//! nearly always UTF-8; here they are text, `OsStr` an unsized wrapper of
//! `str` and `OsString` one of `String`.

use std::fmt;

pub struct OsStr {
    inner: str,
}

impl OsStr {
    pub fn new<S: AsRef<str> + ?Sized>(text: &S) -> &OsStr {
        let text: &str = text.as_ref();
        unsafe { &*(text as *const str as *const OsStr) }
    }

    pub fn as_str(&self) -> &str {
        unsafe { &*(self as *const OsStr as *const str) }
    }

    pub fn to_str(&self) -> Option<&str> {
        Some(self.as_str())
    }

    pub fn to_string_lossy(&self) -> String {
        String::from(self.as_str())
    }

    pub fn to_os_string(&self) -> OsString {
        OsString { inner: String::from(self.as_str()) }
    }

    pub fn len(&self) -> usize {
        self.as_str().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}

impl fmt::Debug for OsStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl PartialEq for OsStr {
    fn eq(&self, other: &OsStr) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for OsStr {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl Eq for OsStr {}

impl AsRef<OsStr> for OsStr {
    fn as_ref(&self) -> &OsStr {
        self
    }
}

impl AsRef<OsStr> for str {
    fn as_ref(&self) -> &OsStr {
        OsStr::new(self)
    }
}

impl AsRef<OsStr> for String {
    fn as_ref(&self) -> &OsStr {
        OsStr::new(self.as_str())
    }
}

impl AsRef<str> for OsStr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// An owned operating-system string.
#[derive(Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OsString {
    inner: String,
}

impl OsString {
    pub fn new() -> OsString {
        OsString { inner: String::new() }
    }

    pub fn as_os_str(&self) -> &OsStr {
        OsStr::new(self.inner.as_str())
    }

    pub fn into_string(self) -> Result<String, OsString> {
        Ok(self.inner)
    }

    pub fn push<S: AsRef<OsStr>>(&mut self, text: S) {
        self.inner.push_str(text.as_ref().as_str());
    }
}

impl std::ops::Deref for OsString {
    type Target = OsStr;

    fn deref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl fmt::Debug for OsString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.inner.as_str(), f)
    }
}

impl From<String> for OsString {
    fn from(text: String) -> OsString {
        OsString { inner: text }
    }
}

impl From<&str> for OsString {
    fn from(text: &str) -> OsString {
        OsString { inner: String::from(text) }
    }
}

impl AsRef<OsStr> for OsString {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}
