use std::collections::HashMap;
use crate::lexer::token::{Token, Keyword};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenTree {
    Token(Token),
    Group {
        delimiter: Delimiter,
        stream: Vec<TokenTree>,
    },
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Delimiter {
    Paren,
    Brace,
    Bracket,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MacroPattern {
    Token(Token),
    MetaVar {
        name: String,
        kind: MetaVarKind,
    },
    Group {
        delimiter: Delimiter,
        patterns: Vec<MacroPattern>,
    },
    Repetition {
        pattern: Box<MacroPattern>,
        separator: Option<Box<Token>>,
        kind: RepetitionKind,
    },
    Or(Vec<Vec<MacroPattern>>),
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum MetaVarKind {
    Expr,
    Ident,
    Ty,
    Path,
    Block,
    Stmt,
    Pat,
    Lit,
    Lifetime,
    Meta,
    Tt,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum RepetitionKind {
    ZeroOrMore,
    OneOrMore,
    ZeroOrOne,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroRule {
    pub pattern: Vec<MacroPattern>,
    pub body: Vec<TokenTree>,
}

#[derive(Debug, Clone)]
pub struct MacroDefinition {
    pub name: String,
    pub rules: Vec<MacroRule>,
}

pub struct MacroExpander {
    definitions: HashMap<String, MacroDefinition>,
}

impl MacroExpander {
    pub fn new() -> Self {
        MacroExpander {
            definitions: HashMap::new(),
        }
    }

    pub fn with_builtins() -> Self {
        let mut expander = MacroExpander::new();
        builtins::register_builtin_macros(&mut expander);
        expander
    }

    pub fn define(&mut self, def: MacroDefinition) {
        self.definitions.insert(def.name.clone(), def);
    }

    pub fn get_definition(&self, name: &str) -> Option<&MacroDefinition> {
        self.definitions.get(name)
    }

    pub fn expand(&self, name: &str, input: Vec<TokenTree>) -> Result<Vec<TokenTree>, String> {
        let definition = self
            .definitions
            .get(name)
            .ok_or_else(|| format!("Macro '{}' not found", name))?;

        for rule in &definition.rules {
            if let Some(bindings) = self.match_pattern(&rule.pattern, &input) {
                return self.substitute(&rule.body, &bindings);
            }
        }

        Err(format!("No matching rule for macro '{}'", name))
    }

    fn match_pattern(
        &self,
        pattern: &[MacroPattern],
        input: &[TokenTree],
    ) -> Option<HashMap<String, Vec<TokenTree>>> {
        let mut bindings = HashMap::new();
        if self.match_patterns(pattern, input, 0, 0, &mut bindings) {
            Some(bindings)
        } else {
            None
        }
    }

    fn match_patterns(
        &self,
        patterns: &[MacroPattern],
        input: &[TokenTree],
        pat_idx: usize,
        input_idx: usize,
        bindings: &mut HashMap<String, Vec<TokenTree>>,
    ) -> bool {
        if pat_idx == patterns.len() {
            return input_idx == input.len();
        }

        let pattern = &patterns[pat_idx];
        match pattern {
            MacroPattern::Token(expected) => {
                if input_idx >= input.len() {
                    return false;
                }
                if let TokenTree::Token(actual) = &input[input_idx] {
                    if self.tokens_match(expected, actual) {
                        return self.match_patterns(patterns, input, pat_idx + 1, input_idx + 1, bindings);
                    }
                }
                false
            }
            MacroPattern::MetaVar { name, kind: _ } => {
                if input_idx >= input.len() {
                    return false;
                }

                let matched_tree = input[input_idx].clone();
                bindings
                    .entry(name.clone())
                    .or_insert_with(Vec::new)
                    .push(matched_tree);

                self.match_patterns(patterns, input, pat_idx + 1, input_idx + 1, bindings)
            }
            MacroPattern::Group {
                delimiter: _,
                patterns: group_patterns,
            } => {
                if input_idx >= input.len() {
                    return false;
                }
                if let TokenTree::Group {
                    delimiter: _,
                    stream,
                } = &input[input_idx]
                {
                    let mut group_bindings = HashMap::new();
                    if self.match_patterns(group_patterns, stream, 0, 0, &mut group_bindings) {
                        for (key, value) in group_bindings {
                            bindings
                                .entry(key)
                                .or_insert_with(Vec::new)
                                .extend(value);
                        }
                        return self.match_patterns(patterns, input, pat_idx + 1, input_idx + 1, bindings);
                    }
                }
                false
            }
            MacroPattern::Repetition {
                pattern: rep_pattern,
                separator: _,
                kind,
            } => {
                let min_matches = match kind {
                    RepetitionKind::ZeroOrMore => 0,
                    RepetitionKind::OneOrMore => 1,
                    RepetitionKind::ZeroOrOne => 0,
                };

                let max_matches = match kind {
                    RepetitionKind::ZeroOrMore => usize::MAX,
                    RepetitionKind::OneOrMore => usize::MAX,
                    RepetitionKind::ZeroOrOne => 1,
                };

                let mut matched = 0;
                let mut current_idx = input_idx;

                while matched < max_matches && current_idx < input.len() {
                    let mut rep_bindings = HashMap::new();
                    if self.match_patterns(&[(**rep_pattern).clone()], input, 0, current_idx, &mut rep_bindings) {
                        for (key, value) in rep_bindings {
                            bindings
                                .entry(key)
                                .or_insert_with(Vec::new)
                                .extend(value);
                        }
                        matched += 1;
                        current_idx += 1;
                    } else {
                        break;
                    }
                }

                if matched >= min_matches {
                    self.match_patterns(patterns, input, pat_idx + 1, current_idx, bindings)
                } else {
                    false
                }
            }
            MacroPattern::Or(alternatives) => {
                for alt in alternatives {
                    let mut alt_bindings = bindings.clone();
                    if self.match_patterns(alt, input, 0, input_idx, &mut alt_bindings) {
                        *bindings = alt_bindings;
                        return self.match_patterns(patterns, input, pat_idx + 1, input_idx, bindings);
                    }
                }
                false
            }
        }
    }

    fn tokens_match(&self, expected: &Token, actual: &Token) -> bool {
        match (expected, actual) {
            (Token::Identifier(e), Token::Identifier(a)) => e == a,
            (Token::Integer(e, _), Token::Integer(a, _)) => e == a,
            (Token::Float(e, _), Token::Float(a, _)) => (e - a).abs() < f64::EPSILON,
            (Token::String(e), Token::String(a)) => e == a,
            (Token::Char(e), Token::Char(a)) => e == a,
            (Token::Keyword(Keyword::True), Token::Keyword(Keyword::True))
            | (Token::Keyword(Keyword::False), Token::Keyword(Keyword::False)) => true,
            (Token::Plus, Token::Plus)
            | (Token::Minus, Token::Minus)
            | (Token::Star, Token::Star)
            | (Token::Slash, Token::Slash)
            | (Token::Percent, Token::Percent)
            | (Token::Equal, Token::Equal)
            | (Token::EqualEqual, Token::EqualEqual)
            | (Token::NotEqual, Token::NotEqual)
            | (Token::Less, Token::Less)
            | (Token::LessEqual, Token::LessEqual)
            | (Token::Greater, Token::Greater)
            | (Token::GreaterEqual, Token::GreaterEqual)
            | (Token::AndAnd, Token::AndAnd)
            | (Token::OrOr, Token::OrOr)
            | (Token::Bang, Token::Bang)
            | (Token::Ampersand, Token::Ampersand)
            | (Token::Caret, Token::Caret)
            | (Token::Pipe, Token::Pipe)
            | (Token::Tilde, Token::Tilde)
            | (Token::LeftShift, Token::LeftShift)
            | (Token::RightShift, Token::RightShift)
            | (Token::Dot, Token::Dot)
            | (Token::Comma, Token::Comma)
            | (Token::Semicolon, Token::Semicolon)
            | (Token::Colon, Token::Colon)
            | (Token::DoubleColon, Token::DoubleColon)
            | (Token::Arrow, Token::Arrow)
            | (Token::FatArrow, Token::FatArrow)
            | (Token::LeftParen, Token::LeftParen)
            | (Token::RightParen, Token::RightParen)
            | (Token::LeftBrace, Token::LeftBrace)
            | (Token::RightBrace, Token::RightBrace)
            | (Token::LeftBracket, Token::LeftBracket)
            | (Token::RightBracket, Token::RightBracket)
            | (Token::Question, Token::Question)
            | (Token::At, Token::At)
            | (Token::Dollar, Token::Dollar)
            | (Token::Hash, Token::Hash) => true,
            _ => false,
        }
    }

    fn substitute(
        &self,
        body: &[TokenTree],
        bindings: &HashMap<String, Vec<TokenTree>>,
    ) -> Result<Vec<TokenTree>, String> {
        self.substitute_internal(body, bindings, 0)
    }

    fn substitute_internal(
        &self,
        body: &[TokenTree],
        bindings: &HashMap<String, Vec<TokenTree>>,
        depth: usize,
    ) -> Result<Vec<TokenTree>, String> {
        if depth > 100 {
            return Err("Macro recursion depth exceeded".to_string());
        }

        let mut result = Vec::new();
        let mut i = 0;

        while i < body.len() {
            match &body[i] {
                TokenTree::Token(Token::Dollar) => {
                    if i + 1 < body.len() {
                        if let TokenTree::Token(Token::Identifier(name)) = &body[i + 1] {
                            if let Some(replacement) = bindings.get(name) {
                                result.extend(replacement.clone());
                                i += 2;
                            } else {
                                return Err(format!("Undefined meta variable: ${}", name));
                            }
                        } else if let TokenTree::Group {
                            delimiter: Delimiter::Paren,
                            stream,
                        } = &body[i + 1]
                        {
                            let (expanded, _consumed) = 
                                self.substitute_repetition(stream, bindings, depth + 1)?;
                            result.extend(expanded);
                            i += 2;
                        } else {
                            return Err("Invalid $ usage in macro body".to_string());
                        }
                    } else {
                        return Err("Trailing $ in macro body".to_string());
                    }
                }
                TokenTree::Group {
                    delimiter,
                    stream,
                } => {
                    let substituted = self.substitute_internal(stream, bindings, depth + 1)?;
                    result.push(TokenTree::Group {
                        delimiter: *delimiter,
                        stream: substituted,
                    });
                    i += 1;
                }
                _ => {
                    result.push(body[i].clone());
                    i += 1;
                }
            }
        }

        Ok(result)
    }

    fn substitute_repetition(
        &self,
        stream: &[TokenTree],
        bindings: &HashMap<String, Vec<TokenTree>>,
        depth: usize,
    ) -> Result<(Vec<TokenTree>, usize), String> {
        let mut result = Vec::new();

        if stream.len() < 3 {
            return Err("Invalid repetition syntax".to_string());
        }

        let pattern = &stream[0..stream.len() - 1];
        let kind_token = &stream[stream.len() - 1];

        let repetition_kind = match kind_token {
            TokenTree::Token(Token::Star) => RepetitionKind::ZeroOrMore,
            TokenTree::Token(Token::Plus) => RepetitionKind::OneOrMore,
            TokenTree::Token(Token::Question) => RepetitionKind::ZeroOrOne,
            _ => return Err("Invalid repetition kind".to_string()),
        };

        match repetition_kind {
            RepetitionKind::ZeroOrMore | RepetitionKind::OneOrMore | RepetitionKind::ZeroOrOne => {
                let count = bindings.values().next()
                    .map(|v| v.len())
                    .unwrap_or(0);

                for _ in 0..count {
                    let expanded = self.substitute_internal(pattern, bindings, depth + 1)?;
                    result.extend(expanded);
                }
            }
        }

        Ok((result, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::token::Token;

    #[test]
    fn test_simple_macro_expansion() {
        let mut expander = MacroExpander::new();
        let rule = MacroRule {
            pattern: vec![
                MacroPattern::MetaVar {
                    name: "a".to_string(),
                    kind: MetaVarKind::Expr,
                },
                MacroPattern::Token(Token::Comma),
                MacroPattern::MetaVar {
                    name: "b".to_string(),
                    kind: MetaVarKind::Expr,
                },
            ],
            body: vec![
                TokenTree::Token(Token::Dollar),
                TokenTree::Token(Token::Identifier("a".to_string())),
                TokenTree::Token(Token::Plus),
                TokenTree::Token(Token::Dollar),
                TokenTree::Token(Token::Identifier("b".to_string())),
            ],
        };

        let def = MacroDefinition {
            name: "add".to_string(),
            rules: vec![rule],
        };

        expander.define(def);

        let input = vec![
            TokenTree::Token(Token::Integer(5, None)),
            TokenTree::Token(Token::Comma),
            TokenTree::Token(Token::Integer(3, None)),
        ];

        let result = expander.expand("add", input);
        assert!(result.is_ok());
    }
}

pub mod parsing;
pub mod hygiene;
pub mod builtins;
pub mod expansion;
pub mod procedural;
pub mod derive;
pub mod custom_derive;
pub mod advanced_macros;
