//! Tokens: the words and punctuation of the language.

use super::span::Span;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Keyword(Keyword),
    /// `'a`, without the quote.
    Lifetime(String),

    /// An integer literal and its optional type suffix (`42`, `0xff`, `7u8`).
    Int(u128, Option<String>),
    /// A float literal and its optional type suffix (`1.5`, `2e10`, `3f32`).
    Float(f64, Option<String>),
    Str(String),
    Char(char),
    ByteStr(Vec<u8>),
    Byte(u8),

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    Comma,
    Semi,
    Colon,
    PathSep,
    Dot,
    DotDot,
    DotDotEq,
    DotDotDot,
    Arrow,
    FatArrow,
    Pound,
    Question,
    At,
    /// `$`, which only appears in `macro_rules!` definitions.
    Dollar,
    Underscore,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Not,
    And,
    Or,
    AndAnd,
    OrOr,
    Shl,
    Shr,

    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    CaretEq,
    AndEq,
    OrEq,
    ShlEq,
    ShrEq,

    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    Eof,
}

/// Declares [`Keyword`] together with its spelling table.
macro_rules! keywords {
    ($($variant:ident => $text:literal),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Keyword {
            $($variant),*
        }

        impl Keyword {
            pub fn from_ident(text: &str) -> Option<Keyword> {
                match text {
                    $($text => Some(Keyword::$variant),)*
                    _ => None,
                }
            }

            pub fn as_str(self) -> &'static str {
                match self {
                    $(Keyword::$variant => $text),*
                }
            }
        }
    };
}

keywords! {
    As => "as",
    Break => "break",
    Const => "const",
    Continue => "continue",
    Crate => "crate",
    Dyn => "dyn",
    Else => "else",
    Enum => "enum",
    Extern => "extern",
    False => "false",
    Fn => "fn",
    For => "for",
    If => "if",
    Impl => "impl",
    In => "in",
    Let => "let",
    Loop => "loop",
    Match => "match",
    Mod => "mod",
    Move => "move",
    Mut => "mut",
    Pub => "pub",
    Ref => "ref",
    Return => "return",
    SelfValue => "self",
    SelfType => "Self",
    Static => "static",
    Struct => "struct",
    Super => "super",
    Trait => "trait",
    True => "true",
    Type => "type",
    Unsafe => "unsafe",
    Use => "use",
    Where => "where",
    While => "while",
}

impl fmt::Display for TokenKind {
    /// How the token is described in "expected X, found Y" messages.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use TokenKind::*;
        let text = match self {
            Ident(name) => return write!(f, "identifier `{name}`"),
            Keyword(kw) => return write!(f, "keyword `{}`", kw.as_str()),
            Lifetime(name) => return write!(f, "lifetime `'{name}`"),
            Int(..) => "integer literal",
            Float(..) => "float literal",
            Str(_) => "string literal",
            Char(_) => "character literal",
            ByteStr(_) => "byte string literal",
            Byte(_) => "byte literal",
            Eof => "end of file",
            LParen => "(",
            RParen => ")",
            LBrace => "{",
            RBrace => "}",
            LBracket => "[",
            RBracket => "]",
            Comma => ",",
            Semi => ";",
            Colon => ":",
            PathSep => "::",
            Dot => ".",
            DotDot => "..",
            DotDotEq => "..=",
            DotDotDot => "...",
            Arrow => "->",
            FatArrow => "=>",
            Pound => "#",
            Question => "?",
            At => "@",
            Dollar => "$",
            Underscore => "_",
            Plus => "+",
            Minus => "-",
            Star => "*",
            Slash => "/",
            Percent => "%",
            Caret => "^",
            Not => "!",
            And => "&",
            Or => "|",
            AndAnd => "&&",
            OrOr => "||",
            Shl => "<<",
            Shr => ">>",
            PlusEq => "+=",
            MinusEq => "-=",
            StarEq => "*=",
            SlashEq => "/=",
            PercentEq => "%=",
            CaretEq => "^=",
            AndEq => "&=",
            OrEq => "|=",
            ShlEq => "<<=",
            ShrEq => ">>=",
            Eq => "=",
            EqEq => "==",
            Ne => "!=",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
        };
        write!(f, "`{text}`")
    }
}
