//! Characters: Unicode scalar values.
//!
//! Classification and case mapping follow the Unicode tables in
//! [`std::unicode`], the ones rustc's core library uses.

/// The largest valid `char`.
pub const MAX: char = '\u{10FFFF}';
/// `�`, which stands in for what could not be decoded.
pub const REPLACEMENT_CHARACTER: char = '\u{FFFD}';

/// The `char` with this code, if it is one: not a surrogate, not past `MAX`.
pub fn from_u32(code: u32) -> Option<char> {
    if code > 0x10FFFF || (code >= 0xD800 && code <= 0xDFFF) {
        None
    } else {
        Some(unsafe { from_u32_unchecked(code) })
    }
}

/// A `char` from a code the caller knows to be valid.
pub unsafe fn from_u32_unchecked(code: u32) -> char {
    // The types have the same representation.
    *(&code as *const u32 as *const char)
}

/// The digit `value` in `radix`: `from_digit(11, 16)` is `'b'`.
pub fn from_digit(value: u32, radix: u32) -> Option<char> {
    if radix < 2 || radix > 36 || value >= radix {
        return None;
    }
    let code = if value < 10 { '0' as u32 + value } else { 'a' as u32 + value - 10 };
    from_u32(code)
}

/// The characters a case conversion produces: usually one, sometimes more
/// (`'ß'.to_uppercase()` is `SS`).
pub struct CaseMapping {
    chars: [char; 3],
    next: usize,
    len: usize,
}

impl CaseMapping {
    /// The conversion a table gives: up to three characters, padded with `\0`.
    fn of(chars: [char; 3]) -> CaseMapping {
        let len = if chars[2] != '\0' {
            3
        } else if chars[1] != '\0' {
            2
        } else {
            1
        };
        CaseMapping { chars, next: 0, len }
    }
}

impl Iterator for CaseMapping {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if self.next == self.len {
            return None;
        }
        self.next += 1;
        Some(self.chars[self.next - 1])
    }
}

impl std::fmt::Display for CaseMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut index = self.next;
        while index < self.len {
            f.write_char(self.chars[index]);
            index += 1;
        }
        Ok(())
    }
}

impl char {
    pub const MAX: char = '\u{10FFFF}';
    pub const REPLACEMENT_CHARACTER: char = '\u{FFFD}';

    pub fn from_u32(code: u32) -> Option<char> { from_u32(code) }
    pub fn from_digit(value: u32, radix: u32) -> Option<char> { from_digit(value, radix) }

    pub fn is_ascii(&self) -> bool { (*self as u32) < 0x80 }
    pub fn is_ascii_alphabetic(&self) -> bool { self.is_ascii_uppercase() || self.is_ascii_lowercase() }
    pub fn is_ascii_uppercase(&self) -> bool { *self >= 'A' && *self <= 'Z' }
    pub fn is_ascii_lowercase(&self) -> bool { *self >= 'a' && *self <= 'z' }
    pub fn is_ascii_digit(&self) -> bool { *self >= '0' && *self <= '9' }
    pub fn is_ascii_alphanumeric(&self) -> bool { self.is_ascii_alphabetic() || self.is_ascii_digit() }
    pub fn is_ascii_hexdigit(&self) -> bool {
        self.is_ascii_digit() || (*self >= 'a' && *self <= 'f') || (*self >= 'A' && *self <= 'F')
    }
    pub fn is_ascii_punctuation(&self) -> bool { self.is_ascii_graphic() && !self.is_ascii_alphanumeric() }
    pub fn is_ascii_graphic(&self) -> bool { *self > ' ' && *self <= '~' }
    pub fn is_ascii_whitespace(&self) -> bool {
        *self == ' ' || *self == '\t' || *self == '\n' || *self == '\r' || *self == '\u{C}'
    }
    pub fn is_ascii_control(&self) -> bool { (*self as u32) < 0x20 || *self == '\u{7F}' }

