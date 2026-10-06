//! The front end: source text in, [`ast`] out.
//!
//! ```text
//! source text ──lexer──▶ tokens ──parser──▶ AST
//! ```
//!
//! Nothing here knows what a name refers to or what type an expression has;
//! that is the job of [`crate::sema`].

pub mod ast;
pub mod build;
pub mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;
pub mod visit;

pub use diagnostic::{Diagnostic, Result};
pub use span::{SourceMap, Span};
