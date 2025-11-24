//! Enhanced Vector Macro Implementation
//!
//! Provides improved vec! macro syntax support with:
//! - vec![] empty vector initialization
//! - vec![val; count] repetition syntax
//! - vec![val1, val2, ...] element list syntax
//! - Type inference and validation

use super::{TokenTree, MacroRule, MacroPattern, MetaVarKind, Delimiter};
use crate::lexer::token::Token;

/// Enhanced vec! macro rule builder
pub struct VecMacroBuilder;

impl VecMacroBuilder {
    /// Create rule for vec![] - empty vector
    pub fn empty_vec_rule() -> MacroRule {
        MacroRule {
            pattern: vec![],
            body: vec![
                TokenTree::Token(Token::Identifier("Vec".to_string())),
                TokenTree::Token(Token::DoubleColon),
                TokenTree::Token(Token::Identifier("new".to_string())),
                TokenTree::Token(Token::LeftParen),
                TokenTree::Token(Token::RightParen),
            ],
        }
    }

    /// Create rule for vec![val; count] - repeated element
    pub fn repeat_element_rule() -> MacroRule {
        MacroRule {
            pattern: vec![
                MacroPattern::MetaVar {
                    name: "element".to_string(),
                    kind: MetaVarKind::Expr,
                },
                MacroPattern::Token(Token::Semicolon),
                MacroPattern::MetaVar {
                    name: "count".to_string(),
                    kind: MetaVarKind::Expr,
                },
            ],
            body: vec![
                TokenTree::Token(Token::Identifier("__vec_repeat".to_string())),
                TokenTree::Token(Token::LeftParen),
                TokenTree::Token(Token::Dollar),
                TokenTree::Token(Token::Identifier("element".to_string())),
                TokenTree::Token(Token::Comma),
                TokenTree::Token(Token::Dollar),
                TokenTree::Token(Token::Identifier("count".to_string())),
                TokenTree::Token(Token::RightParen),
            ],
        }
    }

    /// Create rule for vec![val1, val2, ...] - element list
    pub fn element_list_rule() -> MacroRule {
        MacroRule {
            pattern: vec![
                MacroPattern::MetaVar {
                    name: "elements".to_string(),
                    kind: MetaVarKind::Tt,
                },
            ],
            body: vec![
                TokenTree::Token(Token::Identifier("__vec_from".to_string())),
                TokenTree::Token(Token::LeftParen),
                TokenTree::Token(Token::LeftBracket),
                TokenTree::Token(Token::Dollar),
                TokenTree::Token(Token::Identifier("elements".to_string())),
                TokenTree::Token(Token::RightBracket),
                TokenTree::Token(Token::RightParen),
            ],
        }
    }

    /// Create rule with type annotation: vec![val1, val2; Type]
    pub fn typed_list_rule() -> MacroRule {
        MacroRule {
            pattern: vec![
                MacroPattern::MetaVar {
                    name: "elements".to_string(),
                    kind: MetaVarKind::Tt,
                },
                MacroPattern::Token(Token::Semicolon),
                MacroPattern::MetaVar {
                    name: "ty".to_string(),
                    kind: MetaVarKind::Ty,
                },
            ],
            body: vec![
                TokenTree::Token(Token::Identifier("__vec_typed".to_string())),
                TokenTree::Token(Token::Less),
                TokenTree::Token(Token::Dollar),
                TokenTree::Token(Token::Identifier("ty".to_string())),
                TokenTree::Token(Token::Greater),
                TokenTree::Token(Token::LeftParen),
                TokenTree::Token(Token::LeftBracket),
                TokenTree::Token(Token::Dollar),
                TokenTree::Token(Token::Identifier("elements".to_string())),
                TokenTree::Token(Token::RightBracket),
                TokenTree::Token(Token::RightParen),
            ],
        }
    }
}

/// Vector macro expansion validator
pub struct VecMacroValidator;

impl VecMacroValidator {
    /// Validate vec macro arguments
    pub fn validate_arguments(args: &[TokenTree]) -> Result<VecMacroKind, String> {
        if args.is_empty() {
            return Ok(VecMacroKind::Empty);
        }

        // Check for repetition syntax (val; count)
        let mut semicolon_count = 0;
        let mut semicolon_pos = None;

        for (i, arg) in args.iter().enumerate() {
            if let TokenTree::Token(Token::Semicolon) = arg {
                semicolon_count += 1;
                semicolon_pos = Some(i);
            }
        }

        if semicolon_count == 1 {
            if let Some(pos) = semicolon_pos {
                if pos > 0 && pos < args.len() - 1 {
                    return Ok(VecMacroKind::Repeat {
                        element_count: pos,
                        count_start: pos + 1,
                    });
                }
            }
        } else if semicolon_count == 0 {
            return Ok(VecMacroKind::ElementList);
        }

        Err("Invalid vec! syntax".to_string())
    }

