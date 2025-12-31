//! Source code span tracking for error reporting
//!
//! Provides location tracking for tokens and AST nodes to enable
//! precise error messages with line and column information.

use std::fmt;

/// Represents a location in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (0-indexed, character position on line)
    pub column: usize,
    /// Byte offset from start of file
    pub byte_offset: usize,
    /// Length of token in bytes
    pub byte_length: usize,
}

impl Span {
    /// Create a new span
    pub fn new(line: usize, column: usize, byte_offset: usize, byte_length: usize) -> Self {
        Span {
            line,
            column,
            byte_offset,
            byte_length,
        }
    }

    /// Create a span from a single position
    pub fn at(line: usize, column: usize, byte_offset: usize) -> Self {
        Span::new(line, column, byte_offset, 1)
    }

    /// Create a span covering a range
    pub fn range(start: Span, end: Span) -> Self {
        Span {
            line: start.line,
            column: start.column,
            byte_offset: start.byte_offset,
            byte_length: (end.byte_offset + end.byte_length) - start.byte_offset,
        }
    }

    /// Get the end position of this span
    pub fn end(&self) -> Span {
        Span {
            line: self.line,
            column: self.column + self.byte_length,
            byte_offset: self.byte_offset + self.byte_length,
            byte_length: 0,
        }
    }

    /// Check if this span contains another span
    pub fn contains(&self, other: &Span) -> bool {
        self.byte_offset <= other.byte_offset
            && (self.byte_offset + self.byte_length)
                >= (other.byte_offset + other.byte_length)
    }

    /// Merge two spans into one covering both
    pub fn merge(&self, other: &Span) -> Span {
        let start_offset = self.byte_offset.min(other.byte_offset);
        let end_offset = (self.byte_offset + self.byte_length)
            .max(other.byte_offset + other.byte_length);

        let start_line = self.line.min(other.line);
        let start_column = if self.line < other.line {
            self.column
        } else if other.line < self.line {
            other.column
        } else {
            self.column.min(other.column)
        };

        Span {
            line: start_line,
            column: start_column,
            byte_offset: start_offset,
            byte_length: end_offset - start_offset,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column + 1)
    }
}

/// Context for finding lines in source code
pub struct SourceMap {
    source: String,
    line_starts: Vec<usize>, // Byte offset of each line start
}

impl SourceMap {
    /// Create a new source map from source code
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, ch) in source.bytes().enumerate() {
            if ch == b'\n' {
                line_starts.push(i + 1);
            }
        }
        SourceMap {
            source: source.to_string(),
            line_starts,
        }
    }

    /// Get the line number at a byte offset
    pub fn line_at(&self, byte_offset: usize) -> usize {
        // Find the line by looking at line_starts
        // line_starts[i] contains the byte offset where line i+1 starts
        for (i, &start) in self.line_starts.iter().enumerate() {
            if start > byte_offset {
                // byte_offset is before this line start, so it's on the previous line
                return i; // i is 1-indexed (since line_starts[0] is line 1)
            }
        }
        // If we're here, byte_offset is on the last line
        self.line_starts.len()
    }

    /// Get the column number at a byte offset within a line
    pub fn column_at(&self, byte_offset: usize) -> usize {
        let line = self.line_at(byte_offset);
        let line_start = self.line_starts[line - 1];
        byte_offset - line_start
    }

    /// Get the source code for a line (1-indexed)
    pub fn get_line(&self, line_num: usize) -> Option<&str> {
        if line_num < 1 || line_num > self.line_starts.len() {
            return None;
        }

        let start = self.line_starts[line_num - 1];
        let end = if line_num < self.line_starts.len() {
            self.line_starts[line_num] - 1 // Exclude newline
        } else {
            self.source.len()
        };

        Some(&self.source[start..end])
    }

    /// Get lines around a span for context
    pub fn get_context(&self, span: Span, context_lines: usize) -> Vec<(usize, String)> {
        let start_line = span.line.saturating_sub(context_lines).max(1);
        let end_line = (span.line + context_lines).min(self.line_starts.len());

        let mut lines = Vec::new();
        for line_num in start_line..=end_line {
            if let Some(line_text) = self.get_line(line_num) {
                lines.push((line_num, line_text.to_string()));
            }
        }
        lines
    }

    /// Get span info at a byte offset
    pub fn span_at(&self, byte_offset: usize, length: usize) -> Span {
        let line = self.line_at(byte_offset);
        let column = self.column_at(byte_offset);
        Span::new(line, column, byte_offset, length)
    }

    /// Get the total number of lines
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let span = Span::new(1, 5, 10, 5);
        assert_eq!(span.line, 1);
        assert_eq!(span.column, 5);
        assert_eq!(span.byte_offset, 10);
        assert_eq!(span.byte_length, 5);
    }

    #[test]
    fn test_span_display() {
        let span = Span::new(5, 10, 0, 0);
        assert_eq!(span.to_string(), "5:11");
    }

    #[test]
    fn test_source_map_lines() {
        let source = "line 1\nline 2\nline 3";
        let map = SourceMap::new(source);

        assert_eq!(map.get_line(1), Some("line 1"));
        assert_eq!(map.get_line(2), Some("line 2"));
        assert_eq!(map.get_line(3), Some("line 3"));
        assert_eq!(map.get_line(4), None);
    }

    #[test]
    fn test_source_map_line_at() {
        let source = "line 1\nline 2\nline 3";
        let map = SourceMap::new(source);

        assert_eq!(map.line_at(0), 1);
        assert_eq!(map.line_at(7), 2);
        assert_eq!(map.line_at(14), 3);
    }

    #[test]
    fn test_source_map_context() {
        let source = "line 1\nline 2\nline 3\nline 4\nline 5";
        let map = SourceMap::new(source);

        let span = Span::at(3, 0, 14);
        let context = map.get_context(span, 1);
        
        assert_eq!(context.len(), 3); // Lines 2, 3, 4
        assert_eq!(context[0].0, 2);
        assert_eq!(context[1].0, 3);
        assert_eq!(context[2].0, 4);
    }
}
