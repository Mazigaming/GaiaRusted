//! `macro_rules!`: macros by example.
//!
//! A macro is a list of rules, each a pattern over tokens and the tokens
//! to produce when it matches:
//!
//! ```text
//! macro_rules! maximum {
//!     ($x:expr) => { $x };
//!     ($x:expr, $($rest:expr),+) => { { let a = $x; let b = maximum!($($rest),+); if a > b { a } else { b } } };
//! }
//! ```
//!
//! Macros are expanded while parsing. A `macro_rules!` definition is
//! visible to the code after it, as in Rust. At an invocation the rules are
//! tried in order. The expansion of the first that matches replaces the
//! invocation in the token stream, and parsing carries on through it. An
//! expansion that invokes a macro (itself, say) is expanded in turn, when
//! the parser gets there.
//!
//! A fragment such as `$x:expr` matches whatever the parser would read as
//! an expression at that point, found by running a parser over the
//! remaining tokens. Substituted back, an expression is put in parentheses,
//! so `square!(2 + 3)` with `$x * $x` is `(2 + 3) * (2 + 3)`.

use super::Parser;
use crate::syntax::ast::{Expr, ExprKind};
use crate::syntax::diagnostic::{bail, Diagnostic, Result};
use crate::syntax::span::Span;
use crate::syntax::token::{Keyword, Token, TokenKind};
use std::collections::HashMap;
use std::rc::Rc;

/// Expanding more macros than this in one file is taken to be a macro that
/// never stops invoking itself.
const EXPANSION_LIMIT: usize = 100_000;

pub(super) struct MacroDef {
    name: String,
    rules: Vec<Rule>,
}

struct Rule {
    pattern: Vec<Matcher>,
    body: Vec<Transcribe>,
}

/// One element of a rule's pattern.
enum Matcher {
    /// A token that must be there as it is.
    Token(TokenKind),
    /// `( .. )`, `[ .. ]` or `{ .. }`, by its opening delimiter: a
    /// delimited group, matched inside.
    Group(TokenKind, Vec<Matcher>),
    /// `$name:kind`
    Fragment(String, Fragment),
    /// `$( .. ) sep op`
    Repeat(Vec<Matcher>, Option<TokenKind>, Repetition),
}

#[derive(Clone, Copy, PartialEq)]
enum Fragment {
    Expr,
    Ident,
    Ty,
    TokenTree,
    Literal,
    Pat,
    /// A pattern without alternatives at the top, so `|` can follow it.
    PatParam,
    Block,
    Stmt,
    Path,
    Lifetime,
    Vis,
    Item,
}

#[derive(Clone, Copy, PartialEq)]
enum Repetition {
    /// `*`
    Any,
    /// `+`
    AtLeastOne,
    /// `?`
    AtMostOne,
}

/// One element of a rule's body.
enum Transcribe {
    Token(Token),
    /// `$name`
    Var(String, Span),
    /// `$( .. ) sep op`
    Repeat(Vec<Transcribe>, Option<Token>),
}

/// What a fragment of the pattern matched.
#[derive(Clone)]
enum Binding {
    One(Vec<Token>, Fragment),
    /// One binding per repetition.
    Many(Vec<Binding>),
}

type Bindings = HashMap<String, Binding>;

fn fragment_kind(name: &str) -> Option<Fragment> {
    Some(match name {
        "expr" => Fragment::Expr,
        "ident" => Fragment::Ident,
        "ty" => Fragment::Ty,
        "tt" => Fragment::TokenTree,
        "literal" => Fragment::Literal,
        "pat" => Fragment::Pat,
        "pat_param" => Fragment::PatParam,
        "block" => Fragment::Block,
        "stmt" => Fragment::Stmt,
        "path" => Fragment::Path,
        "lifetime" => Fragment::Lifetime,
        "vis" => Fragment::Vis,
        "item" => Fragment::Item,
        _ => return None,
    })
}

fn closing(open: &TokenKind) -> Option<TokenKind> {
    match open {
        TokenKind::LParen => Some(TokenKind::RParen),
        TokenKind::LBracket => Some(TokenKind::RBracket),
        TokenKind::LBrace => Some(TokenKind::RBrace),
        _ => None,
    }
}

fn is_closing(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace)
}

