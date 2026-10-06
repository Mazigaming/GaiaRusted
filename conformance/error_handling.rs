// Errors: custom error types with Display and source, ? with From
// conversions, Box<dyn Error>, collecting into Result and Option, and the
// Option and Result combinators.
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
enum AppError {
    Parse(ParseIntError),
    Negative(i64),
    Empty,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Parse(e) => write!(f, "parse error: {}", e),
            AppError::Negative(n) => write!(f, "negative: {}", n),
            AppError::Empty => f.write_str("empty input"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AppError::Parse(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ParseIntError> for AppError {
    fn from(e: ParseIntError) -> Self {
        AppError::Parse(e)
    }
}

fn parse_positive(s: &str) -> Result<i64, AppError> {
    if s.is_empty() {
        return Err(AppError::Empty);
    }
    let n: i64 = s.trim().parse()?;
    if n < 0 { Err(AppError::Negative(n)) } else { Ok(n) }
}

fn sum_all(items: &[&str]) -> Result<i64, Box<dyn Error>> {
    let mut total = 0;
    for item in items {
        total += parse_positive(item)?;
    }
    if total > 100 {
        return Err("total too large".into());
    }
    Ok(total)
}

fn first_char_upper(s: &str) -> Option<char> {
    let c = s.chars().next()?;
    Some(c.to_ascii_uppercase())
}

fn main() {
    for input in ["42", " 7 ", "-3", "", "x1"] {
        match parse_positive(input) {
            Ok(n) => println!("ok {}", n),
            Err(e) => println!("err {} / {:?} / source: {}", e, e, e.source().map(|s| s.to_string()).unwrap_or_default()),
        }
    }
    println!("{:?}", sum_all(&["1", "2", "3"]).ok());
    match sum_all(&["60", "50"]) {
        Err(e) => println!("{}", e),
        Ok(v) => println!("{}", v),
    }
    match sum_all(&["1", "bad"]) {
        Err(e) => println!("{}", e),
        Ok(v) => println!("{}", v),
    }
    println!("{:?} {:?}", first_char_upper("hello"), first_char_upper(""));
    let results: Result<Vec<i32>, _> = "1 2 3".split(' ').map(|s| s.parse::<i32>()).collect();
    let bad: Result<Vec<i32>, _> = "1 x 3".split(' ').map(|s| s.parse::<i32>()).collect();
    println!("{:?} {:?}", results, bad.map_err(|e| e.to_string()));
    let opts: Option<Vec<u32>> = vec![Some(1), Some(2)].into_iter().collect();
    println!("{:?}", opts);
    let r: Result<u8, String> = Err("boom".into());
    println!("{} {} {:?}", r.clone().unwrap_or(9), r.clone().unwrap_or_default(), r.as_ref().err());
    println!("{:?} {:?}", r.clone().ok(), Ok::<u8, String>(3).and_then(|v| if v > 2 { Ok(v * 2) } else { Err("small".to_string()) }));
    let o: Option<i32> = Some(4);
    println!("{:?} {:?} {:?} {:?}", o.filter(|v| v % 2 == 0), o.xor(None), o.zip(Some('a')), o.ok_or("none"));
    println!("{:?} {:?}", o.map_or(0, |v| v * 3), None::<i32>.map_or_else(|| -1, |v| v));
    let mut maybe = Some(3);
    if let Some(v) = maybe.as_mut() {
        *v += 1;
    }
    let taken = maybe.take();
    println!("{:?} {:?}", taken, maybe);
    println!("{:?}", maybe.get_or_insert(10));
    let e: Box<dyn Error + Send + Sync> = From::from("converted");
    println!("{}", e);
    let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing file");
    println!("{} {:?}", io, io.kind());
}
