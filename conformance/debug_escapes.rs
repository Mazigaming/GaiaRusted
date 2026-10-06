// How {:?} and the escape_* methods write characters: control and
// unassigned ones, unusual spaces, and combining marks are escaped.
fn main() {
    let samples = [
        "plain ascii", "tab\there", "quote \" and ' apostrophe", "back\\slash", "nul\0",
        "nbsp\u{a0}x", "ideographic\u{3000}space", "e\u{301} combining", "\u{301}leading mark",
        "emoji 😀 ok", "zero\u{200b}width", "bidi\u{202e}x", "private\u{e000}use",
        "unassigned\u{378}", "max\u{10ffff}", "cjk 漢字", "rtl עברית", "del\u{7f}", "c1\u{85}x",
        "soft\u{ad}hyphen", "bom\u{feff}", "variation\u{fe0f}", "tag\u{e0041}",
    ];
    for sample in samples {
        println!("{:?}", sample);
    }
    for c in ['\u{a0}', '\'', '"', '\u{301}', 'é', '\u{2028}', '\u{e000}', '\u{1f600}', '\u{10fffe}', ' '] {
        println!("{:?} {}", c, c.escape_debug());
    }
    println!("{:?}", vec!["a\u{a0}b", "c"]);
    println!("{:?}", Some('\u{3000}'));
    println!("{} {} {}", "\u{301}a\u{301}\t\"\u{a0}".escape_debug(), "é\n\x01".escape_default(), "ab".escape_unicode());
    println!("{}", 'é'.escape_default().count() + 'x'.escape_unicode().len());
}
