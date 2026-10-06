fn main() {
    let s = "héllo wörld 日本";
    println!("{} {}", s.len(), s.chars().count());
    println!("{}", s.to_uppercase());
    println!("{:?}", s.chars().rev().collect::<String>());
    println!("{:?}", s.find('ö'));
    println!("{}", &s[0..3]);
    for (i, c) in s.char_indices().filter(|(_, c)| !c.is_ascii()) { print!("{}:{} ", i, c); }
    println!();
    println!("{}", s.is_char_boundary(2));
    let emoji = "🦀";
    println!("{} {} {:x}", emoji.len(), emoji.chars().count(), emoji.chars().next().unwrap() as u32);
    println!("{:?}", "ß".to_uppercase());
    println!("{}", 'Σ'.to_lowercase());
}
