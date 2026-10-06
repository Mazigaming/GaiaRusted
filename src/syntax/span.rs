//! Source locations.
//!
//! Every token and AST node remembers the byte range it came from, so an
//! error found deep in type checking can still point at the exact source text.

use std::path::{Path, PathBuf};

/// Index of a file inside a [`SourceMap`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct FileId(pub u32);

/// A half-open byte range `lo..hi` within one source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub file: FileId,
    pub lo: u32,
    pub hi: u32,
}

impl Span {
    pub fn new(file: FileId, lo: usize, hi: usize) -> Span {
        Span { file, lo: lo as u32, hi: hi as u32 }
    }

    /// The smallest span covering both `self` and `other`.
    pub fn to(self, other: Span) -> Span {
        Span { file: self.file, lo: self.lo.min(other.lo), hi: self.hi.max(other.hi) }
    }
}

pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
    /// Byte offset of the first character of every line.
    line_starts: Vec<u32>,
}

impl SourceFile {
    fn new(path: PathBuf, text: String) -> SourceFile {
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i as u32 + 1))
            .collect();
        SourceFile { path, text, line_starts }
    }

    /// Zero-based line index containing byte `offset`.
    fn line_index(&self, offset: u32) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next_line) => next_line - 1,
        }
    }

    fn line_text(&self, line: usize) -> &str {
        let start = self.line_starts[line] as usize;
        let end = self
            .line_starts
            .get(line + 1)
            .map_or(self.text.len(), |&next| next as usize);
        self.text[start..end].trim_end_matches(['\n', '\r'])
    }
}

/// A resolved position, ready to show to a person (both fields are 1-based).
pub struct Location<'a> {
    pub path: &'a Path,
    pub line: usize,
    pub column: usize,
    pub line_text: &'a str,
    /// How many columns of `line_text` the span covers (at least one).
    pub width: usize,
}

/// All source text of a compilation, addressable by [`Span`].
#[derive(Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> SourceMap {
        SourceMap::default()
    }

    pub fn add_file(&mut self, path: impl Into<PathBuf>, text: String) -> FileId {
        self.files.push(SourceFile::new(path.into(), text));
        FileId(self.files.len() as u32 - 1)
    }

    pub fn file(&self, id: FileId) -> &SourceFile {
        &self.files[id.0 as usize]
    }

    pub fn snippet(&self, span: Span) -> &str {
        &self.file(span.file).text[span.lo as usize..span.hi as usize]
    }

    pub fn locate(&self, span: Span) -> Location<'_> {
        let file = self.file(span.file);
        let line = file.line_index(span.lo);
        let line_text = file.line_text(line);
        let line_start = file.line_starts[line] as usize;
        let column = file.text[line_start..span.lo as usize].chars().count();
        let rest_of_line = line_text.chars().count().saturating_sub(column);
        let span_chars = file.text[span.lo as usize..span.hi as usize].chars().count();
        Location {
            path: &file.path,
            line: line + 1,
            column: column + 1,
            line_text,
            width: span_chars.clamp(1, rest_of_line.max(1)),
        }
    }
}
