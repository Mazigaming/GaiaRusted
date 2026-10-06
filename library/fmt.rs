//! Formatting values as text.
//!
//! `println!("{} {:?}", a, b)` becomes calls of `Display::fmt(&a, f)` and
//! `Debug::fmt(&b, f)`: a type is printable because it implements these
//! traits, with no special support from the compiler.

/// Formatting failed; the reason is not recorded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Error;

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.write_str("an error occurred when formatting an argument")
    }
}

pub type Result = std::result::Result<(), Error>;

const FLAG_SIGN_PLUS: u32 = 1;
const FLAG_ALTERNATE: u32 = 2;
const FLAG_ZERO_PAD: u32 = 4;
const FLAG_HAS_WIDTH: u32 = 8;
const FLAG_HAS_PRECISION: u32 = 16;

const ALIGN_LEFT: u8 = 1;
const ALIGN_CENTER: u8 = 2;
const ALIGN_RIGHT: u8 = 3;

/// A width or precision taken from an argument (`{:>1$}`), which is a `usize`.
pub fn count_argument(count: usize) -> usize {
    count
}

/// The options of a placeholder: `{:*>8.2}` and the like.
pub struct Spec {
    fill: char,
    align: u8,
    flags: u32,
    width: usize,
    precision: usize,
}

/// Where formatted text is collected, together with the options of the
/// placeholder currently being formatted (`{:>8.2}`).
///
/// `{:#?}` lays nested values out one per line, each level indented by
/// four spaces. The formatter keeps the current level and indents every
/// line written while it is above zero, so a value's `Debug` impl writes
/// the same text at any depth.
pub struct Formatter {
    buffer: String,
    fill: char,
    align: u8,
    flags: u32,
    width: usize,
    precision: usize,
    indent: usize,
    at_line_start: bool,
}

impl Formatter {
    pub fn new() -> Formatter {
        Formatter { buffer: String::new(), fill: ' ', align: 0, flags: 0, width: 0, precision: 0, indent: 0, at_line_start: false }
    }

    /// Append text, indenting each new line to the current level.
    fn emit(&mut self, text: &str) {
        if self.indent == 0 {
            self.buffer.push_str(text);
            self.at_line_start = text.ends_with("\n");
            return;
        }
        for character in text.chars() {
            self.emit_char(character);
        }
    }

    fn emit_char(&mut self, character: char) {
        if self.at_line_start && character != '\n' {
            let mut level = 0;
            while level < self.indent {
                self.buffer.push_str("    ");
                level += 1;
            }
        }
        self.buffer.push(character);
        self.at_line_start = character == '\n';
    }

    /// Hand over the text written so far.
    pub fn finish(&mut self) -> String {
        std::mem::replace(&mut self.buffer, String::new())
    }

    pub fn set_spec(&mut self, fill: char, align: u8, flags: u32, width: usize, precision: usize) {
        self.fill = fill;
        self.align = align;
        self.flags = flags;
        self.width = width;
        self.precision = precision;
    }

    pub fn clear_spec(&mut self) {
        self.set_spec(' ', 0, 0, 0, 0);
    }

    /// The options of the current placeholder, which are cleared. `write!`
    /// into a formatter formats its own placeholders with their own options
    /// and gives the caller's back with [`restore_spec`](Self::restore_spec).
    pub fn take_spec(&mut self) -> Spec {
        let spec = Spec { fill: self.fill, align: self.align, flags: self.flags, width: self.width, precision: self.precision };
        self.clear_spec();
        spec
    }

    pub fn restore_spec(&mut self, spec: Spec) {
        self.set_spec(spec.fill, spec.align, spec.flags, spec.width, spec.precision);
    }

    pub fn write_str(&mut self, text: &str) -> Result {
        self.emit(text);
        Ok(())
    }

    pub fn write_char(&mut self, character: char) -> Result {
        self.emit_char(character);
        Ok(())
    }

    /// `Name { field: value, .. }`, built field by field.
    pub fn debug_struct(&mut self, name: &str) -> DebugStruct {
        self.emit(name);
        DebugStruct { formatter: self, has_fields: false }
    }