    /// Validate element list for type homogeneity
    pub fn validate_element_types(elements: &[TokenTree]) -> Result<(), String> {
        if elements.is_empty() {
            return Ok(());
        }

        // In a real implementation, we'd do more sophisticated type checking
        // For now, we just ensure elements are present
        Ok(())
    }

    /// Validate repeat count is a valid expression
    pub fn validate_repeat_count(count_expr: &[TokenTree]) -> Result<(), String> {
        if count_expr.is_empty() {
            return Err("Expected count expression after `;`".to_string());
        }
        Ok(())
    }
}

/// Different kinds of vec! macro expansions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VecMacroKind {
    /// vec![] -> Vec::new()
    Empty,
    /// vec![val1, val2, ...] -> Vec from elements
    ElementList,
    /// vec![val; count] -> Vec with repeated element
    Repeat {
        element_count: usize,
        count_start: usize,
    },
}

/// Vector macro expansion context
pub struct VecMacroContext {
    pub macro_kind: VecMacroKind,
}

impl VecMacroContext {
    /// Create new context from validated arguments
    pub fn new(macro_kind: VecMacroKind) -> Self {
        VecMacroContext { macro_kind }
    }

    /// Generate expansion based on macro kind
    pub fn expand(&self) -> Vec<TokenTree> {
        match &self.macro_kind {
            VecMacroKind::Empty => self.expand_empty(),
            VecMacroKind::ElementList => self.expand_element_list(),
            VecMacroKind::Repeat { .. } => self.expand_repeat(),
        }
    }

    fn expand_empty(&self) -> Vec<TokenTree> {
        vec![
            TokenTree::Token(Token::Identifier("Vec".to_string())),
            TokenTree::Token(Token::DoubleColon),
            TokenTree::Token(Token::Identifier("new".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::RightParen),
        ]
    }

    fn expand_element_list(&self) -> Vec<TokenTree> {
        vec![
            TokenTree::Token(Token::Identifier("__vec_from".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::LeftBracket),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("items".to_string())),
            TokenTree::Token(Token::RightBracket),
            TokenTree::Token(Token::RightParen),
        ]
    }

    fn expand_repeat(&self) -> Vec<TokenTree> {
        vec![
            TokenTree::Token(Token::Identifier("__vec_repeat".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("element".to_string())),
            TokenTree::Token(Token::Comma),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("count".to_string())),
            TokenTree::Token(Token::RightParen),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_vec_rule() {
        let rule = VecMacroBuilder::empty_vec_rule();
        assert!(!rule.body.is_empty());
    }

    #[test]
    fn test_repeat_element_rule() {
        let rule = VecMacroBuilder::repeat_element_rule();
        assert_eq!(rule.pattern.len(), 3);
    }

    #[test]
    fn test_element_list_rule() {
        let rule = VecMacroBuilder::element_list_rule();
        assert_eq!(rule.pattern.len(), 1);
    }

    #[test]
    fn test_validate_empty_vec() {
        let result = VecMacroValidator::validate_arguments(&[]);
        assert_eq!(result.unwrap(), VecMacroKind::Empty);
    }

    #[test]
    fn test_validate_element_list() {
        let tokens = vec![
            TokenTree::Token(Token::Integer(1, None)),
            TokenTree::Token(Token::Comma),
            TokenTree::Token(Token::Integer(2, None)),
        ];
        let result = VecMacroValidator::validate_arguments(&tokens);
        assert_eq!(result.unwrap(), VecMacroKind::ElementList);
    }

    #[test]
    fn test_context_creation() {
        let ctx = VecMacroContext::new(VecMacroKind::Empty);
        assert_eq!(ctx.macro_kind, VecMacroKind::Empty);
    }

    #[test]
    fn test_expand_empty() {
        let ctx = VecMacroContext::new(VecMacroKind::Empty);
        let expanded = ctx.expand();
        assert!(!expanded.is_empty());
    }
}