    pub fn to_ascii_uppercase(&self) -> char {
        if self.is_ascii_lowercase() { unsafe { from_u32_unchecked(*self as u32 - 32) } } else { *self }
    }
    pub fn to_ascii_lowercase(&self) -> char {
        if self.is_ascii_uppercase() { unsafe { from_u32_unchecked(*self as u32 + 32) } } else { *self }
    }
    pub fn make_ascii_uppercase(&mut self) { *self = self.to_ascii_uppercase(); }
    pub fn make_ascii_lowercase(&mut self) { *self = self.to_ascii_lowercase(); }
    pub fn eq_ignore_ascii_case(&self, other: &char) -> bool {
        self.to_ascii_lowercase() == other.to_ascii_lowercase()
    }

    pub fn is_whitespace(self) -> bool {
        match self {
            ' ' | '\x09'..='\x0d' => true,
            c => c > '\x7f' && std::unicode::white_space(c),
        }
    }

    pub fn is_control(self) -> bool {
        std::unicode::control(self)
    }

    pub fn is_alphabetic(self) -> bool {
        match self {
            'a'..='z' | 'A'..='Z' => true,
            c => c > '\x7f' && std::unicode::alphabetic(c),
        }
    }

    pub fn is_numeric(self) -> bool {
        match self {
            '0'..='9' => true,
            c => c > '\x7f' && std::unicode::numeric(c),
        }
    }

    pub fn is_alphanumeric(self) -> bool {
        self.is_alphabetic() || self.is_numeric()
    }

    pub fn is_uppercase(self) -> bool {
        match self {
            'A'..='Z' => true,
            c => c > '\x7f' && std::unicode::uppercase(c),
        }
    }

    pub fn is_lowercase(self) -> bool {
        match self {
            'a'..='z' => true,
            c => c > '\x7f' && std::unicode::lowercase(c),
        }
    }

    pub fn to_uppercase(self) -> CaseMapping {
        CaseMapping::of(std::unicode::to_upper(self))
    }

    pub fn to_lowercase(self) -> CaseMapping {
        CaseMapping::of(std::unicode::to_lower(self))
    }

    pub fn to_digit(self, radix: u32) -> Option<u32> {
        let value = if self >= '0' && self <= '9' {
            self as u32 - '0' as u32
        } else if self >= 'a' && self <= 'z' {
            self as u32 - 'a' as u32 + 10
        } else if self >= 'A' && self <= 'Z' {
            self as u32 - 'A' as u32 + 10
        } else {
            return None;
        };
        if value < radix { Some(value) } else { None }
    }

    pub fn is_digit(self, radix: u32) -> bool { self.to_digit(radix).is_some() }

    /// How many bytes the character takes in UTF-8.
    pub fn len_utf8(self) -> usize {
        let code = self as u32;
        if code < 0x80 {
            1
        } else if code < 0x800 {
            2
        } else if code < 0x10000 {
            3
        } else {
            4
        }
    }

    pub fn len_utf16(self) -> usize { if (self as u32) < 0x10000 { 1 } else { 2 } }

    /// Write the UTF-8 encoding into `buffer` and return that part of it.
    pub fn encode_utf8(self, buffer: &mut [u8]) -> &mut str {
        let code = self as u32;
        let length = self.len_utf8();
        if length == 1 {
            buffer[0] = code as u8;
        } else if length == 2 {
            buffer[0] = (0xC0 | (code >> 6)) as u8;
            buffer[1] = (0x80 | (code & 0x3F)) as u8;
        } else if length == 3 {
            buffer[0] = (0xE0 | (code >> 12)) as u8;
            buffer[1] = (0x80 | ((code >> 6) & 0x3F)) as u8;
            buffer[2] = (0x80 | (code & 0x3F)) as u8;
        } else {
            buffer[0] = (0xF0 | (code >> 18)) as u8;
            buffer[1] = (0x80 | ((code >> 12) & 0x3F)) as u8;
            buffer[2] = (0x80 | ((code >> 6) & 0x3F)) as u8;
            buffer[3] = (0x80 | (code & 0x3F)) as u8;
        }
        let pointer = buffer.as_mut_ptr();
        unsafe { &mut *(std::intrinsics::str_from_raw_parts(pointer, length) as *const str as *mut str) }
    }
}

/// How one character is written in an escaped form: as itself, after a
/// backslash, or as `\u{..}` with its code in hexadecimal.
#[derive(Clone, Debug)]
struct Escaped {
    chars: [char; 10],
    len: usize,
    next: usize,
}

