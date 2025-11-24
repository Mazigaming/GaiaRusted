//! # Phase 1: LEXER (Lexical Analysis)
//!
//! The lexer is the **first phase** of compilation. Its job is to convert raw source code
//! into a stream of tokens.
//!
//! ## What is a Token?
//!
//! A token is the smallest meaningful unit of code. For example:
//! ```text
//! Input:  let x = 42 + 3;
//! Tokens: [Keyword("let"), Identifier("x"), Equals, Integer(42), Plus, Integer(3), Semicolon]
//! ```
//!
//! ## Why Do We Need a Lexer?
//!
//! Raw source code is just a string of characters. A parser can't work directly on characters
//! because it needs to understand structure. The lexer:
//!
//! 1. **Removes noise** - Comments and whitespace are handled here
//! 2. **Groups characters** - Turns "42" into a number token, not three separate characters
//! 3. **Recognizes keywords** - Distinguishes "let" (keyword) from "let_var" (identifier)
//! 4. **Detects operators** - Identifies "+" vs "+=" vs "+.."
//! 5. **Reports errors early** - Catches invalid characters before the parser sees them
//!
//! ## The Lexer Algorithm
//!
//! ```text
//! function lex(source):
//!     tokens = []
//!     position = 0
//!     
//!     while position < source.length:
//!         current_char = source[position]
//!         
//!         if is_whitespace(current_char):
//!             skip whitespace
//!         elif is_digit(current_char):
//!             read_number()
//!         elif is_letter(current_char) or current_char == '_':
//!             read_identifier_or_keyword()
//!         elif current_char == '"':
//!             read_string()
//!         elif is_operator_char(current_char):
//!             read_operator()
//!         else:
//!             error: unknown character
//!     
//!     return tokens
//! ```

pub mod token;

use std::fmt;

