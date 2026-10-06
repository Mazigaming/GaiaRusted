//! String slices.
//!
//! Searching takes a [`Pattern`]: a `char`, a `&str` (or `&String`), a set
//! of `char`s as a slice, or a predicate on `char`s, as in Rust:
//! `"a,b;c".split(|c| c == ',' || c == ';')`.

/// Something a string can be searched for.
pub trait Pattern {
    /// The first match that starts at or after byte `from`, as the byte
    /// range it covers.
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)>;
    /// The last match that ends at or before byte `to`.
    fn rfind_in(&mut self, haystack: &str, to: usize) -> Option<(usize, usize)>;
    /// The length of the match at the very start of `haystack`, if any.
    fn prefix_of(&mut self, haystack: &str) -> Option<usize>;
    /// The length of the match at the very end of `haystack`, if any.
    fn suffix_of(&mut self, haystack: &str) -> Option<usize>;
}

/// The patterns that match one character at a time.
pub trait CharPredicate {
    fn matches_char(&mut self, c: char) -> bool;
}

impl CharPredicate for char {
    fn matches_char(&mut self, c: char) -> bool { *self == c }
}

impl CharPredicate for &[char] {
    fn matches_char(&mut self, c: char) -> bool { self.contains(&c) }
}

impl<F: FnMut(char) -> bool> CharPredicate for F {
    fn matches_char(&mut self, c: char) -> bool { self(c) }
}

impl<P: CharPredicate> Pattern for P {
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)> {
        let mut position = from;
        for c in haystack[from..].chars() {
            let next = position + c.len_utf8();
            if self.matches_char(c) {
                return Some((position, next));
            }
            position = next;
        }
        None
    }

    fn rfind_in(&mut self, haystack: &str, to: usize) -> Option<(usize, usize)> {
        let mut end = to;
        let mut chars = haystack[..to].chars();
        while let Some(c) = chars.next_back() {
            let start = end - c.len_utf8();
            if self.matches_char(c) {
                return Some((start, end));
            }
            end = start;
        }
        None
    }

    fn prefix_of(&mut self, haystack: &str) -> Option<usize> {
        let c = haystack.chars().next()?;
        if self.matches_char(c) { Some(c.len_utf8()) } else { None }
    }

    fn suffix_of(&mut self, haystack: &str) -> Option<usize> {
        let c = haystack.chars().next_back()?;
        if self.matches_char(c) { Some(c.len_utf8()) } else { None }
    }
}

impl Pattern for &str {
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)> {
        let needle = self.as_bytes();
        let bytes = haystack.as_bytes();
        if needle.len() > bytes.len() {
            return None;
        }
        let mut start = from;
        while start + needle.len() <= bytes.len() {
            // An empty needle matches at every character boundary.
            if (needle.is_empty() && haystack.is_char_boundary(start)) || (!needle.is_empty() && bytes[start..start + needle.len()] == *needle) {
                return Some((start, start + needle.len()));
            }
            start += 1;
        }
        None
    }

    fn rfind_in(&mut self, haystack: &str, to: usize) -> Option<(usize, usize)> {
        let needle = self.as_bytes();
        let bytes = haystack.as_bytes();
        if needle.len() > to {
            return None;
        }
        let mut start = to - needle.len();
        loop {
            if (needle.is_empty() && haystack.is_char_boundary(start)) || (!needle.is_empty() && bytes[start..start + needle.len()] == *needle) {
                return Some((start, start + needle.len()));
            }
            if start == 0 {
                return None;
            }
            start -= 1;
        }
    }

    fn prefix_of(&mut self, haystack: &str) -> Option<usize> {
        if haystack.as_bytes().starts_with(self.as_bytes()) { Some(self.len()) } else { None }
    }

    fn suffix_of(&mut self, haystack: &str) -> Option<usize> {
        if haystack.as_bytes().ends_with(self.as_bytes()) { Some(self.len()) } else { None }
    }
}

impl Pattern for &String {
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)> { self.as_str().find_in(haystack, from) }
    fn rfind_in(&mut self, haystack: &str, to: usize) -> Option<(usize, usize)> { self.as_str().rfind_in(haystack, to) }
    fn prefix_of(&mut self, haystack: &str) -> Option<usize> { self.as_str().prefix_of(haystack) }
    fn suffix_of(&mut self, haystack: &str) -> Option<usize> { self.as_str().suffix_of(haystack) }
}

