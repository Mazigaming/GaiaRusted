//! ANSI color support for terminal output
//!
//! Simple ANSI color codes for better CLI output formatting.
//! Provides colored output for errors (red), warnings (yellow), and success (green).

use std::fmt;

/// ANSI color codes
#[derive(Debug, Clone, Copy)]
pub struct Color {
    code: &'static str,
}

impl Color {
    /// Red color for errors
    pub const RED: Self = Color { code: "\x1b[31m" };
    
    /// Yellow color for warnings
    pub const YELLOW: Self = Color { code: "\x1b[33m" };
    
    /// Green color for success
    pub const GREEN: Self = Color { code: "\x1b[32m" };
    
    /// Cyan color for info/debug
    pub const CYAN: Self = Color { code: "\x1b[36m" };
    
    /// White/default color
    pub const WHITE: Self = Color { code: "\x1b[37m" };
    
    /// Bold formatting
    pub const BOLD: Self = Color { code: "\x1b[1m" };
    
    /// Reset to default
    pub const RESET: Self = Color { code: "\x1b[0m" };
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.code)
    }
}

/// Colored text wrapper
pub struct Colored {
    text: String,
    color: Color,
}

impl Colored {
    /// Create a new colored text
    pub fn new(text: impl Into<String>, color: Color) -> Self {
        Colored {
            text: text.into(),
            color,
        }
    }
    
    /// Red text
    pub fn red(text: impl Into<String>) -> Self {
        Colored::new(text, Color::RED)
    }
    
    /// Yellow text
    pub fn yellow(text: impl Into<String>) -> Self {
        Colored::new(text, Color::YELLOW)
    }
    
    /// Green text
    pub fn green(text: impl Into<String>) -> Self {
        Colored::new(text, Color::GREEN)
    }
    
    /// Cyan text
    pub fn cyan(text: impl Into<String>) -> Self {
        Colored::new(text, Color::CYAN)
    }
    
    /// Bold text
    pub fn bold(text: impl Into<String>) -> Self {
        Colored::new(text, Color::BOLD)
    }
}

impl fmt::Display for Colored {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}", self.color, self.text, Color::RESET)
    }
}

/// Format error severity with color
pub fn format_error(message: &str) -> String {
    format!("{}", Colored::red(message))
}

/// Format warning with color
pub fn format_warning(message: &str) -> String {
    format!("{}", Colored::yellow(message))
}

/// Format success with color
pub fn format_success(message: &str) -> String {
    format!("{}", Colored::green(message))
}

/// Format info/debug with color
pub fn format_info(message: &str) -> String {
    format!("{}", Colored::cyan(message))
}

/// Format header/bold with color
pub fn format_header(message: &str) -> String {
    format!("{}", Colored::bold(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colored_creation() {
        let colored = Colored::red("error");
        let output = colored.to_string();
        assert!(output.contains("error"));
    }

    #[test]
    fn test_color_helpers() {
        let red = format_error("Error message");
        let yellow = format_warning("Warning message");
        let green = format_success("Success!");
        
        assert!(red.contains("Error message"));
        assert!(yellow.contains("Warning message"));
        assert!(green.contains("Success!"));
    }
}