//! Smart error suggestions for common mistakes
//!
//! Provides context-aware suggestions to help users fix compilation errors,
//! similar to rustc's helpful error messages.

use crate::typesystem::HirType;
use std::collections::HashMap;

/// Error suggestions for different error categories
pub struct ErrorSuggestions;

impl ErrorSuggestions {
    /// Suggest a fix for type mismatches
    pub fn type_mismatch(expected: &HirType, found: &HirType) -> Option<String> {
        match (expected, found) {
            // Integer to Float conversions
            (HirType::Float64, HirType::Int64) => {
                Some("try casting: value as f64".to_string())
            }
            (HirType::Float32, HirType::Int32) => {
                Some("try casting: value as f32".to_string())
            }
            // Float to Integer conversions
            (HirType::Int64, HirType::Float64) => {
                Some("try casting: value as i64".to_string())
            }
            (HirType::Int32, HirType::Float32) => {
                Some("try casting: value as i32".to_string())
            }
            // Bool to Integer
            (HirType::Int64, HirType::Bool) => {
                Some("try using: if condition { 1 } else { 0 }".to_string())
            }
            (HirType::Bool, HirType::Int64) => {
                Some("try using: value != 0".to_string())
            }
            // String to str reference
            (HirType::String, HirType::Named(n)) if n == "str" => {
                Some("try using a string literal or &variable".to_string())
            }
            (HirType::Named(n), HirType::String) if n == "str" => {
                Some("try converting: value.as_str()".to_string())
            }
            _ => None,
        }
    }

    /// Suggest candidates for an undefined symbol
    pub fn undefined_symbol(name: &str, available: &[String]) -> Option<String> {
        // Find similar names using simple Levenshtein distance
        let candidates = Self::find_similar(name, available, 2);

        if !candidates.is_empty() {
            if candidates.len() == 1 {
                Some(format!("did you mean: `{}`?", candidates[0]))
            } else {
                let names = candidates.iter()
                    .map(|s| format!("`{}`", s))
                    .collect::<Vec<_>>()
                    .join(", ");
                Some(format!("did you mean one of: {}", names))
            }
        } else {
            None
        }
    }

    /// Suggest a fix for borrowing violations
    pub fn borrow_conflict(var: &str, situation: BorrowSituation) -> String {
        match situation {
            BorrowSituation::UseAfterMove => {
                format!("consider using a reference: &{}, or clone the value", var)
            }
            BorrowSituation::MultipleMutableBorrows => {
                format!("consider using a mutable reference: &mut {}", var)
            }
            BorrowSituation::ImmutableWhileMutable => {
                "split the borrow: take the immutable reference before the mutable use"
                    .to_string()
            }
        }
    }

    /// Suggest a fix for missing methods
    pub fn missing_method(type_name: &str, method: &str, available: &[String]) -> Option<String> {
        if let Some(similar) = Self::find_similar(method, available, 2).first() {
            Some(format!("did you mean: `{}.{}()`?", type_name, similar))
        } else {
            Some(format!(
                "type `{}` has no method named `{}`. Available methods: {}",
                type_name,
                method,
                available.join(", ")
            ))
        }
    }

    /// Find similar strings using simple edit distance
    fn find_similar(target: &str, candidates: &[String], max_distance: usize) -> Vec<String> {
        candidates
            .iter()
            .filter_map(|candidate| {
                let distance = Self::levenshtein_distance(target, candidate);
                if distance <= max_distance && distance > 0 {
                    Some((candidate.clone(), distance))
                } else {
                    None
                }
            })
            .fold(HashMap::new(), |mut acc, (candidate, distance)| {
                acc.entry(distance)
                    .or_default()
                    .push(candidate);
                acc
            })
            .iter()
            .min_by_key(|(distance, _)| *distance)
            .map(|(_, candidates)| candidates.clone())
            .unwrap_or_default()
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(a: &str, b: &str) -> usize {
        let a_len = a.len();
        let b_len = b.len();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];

        for i in 0..=a_len {
            matrix[i][0] = i;
        }
        for j in 0..=b_len {
            matrix[0][j] = j;
        }

        for (i, a_char) in a.chars().enumerate() {
            for (j, b_char) in b.chars().enumerate() {
                let cost = if a_char == b_char { 0 } else { 1 };
                matrix[i + 1][j + 1] = std::cmp::min(
                    std::cmp::min(
                        matrix[i][j + 1] + 1,      // deletion
                        matrix[i + 1][j] + 1,      // insertion
                    ),
                    matrix[i][j] + cost,           // substitution
                );
            }
        }

        matrix[a_len][b_len]
    }

    /// Suggest fix for syntax errors
    pub fn syntax_error(error_type: &str, token: &str) -> Option<String> {
        match error_type {
            "unexpected_eof" => Some(format!(
                "unexpected end of file. Expected closing delimiter for '{}'",
                token
            )),
            "unclosed_paren" => Some("try adding ')' to close this parenthesis".to_string()),
            "unclosed_brace" => Some("try adding '}}' to close this brace".to_string()),
            "unclosed_bracket" => Some("try adding ']' to close this bracket".to_string()),
            "missing_semicolon" => Some("try adding ';' at the end of the statement".to_string()),
            _ => None,
        }
    }

    /// Suggest help for lifetime issues
    pub fn lifetime_issue(issue: LifetimeIssue) -> String {
        match issue {
            LifetimeIssue::MissingLifetime => {
                "consider adding explicit lifetime parameters: fn foo<'a>()".to_string()
            }
            LifetimeIssue::OutlivesMismatch => {
                "consider adjusting the lifetime bound: 'a: 'b".to_string()
            }
            LifetimeIssue::ReferenceEscapes => {
                "consider extending the lifetime of this reference".to_string()
            }
        }
    }
}

/// Types of borrowing conflicts
#[derive(Debug, Clone, Copy)]
pub enum BorrowSituation {
    /// Value used after being moved
    UseAfterMove,
    /// Multiple mutable borrows of the same value
    MultipleMutableBorrows,
    /// Immutable borrow while mutable borrow exists
    ImmutableWhileMutable,
}

/// Types of lifetime issues
#[derive(Debug, Clone, Copy)]
pub enum LifetimeIssue {
    /// Missing required lifetime parameter
    MissingLifetime,
    /// Lifetime bounds don't match
    OutlivesMismatch,
    /// Reference escapes local scope
    ReferenceEscapes,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_mismatch_suggestions() {
        let sugg = ErrorSuggestions::type_mismatch(&HirType::Int64, &HirType::Float64);
        assert!(sugg.is_some());
        assert!(sugg.unwrap().contains("as i64"));
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(ErrorSuggestions::levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(ErrorSuggestions::levenshtein_distance("abc", "abc"), 0);
        assert_eq!(ErrorSuggestions::levenshtein_distance("", "abc"), 3);
    }

    #[test]
    fn test_find_similar() {
        let candidates = vec!["print".to_string(), "println".to_string(), "printf".to_string()];
        let similar = ErrorSuggestions::find_similar("print", &candidates, 2);
        assert!(!similar.is_empty());
        assert!(similar.contains(&"printf".to_string()));
    }

    #[test]
    fn test_undefined_symbol_suggestion() {
        let available = vec!["println".to_string(), "print".to_string(), "printf".to_string()];
        let sugg = ErrorSuggestions::undefined_symbol("prnt", &available);
        assert!(sugg.is_some());
        let msg = sugg.unwrap();
        assert!(msg.contains("print") || msg.contains("printf"));
    }
}
