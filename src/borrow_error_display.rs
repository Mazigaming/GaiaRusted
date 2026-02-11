//! Display borrow checker errors with ownership tracking narrative (rustc style)
//!
//! Shows borrow errors with:
//! - What went wrong (use of moved value, multiple mutable borrows, etc)
//! - Why it happened (ownership transferred to X, went out of scope, etc)
//! - Where it happened (specific lines)
//! - How to fix it (borrow with &, clone, restructure)
//!
//! Example format:
//! ```
//! error[E0382]: use of moved value: `x`
//!   |
//! 5 | let y = x;
//!   |         - ownership moved here
//! 6 | let z = x;
//!   |         ^ use of moved value
//! ```

use std::fs;
use std::path::Path;

/// Represents an ownership event (move, borrow, scope end, etc)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipEvent {
    Move,
    BorrowImmutable,
    BorrowMutable,
    ScopeEnd,
    Used,
}

impl OwnershipEvent {
    pub fn description(&self) -> &'static str {
        match self {
            OwnershipEvent::Move => "ownership moved here",
            OwnershipEvent::BorrowImmutable => "immutably borrowed here",
            OwnershipEvent::BorrowMutable => "mutably borrowed here",
            OwnershipEvent::ScopeEnd => "goes out of scope",
            OwnershipEvent::Used => "use of moved value",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            OwnershipEvent::Move => "-",
            OwnershipEvent::BorrowImmutable => "&",
            OwnershipEvent::BorrowMutable => "&mut",
            OwnershipEvent::ScopeEnd => "↓",
            OwnershipEvent::Used => "^",
        }
    }
}

/// Display borrow error with ownership narrative
pub struct BorrowError {
    pub error_code: String,
    pub error_type: String,  // "use of moved value", "multiple mutable borrows", etc
    pub variable: String,
    pub file_path: String,
    pub events: Vec<(usize, OwnershipEvent)>,  // (line_number, event)
    pub primary_line: usize,
    pub suggestions: Vec<String>,
}

impl BorrowError {
    pub fn new(
        error_code: &str,
        error_type: &str,
        variable: &str,
        file_path: &str,
        primary_line: usize,
    ) -> Self {
        BorrowError {
            error_code: error_code.to_string(),
            error_type: error_type.to_string(),
            variable: variable.to_string(),
            file_path: file_path.to_string(),
            events: vec![],
            primary_line,
            suggestions: vec![],
        }
    }

    pub fn add_event(&mut self, line: usize, event: OwnershipEvent) {
        self.events.push((line, event));
    }

    pub fn add_suggestion(&mut self, suggestion: String) {
        self.suggestions.push(suggestion);
    }

    /// Display error in rustc format with ownership narrative
    pub fn display(&self) -> String {
        let mut output = String::new();

        // Header: error[CODE]: error_type
        output.push_str(&format!(
            "{}error[{}]{}: {}: `{}`\n",
            crate::formatter::Colors::RED,
            self.error_code,
            crate::formatter::Colors::RESET,
            self.error_type,
            self.variable
        ));

        // Show all relevant lines with annotations
        if let Ok(source_lines) = self.read_source_lines() {
            output.push_str(&self.format_source_annotations(&source_lines));
        }

        // Narrative explanation
        output.push_str(&self.format_narrative());

        // Suggestions
        if !self.suggestions.is_empty() {
            output.push_str("\n");
            output.push_str(&format!(
                "{}help:{} Try:\n",
                crate::formatter::Colors::GREEN,
                crate::formatter::Colors::RESET
            ));
            for (idx, suggestion) in self.suggestions.iter().enumerate() {
                output.push_str(&format!("  {}. {}\n", idx + 1, suggestion));
            }
        }

        output.push('\n');
        output
    }

    /// Read source lines from file
    fn read_source_lines(&self) -> Result<Vec<String>, std::io::Error> {
        let contents = fs::read_to_string(&self.file_path)?;
        Ok(contents.lines().map(|s| s.to_string()).collect())
    }

