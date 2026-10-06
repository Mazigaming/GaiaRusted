//! The process's environment: variables, arguments, directories.

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

/// `text` with a NUL after it, for the C library.
fn c_string(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() + 1);
    bytes.extend_from_slice(text.as_bytes());
    bytes.push(0);
    bytes
}

/// The text of a C string.
fn from_c_string(text: *const u8) -> String {
    unsafe { String::from(std::intrinsics::str_from_raw_parts(text, std::libc::strlen(text))) }
}

/// Why `var` gave no value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarError {
    NotPresent,
    NotUnicode(OsString),
}

impl fmt::Display for VarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VarError::NotPresent => f.write_str("environment variable not found"),
            VarError::NotUnicode(text) => write!(f, "environment variable was not valid unicode: {:?}", text),
        }
    }
}

impl std::error::Error for VarError {}

/// The value of the environment variable `key`.
pub fn var<K: AsRef<str>>(key: K) -> Result<String, VarError> {
    let name = c_string(key.as_ref());
    let value = unsafe { std::libc::getenv(name.as_ptr()) };
    if value.is_null() {
        Err(VarError::NotPresent)
    } else {
        Ok(from_c_string(value))
    }
}

pub fn var_os<K: AsRef<str>>(key: K) -> Option<OsString> {
    var(key).ok().map(OsString::from)
}

/// The directory for temporary files: `$TMPDIR`, or `/tmp`.
pub fn temp_dir() -> PathBuf {
    match var("TMPDIR") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from("/tmp"),
    }
}

pub fn current_dir() -> std::io::Result<PathBuf> {
    let mut buffer = vec![0u8; 4096];
    let result = unsafe { std::libc::getcwd(buffer.as_mut_ptr(), buffer.len()) };
    if result.is_null() {
        return Err(std::io::Error::last_os_error());
    }
    Ok(PathBuf::from(from_c_string(buffer.as_ptr())))
}

pub fn home_dir() -> Option<PathBuf> {
    var("HOME").ok().map(PathBuf::from)
}

/// The command-line arguments, the program's name first.
pub fn args() -> Args {
    // The kernel lists them, each ending in a NUL.
    let listed = std::fs::read("/proc/self/cmdline").unwrap_or_default();
    let mut arguments: Vec<String> = Vec::new();
    let mut start = 0;
    for (index, &byte) in listed.iter().enumerate() {
        if byte == 0 {
            arguments.push(String::from_utf8_lossy(&listed[start..index]));
            start = index + 1;
        }
    }
    Args { inner: arguments.into_iter() }
}

pub fn args_os() -> impl Iterator<Item = OsString> {
    args().map(OsString::from)
}

pub struct Args {
    inner: std::vec::IntoIter<String>,
}

impl Iterator for Args {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl DoubleEndedIterator for Args {
    fn next_back(&mut self) -> Option<String> {
        self.inner.next_back()
    }
}

impl ExactSizeIterator for Args {
    fn len(&self) -> usize {
        self.inner.len()
    }
}
