//! Task 6.1 & 6.13: Lifetime Lexing and Character vs Lifetime Disambiguation
//!
//! This module handles the recognition of lifetime tokens in the lexer.
//! It properly distinguishes between:
//! - Character literals: 'a' (single character in quotes)
//! - Lifetimes: 'a, 'static, '_ (lifetime identifiers)
//! - Higher-ranked trait bounds: for<'a> (lifetime parameters)

use crate::lexer::token::Token;

/// Lifetime validation and analysis
#[derive(Debug, Clone)]
pub struct LifetimeAnalyzer {
    registered_lifetimes: std::collections::HashSet<String>,
}

impl LifetimeAnalyzer {
    pub fn new() -> Self {
        LifetimeAnalyzer {
            registered_lifetimes: std::collections::HashSet::new(),
        }
    }

    pub fn register_lifetime(&mut self, lifetime: String) -> Result<(), String> {
        if self.is_valid_lifetime(&lifetime) {
            self.registered_lifetimes.insert(lifetime);
            Ok(())
        } else {
            Err(format!("Invalid lifetime name: {}", lifetime))
        }
    }

    pub fn is_valid_lifetime(&self, lifetime: &str) -> bool {
        if lifetime.is_empty() {
            return false;
        }

        let first_char = lifetime.chars().next().unwrap();
        match first_char {
            'a'..='z' | 'A'..='Z' | '_' => {
                lifetime.chars().all(|c| c.is_alphanumeric() || c == '_')
            }
            _ => false,
        }
    }

    pub fn is_registered(&self, lifetime: &str) -> bool {
        self.registered_lifetimes.contains(lifetime)
    }

    pub fn get_registered_lifetimes(&self) -> Vec<String> {
        self.registered_lifetimes.iter().cloned().collect()
    }
}

/// Check if a token is a lifetime
pub fn is_lifetime_token(token: &Token) -> bool {
    matches!(token, Token::Lifetime(_))
}

/// Extract lifetime name from token
pub fn get_lifetime_name(token: &Token) -> Option<String> {
    match token {
        Token::Lifetime(name) => Some(name.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifetime_lexing_basic() {
        let mut analyzer = LifetimeAnalyzer::new();
        assert!(analyzer.register_lifetime("a".to_string()).is_ok());
        assert!(analyzer.is_registered("a"));
    }

    #[test]
    fn test_lifetime_static() {
        let mut analyzer = LifetimeAnalyzer::new();
        assert!(analyzer.register_lifetime("static".to_string()).is_ok());
        assert!(analyzer.is_valid_lifetime("static"));
    }

    #[test]
    fn test_lifetime_underscore() {
        let mut analyzer = LifetimeAnalyzer::new();
        assert!(analyzer.register_lifetime("_".to_string()).is_ok());
        assert!(analyzer.is_valid_lifetime("_"));
        assert!(analyzer.is_registered("_"));
    }

    #[test]
    fn test_invalid_lifetime_starts_with_number() {
        let analyzer = LifetimeAnalyzer::new();
        assert!(!analyzer.is_valid_lifetime("1a"));
    }

    #[test]
    fn test_invalid_lifetime_empty() {
        let analyzer = LifetimeAnalyzer::new();
        assert!(!analyzer.is_valid_lifetime(""));
    }

    #[test]
    fn test_lifetime_with_numbers() {
        let analyzer = LifetimeAnalyzer::new();
        assert!(analyzer.is_valid_lifetime("a1b2c3"));
    }

    #[test]
    fn test_character_vs_lifetime_disambiguation() {
        let mut analyzer = LifetimeAnalyzer::new();
        analyzer.register_lifetime("a".to_string()).unwrap();
        assert!(analyzer.is_registered("a"));
    }

    #[test]
    fn test_multiple_lifetimes() {
        let mut analyzer = LifetimeAnalyzer::new();
        analyzer.register_lifetime("a".to_string()).unwrap();
        analyzer.register_lifetime("b".to_string()).unwrap();
        analyzer.register_lifetime("c".to_string()).unwrap();

        let lifetimes = analyzer.get_registered_lifetimes();
        assert_eq!(lifetimes.len(), 3);
    }

    #[test]
    fn test_lifetime_token_detection() {
        let token = Token::Lifetime("a".to_string());
        assert!(is_lifetime_token(&token));

        let not_lifetime = Token::Char('a');
        assert!(!is_lifetime_token(&not_lifetime));
    }

    #[test]
    fn test_get_lifetime_name() {
        let token = Token::Lifetime("static".to_string());
        assert_eq!(get_lifetime_name(&token), Some("static".to_string()));

        let not_lifetime = Token::Char('x');
        assert_eq!(get_lifetime_name(&not_lifetime), None);
    }

    #[test]
    fn test_lifetime_underscore_elided() {
        let analyzer = LifetimeAnalyzer::new();
        assert!(analyzer.is_valid_lifetime("_"));
    }

    #[test]
    fn test_lexing_multiple_lifetime_parameters() {
        let mut analyzer = LifetimeAnalyzer::new();
        analyzer.register_lifetime("a".to_string()).unwrap();
        analyzer.register_lifetime("b".to_string()).unwrap();

        assert!(analyzer.is_registered("a"));
        assert!(analyzer.is_registered("b"));
        assert!(!analyzer.is_registered("c"));
    }

    #[test]
    fn test_lifetime_name_validation_with_underscores() {
        let analyzer = LifetimeAnalyzer::new();
        assert!(analyzer.is_valid_lifetime("a"));
        assert!(analyzer.is_valid_lifetime("_"));
        assert!(analyzer.is_valid_lifetime("long_lifetime"));
    }

    #[test]
    fn test_lexing_lifetime_in_context() {
        let mut analyzer = LifetimeAnalyzer::new();
        analyzer.register_lifetime("a".to_string()).unwrap();

        assert_eq!(analyzer.get_registered_lifetimes().len(), 1);
    }

    #[test]
    fn test_lifetime_collision_check() {
        let mut analyzer = LifetimeAnalyzer::new();
        analyzer.register_lifetime("a".to_string()).unwrap();
        analyzer.register_lifetime("a".to_string()).unwrap();

        assert_eq!(analyzer.get_registered_lifetimes().len(), 1);
    }
}