    /// Format source code lines with ownership event annotations
    fn format_source_annotations(&self, source_lines: &[String]) -> String {
        let mut output = String::new();

        // Find min and max line numbers to display
        let mut all_lines: Vec<usize> = self.events.iter().map(|(l, _)| *l).collect();
        all_lines.push(self.primary_line);
        all_lines.sort_unstable();
        all_lines.dedup();

        if all_lines.is_empty() {
            return output;
        }

        let line_num_width = all_lines
            .last()
            .unwrap_or(&self.primary_line)
            .to_string()
            .len()
            .max(3);
        let padding = " ".repeat(line_num_width);

        // Show context lines
        let start_line = *all_lines.first().unwrap_or(&self.primary_line);
        let end_line = *all_lines.last().unwrap_or(&self.primary_line);

        output.push_str(&format!("{}  |{}\n", padding, crate::formatter::Colors::DIM));

        for display_line in start_line..=end_line {
            if display_line > 0 && display_line <= source_lines.len() {
                let line_idx = display_line - 1;
                let line_content = &source_lines[line_idx];

                // Print source line
                output.push_str(&format!(
                    "{}{}|{} {}\n",
                    crate::formatter::Colors::DIM,
                    display_line,
                    crate::formatter::Colors::RESET,
                    line_content
                ));

                // Print annotations for events on this line
                for (event_line, event) in &self.events {
                    if *event_line == display_line {
                        output.push_str(&format!(
                            "{}  |{} {}",
                            padding,
                            crate::formatter::Colors::DIM,
                            crate::formatter::Colors::RESET
                        ));

                        // Position annotation under the variable
                        if let Some(pos) = line_content.find(&self.variable) {
                            let spaces = " ".repeat(pos);
                            let symbol = event.symbol();
                            let is_error = *event == OwnershipEvent::Used;

                            if is_error {
                                output.push_str(&format!(
                                    "{}{}{}{}\n",
                                    spaces,
                                    crate::formatter::Colors::RED,
                                    "^".repeat(self.variable.len()),
                                    crate::formatter::Colors::RESET
                                ));
                            } else {
                                output.push_str(&format!(
                                    "{}{}{} {}{}\n",
                                    spaces,
                                    crate::formatter::Colors::CYAN,
                                    symbol,
                                    event.description(),
                                    crate::formatter::Colors::RESET
                                ));
                            }
                        }
                    }
                }
            }
        }

        output
    }

    /// Format narrative explanation of what happened
    fn format_narrative(&self) -> String {
        let mut output = String::new();

        output.push_str("\n");
        output.push_str(&format!(
            "{}`{}` no longer owns the data because:{}\n",
            crate::formatter::Colors::CYAN,
            self.variable,
            crate::formatter::Colors::RESET
        ));

        // Sort events by line number and describe what happened
        let mut sorted_events = self.events.clone();
        sorted_events.sort_by_key(|(line, _)| *line);

        for (line, event) in &sorted_events {
            if *event != OwnershipEvent::Used {
                output.push_str(&format!(
                    "{}• Line {}: {}{}\n",
                    crate::formatter::Colors::DIM,
                    line,
                    event.description(),
                    crate::formatter::Colors::RESET
                ));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_borrow_error_creation() {
        let error = BorrowError::new("E0382", "use of moved value", "x", "test.rs", 5);
        assert_eq!(error.error_code, "E0382");
        assert_eq!(error.variable, "x");
        assert_eq!(error.primary_line, 5);
    }

    #[test]
    fn test_add_events() {
        let mut error = BorrowError::new("E0382", "use of moved value", "x", "test.rs", 5);
        error.add_event(3, OwnershipEvent::Move);
        error.add_event(5, OwnershipEvent::Used);
        assert_eq!(error.events.len(), 2);
    }

    #[test]
    fn test_add_suggestions() {
        let mut error = BorrowError::new("E0382", "use of moved value", "x", "test.rs", 5);
        error.add_suggestion("Use &x (borrow) instead".to_string());
        error.add_suggestion("Use x.clone() to duplicate".to_string());
        assert_eq!(error.suggestions.len(), 2);
    }

    #[test]
    fn test_ownership_event_descriptions() {
        assert_eq!(OwnershipEvent::Move.description(), "ownership moved here");
        assert_eq!(OwnershipEvent::Used.description(), "use of moved value");
        assert_eq!(OwnershipEvent::ScopeEnd.description(), "goes out of scope");
    }
}
