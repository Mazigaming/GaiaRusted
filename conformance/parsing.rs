// Parsing text as numbers, chars and bools exactly where Rust accepts it,
// with Rust's error values and messages.
fn main() {
    for t in ["1.5", "-0.25", "+3", ".5", "5.", "1e3", "2.5E-2", "inf", "-Infinity", "NaN", "0x1A", "nan(1)", "1e", ".", "", " 1", "1 ", "1_000", "+-1", "1.7976931348623157e309", "4.9e-324", "0.1"] {
        println!("{:?} {:?} {:?}", t, t.parse::<f64>(), t.parse::<f32>());
    }
    for t in ["x", "", "ab", "é"] { println!("{:?}", t.parse::<char>()); }
    for t in ["true", "false", "True", ""] { println!("{:?}", t.parse::<bool>().map_err(|e| e.to_string())); }
    println!("{:?} {}", "s".parse::<String>(), "".parse::<char>().unwrap_err());
    println!("{}", 16777217.0f64 as f32 == "16777217".parse::<f32>().unwrap());
    let b: Box<dyn std::error::Error> = Box::new("q".parse::<bool>().unwrap_err());
    println!("{}", b);
}
