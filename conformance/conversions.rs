// Conversions: From/Into, TryFrom/TryInto, FromStr for one's own types,
// formatting traits with padding, and the standard library's conversions
// between numbers, strings, vectors, arrays and bytes.
use std::convert::TryFrom;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Celsius(f64);

#[derive(Debug, Clone, Copy, PartialEq)]
struct Fahrenheit(f64);

impl From<Celsius> for Fahrenheit {
    fn from(c: Celsius) -> Self {
        Fahrenheit(c.0 * 9.0 / 5.0 + 32.0)
    }
}

#[derive(Debug)]
struct Even(u32);

impl TryFrom<u32> for Even {
    type Error = String;
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        if v % 2 == 0 { Ok(Even(v)) } else { Err(format!("{} is odd", v)) }
    }
}

struct Hex(u32);

impl fmt::Display for Hex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(&format!("0x{:x}", self.0))
    }
}

impl fmt::LowerHex for Hex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct Version {
    major: u8,
    minor: u8,
    patch: u8,
}

impl std::str::FromStr for Version {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("bad version {}", s));
        }
        let n = |p: &str| p.parse::<u8>().map_err(|e| e.to_string());
        Ok(Version { major: n(parts[0])?, minor: n(parts[1])?, patch: n(parts[2])? })
    }
}

fn describe<T: Into<String>>(s: T) -> String {
    let s: String = s.into();
    format!("<{}>", s)
}

fn main() {
    let f: Fahrenheit = Celsius(100.0).into();
    println!("{:?} {:?}", f, Fahrenheit::from(Celsius(-40.0)));
    println!("{:?} {:?}", Even::try_from(4), Even::try_from(5));
    let e: Result<Even, _> = 8u32.try_into();
    println!("{:?}", e);
    println!("[{:>8}] [{:<8}] [{:x}] [{:#010x}]", Hex(255), Hex(16), Hex(255), Hex(255));
    let mut versions: Vec<Version> = ["1.2.3", "0.9.12", "1.10.0", "1.2.0"].iter().map(|s| s.parse().unwrap()).collect();
    versions.sort();
    println!("{:?}", versions.iter().map(|v| format!("{}.{}.{}", v.major, v.minor, v.patch)).collect::<Vec<_>>());
    println!("{:?} {:?}", "1.2".parse::<Version>(), "1.x.3".parse::<Version>());
    println!("{} {} {}", describe("str"), describe(String::from("string")), describe('c'.to_string()));
    println!("{} {} {}", i64::from(7u32), u64::from(true), f64::from(3u8));
    println!("{:?} {:?}", u8::try_from(256i32).is_err(), i16::try_from(-5i64));
    let v: Vec<u8> = "abc".into();
    let s: String = 'x'.into();
    let b: Box<str> = "boxed".into();
    println!("{:?} {} {}", v, s, b);
    let arr: [u8; 3] = vec![1, 2, 3].try_into().unwrap();
    let back: Vec<u8> = arr.into();
    println!("{:?} {:?}", arr, back);
    let n: Option<u32> = "12".parse().ok();
    let total: u64 = n.map(u64::from).unwrap_or(0) + 1;
    println!("{}", total);
    let chars: Vec<char> = vec!['h', 'i'];
    let word: String = chars.iter().collect();
    let back2: String = String::from_iter(['o', 'k']);
    println!("{} {}", word, back2);
    println!("{}", char::from_u32(0x1F600).map(|c| c.len_utf8()).unwrap_or(0));
    println!("{:?}", (b'a'..=b'e').map(char::from).collect::<String>());
    println!("{:?}", u32::from_be_bytes([0, 0, 1, 2]));
    println!("{:?}", 258u16.to_le_bytes());
}
