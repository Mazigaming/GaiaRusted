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
    errors: Vec<ParseError>,
    error_recovery_enabled: bool,
}

impl Parser {
    /// Create a new parser from tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { 
            tokens, 
            position: 0, 
            restrictions: Restrictions::None,
            errors: Vec::new(),
            error_recovery_enabled: true,
        }
    }

    /// Get accumulated errors
    pub fn get_errors(&self) -> Vec<ParseError> {
        self.errors.clone()
    }

    /// Skip tokens until a synchronization point is found
    fn skip_to_sync_point(&mut self) {
        const SYNC_TOKENS: &[Token] = &[
            Token::Semicolon,
            Token::RightBrace,
            Token::Keyword(Keyword::Fn),
            Token::Keyword(Keyword::Struct),
            Token::Keyword(Keyword::Enum),
            Token::Keyword(Keyword::Trait),
            Token::Keyword(Keyword::Impl),
            Token::Keyword(Keyword::Let),
            Token::Keyword(Keyword::If),
            Token::Keyword(Keyword::For),
            Token::Keyword(Keyword::While),
            Token::Keyword(Keyword::Pub),
            Token::Keyword(Keyword::Mod),
            Token::Keyword(Keyword::Use),
        ];

        while !self.check(&Token::Eof) {
            let current = self.current();
            if SYNC_TOKENS.iter().any(|t| std::mem::discriminant(t) == std::mem::discriminant(current)) {
                break;
            }
            self.advance();
        }
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
    pub fn consume(&mut self, expected: &str) -> ParseResult<Token> {
        let token = self.current().clone();
        
        match self.current() {
            Token::Semicolon if expected == ";" => {
                self.advance();
                Ok(token)
            }
            Token::LeftBrace if expected == "{" => {
                self.advance();
                Ok(token)
            }
            Token::RightBrace if expected == "}" => {
                self.advance();
                Ok(token)
            }
            Token::LeftParen if expected == "(" => {
                self.advance();
                Ok(token)
            }
            Token::RightParen if expected == ")" => {
                self.advance();
                Ok(token)
            }
            Token::LeftBracket if expected == "[" => {
                self.advance();
                Ok(token)
            }
            Token::RightBracket if expected == "]" => {
                self.advance();
                Ok(token)
            }
            Token::Comma if expected == "," => {
                self.advance();
                Ok(token)
            }
            Token::Colon if expected == ":" => {
                self.advance();
                Ok(token)
            }
            Token::Arrow if expected == "->" => {
                self.advance();
                Ok(token)
            }
            Token::Greater if expected == ">" => {
                self.advance();
                Ok(token)
            }
            Token::Less if expected == "<" => {
                self.advance();
                Ok(token)
            }
            Token::Plus if expected == "+" => {
                self.advance();
                Ok(token)
            }
            Token::Minus if expected == "-" => {
                self.advance();
                Ok(token)
            }
            Token::Star if expected == "*" => {
                self.advance();
                Ok(token)
            }
            Token::Slash if expected == "/" => {
                self.advance();
                Ok(token)
            }
            Token::Ampersand if expected == "&" => {
                self.advance();
                Ok(token)
            }
            Token::Pipe if expected == "|" => {
                self.advance();
                Ok(token)
            }
            Token::Equal if expected == "=" => {
                self.advance();
                Ok(token)
            }
            Token::FatArrow if expected == "=>" => {
                self.advance();
                Ok(token)
            }
            _ => {
                let err = ParseError::UnexpectedToken {
                    expected: expected.to_string(),
                    found: format!("{:?}", self.current()),
                };

                if self.error_recovery_enabled {
                    self.errors.push(err.clone());
                    self.skip_to_sync_point();
                    Ok(token)
                } else {
                    Err(err)
                }
            }
        }
    }

    /// Expect a specific token type
    pub fn expect_keyword(&mut self, keyword: Keyword) -> ParseResult<()> {
        match self.current() {
            Token::Keyword(kw) if *kw == keyword => {
                self.advance();
                Ok(())
            }
            _ => {
                let err = ParseError::UnexpectedToken {
                    expected: format!("{:?}", keyword),
                    found: format!("{:?}", self.current()),
                };

                if self.error_recovery_enabled {
                    self.errors.push(err.clone());
                    Ok(())
                } else {
                    Err(err)
                }
            }
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

    /// Expect an identifier or keyword-like identifier (for field names, method names after dot)
    pub fn expect_field_name(&mut self) -> ParseResult<String> {
        match self.current() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            // Allow certain keywords as field names (after a dot)
            Token::Keyword(Keyword::Self_) => {
                self.advance();
                Ok("self".to_string())
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "field name".to_string(),
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
        // Skip attributes (#[...])
        while self.check(&Token::Hash) {
            self.advance(); // consume #
            if self.check(&Token::LeftBracket) {
                self.advance(); // consume [
                // Skip until we find the matching ]
                let mut bracket_depth = 1;
                while bracket_depth > 0 && self.current() != &Token::Eof {
                    match self.current() {
                        Token::LeftBracket => bracket_depth += 1,
                        Token::RightBracket => bracket_depth -= 1,
                        _ => {}
                    }
                    self.advance();
                }
            } else {
                // Malformed attribute, but continue parsing
                break;
            }
        }

        // Handle visibility modifiers (pub, etc.)
        let is_pub = if self.check(&Token::Keyword(Keyword::Pub)) {
            self.advance();
            true
        } else {
            false
        };

        // Handle extern functions (skip the ABI string)
        if self.check(&Token::Keyword(Keyword::Extern)) {
            self.advance();
            // Skip the ABI string (e.g., "C")
            if let Token::String(_) = self.current() {
                self.advance();
            }
        }

        match self.current() {
            Token::Keyword(Keyword::Fn) => self.parse_function(),
            Token::Keyword(Keyword::Struct) => self.parse_struct(),
            Token::Keyword(Keyword::Enum) => self.parse_enum(),
            Token::Keyword(Keyword::Trait) => self.parse_trait(),
            Token::Keyword(Keyword::Impl) => self.parse_impl(),
            Token::Keyword(Keyword::Mod) => self.parse_module(),
            Token::Keyword(Keyword::Use) => self.parse_use(is_pub),
            Token::Keyword(Keyword::Const) => self.parse_const_item(is_pub),
            Token::Keyword(Keyword::Static) => self.parse_static_item(is_pub),
            Token::Keyword(Keyword::MacroRules) => self.parse_macro_rules_item(),
            _ => Err(ParseError::InvalidSyntax(
                "Expected function, struct, enum, trait, impl, mod, use, const, static, or macro_rules definition".to_string(),
            )),
        }
    }

    /// Parse a function definition
    fn parse_function(&mut self) -> ParseResult<Item> {
        let abi = if self.check(&Token::Keyword(Keyword::Extern)) {
            self.advance();
            let abi_str = if let Token::String(s) = self.current() {
                let s = s.clone();
                self.advance();
                s
            } else {
                return Err(ParseError::InvalidSyntax("Expected ABI string after 'extern'".to_string()));
            };
            Some(abi_str)
        } else {
            None
        };

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

        // Parse where clause
        let where_clause = self.parse_where_clause()?;

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
            where_clause,
            abi,
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

    /// Parse a where clause: `where T: Trait1, U: Trait2`
    fn parse_where_clause(&mut self) -> ParseResult<Vec<WhereConstraint>> {
        let mut constraints = Vec::new();

        if !self.check(&Token::Keyword(Keyword::Where)) {
            return Ok(constraints);
        }

        self.advance();

        loop {
            let param_name = self.expect_identifier()?;
            self.consume(":")?;
            
            let mut bounds = Vec::new();
            
            loop {
                if let Token::Identifier(bound) = self.current() {
                    bounds.push(bound.clone());
                    self.advance();
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "trait bound".to_string(),
                        found: format!("{:?}", self.current()),
                    });
                }

                if self.check(&Token::Plus) {
                    self.advance();
                } else {
                    break;
                }
            }

            constraints.push(WhereConstraint { param_name, bounds });

            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(constraints)
    }

    /// Parse a type
    fn parse_type(&mut self) -> ParseResult<Type> {
        match self.current() {
            Token::Keyword(Keyword::Impl) => {
                // Parse impl Trait syntax: impl Trait, impl Trait1 + Trait2
                self.advance();
                
                let mut bounds = Vec::new();
                
                // Parse first trait bound
                if let Token::Identifier(trait_name) = self.current() {
                    bounds.push(trait_name.clone());
                    self.advance();
                } else {
                    return Err(ParseError::InvalidSyntax("Expected trait name after 'impl'".to_string()));
                }
                
                // Parse additional trait bounds
                while self.check(&Token::Plus) {
                    self.advance();
                    
                    if let Token::Identifier(trait_name) = self.current() {
                        bounds.push(trait_name.clone());
                        self.advance();
                    } else {
                        return Err(ParseError::InvalidSyntax("Expected trait name after '+'".to_string()));
                    }
                }
                
                Ok(Type::ImplTrait { bounds })
            }
            Token::Keyword(Keyword::Dyn) => {
                // Parse dyn Trait syntax: dyn Trait, dyn Trait + OtherTrait, dyn Trait + 'a
                self.advance();
                
                let mut bounds = Vec::new();
                let mut lifetime = None;
                
                // Parse first trait bound
                if let Token::Identifier(trait_name) = self.current() {
                    bounds.push(trait_name.clone());
                    self.advance();
                } else {
                    return Err(ParseError::InvalidSyntax("Expected trait name after 'dyn'".to_string()));
                }
                
                // Parse additional bounds and lifetime
                while self.check(&Token::Plus) {
                    self.advance();
                    
                    // Check for lifetime
                    if let Token::Lifetime(lt) = self.current() {
                        lifetime = Some(lt.clone());
                        self.advance();
                    } else if let Token::Identifier(trait_name) = self.current() {
                        // Another trait bound
                        bounds.push(trait_name.clone());
                        self.advance();
                    } else {
                        return Err(ParseError::InvalidSyntax("Expected trait name or lifetime after '+'".to_string()));
                    }
                }
                
                Ok(Type::TraitObject { bounds, lifetime })
            }
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
                
                // Check for sized array [T; N]
                let size = if self.check(&Token::Semicolon) {
                    self.advance();
                    // Parse the size expression - could be a literal or identifier
                    if let Token::Integer(n, _) = self.current() {
                        let size_val = *n;
                        self.advance();
                        Some(Box::new(Expression::Integer(size_val)))
                    } else if let Token::Identifier(name) = self.current() {
                        let size_name = name.clone();
                        self.advance();
                        Some(Box::new(Expression::Variable(size_name)))
                    } else {
                        return Err(ParseError::InvalidSyntax("Expected array size (integer or identifier)".to_string()));
                    }
                } else {
                    None
                };
                
                self.consume("]")?;
                Ok(Type::Array { element, size })
            }
            Token::Star => {
                self.advance();
                let mutable = if self.check(&Token::Keyword(Keyword::Mut)) {
                    self.advance();
                    true
                } else if self.check(&Token::Keyword(Keyword::Const)) {
                    self.advance();
                    false
                } else {
                    return Err(ParseError::InvalidSyntax("Raw pointer must be followed by 'const' or 'mut'".to_string()));
                };
                let inner = Box::new(self.parse_type()?);
                Ok(Type::Pointer { mutable, inner })
            }
            _ => Err(ParseError::InvalidSyntax("Expected type".to_string())),
        }
    }

    /// Parse a struct definition
    fn parse_struct(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Struct)?;
        let name = self.expect_identifier()?;

        let generics = self.parse_generics()?;

        let where_clause = self.parse_where_clause()?;

        let mut fields = Vec::new();

        if self.check(&Token::Semicolon) {
            self.consume(";")?;
        } else {
            self.consume("{")?;

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
        }

        Ok(Item::Struct { 
            name, 
            generics,
            fields,
            is_pub: false,
            attributes: Vec::new(),
            where_clause,
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
        
        // Check for range starting with .. (e.g., ..5, ..=10)
        if self.check(&Token::DotDot) {
            self.advance();
            if self.check(&Token::Equal) {
                self.advance();
                let end = Box::new(self.parse_range_end()?);
                return Ok(Expression::Range {
                    start: None,
                    end: Some(end),
                    inclusive: true,
                });
            } else if !self.is_expression_terminator() {
                let end = Box::new(self.parse_range_end()?);
                return Ok(Expression::Range {
                    start: None,
                    end: Some(end),
                    inclusive: false,
                });
            } else {
                // Just .. with no end (e.g., ..)
                return Ok(Expression::Range {
                    start: None,
                    end: None,
                    inclusive: false,
                });
            }
        }
        
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
            Token::Integer(n, _) => {
                // eprintln!("[DEBUG] parse_range_end: Found Integer({}), advancing...", n);
                self.advance();
                // eprintln!("[DEBUG] parse_range_end: After advance, current token={:?}", self.current());
                Ok(Expression::Integer(n))
            }
            Token::Float(f, _) => {
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

    /// Parse postfix: expr.field, expr[index], expr(args)
    fn parse_postfix(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current() {
                Token::LeftParen => {
                    // Handle function calls on Path expressions like `Point::new(5, 10)`
                    if let Expression::Path { segments, is_absolute } = expr {
                        self.advance();
                        let args = self.parse_arguments()?;
                        self.consume(")")?;
                        
                        // Convert path to function call
                        // For Type::method, use the full path as context
                        if segments.len() > 1 {
                            // Path-based call: Type::method(args)
                            // For now, we'll represent this as a special case using GenericCall
                            // with the path information encoded in the name
                            let full_name = segments.join("::");
                            expr = Expression::GenericCall {
                                name: full_name,
                                type_args: Vec::new(),
                                args,
                            };
                        } else {
                            // Simple function call
                            expr = Expression::FunctionCall { 
                                name: segments[0].clone(), 
                                args 
                            };
                        }
                    } else {
                        return Err(ParseError::InvalidSyntax(
                            "Unexpected '(' after non-callable expression".to_string(),
                        ));
                    }
                }
                Token::Dot => {
                    self.advance();
                    let field_or_method = match self.current() {
                        Token::Integer(n, _) => {
                            let n = *n;
                            self.advance();
                            n.to_string()
                        }
                        _ => self.expect_field_name()?
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
                Token::Question => {
                    self.advance();
                    expr = Expression::Try {
                        value: Box::new(expr),
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
            Token::Integer(n, _) => {
                // eprintln!("[DEBUG] parse_primary: Found Integer({}), advancing...", n);
                self.advance();
                // eprintln!("[DEBUG] parse_primary: After advance, current token={:?}", self.current());
                Ok(Expression::Integer(n))
            }
            Token::Float(f, _) => {
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

                // Check for macro call (println!, print!, vec!, etc.)
                if self.check(&Token::Bang) {
                    self.advance();
                    let macro_name = path.last().unwrap().clone();
                    if self.check(&Token::LeftParen) {
                        self.advance();
                        let args = self.parse_arguments()?;
                        self.consume(")")?;
                        Ok(Expression::FunctionCall { name: macro_name, args })
                    } else if self.check(&Token::LeftBracket) {
                        // Handle bracket-style macros like vec![1, 2, 3]
                        self.advance();
                        let args = self.parse_bracket_contents()?;
                        self.consume("]")?;
                        Ok(Expression::FunctionCall { name: macro_name, args })
                    } else {
                        return Err(ParseError::InvalidSyntax(
                            "Expected '(' or '[' after macro '!'".to_string(),
                        ));
                    }
                } else if self.check(&Token::LeftParen) {
                    // Handle function calls: simple `func()` or associated `Type::func()`
                    self.advance();
                    let args = self.parse_arguments()?;
                    self.consume(")")?;
                    // For now, join path with :: to create a qualified name
                    let func_name = path.join("::");
                    Ok(Expression::FunctionCall { name: func_name, args })
                } else if self.check(&Token::LeftBrace) && matches!(self.restrictions, Restrictions::None) {
                    // Struct literal or Enum struct literal
                    // Struct literal: Name { field: value, ... } (path.len() == 1)
                    // Enum struct literal: EnumName::VariantName { field: value, ... } (path.len() == 2)
                    self.advance();
                    let mut fields = Vec::new();
                    
                    while !self.check(&Token::RightBrace) {
                        let field_name = self.expect_identifier()?;
                        
                        // Support shorthand field syntax: `field` is equivalent to `field: field`
                        let field_value = if self.check(&Token::Colon) {
                            self.advance();
                            self.parse_expression()?
                        } else {
                            // Shorthand: field name only, expands to field: field
                            Expression::Variable(field_name.clone())
                        };
                        
                        fields.push((field_name, field_value));
                        
                        if !self.check(&Token::RightBrace) {
                            self.consume(",")?;
                        }
                    }
                    
                    self.consume("}")?;
                    
                    if path.len() == 1 {
                        Ok(Expression::StructLiteral {
                            struct_name: path[0].clone(),
                            fields,
                        })
                    } else if path.len() == 2 {
                        Ok(Expression::EnumStructLiteral {
                            enum_name: path[0].clone(),
                            variant_name: path[1].clone(),
                            fields,
                        })
                    } else {
                        Err(ParseError::InvalidSyntax(
                            "Invalid struct literal path".to_string(),
                        ))
                    }
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
            Token::Keyword(Keyword::Unsafe) => self.parse_unsafe_block(),
            Token::Keyword(Keyword::Self_) => {
                self.advance();
                Ok(Expression::Variable("self".to_string()))
            }
            Token::Pipe | Token::OrOr => self.parse_closure(),
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

    fn parse_bracket_contents(&mut self) -> ParseResult<Vec<Expression>> {
        let mut args = Vec::new();

        while !self.check(&Token::RightBracket) {
            args.push(self.parse_expression()?);
            if !self.check(&Token::RightBracket) {
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
            Token::LeftBracket => {
                self.advance();
                let mut patterns = Vec::new();
                let mut rest_pattern = None;

                while !self.check(&Token::RightBracket) {
                    if self.check(&Token::DotDot) {
                        self.advance();
                        let rest_ident = if matches!(self.current(), Token::Identifier(_)) && 
                                           !self.check(&Token::RightBracket) &&
                                           !self.check(&Token::Comma) {
                            Some(Box::new(Pattern::Identifier(self.expect_identifier()?)))
                        } else {
                            Some(Box::new(Pattern::Wildcard))
                        };
                        rest_pattern = rest_ident;
                    } else {
                        patterns.push(self.parse_pattern()?);
                    }

                    if !self.check(&Token::RightBracket) {
                        self.consume(",")?;
                    }
                }

                self.consume("]")?;
                Pattern::Slice { patterns, rest: rest_pattern }
            }
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
                            // Parse multiple patterns for tuple variants
                            let mut patterns = Vec::new();
                            patterns.push(self.parse_pattern()?);
                            while !self.check(&Token::RightParen) && self.check(&Token::Comma) {
                                self.advance(); // consume comma
                                if !self.check(&Token::RightParen) {
                                    patterns.push(self.parse_pattern()?);
                                }
                            }
                            if patterns.len() == 1 {
                                Some(Box::new(patterns.pop().unwrap()))
                            } else {
                                Some(Box::new(Pattern::Tuple(patterns)))
                            }
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
                        // Parse multiple patterns for tuple variants
                        let mut patterns = Vec::new();
                        patterns.push(self.parse_pattern()?);
                        while !self.check(&Token::RightParen) && self.check(&Token::Comma) {
                            self.advance(); // consume comma
                            if !self.check(&Token::RightParen) {
                                patterns.push(self.parse_pattern()?);
                            }
                        }
                        if patterns.len() == 1 {
                            Some(Box::new(patterns.pop().unwrap()))
                        } else {
                            Some(Box::new(Pattern::Tuple(patterns)))
                        }
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
            Token::Integer(val, _) => {
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
                Token::Integer(val, _) => {
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

    /// Parse unsafe block: `unsafe { ... }`
    fn parse_unsafe_block(&mut self) -> ParseResult<Expression> {
        self.expect_keyword(Keyword::Unsafe)?;
        let body = self.parse_block()?;
        Ok(Expression::UnsafeBlock(body))
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

    /// Parse closure: `|param1, param2| body` or `move |x| x + 1` or `|| body`
    fn parse_closure(&mut self) -> ParseResult<Expression> {
        // Check for `move` keyword
        let is_move = if self.check(&Token::Keyword(Keyword::Move)) {
            self.advance();
            true
        } else {
            false
        };

        let mut params = Vec::new();
        
        // Handle || (OrOr token) as empty parameter list
        if self.check(&Token::OrOr) {
            self.advance();
            // Empty parameter list, continue to body
        } else {
            // Expecting: | params | body
            self.consume("|")?;
            
            while !self.check(&Token::Pipe) {
                let param = self.expect_identifier()?;
                
                // Parse type annotation if present
                let param_type = if self.check(&Token::Colon) {
                    self.advance();
                    Some(self.parse_type()?)
                } else {
                    None
                };
                
                params.push((param, param_type));
                
                if !self.check(&Token::Pipe) {
                    self.consume(",")?;
                }
            }
            
            self.consume("|")?;
        }
        
        // Check for return type annotation: -> Type
        let return_type = if self.check(&Token::Arrow) {
            self.advance();
            Some(Box::new(self.parse_type()?))
        } else {
            None
        };
        
        let body = Box::new(self.parse_expression()?);
        
        Ok(Expression::Closure { params, return_type, body, is_move })
    }

    /// Parse enum definition: `enum Name { Variant1, Variant2(Type), ... }`
    fn parse_enum(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Enum)?;
        let name = self.expect_identifier()?;
        let generics = self.parse_generics()?;
        
        let where_clause = self.parse_where_clause()?;
        
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
            where_clause,
        })
    }

    /// Parse trait definition: `trait Name { ... }`
    fn parse_trait(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Trait)?;
        let name = self.expect_identifier()?;
        let generics = self.parse_generics()?;
        
        let where_clause = self.parse_where_clause()?;
        
        self.consume("{")?;
        
        let mut methods = Vec::new();
        while !self.check(&Token::RightBrace) {
            if self.check(&Token::Keyword(Keyword::Fn)) {
                let method = self.parse_trait_method()?;
                methods.push(method);
            } else if self.check(&Token::Keyword(Keyword::Type)) {
                self.advance();
                let assoc_type_name = self.expect_identifier()?;
                let mut ty = None;
                
                if self.check(&Token::Equal) {
                    self.advance();
                    ty = Some(self.parse_type()?);
                }
                
                self.consume(";")?;
                
                methods.push(Item::AssociatedType {
                    name: assoc_type_name,
                    bounds: Vec::new(),
                    ty,
                    attributes: Vec::new(),
                });
            } else {
                self.advance();
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
            where_clause,
        })
    }

    fn parse_trait_method(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Fn)?;
        let name = self.expect_identifier()?;
        let generics = self.parse_generics()?;
        let where_clause = self.parse_where_clause()?;
        
        self.consume("(")?;
        let params = self.parse_parameters()?;
        self.consume(")")?;
        
        let return_type = if self.check(&Token::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        
        let body = if self.check(&Token::LeftBrace) {
            self.parse_block()?
        } else {
            self.consume(";")?;
            Block {
                statements: Vec::new(),
                expression: None,
            }
        };
        
        Ok(Item::Function {
            name,
            generics,
            params,
            return_type,
            body,
            is_unsafe: false,
            is_async: false,
            is_pub: false,
            attributes: Vec::new(),
            where_clause,
            abi: None,
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
        
        let where_clause = self.parse_where_clause()?;
        
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
            where_clause,
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

    /// Parse use statement: `use path::to::item;` or `pub use path::to::item;`
    fn parse_use(&mut self, is_public: bool) -> ParseResult<Item> {
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
            is_public,
            attributes: Vec::new(),
        })
    }

    /// Parse a const item: const NAME: Type = value;
    fn parse_const_item(&mut self, is_public: bool) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Const)?;
        let name = self.expect_identifier()?;
        self.consume(":")?;
        let ty = self.parse_type()?;
        self.consume("=")?;
        let value = self.parse_expression()?;
        self.consume(";")?;
        
        Ok(Item::Const {
            name,
            ty,
            value,
            is_pub: is_public,
            attributes: Vec::new(),
        })
    }

    /// Parse a static item: static NAME: Type = value; or static mut NAME: Type = value;
    fn parse_static_item(&mut self, is_public: bool) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Static)?;
        let is_mutable = if self.check(&Token::Keyword(Keyword::Mut)) {
            self.advance();
            true
        } else {
            false
        };
        let name = self.expect_identifier()?;
        self.consume(":")?;
        let ty = self.parse_type()?;
        self.consume("=")?;
        let value = self.parse_expression()?;
        self.consume(";")?;
        
        Ok(Item::Static {
            name,
            ty,
            value,
            is_mutable,
            is_pub: is_public,
            attributes: Vec::new(),
        })
    }

    /// Parse macro_rules! definition
    pub fn parse_macro_rules_item(&mut self) -> ParseResult<Item> {
        let (name, rules) = self.parse_macro_rules()?;
        let ast_rules = rules.iter().map(|rule| {
            ast::MacroRule {
                pattern: format!("{:?}", rule.pattern),
                body: format!("{:?}", rule.body),
            }
        }).collect();
        Ok(Item::MacroDefinition {
            name,
            rules: ast_rules,
            attributes: Vec::new(),
        })
    }
}

use std::path::Path;
use std::fs;

fn resolve_file_modules_recursive(
    items: &mut Vec<Item>,
    base_dir: &Path,
) -> Result<(), String> {
    for item in items.iter_mut() {
        if let Item::Module { name, items: ref mut module_items, is_inline, .. } = item {
            if !*is_inline {
                let name_str = name.as_str();
                let rs_file = base_dir.join(format!("{}.rs", name_str));
                let mod_rs_file = base_dir.join(name_str).join("mod.rs");

                let (file_path, is_rs_file) = if rs_file.exists() {
                    (rs_file, true)
                } else if mod_rs_file.exists() {
                    (mod_rs_file, false)
                } else {
                    return Err(format!("Module '{}' not found", name_str));
                };

                let module_source = fs::read_to_string(&file_path)
                    .map_err(|e| format!("Failed to read module file '{}': {}", file_path.display(), e))?;

                let tokens = crate::lexer::lex(&module_source)
                    .map_err(|e| format!("Lexer error in module '{}': {}", name_str, e))?;

                let mut module_parser = Parser::new(tokens);
                let parsed_items = module_parser.parse_program()
                    .map_err(|e| format!("Parser error in module '{}': {}", name_str, e))?;

                let mut new_items = parsed_items;
                let module_dir = if is_rs_file {
                    base_dir.to_path_buf()
                } else {
                    base_dir.join(name)
                };

                resolve_file_modules_recursive(&mut new_items, &module_dir)?;
                *module_items = new_items;
            } else {
                resolve_file_modules_recursive(module_items, base_dir)?;
            }
        }
    }
    Ok(())
}

/// Resolve file-based modules in the AST
pub fn resolve_file_modules(
    mut program: Program,
    base_dir: Option<&str>,
) -> Result<Program, String> {
    let dir = Path::new(base_dir.unwrap_or("."));
    resolve_file_modules_recursive(&mut program, dir)?;
    Ok(program)
}

/// The public parsing function
pub fn parse(tokens: Vec<Token>) -> Result<Program, String> {
    let mut parser = Parser::new(tokens);
    parser.parse_program().map_err(|e| e.to_string())
}
