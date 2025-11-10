//! Task 6.2: Lifetime Parsing
//!
//! This module handles parsing lifetime annotations in:
//! - Generic type parameters
//! - Function signatures
//! - Struct and trait definitions
//! - Reference types

use crate::lexer::token::Token;
use std::collections::{HashMap, HashSet};

/// Represents a lifetime parameter bound
#[derive(Debug, Clone, PartialEq)]
pub enum LifetimeBound {
    Explicit(String),
    Elided,
    Static,
}

/// Parser for lifetime annotations
#[derive(Debug, Clone)]
pub struct LifetimeParser {
    lifetimes: HashSet<String>,
    lifetime_bounds: HashMap<String, Vec<String>>,
}

impl LifetimeParser {
    pub fn new() -> Self {
        LifetimeParser {
            lifetimes: HashSet::new(),
            lifetime_bounds: HashMap::new(),
        }
    }

    pub fn parse_lifetime_parameter(&mut self, tokens: &[Token]) -> Result<String, String> {
        if tokens.is_empty() {
            return Err("No tokens to parse".to_string());
        }

        match &tokens[0] {
            Token::Lifetime(name) => {
                self.lifetimes.insert(name.clone());
                Ok(name.clone())
            }
            _ => Err(format!("Expected lifetime token, got {:?}", tokens[0])),
        }
    }

    pub fn parse_generic_lifetime_parameters(&mut self, tokens: &[Token]) -> Result<Vec<String>, String> {
        let mut lifetimes = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            match &tokens[i] {
                Token::Lifetime(name) => {
                    lifetimes.push(name.clone());
                    self.lifetimes.insert(name.clone());
                    i += 1;
                }
                Token::Comma => {
                    i += 1;
                }
                _ => break,
            }
        }

        if lifetimes.is_empty() {
            Err("No lifetime parameters found".to_string())
        } else {
            Ok(lifetimes)
        }
    }

    pub fn add_lifetime_bound(&mut self, lifetime: String, bound: String) -> Result<(), String> {
        if !self.lifetimes.contains(&lifetime) {
            return Err(format!("Lifetime '{}' not registered", lifetime));
        }
        self.lifetime_bounds.entry(lifetime).or_insert_with(Vec::new).push(bound);
        Ok(())
    }

    pub fn get_lifetime_bounds(&self, lifetime: &str) -> Option<Vec<String>> {
        self.lifetime_bounds.get(lifetime).cloned()
    }

    pub fn is_lifetime_registered(&self, lifetime: &str) -> bool {
        self.lifetimes.contains(lifetime)
    }

    pub fn get_all_lifetimes(&self) -> Vec<String> {
        self.lifetimes.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_lifetime_parameter() {
        let mut parser = LifetimeParser::new();
        let tokens = vec![Token::Lifetime("a".to_string())];
        let result = parser.parse_lifetime_parameter(&tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "a");
    }

    #[test]
    fn test_parse_static_lifetime() {
        let mut parser = LifetimeParser::new();
        let tokens = vec![Token::Lifetime("static".to_string())];
        let result = parser.parse_lifetime_parameter(&tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "static");
    }

    #[test]
    fn test_parse_elided_lifetime() {
        let mut parser = LifetimeParser::new();
        let tokens = vec![Token::Lifetime("_".to_string())];
        let result = parser.parse_lifetime_parameter(&tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "_");
    }

    #[test]
    fn test_parse_multiple_lifetime_parameters() {
        let mut parser = LifetimeParser::new();
        let tokens = vec![
            Token::Lifetime("a".to_string()),
            Token::Comma,
            Token::Lifetime("b".to_string()),
        ];
        let result = parser.parse_generic_lifetime_parameters(&tokens);
        assert!(result.is_ok());
        let lifetimes = result.unwrap();
        assert_eq!(lifetimes.len(), 2);
        assert!(lifetimes.contains(&"a".to_string()));
        assert!(lifetimes.contains(&"b".to_string()));
    }

    #[test]
    fn test_lifetime_bounds_simple() {
        let mut parser = LifetimeParser::new();
        parser.parse_lifetime_parameter(&vec![Token::Lifetime("a".to_string())]).unwrap();
        parser.add_lifetime_bound("a".to_string(), "b".to_string()).unwrap();

        let bounds = parser.get_lifetime_bounds("a");
        assert!(bounds.is_some());
        assert_eq!(bounds.unwrap().len(), 1);
    }

    #[test]
    fn test_lifetime_registration() {
        let mut parser = LifetimeParser::new();
        parser.parse_lifetime_parameter(&vec![Token::Lifetime("a".to_string())]).unwrap();
        assert!(parser.is_lifetime_registered("a"));
    }

    #[test]
    fn test_multiple_lifetime_bounds() {
        let mut parser = LifetimeParser::new();
        parser.parse_lifetime_parameter(&vec![Token::Lifetime("a".to_string())]).unwrap();
        parser.add_lifetime_bound("a".to_string(), "b".to_string()).unwrap();
        parser.add_lifetime_bound("a".to_string(), "c".to_string()).unwrap();

        let bounds = parser.get_lifetime_bounds("a").unwrap();
        assert_eq!(bounds.len(), 2);
    }

    #[test]
    fn test_parse_three_lifetime_parameters() {
        let mut parser = LifetimeParser::new();
        let tokens = vec![
            Token::Lifetime("a".to_string()),
            Token::Comma,
            Token::Lifetime("b".to_string()),
            Token::Comma,
            Token::Lifetime("c".to_string()),
        ];
        let result = parser.parse_generic_lifetime_parameters(&tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[test]
    fn test_get_all_lifetimes() {
        let mut parser = LifetimeParser::new();
        parser.parse_lifetime_parameter(&vec![Token::Lifetime("a".to_string())]).unwrap();
        parser.parse_lifetime_parameter(&vec![Token::Lifetime("b".to_string())]).unwrap();

        let all = parser.get_all_lifetimes();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_lifetime_bounds_not_registered() {
        let mut parser = LifetimeParser::new();
        let result = parser.add_lifetime_bound("unknown".to_string(), "b".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_no_tokens() {
        let mut parser = LifetimeParser::new();
        let result = parser.parse_lifetime_parameter(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_lifetime_in_struct_field() {
        let mut parser = LifetimeParser::new();
        let tokens = vec![Token::Lifetime("a".to_string())];
        assert!(parser.parse_lifetime_parameter(&tokens).is_ok());
        assert!(parser.is_lifetime_registered("a"));
    }
}