/// The index just past the group that opens at `tokens[start]`.
fn group_end(tokens: &[Token], start: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if closing(&token.kind).is_some() {
            depth += 1;
        } else if is_closing(&token.kind) {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index + 1);
            }
        }
    }
    None
}

/// The name of a `$name`: an identifier or a keyword (`$type`, `$self`).
fn variable_name(kind: &TokenKind) -> Option<String> {
    match kind {
        TokenKind::Ident(name) => Some(name.clone()),
        TokenKind::Keyword(keyword) => Some(TokenKind::Keyword(*keyword).to_string()),
        _ => None,
    }
}

// -- reading a definition ---------------------------------------------------

impl Parser<'_> {
    /// Is a `macro_rules! name { .. }` definition next?
    pub(super) fn at_macro_rules(&self) -> bool {
        matches!(self.peek(), TokenKind::Ident(name) if name == "macro_rules") && *self.peek_nth(1) == TokenKind::Not
    }

    /// Read a `macro_rules!` definition and make it visible to what follows.
    pub(super) fn parse_macro_rules(&mut self) -> Result<()> {
        self.bump();
        self.expect(&TokenKind::Not)?;
        let name = self.expect_ident()?;
        let open = self.bump();
        let Some(close) = closing(&open.kind) else {
            bail!(open.span, "expected `{{`, `(` or `[` after `macro_rules! {}`", name.name);
        };
        let mut rules = Vec::new();
        while !self.at(&close) {
            let pattern_tokens = self.delimited_tokens()?;
            self.expect(&TokenKind::FatArrow)?;
            let body_tokens = self.delimited_tokens()?;
            rules.push(Rule { pattern: parse_matchers(&pattern_tokens)?, body: parse_transcribers(&body_tokens)? });
            if !self.eat(&TokenKind::Semi) {
                break;
            }
        }
        self.expect(&close)?;
        // `macro_rules! name ( .. );` ends with a semicolon.
        if close != TokenKind::RBrace {
            self.eat(&TokenKind::Semi);
        }
        if rules.is_empty() {
            bail!(name.span, "`macro_rules! {}` has no rules", name.name);
        }
        self.macros.insert(name.name.clone(), Rc::new(MacroDef { name: name.name, rules }));
        Ok(())
    }

    /// The tokens inside the delimited group that starts at the cursor;
    /// the cursor moves past it.
    fn delimited_tokens(&mut self) -> Result<Vec<Token>> {
        let start = self.pos;
        if closing(self.peek()).is_none() {
            return Err(self.unexpected("`(`, `[` or `{`"));
        }
        let Some(end) = group_end(&self.tokens, start) else {
            bail!(self.span(), "this delimiter is never closed");
        };
        self.pos = end;
        Ok(self.tokens[start + 1..end - 1].to_vec())
    }
}

fn parse_matchers(tokens: &[Token]) -> Result<Vec<Matcher>> {
    let mut matchers = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        if closing(&token.kind).is_some() {
            let end = group_end(tokens, index).ok_or_else(|| Diagnostic::new(token.span, "unclosed delimiter in macro pattern"))?;
            matchers.push(Matcher::Group(token.kind.clone(), parse_matchers(&tokens[index + 1..end - 1])?));
            index = end;
            continue;
        }
        if token.kind != TokenKind::Dollar {
            matchers.push(Matcher::Token(token.kind.clone()));
            index += 1;
            continue;
        }
        let Some(next) = tokens.get(index + 1) else { bail!(token.span, "expected a name after `$`") };
        if next.kind == TokenKind::LParen {
            let end = group_end(tokens, index + 1).ok_or_else(|| Diagnostic::new(next.span, "unclosed `$(`"))?;
            let inner = parse_matchers(&tokens[index + 2..end - 1])?;
            let (separator, repetition, after) = repetition_suffix(tokens, end)?;
            matchers.push(Matcher::Repeat(inner, separator.map(|token| token.kind), repetition));
            index = after;
            continue;
        }
        let name = variable_name(&next.kind).ok_or_else(|| Diagnostic::new(next.span, "expected a name after `$`"))?;
        let kind = match (tokens.get(index + 2), tokens.get(index + 3)) {
            (Some(Token { kind: TokenKind::Colon, .. }), Some(kind)) => kind,
            _ => bail!(next.span, "`${name}` in a macro pattern needs a fragment kind, such as `${name}:expr`"),
        };
        let fragment = variable_name(&kind.kind)
            .and_then(|name| fragment_kind(&name))
            .ok_or_else(|| Diagnostic::new(kind.span, format!("unknown fragment kind `{}`", kind.kind)))?;
        matchers.push(Matcher::Fragment(name, fragment));
        index += 4;
    }
    Ok(matchers)
}

