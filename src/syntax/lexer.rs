//! The lexer turns source text into [`Token`]s.

use super::diagnostic::{bail, Result};
use super::span::{FileId, Span};
use super::token::{Keyword, Token, TokenKind};

/// Tokenize one source file. The result always ends with [`TokenKind::Eof`].
pub fn tokenize(file: FileId, source: &str) -> Result<Vec<Token>> {
    let mut lexer = Lexer { file, source, pos: 0 };
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token()?;
        let done = token.kind == TokenKind::Eof;
        tokens.push(token);
        if done {
            return Ok(tokens);
        }
    }
}

struct Lexer<'a> {
    file: FileId,
    source: &'a str,
    /// Byte offset of the next unread character.
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn peek_at(&self, chars_ahead: usize) -> Option<char> {
        self.source[self.pos..].chars().nth(chars_ahead)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn eat_while(&mut self, keep: impl Fn(char) -> bool) -> &'a str {
        let start = self.pos;
        while self.peek().is_some_and(&keep) {
            self.bump();
        }
        &self.source[start..self.pos]
    }

    fn span_from(&self, start: usize) -> Span {
        Span::new(self.file, start, self.pos)
    }

    fn next_token(&mut self) -> Result<Token> {
        self.skip_trivia()?;
        let start = self.pos;
        let Some(c) = self.bump() else {
            return Ok(Token { kind: TokenKind::Eof, span: self.span_from(start) });
        };

        let kind = match c {
            'b' if self.peek() == Some('"') => {
                self.bump();
                TokenKind::ByteStr(self.string_body(start)?.into_bytes())
            }
            'b' if self.peek() == Some('\'') => {
                self.bump();
                let byte = self.char_body(start)?;
                if !byte.is_ascii() {
                    bail!(self.span_from(start), "non-ASCII character in byte literal");
                }
                TokenKind::Byte(byte as u8)
            }
            'r' if matches!(self.peek(), Some('"' | '#')) && self.starts_raw_string() => {
                TokenKind::Str(self.raw_string(start)?)
            }
            c if is_ident_start(c) => self.ident_or_keyword(start),
            c if c.is_ascii_digit() => self.number(start)?,
            '"' => TokenKind::Str(self.string_body(start)?),
            '\'' => self.char_or_lifetime(start)?,
            _ => self.punctuation(c, start)?,
        };
        Ok(Token { kind, span: self.span_from(start) })
    }

    /// Skip whitespace and comments (`//` to end of line, nestable `/* */`).
    fn skip_trivia(&mut self) -> Result<()> {
        loop {
            self.eat_while(char::is_whitespace);
            let rest = &self.source[self.pos..];
            if rest.starts_with("//") {
                self.eat_while(|c| c != '\n');
            } else if rest.starts_with("/*") {
                self.block_comment()?;
            } else {
                return Ok(());
            }
        }
    }

    fn block_comment(&mut self) -> Result<()> {
        let start = self.pos;
        let mut depth = 0usize;
        loop {
            let rest = &self.source[self.pos..];
            if rest.starts_with("/*") {
                depth += 1;
                self.pos += 2;
            } else if rest.starts_with("*/") {
                depth -= 1;
                self.pos += 2;
                if depth == 0 {
                    return Ok(());
                }
            } else if self.bump().is_none() {
                bail!(Span::new(self.file, start, start + 2), "unterminated block comment");
            }
        }
    }

    fn ident_or_keyword(&mut self, start: usize) -> TokenKind {
        self.eat_while(is_ident_continue);
        let text = &self.source[start..self.pos];
        match Keyword::from_ident(text) {
            Some(keyword) => TokenKind::Keyword(keyword),
            None if text == "_" => TokenKind::Underscore,
            None => TokenKind::Ident(text.to_string()),
        }
    }

    fn number(&mut self, start: usize) -> Result<TokenKind> {
        let first = self.source.as_bytes()[start];
        let radix = match (first, self.peek()) {
            (b'0', Some('x')) => 16,
            (b'0', Some('o')) => 8,
            (b'0', Some('b')) => 2,
            _ => 10,
        };

        if radix != 10 {
            self.bump();
            let digits_start = self.pos;
            // Stop before a type suffix: `0xffu8` is the digits `ff` then `u8`.
            self.eat_while(|c| c == '_' || c.is_digit(radix));
            let digits = self.source[digits_start..self.pos].replace('_', "");
            let suffix = self.literal_suffix();
            let Ok(value) = u128::from_str_radix(&digits, radix) else {
                bail!(self.span_from(start), "invalid integer literal");
            };
            return Ok(TokenKind::Int(value, suffix));
        }

        self.eat_while(|c| c == '_' || c.is_ascii_digit());
        let mut is_float = false;

        // A `.` continues the number only when a digit follows: `1.5` is a
        // float, while `1..5` is a range and `1.max(2)` is a method call.
        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            self.bump();
            self.eat_while(|c| c == '_' || c.is_ascii_digit());
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            let sign_len = usize::from(matches!(self.peek_at(1), Some('+' | '-')));
            if self.peek_at(1 + sign_len).is_some_and(|c| c.is_ascii_digit()) {
                is_float = true;
                for _ in 0..=sign_len {
                    self.bump();
                }
                self.eat_while(|c| c == '_' || c.is_ascii_digit());
            }
        }

        let digits = self.source[start..self.pos].replace('_', "");
        let suffix = self.literal_suffix();
        if is_float || matches!(suffix.as_deref(), Some("f32" | "f64")) {
            let Ok(value) = digits.parse::<f64>() else {
                bail!(self.span_from(start), "invalid float literal");
            };
            Ok(TokenKind::Float(value, suffix))
        } else {
            let Ok(value) = digits.parse::<u128>() else {
                bail!(self.span_from(start), "integer literal is too large");
            };
            Ok(TokenKind::Int(value, suffix))
        }
    }

    /// The `u8` in `7u8`: an identifier glued directly onto a number.
    fn literal_suffix(&mut self) -> Option<String> {
        if self.peek().is_some_and(is_ident_start) {
            Some(self.eat_while(is_ident_continue).to_string())
        } else {
            None
        }
    }

    /// The contents of a `"..."` literal; the opening quote is already consumed.
    fn string_body(&mut self, start: usize) -> Result<String> {
        let mut text = String::new();
        loop {
            match self.bump() {
                None => bail!(Span::new(self.file, start, start + 1), "unterminated string literal"),
                Some('"') => return Ok(text),
                Some('\\') if self.peek() == Some('\n') => {
                    // A backslash at end of line swallows the newline and the
                    // next line's indentation.
                    self.eat_while(char::is_whitespace);
                }
                Some('\\') => text.push(self.escape()?),
                Some(c) => text.push(c),
            }
        }
    }

    /// The contents of a `'c'` literal; the opening quote is already consumed.
    fn char_body(&mut self, start: usize) -> Result<char> {
        let c = match self.bump() {
            Some('\\') => self.escape()?,
            Some(c) if c != '\'' => c,
            _ => bail!(self.span_from(start), "empty character literal"),
        };
        if !self.eat('\'') {
            bail!(self.span_from(start), "unterminated character literal");
        }
        Ok(c)
    }

    /// Decode an escape sequence; the backslash is already consumed.
    fn escape(&mut self) -> Result<char> {
        let start = self.pos - 1;
        let decoded = match self.bump() {
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            Some('0') => '\0',
            Some('\\') => '\\',
            Some('\'') => '\'',
            Some('"') => '"',
            Some('x') => {
                let digits = self.source.get(self.pos..self.pos + 2).unwrap_or("");
                let Ok(code) = u8::from_str_radix(digits, 16) else {
                    bail!(self.span_from(start), "invalid `\\x` escape");
                };
                self.pos += 2;
                code as char
            }
            Some('u') => {
                if !self.eat('{') {
                    bail!(self.span_from(start), "expected `{{` after `\\u`");
                }
                let digits = self.eat_while(|c| c.is_ascii_hexdigit() || c == '_').replace('_', "");
                let code = u32::from_str_radix(&digits, 16).ok().and_then(char::from_u32);
                match code {
                    Some(c) if self.eat('}') => c,
                    _ => bail!(self.span_from(start), "invalid unicode escape"),
                }
            }
            _ => bail!(self.span_from(start), "unknown escape sequence"),
        };
        Ok(decoded)
    }

    /// After an `r`, do hashes followed by a quote come next?
    fn starts_raw_string(&self) -> bool {
        self.source[self.pos..].trim_start_matches('#').starts_with('"')
    }

    /// `r"..."` / `r#"..."#`; the leading `r` is already consumed.
    fn raw_string(&mut self, start: usize) -> Result<String> {
        let hashes = self.eat_while(|c| c == '#').len();
        self.bump(); // opening quote
        let terminator = format!("\"{}", "#".repeat(hashes));
        let Some(len) = self.source[self.pos..].find(&terminator) else {
            bail!(Span::new(self.file, start, start + 1), "unterminated raw string literal");
        };
        let text = self.source[self.pos..self.pos + len].to_string();
        self.pos += len + terminator.len();
        Ok(text)
    }

    /// `'a'` is a character, `'a` is a lifetime; the quote is already consumed.
    fn char_or_lifetime(&mut self, start: usize) -> Result<TokenKind> {
        let is_lifetime =
            self.peek().is_some_and(is_ident_start) && self.peek_at(1) != Some('\'');
        if is_lifetime {
            Ok(TokenKind::Lifetime(self.eat_while(is_ident_continue).to_string()))
        } else {
            Ok(TokenKind::Char(self.char_body(start)?))
        }
    }

    fn punctuation(&mut self, first: char, start: usize) -> Result<TokenKind> {
        use TokenKind::*;
        let kind = match first {
            '(' => LParen,
            ')' => RParen,
            '{' => LBrace,
            '}' => RBrace,
            '[' => LBracket,
            ']' => RBracket,
            ',' => Comma,
            ';' => Semi,
            '#' => Pound,
            '?' => Question,
            '@' => At,
            '$' => Dollar,
            ':' if self.eat(':') => PathSep,
            ':' => Colon,
            '.' if self.eat('.') => {
                if self.eat('=') {
                    DotDotEq
                } else if self.eat('.') {
                    DotDotDot
                } else {
                    DotDot
                }
            }
            '.' => Dot,
            '-' if self.eat('>') => Arrow,
            '-' if self.eat('=') => MinusEq,
            '-' => Minus,
            '=' if self.eat('>') => FatArrow,
            '=' if self.eat('=') => EqEq,
            '=' => Eq,
            '!' if self.eat('=') => Ne,
            '!' => Not,
            '+' if self.eat('=') => PlusEq,
            '+' => Plus,
            '*' if self.eat('=') => StarEq,
            '*' => Star,
            '/' if self.eat('=') => SlashEq,
            '/' => Slash,
            '%' if self.eat('=') => PercentEq,
            '%' => Percent,
            '^' if self.eat('=') => CaretEq,
            '^' => Caret,
            '&' if self.eat('&') => AndAnd,
            '&' if self.eat('=') => AndEq,
            '&' => And,
            '|' if self.eat('|') => OrOr,
            '|' if self.eat('=') => OrEq,
            '|' => Or,
            '<' if self.eat('<') => {
                if self.eat('=') {
                    ShlEq
                } else {
                    Shl
                }
            }
            '<' if self.eat('=') => Le,
            '<' => Lt,
            '>' if self.eat('>') => {
                if self.eat('=') {
                    ShrEq
                } else {
                    Shr
                }
            }
            '>' if self.eat('=') => Ge,
            '>' => Gt,
            other => bail!(self.span_from(start), "unexpected character `{other}`"),
        };
        Ok(kind)
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;
    use TokenKind::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        let mut tokens = tokenize(FileId(0), source).unwrap();
        tokens.pop(); // Eof
        tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn numbers_distinguish_floats_ranges_and_method_calls() {
        assert_eq!(kinds("1.5"), [Float(1.5, None)]);
        assert_eq!(kinds("1..5"), [Int(1, None), DotDot, Int(5, None)]);
        assert_eq!(kinds("1.max"), [Int(1, None), Dot, Ident("max".into())]);
        assert_eq!(kinds("2e3 0xff_u8 1_000"), [
            Float(2000.0, None),
            Int(255, Some("u8".into())),
            Int(1000, None),
        ]);
        assert_eq!(kinds("3f32"), [Float(3.0, Some("f32".into()))]);
    }

    #[test]
    fn quotes_distinguish_chars_from_lifetimes() {
        assert_eq!(kinds("'a' 'a '\\n'"), [Char('a'), Lifetime("a".into()), Char('\n')]);
    }

    #[test]
    fn strings_decode_escapes() {
        assert_eq!(kinds(r#""a\tb\u{41}\x42""#), [Str("a\tbAB".into())]);
        assert_eq!(kinds(r##"r#"raw "quoted""#"##), [Str("raw \"quoted\"".into())]);
        assert_eq!(kinds(r#"b"hi" b'x'"#), [ByteStr(b"hi".to_vec()), Byte(b'x')]);
    }

    #[test]
    fn comments_are_skipped_and_nest() {
        assert_eq!(kinds("a /* x /* y */ z */ b // tail"), [Ident("a".into()), Ident("b".into())]);
    }

    #[test]
    fn compound_punctuation_is_greedy() {
        assert_eq!(kinds("a >>= b ..= c"), [
            Ident("a".into()),
            ShrEq,
            Ident("b".into()),
            DotDotEq,
            Ident("c".into()),
        ]);
    }

    #[test]
    fn spans_point_at_the_token() {
        let tokens = tokenize(FileId(0), "let  x").unwrap();
        assert_eq!((tokens[1].span.lo, tokens[1].span.hi), (5, 6));
    }

    #[test]
    fn unterminated_string_is_an_error() {
        assert!(tokenize(FileId(0), "\"abc").is_err());
    }
}