    /// `Name(value, ..)`, or `(value, ..)` with an empty name.
    pub fn debug_tuple(&mut self, name: &str) -> DebugTuple {
        self.emit(name);
        DebugTuple { formatter: self, fields: 0, empty_name: name.is_empty() }
    }

    /// `[value, ..]`
    pub fn debug_list(&mut self) -> DebugList {
        self.emit("[");
        DebugList { inner: DebugInner { formatter: self, has_entries: false }, close: "]" }
    }

    /// `{value, ..}`
    pub fn debug_set(&mut self) -> DebugList {
        self.emit("{");
        DebugList { inner: DebugInner { formatter: self, has_entries: false }, close: "}" }
    }

    /// `{key: value, ..}`
    pub fn debug_map(&mut self) -> DebugMap {
        self.emit("{");
        DebugMap { inner: DebugInner { formatter: self, has_entries: false } }
    }

    pub fn alternate(&self) -> bool {
        self.flags & FLAG_ALTERNATE != 0
    }

    pub fn precision(&self) -> Option<usize> {
        if self.flags & FLAG_HAS_PRECISION != 0 { Some(self.precision) } else { None }
    }

    fn fill_with(&mut self, count: usize, fill: char) {
        let mut written = 0;
        while written < count {
            self.emit_char(fill);
            written += 1;
        }
    }

    /// Write `text` padded to the requested width. Text is left-aligned
    /// unless the placeholder says otherwise; `default_align` is the
    /// alignment used when it does not say (numbers are right-aligned).
    fn pad_with(&mut self, text: &str, default_align: u8) -> Result {
        let length = text.chars().count();
        if self.flags & FLAG_HAS_WIDTH == 0 || length >= self.width {
            return self.write_str(text);
        }
        let padding = self.width - length;
        let align = if self.align == 0 { default_align } else { self.align };
        let before = if align == ALIGN_LEFT {
            0
        } else if align == ALIGN_CENTER {
            padding / 2
        } else {
            padding
        };
        let fill = self.fill;
        self.fill_with(before, fill);
        self.emit(text);
        self.fill_with(padding - before, fill);
        Ok(())
    }

    /// Write a string, honouring width, alignment and precision (which for
    /// text means "at most this many characters").
    pub fn pad(&mut self, text: &str) -> Result {
        match self.precision() {
            Some(limit) => {
                let mut end = 0;
                let mut taken = 0;
                let mut characters = text.chars();
                while taken < limit {
                    match characters.next() {
                        Some(character) => end += utf8_len(character),
                        None => break,
                    }
                    taken += 1;
                }
                self.pad_with(&text[..end], ALIGN_LEFT)
            }
            None => self.pad_with(text, ALIGN_LEFT),
        }
    }

    /// Write a number given as sign and digits. `{:05}` pads with zeros
    /// between the sign and the digits.
    pub fn pad_integral(&mut self, is_nonnegative: bool, prefix: &str, digits: &str) -> Result {
        let sign = if !is_nonnegative {
            "-"
        } else if self.flags & FLAG_SIGN_PLUS != 0 {
            "+"
        } else {
            ""
        };
        let length = sign.len() + prefix.len() + digits.len();
        if self.flags & FLAG_ZERO_PAD != 0 && self.flags & FLAG_HAS_WIDTH != 0 && length < self.width {
            self.emit(sign);
            self.emit(prefix);
            let zeros = self.width - length;
            self.fill_with(zeros, '0');
            return self.write_str(digits);
        }
        let mut text = String::with_capacity(length);
        text.push_str(sign);
        text.push_str(prefix);
        text.push_str(digits);
        self.pad_with(text.as_str(), ALIGN_RIGHT)
    }
}

/// What every builder does with one entry: in `{:?}`, separate it from
/// the previous one with `, `; in `{:#?}`, put it on its own line, one
/// level deeper, followed by `,`.
struct DebugInner<'a> {
    formatter: &'a mut Formatter,
    has_entries: bool,
}

impl<'a> DebugInner<'a> {
    fn entry_with(&mut self, label: &str, value: &dyn Debug) {
        if self.formatter.alternate() {
            if !self.has_entries {
                self.formatter.emit("\n");
            }
            self.formatter.indent += 1;
            self.formatter.emit(label);
            value.fmt(self.formatter);
            self.formatter.emit(",\n");
            self.formatter.indent -= 1;
        } else {
            if self.has_entries {
                self.formatter.emit(", ");
            }
            self.formatter.emit(label);
            value.fmt(self.formatter);
        }
        self.has_entries = true;
    }

