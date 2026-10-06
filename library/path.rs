//! File system paths: `Path`, a borrowed path, and `PathBuf`, an owned one.
//!
//! A path is text here, so `Path` is to `PathBuf` what `str` is to
//! `String`: an unsized wrapper of `str`, seen only behind a reference.

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

pub const MAIN_SEPARATOR: char = '/';

pub struct Path {
    inner: str,
}

impl Path {
    pub fn new<S: AsRef<OsStr> + ?Sized>(text: &S) -> &Path {
        let text: &str = text.as_ref().as_str();
        unsafe { &*(text as *const str as *const Path) }
    }

    fn as_str(&self) -> &str {
        unsafe { &*(self as *const Path as *const str) }
    }

    pub fn as_os_str(&self) -> &OsStr {
        OsStr::new(self.as_str())
    }

    pub fn to_str(&self) -> Option<&str> {
        Some(self.as_str())
    }

    pub fn to_string_lossy(&self) -> String {
        String::from(self.as_str())
    }

    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf { inner: String::from(self.as_str()) }
    }

    pub fn display(&self) -> Display<'_> {
        Display { path: self }
    }

    pub fn is_absolute(&self) -> bool {
        self.as_str().starts_with('/')
    }

    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }

    pub fn has_root(&self) -> bool {
        self.is_absolute()
    }

    /// `self` followed by `other`, or `other` alone if it is absolute.
    pub fn join<P: AsRef<Path>>(&self, other: P) -> PathBuf {
        let mut joined = self.to_path_buf();
        joined.push(other);
        joined
    }

    /// The path without its last component.
    pub fn parent(&self) -> Option<&Path> {
        let text = self.as_str().trim_end_matches('/');
        if text.is_empty() {
            return None;
        }
        match text.rfind('/') {
            Some(0) => Some(Path::new("/")),
            Some(index) => Some(Path::new(&text[..index])),
            None => Some(Path::new("")),
        }
    }

    /// The last component, unless it is `..`.
    pub fn file_name(&self) -> Option<&OsStr> {
        let text = self.as_str().trim_end_matches('/');
        let name = match text.rfind('/') {
            Some(index) => &text[index + 1..],
            None => text,
        };
        if name.is_empty() || name == ".." {
            None
        } else {
            Some(OsStr::new(name))
        }
    }

    /// The file name before its last `.`.
    pub fn file_stem(&self) -> Option<&OsStr> {
        let name = self.file_name()?.as_str();
        match name.rfind('.') {
            Some(0) | None => Some(OsStr::new(name)),
            Some(index) => Some(OsStr::new(&name[..index])),
        }
    }

    /// The file name after its last `.`.
    pub fn extension(&self) -> Option<&OsStr> {
        let name = self.file_name()?.as_str();
        match name.rfind('.') {
            Some(0) | None => None,
            Some(index) => Some(OsStr::new(&name[index + 1..])),
        }
    }

    pub fn with_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf {
        let mut path = self.to_path_buf();
        path.set_extension(extension);
        path
    }

    pub fn with_file_name<S: AsRef<OsStr>>(&self, name: S) -> PathBuf {
        let mut path = self.to_path_buf();
        path.set_file_name(name);
        path
    }

    pub fn starts_with<P: AsRef<Path>>(&self, base: P) -> bool {
        let mine: Vec<&str> = self.components_text();
        let theirs: Vec<&str> = base.as_ref().components_text();
        mine.len() >= theirs.len() && mine[..theirs.len()] == theirs[..]
    }

    pub fn ends_with<P: AsRef<Path>>(&self, child: P) -> bool {
        let mine: Vec<&str> = self.components_text();
        let theirs: Vec<&str> = child.as_ref().components_text();
        mine.len() >= theirs.len() && mine[mine.len() - theirs.len()..] == theirs[..]
    }

    fn components_text(&self) -> Vec<&str> {
        let text = self.as_str();
        let mut parts: Vec<&str> = text.split('/').filter(|part| !part.is_empty() && *part != ".").collect();
        if text.starts_with('/') {
            parts.insert(0, "/");
        }
        parts
    }

    pub fn exists(&self) -> bool {
        std::fs::metadata(self).is_ok()
    }

    pub fn is_file(&self) -> bool {
        std::fs::metadata(self).map(|metadata| metadata.is_file()).unwrap_or(false)
    }

    pub fn is_dir(&self) -> bool {
        std::fs::metadata(self).map(|metadata| metadata.is_dir()).unwrap_or(false)
    }
}

