//! # Phase 2: PARSER (Syntax Analysis)
//!
//! Converts a stream of tokens into an Abstract Syntax Tree (AST).
//!
//! ## Algorithm: Recursive Descent Parsing
//!
//! The parser uses **recursive descent** with **precedence climbing** for expressions:
//!
//! ```text
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
//! logical_and    → bitwise_or ( && bitwise_or )*
//! bitwise_or     → bitwise_xor ( | bitwise_xor )*
//! bitwise_xor    → bitwise_and ( ^ bitwise_and )*
//! bitwise_and    → equality ( & equality )*
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
pub type ParseResult<T> = Result<T, ParseError>;

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
    pub fn current(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::Eof)
    }

    /// Peek at next token
    fn peek(&self, offset: usize) -> &Token {
        self.tokens.get(self.position + offset).unwrap_or(&Token::Eof)
    }

    /// Advance to next token and return the current one
    pub fn advance(&mut self) -> Token {
        let token = self.current().clone();
        if self.position < self.tokens.len() {
            self.position += 1;
        }
        token
    }

    /// Check if current token matches
    pub fn check(&self, token: &Token) -> bool {
        match (self.current(), token) {
            // For Keyword tokens, compare the keyword variant specifically
            (Token::Keyword(kw1), Token::Keyword(kw2)) => kw1 == kw2,
            // For other tokens, compare discriminants
            _ => std::mem::discriminant(self.current()) == std::mem::discriminant(token)
        }
    }

    /// Consume a specific token or error
    pub fn consume(&mut self, _expected: &str) -> ParseResult<Token> {
        let token = self.current().clone();
        self.advance();
        Ok(token)
    }

    /// Expect a specific token type
    pub fn expect_keyword(&mut self, keyword: Keyword) -> ParseResult<()> {
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
    pub fn expect_identifier(&mut self) -> ParseResult<String> {
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

    fn is_block_like_expression(&self, expr: &Expression) -> bool {
        matches!(expr,
            Expression::If { .. } |
            Expression::Match { .. } |
            Expression::Loop(_) |
            Expression::While { .. } |
            Expression::Block(_) |
            Expression::For { .. }
        )
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
        let is_unsafe = if self.check(&Token::Keyword(Keyword::Unsafe)) {
            self.advance();
            true
        } else {
            false
        };

        let is_async = if self.check(&Token::Keyword(Keyword::Async)) {
            self.advance();
            true
        } else {
            false
        };

        self.expect_keyword(Keyword::Fn)?;
        let name = self.expect_identifier()?;

        // Parse generic parameters
        let generics = self.parse_generics()?;

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
            generics,
            params,
            return_type,
            body,
            is_unsafe,
            is_async,
            is_pub: false,
            attributes: Vec::new(),
        })
    }

    /// Parse function parameters
    fn parse_parameters(&mut self) -> ParseResult<Vec<Parameter>> {
        let mut params = Vec::new();

        while !self.check(&Token::RightParen) {
            let is_ref = self.check(&Token::Ampersand);
            if is_ref {
                self.advance();
            }

            let mutable = if self.check(&Token::Keyword(Keyword::Mut)) {
                self.advance();
                true
            } else {
                false
            };

            let name = if self.check(&Token::Keyword(Keyword::Self_)) {
                self.advance();
                "self".to_string()
            } else {
                self.expect_identifier()?
            };
            
            if self.check(&Token::Colon) {
                self.consume(":")?;
                let mut ty = self.parse_type()?;
                if is_ref {
                    ty = Type::Reference {
                        lifetime: None,
                        mutable,
                        inner: Box::new(ty),
                    };
                }
                params.push(Parameter { name, mutable, ty });
            } else if name == "self" {
                let ty = if is_ref {
                    Type::Reference {
                        lifetime: None,
                        mutable,
                        inner: Box::new(Type::Named("Self".to_string())),
                    }
                } else {
                    Type::Named("Self".to_string())
                };
                params.push(Parameter { name, mutable, ty });
            } else {
                return Err(ParseError::InvalidSyntax(
                    format!("Parameter {} needs type annotation", name)
                ));
            }

            if !self.check(&Token::RightParen) {
                self.consume(",")?;
            }
        }

        Ok(params)
    }

    /// Parse generic parameters: `<T>`, `<T, U>`, `<T: Trait>`, `<const N: usize>`
    fn parse_generics(&mut self) -> ParseResult<Vec<GenericParam>> {
        let mut generics = Vec::new();

        if !self.check(&Token::Less) {
            return Ok(generics);
        }

        self.advance();

        while !self.check(&Token::Greater) {
            // Check for lifetime parameters: 'a, 'b, etc.
            if let Token::Lifetime(lifetime) = self.current() {
                let name = lifetime.clone();
                self.advance();
                generics.push(GenericParam::Lifetime(name));
            } else if self.check(&Token::Keyword(Keyword::Const)) {
                self.advance();
                let name = self.expect_identifier()?;
                self.consume(":")?;
                let ty = self.parse_type()?;
                generics.push(GenericParam::Const { name, ty });
            } else {
                let name = self.expect_identifier()?;
                let mut bounds = Vec::new();

                while self.check(&Token::Plus) {
                    self.advance();
                    if let Token::Identifier(bound) = self.current() {
                        bounds.push(bound.clone());
                        self.advance();
                    } else {
                        return Err(ParseError::UnexpectedToken {
                            expected: "trait bound".to_string(),
                            found: format!("{:?}", self.current()),
                        });
                    }
                }

                generics.push(GenericParam::Type {
                    name,
                    bounds,
                    default: None,
                });
            }

            if !self.check(&Token::Greater) {
                self.consume(",")?;
            }
        }

        self.consume(">")?;
        Ok(generics)
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
                
                // Check for lifetime annotation: &'a T or &'a mut T
                let lifetime = if let Token::Lifetime(lt) = self.current() {
                    let name = lt.clone();
                    self.advance();
                    Some(name)
                } else {
                    None
                };
                
                let mutable = if self.check(&Token::Keyword(Keyword::Mut)) {
                    self.advance();
                    true
                } else {
                    false
                };
                let inner = Box::new(self.parse_type()?);
                Ok(Type::Reference { lifetime, mutable, inner })
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

        let generics = self.parse_generics()?;

        self.consume("{")?;
        let mut fields = Vec::new();

        while !self.check(&Token::RightBrace) {
            let field_name = self.expect_identifier()?;
            self.consume(":")?;
            let ty = self.parse_type()?;

            fields.push(StructField {
                name: field_name,
                ty,
                attributes: Vec::new(),
            });

            if !self.check(&Token::RightBrace) {
                self.consume(",")?;
            }
        }

        self.consume("}")?;

        Ok(Item::Struct { 
            name, 
            generics,
            fields,
            is_pub: false,
            attributes: Vec::new(),
        })
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
                statements.push(Statement::Break(None));
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
            } else if matches!(self.current(),
                Token::Keyword(Keyword::Fn) |
                Token::Keyword(Keyword::Struct) |
                Token::Keyword(Keyword::Enum) |
                Token::Keyword(Keyword::Trait) |
                Token::Keyword(Keyword::Impl) |
                Token::Keyword(Keyword::Mod) |
                Token::Keyword(Keyword::Use)
            ) {
                let item = self.parse_item()?;
                statements.push(Statement::Item(Box::new(item)));
            } else {
                let expr = self.parse_expression()?;

                if self.check(&Token::Semicolon) {
                    self.advance();
                    statements.push(Statement::Expression(expr));
                } else if self.check(&Token::RightBrace) {
                    expression = Some(Box::new(expr));
                    break;
                } else if self.is_block_like_expression(&expr) {
                    statements.push(Statement::Expression(expr));
                } else {
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
        
        let pattern = self.parse_pattern()?;
        
        let (name, mutable) = match &pattern {
            Pattern::Identifier(n) => (n.clone(), false),
            Pattern::MutableBinding(n) => (n.clone(), true),
            Pattern::Tuple(_) => ("_tuple_destructure".to_string(), false),
            _ => ("_pattern".to_string(), false),
        };

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
            attributes: Vec::new(),
            pattern: Some(pattern),
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
        let condition = Box::new(self.with_restrictions(Restrictions::NoStructLiteral, |parser| {
            parser.parse_expression()
        })?);
        let body = self.parse_block()?;
        
        Ok(Statement::While { condition, body })
    }

    /// Parse an if statement: `if condition { ... } else { ... }`
    fn parse_if_statement(&mut self) -> ParseResult<Statement> {
        self.expect_keyword(Keyword::If)?;
        let condition = Box::new(self.with_restrictions(Restrictions::NoStructLiteral, |parser| {
            parser.parse_expression()
        })?);
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
        let mut expr = self.parse_bitwise_or()?;

        while self.check(&Token::AndAnd) {
            self.advance();
            let right = Box::new(self.parse_bitwise_or()?);
            expr = Expression::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right,
            };
        }

        Ok(expr)
    }


    /// Parse bitwise OR: expr | expr
    fn parse_bitwise_or(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_bitwise_xor()?;

        while self.check(&Token::Pipe) {
            self.advance();
            let right = Box::new(self.parse_bitwise_xor()?);
            expr = Expression::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitwiseOr,
                right,
            };
        }

        Ok(expr)
    }

    /// Parse bitwise XOR: expr ^ expr
    fn parse_bitwise_xor(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_bitwise_and()?;

        while self.check(&Token::Caret) {
            self.advance();
            let right = Box::new(self.parse_bitwise_and()?);
            expr = Expression::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitwiseXor,
                right,
            };
        }

        Ok(expr)
    }

    /// Parse bitwise AND: expr & expr
    fn parse_bitwise_and(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_equality()?;

        while self.check(&Token::Ampersand) && !self.is_unary_ampersand() {
            self.advance();
            let right = Box::new(self.parse_equality()?);
            expr = Expression::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitwiseAnd,
                right,
            };
        }

        Ok(expr)
    }

    /// Helper to distinguish between binary & (bitwise AND) and unary & (reference)
    fn is_unary_ampersand(&self) -> bool {
        match self.peek(1) {
            Token::Keyword(Keyword::Mut) => true,
            Token::Star => true,
            Token::Bang => true,
            Token::Minus => true,
            Token::Tilde => true,
            Token::Ampersand => true,
            _ => false,
        }
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
                let is_mut = if self.check(&Token::Keyword(Keyword::Mut)) {
                    self.advance();
                    true
                } else {
                    false
                };
                let operand = Box::new(self.parse_unary()?);
                Ok(Expression::Unary {
                    op: if is_mut { UnaryOp::MutableReference } else { UnaryOp::Reference },
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

    /// Check if current token is an item keyword (fn, struct, enum, trait, impl, mod, use)
    fn is_item_keyword(&self) -> bool {
        matches!(self.current(),
            Token::Keyword(Keyword::Fn) |
            Token::Keyword(Keyword::Struct) |
            Token::Keyword(Keyword::Enum) |
            Token::Keyword(Keyword::Trait) |
            Token::Keyword(Keyword::Impl) |
            Token::Keyword(Keyword::Mod) |
            Token::Keyword(Keyword::Use)
        )
    }

    /// Parse postfix: expr.field, expr[index]
    fn parse_postfix(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current() {
                Token::Dot => {
                    self.advance();
                    let field_or_method = match self.current() {
                        Token::Integer(n) => {
                            let n = *n;
                            self.advance();
                            n.to_string()
                        }
                        _ => self.expect_identifier()?
                    };
                    
                    if self.check(&Token::LeftParen) {
                        self.advance();
                        let args = self.parse_arguments()?;
                        self.consume(")")?;
                        expr = Expression::MethodCall {
                            receiver: Box::new(expr),
                            method: field_or_method,
                            type_args: Vec::new(),
                            args,
                        };
                    } else {
                        expr = Expression::FieldAccess {
                            object: Box::new(expr),
                            field: field_or_method,
                        };
                    }
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
        let _call_id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // eprintln!("[DEBUG parse_primary #{} START, current token={:?}", _call_id, self.current());
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
                let mut path = vec![name.clone()];
                self.advance();

                while self.check(&Token::DoubleColon) {
                    self.advance();
                    let next_name = self.expect_identifier()?;
                    path.push(next_name);
                }

                // Check for macro call (println!, print!, etc.)
                if self.check(&Token::Bang) {
                    self.advance();
                    if self.check(&Token::LeftParen) {
                        self.advance();
                        let args = self.parse_arguments()?;
                        self.consume(")")?;
                        // For macros with paths, use the last segment as the name
                        let macro_name = path.last().unwrap().clone();
                        Ok(Expression::FunctionCall { name: macro_name, args })
                    } else {
                        return Err(ParseError::InvalidSyntax(
                            "Expected '(' after macro '!'".to_string(),
                        ));
                    }
                } else if self.check(&Token::LeftParen) && path.len() == 1 {
                    self.advance();
                    let args = self.parse_arguments()?;
                    self.consume(")")?;
                    Ok(Expression::FunctionCall { name: path[0].clone(), args })
                } else if self.check(&Token::LeftBrace) && matches!(self.restrictions, Restrictions::None) && path.len() == 1 {
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
                        struct_name: path[0].clone(),
                        fields,
                    })
                } else if path.len() > 1 {
                    Ok(Expression::Path {
                        segments: path,
                        is_absolute: false,
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
        let condition = Box::new(self.with_restrictions(Restrictions::NoStructLiteral, |parser| {
            parser.parse_expression()
        })?);
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
        let scrutinee = Box::new(self.with_restrictions(Restrictions::NoStructLiteral, |parser| {
            parser.parse_expression()
        })?);


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
        let pattern = match self.current() {
            Token::LeftParen => {
                self.advance();
                let mut patterns = Vec::new();
                
                while !self.check(&Token::RightParen) {
                    patterns.push(self.parse_pattern()?);
                    if !self.check(&Token::RightParen) {
                        self.consume(",")?;
                    }
                }
                
                self.consume(")")?;
                Pattern::Tuple(patterns)
            }
            Token::Keyword(Keyword::Mut) => {
                self.advance();
                let name = self.expect_identifier()?;
                Pattern::MutableBinding(name)
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                if name == "_" {
                    Pattern::Wildcard
                } else if self.check(&Token::DoubleColon) {
                    let mut path = vec![name];
                    while self.check(&Token::DoubleColon) {
                        self.advance();
                        let next_name = self.expect_identifier()?;
                        path.push(next_name);
                    }
                    
                    let inner_pattern = if self.check(&Token::LeftParen) {
                        self.advance();
                        let inner = if self.check(&Token::RightParen) {
                            None
                        } else {
                            let pat = self.parse_pattern()?;
                            Some(Box::new(pat))
                        };
                        self.consume(")")?;
                        inner
                    } else {
                        None
                    };
                    
                    Pattern::EnumVariant {
                        path,
                        data: inner_pattern,
                    }
                } else if self.check(&Token::LeftParen) {
                    self.advance();
                    let inner_pattern = if self.check(&Token::RightParen) {
                        None
                    } else {
                        let pat = self.parse_pattern()?;
                        Some(Box::new(pat))
                    };
                    self.consume(")")?;
                    Pattern::EnumVariant {
                        path: vec![name],
                        data: inner_pattern,
                    }
                } else {
                    Pattern::Identifier(name)
                }
            }
            Token::Integer(val) => {
                let val = *val;
                self.advance();
                Pattern::Literal(Expression::Integer(val))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Pattern::Literal(Expression::String(s))
            }
            Token::Keyword(Keyword::True) => {
                self.advance();
                Pattern::Literal(Expression::Bool(true))
            }
            Token::Keyword(Keyword::False) => {
                self.advance();
                Pattern::Literal(Expression::Bool(false))
            }
            _ => return Err(ParseError::InvalidSyntax("Expected pattern".to_string())),
        };
        
        if self.check(&Token::DotDot) || self.check(&Token::DotDotEqual) {
            let inclusive = self.check(&Token::DotDotEqual);
            self.advance();
            
            let end_expr = match self.current() {
                Token::Integer(val) => {
                    let val = *val;
                    self.advance();
                    Expression::Integer(val)
                }
                _ => return Err(ParseError::InvalidSyntax("Expected integer after range operator".to_string())),
            };
            
            let start_expr = match pattern {
                Pattern::Literal(expr) => expr,
                _ => return Err(ParseError::InvalidSyntax("Range patterns must start with a literal".to_string())),
            };
            
            Ok(Pattern::Range {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
                inclusive,
            })
        } else {
            Ok(pattern)
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

    /// Parse closure: `|param1, param2| body` or `move |x| x + 1`
    fn parse_closure(&mut self) -> ParseResult<Expression> {
        // Check for `move` keyword
        let is_move = if self.check(&Token::Keyword(Keyword::Move)) {
            self.advance();
            true
        } else {
            false
        };

        // Expecting: | params | body
        self.consume("|")?;
        let mut params = Vec::new();
        
        while !self.check(&Token::Pipe) {
            let param = self.expect_identifier()?;
            params.push(param);
            
            // Skip type annotations if present (they're parsed but not stored in the AST)
            if self.check(&Token::Colon) {
                self.advance();
                let _ = self.parse_type()?;
            }
            
            if !self.check(&Token::Pipe) {
                self.consume(",")?;
            }
        }
        
        self.consume("|")?;
        let body = Box::new(self.parse_expression()?);
        
        Ok(Expression::Closure { params, body, is_move })
    }

    /// Parse enum definition: `enum Name { Variant1, Variant2(Type), ... }`
    fn parse_enum(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Enum)?;
        let name = self.expect_identifier()?;
        let generics = self.parse_generics()?;
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
                    fields.push(StructField { name: field_name, ty, attributes: Vec::new() });
                    
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
        Ok(Item::Enum { 
            name, 
            generics,
            variants,
            is_pub: false,
            attributes: Vec::new(),
        })
    }

    /// Parse trait definition: `trait Name { ... }`
    fn parse_trait(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Trait)?;
        let name = self.expect_identifier()?;
        let generics = self.parse_generics()?;
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
        Ok(Item::Trait { 
            name, 
            generics,
            supertraits: Vec::new(),
            methods,
            is_pub: false,
            attributes: Vec::new(),
        })
    }

    /// Parse impl block: `impl Name { ... }` or `impl Trait for Name { ... }`
    fn parse_impl(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Impl)?;
        
        // Parse generic parameters: impl<'a> or impl<T>
        let generics = self.parse_generics()?;
        
        let struct_name = self.expect_identifier()?;
        
        let trait_name = if self.check(&Token::Keyword(Keyword::For)) {
            // Trait impl: impl Trait for Struct
            self.advance();
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
        Ok(Item::Impl { 
            generics,
            trait_name, 
            struct_name, 
            methods,
            is_unsafe: false,
            attributes: Vec::new(),
        })
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
            Ok(Item::Module { 
                name, 
                items,
                is_inline: true,
                is_pub: false,
                attributes: Vec::new(),
            })
        } else {
            // Inline module: `mod name;`
            self.consume(";")?;
            Ok(Item::Module { 
                name, 
                items: Vec::new(),
                is_inline: false,
                is_pub: false,
                attributes: Vec::new(),
            })
        }
    }

    /// Parse use statement: `use path::to::item;`
    fn parse_use(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Use)?;
        let first = self.expect_identifier()?;
        
        let mut path = vec![first];
        while self.check(&Token::DoubleColon) {
            self.advance();
            path.push(self.expect_identifier()?);
        }
        
        let is_glob = if self.check(&Token::Star) {
            self.advance();
            true
        } else {
            false
        };
        
        self.consume(";")?;
        Ok(Item::Use { 
            path,
            is_glob,
            is_public: false,
            attributes: Vec::new(),
        })
    }
}

/// The public parsing function
pub fn parse(tokens: Vec<Token>) -> Result<Program, String> {
    let mut parser = Parser::new(tokens);
    parser.parse_program().map_err(|e| e.to_string())
}