    fn finish(&mut self, close: &str) -> Result {
        self.formatter.emit(close);
        Ok(())
    }
}

pub struct DebugStruct<'a> {
    formatter: &'a mut Formatter,
    has_fields: bool,
}

impl<'a> DebugStruct<'a> {
    pub fn field(&mut self, name: &str, value: &dyn Debug) -> &mut DebugStruct<'a> {
        if !self.has_fields {
            self.formatter.emit(" {");
            if !self.formatter.alternate() {
                self.formatter.emit(" ");
            }
        }
        let mut inner = DebugInner { formatter: self.formatter, has_entries: self.has_fields };
        let mut label = String::from(name);
        label.push_str(": ");
        inner.entry_with(label.as_str(), value);
        self.has_fields = true;
        self
    }

    pub fn finish(&mut self) -> Result {
        if self.has_fields {
            if self.formatter.alternate() {
                self.formatter.emit("}");
            } else {
                self.formatter.emit(" }");
            }
        }
        Ok(())
    }

    /// Finish with `..`, saying there are fields not shown.
    pub fn finish_non_exhaustive(&mut self) -> Result {
        if !self.has_fields {
            self.formatter.emit(" { .. }");
            return Ok(());
        }
        let mut inner = DebugInner { formatter: self.formatter, has_entries: true };
        inner.entry_with("", &Elision);
        self.finish()
    }
}

/// The `..` of a non-exhaustive struct.
struct Elision;

impl Debug for Elision {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.write_str("..")
    }
}

pub struct DebugTuple<'a> {
    formatter: &'a mut Formatter,
    fields: usize,
    empty_name: bool,
}

impl<'a> DebugTuple<'a> {
    pub fn field(&mut self, value: &dyn Debug) -> &mut DebugTuple<'a> {
        if self.fields == 0 {
            self.formatter.emit("(");
        }
        let mut inner = DebugInner { formatter: self.formatter, has_entries: self.fields > 0 };
        inner.entry_with("", value);
        self.fields += 1;
        self
    }

    pub fn finish(&mut self) -> Result {
        if self.fields > 0 {
            // A one-element tuple is written `(x,)`, like in the source.
            if self.fields == 1 && self.empty_name && !self.formatter.alternate() {
                self.formatter.emit(",");
            }
            self.formatter.emit(")");
        }
        Ok(())
    }
}

pub struct DebugList<'a> {
    inner: DebugInner<'a>,
    close: &'static str,
}

impl<'a> DebugList<'a> {
    pub fn entry(&mut self, value: &dyn Debug) -> &mut DebugList<'a> {
        self.inner.entry_with("", value);
        self
    }

    pub fn entries<D: Debug, I: IntoIterator<Item = D>>(&mut self, values: I) -> &mut DebugList<'a> {
        for value in values {
            self.entry(&value);
        }
        self
    }

    pub fn finish(&mut self) -> Result {
        let close = self.close;
        self.inner.finish(close)
    }
}

pub struct DebugMap<'a> {
    inner: DebugInner<'a>,
}

impl<'a> DebugMap<'a> {
    pub fn entry(&mut self, key: &dyn Debug, value: &dyn Debug) -> &mut DebugMap<'a> {
        // The key is written like a label in front of the value.
        let mut key_text = Formatter::new();
        key_text.flags = self.inner.formatter.flags;
        key.fmt(&mut key_text);
        key_text.emit(": ");
        let label = key_text.finish();
        self.inner.entry_with(label.as_str(), value);
        self
    }

    pub fn entries<K: Debug, V: Debug, I: IntoIterator<Item = (K, V)>>(&mut self, entries: I) -> &mut DebugMap<'a> {
        for (key, value) in entries {
            self.entry(&key, &value);
        }
        self
    }

    pub fn finish(&mut self) -> Result {
        self.inner.finish("}")
    }
}

fn utf8_len(character: char) -> usize {
    let code = character as u32;
    if code < 128 {
        1
    } else if code < 2048 {
        2
    } else if code < 65536 {
        3
    } else {
        4
    }
}

