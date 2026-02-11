//! Output Formatter for v1.0.2
//! 
//! Replaces verbose, robotic output with human-feeling, modern, and unique compiler messages.
//! Focus: Simple, clean, informative.

use std::fmt;
use std::time::Duration;

/// Color codes for terminal output
pub struct Colors;

impl Colors {
    pub const RED: &'static str = "\x1b[31m";
    pub const YELLOW: &'static str = "\x1b[33m";
    pub const GREEN: &'static str = "\x1b[32m";
    pub const BLUE: &'static str = "\x1b[34m";
    pub const CYAN: &'static str = "\x1b[36m";
    pub const BOLD: &'static str = "\x1b[1m";
    pub const DIM: &'static str = "\x1b[2m";
    pub const RESET: &'static str = "\x1b[0m";
}

/// Compilation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Success,
    Warning,
    Error,
    Info,
}

impl Status {
    pub fn symbol(&self) -> &'static str {
        match self {
            Status::Success => "✓",
            Status::Warning => "⚠",
            Status::Error => "✗",
            Status::Info => "•",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Status::Success => Colors::GREEN,
            Status::Warning => Colors::YELLOW,
            Status::Error => Colors::RED,
            Status::Info => Colors::CYAN,
        }
    }
}

/// Phase during compilation
#[derive(Debug, Clone)]
pub struct Phase {
    pub number: usize,
    pub name: &'static str,
    pub description: &'static str,
}

impl Phase {
    pub fn new(number: usize, name: &'static str, description: &'static str) -> Self {
        Phase { number, name, description }
    }

    pub const LEXING: Phase = Phase {
        number: 1,
        name: "Lexing",
        description: "tokenizing source",
    };

    pub const PARSING: Phase = Phase {
        number: 2,
        name: "Parsing",
        description: "building syntax tree",
    };

    pub const LOWERING: Phase = Phase {
        number: 3,
        name: "Lowering",
        description: "removing syntactic sugar",
    };

    pub const TYPECHECKING: Phase = Phase {
        number: 4,
        name: "Type Checking",
        description: "verifying types",
    };

    pub const BORROWCHECKING: Phase = Phase {
        number: 5,
        name: "Borrow Checking",
        description: "checking memory safety",
    };

    pub const MIR_LOWERING: Phase = Phase {
        number: 6,
        name: "MIR Lowering",
        description: "building control flow",
    };

    pub const OPTIMIZATION: Phase = Phase {
        number: 7,
        name: "Optimization",
        description: "optimizing code",
    };

    pub const CODEGEN: Phase = Phase {
        number: 8,
        name: "Code Generation",
        description: "generating assembly",
    };

    pub fn format(&self, status: Status) -> String {
        format!(
            "{}[Phase {}]{} {} ({})",
            status.color(),
            self.number,
            Colors::RESET,
            self.name,
            Colors::DIM,
        ) + self.description + Colors::RESET
        }
}

/// Compiler message with severity
#[derive(Debug, Clone)]
pub struct Message {
    pub severity: Status,
    pub phase: Option<String>,
    pub title: String,
    pub details: Option<String>,
    pub suggestion: Option<String>,
}

impl Message {
    pub fn new(severity: Status, title: impl Into<String>) -> Self {
        Message {
            severity,
            phase: None,
            title: title.into(),
            details: None,
            suggestion: None,
        }
    }

    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = Some(phase.into());
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn print(&self) {
        let prefix = format!(
            "{}{}{}",
            self.severity.color(),
            self.severity.symbol(),
            Colors::RESET
        );

        println!("{} {}", prefix, Colors::BOLD.to_string() + &self.title + Colors::RESET);

        if let Some(details) = &self.details {
            println!("  {}", details);
        }

        if let Some(phase) = &self.phase {
            println!("  {}{} in {}{}", Colors::DIM, "→", phase, Colors::RESET);
        }

        if let Some(suggestion) = &self.suggestion {
            println!(
                "  {}hint:{} {}",
                Colors::CYAN,
                Colors::RESET,
                suggestion
            );
        }
    }
}

/// Compilation statistics formatter
pub struct Stats {
    pub files: usize,
    pub lines: usize,
    pub assembly_size: usize,
    pub total_time: Duration,
    pub phases: Vec<(String, Duration)>,
}

impl Stats {
    pub fn print(&self) {
        println!();
        println!(
            "{}─────────────────────────────────────────{}",
            Colors::DIM,
            Colors::RESET
        );
        println!(
            "{}Compilation Summary{}",
            Colors::BOLD,
            Colors::RESET
        );
        println!(
            "{}─────────────────────────────────────────{}",
            Colors::DIM,
            Colors::RESET
        );

        println!(
            "  {} {}files{}",
            Colors::CYAN,
            self.files,
            Colors::RESET
        );
        println!(
            "  {} {}lines of code{}",
            Colors::CYAN,
            self.lines,
            Colors::RESET
        );
        println!(
            "  {} {}bytes of assembly{}",
            Colors::CYAN,
            self.assembly_size,
            Colors::RESET
        );
        println!(
            "  {} {}{}ms total{}",
            Colors::GREEN,
            self.total_time.as_millis(),
            Colors::RESET,
            Colors::RESET
        );

        println!();
        println!("{}Phase breakdown:{}", Colors::DIM, Colors::RESET);
        for (phase, duration) in &self.phases {
            let percent = (duration.as_millis() as f64 / self.total_time.as_millis() as f64) * 100.0;
            println!(
                "  {} {}ms {:.1}% {}{}",
                Colors::CYAN,
                duration.as_millis(),
                percent,
                phase,
                Colors::RESET
            );
        }

        println!();
    }
}

