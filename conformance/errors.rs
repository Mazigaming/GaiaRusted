// Option and Result: combinators, `?`, custom error types and conversions.
use std::fmt;

#[derive(Debug)]
enum ParseError {
    Empty,
    BadNumber(String),
    Negative(i64),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "empty input"),
            ParseError::BadNumber(text) => write!(f, "`{}` is not a number", text),
            ParseError::Negative(n) => write!(f, "{} is negative", n),
        }
    }
}

#[derive(Debug)]
struct AppError {
    message: String,
}

impl From<ParseError> for AppError {
    fn from(error: ParseError) -> AppError {
        AppError { message: format!("parse failed: {}", error) }
    }
}

fn parse_positive(text: &str) -> Result<i64, ParseError> {
    if text.is_empty() {
        return Err(ParseError::Empty);
    }
    let value: i64 = text.parse().map_err(|_| ParseError::BadNumber(text.to_string()))?;
    if value < 0 {
        Err(ParseError::Negative(value))
    } else {
        Ok(value)
    }
}

fn sum_all(inputs: &[&str]) -> Result<i64, AppError> {
    let mut total = 0;
    for input in inputs {
        total += parse_positive(input)?;
    }
    Ok(total)
}

fn middle(items: &[i32]) -> Option<i32> {
    let first = items.first()?;
    let last = items.last()?;
    Some((first + last) / 2)
}

fn main() {
    println!("{:?}", parse_positive("42"));
    for bad in ["", "x1", "-3"] {
        match parse_positive(bad) {
            Ok(v) => println!("ok {}", v),
            Err(e) => println!("error: {}", e),
        }
    }
    println!("{:?}", sum_all(&["1", "2", "3"]).unwrap());
    match sum_all(&["1", "oops"]) {
        Ok(_) => println!("unexpected"),
        Err(e) => println!("{}", e.message),
    }

    println!("{:?} {:?}", middle(&[10, 20, 30]), middle(&[]));
    let some = Some(4);
    println!("{:?}", some.map(|n| n * 2).and_then(|n| if n > 5 { Some(n) } else { None }));
    println!("{}", None.unwrap_or(7));
    println!("{}", some.is_some() && !some.is_none());
    println!("{:?}", some.ok_or("missing"));
    let results: Vec<Result<i32, String>> = vec![Ok(1), Err("bad".to_string())];
    for r in &results {
        println!("{} {:?}", r.is_ok(), r);
    }
    let fallback = Err::<i32, &str>("nope").unwrap_or_else(|e| e.len() as i32);
    println!("{}", fallback);
    let mut slot = Some(3);
    println!("{:?} {:?}", slot.take(), slot);
    if let Some(x) = some {
        println!("got {}", x);
    }
    let Some(y) = some else { return };
    println!("{}", y);
}
