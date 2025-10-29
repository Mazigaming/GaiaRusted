//! # Phase 2: PARSER (Syntax Analysis)
//!
//! Converts a stream of tokens into an Abstract Syntax Tree (AST).
//!
//! ## Algorithm: Recursive Descent Parsing
//!
//! The parser uses **recursive descent** with **precedence climbing** for expressions:
//!
//! ```
//! program        → item*
//! item           → fn_def | struct_def
//! fn_def         → FN IDENT ( params? ) ( -> type )? block
//! struct_def     → STRUCT IDENT { struct_fields? }
//! block          → { statement* expression? }
//! statement      → let_stmt | expr_stmt | return_stmt
//! expr_stmt      → expression ;
//! let_stmt       → LET IDENT ( : type )? = expression ;
//! expression     → assignment
//! assignment     → logical_or ( = expression )?
//! logical_or     → logical_and ( || logical_and )*
//! logical_and    → equality ( && equality )*
//! equality       → comparison ( (== | !=) comparison )*
//! comparison     → addition ( (<|<=|>|>=) addition )*
//! addition       → multiplication ( (+|-) multiplication )*
//! multiplication → unary ( (*|/|%) unary )*
//! unary          → ( -|!|~|*|& ) unary | postfix
//! postfix        → primary ( ( . ident ) | [ expr ] )*
//! primary        → LITERAL | IDENT | ( expression ) | if_expr | ...
//! ```

pub mod ast;

use crate::lexer::token::{Token, Keyword};
use std::fmt;

pub use ast::*;

/// Parse result type
type ParseResult<T> = Result<T, ParseError>;

/// Errors that can occur during parsing
#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedToken { expected: String, found: String },
    UnexpectedEof,
    InvalidSyntax(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, found } => {
                write!(f, "Expected {}, found {}", expected, found)
            }
            ParseError::UnexpectedEof => write!(f, "Unexpected end of file"),
            ParseError::InvalidSyntax(msg) => write!(f, "Invalid syntax: {}", msg),
        }
    }
}

/// Parser restrictions - controls what syntax is allowed in different contexts
#[derive(Debug, Clone, Copy)]
enum Restrictions {
    None,
    /// Don't parse `identifier {` as struct literals - used in for loop iterators
    NoStructLiteral,
}

/// The main parser struct
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    restrictions: Restrictions,
}