/// Simple progress indicator
pub fn progress(phase: &Phase) {
    println!(
        "{}→ {} {}...{}",
        Colors::BLUE,
        phase.name,
        Colors::DIM,
        Colors::RESET
    );
}

/// Success banner
pub fn success(msg: &str) {
    println!(
        "{}{}✓ {}{}",
        Colors::GREEN,
        Colors::BOLD,
        msg,
        Colors::RESET
    );
}

/// Error banner
pub fn error(msg: &str) {
    eprintln!(
        "{}{}✗ {}{}",
        Colors::RED,
        Colors::BOLD,
        msg,
        Colors::RESET
    );
}

/// Info message
pub fn info(msg: &str) {
    println!(
        "{}{}• {}{}",
        Colors::CYAN,
        Colors::BOLD,
        msg,
        Colors::RESET
    );
}

/// Start compilation display
pub fn start_compilation(input_file: &str) {
    println!();
    println!(
        "{}Compiling {}{}",
        Colors::BLUE,
        input_file,
        Colors::RESET
    );
}

/// Type mismatch error with suggestions
pub fn type_mismatch(
    expected: &str,
    found: &str,
    context: &str,
    suggestions: &[&str],
) {
    println!();
    eprintln!(
        "{}error{}[E0308]: {}",
        Colors::RED,
        Colors::RESET,
        "type mismatch"
    );
    eprintln!(
        "{}  {}{}",
        Colors::DIM,
        context,
        Colors::RESET
    );
    eprintln!(
        "{}  expected: {} found: {}{}",
        Colors::CYAN,
        expected,
        found,
        Colors::RESET
    );

    if !suggestions.is_empty() {
        eprintln!();
        eprintln!("{}possible solutions:{}",Colors::YELLOW, Colors::RESET);
        for (i, suggestion) in suggestions.iter().enumerate() {
            eprintln!("  {}. {}", i + 1, suggestion);
        }
    }
    eprintln!();
}

/// Enhanced type mismatch with intelligent suggestions
pub fn type_mismatch_with_suggestions(
    expected: &str,
    found: &str,
    variable: Option<&str>,
) {
    use crate::error_suggestions::{TypeErrorSuggester, Confidence};

    eprintln!();
    eprintln!(
        "{}error{}[E0308]: mismatched types",
        Colors::RED,
        Colors::RESET
    );
    eprintln!(
        "{}  expected: {}{}",
        Colors::CYAN,
        expected,
        Colors::RESET
    );
    eprintln!(
        "{}  found:    {}{}",
        Colors::YELLOW,
        found,
        Colors::RESET
    );

    let mut suggestions = TypeErrorSuggester::suggest_type_mismatch(expected, found, variable);
    if !suggestions.is_empty() {
        // Sort by confidence (high first)
        suggestions.sort_by(|a, b| b.confidence.cmp(&a.confidence));
        
        eprintln!();
        eprintln!("{}help:{} Consider these options:", Colors::GREEN, Colors::RESET);
        for (i, suggestion) in suggestions.iter().take(3).enumerate() {
            eprintln!("  {}. {}", i + 1, suggestion.code);
        }
    }
    eprintln!();
}

/// Categorize errors
pub fn categorize_error(message: &str) -> &'static str {
    let lower = message.to_lowercase();
    
    if lower.contains("async") || lower.contains("await") || lower.contains("not implemented") {
        "[compiler limitation]"
    } else if lower.contains("generic") || lower.contains("lifetime") {
        "[type system]"
    } else {
        "[type error]"
    }
}

/// Borrow checker error with explanation
pub fn borrow_error(
    error_type: &str,
    variable: &str,
    reason: &str,
) {
    eprintln!();
    eprintln!(
        "{}error{}[E0502]: {}",
        Colors::RED,
        Colors::RESET,
        error_type
    );
    eprintln!(
        "{}  variable `{}` {}{}",
        Colors::CYAN,
        variable,
        reason,
        Colors::RESET
    );
    eprintln!();
}

/// Lifetime error with visualization
pub fn lifetime_error(
    description: &str,
    scope_a: &str,
    scope_b: &str,
) {
    eprintln!();
    eprintln!(
        "{}error{}[E0623]: {}",
        Colors::RED,
        Colors::RESET,
        "lifetime mismatch"
    );
    eprintln!("{}  {}────────────────────{}",Colors::DIM, Colors::RESET, Colors::RESET);
    eprintln!("{}  {} {}",Colors::CYAN, scope_a, Colors::RESET);
    eprintln!("{}  │{}",Colors::DIM, Colors::RESET);
    eprintln!("{}  └─→ conflicts with {}{}",Colors::DIM, scope_b, Colors::RESET);
    eprintln!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_symbols() {
        assert_eq!(Status::Success.symbol(), "✓");
        assert_eq!(Status::Error.symbol(), "✗");
        assert_eq!(Status::Warning.symbol(), "⚠");
    }

    #[test]
    fn test_phase_constants() {
        assert_eq!(Phase::LEXING.number, 1);
        assert_eq!(Phase::CODEGEN.number, 8);
    }

    #[test]
    fn test_message_builder() {
        let msg = Message::new(Status::Error, "test error")
            .with_phase("Parsing")
            .with_suggestion("check syntax");

        assert_eq!(msg.severity, Status::Error);
        assert_eq!(msg.title, "test error");
        assert!(msg.phase.is_some());
        assert!(msg.suggestion.is_some());
    }
}