/// After `$( .. )`: an optional separator and `*`, `+` or `?`. Returns
/// them and the index after them.
fn repetition_suffix(tokens: &[Token], at: usize) -> Result<(Option<Token>, Repetition, usize)> {
    let operator = |token: Option<&Token>| match token.map(|token| &token.kind) {
        Some(TokenKind::Star) => Some(Repetition::Any),
        Some(TokenKind::Plus) => Some(Repetition::AtLeastOne),
        Some(TokenKind::Question) => Some(Repetition::AtMostOne),
        _ => None,
    };
    if let Some(repetition) = operator(tokens.get(at)) {
        return Ok((None, repetition, at + 1));
    }
    match (tokens.get(at), operator(tokens.get(at + 1))) {
        (Some(separator), Some(repetition)) => Ok((Some(separator.clone()), repetition, at + 2)),
        (Some(token), None) => bail!(token.span, "expected `*`, `+` or `?` after a repetition"),
        (None, _) => bail!(tokens[at - 1].span, "expected `*`, `+` or `?` after a repetition"),
    }
}

fn parse_transcribers(tokens: &[Token]) -> Result<Vec<Transcribe>> {
    let mut items = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        if token.kind != TokenKind::Dollar {
            items.push(Transcribe::Token(token.clone()));
            index += 1;
            continue;
        }
        let Some(next) = tokens.get(index + 1) else { bail!(token.span, "expected a name after `$`") };
        if next.kind == TokenKind::LParen {
            let end = group_end(tokens, index + 1).ok_or_else(|| Diagnostic::new(next.span, "unclosed `$(`"))?;
            let inner = parse_transcribers(&tokens[index + 2..end - 1])?;
            let (separator, _, after) = repetition_suffix(tokens, end)?;
            items.push(Transcribe::Repeat(inner, separator));
            index = after;
            continue;
        }
        let name = variable_name(&next.kind).ok_or_else(|| Diagnostic::new(next.span, "expected a name after `$`"))?;
        items.push(Transcribe::Var(name, next.span));
        index += 2;
    }
    Ok(items)
}

// -- expanding an invocation -----------------------------------------------

/// Where an invocation stands, which decides how its expansion is read.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Position {
    /// Among the items of a module, trait or impl: the expansion is items.
    Items,
    /// Among the statements of a block: the expansion is statements, items
    /// or the block's final expression.
    Statements,
    /// Inside an expression: the expansion is one expression.
    Expression,
}

