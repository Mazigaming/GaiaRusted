//! Compiler diagnostics: an error message tied to the source text it is about.

use super::span::{SourceMap, Span};
use std::fmt::Write;

/// A compile error. Every stage of the pipeline reports failures as one of these.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub message: String,
    pub span: Option<Span>,
    /// Extra context lines shown under the snippet (`= note: ...`).
    pub notes: Vec<String>,
}

pub type Result<T> = std::result::Result<T, Diagnostic>;

impl Diagnostic {
    pub fn new(span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic { message: message.into(), span: Some(span), notes: Vec::new() }
    }

    /// An error with no useful source position (missing `main`, linker failure, ...).
    pub fn global(message: impl Into<String>) -> Diagnostic {
        Diagnostic { message: message.into(), span: None, notes: Vec::new() }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Diagnostic {
        self.notes.push(note.into());
        self
    }

    /// Format the way rustc does:
    ///
    /// ```text
    /// error: mismatched types: expected `i64`, found `&str`
    ///  --> demo.rs:3:18
    ///   |
    /// 3 |     let x: i64 = "three";
    ///   |                  ^^^^^^^
    /// ```
    pub fn render(&self, sources: &SourceMap) -> String {
        let mut out = format!("error: {}\n", self.message);
        let mut gutter = String::new();
        if let Some(span) = self.span {
            let at = sources.locate(span);
            let line_number = at.line.to_string();
            gutter = " ".repeat(line_number.len());
            let _ = writeln!(out, "{gutter}--> {}:{}:{}", at.path.display(), at.line, at.column);
            let _ = writeln!(out, "{gutter} |");
            let _ = writeln!(out, "{line_number} | {}", at.line_text);
            let _ = writeln!(
                out,
                "{gutter} | {}{}",
                " ".repeat(at.column - 1),
                "^".repeat(at.width)
            );
        }
        for note in &self.notes {
            let _ = writeln!(out, "{gutter} = note: {note}");
        }
        out
    }
}

/// `bail!(span, "format {}", args)` returns early with a [`Diagnostic`].
macro_rules! bail {
    ($span:expr, $($fmt:tt)+) => {
        return Err($crate::syntax::diagnostic::Diagnostic::new($span, format!($($fmt)+)))
    };
}
pub(crate) use bail;