impl Pattern for &&str {
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)> { (**self).find_in(haystack, from) }
    fn rfind_in(&mut self, haystack: &str, to: usize) -> Option<(usize, usize)> { (**self).rfind_in(haystack, to) }
    fn prefix_of(&mut self, haystack: &str) -> Option<usize> { (**self).prefix_of(haystack) }
    fn suffix_of(&mut self, haystack: &str) -> Option<usize> { (**self).suffix_of(haystack) }
}

impl str {
    /// Length in bytes.
    pub fn len(&self) -> usize {
        std::intrinsics::str_len(self)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_ptr(&self) -> *const u8 {
        std::intrinsics::str_as_ptr(self)
    }

    pub fn as_bytes(&self) -> &[u8] {
        std::intrinsics::slice_from_raw_parts(self.as_ptr(), self.len())
    }

    pub fn to_string(&self) -> String {
        String::from(self)
    }

    pub fn to_owned(&self) -> String {
        String::from(self)
    }

    /// The part of the string from byte `start` up to byte `end`.
    fn slice(&self, start: usize, end: usize) -> &str {
        if start > end || end > self.len() {
            panic!("byte range {}..{} is out of bounds of a string of length {}", start, end, self.len());
        }
        if !self.is_char_boundary(start) || !self.is_char_boundary(end) {
            panic!("byte index {} is not a char boundary", if self.is_char_boundary(start) { end } else { start });
        }
        std::intrinsics::str_from_raw_parts(self.as_ptr().add(start), end - start)
    }

    /// Does a character start at byte `index` (or the string end there)?
    pub fn is_char_boundary(&self, index: usize) -> bool {
        if index == 0 || index == self.len() {
            return true;
        }
        // Continuation bytes look like `10xxxxxx`.
        index < self.len() && (self.as_bytes()[index] & 0xC0) != 0x80
    }

    /// The part of the string in `range`, if it is one.
    pub fn get<R: std::ops::RangeBounds<usize>>(&self, range: R) -> Option<&str> {
        let (start, end) = std::ops::range_positions(&range, self.len());
        if start <= end && end <= self.len() && self.is_char_boundary(start) && self.is_char_boundary(end) {
            Some(std::intrinsics::str_from_raw_parts(self.as_ptr().add(start), end - start))
        } else {
            None
        }
    }

    // -- searching -----------------------------------------------------------

    pub fn find<P: Pattern>(&self, mut pattern: P) -> Option<usize> {
        pattern.find_in(self, 0).map(|(start, _)| start)
    }

    pub fn rfind<P: Pattern>(&self, mut pattern: P) -> Option<usize> {
        pattern.rfind_in(self, self.len()).map(|(start, _)| start)
    }

    pub fn contains<P: Pattern>(&self, mut pattern: P) -> bool {
        pattern.find_in(self, 0).is_some()
    }

    pub fn starts_with<P: Pattern>(&self, mut pattern: P) -> bool {
        pattern.prefix_of(self).is_some()
    }

    pub fn ends_with<P: Pattern>(&self, mut pattern: P) -> bool {
        pattern.suffix_of(self).is_some()
    }

    pub fn strip_prefix<P: Pattern>(&self, mut pattern: P) -> Option<&str> {
        pattern.prefix_of(self).map(|length| self.slice(length, self.len()))
    }

    pub fn strip_suffix<P: Pattern>(&self, mut pattern: P) -> Option<&str> {
        pattern.suffix_of(self).map(|length| self.slice(0, self.len() - length))
    }

    pub fn split_once<P: Pattern>(&self, mut pattern: P) -> Option<(&str, &str)> {
        let (start, end) = pattern.find_in(self, 0)?;
        Some((self.slice(0, start), self.slice(end, self.len())))
    }

    pub fn rsplit_once<P: Pattern>(&self, mut pattern: P) -> Option<(&str, &str)> {
        let (start, end) = pattern.rfind_in(self, self.len())?;
        Some((self.slice(0, start), self.slice(end, self.len())))
    }

    pub fn split<P: Pattern>(&self, pattern: P) -> Split<P> {
        Split { haystack: self, pattern, start: 0, search_from: 0, finished: false, keep_last_empty: true, inclusive: false }
    }

    /// Like `split`, but a separator at the very end does not make an empty
    /// last piece.
    pub fn split_terminator<P: Pattern>(&self, pattern: P) -> Split<P> {
        Split { haystack: self, pattern, start: 0, search_from: 0, finished: false, keep_last_empty: false, inclusive: false }
    }

    /// Like `split`, but each piece keeps the separator that ends it.
    pub fn split_inclusive<P: Pattern>(&self, pattern: P) -> Split<P> {
        Split { haystack: self, pattern, start: 0, search_from: 0, finished: false, keep_last_empty: false, inclusive: true }
    }

    pub fn rsplit<P: Pattern>(&self, pattern: P) -> RSplit<P> {
        RSplit { haystack: self, pattern, end: self.len(), finished: false }
    }

    /// At most `count` pieces; the last is the rest of the string.
    pub fn splitn<P: Pattern>(&self, count: usize, pattern: P) -> SplitN<P> {
        SplitN { inner: self.split(pattern), remaining: count }
    }

    pub fn rsplitn<P: Pattern>(&self, count: usize, pattern: P) -> RSplitN<P> {
        RSplitN { inner: self.rsplit(pattern), remaining: count }
    }

    /// The pieces separated by white space, without empty ones.
    pub fn split_whitespace(&self) -> SplitWhitespace {
        SplitWhitespace { rest: self, ascii_only: false }
    }

    /// Like [`split_whitespace`](str::split_whitespace), splitting only at
    /// ASCII whitespace.
    pub fn split_ascii_whitespace(&self) -> SplitWhitespace {
        SplitWhitespace { rest: self, ascii_only: true }
    }

    /// The lines, each without its `\n` or `\r\n`; a final line ending does
    /// not start an empty line.
    pub fn lines(&self) -> Lines {
        Lines { rest: self }
    }

    pub fn matches<P: Pattern>(&self, pattern: P) -> Matches<P> {
        Matches { haystack: self, pattern, from: 0 }
    }

    pub fn match_indices<P: Pattern>(&self, pattern: P) -> MatchIndices<P> {
        MatchIndices { haystack: self, pattern, from: 0 }
    }

    pub fn replace<P: Pattern>(&self, pattern: P, to: &str) -> String {
        self.replacen(pattern, to, usize::MAX)
    }

    pub fn replacen<P: Pattern>(&self, mut pattern: P, to: &str, count: usize) -> String {
        let mut result = String::with_capacity(self.len());
        let mut copied = 0;
        let mut search_from = 0;
        let mut replaced = 0;
        while replaced < count && search_from <= self.len() {
            let Some((start, end)) = pattern.find_in(self, search_from) else { break };
            result.push_str(self.slice(copied, start));
            result.push_str(to);
            copied = end;
            replaced += 1;
            search_from = if end > start { end } else { self.next_boundary(end) };
        }
        result.push_str(self.slice(copied, self.len()));
        result
    }

    /// The boundary after the character at byte `index`.
    fn next_boundary(&self, index: usize) -> usize {
        let mut next = index + 1;
        while next < self.len() && !self.is_char_boundary(next) {
            next += 1;
        }
        next
    }

    // -- trimming ------------------------------------------------------------

    pub fn trim(&self) -> &str {
        self.trim_matches(char::is_whitespace)
    }

    pub fn trim_start(&self) -> &str {
        self.trim_start_matches(char::is_whitespace)
    }

    pub fn trim_end(&self) -> &str {
        self.trim_end_matches(char::is_whitespace)
    }

    pub fn trim_matches<P: Pattern>(&self, mut pattern: P) -> &str {
        let mut rest = self;
        while let Some(length) = pattern.prefix_of(rest) {
            if length == 0 {
                break;
            }
            rest = rest.slice(length, rest.len());
        }
        while let Some(length) = pattern.suffix_of(rest) {
            if length == 0 {
                break;
            }
            rest = rest.slice(0, rest.len() - length);
        }
        rest
    }

    pub fn trim_start_matches<P: Pattern>(&self, mut pattern: P) -> &str {
        let mut rest = self;
        while let Some(length) = pattern.prefix_of(rest) {
            if length == 0 {
                break;
            }
            rest = rest.slice(length, rest.len());
        }
        rest
    }

    pub fn trim_end_matches<P: Pattern>(&self, mut pattern: P) -> &str {
        let mut rest = self;
        while let Some(length) = pattern.suffix_of(rest) {
            if length == 0 {
                break;
            }
            rest = rest.slice(0, rest.len() - length);
        }
        rest
    }

    // -- characters and case -------------------------------------------------

    pub fn chars(&self) -> Chars {
        Chars { rest: self }
    }

    /// The string as `{:?}` writes it, without the quotes.
    pub fn escape_debug(&self) -> EscapeDebug {
        EscapeDebug { chars: self.chars(), current: None, first: true }
    }

    /// Each character as [`char::escape_default`] writes it.
    pub fn escape_default(&self) -> EscapeDefault {
        EscapeDefault { chars: self.chars(), current: None }
    }

    /// Each character as [`char::escape_unicode`] writes it.
    pub fn escape_unicode(&self) -> EscapeUnicode {
        EscapeUnicode { chars: self.chars(), current: None }
    }

    pub fn char_indices(&self) -> CharIndices {
        CharIndices { rest: self, offset: 0 }
    }

    pub fn bytes(&self) -> std::iter::Copied<std::slice::Iter<u8>> {
        self.as_bytes().iter().copied()
    }

    pub fn to_uppercase(&self) -> String {
        let mut result = String::with_capacity(self.len());
        for c in self.chars() {
            for upper in c.to_uppercase() {
                result.push(upper);
            }
        }
        result
    }

    /// Lower case, with a capital sigma that ends a word becoming `ς`.
    pub fn to_lowercase(&self) -> String {
        let mut result = String::with_capacity(self.len());
        for (at, c) in self.char_indices() {
            if c == '\u{3A3}' {
                result.push(lower_sigma(self, at));
            } else {
                for lower in c.to_lowercase() {
                    result.push(lower);
                }
            }
        }
        result
    }

    pub fn to_ascii_uppercase(&self) -> String {
        let mut result = String::with_capacity(self.len());
        for c in self.chars() {
            result.push(c.to_ascii_uppercase());
        }
        result
    }

    pub fn to_ascii_lowercase(&self) -> String {
        let mut result = String::with_capacity(self.len());
        for c in self.chars() {
            result.push(c.to_ascii_lowercase());
        }
        result
    }

    /// Changing ASCII letters keeps the bytes valid UTF-8.
    pub fn make_ascii_uppercase(&mut self) {
        unsafe { std::intrinsics::slice_from_raw_parts_mut(self.as_ptr() as *mut u8, self.len()) }.make_ascii_uppercase();
    }

    pub fn make_ascii_lowercase(&mut self) {
        unsafe { std::intrinsics::slice_from_raw_parts_mut(self.as_ptr() as *mut u8, self.len()) }.make_ascii_lowercase();
    }

    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {
        let (a, b) = (self.as_bytes(), other.as_bytes());
        if a.len() != b.len() {
            return false;
        }
        let mut index = 0;
        while index < a.len() {
            if (a[index] as char).to_ascii_lowercase() != (b[index] as char).to_ascii_lowercase() {
                return false;
            }
            index += 1;
        }
        true
    }

    pub fn is_ascii(&self) -> bool {
        self.bytes().all(|byte| byte < 128)
    }

    pub fn repeat(&self, count: usize) -> String {
        let mut result = String::with_capacity(self.len() * count);
        let mut done = 0;
        while done < count {
            result.push_str(self);
            done += 1;
        }
        result
    }

    pub fn parse<F: FromStr>(&self) -> Result<F, F::Err> {
        FromStr::from_str(self)
    }
}

/// Iterator over the characters of a string, decoding UTF-8, from either end.
#[derive(Clone)]
pub struct Chars {
    rest: &str,
}

impl Chars {
    /// What has not been iterated over yet.
    pub fn as_str(&self) -> &str {
        self.rest
    }
}

impl Iterator for Chars {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        let bytes = self.rest.as_bytes();
        if bytes.len() == 0 {
            return None;
        }
        let first = bytes[0] as u32;
        let (length, mut code) = if first < 128 {
            (1, first)
        } else if first < 224 {
            (2, first & 31)
        } else if first < 240 {
            (3, first & 15)
        } else {
            (4, first & 7)
        };
        let mut index = 1;
        while index < length {
            code = (code << 6) | (bytes[index] as u32 & 63);
            index += 1;
        }
        self.rest = std::intrinsics::str_from_raw_parts(self.rest.as_ptr().add(length), self.rest.len() - length);
        Some(unsafe { std::char::from_u32_unchecked(code) })
    }