impl Parser<'_> {
    /// The macro invoked at the cursor (`name!(..)`), if it is one defined
    /// with `macro_rules!` and macros are being expanded.
    pub(super) fn at_macro_invocation(&self) -> Option<Rc<MacroDef>> {
        if !self.expand_macros || *self.peek_nth(1) != TokenKind::Not || closing(self.peek_nth(2)).is_none() {
            return None;
        }
        let TokenKind::Ident(name) = self.peek() else { return None };
        self.macros.get(name).cloned()
    }

    /// Replace the invocation `name!(..)` at the cursor by its expansion.
    /// In item or statement position a `;` after it goes with it.
    pub(super) fn expand_invocation(&mut self, definition: &MacroDef, position: Position) -> Result<()> {
        let start = self.pos;
        let call_span = self.span();
        let group_start = start + 2;
        let Some(end) = group_end(&self.tokens, group_start) else {
            bail!(call_span, "the arguments of `{}!` are never closed", definition.name);
        };
        let input = self.tokens[group_start + 1..end - 1].to_vec();

        self.expansions += 1;
        if self.expansions > EXPANSION_LIMIT {
            bail!(call_span, "recursion limit reached while expanding `{}!`", definition.name);
        }
        let mut expansion = None;
        for rule in &definition.rules {
            let mut bindings = Bindings::new();
            if self.match_sequence(&rule.pattern, &input, 0, &mut bindings) == Some(input.len()) {
                expansion = Some(transcribe(&rule.body, &bindings, &[], definition)?);
                break;
            }
        }
        let Some(mut tokens) = expansion else {
            bail!(call_span, "no rule of `{}!` matches these arguments", definition.name);
        };

        let mut replaced_end = end;
        match position {
            Position::Expression => {
                let close = self.tokens[end - 1].span;
                tokens.insert(0, Token { kind: TokenKind::LParen, span: call_span });
                tokens.push(Token { kind: TokenKind::RParen, span: close });
            }
            // `m!(..);` among items: the semicolon only ends the invocation.
            Position::Items if self.tokens.get(end).is_some_and(|token| token.kind == TokenKind::Semi) => {
                replaced_end += 1;
            }
            // Among statements it stays, ending a statement the expansion
            // may leave open (`square!(3);`).
            Position::Items | Position::Statements => {}
        }
        self.tokens.splice(start..replaced_end, tokens);
        self.pos = start;
        Ok(())
    }

    /// Among items (`position` is [`Position::Items`]) or statements,
    /// expand an invocation of a user-defined macro, or read a
    /// `macro_rules!` definition. Returns whether it did either.
    pub(super) fn expand_in_item_position(&mut self, position: Position) -> Result<bool> {
        if self.at_macro_rules() {
            self.parse_macro_rules()?;
            return Ok(true);
        }
        let Some(definition) = self.at_macro_invocation() else { return Ok(false) };
        let mut position = position;
        if position == Position::Statements {
            // `m!(..).len()` in statement position is an expression after all.
            let group_end = group_end(&self.tokens, self.pos + 2).unwrap_or(self.tokens.len());
            let continues = self.tokens.get(group_end).is_some_and(|token| {
                !matches!(token.kind, TokenKind::Semi | TokenKind::RBrace | TokenKind::Eof)
                    && !self.starts_item_or_statement(group_end)
            });
            if continues && self.tokens[self.pos + 2].kind != TokenKind::LBrace {
                position = Position::Expression;
            }
        }
        self.expand_invocation(&definition, position)?;
        Ok(true)
    }

    /// A `stmt` fragment: an item, a `let` without its semicolon, or an
    /// expression.
    fn parse_statement_fragment(&mut self) -> Result<()> {
        if self.at_item_start() {
            self.parse_item()?;
        } else if self.eat_keyword(Keyword::Let) {
            self.parse_pattern()?;
            if self.eat(&TokenKind::Colon) {
                self.parse_type()?;
            }
            if self.eat(&TokenKind::Eq) {
                self.parse_expr()?;
            }
        } else {
            self.parse_expr()?;
        }
        Ok(())
    }

    /// Could a new item or statement start at `tokens[index]`?
    fn starts_item_or_statement(&self, index: usize) -> bool {
        matches!(
            self.tokens[index].kind,
            TokenKind::Keyword(
                Keyword::Let
                    | Keyword::Fn
                    | Keyword::Struct
                    | Keyword::Enum
                    | Keyword::Impl
                    | Keyword::Trait
                    | Keyword::Pub
                    | Keyword::Use
                    | Keyword::Mod
                    | Keyword::Const
                    | Keyword::Static
                    | Keyword::Type
            ) | TokenKind::Pound
                | TokenKind::Ident(_)
        )
    }

    /// While measuring a fragment, a macro invocation is skipped whole: it
    /// is expanded later, when the fragment is parsed for real.
    pub(super) fn skip_invocation(&mut self, name_span: Span) -> Result<ExprKind> {
        let Some(end) = group_end(&self.tokens, self.pos) else {
            bail!(name_span, "the arguments of this macro are never closed");
        };
        self.pos = end;
        Ok(ExprKind::Tuple(Vec::<Expr>::new()))
    }

    /// Match `matchers` against `tokens` from `start`. Returns where the
    /// match ends, or `None` if it fails.
    fn match_sequence(&mut self, matchers: &[Matcher], tokens: &[Token], start: usize, bindings: &mut Bindings) -> Option<usize> {
        let mut pos = start;
        for matcher in matchers {
            pos = match matcher {
                Matcher::Token(kind) => (tokens.get(pos)?.kind == *kind).then_some(pos + 1)?,
                Matcher::Group(open, inner) => {
                    if tokens.get(pos)?.kind != *open {
                        return None;
                    }
                    let end = group_end(tokens, pos)?;
                    let inner_tokens = &tokens[pos + 1..end - 1];
                    if self.match_sequence(inner, inner_tokens, 0, bindings)? != inner_tokens.len() {
                        return None;
                    }
                    end
                }
                Matcher::Fragment(name, fragment) => {
                    let length = self.fragment_length(*fragment, &tokens[pos..])?;
                    bindings.insert(name.clone(), Binding::One(tokens[pos..pos + length].to_vec(), *fragment));
                    pos + length
                }
                Matcher::Repeat(inner, separator, repetition) => {
                    let mut rounds: Vec<Bindings> = Vec::new();
                    let mut at = pos;
                    loop {
                        let mut next = at;
                        if !rounds.is_empty() {
                            if let Some(separator) = separator {
                                if tokens.get(next).map(|token| &token.kind) != Some(separator) {
                                    break;
                                }
                                next += 1;
                            }
                        }
                        let mut round = Bindings::new();
                        match self.match_sequence(inner, tokens, next, &mut round) {
                            Some(end) if end > next || (end == next && inner.is_empty()) => {
                                rounds.push(round);
                                at = end;
                            }
                            _ => break,
                        }
                        if *repetition == Repetition::AtMostOne || at == next {
                            break;
                        }
                    }
                    if *repetition == Repetition::AtLeastOne && rounds.is_empty() {
                        return None;
                    }
                    for name in names_in(inner) {
                        let each = rounds.iter_mut().filter_map(|round| round.remove(&name)).collect();
                        bindings.insert(name, Binding::Many(each));
                    }
                    at
                }
            };
        }
        Some(pos)
    }

    /// How many of `tokens` a fragment of the given kind takes, or `None`
    /// if it does not start here.
    fn fragment_length(&mut self, fragment: Fragment, tokens: &[Token]) -> Option<usize> {
        let first = tokens.first()?;
        match fragment {
            Fragment::Ident => matches!(first.kind, TokenKind::Ident(_)).then_some(1),
            Fragment::Lifetime => matches!(first.kind, TokenKind::Lifetime(_)).then_some(1),
            Fragment::TokenTree => match closing(&first.kind) {
                Some(_) => group_end(tokens, 0),
                None => (!is_closing(&first.kind)).then_some(1),
            },
            Fragment::Literal => {
                let literal = |kind: &TokenKind| {
                    matches!(
                        kind,
                        TokenKind::Int(..)
                            | TokenKind::Float(..)
                            | TokenKind::Str(_)
                            | TokenKind::Char(_)
                            | TokenKind::Byte(_)
                            | TokenKind::ByteStr(_)
                            | TokenKind::Keyword(Keyword::True | Keyword::False)
                    )
                };
                match &first.kind {
                    TokenKind::Minus => tokens.get(1).filter(|token| literal(&token.kind)).map(|_| 2),
                    kind => literal(kind).then_some(1),
                }
            }
            Fragment::Vis => match first.kind {
                TokenKind::Keyword(Keyword::Pub) => match tokens.get(1).map(|token| &token.kind) {
                    Some(TokenKind::LParen) => group_end(tokens, 1),
                    _ => Some(1),
                },
                _ => Some(0),
            },
            _ => self.measure(tokens, fragment),
        }
    }

    /// Parse a fragment from `tokens` with a parser of its own and report
    /// how far it got.
    fn measure(&mut self, tokens: &[Token], fragment: Fragment) -> Option<usize> {
        let mut tokens = tokens.to_vec();
        let end = tokens.last().map_or(Span::default(), |token| token.span);
        tokens.push(Token { kind: TokenKind::Eof, span: end });
        let mut parser = Parser {
            sources: &mut *self.sources,
            tokens,
            pos: 0,
            module_dir: self.module_dir.clone(),
            no_struct_literal: false,
            macros: HashMap::new(),
            expand_macros: false,
            expansions: 0,
        };
        let parsed = match fragment {
            Fragment::Expr => parser.parse_expr().is_ok(),
            Fragment::Ty => parser.parse_type().is_ok(),
            Fragment::Pat => parser.parse_pattern().is_ok(),
            Fragment::PatParam => parser.parse_pattern_no_alternatives().is_ok(),
            Fragment::Block => parser.parse_block().is_ok(),
            Fragment::Path => parser.parse_type().is_ok_and(|ty| matches!(ty.kind, crate::syntax::ast::TypeKind::Path(_))),
            Fragment::Item => parser.parse_item().is_ok(),
            Fragment::Stmt => parser.parse_statement_fragment().is_ok(),
            Fragment::Ident | Fragment::Lifetime | Fragment::TokenTree | Fragment::Literal | Fragment::Vis => {
                unreachable!("single-token fragments are matched directly")
            }
        };
        (parsed && parser.pos > 0).then_some(parser.pos)
    }
}