/// The main lexer struct. Contains the source code and current position.
pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    /// Create a new lexer from source code.
    ///
    /// # Example
    /// ```ignore
    /// let lexer = Lexer::new("let x = 42;");
    /// ```
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
        }
    }

    /// Get the current character without advancing.
    fn current_char(&self) -> Option<char> {
        if self.position < self.input.len() {
            Some(self.input[self.position])
        } else {
            None
        }
    }

    /// Peek at the next character without advancing.
    fn peek_char(&self, offset: usize) -> Option<char> {
        let pos = self.position + offset;
        if pos < self.input.len() {
            Some(self.input[pos])
        } else {
            None
        }
    }

    /// Advance to the next character.
    fn advance(&mut self) -> Option<char> {
        if self.position < self.input.len() {
            let ch = self.input[self.position];
            self.position += 1;
            Some(ch)
        } else {
            None
        }
    }

    /// Check if we're at the end of input.
    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }

    /// Skip whitespace (spaces, tabs, newlines).
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Skip a comment. Assumes we're at the start of a comment.
    fn skip_comment(&mut self) {
        // Single-line comment: //
        if self.current_char() == Some('/') && self.peek_char(1) == Some('/') {
            self.advance(); // skip first /
            self.advance(); // skip second /
            // Skip until newline
            while let Some(ch) = self.current_char() {
                if ch == '\n' {
                    self.advance();
                    break;
                }
                self.advance();
            }
        }
        // Multi-line comment: /* ... */
        else if self.current_char() == Some('/') && self.peek_char(1) == Some('*') {
            self.advance(); // skip /
            self.advance(); // skip *
            // Skip until */
            while !self.is_at_end() {
                if self.current_char() == Some('*') && self.peek_char(1) == Some('/') {
                    self.advance(); // skip *
                    self.advance(); // skip /
                    break;
                }
                self.advance();
            }
        }
    }

    /// Read a number (integer or float).
    ///
    /// Examples: `42`, `3.14`, `0xFF`, `0b1010`
    fn read_number(&mut self) -> Result<token::Token, LexError> {
        let mut num_str = String::new();
        let mut is_float = false;

        // Check for hex, binary, octal
        if self.current_char() == Some('0') {
            num_str.push('0');
            self.advance();

            if let Some(ch) = self.current_char() {
                match ch {
                    'x' | 'X' => {
                        // Hex literal
                        self.advance();
                        while let Some(ch) = self.current_char() {
                            if ch.is_ascii_hexdigit() {
                                num_str.push(ch);
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        let value = i64::from_str_radix(&num_str, 16)
                            .map_err(|_| LexError::InvalidNumber(num_str.clone()))?;
                        let suffix = self.read_numeric_suffix();
                        return Ok(token::Token::Integer(value, suffix));
                    }
                    'b' | 'B' => {
                        // Binary literal
                        self.advance();
                        while let Some(ch) = self.current_char() {
                            if ch == '0' || ch == '1' {
                                num_str.push(ch);
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        let value = i64::from_str_radix(&num_str, 2)
                            .map_err(|_| LexError::InvalidNumber(num_str.clone()))?;
                        let suffix = self.read_numeric_suffix();
                        return Ok(token::Token::Integer(value, suffix));
                    }
                    'o' | 'O' => {
                        // Octal literal
                        self.advance();
                        while let Some(ch) = self.current_char() {
                            if ch >= '0' && ch <= '7' {
                                num_str.push(ch);
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        let value = i64::from_str_radix(&num_str, 8)
                            .map_err(|_| LexError::InvalidNumber(num_str.clone()))?;
                        let suffix = self.read_numeric_suffix();
                        return Ok(token::Token::Integer(value, suffix));
                    }
                    _ => {}
                }
            }
        }

        // Regular decimal number
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' && !is_float && self.peek_char(1).map_or(false, |c| c.is_ascii_digit()) {
                is_float = true;
                num_str.push(ch);
                self.advance();
            } else if ch == '_' {
                // Underscore separators are allowed in numbers but not added to the string
                self.advance();
            } else {
                break;
            }
        }

        let suffix = self.read_numeric_suffix();

        if is_float {
            let value = num_str.parse::<f64>()
                .map_err(|_| LexError::InvalidNumber(num_str))?;
            Ok(token::Token::Float(value, suffix))
        } else {
            // Try to parse as i64; if it fails, try u64 for large unsigned values
            match num_str.parse::<i64>() {
                Ok(value) => Ok(token::Token::Integer(value, suffix)),
                Err(_) => {
                    match num_str.parse::<u64>() {
                        Ok(value) => Ok(token::Token::Integer(value as i64, suffix)),
                        Err(_) => Err(LexError::InvalidNumber(num_str))
                    }
                }
            }
        }
    }

    /// Read numeric suffix (e.g., i32, u64, f32, f64, isize, usize) and return it if valid
    fn read_numeric_suffix(&mut self) -> Option<String> {
        if let Some(ch) = self.current_char() {
            match ch {
                'i' | 'u' | 'f' => {
                    self.advance();
                    let start_pos = self.position;
                    let mut suffix = String::new();
                    suffix.push(ch);
                    
                    while let Some(c) = self.current_char() {
                        if c.is_ascii_alphanumeric() || c == '_' {
                            suffix.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    
                    let valid_suffixes = [
                        "i8", "i16", "i32", "i64", "isize",
                        "u8", "u16", "u32", "u64", "usize",
                        "f32", "f64",
                    ];
                    
                    if valid_suffixes.contains(&suffix.as_str()) {
                        Some(suffix)
                    } else {
                        self.position = start_pos - 1;
                        None
                    }
                }
                _ => None
            }
        } else {
            None
        }
    }

    /// Read an identifier or keyword.
    fn read_identifier_or_keyword(&mut self) -> Result<token::Token, LexError> {
        let mut ident = String::new();

        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Ok(token::Token::from_identifier(&ident))
    }

    /// Read a string literal.
    fn read_string(&mut self) -> Result<token::Token, LexError> {
        self.advance(); // skip opening quote
        let mut string = String::new();

        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance(); // skip closing quote
                return Ok(token::Token::String(string));
            } else if ch == '\\' {
                self.advance();
                match self.current_char() {
                    Some('n') => { string.push('\n'); self.advance(); }
                    Some('t') => { string.push('\t'); self.advance(); }
                    Some('r') => { string.push('\r'); self.advance(); }
                    Some('\\') => { string.push('\\'); self.advance(); }
                    Some('"') => { string.push('"'); self.advance(); }
                    Some(ch) => {
                        string.push(ch);
                        self.advance();
                    }
                    None => return Err(LexError::UnterminatedString),
                }
            } else {
                string.push(ch);
                self.advance();
            }
        }

        Err(LexError::UnterminatedString)
    }

    /// Check if this looks like a lifetime (e.g., 'a, 'static, '_).
    /// Returns true if current char is ' and next is a valid lifetime start.
    fn is_lifetime_start(&self) -> bool {
        if self.current_char() == Some('\'') {
            if let Some(next_ch) = self.peek_char(1) {
                match next_ch {
                    'a'..='z' | 'A'..='Z' | '_' => {
                        // Check if this is actually a lifetime (no closing quote following)
                        // Lifetimes like 'a, 'static, '_ don't have a closing quote immediately after the identifier
                        // Character literals like 'a', 'z' have a closing quote
                        
                        // Look ahead to check if there's a closing quote after the identifier
                        let mut pos = 2; // Skip the opening quote and first letter
                        while let Some(ch) = self.peek_char(pos) {
                            if ch.is_alphanumeric() || ch == '_' {
                                pos += 1;
                            } else if ch == '\'' {
                                // Found closing quote - this is a character literal, not a lifetime
                                return false;
                            } else {
                                // Found something else - this is a lifetime
                                return true;
                            }
                        }
                        // No closing quote found - this is a lifetime
                        true
                    }
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Read a lifetime token (e.g., 'a, 'static, '_).
    fn read_lifetime(&mut self) -> Result<token::Token, LexError> {
        self.advance(); // skip opening quote
        let mut lifetime = String::new();

        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' {
                lifetime.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if lifetime.is_empty() {
            Err(LexError::UnterminatedChar)
        } else {
            Ok(token::Token::Lifetime(lifetime))
        }
    }

    /// Read a character literal.
    fn read_char(&mut self) -> Result<token::Token, LexError> {
        self.advance(); // skip opening quote
        let mut char_val = '\0';

        if let Some(ch) = self.current_char() {
            if ch == '\\' {
                self.advance();
                char_val = match self.current_char() {
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('r') => '\r',
                    Some('\\') => '\\',
                    Some('\'') => '\'',
                    Some(c) => c,
                    None => return Err(LexError::UnterminatedChar),
                };
                self.advance();
            } else {
                char_val = ch;
                self.advance();
            }
        }

        if self.current_char() == Some('\'') {
            self.advance(); // skip closing quote
            Ok(token::Token::Char(char_val))
        } else {
            Err(LexError::UnterminatedChar)
        }
    }

    /// Read a raw string: r"..."
    fn read_raw_string(&mut self) -> Result<token::Token, LexError> {
        self.advance(); // skip opening quote
        let mut string = String::new();

        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance(); // skip closing quote
                return Ok(token::Token::RawString(string));
            } else {
                string.push(ch);
                self.advance();
            }
        }

        Err(LexError::UnterminatedString)
    }

    /// Read a raw string with hashes: r#"..."#
    fn read_raw_string_with_hashes(&mut self) -> Result<token::Token, LexError> {
        // Count opening hashes
        let mut hash_count = 0;
        while self.current_char() == Some('#') {
            hash_count += 1;
            self.advance();
        }

        if self.current_char() != Some('"') {
            return Err(LexError::UnterminatedString);
        }
        self.advance(); // skip opening quote

        let mut string = String::new();
        let mut closing_hashes = 0;

        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance();
                closing_hashes = 0;

                while self.current_char() == Some('#') && closing_hashes < hash_count {
                    closing_hashes += 1;
                    self.advance();
                }

                if closing_hashes == hash_count {
                    return Ok(token::Token::RawString(string));
                } else {
                    string.push('"');
                    for _ in 0..closing_hashes {
                        string.push('#');
                    }
                    closing_hashes = 0;
                }
            } else {
                string.push(ch);
                self.advance();
            }
        }

        Err(LexError::UnterminatedString)
    }

    /// Read a byte string: b"..."
    fn read_byte_string(&mut self) -> Result<token::Token, LexError> {
        self.advance(); // skip opening quote
        let mut bytes = Vec::new();

        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance(); // skip closing quote
                return Ok(token::Token::ByteString(bytes));
            } else if ch == '\\' {
                self.advance();
                match self.current_char() {
                    Some('n') => { bytes.push(b'\n'); self.advance(); }
                    Some('t') => { bytes.push(b'\t'); self.advance(); }
                    Some('r') => { bytes.push(b'\r'); self.advance(); }
                    Some('\\') => { bytes.push(b'\\'); self.advance(); }
                    Some('"') => { bytes.push(b'"'); self.advance(); }
                    Some('0') => { bytes.push(0u8); self.advance(); }
                    Some(ch) if ch.is_ascii() => { 
                        bytes.push(ch as u8); 
                        self.advance(); 
                    }
                    _ => return Err(LexError::UnterminatedString),
                }
            } else if ch.is_ascii() {
                bytes.push(ch as u8);
                self.advance();
            } else {
                return Err(LexError::UnterminatedString);
            }
        }

        Err(LexError::UnterminatedString)
    }

    /// Read a byte character: b'...'
    fn read_byte_char(&mut self) -> Result<token::Token, LexError> {
        self.advance(); // skip opening quote
        let mut byte_val: u8 = 0;

        if let Some(ch) = self.current_char() {
            if ch == '\\' {
                self.advance();
                byte_val = match self.current_char() {
                    Some('n') => b'\n',
                    Some('t') => b'\t',
                    Some('r') => b'\r',
                    Some('\\') => b'\\',
                    Some('\'') => b'\'',
                    Some('0') => 0u8,
                    Some(c) if c.is_ascii() => c as u8,
                    _ => return Err(LexError::UnterminatedChar),
                };
                self.advance();
            } else if ch.is_ascii() {
                byte_val = ch as u8;
                self.advance();
            } else {
                return Err(LexError::UnterminatedChar);
            }
        }

        if self.current_char() == Some('\'') {
            self.advance(); // skip closing quote
            Ok(token::Token::ByteChar(byte_val))
        } else {
            Err(LexError::UnterminatedChar)
        }
    }

    /// Read a macro metavariable (e.g., $x, $expr, $tt).
    fn read_metavariable(&mut self) -> Result<token::Token, LexError> {
        self.advance(); // skip $
        let mut metavar = String::new();

        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' {
                metavar.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if metavar.is_empty() {
            Ok(token::Token::Dollar)
        } else {
            Ok(token::Token::Metavariable(metavar))
        }
    }

    /// Read the next token from the input.
    fn next_token(&mut self) -> Result<Option<token::Token>, LexError> {
        loop {
            self.skip_whitespace();

            if self.is_at_end() {
                return Ok(None);
            }

            // Check for comments
            if self.current_char() == Some('/') && (self.peek_char(1) == Some('/') || self.peek_char(1) == Some('*')) {
                self.skip_comment();
                continue;
            }

            break;
        }

        let ch = match self.current_char() {
            Some(c) => c,
            None => return Ok(None),
        };

        let token = match ch {
            // Numbers
            c if c.is_ascii_digit() => self.read_number()?,

            // Raw strings: r"..." or r#"..."#
            'r' if self.peek_char(1) == Some('"') => {
                self.advance();
                self.read_raw_string()?
            }
            'r' if self.peek_char(1) == Some('#') => {
                self.advance();
                self.read_raw_string_with_hashes()?
            }

            // Byte strings: b"..." or byte chars: b'...'
            'b' if self.peek_char(1) == Some('"') => {
                self.advance();
                self.read_byte_string()?
            }
            'b' if self.peek_char(1) == Some('\'') => {
                self.advance();
                self.read_byte_char()?
            }

            // Identifiers and keywords
            c if c.is_alphabetic() || c == '_' => self.read_identifier_or_keyword()?,

            // Strings
            '"' => self.read_string()?,

            // Lifetimes (must be checked before character literals)
            '\'' if self.is_lifetime_start() => self.read_lifetime()?,

            // Characters
            '\'' => self.read_char()?,

            // Single character tokens
            ';' => {
                self.advance();
                token::Token::Semicolon
            }
            ',' => {
                self.advance();
                token::Token::Comma
            }
            '(' => {
                self.advance();
                token::Token::LeftParen
            }
            ')' => {
                self.advance();
                token::Token::RightParen
            }
            '{' => {
                self.advance();
                token::Token::LeftBrace
            }
            '}' => {
                self.advance();
                token::Token::RightBrace
            }
            '[' => {
                self.advance();
                token::Token::LeftBracket
            }
            ']' => {
                self.advance();
                token::Token::RightBracket
            }
            ':' => {
                self.advance();
                if self.current_char() == Some(':') {
                    self.advance();
                    token::Token::DoubleColon
                } else {
                    token::Token::Colon
                }
            }
            '.' => {
                self.advance();
                if self.current_char() == Some('.') {
                    self.advance();
                    if self.current_char() == Some('.') {
                        self.advance();
                        token::Token::DotDotDot
                    } else if self.current_char() == Some('=') {
                        self.advance();
                        token::Token::DotDotEqual
                    } else {
                        token::Token::DotDot
                    }
                } else {
                    token::Token::Dot
                }
            }
            '!' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    token::Token::NotEqual
                } else {
                    token::Token::Bang
                }
            }
            '=' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    token::Token::EqualEqual
                } else if self.current_char() == Some('>') {
                    self.advance();
                    token::Token::FatArrow
                } else {
                    token::Token::Equal
                }
            }
            '+' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    token::Token::PlusEqual
                } else {
                    token::Token::Plus
                }
            }
            '-' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    token::Token::MinusEqual
                } else if self.current_char() == Some('>') {
                    self.advance();
                    token::Token::Arrow
                } else {
                    token::Token::Minus
                }
            }
            '*' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    token::Token::StarEqual
                } else {
                    token::Token::Star
                }
            }
            '/' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    token::Token::SlashEqual
                } else {
                    token::Token::Slash
                }
            }
            '%' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    token::Token::PercentEqual
                } else {
                    token::Token::Percent
                }
            }
            '<' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    token::Token::LessEqual
                } else if self.current_char() == Some('<') {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        token::Token::LeftShiftEqual
                    } else {
                        token::Token::LeftShift
                    }
                } else {
                    token::Token::Less
                }
            }
            '>' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    token::Token::GreaterEqual
                } else if self.current_char() == Some('>') {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        token::Token::RightShiftEqual
                    } else {
                        token::Token::RightShift
                    }
                } else {
                    token::Token::Greater
                }
            }
            '&' => {
                self.advance();
                if self.current_char() == Some('&') {
                    self.advance();
                    token::Token::AndAnd
                } else if self.current_char() == Some('=') {
                    self.advance();
                    token::Token::AmpersandEqual
                } else {
                    token::Token::Ampersand
                }
            }
            '|' => {
                self.advance();
                if self.current_char() == Some('|') {
                    self.advance();
                    token::Token::OrOr
                } else if self.current_char() == Some('=') {
                    self.advance();
                    token::Token::PipeEqual
                } else {
                    token::Token::Pipe
                }
            }
            '^' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    token::Token::CaretEqual
                } else {
                    token::Token::Caret
                }
            }
            '~' => {
                self.advance();
                token::Token::Tilde
            }
            '@' => {
                self.advance();
                token::Token::At
            }
            '#' => {
                self.advance();
                token::Token::Hash
            }
            '?' => {
                self.advance();
                token::Token::Question
            }
            '$' => self.read_metavariable()?,
            _ => {
                return Err(LexError::UnexpectedCharacter(ch));
            }
        };

        Ok(Some(token))
    }
}