    fn count(self) -> usize {
        let mut count = 0;
        for byte in self.rest.bytes() {
            if (byte & 0xC0) != 0x80 {
                count += 1;
            }
        }
        count
    }
}

impl std::iter::DoubleEndedIterator for Chars {
    fn next_back(&mut self) -> Option<char> {
        let bytes = self.rest.as_bytes();
        if bytes.len() == 0 {
            return None;
        }
        // Back up to the first byte of the last character.
        let mut start = bytes.len() - 1;
        while start > 0 && (bytes[start] & 0xC0) == 0x80 {
            start -= 1;
        }
        let tail = std::intrinsics::str_from_raw_parts(self.rest.as_ptr().add(start), bytes.len() - start);
        let c = tail.chars().next().expect("a character starts there");
        self.rest = std::intrinsics::str_from_raw_parts(self.rest.as_ptr(), start);
        Some(c)
    }
}

/// Iterator over the characters of a string with their byte positions.
pub struct CharIndices {
    rest: &str,
    offset: usize,
}

impl Iterator for CharIndices {
    type Item = (usize, char);

    fn next(&mut self) -> Option<(usize, char)> {
        let c = self.rest.chars().next()?;
        let position = self.offset;
        let length = c.len_utf8();
        self.rest = std::intrinsics::str_from_raw_parts(self.rest.as_ptr().add(length), self.rest.len() - length);
        self.offset += length;
        Some((position, c))
    }
}

