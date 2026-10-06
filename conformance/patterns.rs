// Pattern matching in depth.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Number(i64),
    Op(char),
    Open,
    Close,
}

struct Config {
    name: &'static str,
    retries: u32,
    verbose: bool,
}

fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut number: Option<i64> = None;
    for c in text.chars() {
        match c {
            '0'..='9' => {
                let digit = c.to_digit(10).unwrap() as i64;
                number = Some(number.unwrap_or(0) * 10 + digit);
                continue;
            }
            _ => {}
        }
        if let Some(n) = number.take() {
            tokens.push(Token::Number(n));
        }
        match c {
            '+' | '-' | '*' | '/' => tokens.push(Token::Op(c)),
            '(' => tokens.push(Token::Open),
            ')' => tokens.push(Token::Close),
            ' ' => {}
            other => panic!("unexpected {}", other),
        }
    }
    if let Some(n) = number {
        tokens.push(Token::Number(n));
    }
    tokens
}

// Recursive descent over the token slice: returns (value, tokens consumed).
fn expr(tokens: &[Token]) -> (i64, usize) {
    let (mut value, mut used) = term(tokens);
    while let Some(Token::Op(op @ ('+' | '-'))) = tokens.get(used) {
        let (rhs, more) = term(&tokens[used + 1..]);
        value = if *op == '+' { value + rhs } else { value - rhs };
        used += 1 + more;
    }
    (value, used)
}

fn term(tokens: &[Token]) -> (i64, usize) {
    let (mut value, mut used) = atom(tokens);
    while let Some(Token::Op(op @ ('*' | '/'))) = tokens.get(used) {
        let (rhs, more) = atom(&tokens[used + 1..]);
        value = if *op == '*' { value * rhs } else { value / rhs };
        used += 1 + more;
    }
    (value, used)
}

fn atom(tokens: &[Token]) -> (i64, usize) {
    match tokens[0] {
        Token::Number(n) => (n, 1),
        Token::Open => {
            let (value, used) = expr(&tokens[1..]);
            (value, used + 2)
        }
        Token::Op('-') => {
            let (value, used) = atom(&tokens[1..]);
            (-value, used + 1)
        }
        other => panic!("unexpected {:?}", other),
    }
}

fn describe(config: &Config) -> String {
    match config {
        Config { verbose: true, name, .. } => format!("{} (verbose)", name),
        Config { retries: 0, name, .. } => format!("{} (no retries)", name),
        Config { retries: n @ 1..=3, name, .. } => format!("{} ({} retries)", name, n),
        Config { name, retries, .. } => format!("{} (many: {})", name, retries),
    }
}

fn point_kind(point: (i32, i32)) -> &'static str {
    match point {
        (0, 0) => "origin",
        (0, _) | (_, 0) => "axis",
        (x, y) if x == y => "diagonal",
        (x, _) if x < 0 => "left",
        _ => "elsewhere",
    }
}

fn main() {
    let tokens = tokenize("2 * (3 + 4) - 10 / 5");
    println!("{}", tokens.len());
    println!("{:?} {:?}", tokens[0], tokens[2]);
    println!("{}", expr(&tokens).0);
    println!("{}", expr(&tokenize("-(1 + 2) * 3")).0);

    let configs = [
        Config { name: "a", retries: 5, verbose: true },
        Config { name: "b", retries: 0, verbose: false },
        Config { name: "c", retries: 2, verbose: false },
        Config { name: "d", retries: 9, verbose: false },
    ];
    for config in &configs {
        println!("{}", describe(config));
    }
    for p in [(0, 0), (0, 5), (3, 3), (-1, 2), (4, 1)] {
        println!("{}", point_kind(p));
    }

    let mut stack = vec![1, 2, 3];
    while let Some(top) = stack.pop() {
        print!("{} ", top);
    }
    println!();

    let nested = Some(Some(7));
    match nested {
        Some(Some(n)) if n > 5 => println!("big {}", n),
        Some(Some(n)) => println!("small {}", n),
        Some(None) | None => println!("nothing"),
    }
    let value = 42u8;
    let text = match value {
        0 => "zero",
        n if n % 2 == 0 => "even",
        _ => "odd",
    };
    println!("{}", text);
    let &(a, ref b) = &(1, String::from("two"));
    println!("{} {}", a, b);
    let mut pair = (1, 2);
    let (ref mut first, _) = pair;
    *first = 10;
    println!("{:?}", pair);
    println!("{}", matches!(tokens[1], Token::Op('*')));
    let words = ["alpha", "beta", "gamma"];
    for (i, word) in words.iter().enumerate() {
        match (i, *word) {
            (0, w) => println!("first {}", w),
            (_, "gamma") => println!("found gamma"),
            _ => {}
        }
    }
}
