fn main() {
    let c = 'a';
    println!("{} {} {}", c.is_alphabetic(), c.is_numeric(), c.is_alphanumeric());
    println!("{} {}", c.to_ascii_uppercase(), 'Z'.to_ascii_lowercase());
    println!("{:?} {:?}", '7'.to_digit(10), 'f'.to_digit(16));
    println!("{:?}", std::char::from_digit(9, 10));
    println!("{} {}", 'A' as u32, 97u8 as char);
    println!("{:?}", char::from_u32(0x41));
    println!("{} {}", ' '.is_whitespace(), 'x'.is_uppercase());
    println!("{}", ('a'..='e').collect::<String>());
    println!("{}", 'é'.len_utf8());
    let b = true;
    println!("{} {} {} {}", b && false, b || false, !b, b ^ true);
    println!("{}", b as i32);
    println!("{}", 'a' < 'b');
    for ch in "hey".chars() { print!("{} ", ch as u8); }
    println!();
    println!("{}", char::from(65u8));
    println!("{}", 'q'.is_ascii_lowercase());
}
