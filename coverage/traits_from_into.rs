use std::fmt;
#[derive(Debug)] struct Celsius(f64);
#[derive(Debug)] struct Fahrenheit(f64);
impl From<Celsius> for Fahrenheit { fn from(c: Celsius) -> Self { Fahrenheit(c.0 * 9.0 / 5.0 + 32.0) } }
#[derive(Debug)] enum AppError { Parse(String), Neg(i64) }
impl From<std::num::ParseIntError> for AppError { fn from(e: std::num::ParseIntError) -> Self { AppError::Parse(e.to_string()) } }
impl fmt::Display for AppError { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { match self { AppError::Parse(s) => write!(f, "parse: {}", s), AppError::Neg(n) => write!(f, "negative: {}", n) } } }
fn parse_pos(s: &str) -> Result<i64, AppError> { let n: i64 = s.parse()?; if n < 0 { return Err(AppError::Neg(n)); } Ok(n) }
fn takes_string<S: Into<String>>(s: S) -> usize { s.into().len() }
fn main() {
    let f: Fahrenheit = Celsius(100.0).into();
    println!("{:?}", f);
    println!("{:?}", Fahrenheit::from(Celsius(0.0)));
    for s in ["42", "-3", "x"] { match parse_pos(s) { Ok(n) => println!("ok {}", n), Err(e) => println!("err {}", e) } }
    println!("{} {}", takes_string("abc"), takes_string(String::from("hello")));
    let v: Vec<i32> = Vec::from([1, 2, 3]);
    let s: String = String::from('x');
    let b: Box<str> = "boxed".into();
    println!("{:?} {} {}", v, s, b);
}
