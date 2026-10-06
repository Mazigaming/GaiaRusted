use std::fmt;
struct Money { cents: i64 }
impl fmt::Display for Money { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "${}.{:02}", self.cents / 100, self.cents % 100) } }
#[derive(Debug)] struct Inner { id: u8, tags: Vec<&'static str> }
#[derive(Debug)] struct Outer { name: String, inner: Inner, pair: (i32, f64), opt: Option<Box<i32>> }
#[derive(Debug)] enum E { A, B(i32), C { x: u8 } }
struct Matrix([[i32; 2]; 2]);
impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in &self.0 { writeln!(f, "{} {}", row[0], row[1])?; }
        Ok(())
    }
}
fn main() {
    println!("{}", Money { cents: 12345 });
    let o = Outer { name: "o".into(), inner: Inner { id: 1, tags: vec!["a", "b"] }, pair: (3, 1.5), opt: Some(Box::new(9)) };
    println!("{:?}", o);
    println!("{:#?}", o);
    println!("{:?} {:?} {:?}", E::A, E::B(4), E::C { x: 2 });
    print!("{}", Matrix([[1, 2], [3, 4]]));
    let s = format!("{:>10}|{:<6}|{:^7}|", Money { cents: 5 }, "ab", "mid");
    println!("{}", s);
    println!("{:?}", "quote\"s\n");
    println!("{:?}", 'c');
}
