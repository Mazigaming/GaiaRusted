use crate::lexer::token::{Token, Keyword};
use crate::parser::{Parser, ParseError, ParseResult};
use crate::macros::{
    TokenTree, MacroRule, MacroPattern, MetaVarKind, RepetitionKind, Delimiter,
};

impl Parser {
    pub fn parse_macro_rules(&mut self) -> ParseResult<(String, Vec<MacroRule>)> {
        self.expect_keyword(Keyword::MacroRules)?;
        
        // Check for Bang token explicitly instead of using consume
        if !self.check(&Token::Bang) {
            return Err(ParseError::InvalidSyntax("Expected ! after macro_rules".to_string()));
        }
        self.advance();

        let name = self.expect_identifier()?;

        self.consume("{")?;
        let mut rules = Vec::new();

        while !self.check(&Token::RightBrace) {
            let pattern = self.parse_macro_pattern()?;
            self.consume("=>")?;
            let body = self.parse_token_tree_vec()?;
            rules.push(MacroRule { pattern, body });

            if !self.check(&Token::RightBrace) {
                self.consume(";")?;
            }
        }

        self.consume("}")?;
        Ok((name, rules))
    }

    fn parse_macro_pattern(&mut self) -> ParseResult<Vec<MacroPattern>> {
        let mut patterns = Vec::new();

        match self.current() {
            Token::LeftParen => {
                self.advance();
                while !self.check(&Token::RightParen) {
                    patterns.push(self.parse_macro_pattern_element()?);
                    if self.check(&Token::Comma) {
                        self.advance();
                    }
                }
                self.consume(")")?;
            }
            Token::LeftBrace => {
                self.advance();
                while !self.check(&Token::RightBrace) {
                    patterns.push(self.parse_macro_pattern_element()?);
                    if self.check(&Token::Comma) {
                        self.advance();
                    }
                }
                self.consume("}")?;
            }
            Token::LeftBracket => {
                self.advance();
                while !self.check(&Token::RightBracket) {
                    patterns.push(self.parse_macro_pattern_element()?);
                    if self.check(&Token::Comma) {
                        self.advance();
                    }
                }
                self.consume("]")?;
            }
            _ => return Err(ParseError::InvalidSyntax("Expected macro pattern delimiters".to_string())),
        }

        Ok(patterns)
    }

    fn parse_macro_pattern_element(&mut self) -> ParseResult<MacroPattern> {
        match self.current() {
            Token::Dollar => {
                self.advance();
                if self.check(&Token::LeftParen) {
                    self.advance();
                    let pattern = Box::new(self.parse_macro_pattern_element()?);
                    let separator = if self.check(&Token::Comma) || self.check(&Token::Semicolon) {
                        let sep_token = self.current().clone();
                        self.advance();
                        Some(Box::new(sep_token))
                    } else {
                        None
                    };

                    let kind = if self.check(&Token::Star) {
                        self.advance();
                        RepetitionKind::ZeroOrMore
                    } else if self.check(&Token::Plus) {
                        self.advance();
                        RepetitionKind::OneOrMore
                    } else if self.check(&Token::Question) {
                        self.advance();
                        RepetitionKind::ZeroOrOne
                    } else {
                        return Err(ParseError::InvalidSyntax("Expected *, +, or ?".to_string()));
                    };

                    self.consume(")")?;
                    Ok(MacroPattern::Repetition {
                        pattern,
                        separator,
                        kind,
                    })
                } else if self.check(&Token::Identifier(String::new())) {
                    let name = self.expect_identifier()?;
                    self.consume(":")?;
                    let kind_str = self.expect_identifier()?;
                    let kind = match kind_str.as_str() {
                        "expr" => MetaVarKind::Expr,
                        "ident" => MetaVarKind::Ident,
                        "ty" => MetaVarKind::Ty,
                        "path" => MetaVarKind::Path,
                        "block" => MetaVarKind::Block,
                        "stmt" => MetaVarKind::Stmt,
                        "pat" => MetaVarKind::Pat,
                        "lit" => MetaVarKind::Lit,
                        "lifetime" => MetaVarKind::Lifetime,
                        "meta" => MetaVarKind::Meta,
                        "tt" => MetaVarKind::Tt,
                        _ => return Err(ParseError::InvalidSyntax(format!("Unknown meta-var kind: {}", kind_str))),
                    };
                    Ok(MacroPattern::MetaVar { name, kind })
                } else {
                    Err(ParseError::InvalidSyntax("Expected identifier after $".to_string()))
                }
            }
            Token::LeftParen | Token::LeftBrace | Token::LeftBracket => {
                let delimiter = match self.current() {
                    Token::LeftParen => {
                        self.advance();
                        Delimiter::Paren
                    }
                    Token::LeftBrace => {
                        self.advance();
                        Delimiter::Brace
                    }
                    Token::LeftBracket => {
                        self.advance();
                        Delimiter::Bracket
                    }
                    _ => unreachable!(),
                };

                let mut patterns = Vec::new();
                let closing = match delimiter {
                    Delimiter::Paren => Token::RightParen,
                    Delimiter::Brace => Token::RightBrace,
                    Delimiter::Bracket => Token::RightBracket,
                };

                while self.current() != &closing {
                    patterns.push(self.parse_macro_pattern_element()?);
                }
                self.advance();

                Ok(MacroPattern::Group { delimiter, patterns })
            }
            _ => {
                let token = self.current().clone();
                self.advance();
                Ok(MacroPattern::Token(token))
            }
        }
    }

    fn parse_token_tree_vec(&mut self) -> ParseResult<Vec<TokenTree>> {
        let mut trees = Vec::new();

        match self.current() {
            Token::LeftParen | Token::LeftBrace | Token::LeftBracket => {
                while !matches!(self.current(), Token::Semicolon | Token::RightBrace) {
                    trees.push(self.parse_token_tree()?);
                }
            }
            _ => {
                return Err(ParseError::InvalidSyntax("Expected token tree".to_string()));
            }
        }

        Ok(trees)
    }

    fn parse_token_tree(&mut self) -> ParseResult<TokenTree> {
        match self.current() {
            Token::LeftParen => {
                self.advance();
                let stream = self.parse_token_tree_until(&Token::RightParen)?;
                self.consume(")")?;
                Ok(TokenTree::Group {
                    delimiter: Delimiter::Paren,
                    stream,
                })
            }
            Token::LeftBrace => {
                self.advance();
                let stream = self.parse_token_tree_until(&Token::RightBrace)?;
                self.consume("}")?;
                Ok(TokenTree::Group {
                    delimiter: Delimiter::Brace,
                    stream,
                })
            }
            Token::LeftBracket => {
                self.advance();
                let stream = self.parse_token_tree_until(&Token::RightBracket)?;
                self.consume("]")?;
                Ok(TokenTree::Group {
                    delimiter: Delimiter::Bracket,
                    stream,
                })
            }
            _ => {
                let token = self.current().clone();
                self.advance();
                Ok(TokenTree::Token(token))
            }
        }
    }

    fn parse_token_tree_until(&mut self, end: &Token) -> ParseResult<Vec<TokenTree>> {
        let mut trees = Vec::new();

        while self.current() != end && !matches!(self.current(), Token::Semicolon | Token::RightBrace) {
            trees.push(self.parse_token_tree()?);
        }

        Ok(trees)
    }
}
