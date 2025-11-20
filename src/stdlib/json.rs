
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn to_string(&self) -> String {
        match self {
            JsonValue::Null => "null".to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{:.0}", n)
                } else {
                    n.to_string()
                }
            }
            JsonValue::String(s) => format!("\"{}\"", escape_string(s)),
            JsonValue::Array(arr) => {
                let items = arr
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("[{}]", items)
            }
            JsonValue::Object(obj) => {
                let items = obj
                    .iter()
                    .map(|(k, v)| format!("\"{}\":{}", k, v.to_string()))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{{{}}}", items)
            }
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            JsonValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<JsonValue> {
        match self {
            JsonValue::Object(obj) => obj.get(key).cloned(),
            _ => None,
        }
    }

    pub fn get_array_index(&self, index: usize) -> Option<JsonValue> {
        match self {
            JsonValue::Array(arr) => arr.get(index).cloned(),
            _ => None,
        }
    }
}

pub struct JsonParser {
    input: String,
    pos: usize,
}

impl JsonParser {
    pub fn new(input: &str) -> Self {
        JsonParser {
            input: input.to_string(),
            pos: 0,
        }
    }

    pub fn parse(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        self.parse_value()
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();

        if self.pos >= self.input.len() {
            return Err("Unexpected end of input".to_string());
        }

        match self.current_char() {
            'n' => self.parse_null(),
            't' | 'f' => self.parse_bool(),
            '"' => self.parse_string(),
            '[' => self.parse_array(),
            '{' => self.parse_object(),
            '-' | '0'..='9' => self.parse_number(),
            c => Err(format!("Unexpected character: {}", c)),
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        if self.input[self.pos..].starts_with("null") {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err("Expected 'null'".to_string())
        }
    }

    fn parse_bool(&mut self) -> Result<JsonValue, String> {
        if self.input[self.pos..].starts_with("true") {
            self.pos += 4;
            Ok(JsonValue::Bool(true))
        } else if self.input[self.pos..].starts_with("false") {
            self.pos += 5;
            Ok(JsonValue::Bool(false))
        } else {
            Err("Expected 'true' or 'false'".to_string())
        }
    }

    fn parse_string(&mut self) -> Result<JsonValue, String> {
        if self.current_char() != '"' {
            return Err("Expected '\"'".to_string());
        }

        self.pos += 1;
        let mut result = String::new();

        while self.pos < self.input.len() {
            match self.current_char() {
                '"' => {
                    self.pos += 1;
                    return Ok(JsonValue::String(result));
                }
                '\\' => {
                    self.pos += 1;
                    if self.pos >= self.input.len() {
                        return Err("Unexpected end in string".to_string());
                    }
                    match self.current_char() {
                        '"' => result.push('"'),
                        '\\' => result.push('\\'),
                        '/' => result.push('/'),
                        'b' => result.push('\u{0008}'),
                        'f' => result.push('\u{000C}'),
                        'n' => result.push('\n'),
                        'r' => result.push('\r'),
                        't' => result.push('\t'),
                        _ => return Err("Invalid escape sequence".to_string()),
                    }
                    self.pos += 1;
                }
                c => {
                    result.push(c);
                    self.pos += 1;
                }
            }
        }

        Err("Unterminated string".to_string())
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.pos;

        if self.current_char() == '-' {
            self.pos += 1;
        }

        while self.pos < self.input.len() && self.current_char().is_ascii_digit() {
            self.pos += 1;
        }

        if self.pos < self.input.len() && self.current_char() == '.' {
            self.pos += 1;
            while self.pos < self.input.len() && self.current_char().is_ascii_digit() {
                self.pos += 1;
            }
        }

        if self.pos < self.input.len() && (self.current_char() == 'e' || self.current_char() == 'E') {
            self.pos += 1;
            if self.pos < self.input.len() && (self.current_char() == '+' || self.current_char() == '-') {
                self.pos += 1;
            }
            while self.pos < self.input.len() && self.current_char().is_ascii_digit() {
                self.pos += 1;
            }
        }

        let num_str = &self.input[start..self.pos];
        num_str
            .parse::<f64>()
            .map(JsonValue::Number)
            .map_err(|_| "Invalid number".to_string())
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        if self.current_char() != '[' {
            return Err("Expected '['".to_string());
        }

        self.pos += 1;
        let mut arr = Vec::new();

        self.skip_whitespace();
        if self.pos < self.input.len() && self.current_char() == ']' {
            self.pos += 1;
            return Ok(JsonValue::Array(arr));
        }

        loop {
            arr.push(self.parse_value()?);
            self.skip_whitespace();

            if self.pos >= self.input.len() {
                return Err("Unexpected end in array".to_string());
            }

            match self.current_char() {
                ',' => {
                    self.pos += 1;
                    self.skip_whitespace();
                }
                ']' => {
                    self.pos += 1;
                    return Ok(JsonValue::Array(arr));
                }
                _ => return Err("Expected ',' or ']'".to_string()),
            }
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        if self.current_char() != '{' {
            return Err("Expected '{'".to_string());
        }

        self.pos += 1;
        let mut obj = HashMap::new();

        self.skip_whitespace();
        if self.pos < self.input.len() && self.current_char() == '}' {
            self.pos += 1;
            return Ok(JsonValue::Object(obj));
        }

        loop {
            self.skip_whitespace();
            let key = match self.parse_string()? {
                JsonValue::String(s) => s,
                _ => return Err("Expected string key".to_string()),
            };

            self.skip_whitespace();
            if self.pos >= self.input.len() || self.current_char() != ':' {
                return Err("Expected ':'".to_string());
            }

            self.pos += 1;
            let value = self.parse_value()?;
            obj.insert(key, value);

            self.skip_whitespace();
            if self.pos >= self.input.len() {
                return Err("Unexpected end in object".to_string());
            }

            match self.current_char() {
                ',' => {
                    self.pos += 1;
                }
                '}' => {
                    self.pos += 1;
                    return Ok(JsonValue::Object(obj));
                }
                _ => return Err("Expected ',' or '}'".to_string()),
            }
        }
    }

    fn current_char(&self) -> char {
        self.input.chars().nth(self.pos).unwrap_or('\0')
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.current_char().is_whitespace() {
            self.pos += 1;
        }
    }
}

fn escape_string(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_null() {
        let val = JsonValue::Null;
        assert!(val.is_null());
    }

    #[test]
    fn test_json_bool() {
        let val = JsonValue::Bool(true);
        assert_eq!(val.as_bool(), Some(true));
    }

    #[test]
    fn test_json_number() {
        let val = JsonValue::Number(42.5);
        assert_eq!(val.as_number(), Some(42.5));
    }

    #[test]
    fn test_json_string() {
        let val = JsonValue::String("hello".to_string());
        assert_eq!(val.as_string(), Some("hello".to_string()));
    }

    #[test]
    fn test_parse_null() {
        let mut parser = JsonParser::new("null");
        assert_eq!(parser.parse().unwrap(), JsonValue::Null);
    }

    #[test]
    fn test_parse_bool() {
        let mut parser = JsonParser::new("true");
        assert_eq!(parser.parse().unwrap(), JsonValue::Bool(true));
    }

    #[test]
    fn test_parse_number() {
        let mut parser = JsonParser::new("42");
        assert_eq!(parser.parse().unwrap(), JsonValue::Number(42.0));
    }

    #[test]
    fn test_parse_string() {
        let mut parser = JsonParser::new("\"hello\"");
        assert_eq!(parser.parse().unwrap(), JsonValue::String("hello".to_string()));
    }

    #[test]
    fn test_parse_array() {
        let mut parser = JsonParser::new("[1, 2, 3]");
        let result = parser.parse().unwrap();
        assert!(matches!(result, JsonValue::Array(_)));
    }

    #[test]
    fn test_parse_object() {
        let mut parser = JsonParser::new("{\"key\": \"value\"}");
        let result = parser.parse().unwrap();
        assert!(matches!(result, JsonValue::Object(_)));
    }
}
