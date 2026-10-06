#[derive(Debug, PartialEq, Clone, Copy)]
enum Dir { North, East, South, West }
impl Dir {
    fn turn(self) -> Dir { match self { Dir::North => Dir::East, Dir::East => Dir::South, Dir::South => Dir::West, Dir::West => Dir::North } }
}
#[derive(Debug)]
enum Shape { Circle { r: f64 }, Rect(f64, f64), Empty }
fn area(s: &Shape) -> f64 {
    match s { Shape::Circle { r } => 3.0 * r * r, Shape::Rect(w, h) => w * h, Shape::Empty => 0.0 }
}
enum Code { Ok = 200, NotFound = 404, Teapot = 418 }
fn main() {
    let mut d = Dir::North;
    for _ in 0..5 { d = d.turn(); }
    println!("{:?} {}", d, d == Dir::East);
    let shapes = vec![Shape::Circle { r: 2.0 }, Shape::Rect(3.0, 4.0), Shape::Empty];
    for s in &shapes { println!("{:?} {}", s, area(s)); }
    println!("{} {} {}", Code::Ok as i32, Code::NotFound as i32, Code::Teapot as u16);
    let maybe: Option<Dir> = Some(Dir::West);
    if let Some(Dir::West) = maybe { println!("west"); }
    println!("{}", std::mem::size_of::<Dir>());
}