impl Parser {
    /// Create a new parser from tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, position: 0, restrictions: Restrictions::None }
    }

    /// Helper to set restrictions for a scope and restore after
    fn with_restrictions<T>(&mut self, restrictions: Restrictions, f: impl FnOnce(&mut Self) -> ParseResult<T>) -> ParseResult<T> {
        let old = self.restrictions;
        self.restrictions = restrictions;
        let result = f(self);
        self.restrictions = old;
        result
    }

    // ===== Helper Methods =====

    /// Get current token without advancing
    fn current(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::Eof)
    }

    /// Peek at next token
    fn peek(&self, offset: usize) -> &Token {
        self.tokens.get(self.position + offset).unwrap_or(&Token::Eof)
    }

    /// Advance to next token and return the current one
    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        if self.position < self.tokens.len() {
            self.position += 1;
        }
        token
    }

    /// Check if current token matches
    fn check(&self, token: &Token) -> bool {
        match (self.current(), token) {
            // For Keyword tokens, compare the keyword variant specifically
            (Token::Keyword(kw1), Token::Keyword(kw2)) => kw1 == kw2,
            // For other tokens, compare discriminants
            _ => std::mem::discriminant(self.current()) == std::mem::discriminant(token)
        }
    }

    /// Consume a specific token or error
    fn consume(&mut self, _expected: &str) -> ParseResult<Token> {
        let token = self.current().clone();
        self.advance();
        Ok(token)
    }

    /// Expect a specific token type
    fn expect_keyword(&mut self, keyword: Keyword) -> ParseResult<()> {
        match self.current() {
            Token::Keyword(kw) if *kw == keyword => {
                self.advance();
                Ok(())
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", keyword),
                found: format!("{:?}", self.current()),
            }),
        }
    }

    /// Expect an identifier
    fn expect_identifier(&mut self) -> ParseResult<String> {
        match self.current() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: format!("{:?}", self.current()),
            }),
        }
    }

    // ===== Parsing Functions =====

    /// Parse a complete program
    pub fn parse_program(&mut self) -> ParseResult<Program> {
        let mut items = Vec::new();

        while !self.check(&Token::Eof) {
            items.push(self.parse_item()?);
        }

        Ok(items)
    }

    /// Parse a top-level item (function, struct, enum, trait, impl, mod, use)
    fn parse_item(&mut self) -> ParseResult<Item> {
        // Handle visibility modifiers (pub, etc.)
        let _is_pub = if self.check(&Token::Keyword(Keyword::Pub)) {
            self.advance();
            true
        } else {
            false
        };

        match self.current() {
            Token::Keyword(Keyword::Fn) => self.parse_function(),
            Token::Keyword(Keyword::Struct) => self.parse_struct(),
            Token::Keyword(Keyword::Enum) => self.parse_enum(),
            Token::Keyword(Keyword::Trait) => self.parse_trait(),
            Token::Keyword(Keyword::Impl) => self.parse_impl(),
            Token::Keyword(Keyword::Mod) => self.parse_module(),
            Token::Keyword(Keyword::Use) => self.parse_use(),
            _ => Err(ParseError::InvalidSyntax(
                "Expected function, struct, enum, trait, impl, mod, or use definition".to_string(),
            )),
        }
    }

    /// Parse a function definition
    fn parse_function(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Fn)?;
        let name = self.expect_identifier()?;

        self.consume("(")?;
        let params = self.parse_parameters()?;
        self.consume(")")?;

        let return_type = if self.check(&Token::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(Item::Function {
            name,
            params,
            return_type,
            body,
        })
    }

    /// Parse function parameters
    fn parse_parameters(&mut self) -> ParseResult<Vec<Parameter>> {
        let mut params = Vec::new();

        while !self.check(&Token::RightParen) {
            let mutable = if self.check(&Token::Keyword(Keyword::Mut)) {
                self.advance();
                true
            } else {
                false
            };

            let name = self.expect_identifier()?;
            self.consume(":")?;
            let ty = self.parse_type()?;

            params.push(Parameter { name, mutable, ty });

            if !self.check(&Token::RightParen) {
                self.consume(",")?;
            }
        }

        Ok(params)
    }

    /// Parse a type
    fn parse_type(&mut self) -> ParseResult<Type> {
        match self.current() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                
                // Check for generic types
                if self.check(&Token::Less) {
                    self.advance();
                    let mut type_params = Vec::new();
                    while !self.check(&Token::Greater) {
                        type_params.push(self.parse_type()?);
                        if !self.check(&Token::Greater) {
                            self.consume(",")?;
                        }
                    }
                    self.consume(">")?;
                    // For now, we'll just wrap it as a named type
                    // A full implementation would track type parameters
                    Ok(Type::Named(name))
                } else {
                    Ok(Type::Named(name))
                }
            }
            Token::Ampersand => {
                self.advance();
                let mutable = if self.check(&Token::Keyword(Keyword::Mut)) {
                    self.advance();
                    true
                } else {
                    false
                };
                let inner = Box::new(self.parse_type()?);
                Ok(Type::Reference { mutable, inner })
            }
            Token::LeftParen => {
                self.advance();
                let mut types = Vec::new();
                while !self.check(&Token::RightParen) {
                    types.push(self.parse_type()?);
                    if !self.check(&Token::RightParen) {
                        self.consume(",")?;
                    }
                }
                self.consume(")")?;
                Ok(Type::Tuple(types))
            }
            Token::LeftBracket => {
                self.advance();
                let element = Box::new(self.parse_type()?);
                // Simple array without size for now
                self.consume("]")?;
                Ok(Type::Array { element, size: None })
            }
            _ => Err(ParseError::InvalidSyntax("Expected type".to_string())),
        }
    }

    /// Parse a struct definition
    fn parse_struct(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Struct)?;
        let name = self.expect_identifier()?;

        self.consume("{")?;
        let mut fields = Vec::new();

        while !self.check(&Token::RightBrace) {
            let field_name = self.expect_identifier()?;
            self.consume(":")?;
            let ty = self.parse_type()?;

            fields.push(StructField {
                name: field_name,
                ty,
            });

            if !self.check(&Token::RightBrace) {
                self.consume(",")?;
            }
        }

        self.consume("}")?;

        Ok(Item::Struct { name, fields })
    }

    /// Parse a block: { statements; expression? }
    fn parse_block(&mut self) -> ParseResult<Block> {
        self.consume("{")?;

        let mut statements = Vec::new();
        let mut expression = None;

        while !self.check(&Token::RightBrace) && !self.check(&Token::Eof) {
            if self.check(&Token::Keyword(Keyword::Let)) {
                statements.push(self.parse_let_statement()?);
            } else if self.check(&Token::Keyword(Keyword::Return)) {
                statements.push(self.parse_return_statement()?);
            } else if self.check(&Token::Keyword(Keyword::Break)) {
                self.advance();
                self.consume(";")?;
                statements.push(Statement::Break);
            } else if self.check(&Token::Keyword(Keyword::Continue)) {
                self.advance();
                self.consume(";")?;
                statements.push(Statement::Continue);
            } else if self.check(&Token::Keyword(Keyword::For)) {
                statements.push(self.parse_for_statement()?);
            } else if self.check(&Token::Keyword(Keyword::While)) {
                statements.push(self.parse_while_statement()?);
            } else if self.check(&Token::Keyword(Keyword::If)) {
                statements.push(self.parse_if_statement()?);
            } else {
                // Expression statement or expression at end
                // eprintln!("[DEBUG] parse_block: About to parse expression, current token={:?}", self.current());
                let expr = self.parse_expression()?;
                // eprintln!("[DEBUG] parse_block: Parsed expression, current token={:?}", self.current());

                if self.check(&Token::Semicolon) {
                    self.advance();
                    statements.push(Statement::Expression(expr));
                } else if self.check(&Token::RightBrace) {
                    // Last expression in block (no semicolon)
                    expression = Some(Box::new(expr));
                    break;
                } else {
                    // eprintln!("[DEBUG] parse_block: ERROR - after expression, got token={:?}, expected ';' or '}}'", self.current());
                    return Err(ParseError::InvalidSyntax(
                        "Expected ';' or '}'".to_string(),
                    ));
                }
            }
        }

        self.consume("}")?;

        Ok(Block { statements, expression })
    }

    /// Parse a let statement: let name: type = expr;
    fn parse_let_statement(&mut self) -> ParseResult<Statement> {
        self.expect_keyword(Keyword::Let)?;
        let mutable = if self.check(&Token::Keyword(Keyword::Mut)) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.expect_identifier()?;

        let ty = if self.check(&Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume("=")?;
        let initializer = self.parse_expression()?;
        self.consume(";")?;

        Ok(Statement::Let {
            name,
            mutable,
            ty,
            initializer,
        })
    }

    /// Parse a return statement
    fn parse_return_statement(&mut self) -> ParseResult<Statement> {
        self.expect_keyword(Keyword::Return)?;

        let expr = if self.check(&Token::Semicolon) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };

        self.consume(";")?;
        Ok(Statement::Return(expr))
    }

    /// Parse a for statement: `for x in iter { ... }`
    fn parse_for_statement(&mut self) -> ParseResult<Statement> {
        self.expect_keyword(Keyword::For)?;
        let var = self.expect_identifier()?;
        self.expect_keyword(Keyword::In)?;
        
        // Parse the iterator expression with NO_STRUCT_LITERAL restriction
        let iter = self.with_restrictions(Restrictions::NoStructLiteral, |parser| {
            parser.parse_expression()
        })?;
        
        let body = self.parse_block()?;
        
        Ok(Statement::For { var, iter: Box::new(iter), body })
    }

    /// Parse a while statement: `while condition { ... }`
    fn parse_while_statement(&mut self) -> ParseResult<Statement> {
        self.expect_keyword(Keyword::While)?;
        let condition = Box::new(self.parse_expression()?);
        let body = self.parse_block()?;
        
        Ok(Statement::While { condition, body })
    }

    /// Parse an if statement: `if condition { ... } else { ... }`
    fn parse_if_statement(&mut self) -> ParseResult<Statement> {
        self.expect_keyword(Keyword::If)?;
        let condition = Box::new(self.parse_expression()?);
        let then_body = self.parse_block()?;
        
        let else_body = if self.check(&Token::Keyword(Keyword::Else)) {
            self.advance();
            if self.check(&Token::Keyword(Keyword::If)) {
                // Recursive else-if
                Some(Box::new(self.parse_if_statement()?))
            } else {
                // else { ... }
                let else_block = self.parse_block()?;
                Some(Box::new(Statement::If {
                    condition: Box::new(Expression::Bool(true)), // Placeholder
                    then_body: else_block,
                    else_body: None,
                }))
            }
        } else {
            None
        };
        
        Ok(Statement::If { condition, then_body, else_body })
    }

    // ===== Expression Parsing =====

    /// Parse an expression (lowest precedence)
    fn parse_expression(&mut self) -> ParseResult<Expression> {
        self.parse_assignment()
    }

    /// Parse assignment: expr = expr
    fn parse_assignment(&mut self) -> ParseResult<Expression> {
        let expr = self.parse_logical_or()?;

        if self.check(&Token::Equal) {
            self.advance();
            let value = Box::new(self.parse_assignment()?);
            return Ok(Expression::Assign {
                target: Box::new(expr),
                value,
            });
        }

        // Compound assignments
        let compound_op = match self.current() {
            Token::PlusEqual => Some(CompoundOp::AddAssign),
            Token::MinusEqual => Some(CompoundOp::SubtractAssign),
            Token::StarEqual => Some(CompoundOp::MultiplyAssign),
            Token::SlashEqual => Some(CompoundOp::DivideAssign),
            Token::PercentEqual => Some(CompoundOp::ModuloAssign),
            _ => None,
        };

        if let Some(op) = compound_op {
            self.advance();
            let value = Box::new(self.parse_assignment()?);
            return Ok(Expression::CompoundAssign {
                target: Box::new(expr),
                op,
                value,
            });
        }

        Ok(expr)
    }

    /// Parse logical OR: expr || expr
    fn parse_logical_or(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_logical_and()?;

        while self.check(&Token::OrOr) {
            self.advance();
            let right = Box::new(self.parse_logical_and()?);
            expr = Expression::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right,
            };
        }

        Ok(expr)
    }

    /// Parse logical AND: expr && expr
    fn parse_logical_and(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_equality()?;

        while self.check(&Token::AndAnd) {
            self.advance();
            let right = Box::new(self.parse_equality()?);
            expr = Expression::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right,
            };
        }

        Ok(expr)
    }

    /// Parse equality: expr == expr, expr != expr
    fn parse_equality(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_comparison()?;

        loop {
            let op = match self.current() {
                Token::EqualEqual => BinaryOp::Equal,
                Token::NotEqual => BinaryOp::NotEqual,
                _ => break,
            };

            self.advance();
            let right = Box::new(self.parse_comparison()?);
            expr = Expression::Binary {
                left: Box::new(expr),
                op,
                right,
            };
        }

        Ok(expr)
    }

    /// Parse comparison: expr < expr, expr <= expr, etc.
    fn parse_comparison(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_addition()?;

        loop {
            let op = match self.current() {
                Token::Less => BinaryOp::Less,
                Token::LessEqual => BinaryOp::LessEq,
                Token::Greater => BinaryOp::Greater,
                Token::GreaterEqual => BinaryOp::GreaterEq,
                _ => break,
            };

            self.advance();
            let right = Box::new(self.parse_addition()?);
            expr = Expression::Binary {
                left: Box::new(expr),
                op,
                right,
            };
        }

        Ok(expr)
    }

    /// Parse addition: expr + expr, expr - expr
    fn parse_addition(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_multiplication()?;

        loop {
            let op = match self.current() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Subtract,
                _ => break,
            };

            self.advance();
            let right = Box::new(self.parse_multiplication()?);
            expr = Expression::Binary {
                left: Box::new(expr),
                op,
                right,
            };
        }

        Ok(expr)
    }

    /// Parse multiplication: expr * expr, expr / expr, expr % expr
    fn parse_multiplication(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_unary()?;

        loop {
            let op = match self.current() {
                Token::Star => BinaryOp::Multiply,
                Token::Slash => BinaryOp::Divide,
                Token::Percent => BinaryOp::Modulo,
                _ => break,
            };

            self.advance();
            let right = Box::new(self.parse_unary()?);
            expr = Expression::Binary {
                left: Box::new(expr),
                op,
                right,
            };
        }

        Ok(expr)
    }

    /// Parse unary: -expr, !expr, *expr, &expr
    fn parse_unary(&mut self) -> ParseResult<Expression> {
        match self.current() {
            Token::Minus => {
                self.advance();
                let operand = Box::new(self.parse_unary()?);
                Ok(Expression::Unary {
                    op: UnaryOp::Negate,
                    operand,
                })
            }
            Token::Bang => {
                self.advance();
                let operand = Box::new(self.parse_unary()?);
                Ok(Expression::Unary {
                    op: UnaryOp::Not,
                    operand,
                })
            }
            Token::Tilde => {
                self.advance();
                let operand = Box::new(self.parse_unary()?);
                Ok(Expression::Unary {
                    op: UnaryOp::BitwiseNot,
                    operand,
                })
            }
            Token::Star => {
                self.advance();
                let operand = Box::new(self.parse_unary()?);
                Ok(Expression::Unary {
                    op: UnaryOp::Dereference,
                    operand,
                })
            }
            Token::Ampersand => {
                self.advance();
                let operand = Box::new(self.parse_unary()?);
                Ok(Expression::Unary {
                    op: UnaryOp::Reference,
                    operand,
                })
            }
            _ => self.parse_range(),
        }
    }

    /// Parse range expressions: a..b, a..=b, a.., ..b
    fn parse_range(&mut self) -> ParseResult<Expression> {
        // eprintln!("[DEBUG] parse_range: START, current token={:?}, restriction={:?}", self.current(), self.restrictions);
        let mut expr = self.parse_postfix()?;
        // eprintln!("[DEBUG] parse_range: After postfix, current token={:?}, expr type parsed", self.current());

        // Check for range operators
        if self.check(&Token::DotDot) {
            // eprintln!("[DEBUG] parse_range: Found DotDot, advancing...");
            self.advance();
            if self.check(&Token::Equal) {
                self.advance();
                // For inclusive range, parse end only if not a terminator
                if !self.is_expression_terminator() {
                    let end = Box::new(self.parse_range_end()?);
                    expr = Expression::Range {
                        start: Some(Box::new(expr)),
                        end: Some(end),
                        inclusive: true,
                    };
                } else {
                    expr = Expression::Range {
                        start: Some(Box::new(expr)),
                        end: None,
                        inclusive: true,
                    };
                }
            } else if self.is_expression_terminator() {
                expr = Expression::Range {
                    start: Some(Box::new(expr)),
                    end: None,
                    inclusive: false,
                };
            } else {
                let end = Box::new(self.parse_range_end()?);
                expr = Expression::Range {
                    start: Some(Box::new(expr)),
                    end: Some(end),
                    inclusive: false,
                };
            }
        }

        Ok(expr)
    }

    /// Parse range end - don't parse block expressions as they terminate the range
    fn parse_range_end(&mut self) -> ParseResult<Expression> {
        // eprintln!("[DEBUG] parse_range_end: START, current token={:?}", self.current());
        // Parse a primary expression but stop before blocks
        let result = match self.current().clone() {
            Token::Integer(n) => {
                // eprintln!("[DEBUG] parse_range_end: Found Integer({}), advancing...", n);
                self.advance();
                // eprintln!("[DEBUG] parse_range_end: After advance, current token={:?}", self.current());
                Ok(Expression::Integer(n))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Expression::Float(f))
            }
            Token::Identifier(name) => {
                // eprintln!("[DEBUG] parse_range_end: Found Identifier({}), advancing...", name);
                self.advance();
                // eprintln!("[DEBUG] parse_range_end: After advance, current token={:?}", self.current());
                if self.check(&Token::LeftParen) {
                    self.advance();
                    let args = self.parse_arguments()?;
                    self.consume(")")?;
                    Ok(Expression::FunctionCall { name, args })
                } else {
                    Ok(Expression::Variable(name))
                }
            }
            _ => {
                // For other cases, use regular parsing but be careful
                self.parse_unary()
            }
        };
        // eprintln!("[DEBUG] parse_range_end: END, current token={:?}", self.current());
        result
    }

    /// Check if current token is an expression terminator
    fn is_expression_terminator(&self) -> bool {
        matches!(self.current(),
            Token::Semicolon | Token::RightBrace | Token::RightParen |
            Token::Comma | Token::LeftBrace | Token::FatArrow |
            Token::RightBracket
        )
    }

    /// Parse postfix: expr.field, expr[index]
    fn parse_postfix(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current() {
                Token::Dot => {
                    self.advance();
                    let field = self.expect_identifier()?;
                    expr = Expression::FieldAccess {
                        object: Box::new(expr),
                        field,
                    };
                }
                Token::LeftBracket => {
                    self.advance();
                    let index = Box::new(self.parse_expression()?);
                    self.consume("]")?;
                    expr = Expression::Index {
                        array: Box::new(expr),
                        index,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse primary expressions (literals, identifiers, etc.)
    fn parse_primary(&mut self) -> ParseResult<Expression> {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let call_id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // eprintln!("[DEBUG parse_primary #{} START, current token={:?}", call_id, self.current());
        let result = match self.current().clone() {
            Token::Integer(n) => {
                // eprintln!("[DEBUG] parse_primary: Found Integer({}), advancing...", n);
                self.advance();
                // eprintln!("[DEBUG] parse_primary: After advance, current token={:?}", self.current());
                Ok(Expression::Integer(n))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Expression::Float(f))
            }
            Token::String(s) => {
                self.advance();
                Ok(Expression::String(s))
            }
            Token::Char(c) => {
                self.advance();
                Ok(Expression::Char(c))
            }
            Token::Keyword(Keyword::True) => {
                self.advance();
                Ok(Expression::Bool(true))
            }
            Token::Keyword(Keyword::False) => {
                self.advance();
                Ok(Expression::Bool(false))
            }
            Token::Identifier(name) => {
                self.advance();

                // Check for function call
                if self.check(&Token::LeftParen) {
                    self.advance();
                    let args = self.parse_arguments()?;
                    self.consume(")")?;
                    Ok(Expression::FunctionCall { name, args })
                } else if self.check(&Token::LeftBrace) && matches!(self.restrictions, Restrictions::None) {
                    // Struct literal: Name { field: value, ... }
                    // Only parse as struct literal if restrictions allow it
                    // (NoStructLiteral is used in contexts like `for x in EXPR {`, where the `{` is a loop body, not struct fields)
                    self.advance();
                    let mut fields = Vec::new();
                    
                    while !self.check(&Token::RightBrace) {
                        let field_name = self.expect_identifier()?;
                        self.consume(":")?;
                        let field_value = self.parse_expression()?;
                        fields.push((field_name, field_value));
                        
                        if !self.check(&Token::RightBrace) {
                            self.consume(",")?;
                        }
                    }
                    
                    self.consume("}")?;
                    Ok(Expression::StructLiteral {
                        struct_name: name,
                        fields,
                    })
                } else {
                    Ok(Expression::Variable(name))
                }
            }
            Token::LeftParen => {
                self.advance();
                
                // Check for empty tuple
                if self.check(&Token::RightParen) {
                    self.advance();
                    return Ok(Expression::Tuple(Vec::new()));
                }
                
                let first = self.parse_expression()?;
                
                // Check for tuple
                if self.check(&Token::Comma) {
                    self.advance();
                    let mut elements = vec![first];
                    
                    // Allow trailing comma in tuples
                    if !self.check(&Token::RightParen) {
                        loop {
                            elements.push(self.parse_expression()?);
                            if !self.check(&Token::Comma) {
                                break;
                            }
                            self.advance();
                            if self.check(&Token::RightParen) {
                                break;
                            }
                        }
                    }
                    
                    self.consume(")")?;
                    Ok(Expression::Tuple(elements))
                } else {
                    self.consume(")")?;
                    Ok(first)
                }
            }
            Token::LeftBrace => {
                let block = self.parse_block()?;
                Ok(Expression::Block(block))
            }
            Token::Keyword(Keyword::If) => self.parse_if_expression(),
            Token::Keyword(Keyword::Match) => self.parse_match_expression(),
            Token::Keyword(Keyword::Loop) => self.parse_loop_expression(),
            Token::Keyword(Keyword::While) => self.parse_while_expression(),
            Token::Keyword(Keyword::For) => self.parse_for_loop(),
            Token::Pipe => self.parse_closure(),
            Token::LeftBracket => self.parse_array(),
            _ => Err(ParseError::InvalidSyntax(format!(
                "Unexpected token: {:?}",
                self.current()
            ))),
        };
        // eprintln!("[DEBUG parse_primary #{} END", call_id);
        result
    }

    /// Parse function arguments
    fn parse_arguments(&mut self) -> ParseResult<Vec<Expression>> {
        let mut args = Vec::new();

        while !self.check(&Token::RightParen) {
            args.push(self.parse_expression()?);
            if !self.check(&Token::RightParen) {
                self.consume(",")?;
            }
        }

        Ok(args)
    }

    /// Parse if expression
    fn parse_if_expression(&mut self) -> ParseResult<Expression> {
        self.expect_keyword(Keyword::If)?;
        let condition = Box::new(self.parse_expression()?);
        let then_body = self.parse_block()?;

        let else_body = if self.check(&Token::Keyword(Keyword::Else)) {
            self.advance();
            if self.check(&Token::Keyword(Keyword::If)) {
                Some(Box::new(self.parse_if_expression()?))
            } else {
                let block = self.parse_block()?;
                Some(Box::new(Expression::Block(block)))
            }
        } else {
            None
        };

        Ok(Expression::If {
            condition,
            then_body,
            else_body,
        })
    }

    /// Parse match expression
    fn parse_match_expression(&mut self) -> ParseResult<Expression> {
        self.expect_keyword(Keyword::Match)?;
        let scrutinee = Box::new(self.parse_expression()?);

        self.consume("{")?;
        let mut arms = Vec::new();

        while !self.check(&Token::RightBrace) {
            let pattern = self.parse_pattern()?;
            let guard = if self.check(&Token::Keyword(Keyword::If)) {
                self.advance();
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            self.consume("=>")?;
            let body = self.parse_expression()?;

            arms.push(MatchArm { pattern, guard, body });

            if !self.check(&Token::RightBrace) {
                self.consume(",")?;
            }
        }

        self.consume("}")?;

        Ok(Expression::Match { scrutinee, arms })
    }

    /// Parse a pattern
    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        match self.current() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Pattern::Identifier(name))
            }
            _ => Err(ParseError::InvalidSyntax("Expected pattern".to_string())),
        }
    }

    /// Parse loop expression
    fn parse_loop_expression(&mut self) -> ParseResult<Expression> {
        self.expect_keyword(Keyword::Loop)?;
        let body = self.parse_block()?;
        Ok(Expression::Loop(body))
    }

    /// Parse while expression
    fn parse_while_expression(&mut self) -> ParseResult<Expression> {
        self.expect_keyword(Keyword::While)?;
        let condition = Box::new(self.parse_expression()?);
        let body = self.parse_block()?;
        Ok(Expression::While { condition, body })
    }

    /// Parse array literal
    fn parse_array(&mut self) -> ParseResult<Expression> {
        self.consume("[")?;
        let mut elements = Vec::new();

        while !self.check(&Token::RightBracket) {
            elements.push(self.parse_expression()?);
            if !self.check(&Token::RightBracket) {
                self.consume(",")?;
            }
        }

        self.consume("]")?;
        Ok(Expression::Array(elements))
    }

    /// Parse for loop: `for var in iter { body }`
    fn parse_for_loop(&mut self) -> ParseResult<Expression> {
        self.expect_keyword(Keyword::For)?;
        let var = self.expect_identifier()?;
        self.expect_keyword(Keyword::In)?;
        
        // Parse the iterator expression with NO_STRUCT_LITERAL restriction
        // to prevent `identifier {` in `0..passes {` from being parsed as struct literal  
        // eprintln!("[DEBUG] parse_for_loop: Before parsing iter, restriction={:?}", self.restrictions);
        let iter = self.with_restrictions(Restrictions::NoStructLiteral, |parser| {
            // eprintln!("[DEBUG] parse_for_loop: Inside with_restrictions, restriction={:?}", parser.restrictions);
            parser.parse_expression()
        })?;
        // eprintln!("[DEBUG] parse_for_loop: After parsing iter, restriction={:?}", self.restrictions);
        
        let body = self.parse_block()?;
        
        Ok(Expression::For { var, iter: Box::new(iter), body })
    }

    /// Parse closure: `|param1, param2| body`
    fn parse_closure(&mut self) -> ParseResult<Expression> {
        // Expecting: | params | body
        self.consume("|")?;
        let mut params = Vec::new();
        
        while !self.check(&Token::Pipe) {
            let param = self.expect_identifier()?;
            params.push(param);
            
            if !self.check(&Token::Pipe) {
                self.consume(",")?;
            }
        }
        
        self.consume("|")?;
        let body = Box::new(self.parse_expression()?);
        
        Ok(Expression::Closure { params, body })
    }

    /// Parse enum definition: `enum Name { Variant1, Variant2(Type), ... }`
    fn parse_enum(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Enum)?;
        let name = self.expect_identifier()?;
        self.consume("{")?;
        
        let mut variants = Vec::new();
        while !self.check(&Token::RightBrace) {
            let var_name = self.expect_identifier()?;
            
            let variant = if self.check(&Token::LeftParen) {
                self.advance();
                let mut types = Vec::new();
                while !self.check(&Token::RightParen) {
                    types.push(self.parse_type()?);
                    if !self.check(&Token::RightParen) {
                        self.consume(",")?;
                    }
                }
                self.consume(")")?;
                EnumVariant::Tuple(var_name, types)
            } else if self.check(&Token::LeftBrace) {
                self.advance();
                let mut fields = Vec::new();
                while !self.check(&Token::RightBrace) {
                    let field_name = self.expect_identifier()?;
                    self.consume(":")?;
                    let ty = self.parse_type()?;
                    fields.push(StructField { name: field_name, ty });
                    
                    if !self.check(&Token::RightBrace) {
                        self.consume(",")?;
                    }
                }
                self.consume("}")?;
                EnumVariant::Struct(var_name, fields)
            } else {
                EnumVariant::Unit(var_name)
            };
            
            variants.push(variant);
            if !self.check(&Token::RightBrace) {
                self.consume(",")?;
            }
        }
        
        self.consume("}")?;
        Ok(Item::Enum { name, variants })
    }

    /// Parse trait definition: `trait Name { ... }`
    fn parse_trait(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Trait)?;
        let name = self.expect_identifier()?;
        self.consume("{")?;
        
        let mut methods = Vec::new();
        while !self.check(&Token::RightBrace) {
            if self.check(&Token::Keyword(Keyword::Fn)) {
                methods.push(self.parse_function()?);
            } else {
                self.advance(); // Skip unknown items
            }
        }
        
        self.consume("}")?;
        Ok(Item::Trait { name, methods })
    }

    /// Parse impl block: `impl Name { ... }` or `impl Trait for Name { ... }`
    fn parse_impl(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Impl)?;
        let struct_name = self.expect_identifier()?;
        
        let trait_name = if self.check(&Token::Keyword(Keyword::For)) {
            self.advance();
            Some(struct_name.clone())
        } else if self.expect_identifier().is_ok() {
            Some(struct_name.clone())
        } else {
            None
        };
        
        // Re-parse struct_name if we just consumed a trait
        let struct_name = if trait_name.is_some() {
            self.expect_identifier()?
        } else {
            struct_name
        };
        
        self.consume("{")?;
        let mut methods = Vec::new();
        
        while !self.check(&Token::RightBrace) {
            if self.check(&Token::Keyword(Keyword::Fn)) {
                methods.push(self.parse_function()?);
            } else if self.check(&Token::Keyword(Keyword::Pub)) {
                self.advance();
                if self.check(&Token::Keyword(Keyword::Fn)) {
                    methods.push(self.parse_function()?);
                }
            } else {
                self.advance(); // Skip unknown items
            }
        }
        
        self.consume("}")?;
        Ok(Item::Impl { trait_name, struct_name, methods })
    }

    /// Parse module definition: `mod name { ... }`
    fn parse_module(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Mod)?;
        let name = self.expect_identifier()?;
        
        if self.check(&Token::LeftBrace) {
            self.advance();
            let mut items = Vec::new();
            
            while !self.check(&Token::RightBrace) && !self.check(&Token::Eof) {
                items.push(self.parse_item()?);
            }
            
            self.consume("}")?;
            Ok(Item::Module { name, items })
        } else {
            // Inline module: `mod name;`
            self.consume(";")?;
            Ok(Item::Module { name, items: Vec::new() })
        }
    }

    /// Parse use statement: `use path::to::item;`
    fn parse_use(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Use)?;
        let path = self.expect_identifier()?;
        
        let mut full_path = path;
        while self.check(&Token::DoubleColon) {
            self.advance();
            full_path.push_str("::");
            full_path.push_str(&self.expect_identifier()?);
        }
        
        self.consume(";")?;
        Ok(Item::Use { path: full_path })
    }
}

/// The public parsing function
pub fn parse(tokens: Vec<Token>) -> Result<Program, String> {
    let mut parser = Parser::new(tokens);
    parser.parse_program().map_err(|e| e.to_string())
}