impl std::iter::DoubleEndedIterator for CharIndices {
    fn next_back(&mut self) -> Option<(usize, char)> {
        let mut chars = self.rest.chars();
        let c = chars.next_back()?;
        let remaining = chars.as_str().len();
        self.rest = std::intrinsics::str_from_raw_parts(self.rest.as_ptr(), remaining);
        Some((self.offset + remaining, c))
    }
}

/// The pieces of a string between the matches of a pattern.
pub struct Split<P> {
    haystack: &str,
    pattern: P,
    /// Where the next piece begins.
    start: usize,
    /// Where to look for the next match; past `start` after an empty match.
    search_from: usize,
    finished: bool,
    /// Whether an empty piece after the last separator counts.
    keep_last_empty: bool,
    /// Whether each piece keeps its separator.
    inclusive: bool,
}

impl<P: Pattern> Iterator for Split<P> {
    type Item = &str;

    fn next(&mut self) -> Option<&str> {
        if self.finished {
            return None;
        }
        let found = if self.search_from <= self.haystack.len() { self.pattern.find_in(self.haystack, self.search_from) } else { None };
        match found {
            Some((match_start, match_end)) => {
                let piece_end = if self.inclusive { match_end } else { match_start };
                let piece = self.haystack.slice(self.start, piece_end);
                self.start = match_end;
                self.search_from = if match_end > match_start { match_end } else { self.haystack.next_boundary(match_end) };
                Some(piece)
            }
            None => {
                self.finished = true;
                if !self.keep_last_empty && self.start == self.haystack.len() {
                    return None;
                }
                Some(self.haystack.slice(self.start, self.haystack.len()))
            }
        }
    }
}

