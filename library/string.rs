//! `String`: growable, heap-allocated UTF-8 text.

pub struct String {
    pointer: *mut u8,
    len: usize,
    capacity: usize,
}

impl String {
    pub fn new() -> String {
        String { pointer: std::ptr::null_mut(), len: 0, capacity: 0 }
    }

    pub fn with_capacity(capacity: usize) -> String {
        let mut text = String::new();
        text.reserve(capacity);
        text
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn as_str(&self) -> &str {
        std::intrinsics::str_from_raw_parts(self.pointer, self.len)
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.pointer
    }

    /// Make room for at least `additional` more bytes. Capacity doubles, so
    /// appending one byte at a time takes linear time overall.
    pub fn reserve(&mut self, additional: usize) {
        let needed = self.len + additional;
        if needed <= self.capacity {
            return;
        }
        let mut capacity = if self.capacity == 0 { 16 } else { self.capacity * 2 };
        while capacity < needed {
            capacity = capacity * 2;
        }
        self.pointer = unsafe { std::libc::realloc(self.pointer, capacity) };
        self.capacity = capacity;
    }

    pub fn push_str(&mut self, text: &str) {
        self.reserve(text.len());
        unsafe {
            std::libc::memcpy(self.pointer.add(self.len), text.as_ptr(), text.len());
        }
        self.len += text.len();
    }

    pub fn push_byte(&mut self, byte: u8) {
        self.reserve(1);
        unsafe {
            *self.pointer.add(self.len) = byte;
        }
        self.len += 1;
    }

    /// Append a character, encoded as UTF-8.
    pub fn push(&mut self, character: char) {
        let code = character as u32;
        if code < 128 {
            self.push_byte(code as u8);
        } else if code < 2048 {
            self.push_byte((192 | (code >> 6)) as u8);
            self.push_byte((128 | (code & 63)) as u8);
        } else if code < 65536 {
            self.push_byte((224 | (code >> 12)) as u8);
            self.push_byte((128 | ((code >> 6) & 63)) as u8);
            self.push_byte((128 | (code & 63)) as u8);
        } else {
            self.push_byte((240 | (code >> 18)) as u8);
            self.push_byte((128 | ((code >> 12) & 63)) as u8);
            self.push_byte((128 | ((code >> 6) & 63)) as u8);
            self.push_byte((128 | (code & 63)) as u8);
        }
    }

    pub fn pop(&mut self) -> Option<char> {
        let last = match self.as_str().chars().last() {
            Some(character) => character,
            None => return None,
        };
        let mut encoded = String::new();
        encoded.push(last);
        self.len -= encoded.len();
        Some(last)
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn truncate(&mut self, len: usize) {
        if len < self.len {
            if !self.as_str().is_char_boundary(len) {
                panic!("assertion failed: self.is_char_boundary(new_len)");
            }
            self.len = len;
        }
    }

    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe { &mut *(std::intrinsics::str_from_raw_parts(self.pointer, self.len) as *const str as *mut str) }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let bytes = self.as_bytes().to_vec();
        bytes
    }

    pub fn into_boxed_str(self) -> Box<str> {
        Box::from(self)
    }

    /// The text, owned: for code written against `Cow<str>`.
    pub fn into_owned(self) -> String {
        self
    }

    /// Put `text` at byte position `index`.
    pub fn insert_str(&mut self, index: usize, text: &str) {
        if !self.as_str().is_char_boundary(index) {
            panic!("assertion failed: self.is_char_boundary(idx)");
        }
        self.reserve(text.len());
        unsafe {
            std::libc::memmove(self.pointer.add(index + text.len()), self.pointer.add(index), self.len - index);
            std::libc::memcpy(self.pointer.add(index), text.as_ptr(), text.len());
        }
        self.len += text.len();
    }

    pub fn insert(&mut self, index: usize, character: char) {
        let mut buffer = [0u8; 4];
        let encoded = character.encode_utf8(&mut buffer);
        self.insert_str(index, encoded);
    }

    /// Take out the character at byte position `index`.
    pub fn remove(&mut self, index: usize) -> char {
        let character = match self.as_str()[index..].chars().next() {
            Some(character) => character,
            None => panic!("cannot remove a char from the end of a string"),
        };
        let length = character.len_utf8();
        unsafe {
            std::libc::memmove(self.pointer.add(index), self.pointer.add(index + length), self.len - index - length);
        }
        self.len -= length;
        character
    }

    /// Keep only the characters for which `keep` is true.
    pub fn retain<F: FnMut(char) -> bool>(&mut self, keep: F) {
        let mut keep = keep;
        let mut kept = String::with_capacity(self.len);
        for character in self.as_str().chars() {
            if keep(character) {
                kept.push(character);
            }
        }
        *self = kept;
    }

    /// The text from byte `at` on, as a new string; this one keeps the rest.
    pub fn split_off(&mut self, at: usize) -> String {
        let rest = String::from(&self.as_str()[at..]);
        self.truncate(at);
        rest
    }

    /// Take the text in `range` out, as an iterator over its characters.
    pub fn drain<R: std::ops::RangeBounds<usize>>(&mut self, range: R) -> Drain {
        let (start, end) = std::ops::range_positions(&range, self.len);
        let removed = String::from(&self.as_str()[start..end]);
        self.replace_range(start..end, "");
        Drain { text: removed, position: 0 }
    }

    pub fn replace_range<R: std::ops::RangeBounds<usize>>(&mut self, range: R, with: &str) {
        let (start, end) = std::ops::range_positions(&range, self.len);
        let mut result = String::with_capacity(self.len - (end - start) + with.len());
        result.push_str(&self.as_str()[..start]);
        result.push_str(with);
        result.push_str(&self.as_str()[end..]);
        *self = result;
    }

    /// The bytes as a string, if they are valid UTF-8.
    pub fn from_utf8(bytes: Vec<u8>) -> Result<String, FromUtf8Error> {
        match std::str::from_utf8(bytes.as_slice()) {
            Ok(text) => Ok(String::from(text)),
            Err(error) => Err(FromUtf8Error { bytes, error }),
        }
    }

    /// The bytes as a string, with `U+FFFD` for what is not valid UTF-8.
    pub fn from_utf8_lossy(bytes: &[u8]) -> String {
        let mut result = String::with_capacity(bytes.len());
        let mut rest = bytes;
        loop {
            let valid = std::str::utf8_prefix(rest);
            result.push_str(std::intrinsics::str_from_raw_parts(rest.as_ptr(), valid));
            if valid == rest.len() {
                return result;
            }
            result.push('\u{FFFD}');
            // Skip the bytes of the broken sequence: the first, and any
            // continuation bytes after it.
            let mut skip = valid + 1;
            while skip < rest.len() && (rest[skip] & 0xC0) == 0x80 && skip < valid + 4 {
                skip += 1;
            }
            rest = &rest[skip..];
        }
    }

    pub fn from_iter<I: IntoIterator<Item = char>>(items: I) -> String {
        let mut text = String::new();
        for character in items {
            text.push(character);
        }
        text
    }
}

/// The characters taken out of a string by [`String::drain`].
pub struct Drain {
    text: String,
    position: usize,
}

impl Iterator for Drain {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        let character = self.text.as_str()[self.position..].chars().next()?;
        self.position += character.len_utf8();
        Some(character)
    }
}

