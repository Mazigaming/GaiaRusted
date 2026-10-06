// Standard input read line by line and in bulk, buffered output, and files.
use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};

fn main() {
    let stdin = io::stdin();
    let mut header = String::new();
    stdin.lock().read_line(&mut header).unwrap();
    let rows: usize = header.trim().parse().unwrap();
    let mut table = Vec::new();
    for line in stdin.lock().lines().take(rows) {
        let numbers: Vec<i64> = line.unwrap().split_whitespace().map(|word| word.parse().unwrap()).collect();
        table.push(numbers);
    }
    let mut rest = String::new();
    io::stdin().read_to_string(&mut rest).unwrap();

    let out = io::stdout();
    let mut writer = BufWriter::new(out.lock());
    for (index, row) in table.iter().enumerate() {
        writeln!(writer, "row {}: sum {} max {:?}", index, row.iter().sum::<i64>(), row.iter().max()).unwrap();
    }
    writeln!(writer, "rest {:?}", rest.lines().collect::<Vec<_>>()).unwrap();
    writer.flush().unwrap();

    let path = std::env::temp_dir().join("gaiarusted_conformance_io.txt");
    {
        let mut file = fs::File::create(&path).unwrap();
        for row in &table {
            let line: Vec<String> = row.iter().map(|n| n.to_string()).collect();
            writeln!(file, "{}", line.join(",")).unwrap();
        }
    }
    let file = fs::File::open(&path).unwrap();
    let parsed: Vec<usize> = BufReader::new(file).lines().map(|line| line.unwrap().split(',').count()).collect();
    println!("{:?} {} bytes", parsed, fs::metadata(&path).unwrap().len());
    fs::remove_file(&path).unwrap();
    println!("{} {}", path.exists(), fs::read_to_string(&path).is_err());
    let error = fs::File::open("/no/such/file").unwrap_err();
    println!("{:?} | {}", error.kind(), error);
    let mut cursor = io::Cursor::new(b"one\ntwo\n".to_vec());
    let mut first = String::new();
    cursor.read_line(&mut first).unwrap();
    println!("{:?} {}", first, cursor.position());
}
