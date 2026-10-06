type Km = f64;
type Result2<T> = std::result::Result<T, String>;
struct Meters(f64);
impl Meters { fn to_km(&self) -> Km { self.0 / 1000.0 } }
fn check(x: i32) -> Result2<i32> { if x > 0 { Ok(x) } else { Err(format!("bad {}", x)) } }
fn main() {
    let d: Km = 4.5;
    println!("{}", d * 2.0);
    println!("{}", Meters(2500.0).to_km());
    println!("{:?} {:?}", check(3), check(-1));
}
