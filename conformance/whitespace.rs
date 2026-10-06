// Splitting at whitespace: ASCII and Unicode spaces for `split_whitespace`,
// ASCII ones only for `split_ascii_whitespace`.
fn main() {
    let text = "  alpha\tbeta\n\ngamma\u{a0}delta\u{3000}eps\x0bzeta  \u{e9} \u{fc}\r\n";
    let words: Vec<&str> = text.split_whitespace().collect();
    println!("{} {:?}", words.len(), &words[..6]);
    let ascii: Vec<usize> = text.split_ascii_whitespace().map(str::len).collect();
    println!("{:?}", ascii);
    println!("{} {:?}", "".split_whitespace().count(), "   ".split_whitespace().next());
    println!("{:?}", "a b  c".split_whitespace().rev().collect::<Vec<_>>());
    let line = "  12  -7 300\t42 ";
    let total: i64 = line.split_whitespace().map(|n| n.parse::<i64>().unwrap()).sum();
    println!("{}", total);
}