/// Shows a path with `{}`.
pub struct Display<'a> {
    path: &'a Path,
}

impl<'a> fmt::Display for Display<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.path.as_str(), f)
    }
}

impl<'a> fmt::Debug for Display<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.path.as_str(), f)
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Path) -> bool {
        self.components_text() == other.components_text()
    }
}

impl Eq for Path {}

impl PartialOrd for Path {
    fn partial_cmp(&self, other: &Path) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Path {
    fn cmp(&self, other: &Path) -> Ordering {
        self.components_text().cmp(&other.components_text())
    }
}

impl Hash for Path {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for part in self.components_text() {
            part.hash(state);
        }
    }
}

impl ToOwned for Path {
    type Owned = PathBuf;

    fn to_owned(&self) -> PathBuf {
        self.to_path_buf()
    }
}

/// An owned, growable path.
#[derive(Clone, Default)]
pub struct PathBuf {
    inner: String,
}

impl PathBuf {
    pub fn new() -> PathBuf {
        PathBuf { inner: String::new() }
    }

    pub fn as_path(&self) -> &Path {
        Path::new(self.inner.as_str())
    }

    /// Add `other` as the last component; an absolute `other` replaces
    /// the path.
    pub fn push<P: AsRef<Path>>(&mut self, other: P) {
        let other = other.as_ref().as_str();
        if other.starts_with('/') {
            self.inner = String::from(other);
            return;
        }
        if !self.inner.is_empty() && !self.inner.ends_with('/') {
            self.inner.push('/');
        }
        self.inner.push_str(other);
    }

    /// Remove the last component; `false` if there is none.
    pub fn pop(&mut self) -> bool {
        match self.as_path().parent() {
            Some(parent) => {
                let len = parent.as_str().len();
                self.inner.truncate(len);
                true
            }
            None => false,
        }
    }

    pub fn set_file_name<S: AsRef<OsStr>>(&mut self, name: S) {
        if self.as_path().file_name().is_some() {
            self.pop();
        }
        self.push(name.as_ref().as_str());
    }

    pub fn set_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        let Some(stem) = self.as_path().file_stem() else { return false };
        let mut name = String::from(stem.as_str());
        let extension = extension.as_ref().as_str();
        if !extension.is_empty() {
            name.push('.');
            name.push_str(extension);
        }
        self.set_file_name(name.as_str());
        true
    }

    pub fn into_os_string(self) -> std::ffi::OsString {
        std::ffi::OsString::from(self.inner)
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }
}

impl Deref for PathBuf {
    type Target = Path;

    fn deref(&self) -> &Path {
        self.as_path()
    }
}

impl fmt::Debug for PathBuf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.inner.as_str(), f)
    }
}

impl PartialEq for PathBuf {
    fn eq(&self, other: &PathBuf) -> bool {
        self.as_path() == other.as_path()
    }
}

impl Eq for PathBuf {}

impl PartialOrd for PathBuf {
    fn partial_cmp(&self, other: &PathBuf) -> Option<Ordering> {
        Some(self.as_path().cmp(other.as_path()))
    }
}

impl Ord for PathBuf {
    fn cmp(&self, other: &PathBuf) -> Ordering {
        self.as_path().cmp(other.as_path())
    }
}

impl Hash for PathBuf {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_path().hash(state)
    }
}

impl std::borrow::Borrow<Path> for PathBuf {
    fn borrow(&self) -> &Path {
        self.as_path()
    }
}

impl From<String> for PathBuf {
    fn from(text: String) -> PathBuf {
        PathBuf { inner: text }
    }
}

impl From<&str> for PathBuf {
    fn from(text: &str) -> PathBuf {
        PathBuf { inner: String::from(text) }
    }
}

impl From<&Path> for PathBuf {
    fn from(path: &Path) -> PathBuf {
        path.to_path_buf()
    }
}

impl<P: AsRef<Path>> FromIterator<P> for PathBuf {
    fn from_iter<I: IntoIterator<Item = P>>(parts: I) -> PathBuf {
        let mut path = PathBuf::new();
        for part in parts {
            path.push(part);
        }
        path
    }
}

impl AsRef<Path> for Path {
    fn as_ref(&self) -> &Path {
        self
    }
}

impl AsRef<Path> for PathBuf {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<Path> for str {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl AsRef<Path> for String {
    fn as_ref(&self) -> &Path {
        Path::new(self.as_str())
    }
}

impl AsRef<Path> for OsStr {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl AsRef<OsStr> for Path {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl AsRef<OsStr> for PathBuf {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}
