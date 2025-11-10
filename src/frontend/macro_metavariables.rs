//! Task 6.4 & 6.11: Macro Metavariables and Metavariable Types
//!
//! This module handles recognition and processing of macro metavariables:
//! - Metavariable recognition: $x, $expr, $ty, etc.
//! - Metavariable types: expr, stmt, item, pat, ty, lifetime, block, meta, vis
//! - Repetition patterns: $(...),*, $(...),+, $(...),?

use crate::lexer::token::Token;
use std::collections::HashMap;

/// Types of macro metavariables
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetavariableType {
    Item,
    Block,
    Stmt,
    Expr,
    Pat,
    Ty,
    Ident,
    Path,
    Tt,
    Meta,
    Lifetime,
    Vis,
    Unknown,
}

impl MetavariableType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "item" => MetavariableType::Item,
            "block" => MetavariableType::Block,
            "stmt" => MetavariableType::Stmt,
            "expr" => MetavariableType::Expr,
            "pat" => MetavariableType::Pat,
            "ty" => MetavariableType::Ty,
            "ident" => MetavariableType::Ident,
            "path" => MetavariableType::Path,
            "tt" => MetavariableType::Tt,
            "meta" => MetavariableType::Meta,
            "lifetime" => MetavariableType::Lifetime,
            "vis" => MetavariableType::Vis,
            _ => MetavariableType::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MetavariableType::Item => "item",
            MetavariableType::Block => "block",
            MetavariableType::Stmt => "stmt",
            MetavariableType::Expr => "expr",
            MetavariableType::Pat => "pat",
            MetavariableType::Ty => "ty",
            MetavariableType::Ident => "ident",
            MetavariableType::Path => "path",
            MetavariableType::Tt => "tt",
            MetavariableType::Meta => "meta",
            MetavariableType::Lifetime => "lifetime",
            MetavariableType::Vis => "vis",
            MetavariableType::Unknown => "unknown",
        }
    }
}

/// Macro metavariable analyzer
#[derive(Debug, Clone)]
pub struct MetavariableAnalyzer {
    metavariables: HashMap<String, MetavariableType>,
}

impl MetavariableAnalyzer {
    pub fn new() -> Self {
        MetavariableAnalyzer {
            metavariables: HashMap::new(),
        }
    }

    pub fn register_metavariable(&mut self, name: String, ty: MetavariableType) -> Result<(), String> {
        if name.is_empty() {
            return Err("Metavariable name cannot be empty".to_string());
        }
        self.metavariables.insert(name, ty);
        Ok(())
    }

    pub fn get_metavariable_type(&self, name: &str) -> Option<MetavariableType> {
        self.metavariables.get(name).copied()
    }

    pub fn is_metavariable_registered(&self, name: &str) -> bool {
        self.metavariables.contains_key(name)
    }

