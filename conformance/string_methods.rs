// String and str methods: searching, splitting from either end, trimming,
// case, UTF-8 conversion, and building strings with write!.
fn main() {
    let s = "Hello, Wörld! Hello again.";
    println!("{:?} {:?} {:?}", s.find("Hello"), s.rfind("Hello"), s.find(char::is_whitespace));
    println!("{:?}", s.char_indices().filter(|(_, c)| c.is_uppercase()).collect::<Vec<_>>());
    println!("{:?}", s.splitn(3, ' ').collect::<Vec<_>>());
    println!("{:?}", s.rsplit(' ').collect::<Vec<_>>());
    println!("{:?}", s.rsplitn(2, ' ').collect::<Vec<_>>());
    println!("{:?}", "--a--b--".trim_matches('-'));
    println!("{:?} {:?}", s.strip_prefix("Hello"), s.strip_suffix("x"));
    println!("{}", s.replace("Hello", "Bye").replacen("Bye", "Hi", 1));
    println!("{} {}", s.to_uppercase(), s.to_lowercase());
    println!("{} {} {}", s.len(), s.chars().count(), s.bytes().filter(|b| *b > 127).count());
    println!("{:?}", s.split_terminator('.').collect::<Vec<_>>());
    println!("{:?}", "a1b22c333".split(|c: char| c.is_ascii_digit()).filter(|p| !p.is_empty()).collect::<Vec<_>>());
    println!("{:?}", "line1\nline2\r\nline3\n".lines().collect::<Vec<_>>());
    println!("{} {} {}", s.starts_with("Hell"), s.ends_with('.'), s.contains("Wö"));
    println!("{:?} {:?}", s.get(0..5), s.get(8..9));
    println!("{}", &s[7..13]);
    let mut owned = String::from("abc");
    owned.insert(1, 'X'); owned.insert_str(0, ">>"); owned.push_str("!!");
    let popped = owned.pop(); let removed = owned.remove(0);
    println!("{} {:?} {}", owned, popped, removed);
    owned.truncate(4); owned.retain(|c| c != 'X');
    println!("{} {} {}", owned, owned.capacity() >= owned.len(), owned.is_empty());
    println!("{}", "ab".repeat(3) + &"-".repeat(2));
    let words = vec!["one", "two", "three"];
    println!("{} {}", words.concat(), words.join(", "));
    println!("{:?}", "Mary had a little lamb".split_whitespace().rev().collect::<Vec<&str>>().join(" "));
    println!("{:?} {:?}", "ß".to_uppercase(), "ǅ".to_lowercase());
    let num = 1234567;
    let with_commas: String = num.to_string().chars().rev().collect::<Vec<_>>().chunks(3).map(|c| c.iter().collect::<String>()).collect::<Vec<_>>().join(",").chars().rev().collect();
    println!("{}", with_commas);
    println!("{:?}", "abc".chars().rev().collect::<String>());
    println!("{:?}", "hello".bytes().map(|b| b as u32).sum::<u32>());
    println!("{:?}", String::from_utf8(vec![104, 105]).unwrap());
    println!("{:?}", String::from_utf8(vec![0xff, 0xfe]).is_err());
    println!("{:?}", String::from_utf8_lossy(&[72, 0xff, 73]));
    println!("{:?}", std::str::from_utf8(b"ok"));
    let c = "x=1;y=22;z=333";
    let kv: Vec<(&str, i32)> = c.split(';').filter_map(|p| p.split_once('=')).map(|(k, v)| (k, v.parse().unwrap())).collect();
    println!("{:?}", kv);
    println!("{:?}", "  padded  ".trim_start().trim_end_matches(' '));
    println!("{}", format!("{:>width$}", "r", width = 4));
    println!("{:?}", "AbC".chars().map(|c| if c.is_uppercase() { c.to_ascii_lowercase() } else { c.to_ascii_uppercase() }).collect::<String>());
    println!("{} {}", "apple" < "banana", "Zebra".cmp("apple") == std::cmp::Ordering::Less);
    println!("{:?}", "a-b_c d".split(&['-', '_', ' '][..]).collect::<Vec<_>>());
    println!("{:?}", "one two".matches('o').count());
    println!("{:?}", "abcdef".char_indices().nth(3));
    println!("{}", 'Z'.is_alphanumeric() && '9'.is_numeric() && !' '.is_alphabetic());
    println!("{:?}", "tschüß".escape_unicode().to_string().len());
    let mut s2 = String::with_capacity(4);
    for i in 0..3 { use std::fmt::Write; write!(s2, "{}-", i).unwrap(); }
    println!("{}", s2);
    println!("{:?}", "data".as_bytes().iter().rev().map(|&b| b as char).collect::<String>());
    println!("{:?}", "a,b,c".rsplit_once(','));
    println!("{}", "x".parse::<char>().unwrap());
}
