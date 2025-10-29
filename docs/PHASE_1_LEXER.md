# Phase 1: Lexer (Lexical Analysis) - Complete Documentation

## Overview

The **Lexer** is the first phase of the compiler. It reads raw source code (a string of characters) and converts it into a stream of **tokens** — the smallest meaningful units of code.

### Example
```
Input:  let x = 42 + 3;
Output: [Keyword(let), Identifier(x), Equal, Integer(42), Plus, Integer(3), Semicolon, EOF]
```

---

## Why Do We Need a Lexer?

The parser can't work directly on raw characters. Here's why a lexer is essential:

1. **Removes Noise** - Comments and whitespace are discarded
2. **Groups Characters** - "42" becomes one Integer(42) token, not three separate chars
3. **Recognizes Keywords** - Distinguishes "let" (keyword) from "letter" (identifier)
4. **Detects Operators** - Identifies "+" vs "+=" vs "..."
5. **Error Detection** - Catches invalid characters early, before parsing
6. **Simplifies Parsing** - Parser works with tokens, not individual chars

---

## The Lexer Algorithm

```pseudocode
function lex(source_code):
    tokens = []
    position = 0
    
    while position < length(source_code):
        current_char = source_code[position]
        
        if is_whitespace(current_char):
            skip it
        elif is_digit(current_char):
            read entire number
        elif is_letter(current_char) or '_':
            read identifier or keyword
        elif current_char == '"':
            read string
        elif current_char == '\'':
            read character
        elif is_operator_char(current_char):
            read operator (might be multi-char)
        else:
            error: unknown character
    
    tokens.append(EOF)
    return tokens
```

---

## Token Types

### Literals
- **Integer**: `42`, `0xFF`, `0b1010`, `0o755`
- **Float**: `3.14`, `2.0`
- **String**: `"hello world"`
- **Char**: `'a'`, `'\n'`

### Keywords
These are reserved words that have special meaning:
- **Type Definition**: `fn`, `struct`, `enum`, `trait`, `type`, `impl`
- **Variables**: `let`, `mut`, `const`, `static`
- **Control Flow**: `if`, `else`, `match`, `loop`, `while`, `for`, `break`, `continue`, `return`
- **Scope**: `pub`, `crate`, `mod`, `use`, `as`
- **Memory**: `ref`, `unsafe`, `move`, `box`
- **Other**: `true`, `false`, `self`, `where`, `async`, `await`

### Identifiers
Any name that's NOT a keyword: `x`, `variable_name`, `my_function`, `Type`

### Operators
- **Arithmetic**: `+`, `-`, `*`, `/`, `%`
- **Comparison**: `==`, `!=`, `<`, `<=`, `>`, `>=`
- **Logical**: `&&`, `||`, `!`
- **Bitwise**: `&`, `|`, `^`, `~`, `<<`, `>>`
- **Assignment**: `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=`

### Punctuation
- **Delimiters**: `(`, `)`, `{`, `}`, `[`, `]`
- **Separators**: `;`, `,`
- **Dots**: `.`, `..`, `...`
- **Arrows**: `->`, `=>`
- **Colons**: `:`, `::`
- **Special**: `@`, `#`, `?`

### Special
- **EOF**: End of file marker (added at the end)

---

## Key Lexer Features

### 1. Keyword Recognition
```rust
// The lexer distinguishes between keywords and identifiers
let        → Keyword(Let)
letter     → Identifier("letter")
fn         → Keyword(Fn)
function   → Identifier("function")
```

### 2. Multi-Character Operators
The lexer carefully handles operators that can be 1, 2, or 3 characters:

```
+   → Plus
+=  → PlusEqual
++  → Error (Rust doesn't have this)

=   → Equal
==  → EqualEqual
=>  → FatArrow

.   → Dot
..  → DotDot
... → DotDotDot
```

### 3. Number Parsing
Supports multiple number formats:
```
42        → Integer(42)
3.14      → Float(3.14)
0xFF      → Integer(255)  // Hex
0b1010    → Integer(10)   // Binary
0o755     → Integer(493)  // Octal
1_000_000 → Integer(1000000)  // Underscores allowed for readability
```

### 4. String and Char Literals
```rust
"hello"     → String("hello")
"line1\nline2" → String("line1\nline2")  // Escape sequences
'a'         → Char('a')
'\n'        → Char('\n')
```

### 5. Comment Handling
```rust
// Single-line comment → ignored
/* Multi-line
   comment */        → ignored
```

---

## Lexer Implementation Details

