use std::io::{self, BufRead, Read, Write};
fn main() {
    let stdin = io::stdin();
    let mut first = String::new();
    stdin.lock().read_line(&mut first).unwrap();
    let n: usize = first.trim().parse().unwrap();
    let mut nums = Vec::new();
    for line in stdin.lock().lines().take(n) { let line = line.unwrap(); nums.extend(line.split_whitespace().map(|t| t.parse::<i64>().unwrap())); }
    let mut rest = String::new();
    io::stdin().read_to_string(&mut rest).unwrap();
    let out = io::stdout();
    let mut w = io::BufWriter::new(out.lock());
    writeln!(w, "sum {}", nums.iter().sum::<i64>()).unwrap();
    writeln!(w, "max {:?}", nums.iter().max()).unwrap();
    writeln!(w, "rest {:?}", rest.trim()).unwrap();
    w.flush().unwrap();
}