/// Bytes that are not valid UTF-8, given back.
#[derive(Debug)]
pub struct FromUtf8Error {
    bytes: Vec<u8>,
    error: std::str::Utf8Error,
}

impl FromUtf8Error {
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn utf8_error(&self) -> std::str::Utf8Error {
        self.error.clone()
    }
}

impl std::fmt::Display for FromUtf8Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.error, f)
    }
}

impl From<char> for String {
    fn from(character: char) -> String {
        let mut text = String::new();
        text.push(character);
        text
    }
}

impl From<&String> for String {
    fn from(text: &String) -> String {
        text.clone()
    }
}

impl From<&mut str> for String {
    fn from(text: &mut str) -> String {
        String::from(&*text)
    }
}

impl std::iter::FromIterator<char> for String {
    fn from_iter<I>(items: I) -> String {
        let mut items = items;
        let mut text = String::new();
        while let Some(character) = items.next() {
            text.push(character);
        }
        text
    }
}

impl std::iter::FromIterator<&char> for String {
    fn from_iter<I>(items: I) -> String {
        let mut items = items;
        let mut text = String::new();
        while let Some(character) = items.next() {
            text.push(*character);
        }
        text
    }
}

impl std::iter::FromIterator<&str> for String {
    fn from_iter<I>(items: I) -> String {
        let mut items = items;
        let mut text = String::new();
        while let Some(piece) = items.next() {
            text.push_str(piece);
        }
        text
    }
}

