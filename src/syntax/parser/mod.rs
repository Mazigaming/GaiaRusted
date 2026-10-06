//! A hand-written recursive-descent parser.
//!
//! The grammar is split by what is being parsed:
//!
//! * [`item`] — functions, structs, enums, traits, impls, modules, `use`
//! * [`ty`] — types, generics and trait bounds
//! * [`expr`] — expressions, statements and blocks (precedence climbing)
//! * [`pat`] — patterns
//! * [`macros`] — `macro_rules!` definitions and their expansion
//!
//! This file holds the token cursor those pieces share.

mod expr;
mod item;
mod macros;
mod pat;
mod ty;

use super::ast::{Ident, Item};
use super::diagnostic::{Diagnostic, Result};
use super::lexer::tokenize;
use super::span::{FileId, SourceMap, Span};
use super::token::{Keyword, Token, TokenKind};
use macros::MacroDef;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Parse the crate rooted at `root_file`, following `mod name;` declarations
/// into further files. Every file read is registered in `sources`.
pub fn parse_crate(sources: &mut SourceMap, root_file: &Path) -> Result<Vec<Item>> {
    let module_dir = root_file.parent().unwrap_or(Path::new("")).to_path_buf();
    parse_file(sources, root_file, module_dir, HashMap::new())
}

/// Parse source text that does not come from a file on disk (the built-in
/// prelude, tests). `mod name;` declarations cannot be resolved here.
pub fn parse_source(sources: &mut SourceMap, name: &str, text: &str) -> Result<Vec<Item>> {
    let file = sources.add_file(name, text.to_string());
    Parser::new(sources, file, PathBuf::new(), HashMap::new())?.parse_items_until_eof()
}

/// Parse one file. `macros` are the `macro_rules!` macros defined before
/// the `mod` declaration that leads here, which the file can use.
fn parse_file(
    sources: &mut SourceMap,
    path: &Path,
    module_dir: PathBuf,
    macros: HashMap<String, Rc<MacroDef>>,
) -> Result<Vec<Item>> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Diagnostic::global(format!("cannot read `{}`: {e}", path.display())))?;
    let file = sources.add_file(path, text);
    Parser::new(sources, file, module_dir, macros)?.parse_items_until_eof()
}

struct Parser<'a> {
    sources: &'a mut SourceMap,
    tokens: Vec<Token>,
    pos: usize,
    /// Directory in which `mod name;` looks for `name.rs` / `name/mod.rs`.
    module_dir: PathBuf,
    /// Set while parsing the condition of `if` / `while` / `match` / `for`,
    /// where `name {` starts the body rather than a struct literal.
    no_struct_literal: bool,
    /// The `macro_rules!` macros defined so far.
    macros: HashMap<String, Rc<MacroDef>>,
    /// Off while measuring how many tokens a macro fragment takes: macro
    /// invocations inside are then skipped, to be expanded later.
    expand_macros: bool,
    /// How many macro invocations have been expanded.
    expansions: usize,
}

