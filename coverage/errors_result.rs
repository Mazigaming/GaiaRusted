use std::fmt;
use std::error::Error;
#[derive(Debug)]
enum BankError { Insufficient { needed: u32, available: u32 }, Frozen }
impl fmt::Display for BankError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self { BankError::Insufficient { needed, available } => write!(f, "need {} have {}", needed, available), BankError::Frozen => write!(f, "frozen") }
    }
}
impl Error for BankError {}
fn withdraw(balance: u32, amt: u32, frozen: bool) -> Result<u32, BankError> {
    if frozen { return Err(BankError::Frozen); }
    if amt > balance { Err(BankError::Insufficient { needed: amt, available: balance }) } else { Ok(balance - amt) }
}
fn parse_sum(a: &str, b: &str) -> Result<i32, std::num::ParseIntError> { Ok(a.parse::<i32>()? + b.parse::<i32>()?) }
fn dynamic(s: &str) -> Result<i32, Box<dyn Error>> { let n: i32 = s.parse()?; if n < 0 { return Err(Box::new(BankError::Frozen)); } Ok(n * 2) }
fn main() {
    println!("{:?}", withdraw(100, 30, false));
    match withdraw(10, 30, false) { Ok(b) => println!("ok {}", b), Err(e) => println!("error: {}", e) }
    println!("{}", withdraw(1, 1, true).unwrap_err());
    println!("{:?} {}", parse_sum("2", "3"), parse_sum("2", "x").is_err());
    match dynamic("21") { Ok(v) => println!("{}", v), Err(e) => println!("{}", e) }
    match dynamic("-1") { Ok(v) => println!("{}", v), Err(e) => println!("{}", e) }
    match dynamic("q") { Ok(v) => println!("{}", v), Err(e) => println!("{}", e) }
    let r: Result<i32, String> = Err("bad".into());
    println!("{:?} {}", r.clone().map_err(|e| e.len()), r.clone().unwrap_or(0));
    println!("{:?}", Ok::<i32, String>(3).and_then(|x| if x > 2 { Ok(x * 2) } else { Err("small".to_string()) }));
    println!("{:?}", r.as_ref().ok());
    let all: Result<Vec<i32>, _> = "1 2 3".split(' ').map(|s| s.parse::<i32>()).collect();
    println!("{:?}", all);
    let bad: Result<Vec<i32>, _> = "1 x 3".split(' ').map(|s| s.parse::<i32>()).collect();
    println!("{}", bad.is_err());
    println!("{}", Ok::<u8, ()>(5).is_ok_and(|v| v > 3));
    println!("{}", parse_sum("1", "1").expect("valid"));
}