impl std::iter::FromIterator<String> for String {
    fn from_iter<I>(items: I) -> String {
        let mut items = items;
        let mut text = String::new();
        while let Some(piece) = items.next() {
            text.push_str(piece.as_str());
        }
        text
    }
}

impl std::iter::Extend<char> for String {
    fn extend<I: IntoIterator<Item = char>>(&mut self, items: I) {
        for character in items {
            self.push(character);
        }
    }

    fn extend_one(&mut self, character: char) {
        self.push(character);
    }
}

impl std::iter::Extend<&str> for String {
    fn extend<I: IntoIterator<Item = &str>>(&mut self, items: I) {
        for piece in items {
            self.push_str(piece);
        }
    }

    fn extend_one(&mut self, piece: &str) {
        self.push_str(piece);
    }
}

impl std::iter::Extend<String> for String {
    fn extend<I: IntoIterator<Item = String>>(&mut self, items: I) {
        for piece in items {
            self.push_str(piece.as_str());
        }
    }

    fn extend_one(&mut self, piece: String) {
        self.push_str(piece.as_str());
    }
}

impl std::fmt::Write for String {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.push_str(text);
        Ok(())
    }
}

impl std::ops::DerefMut for String {
    fn deref_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl Drop for String {
    fn drop(&mut self) {
        unsafe { std::libc::free(self.pointer) }
    }
}

impl std::ops::Deref for String {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl Clone for String {
    fn clone(&self) -> String {
        String::from(self.as_str())
    }
}

impl Default for String {
    fn default() -> String {
        String::new()
    }
}

impl PartialEq for String {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for String {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for String {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for str {
    fn eq(&self, other: &String) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<String> for &str {
    fn eq(&self, other: &String) -> bool {
        *self == other.as_str()
    }
}

impl Eq for String {}

impl PartialOrd for String {
    fn partial_cmp(&self, other: &String) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self.as_str(), other.as_str()))
    }
}

impl Ord for String {
    fn cmp(&self, other: &String) -> std::cmp::Ordering {
        Ord::cmp(self.as_str(), other.as_str())
    }
}

impl From<&str> for String {
    fn from(text: &str) -> String {
        let mut owned = String::with_capacity(text.len());
        owned.push_str(text);
        owned
    }
}

impl std::ops::Add<&str> for String {
    type Output = String;

    fn add(self, other: &str) -> String {
        let mut joined = self;
        joined.push_str(other);
        joined
    }
}

impl std::ops::AddAssign<&str> for String {
    fn add_assign(&mut self, other: &str) {
        self.push_str(other);
    }
}

/// Conversion to text through `Display`.
pub trait ToString {
    fn to_string(&self) -> String;
}

impl<T: std::fmt::Display + ?Sized> ToString for T {
    fn to_string(&self) -> String {
        let mut formatter = std::fmt::Formatter::new();
        std::fmt::Display::fmt(self, &mut formatter);
        formatter.finish()
    }
}
