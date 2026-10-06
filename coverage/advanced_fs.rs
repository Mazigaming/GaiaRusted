use std::fs;
use std::io::{Write, BufRead, BufReader};
fn main() {
    let path = std::env::temp_dir().join("gaia_coverage_fs_test.txt");
    {
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "alpha").unwrap();
        writeln!(f, "beta 2").unwrap();
    }
    let text = fs::read_to_string(&path).unwrap();
    println!("{:?}", text);
    let f = fs::File::open(&path).unwrap();
    for line in BufReader::new(f).lines() { println!("line {}", line.unwrap()); }
    fs::write(&path, b"overwritten").unwrap();
    println!("{}", fs::read_to_string(&path).unwrap());
    println!("{}", fs::metadata(&path).unwrap().len());
    fs::remove_file(&path).unwrap();
    println!("{}", path.exists());
    println!("{}", fs::read_to_string("/definitely/not/here").is_err());
}
