// Character-level parsing into an enum: tokenize generated JSON-like text.
#[derive(Debug, PartialEq)]
enum Token { LBrace, RBrace, LBracket, RBracket, Colon, Comma, Str(String), Num(f64), True, False, Null }
fn tokenize(src: &str) -> Vec<Token> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '{' => out.push(Token::LBrace), '}' => out.push(Token::RBrace),
            '[' => out.push(Token::LBracket), ']' => out.push(Token::RBracket),
            ':' => out.push(Token::Colon), ',' => out.push(Token::Comma),
            '"' => { let start = i + 1; i += 1; while chars[i] != '"' { i += 1; } out.push(Token::Str(chars[start..i].iter().collect())); }
            c if c.is_ascii_digit() || c == '-' => {
                let start = i;
                while i + 1 < chars.len() && (chars[i + 1].is_ascii_digit() || chars[i + 1] == '.') { i += 1; }
                let text: String = chars[start..=i].iter().collect();
                out.push(Token::Num(text.parse().unwrap()));
            }
            't' => { out.push(Token::True); i += 3; }
            'f' => { out.push(Token::False); i += 4; }
            'n' => { out.push(Token::Null); i += 3; }
            _ => {}
        }
        i += 1;
    }
    out
}
fn main() {
    let mut doc = String::from("[");
    for i in 0..60_000 {
        if i > 0 { doc.push(','); }
        doc.push_str(&format!("{{\"id\": {}, \"name\": \"item{}\", \"price\": {}.{}, \"ok\": {}, \"tags\": [null, false]}}", i, i % 97, i % 1000, i % 10, i % 3 == 0));
    }
    doc.push(']');
    let tokens = tokenize(&doc);
    let strings = tokens.iter().filter(|t| matches!(t, Token::Str(_))).count();
    let sum: f64 = tokens.iter().map(|t| if let Token::Num(n) = t { *n } else { 0.0 }).sum();
    println!("{} tokens, {} strings, sum {:.1}", tokens.len(), strings, sum);
}