/// The variables a pattern binds.
fn names_in(matchers: &[Matcher]) -> Vec<String> {
    let mut names = Vec::new();
    for matcher in matchers {
        match matcher {
            Matcher::Fragment(name, _) => names.push(name.clone()),
            Matcher::Group(_, inner) | Matcher::Repeat(inner, ..) => names.extend(names_in(inner)),
            Matcher::Token(_) => {}
        }
    }
    names
}

/// The variables a body mentions.
fn vars_in(items: &[Transcribe]) -> Vec<String> {
    let mut names = Vec::new();
    for item in items {
        match item {
            Transcribe::Var(name, _) => names.push(name.clone()),
            Transcribe::Repeat(inner, _) => names.extend(vars_in(inner)),
            Transcribe::Token(_) => {}
        }
    }
    names
}

/// What `name` stands for in the repetition `indices` selects.
fn lookup<'b>(bindings: &'b Bindings, name: &str, indices: &[usize]) -> Option<&'b Binding> {
    let mut binding = bindings.get(name)?;
    for &index in indices {
        match binding {
            Binding::Many(each) => binding = each.get(index)?,
            Binding::One(..) => break,
        }
    }
    Some(binding)
}

fn transcribe(items: &[Transcribe], bindings: &Bindings, indices: &[usize], definition: &MacroDef) -> Result<Vec<Token>> {
    let mut out = Vec::new();
    for item in items {
        match item {
            Transcribe::Token(token) => out.push(token.clone()),
            Transcribe::Var(name, span) => match lookup(bindings, name, indices) {
                Some(Binding::One(tokens, fragment)) => {
                    // An expression keeps its grouping wherever it lands.
                    let wrap = *fragment == Fragment::Expr && tokens.len() > 1;
                    if wrap {
                        out.push(Token { kind: TokenKind::LParen, span: *span });
                    }
                    out.extend(tokens.iter().cloned());
                    if wrap {
                        out.push(Token { kind: TokenKind::RParen, span: *span });
                    }
                }
                Some(Binding::Many(_)) => {
                    bail!(*span, "`${name}` is repeated in `{}!`: use it inside `$( .. )*`", definition.name)
                }
                // Not a variable of this rule: `$` and the name as written.
                None => {
                    out.push(Token { kind: TokenKind::Dollar, span: *span });
                    out.push(Token { kind: TokenKind::Ident(name.clone()), span: *span });
                }
            },
            Transcribe::Repeat(inner, separator) => {
                let mut count = None;
                for name in vars_in(inner) {
                    if let Some(Binding::Many(each)) = lookup(bindings, &name, indices) {
                        match count {
                            None => count = Some(each.len()),
                            Some(n) if n != each.len() => {
                                bail!(definition_span(inner), "variables repeated together in `{}!` match different numbers of times", definition.name)
                            }
                            Some(_) => {}
                        }
                    }
                }
                let Some(count) = count else {
                    bail!(definition_span(inner), "a repetition in `{}!` repeats no variable", definition.name);
                };
                for round in 0..count {
                    if round > 0 {
                        out.extend(separator.iter().cloned());
                    }
                    let mut deeper = indices.to_vec();
                    deeper.push(round);
                    out.extend(transcribe(inner, bindings, &deeper, definition)?);
                }
            }
        }
    }
    Ok(out)
}

/// Somewhere in a body to point a diagnostic at.
fn definition_span(items: &[Transcribe]) -> Span {
    items
        .iter()
        .find_map(|item| match item {
            Transcribe::Token(token) => Some(token.span),
            Transcribe::Var(_, span) => Some(*span),
            Transcribe::Repeat(inner, _) => Some(definition_span(inner)),
        })
        .unwrap_or_default()
}
