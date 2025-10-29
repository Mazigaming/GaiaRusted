//! # Token Definitions
//!
//! This module defines all possible tokens in Rust.
//!
//! Tokens are the basic building blocks that the lexer produces. Each token represents
//! a meaningful unit of Rust code.

use std::fmt;

/// All possible token types in Rust.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Integer(i64),           // 42, 0xFF, 0b1010
    Float(f64),             // 3.14
    String(String),         // "hello"
    Char(char),             // 'a'

    // Keywords
    Keyword(Keyword),

    // Identifiers
    Identifier(String),

    // Operators and Punctuation
    Plus,                   // +
    Minus,                  // -
    Star,                   // *
    Slash,                  // /
    Percent,                // %
    Equal,                  // =
    EqualEqual,             // ==
    NotEqual,               // !=
    Less,                   // <
    LessEqual,              // <=
    Greater,                // >
    GreaterEqual,           // >=
    Ampersand,              // &
    Pipe,                   // |
    Caret,                  // ^
    Bang,                   // !
    Tilde,                  // ~
    LeftShift,              // <<
    RightShift,             // >>
    AndAnd,                 // &&
    OrOr,                   // ||

    // Compound assignment operators
    PlusEqual,              // +=
    MinusEqual,             // -=
    StarEqual,              // *=
    SlashEqual,             // /=
    PercentEqual,           // %=
    AmpersandEqual,         // &=
    PipeEqual,              // |=
    CaretEqual,             // ^=
    LeftShiftEqual,         // <<=
    RightShiftEqual,        // >>=

    // Delimiters
    LeftParen,              // (
    RightParen,             // )
    LeftBrace,              // {
    RightBrace,             // }
    LeftBracket,            // [
    RightBracket,           // ]
    Semicolon,              // ;
    Comma,                  // ,
    Dot,                    // .
    DotDot,                 // ..
    DotDotDot,              // ...
    Colon,                  // :
    DoubleColon,            // ::
    Arrow,                  // ->
    FatArrow,               // =>

    // Special
    At,                     // @
    Hash,                   // #
    Question,               // ?

    // End of file
    Eof,
}

/// All Rust keywords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    // Type definitions
    Fn,
    Struct,
    Enum,
    Trait,
    Type,
    Impl,

    // Variables
    Let,
    Mut,
    Const,
    Static,

    // Control flow
    If,
    Else,
    Match,
    Loop,
    While,
    For,
    In,
    Break,
    Continue,
    Return,

    // Scope
    Pub,
    Priv,
    Crate,
    Mod,
    Use,
    As,

    // Memory
    Box,
    Ref,
    Unsafe,
    Move,

    // Rust-specific
    Self_,          // self (keyword, different from identifier)
    True,
    False,
    Null,

    // Advanced
    Where,
    Generic,
    Async,
    Await,
    Dyn,
    Lifetime,
}

impl Token {
    /// Create a token from an identifier string.
    ///
    /// If the identifier is a keyword, returns a Keyword token.
    /// Otherwise, returns an Identifier token.
    pub fn from_identifier(ident: &str) -> Token {
        match ident {
            // Keywords
            "fn" => Token::Keyword(Keyword::Fn),
            "struct" => Token::Keyword(Keyword::Struct),
            "enum" => Token::Keyword(Keyword::Enum),
            "trait" => Token::Keyword(Keyword::Trait),
            "type" => Token::Keyword(Keyword::Type),
            "impl" => Token::Keyword(Keyword::Impl),
            "let" => Token::Keyword(Keyword::Let),
            "mut" => Token::Keyword(Keyword::Mut),
            "const" => Token::Keyword(Keyword::Const),
            "static" => Token::Keyword(Keyword::Static),
            "if" => Token::Keyword(Keyword::If),
            "else" => Token::Keyword(Keyword::Else),
            "match" => Token::Keyword(Keyword::Match),
            "loop" => Token::Keyword(Keyword::Loop),
            "while" => Token::Keyword(Keyword::While),
            "for" => Token::Keyword(Keyword::For),
            "in" => Token::Keyword(Keyword::In),
            "break" => Token::Keyword(Keyword::Break),
            "continue" => Token::Keyword(Keyword::Continue),
            "return" => Token::Keyword(Keyword::Return),
            "pub" => Token::Keyword(Keyword::Pub),
            "crate" => Token::Keyword(Keyword::Crate),
            "mod" => Token::Keyword(Keyword::Mod),
            "use" => Token::Keyword(Keyword::Use),
            "as" => Token::Keyword(Keyword::As),
            "box" => Token::Keyword(Keyword::Box),
            "ref" => Token::Keyword(Keyword::Ref),
            "unsafe" => Token::Keyword(Keyword::Unsafe),
            "move" => Token::Keyword(Keyword::Move),
            "self" => Token::Keyword(Keyword::Self_),
            "true" => Token::Keyword(Keyword::True),
            "false" => Token::Keyword(Keyword::False),
            "null" => Token::Keyword(Keyword::Null),
            "where" => Token::Keyword(Keyword::Where),
            "async" => Token::Keyword(Keyword::Async),
            "await" => Token::Keyword(Keyword::Await),
            "dyn" => Token::Keyword(Keyword::Dyn),

            // Default: treat as identifier
            _ => Token::Identifier(ident.to_string()),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Integer(n) => write!(f, "Integer({})", n),
            Token::Float(n) => write!(f, "Float({})", n),
            Token::String(s) => write!(f, "String(\"{}\")", s),
            Token::Char(c) => write!(f, "Char('{}')", c),
            Token::Keyword(kw) => write!(f, "Keyword({:?})", kw),
            Token::Identifier(id) => write!(f, "Identifier({})", id),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Equal => write!(f, "="),
            Token::EqualEqual => write!(f, "=="),
            Token::NotEqual => write!(f, "!="),
            Token::Less => write!(f, "<"),
            Token::LessEqual => write!(f, "<="),
            Token::Greater => write!(f, ">"),
            Token::GreaterEqual => write!(f, ">="),
            Token::Ampersand => write!(f, "&"),
            Token::Pipe => write!(f, "|"),
            Token::Caret => write!(f, "^"),
            Token::Bang => write!(f, "!"),
            Token::Tilde => write!(f, "~"),
            Token::LeftShift => write!(f, "<<"),
            Token::RightShift => write!(f, ">>"),
            Token::AndAnd => write!(f, "&&"),
            Token::OrOr => write!(f, "||"),
            Token::PlusEqual => write!(f, "+="),
            Token::MinusEqual => write!(f, "-="),
            Token::StarEqual => write!(f, "*="),
            Token::SlashEqual => write!(f, "/="),
            Token::PercentEqual => write!(f, "%="),
            Token::AmpersandEqual => write!(f, "&="),
            Token::PipeEqual => write!(f, "|="),
            Token::CaretEqual => write!(f, "^="),
            Token::LeftShiftEqual => write!(f, "<<="),
            Token::RightShiftEqual => write!(f, ">>="),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::LeftBracket => write!(f, "["),
            Token::RightBracket => write!(f, "]"),
            Token::Semicolon => write!(f, ";"),
            Token::Comma => write!(f, ","),
            Token::Dot => write!(f, "."),
            Token::DotDot => write!(f, ".."),
            Token::DotDotDot => write!(f, "..."),
            Token::Colon => write!(f, ":"),
            Token::DoubleColon => write!(f, "::"),
            Token::Arrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::At => write!(f, "@"),
            Token::Hash => write!(f, "#"),
            Token::Question => write!(f, "?"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}