/// Something formatted text can be written to: a `String`, or a
/// `Formatter` when one `Display` impl writes another value.
pub trait Write {
    fn write_str(&mut self, text: &str) -> Result;

    fn write_char(&mut self, character: char) -> Result {
        let mut buffer = [0u8; 4];
        self.write_str(character.encode_utf8(&mut buffer))
    }

    /// What `write!` calls.
    fn write_fmt(&mut self, arguments: Arguments) -> Result {
        self.write_str(arguments.as_str())
    }
}

impl Write for Formatter {
    fn write_str(&mut self, text: &str) -> Result {
        self.emit(text);
        Ok(())
    }
}

/// Formatted text on its way to where `write!` sends it, or what
/// `format_args!` makes.
pub struct Arguments {
    text: String,
}

impl Arguments {
    pub fn new(text: String) -> Arguments {
        Arguments { text }
    }

    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }
}

impl Display for Arguments {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.write_str(self.text.as_str())
    }
}

impl Debug for Arguments {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.write_str(self.text.as_str())
    }
}

/// `{}` — the form meant for people.
pub trait Display {
    fn fmt(&self, f: &mut Formatter) -> Result;
}

/// `{:?}` — the form meant for programmers.
pub trait Debug {
    fn fmt(&self, f: &mut Formatter) -> Result;
}

/// `{:x}`
pub trait LowerHex {
    fn fmt(&self, f: &mut Formatter) -> Result;
}

/// `{:X}`
pub trait UpperHex {
    fn fmt(&self, f: &mut Formatter) -> Result;
}

/// `{:b}`
pub trait Binary {
    fn fmt(&self, f: &mut Formatter) -> Result;
}

/// `{:o}`
pub trait Octal {
    fn fmt(&self, f: &mut Formatter) -> Result;
}

/// `{:e}`
pub trait LowerExp {
    fn fmt(&self, f: &mut Formatter) -> Result;
}

/// `{:E}`
pub trait UpperExp {
    fn fmt(&self, f: &mut Formatter) -> Result;
}

/// Write `value` in the given radix, with `prefix` (`0x`) in alternate form.
fn fmt_unsigned(f: &mut Formatter, value: u64, is_nonnegative: bool, radix: u64, uppercase: bool, prefix: &str) -> Result {
    // 64 binary digits is the longest any u64 gets.
    let mut buffer = [0u8; 64];
    let mut position = 64;
    let mut remaining = value;
    loop {
        position -= 1;
        let digit = (remaining % radix) as u8;
        buffer[position] = if digit < 10 {
            b'0' + digit
        } else if uppercase {
            b'A' + digit - 10
        } else {
            b'a' + digit - 10
        };
        remaining = remaining / radix;
        if remaining == 0 {
            break;
        }
    }
    let digits = std::intrinsics::str_from_raw_parts(&buffer[position] as *const u8, 64 - position);
    let prefix = if f.alternate() { prefix } else { "" };
    f.pad_integral(is_nonnegative, prefix, digits)
}

/// `fmt_unsigned` for 128-bit values: those that fit in 64 bits take the
/// quicker 64-bit route.
fn fmt_unsigned_wide(f: &mut Formatter, value: u128, is_nonnegative: bool, radix: u128, uppercase: bool, prefix: &str) -> Result {
    if value <= u64::MAX as u128 {
        return fmt_unsigned(f, value as u64, is_nonnegative, radix as u64, uppercase, prefix);
    }
    let mut buffer = [0u8; 128];
    let mut position = 128;
    let mut remaining = value;
    while remaining != 0 {
        position -= 1;
        let digit = (remaining % radix) as u8;
        buffer[position] = if digit < 10 {
            b'0' + digit
        } else if uppercase {
            b'A' + digit - 10
        } else {
            b'a' + digit - 10
        };
        remaining = remaining / radix;
    }
    let digits = std::intrinsics::str_from_raw_parts(&buffer[position] as *const u8, 128 - position);
    let prefix = if f.alternate() { prefix } else { "" };
    f.pad_integral(is_nonnegative, prefix, digits)
}