    pub fn get_all_metavariables(&self) -> Vec<(String, MetavariableType)> {
        self.metavariables
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    pub fn parse_metavariable_from_token(&mut self, token: &Token) -> Result<String, String> {
        match token {
            Token::Metavariable(name) => {
                let ty = MetavariableType::from_str(name);
                self.register_metavariable(name.clone(), ty)?;
                Ok(name.clone())
            }
            _ => Err(format!("Expected metavariable token, got {:?}", token)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metavariable_type_from_str() {
        assert_eq!(MetavariableType::from_str("expr"), MetavariableType::Expr);
        assert_eq!(MetavariableType::from_str("ty"), MetavariableType::Ty);
        assert_eq!(MetavariableType::from_str("lifetime"), MetavariableType::Lifetime);
    }

    #[test]
    fn test_metavariable_type_as_str() {
        assert_eq!(MetavariableType::Expr.as_str(), "expr");
        assert_eq!(MetavariableType::Ty.as_str(), "ty");
        assert_eq!(MetavariableType::Pat.as_str(), "pat");
    }

    #[test]
    fn test_register_metavariable() {
        let mut analyzer = MetavariableAnalyzer::new();
        assert!(analyzer.register_metavariable("x".to_string(), MetavariableType::Expr).is_ok());
        assert!(analyzer.is_metavariable_registered("x"));
    }

    #[test]
    fn test_get_metavariable_type() {
        let mut analyzer = MetavariableAnalyzer::new();
        analyzer.register_metavariable("expr".to_string(), MetavariableType::Expr).unwrap();
        let ty = analyzer.get_metavariable_type("expr");
        assert_eq!(ty, Some(MetavariableType::Expr));
    }

    #[test]
    fn test_metavariable_type_all_variants() {
        assert_eq!(MetavariableType::Item.as_str(), "item");
        assert_eq!(MetavariableType::Block.as_str(), "block");
        assert_eq!(MetavariableType::Stmt.as_str(), "stmt");
        assert_eq!(MetavariableType::Ident.as_str(), "ident");
        assert_eq!(MetavariableType::Path.as_str(), "path");
        assert_eq!(MetavariableType::Tt.as_str(), "tt");
        assert_eq!(MetavariableType::Meta.as_str(), "meta");
        assert_eq!(MetavariableType::Vis.as_str(), "vis");
    }

    #[test]
    fn test_metavariable_not_registered() {
        let analyzer = MetavariableAnalyzer::new();
        assert!(!analyzer.is_metavariable_registered("x"));
    }

    #[test]
    fn test_get_all_metavariables() {
        let mut analyzer = MetavariableAnalyzer::new();
        analyzer.register_metavariable("x".to_string(), MetavariableType::Expr).unwrap();
        analyzer.register_metavariable("y".to_string(), MetavariableType::Ty).unwrap();
        let all = analyzer.get_all_metavariables();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_metavariable_type_unknown() {
        assert_eq!(MetavariableType::from_str("invalid"), MetavariableType::Unknown);
    }

    #[test]
    fn test_register_multiple_metavariables() {
        let mut analyzer = MetavariableAnalyzer::new();
        analyzer.register_metavariable("expr".to_string(), MetavariableType::Expr).unwrap();
        analyzer.register_metavariable("ty".to_string(), MetavariableType::Ty).unwrap();
        analyzer.register_metavariable("pat".to_string(), MetavariableType::Pat).unwrap();

        assert_eq!(analyzer.get_all_metavariables().len(), 3);
    }

    #[test]
    fn test_parse_metavariable_from_token() {
        let mut analyzer = MetavariableAnalyzer::new();
        let token = Token::Metavariable("x".to_string());
        assert!(analyzer.parse_metavariable_from_token(&token).is_ok());
    }

    #[test]
    fn test_metavariable_empty_name() {
        let mut analyzer = MetavariableAnalyzer::new();
        let result = analyzer.register_metavariable("".to_string(), MetavariableType::Expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_metavariable_overwrite() {
        let mut analyzer = MetavariableAnalyzer::new();
        analyzer.register_metavariable("x".to_string(), MetavariableType::Expr).unwrap();
        analyzer.register_metavariable("x".to_string(), MetavariableType::Ty).unwrap();
        
        let ty = analyzer.get_metavariable_type("x");
        assert_eq!(ty, Some(MetavariableType::Ty));
    }

    #[test]
    fn test_metavariable_type_parsing_all() {
        assert_eq!(MetavariableType::from_str("item"), MetavariableType::Item);
        assert_eq!(MetavariableType::from_str("block"), MetavariableType::Block);
        assert_eq!(MetavariableType::from_str("stmt"), MetavariableType::Stmt);
        assert_eq!(MetavariableType::from_str("expr"), MetavariableType::Expr);
        assert_eq!(MetavariableType::from_str("pat"), MetavariableType::Pat);
        assert_eq!(MetavariableType::from_str("ty"), MetavariableType::Ty);
        assert_eq!(MetavariableType::from_str("ident"), MetavariableType::Ident);
        assert_eq!(MetavariableType::from_str("path"), MetavariableType::Path);
        assert_eq!(MetavariableType::from_str("tt"), MetavariableType::Tt);
        assert_eq!(MetavariableType::from_str("meta"), MetavariableType::Meta);
        assert_eq!(MetavariableType::from_str("lifetime"), MetavariableType::Lifetime);
        assert_eq!(MetavariableType::from_str("vis"), MetavariableType::Vis);
    }
}