impl Escaped {
    fn literal(c: char) -> Escaped {
        Escaped { chars: [c; 10], len: 1, next: 0 }
    }

    fn backslash(c: char) -> Escaped {
        let mut escaped = Escaped::literal('\\');
        escaped.chars[1] = c;
        escaped.len = 2;
        escaped
    }

    fn unicode(c: char) -> Escaped {
        let mut escaped = Escaped::literal('\\');
        escaped.chars[1] = 'u';
        escaped.chars[2] = '{';
        let code = c as u32;
        let mut len = 3;
        let mut shift = 20;
        while shift > 0 && (code >> shift) == 0 {
            shift -= 4;
        }
        loop {
            escaped.chars[len] = from_digit((code >> shift) & 15, 16).unwrap_or('0');
            len += 1;
            if shift == 0 {
                break;
            }
            shift -= 4;
        }
        escaped.chars[len] = '}';
        escaped.len = len + 1;
        escaped
    }

    fn remaining(&self) -> usize {
        self.len - self.next
    }
}

impl Iterator for Escaped {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        if self.next == self.len {
            return None;
        }
        self.next += 1;
        Some(self.chars[self.next - 1])
    }
}

/// What `{:?}` escapes besides the characters that are not printable.
pub(crate) struct DebugEscapes {
    pub grapheme_extended: bool,
    pub single_quote: bool,
    pub double_quote: bool,
}

impl char {
    /// How `{:?}` writes the character inside a literal.
    pub(crate) fn escape_debug_ext(self, escapes: DebugEscapes) -> EscapeDebug {
        let escaped = match self {
            '\0' => Escaped::backslash('0'),
            '\t' => Escaped::backslash('t'),
            '\r' => Escaped::backslash('r'),
            '\n' => Escaped::backslash('n'),
            '\\' => Escaped::backslash('\\'),
            '"' if escapes.double_quote => Escaped::backslash('"'),
            '\'' if escapes.single_quote => Escaped::backslash('\''),
            _ if escapes.grapheme_extended && std::unicode::is_grapheme_extended(self) => Escaped::unicode(self),
            _ if std::unicode::is_printable(self) => Escaped::literal(self),
            _ => Escaped::unicode(self),
        };
        EscapeDebug(escaped)
    }

    /// The character as `{:?}` writes it, without the quotes.
    pub fn escape_debug(self) -> EscapeDebug {
        self.escape_debug_ext(DebugEscapes { grapheme_extended: true, single_quote: true, double_quote: true })
    }

    /// The character as a Rust literal in ASCII: printable ASCII as itself,
    /// the usual backslash escapes, and `\u{..}` for everything else.
    pub fn escape_default(self) -> EscapeDefault {
        EscapeDefault(match self {
            '\t' => Escaped::backslash('t'),
            '\r' => Escaped::backslash('r'),
            '\n' => Escaped::backslash('n'),
            '\\' | '\'' | '"' => Escaped::backslash(self),
            '\x20'..='\x7e' => Escaped::literal(self),
            _ => Escaped::unicode(self),
        })
    }

    /// The character as `\u{..}`.
    pub fn escape_unicode(self) -> EscapeUnicode {
        EscapeUnicode(Escaped::unicode(self))
    }
}

/// Defines the iterators over the characters of one escaped character.
macro_rules! escape_iterator {
    ($($name:ident)*) => {
        $(
            #[derive(Clone, Debug)]
            pub struct $name(Escaped);

            impl Iterator for $name {
                type Item = char;

                fn next(&mut self) -> Option<char> {
                    self.0.next()
                }

                fn size_hint(&self) -> (usize, Option<usize>) {
                    (self.0.remaining(), Some(self.0.remaining()))
                }
            }

            impl std::iter::ExactSizeIterator for $name {
                fn len(&self) -> usize {
                    self.0.remaining()
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    for c in self.0.clone() {
                        f.write_char(c)?;
                    }
                    Ok(())
                }
            }
        )*
    };
}

escape_iterator!(EscapeDebug EscapeDefault EscapeUnicode);