fn fmt_signed(f: &mut Formatter, value: i64) -> Result {
    // Negating in u64 is right even for i64::MIN.
    let magnitude = if value < 0 { 0 - (value as u64) } else { value as u64 };
    fmt_unsigned(f, magnitude, value >= 0, 10, false, "")
}

/// The decimal digits of `value`, which is finite and not negative, and
/// the power of ten of the first one: `0.025` is `("25", -2)`. With
/// `significant`, that many digits, rounded; without, the fewest that read
/// back as exactly `value` (as an `f32` if `single`), which is what Rust
/// prints: `0.1 + 0.2` is `0.30000000000000004`.
fn decimal_digits(value: f64, single: bool, significant: Option<usize>) -> (String, i32) {
    let mut buffer = [0u8; 64];
    let pointer = &mut buffer[0] as *mut u8;
    let format = "%.*e\0".as_ptr();
    let mut decimals = match significant {
        Some(count) => if count > 0 { count - 1 } else { 0 },
        None => 0,
    };
    loop {
        let length = unsafe { std::libc::snprintf(pointer, 64, format, decimals as i32, value) } as usize;
        let reads_back = if single {
            let read = unsafe { std::libc::strtof(pointer, std::ptr::null_mut()) };
            read == value as f32
        } else {
            let read = unsafe { std::libc::strtod(pointer, std::ptr::null_mut()) };
            read == value
        };
        if significant.is_some() || reads_back || decimals >= 17 {
            // The text is `d.ddde±XX`.
            let text = std::intrinsics::str_from_raw_parts(pointer, length);
            let (mantissa, exponent) = text.split_once("e").expect("`%e` writes an exponent");
            let mut digits = String::new();
            for byte in mantissa.bytes() {
                if byte != b'.' {
                    digits.push(byte as char);
                }
            }
            let exponent: i32 = exponent.parse().expect("`%e` writes a number after `e`");
            return (digits, exponent);
        }
        decimals += 1;
    }
}

/// `digits` × 10^`exponent` written out: `("25", -2)` is `0.025`. At least
/// `min_fraction` digits follow the point (`1.0` rather than `1`).
fn plain_decimal(digits: &str, exponent: i32, min_fraction: usize) -> String {
    let mut text = String::new();
    let length = digits.len() as i32;
    let mut fraction_digits = 0;
    if exponent < 0 {
        text.push_str("0.");
        let mut zeros = -exponent - 1;
        while zeros > 0 {
            text.push('0');
            zeros -= 1;
            fraction_digits += 1;
        }
        text.push_str(digits);
        fraction_digits += digits.len();
    } else {
        let whole = exponent + 1;
        if length <= whole {
            text.push_str(digits);
            let mut zeros = whole - length;
            while zeros > 0 {
                text.push('0');
                zeros -= 1;
            }
        } else {
            text.push_str(&digits[..whole as usize]);
            text.push('.');
            text.push_str(&digits[whole as usize..]);
            fraction_digits = (length - whole) as usize;
        }
    }
    if fraction_digits < min_fraction {
        if fraction_digits == 0 {
            text.push('.');
        }
        while fraction_digits < min_fraction {
            text.push('0');
            fraction_digits += 1;
        }
    }
    text
}

/// `digits` × 10^`exponent` in scientific form: `("25", -2)` is `2.5e-2`.
fn scientific(digits: &str, exponent: i32, upper: bool) -> String {
    let mut text = String::from(&digits[..1]);
    if digits.len() > 1 {
        text.push('.');
        text.push_str(&digits[1..]);
    }
    text.push(if upper { 'E' } else { 'e' });
    text.push_str(&exponent.to_string());
    text
}

/// Which form a float is written in.
#[derive(Clone, Copy, PartialEq)]
enum FloatForm {
    Display,
    Debug,
    LowerExp,
    UpperExp,
}