/// The pieces between matches, from the end.
pub struct RSplit<P> {
    haystack: &str,
    pattern: P,
    end: usize,
    finished: bool,
}

impl<P: Pattern> Iterator for RSplit<P> {
    type Item = &str;

    fn next(&mut self) -> Option<&str> {
        if self.finished {
            return None;
        }
        match self.pattern.rfind_in(self.haystack, self.end) {
            Some((match_start, match_end)) if match_end > match_start || match_start < self.end => {
                let piece = self.haystack.slice(match_end, self.end);
                self.end = match_start;
                Some(piece)
            }
            _ => {
                self.finished = true;
                Some(self.haystack.slice(0, self.end))
            }
        }
    }
}

pub struct SplitN<P> {
    inner: Split<P>,
    remaining: usize,
}

impl<P: Pattern> Iterator for SplitN<P> {
    type Item = &str;

    fn next(&mut self) -> Option<&str> {
        match self.remaining {
            0 => None,
            1 => {
                self.remaining = 0;
                if self.inner.finished {
                    return None;
                }
                self.inner.finished = true;
                Some(self.inner.haystack.slice(self.inner.start, self.inner.haystack.len()))
            }
            _ => {
                self.remaining -= 1;
                self.inner.next()
            }
        }
    }
}

pub struct RSplitN<P> {
    inner: RSplit<P>,
    remaining: usize,
}

impl<P: Pattern> Iterator for RSplitN<P> {
    type Item = &str;

    fn next(&mut self) -> Option<&str> {
        match self.remaining {
            0 => None,
            1 => {
                self.remaining = 0;
                if self.inner.finished {
                    return None;
                }
                self.inner.finished = true;
                Some(self.inner.haystack.slice(0, self.inner.end))
            }
            _ => {
                self.remaining -= 1;
                self.inner.next()
            }
        }
    }
}

pub struct SplitWhitespace {
    rest: &str,
    ascii_only: bool,
}

impl SplitWhitespace {
    /// The length of the whitespace character at byte `at` of what is
    /// left, or 0 if the character there is not whitespace.
    fn space_at(&self, at: usize) -> usize {
        let byte = self.rest.as_bytes()[at];
        if byte < 0x80 {
            let space = matches!(byte, b' ' | b'\t' | b'\n' | 0x0c | b'\r') || (byte == 0x0b && !self.ascii_only);
            return space as usize;
        }
        self.wide_space_at(at)
    }

