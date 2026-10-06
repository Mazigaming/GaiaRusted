// Character properties and case mapping across all of Unicode, from the same
// tables as rustc's core: a digest over every 97th code point, then cases
// that need care (title case, multi-character mappings, final sigma).
fn summary(c: char) -> String {
    let flags = [c.is_alphabetic(), c.is_numeric(), c.is_alphanumeric(), c.is_lowercase(), c.is_uppercase(),
                 c.is_whitespace(), c.is_control()];
    flags.iter().map(|&f| if f { '1' } else { '0' }).collect()
}
fn main() {
    // Every 97th code point, so all planes are touched.
    let mut digest = 0u64;
    let mut code = 0u32;
    while code <= 0x10FFFF {
        if let Some(c) = char::from_u32(code) {
            let s = summary(c);
            for (i, b) in s.bytes().enumerate() { digest = digest.wrapping_mul(31).wrapping_add((b as u64) << i); }
            for u in c.to_uppercase() { digest = digest.wrapping_mul(131).wrapping_add(u as u64); }
            for l in c.to_lowercase() { digest = digest.wrapping_mul(137).wrapping_add(l as u64); }
        }
        code += 97;
    }
    println!("digest {}", digest);
    for c in ['a', 'Z', 'ß', 'İ', 'ǅ', 'Ⅻ', '½', '٣', '\u{a0}', '\u{85}', '\u{2028}', 'ﬀ', 'ŉ', 'Σ', 'ς', 'ﬃ', '\u{1f600}', '𝔄', 'ꭰ'] {
        let upper: String = c.to_uppercase().collect();
        let lower: String = c.to_lowercase().collect();
        println!("{:?} {} {:?} {:?} {}", c, summary(c), upper, lower, c.to_uppercase().count());
    }
    for text in ["ΟΔΟΣ", "ΣΑΣ ΟΔΟΣ.", "Σ", "aΣ'b", "ΑΣ'", "straße", "İstanbul", "ǅemal", "MAŇANA"] {
        println!("{} {} {}", text.to_lowercase(), text.to_uppercase(), text.chars().filter(|c| c.is_uppercase()).count());
    }
}