impl<'a> Parser<'a> {
    fn new(
        sources: &'a mut SourceMap,
        file: FileId,
        module_dir: PathBuf,
        macros: HashMap<String, Rc<MacroDef>>,
    ) -> Result<Parser<'a>> {
        let tokens = tokenize(file, &sources.file(file).text)?;
        Ok(Parser { sources, tokens, pos: 0, module_dir, no_struct_literal: false, macros, expand_macros: true, expansions: 0 })
    }

    // -- cursor -------------------------------------------------------------

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    /// The token `n` positions after the current one (`Eof` past the end).
    fn peek_nth(&self, n: usize) -> &TokenKind {
        let last = self.tokens.len() - 1;
        &self.tokens[(self.pos + n).min(last)].kind
    }

    fn span(&self) -> Span {
        self.tokens[self.pos].span
    }

    /// Span of the token consumed most recently.
    fn prev_span(&self) -> Span {
        self.tokens[self.pos.saturating_sub(1)].span
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        token
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        *self.peek() == TokenKind::Keyword(keyword)
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        let found = self.at(kind);
        if found {
            self.bump();
        }
        found
    }

    fn eat_keyword(&mut self, keyword: Keyword) -> bool {
        self.eat(&TokenKind::Keyword(keyword))
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<Span> {
        if self.at(kind) {
            Ok(self.bump().span)
        } else {
            Err(self.unexpected(&kind.to_string()))
        }
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> Result<Span> {
        self.expect(&TokenKind::Keyword(keyword))
    }

    fn expect_ident(&mut self) -> Result<Ident> {
        match self.peek() {
            TokenKind::Ident(name) => {
                let name = name.clone();
                Ok(Ident { name, span: self.bump().span })
            }
            _ => Err(self.unexpected("an identifier")),
        }
    }

    /// "expected X, found Y" at the current token.
    fn unexpected(&self, expected: &str) -> Diagnostic {
        Diagnostic::new(self.span(), format!("expected {expected}, found {}", self.peek()))
    }

    fn unsupported(&self, what: &str) -> Diagnostic {
        Diagnostic::new(self.span(), format!("{what} is not supported yet"))
    }

    // -- compound tokens that sometimes need splitting ------------------------

    /// Replace the current token by `rest`, the part left over after its
    /// first character has been consumed (`>>` → `>`).
    fn split_current(&mut self, rest: TokenKind) {
        let token = &mut self.tokens[self.pos];
        token.kind = rest;
        token.span.lo += 1;
    }

    /// Consume one `>`, even when the lexer glued it to what follows
    /// (`Vec<Vec<i32>>`, `Option<T>= ...`).
    fn expect_closing_angle(&mut self) -> Result<()> {
        match self.peek() {
            TokenKind::Gt => {
                self.bump();
            }
            TokenKind::Shr => self.split_current(TokenKind::Gt),
            TokenKind::Ge => self.split_current(TokenKind::Eq),
            TokenKind::ShrEq => self.split_current(TokenKind::Ge),
            _ => return Err(self.unexpected("`>`")),
        }
        Ok(())
    }

    /// Consume one `&`, treating `&&` as two of them (`&&x`, `&&str`).
    fn eat_ampersand(&mut self) -> bool {
        match self.peek() {
            TokenKind::And => {
                self.bump();
                true
            }
            TokenKind::AndAnd => {
                self.split_current(TokenKind::And);
                true
            }
            _ => false,
        }
    }

    // -- helpers --------------------------------------------------------------

    /// Parse `item (sep item)* sep?` up to (and including) `close`.
    fn comma_separated<T>(
        &mut self,
        close: &TokenKind,
        mut parse_one: impl FnMut(&mut Self) -> Result<T>,
    ) -> Result<Vec<T>> {
        let mut items = Vec::new();
        while !self.at(close) {
            items.push(parse_one(self)?);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(close)?;
        Ok(items)
    }

    /// Run `parse` with struct literals allowed or forbidden, then restore.
    fn with_struct_literals<T>(
        &mut self,
        allowed: bool,
        parse: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let saved = std::mem::replace(&mut self.no_struct_literal, !allowed);
        let result = parse(self);
        self.no_struct_literal = saved;
        result
    }

    /// Parse the file behind a `mod name;` declaration.
    fn load_module_file(&mut self, name: &Ident) -> Result<Vec<Item>> {
        let flat = self.module_dir.join(format!("{}.rs", name.name));
        let nested = self.module_dir.join(&name.name).join("mod.rs");
        let path = if flat.exists() {
            flat
        } else if nested.exists() {
            nested
        } else {
            return Err(Diagnostic::new(
                name.span,
                format!("file not found for module `{}`", name.name),
            )
            .with_note(format!("looked for `{}` and `{}`", flat.display(), nested.display())));
        };
        parse_file(self.sources, &path, self.module_dir.join(&name.name), self.macros.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::ast::*;

    fn parse(source: &str) -> Vec<Item> {
        let mut sources = SourceMap::new();
        match parse_source(&mut sources, "test.rs", source) {
            Ok(items) => items,
            Err(error) => panic!("{}", error.render(&sources)),
        }
    }

    fn body_of(source: &str) -> Block {
        let items = parse(&format!("fn test() {{ {source} }}"));
        match items.into_iter().next().map(|item| item.kind) {
            Some(ItemKind::Fn(function)) => function.body.unwrap(),
            other => panic!("expected a function, got {other:?}"),
        }
    }

    fn tail_of(source: &str) -> ExprKind {
        body_of(source).expr.expect("block has a tail expression").kind
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        let ExprKind::Binary(BinOp::Add, _, rhs) = tail_of("1 + 2 * 3") else { panic!() };
        assert!(matches!(rhs.kind, ExprKind::Binary(BinOp::Mul, ..)));
    }

    #[test]
    fn cast_binds_tighter_than_arithmetic() {
        let ExprKind::Binary(BinOp::Add, lhs, _) = tail_of("a as i64 + b") else { panic!() };
        assert!(matches!(lhs.kind, ExprKind::Cast(..)));
    }

    #[test]
    fn generic_arguments_survive_and_nested_angles_close() {
        let items = parse("fn f(m: HashMap<String, Vec<i64>>) {}");
        let ItemKind::Fn(function) = &items[0].kind else { panic!() };
        let TypeKind::Path(path) = &function.params[0].ty.kind else { panic!() };
        assert_eq!(path.last().args.len(), 2);
        let TypeKind::Path(inner) = &path.last().args[1].kind else { panic!() };
        assert_eq!(inner.last().ident.name, "Vec");
        assert_eq!(inner.last().args.len(), 1);
    }

    #[test]
    fn condition_does_not_swallow_the_body_as_a_struct_literal() {
        let ExprKind::If { cond, .. } = tail_of("if x { 1 } else { 2 }") else { panic!() };
        assert!(matches!(cond.kind, ExprKind::Path(_)));
        assert!(matches!(tail_of("Point { x: 1, y }"), ExprKind::Struct { .. }));
    }

    #[test]
    fn if_let_keeps_its_pattern() {
        let ExprKind::If { cond, .. } = tail_of("if let Some(v) = opt { v } else { 0 }") else {
            panic!()
        };
        let ExprKind::Let(pattern, _) = cond.kind else { panic!() };
        assert!(matches!(pattern.kind, PatternKind::TupleStruct(..)));
    }

    #[test]
    fn block_like_statement_ends_at_its_closing_brace() {
        let block = body_of("if a { } *p = 1;");
        assert_eq!(block.stmts.len(), 2);
        assert!(matches!(block.stmts[1].kind, StmtKind::Expr(Expr { kind: ExprKind::Assign(..), .. })));
    }

    #[test]
    fn match_arms_parse_ranges_guards_and_bindings() {
        let ExprKind::Match { arms, .. } =
            tail_of("match n { 0 => a, 1..=9 => b, x if x < 0 => c, k @ 10..=99 => d, _ => e }")
        else {
            panic!()
        };
        assert_eq!(arms.len(), 5);
        assert!(matches!(arms[1].pat.kind, PatternKind::Range { inclusive: true, .. }));
        assert!(arms[2].guard.is_some());
        assert!(matches!(arms[3].pat.kind, PatternKind::Binding { sub: Some(_), .. }));
    }

    #[test]
    fn block_bodied_arm_is_not_called_by_the_next_tuple_pattern() {
        let ExprKind::Match { arms, .. } = tail_of("match p { (0, 0) => { a } (x, y) => b }")
        else {
            panic!()
        };
        assert_eq!(arms.len(), 2);
    }

    #[test]
    fn closures_methods_and_turbofish() {
        let ExprKind::MethodCall { method, turbofish, receiver, .. } =
            tail_of("v.iter().map(|x| x * 2).collect::<Vec<i64>>()")
        else {
            panic!()
        };
        assert_eq!(method.name, "collect");
        assert_eq!(turbofish.len(), 1);
        let ExprKind::MethodCall { args, .. } = receiver.kind else { panic!() };
        assert!(matches!(args[0].kind, ExprKind::Closure { .. }));
    }

    #[test]
    fn nested_tuple_index_is_split_from_a_float_token() {
        let ExprKind::TupleField(outer, 1) = tail_of("pair.0.1") else { panic!() };
        assert!(matches!(outer.kind, ExprKind::TupleField(_, 0)));
    }

    #[test]
    fn trait_bounds_with_fn_sugar() {
        let items = parse("fn apply<F: Fn(i64) -> i64 + Clone>(f: F, x: i64) -> i64 { f(x) }");
        let ItemKind::Fn(function) = &items[0].kind else { panic!() };
        let bounds = &function.generics.params[0].bounds;
        assert_eq!(bounds.len(), 2);
        assert!(bounds[0].fn_sugar.is_some());
    }

    #[test]
    fn items_of_every_kind() {
        let items = parse(
            r#"
            use std::collections::{HashMap, HashSet as Set};
            const MAX: i64 = 100;
            #[derive(Clone, Debug)]
            pub struct Point<T> { x: T, y: T }
            struct Pair(i64, i64);
            struct Marker;
            enum Shape { Circle(i64), Rect { w: i64, h: i64 }, Empty }
            trait Area: Clone { type Unit; fn area(&self) -> i64; fn twice(&self) -> i64 { 2 * self.area() } }
            impl<T: Clone> Point<T> { fn new(x: T, y: T) -> Self { Point { x, y } } }
            impl Area for Shape { type Unit = i64; fn area(&self) -> i64 { 0 } }
            extern "C" { fn strlen(s: *const u8) -> usize; fn printf(fmt: *const u8, ...) -> i32; }
            mod inner { pub fn f() {} }
            "#,
        );
        assert_eq!(items.len(), 11);
        let ItemKind::Struct(point) = &items[2].kind else { panic!() };
        assert_eq!(items[2].attrs[0].args, ["Clone", "Debug"]);
        assert_eq!(point.generics.params.len(), 1);
        let ItemKind::ExternBlock(functions) = &items[9].kind else { panic!() };
        assert!(functions[1].is_variadic);
    }

    #[test]
    fn errors_point_at_the_offending_token() {
        let mut sources = SourceMap::new();
        let error = parse_source(&mut sources, "test.rs", "fn main() {\n    let x = ;\n}").unwrap_err();
        let rendered = error.render(&sources);
        assert!(rendered.contains("test.rs:2:13"), "{rendered}");
        assert!(rendered.contains("expected an expression"), "{rendered}");
    }
}