/// Write a float as Rust does in the given form. `{}` is plain decimal;
/// `{:?}` is too, with a fractional part, except that very large and very
/// small numbers switch to scientific form; `{:e}` is scientific.
fn fmt_float(f: &mut Formatter, value: f64, single: bool, form: FloatForm) -> Result {
    if value != value {
        return f.pad_with("NaN", ALIGN_RIGHT);
    }
    let is_nonnegative = !value.is_sign_negative();
    let magnitude = value.abs();
    if magnitude == f64::INFINITY {
        return f.pad_integral(is_nonnegative, "", "inf");
    }
    let text = match (form, f.precision()) {
        (FloatForm::Display | FloatForm::Debug, Some(precision)) => {
            let mut buffer = [0u8; 400];
            let pointer = &mut buffer[0] as *mut u8;
            let length = unsafe { std::libc::snprintf(pointer, 400, "%.*f\0".as_ptr(), precision as i32, magnitude) };
            String::from(std::intrinsics::str_from_raw_parts(pointer, length as usize))
        }
        (FloatForm::LowerExp | FloatForm::UpperExp, precision) => {
            let (digits, exponent) = decimal_digits(magnitude, single, precision.map(|p| p + 1));
            scientific(digits.as_str(), exponent, form == FloatForm::UpperExp)
        }
        (_, None) => {
            let (digits, exponent) = decimal_digits(magnitude, single, None);
            let tiny_or_huge = magnitude != 0.0 && (magnitude < 1e-4 || magnitude >= 1e16);
            if form == FloatForm::Debug && tiny_or_huge {
                scientific(digits.as_str(), exponent, false)
            } else {
                plain_decimal(digits.as_str(), exponent, if form == FloatForm::Debug { 1 } else { 0 })
            }
        }
    };
    f.pad_integral(is_nonnegative, "", text.as_str())
}

/// `{:e}` of an integer: its digits in scientific form, rounded half to
/// even if a precision asks for fewer of them.
fn fmt_integer_exp(f: &mut Formatter, value: i64, magnitude: u64, upper: bool) -> Result {
    let all = magnitude.to_string();
    let mut digits = String::from(all.as_str());
    let mut exponent = all.len() as i32 - 1;
    match f.precision() {
        Some(precision) if precision + 1 < all.len() => {
            let keep = precision + 1;
            let bytes = all.as_bytes();
            let next = bytes[keep];
            let rest_nonzero = bytes[keep + 1..].iter().any(|&b| b != b'0');
            let odd = (bytes[keep - 1] - b'0') % 2 == 1;
            let round_up = next > b'5' || (next == b'5' && (rest_nonzero || odd));
            let mut kept: Vec<u8> = bytes[..keep].to_vec();
            if round_up {
                let mut position = keep;
                loop {
                    if position == 0 {
                        kept.insert(0, b'1');
                        kept.pop();
                        exponent += 1;
                        break;
                    }
                    position -= 1;
                    if kept[position] == b'9' {
                        kept[position] = b'0';
                    } else {
                        kept[position] += 1;
                        break;
                    }
                }
            }
            digits = String::from_utf8(kept).expect("digits are ASCII");
        }
        Some(precision) => {
            while digits.len() < precision + 1 {
                digits.push('0');
            }
        }
        None => {
            // The fewest digits: trailing zeros go into the exponent.
            while digits.len() > 1 && digits.ends_with("0") {
                digits.pop();
            }
        }
    }
    let text = scientific(digits.as_str(), exponent, upper);
    f.pad_integral(value >= 0, "", text.as_str())
}

impl Display for i8 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_signed(f, *self as i64) } }
impl Debug for i8 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_signed(f, *self as i64) } }
impl LowerHex for i8 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u8) as u64, true, 16, false, "0x") } }
impl UpperHex for i8 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u8) as u64, true, 16, true, "0x") } }
impl Binary for i8 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u8) as u64, true, 2, false, "0b") } }
impl Octal for i8 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u8) as u64, true, 8, false, "0o") } }

impl Display for i16 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_signed(f, *self as i64) } }
impl Debug for i16 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_signed(f, *self as i64) } }
impl LowerHex for i16 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u16) as u64, true, 16, false, "0x") } }
impl UpperHex for i16 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u16) as u64, true, 16, true, "0x") } }
impl Binary for i16 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u16) as u64, true, 2, false, "0b") } }
impl Octal for i16 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u16) as u64, true, 8, false, "0o") } }

impl Display for i32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_signed(f, *self as i64) } }
impl Debug for i32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_signed(f, *self as i64) } }
impl LowerHex for i32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u32) as u64, true, 16, false, "0x") } }
impl UpperHex for i32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u32) as u64, true, 16, true, "0x") } }
impl Binary for i32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u32) as u64, true, 2, false, "0b") } }
impl Octal for i32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u32) as u64, true, 8, false, "0o") } }

