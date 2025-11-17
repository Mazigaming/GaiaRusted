//! Format string support for println!, format!, and other formatting macros
//!
//! Handles Rust-style format strings with:
//! - {} - Display formatting
//! - {:?} - Debug formatting
//! - {:#?} - Pretty debug
//! - {:x} - Hex formatting
//! - {:b} - Binary formatting
//! - {:o} - Octal formatting

/// Represents a single format specification
#[derive(Debug, Clone, PartialEq)]
pub struct FormatSpec {
    pub width: Option<usize>,
    pub precision: Option<usize>,
    pub fill_char: char,
    pub alignment: Alignment,
    pub format_type: FormatType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatType {
    Display,    // {}
    Debug,      // {:?}
    Hex,        // {:x}
    UpperHex,   // {:X}
    Binary,     // {:b}
    Octal,      // {:o}
    Pointer,    // {:p}
}

impl Default for FormatSpec {
    fn default() -> Self {
        FormatSpec {
            width: None,
            precision: None,
            fill_char: ' ',
            alignment: Alignment::Left,
            format_type: FormatType::Display,
        }
    }
}

/// Parse format string and return format specs with text segments
pub fn parse_format_string(format: &str) -> Result<Vec<FormatPart>, String> {
    let mut parts = Vec::new();
    let mut chars = format.chars().peekable();
    let mut current_text = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                if chars.peek() == Some(&'{') {
                    current_text.push('{');
                    chars.next();
                } else {
                    if !current_text.is_empty() {
                        parts.push(FormatPart::Text(current_text.clone()));
                        current_text.clear();
                    }

                    let spec = parse_format_spec(&mut chars)?;
                    parts.push(FormatPart::Placeholder(spec));
                }
            }
            '}' => {
                if chars.peek() == Some(&'}') {
                    current_text.push('}');
                    chars.next();
                } else {
                    return Err("Unmatched closing brace".to_string());
                }
            }
            _ => current_text.push(ch),
        }
    }

    if !current_text.is_empty() {
        parts.push(FormatPart::Text(current_text));
    }

    Ok(parts)
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormatPart {
    Text(String),
    Placeholder(FormatSpec),
}

fn parse_format_spec(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<FormatSpec, String> {
    let mut spec = FormatSpec::default();

    // Check for closing brace immediately
    if chars.peek() == Some(&'}') {
        chars.next();
        return Ok(spec);
    }

    // Parse alignment and fill character
    let mut temp_str = String::new();
    while let Some(&ch) = chars.peek() {
        if ch == '<' || ch == '>' || ch == '^' {
            if !temp_str.is_empty() {
                spec.fill_char = temp_str.chars().next().unwrap_or(' ');
            }
            chars.next();
            spec.alignment = match ch {
                '<' => Alignment::Left,
                '>' => Alignment::Right,
                '^' => Alignment::Center,
                _ => Alignment::Left,
            };
            break;
        } else if ch == ':' {
            break;
        } else {
            temp_str.push(ch);
            chars.next();
        }
    }

    // Parse width
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() {
            let mut width_str = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_ascii_digit() {
                    width_str.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }
            spec.width = width_str.parse::<usize>().ok();
            break;
        } else if ch == '.' {
            break;
        } else if ch == ':' {
            chars.next();
            break;
        } else {
            chars.next();
        }
    }

    // Parse precision
    if chars.peek() == Some(&'.') {
        chars.next();
        let mut precision_str = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_ascii_digit() {
                precision_str.push(ch);
                chars.next();
            } else {
                break;
            }
        }
        spec.precision = precision_str.parse::<usize>().ok();
    }

    // Parse format type
    if chars.peek().is_some() && chars.peek() != Some(&'}') {
        match chars.next() {
            Some('?') => {
                spec.format_type = FormatType::Debug;
                if chars.peek() == Some(&'#') {
                    chars.next();
                }
            }
            Some('x') => spec.format_type = FormatType::Hex,
            Some('X') => spec.format_type = FormatType::UpperHex,
            Some('b') => spec.format_type = FormatType::Binary,
            Some('o') => spec.format_type = FormatType::Octal,
            Some('p') => spec.format_type = FormatType::Pointer,
            Some(_) => {}
            None => {}
        }
    }

    // Consume closing brace
    if chars.peek() == Some(&'}') {
        chars.next();
    } else {
        return Err("Expected closing brace in format spec".to_string());
    }

    Ok(spec)
}

/// Format an integer according to the spec
pub fn format_integer(value: i64, spec: &FormatSpec) -> String {
    let formatted = match spec.format_type {
        FormatType::Display => value.to_string(),
        FormatType::Debug => value.to_string(),
        FormatType::Hex => format!("{:x}", value),
        FormatType::UpperHex => format!("{:X}", value),
        FormatType::Binary => format!("{:b}", value),
        FormatType::Octal => format!("{:o}", value),
        FormatType::Pointer => format!("{:p}", &value as *const i64),
    };

    apply_formatting(&formatted, spec)
}

/// Format a float according to the spec
pub fn format_float(value: f64, spec: &FormatSpec) -> String {
    let formatted = if let Some(prec) = spec.precision {
        format!("{:.prec$}", value, prec = prec)
    } else {
        value.to_string()
    };

    apply_formatting(&formatted, spec)
}

/// Format a string according to the spec
pub fn format_string(value: &str, spec: &FormatSpec) -> String {
    let s = if let Some(prec) = spec.precision {
        &value[..value.len().min(prec)]
    } else {
        value
    };
    apply_formatting(s, spec)
}

/// Apply width and alignment formatting
fn apply_formatting(s: &str, spec: &FormatSpec) -> String {
    if let Some(width) = spec.width {
        if s.len() >= width {
            return s.to_string();
        }

        let padding = width - s.len();
        let left_pad = match spec.alignment {
            Alignment::Left => 0,
            Alignment::Right => padding,
            Alignment::Center => padding / 2,
        };
        let right_pad = padding - left_pad;

        let mut result = String::new();
        for _ in 0..left_pad {
            result.push(spec.fill_char);
        }
        result.push_str(s);
        for _ in 0..right_pad {
            result.push(spec.fill_char);
        }
        result
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_format_string() {
        let parts = parse_format_string("Hello, {}!").unwrap();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_format_integer() {
        let spec = FormatSpec::default();
        let result = format_integer(42, &spec);
        assert_eq!(result, "42");
    }

    #[test]
    fn test_format_hex() {
        let mut spec = FormatSpec::default();
        spec.format_type = FormatType::Hex;
        let result = format_integer(255, &spec);
        assert_eq!(result, "ff");
    }
}
