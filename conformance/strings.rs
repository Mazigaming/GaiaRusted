// String and &str operations.
fn shout(s: &str) -> String {
    let mut out = s.to_uppercase();
    out.push('!');
    out
}

fn main() {
    let mut s = String::new();
    s.push_str("hello");
    s.push(' ');
    s += "world";
    println!("{} {}", s, s.len());
    println!("{}", shout(&s));
    println!("{}", s.contains("lo w"));
    println!("{} {}", s.starts_with("hell"), s.ends_with("x"));
    println!("{:?}", s.find("world"));
    println!("{}", s.replace("l", "L"));
    println!("[{}]", "  padded \n".trim());
    println!("{}", &s[6..]);
    println!("{}", &s[..5]);
    let parts: Vec<&str> = "a,b,,c".split(",").collect();
    println!("{:?} {}", parts, parts.len());
    for (i, c) in "héllo".chars().enumerate() {
        print!("{}:{} ", i, c);
    }
    println!();
    println!("{}", "héllo".len());
    println!("{}", "abc".repeat(3));
    let n: i32 = "-42".parse().unwrap();
    println!("{}", n + 1);
    println!("{:?}", "4x".parse::<i32>().is_err());
    let f: f64 = "2.5".parse().unwrap();
    println!("{}", f * 2.0);
    println!("{}", s == "hello world");
    println!("{}", "abc" < "abd");
    let owned = s.clone() + "!";
    println!("{}", owned);
    println!("{:?}", "quote\"d\n");
    println!("{:>8}|{:<8}|{:^8}|", "r", "l", "c");
    println!("{:*^9}", "mid");
    println!("{}", format!("{}-{}", 1, "two"));
    let name = "inline";
    println!("{name} {0} {0}", 7);
    println!("{:?}", 'x');
    println!("{}", 42.to_string() + &1.5.to_string());
    let words = vec!["x".to_string(), "y".to_string()];
    println!("{}", words[0].as_str() == "x");
    println!("{{}}");
}