impl Display for i64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_signed(f, *self as i64) } }
impl Debug for i64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_signed(f, *self as i64) } }
impl LowerHex for i64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u64) as u64, true, 16, false, "0x") } }
impl UpperHex for i64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u64) as u64, true, 16, true, "0x") } }
impl Binary for i64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u64) as u64, true, 2, false, "0b") } }
impl Octal for i64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, (*self as u64) as u64, true, 8, false, "0o") } }

impl Display for isize { fn fmt(&self, f: &mut Formatter) -> Result { fmt_signed(f, *self as i64) } }
impl Debug for isize { fn fmt(&self, f: &mut Formatter) -> Result { fmt_signed(f, *self as i64) } }
impl LowerHex for isize { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 16, false, "0x") } }
impl UpperHex for isize { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 16, true, "0x") } }
impl Binary for isize { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 2, false, "0b") } }
impl Octal for isize { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 8, false, "0o") } }

impl Display for u8 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 10, false, "") } }
impl Debug for u8 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 10, false, "") } }
impl LowerHex for u8 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 16, false, "0x") } }
impl UpperHex for u8 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 16, true, "0x") } }
impl Binary for u8 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 2, false, "0b") } }
impl Octal for u8 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 8, false, "0o") } }

impl Display for u16 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 10, false, "") } }
impl Debug for u16 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 10, false, "") } }
impl LowerHex for u16 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 16, false, "0x") } }
impl UpperHex for u16 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 16, true, "0x") } }
impl Binary for u16 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 2, false, "0b") } }
impl Octal for u16 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 8, false, "0o") } }

impl Display for u32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 10, false, "") } }
impl Debug for u32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 10, false, "") } }
impl LowerHex for u32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 16, false, "0x") } }
impl UpperHex for u32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 16, true, "0x") } }
impl Binary for u32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 2, false, "0b") } }
impl Octal for u32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 8, false, "0o") } }

impl Display for u64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 10, false, "") } }
impl Debug for u64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 10, false, "") } }
impl LowerHex for u64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 16, false, "0x") } }
impl UpperHex for u64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 16, true, "0x") } }
impl Binary for u64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 2, false, "0b") } }
impl Octal for u64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 8, false, "0o") } }

impl Display for usize { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 10, false, "") } }
impl Debug for usize { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 10, false, "") } }
impl LowerHex for usize { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 16, false, "0x") } }
impl UpperHex for usize { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 16, true, "0x") } }
impl Binary for usize { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 2, false, "0b") } }
impl Octal for usize { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned(f, *self as u64, true, 8, false, "0o") } }

impl Display for f64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_float(f, *self, false, FloatForm::Display) } }
impl Debug for f64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_float(f, *self, false, FloatForm::Debug) } }
impl LowerExp for f64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_float(f, *self, false, FloatForm::LowerExp) } }
impl UpperExp for f64 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_float(f, *self, false, FloatForm::UpperExp) } }
impl Display for f32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_float(f, *self as f64, true, FloatForm::Display) } }
impl Debug for f32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_float(f, *self as f64, true, FloatForm::Debug) } }
impl LowerExp for f32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_float(f, *self as f64, true, FloatForm::LowerExp) } }
impl UpperExp for f32 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_float(f, *self as f64, true, FloatForm::UpperExp) } }

macro_rules! integer_exp {
    ($($t:ident)*) => {
        $(
            impl LowerExp for $t {
                fn fmt(&self, f: &mut Formatter) -> Result {
                    let value = *self as i64;
                    let magnitude = if *self < (0 as $t) { 0 - (value as u64) } else { *self as u64 };
                    fmt_integer_exp(f, value, magnitude, false)
                }
            }
            impl UpperExp for $t {
                fn fmt(&self, f: &mut Formatter) -> Result {
                    let value = *self as i64;
                    let magnitude = if *self < (0 as $t) { 0 - (value as u64) } else { *self as u64 };
                    fmt_integer_exp(f, value, magnitude, true)
                }
            }
        )*
    };
}