    /// [`space_at`](Self::space_at) for a character beyond ASCII: rare, so
    /// kept apart for the common case to stay small.
    fn wide_space_at(&self, at: usize) -> usize {
        if self.ascii_only {
            return 0;
        }
        match self.rest.slice(at, self.rest.len()).chars().next() {
            Some(c) if c.is_whitespace() => c.len_utf8(),
            _ => 0,
        }
    }
}

/// The length of the UTF-8 encoded character that starts with `lead`.
fn encoded_len(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if lead < 0xe0 {
        2
    } else if lead < 0xf0 {
        3
    } else {
        4
    }
}

impl Iterator for SplitWhitespace {
    type Item = &str;

    fn next(&mut self) -> Option<&str> {
        let len = self.rest.len();
        let mut start = 0;
        loop {
            if start == len {
                self.rest = self.rest.slice(len, len);
                return None;
            }
            let space = self.space_at(start);
            if space == 0 {
                break;
            }
            start += space;
        }
        let mut end = start;
        while end < len && self.space_at(end) == 0 {
            end += encoded_len(self.rest.as_bytes()[end]);
        }
        let word = self.rest.slice(start, end);
        self.rest = self.rest.slice(end, len);
        Some(word)
    }
}

impl std::iter::DoubleEndedIterator for SplitWhitespace {
    fn next_back(&mut self) -> Option<&str> {
        let trimmed = self.rest.trim_end();
        if trimmed.is_empty() {
            self.rest = trimmed;
            return None;
        }
        let start = match trimmed.rfind(char::is_whitespace) {
            Some(position) => position + trimmed.slice(position, trimmed.len()).chars().next().map_or(1, |c| c.len_utf8()),
            None => 0,
        };
        self.rest = trimmed.slice(0, start);
        Some(trimmed.slice(start, trimmed.len()))
    }
}

pub struct Lines {
    rest: &str,
}

impl Iterator for Lines {
    type Item = &str;

    fn next(&mut self) -> Option<&str> {
        if self.rest.is_empty() {
            return None;
        }
        let (line, rest) = match self.rest.find('\n') {
            Some(end) => (self.rest.slice(0, end), self.rest.slice(end + 1, self.rest.len())),
            None => (self.rest, self.rest.slice(self.rest.len(), self.rest.len())),
        };
        self.rest = rest;
        Some(line.strip_suffix('\r').unwrap_or(line))
    }
}

/// The matches of a pattern, as the text they cover.
pub struct Matches<P> {
    haystack: &str,
    pattern: P,
    from: usize,
}

impl<P: Pattern> Iterator for Matches<P> {
    type Item = &str;

    fn next(&mut self) -> Option<&str> {
        if self.from > self.haystack.len() {
            return None;
        }
        let (start, end) = self.pattern.find_in(self.haystack, self.from)?;
        self.from = if end > start { end } else { self.haystack.next_boundary(end) };
        Some(self.haystack.slice(start, end))
    }
}

/// The matches of a pattern, with where each starts.
pub struct MatchIndices<P> {
    haystack: &str,
    pattern: P,
    from: usize,
}

impl<P: Pattern> Iterator for MatchIndices<P> {
    type Item = (usize, &str);

    fn next(&mut self) -> Option<(usize, &str)> {
        if self.from > self.haystack.len() {
            return None;
        }
        let (start, end) = self.pattern.find_in(self.haystack, self.from)?;
        self.from = if end > start { end } else { self.haystack.next_boundary(end) };
        Some((start, self.haystack.slice(start, end)))
    }
}

/// Bytes that are not valid UTF-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utf8Error {
    valid_up_to: usize,
}

impl Utf8Error {
    pub fn valid_up_to(&self) -> usize {
        self.valid_up_to
    }
}

impl std::fmt::Display for Utf8Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "invalid utf-8 sequence of 1 bytes from index {}", self.valid_up_to)
    }
}

/// How many bytes of `bytes` are valid UTF-8 from the start.
pub fn utf8_prefix(bytes: &[u8]) -> usize {
    let mut index = 0;
    while index < bytes.len() {
        let first = bytes[index];
        let length = if first < 0x80 {
            1
        } else if first >= 0xC2 && first < 0xE0 {
            2
        } else if first >= 0xE0 && first < 0xF0 {
            3
        } else if first >= 0xF0 && first < 0xF5 {
            4
        } else {
            return index;
        };
        if index + length > bytes.len() {
            return index;
        }
        let mut continuation = 1;
        while continuation < length {
            if (bytes[index + continuation] & 0xC0) != 0x80 {
                return index;
            }
            continuation += 1;
        }
        index += length;
    }
    index
}