/// Error types that can occur during lexing.
#[derive(Debug, Clone)]
pub enum LexError {
    UnexpectedCharacter(char),
    InvalidNumber(String),
    UnterminatedString,
    UnterminatedChar,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LexError::UnexpectedCharacter(ch) => write!(f, "Unexpected character: '{}'", ch),
            LexError::InvalidNumber(num) => write!(f, "Invalid number: {}", num),
            LexError::UnterminatedString => write!(f, "Unterminated string"),
            LexError::UnterminatedChar => write!(f, "Unterminated character literal"),
        }
    }
}

/// The main lexing function. Takes source code and returns a vector of tokens.
///
/// # Example
/// ```ignore
/// let tokens = lex("let x = 42;")?;
/// assert_eq!(tokens.len(), 5);
/// ```
pub fn lex(input: &str) -> Result<Vec<token::Token>, LexError> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next_token()? {
        tokens.push(token);
    }

    tokens.push(token::Token::Eof);
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_number() {
        let tokens = lex("42").unwrap();
        assert_eq!(tokens.len(), 2); // 42, EOF
        assert!(matches!(tokens[0], token::Token::Integer(42, None)));
    }

    #[test]
    fn test_keyword_recognition() {
        let tokens = lex("let").unwrap();
        assert!(matches!(tokens[0], token::Token::Keyword(_)));
    }

    #[test]
    fn test_identifier() {
        let tokens = lex("variable_name").unwrap();
        assert!(matches!(tokens[0], token::Token::Identifier(_)));
    }
}