integer_exp!(i8 i16 i32 i64 isize u8 u16 u32 u64 usize);

impl Display for u128 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned_wide(f, *self, true, 10, false, "") } }
impl Debug for u128 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned_wide(f, *self, true, 10, false, "") } }
impl LowerHex for u128 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned_wide(f, *self, true, 16, false, "0x") } }
impl UpperHex for u128 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned_wide(f, *self, true, 16, true, "0x") } }
impl Binary for u128 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned_wide(f, *self, true, 2, false, "0b") } }
impl Octal for u128 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned_wide(f, *self, true, 8, false, "0o") } }

/// An `i128`'s magnitude: negating in `u128` is right even for `i128::MIN`.
fn magnitude_wide(value: i128) -> u128 {
    if value < 0 { 0u128.wrapping_sub(value as u128) } else { value as u128 }
}

impl Display for i128 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned_wide(f, magnitude_wide(*self), *self >= 0, 10, false, "") } }
impl Debug for i128 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned_wide(f, magnitude_wide(*self), *self >= 0, 10, false, "") } }
impl LowerHex for i128 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned_wide(f, *self as u128, true, 16, false, "0x") } }
impl UpperHex for i128 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned_wide(f, *self as u128, true, 16, true, "0x") } }
impl Binary for i128 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned_wide(f, *self as u128, true, 2, false, "0b") } }
impl Octal for i128 { fn fmt(&self, f: &mut Formatter) -> Result { fmt_unsigned_wide(f, *self as u128, true, 8, false, "0o") } }

impl Display for bool {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.pad(if *self { "true" } else { "false" })
    }
}

impl Debug for bool {
    fn fmt(&self, f: &mut Formatter) -> Result {
        Display::fmt(self, f)
    }
}

impl Display for char {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut text = String::new();
        text.push(*self);
        f.pad(text.as_str())
    }
}

impl Debug for char {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.write_char('\'');
        write_escaped(f, *self, '\'');
        f.write_char('\'')
    }
}

/// Write a character the way it would appear inside a literal, as rustc's
/// `{:?}` does: characters that are not printable, and those that would
/// combine with what comes before them, as `\u{..}` escapes.
fn write_escaped(f: &mut Formatter, character: char, quote: char) {
    let escapes = std::char::DebugEscapes { grapheme_extended: true, single_quote: quote == '\'', double_quote: quote == '"' };
    for c in character.escape_debug_ext(escapes) {
        f.write_char(c);
    }
}

impl Display for str {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.pad(self)
    }
}

impl Debug for str {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.write_char('"');
        for character in self.chars() {
            write_escaped(f, character, '"');
        }
        f.write_char('"')
    }
}

impl Display for String {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.pad(self.as_str())
    }
}

impl Debug for String {
    fn fmt(&self, f: &mut Formatter) -> Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl Debug for () {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.write_str("()")
    }
}

/// A reference is formatted as what it points to, in every style.
macro_rules! through_references {
    ($($style:ident)*) => {
        $(
            impl<T: $style + ?Sized> $style for &T {
                fn fmt(&self, f: &mut Formatter) -> Result {
                    $style::fmt(&**self, f)
                }
            }

            impl<T: $style + ?Sized> $style for &mut T {
                fn fmt(&self, f: &mut Formatter) -> Result {
                    $style::fmt(&**self, f)
                }
            }
        )*
    };
}

through_references!(Display Debug LowerHex UpperHex Binary Octal LowerExp UpperExp);

/// Defines `Debug` for tuples whose elements all have it: `(1, "a")`.
macro_rules! tuple_debug {
    ($(($($name:ident $index:tt),+))*) => {
        $(
            impl<$($name: Debug),+> Debug for ($($name,)+) {
                fn fmt(&self, f: &mut Formatter) -> Result {
                    let mut tuple = f.debug_tuple("");
                    $(tuple.field(&self.$index);)+
                    tuple.finish()
                }
            }
        )*
    };
}

tuple_debug! {
    (A 0)
    (A 0, B 1)
    (A 0, B 1, C 2)
    (A 0, B 1, C 2, D 3)
    (A 0, B 1, C 2, D 3, E 4)
    (A 0, B 1, C 2, D 3, E 4, F 5)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11)
}