/// The bytes as a string, if they are valid UTF-8.
pub fn from_utf8(bytes: &[u8]) -> Result<&str, Utf8Error> {
    let valid = utf8_prefix(bytes);
    if valid == bytes.len() {
        Ok(std::intrinsics::str_from_raw_parts(bytes.as_ptr(), bytes.len()))
    } else {
        Err(Utf8Error { valid_up_to: valid })
    }
}

impl PartialEq for str {
    fn eq(&self, other: &str) -> bool {
        self.len() == other.len()
            && unsafe { std::libc::memcmp(self.as_ptr(), other.as_ptr(), self.len()) } == 0
    }
}

impl Eq for str {}

impl PartialOrd for str {
    fn partial_cmp(&self, other: &str) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for str {
    fn cmp(&self, other: &str) -> std::cmp::Ordering {
        let shorter = if self.len() < other.len() { self.len() } else { other.len() };
        let difference = unsafe { std::libc::memcmp(self.as_ptr(), other.as_ptr(), shorter) };
        if difference < 0 {
            std::cmp::Ordering::Less
        } else if difference > 0 {
            std::cmp::Ordering::Greater
        } else {
            Ord::cmp(&self.len(), &other.len())
        }
    }
}

impl std::ops::Index<std::ops::Range<usize>> for str {
    type Output = str;
    fn index(&self, range: std::ops::Range<usize>) -> &str {
        self.slice(range.start, range.end)
    }
}

impl std::ops::Index<std::ops::RangeFrom<usize>> for str {
    type Output = str;
    fn index(&self, range: std::ops::RangeFrom<usize>) -> &str {
        self.slice(range.start, self.len())
    }
}

impl std::ops::Index<std::ops::RangeTo<usize>> for str {
    type Output = str;
    fn index(&self, range: std::ops::RangeTo<usize>) -> &str {
        self.slice(0, range.end)
    }
}

impl std::ops::Index<std::ops::RangeInclusive<usize>> for str {
    type Output = str;
    fn index(&self, range: std::ops::RangeInclusive<usize>) -> &str {
        self.slice(*range.start(), *range.end() + 1)
    }
}

impl std::ops::Index<std::ops::RangeToInclusive<usize>> for str {
    type Output = str;
    fn index(&self, range: std::ops::RangeToInclusive<usize>) -> &str {
        self.slice(0, range.end + 1)
    }
}

impl std::ops::Index<std::ops::RangeFull> for str {
    type Output = str;
    fn index(&self, range: std::ops::RangeFull) -> &str {
        self
    }
}

/// Parsing a value from text.
pub trait FromStr {
    type Err;
    fn from_str(text: &str) -> Result<Self, Self::Err>;
}

/// Text that is not a floating-point number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFloatError {
    kind: FloatErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatErrorKind {
    Empty,
    Invalid,
}

impl std::fmt::Display for ParseFloatError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.pad(match self.kind {
            FloatErrorKind::Empty => "cannot parse float from empty string",
            FloatErrorKind::Invalid => "invalid float literal",
        })
    }
}

/// Is `text` a float as Rust writes them: an optional sign, then `inf`,
/// `infinity` or `nan` in any case, or decimal digits with an optional
/// fraction and exponent (`1`, `1.`, `.5`, `2.5e-3`)?
fn is_float_literal(text: &str) -> bool {
    let unsigned = text.strip_prefix(|c| c == '+' || c == '-').unwrap_or(text);
    let lower = unsigned.to_ascii_lowercase();
    if lower == "inf" || lower == "infinity" || lower == "nan" {
        return true;
    }
    let bytes = unsigned.as_bytes();
    let digits_from = |mut at: usize| {
        let start = at;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            at += 1;
        }
        (at, at - start)
    };
    let (mut at, whole) = digits_from(0);
    let mut fraction = 0;
    if at < bytes.len() && bytes[at] == b'.' {
        (at, fraction) = digits_from(at + 1);
    }
    if whole + fraction == 0 {
        return false;
    }
    if at < bytes.len() && (bytes[at] == b'e' || bytes[at] == b'E') {
        at += 1;
        if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {
            at += 1;
        }
        let (end, exponent) = digits_from(at);
        if exponent == 0 {
            return false;
        }
        at = end;
    }
    at == bytes.len()
}

