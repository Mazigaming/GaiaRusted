const MAX: usize = 3;
const NAMES: [&str; MAX] = ["a", "b", "c"];
static GREETING: &str = "hello";
static TABLE: [u8; 4] = [1, 2, 4, 8];
const fn cube(x: u64) -> u64 { x * x * x }
const CUBE: u64 = cube(3);
struct Limits;
impl Limits { const LOW: i32 = -5; const HIGH: i32 = Self::LOW * -2; }
fn main() {
    println!("{} {:?}", MAX, NAMES);
    println!("{} {:?}", GREETING, TABLE);
    println!("{} {}", CUBE, cube(4));
    println!("{} {}", Limits::LOW, Limits::HIGH);
    let arr = [0; MAX * 2];
    println!("{}", arr.len());
    println!("{}", TABLE.iter().map(|&x| x as u32).sum::<u32>());
}