### The `Lexer` Struct
```rust
pub struct Lexer {
    input: Vec<char>,      // Source code as characters
    position: usize,       // Current position in input
}
```

### Core Methods

#### `current_char() -> Option<char>`
Returns the character at current position without advancing.

#### `peek_char(offset) -> Option<char>`
Looks ahead `offset` characters without advancing.

#### `advance() -> Option<char>`
Moves to the next character and returns the current one.

#### `skip_whitespace()`
Skips spaces, tabs, newlines until a non-whitespace character.

#### `read_number() -> Token`
Reads an entire number (handles int, float, hex, binary, octal).

#### `read_identifier_or_keyword() -> Token`
Reads an identifier and checks if it's a keyword.

#### `read_string() -> Token`
Reads a string literal with escape sequence handling.

#### `read_char() -> Token`
Reads a character literal with escape sequences.

#### `next_token() -> Result<Option<Token>, LexError>`
The main method that reads the next token.

---

## Error Handling

The lexer returns `LexError` for invalid input:

```rust
pub enum LexError {
    UnexpectedCharacter(char),      // Unknown character like $, @
    InvalidNumber(String),          // Number that can't be parsed
    UnterminatedString,             // String without closing quote
    UnterminatedChar,               // Char without closing quote
}
```

### Examples
```rust
$variable     → Error: UnexpectedCharacter('$')
42.34.56      → Error: InvalidNumber("42.34.56")
"hello        → Error: UnterminatedString
'a            → Error: UnterminatedChar
```

---

## Testing the Lexer

### Test: Simple Number
```rust
#[test]
fn test_simple_number() {
    let tokens = lex("42").unwrap();
    assert_eq!(tokens[0], Token::Integer(42));
}
```

### Test: Keywords
```rust
#[test]
fn test_keyword_recognition() {
    let tokens = lex("let").unwrap();
    assert!(matches!(tokens[0], Token::Keyword(Keyword::Let)));
}
```

### Test: Complex Expression
```rust
#[test]
fn test_complex_expression() {
    let tokens = lex("let x = 42 + 3.14;").unwrap();
    // Should produce 8 tokens (including EOF)
}
```

---

## Performance Considerations

**Current Implementation**: O(n) time complexity where n = source code length
- Single pass through source
- No backtracking
- O(n) space for token vector

**Could Be Optimized**:
- Intern strings to reduce memory
- Use buffer pooling for token storage
- Cache common tokens

---

## Integration with Next Phase (Parser)

The lexer output (token stream) becomes the **parser's input**.

```
Lexer Output:  [Keyword(Fn), Identifier(main), LeftParen, RightParen, ...]
                    ↓
Parser Input:  Same token stream
                    ↓
Parser Output: Abstract Syntax Tree (AST)
```

The parser will expect tokens in a specific order and structure.

---

## Example: Full Lexing Process

### Input Program
```rust
fn main() {
    let x = 42;
    println!("{}", x);
}
```

### Token Stream Output
```
[0] Keyword(Fn)
[1] Identifier(main)
[2] LeftParen
[3] RightParen
[4] LeftBrace
[5] Keyword(Let)
[6] Identifier(x)
[7] Equal
[8] Integer(42)
[9] Semicolon
[10] Identifier(println)
[11] Bang
[12] LeftParen
[13] String("{}")
[14] Comma
[15] Identifier(x)
[16] RightParen
[17] Semicolon
[18] RightBrace
[19] EOF
```

Notice:
- Comments removed (none in this example)
- Whitespace removed
- Keywords recognized
- Operators identified
- EOF marker added
- String literal captured

---

## Next Phase: Parser

The parser will:
1. Take this token stream
2. Verify it follows Rust grammar
3. Build an Abstract Syntax Tree (AST)

For example, from the tokens above, the parser will create:
```
Program
├─ Function "main"
│  └─ Body
│     ├─ VariableDeclaration
│     │  ├─ name: "x"
│     │  └─ value: Integer(42)
│     └─ FunctionCall
│        ├─ function: "println"
│        ├─ args: [String("{}"), Variable("x")]
└─ EOF
```

---

## Summary

| Aspect | Details |
|--------|---------|
| **Input** | Raw source code (String) |
| **Output** | Vector of Tokens |
| **Main Job** | Convert characters → meaningful tokens |
| **Error Handling** | LexError enum |
| **Complexity** | O(n) time, O(n) space |
| **Tests** | Unit tests in src/lexer/mod.rs |
| **Next Step** | Parser (Phase 2) |

**Phase 1 is complete and working!** ✅