/// Defines `FromStr` for a float type with the C function that converts
/// text to it, correctly rounded.
macro_rules! float_from_str {
    ($($t:ident $convert:ident)*) => {
        $(
            impl FromStr for $t {
                type Err = ParseFloatError;

                fn from_str(text: &str) -> Result<$t, ParseFloatError> {
                    if text.is_empty() {
                        return Err(ParseFloatError { kind: FloatErrorKind::Empty });
                    }
                    if !is_float_literal(text) {
                        return Err(ParseFloatError { kind: FloatErrorKind::Invalid });
                    }
                    // The C function needs a NUL-terminated copy.
                    let mut copy = String::from(text);
                    copy.push_byte(0);
                    Ok(unsafe { std::libc::$convert(copy.as_ptr(), std::ptr::null_mut()) })
                }
            }
        )*
    };
}

float_from_str!(f64 strtod f32 strtof);

/// Text that is not exactly one character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseCharError {
    kind: CharErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharErrorKind {
    EmptyString,
    TooManyChars,
}

impl std::fmt::Display for ParseCharError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.pad(match self.kind {
            CharErrorKind::EmptyString => "cannot parse char from empty string",
            CharErrorKind::TooManyChars => "too many characters in string",
        })
    }
}

impl FromStr for char {
    type Err = ParseCharError;

    fn from_str(text: &str) -> Result<char, ParseCharError> {
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (None, _) => Err(ParseCharError { kind: CharErrorKind::EmptyString }),
            (Some(c), None) => Ok(c),
            _ => Err(ParseCharError { kind: CharErrorKind::TooManyChars }),
        }
    }
}

/// Text that is neither `true` nor `false`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseBoolError;

impl std::fmt::Display for ParseBoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.pad("provided string was not `true` or `false`")
    }
}

impl FromStr for bool {
    type Err = ParseBoolError;

    fn from_str(text: &str) -> Result<bool, ParseBoolError> {
        match text {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(ParseBoolError),
        }
    }
}

impl FromStr for String {
    type Err = std::convert::Infallible;

    fn from_str(text: &str) -> Result<String, std::convert::Infallible> {
        Ok(String::from(text))
    }
}

/// The characters of a string as `{:?}` writes them. Only a combining
/// mark at the very start is escaped: elsewhere it combines with the
/// character before it, as in the string itself.
#[derive(Clone)]
pub struct EscapeDebug {
    chars: Chars,
    current: Option<std::char::EscapeDebug>,
    first: bool,
}

impl Iterator for EscapeDebug {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        loop {
            if let Some(c) = self.current.as_mut().and_then(|escaped| escaped.next()) {
                return Some(c);
            }
            let next = self.chars.next()?;
            let escapes = std::char::DebugEscapes { grapheme_extended: self.first, single_quote: true, double_quote: true };
            self.first = false;
            self.current = Some(next.escape_debug_ext(escapes));
        }
    }
}

/// Defines the iterators over a string's characters escaped one by one.
macro_rules! str_escape_iterator {
    ($($name:ident by $method:ident)*) => {
        $(
            #[derive(Clone)]
            pub struct $name {
                chars: Chars,
                current: Option<std::char::$name>,
            }

            impl Iterator for $name {
                type Item = char;

                fn next(&mut self) -> Option<char> {
                    loop {
                        if let Some(c) = self.current.as_mut().and_then(|escaped| escaped.next()) {
                            return Some(c);
                        }
                        self.current = Some(self.chars.next()?.$method());
                    }
                }
            }
        )*
    };
}

str_escape_iterator!(EscapeDefault by escape_default EscapeUnicode by escape_unicode);

/// Defines `Display` for the string escaping iterators.
macro_rules! str_escape_display {
    ($($name:ident)*) => {
        $(
            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    for c in self.clone() {
                        f.write_char(c)?;
                    }
                    Ok(())
                }
            }
        )*
    };
}

str_escape_display!(EscapeDebug EscapeDefault EscapeUnicode);

/// The lower case of the capital sigma at byte `at` of `text`: `ς` where it
/// ends a word (a cased letter before it, none after it, looking past
/// characters that do not count for case), else `σ`.
fn lower_sigma(text: &str, at: usize) -> char {
    let before = text.slice(0, at).chars().rev();
    let after = text.slice(at + 2, text.len()).chars();
    if case_ignorable_then_cased(before) && !case_ignorable_then_cased(after) {
        '\u{3C2}'
    } else {
        '\u{3C3}'
    }
}

/// Is the first character that is not case-ignorable a cased one?
fn case_ignorable_then_cased<I: Iterator<Item = char>>(chars: I) -> bool {
    let mut chars = chars;
    loop {
        match chars.next() {
            Some(c) if std::unicode::case_ignorable(c) => {}
            Some(c) => return std::unicode::cased(c),
            None => return false,
        }
    }
}
