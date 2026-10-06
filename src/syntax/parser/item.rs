//! Items: the declarations a module is made of.

use super::macros::Position;
use super::Parser;
use crate::syntax::ast::*;
use crate::syntax::diagnostic::{bail, Result};
use crate::syntax::span::Span;
use crate::syntax::token::{Keyword, TokenKind};

impl Parser<'_> {
    pub(super) fn parse_items_until_eof(mut self) -> Result<Vec<Item>> {
        self.parse_items_until(&TokenKind::Eof)
    }

    fn parse_items_until(&mut self, close: &TokenKind) -> Result<Vec<Item>> {
        let mut items = Vec::new();
        while !self.at(close) {
            if self.at(&TokenKind::Eof) {
                return Err(self.unexpected(&close.to_string()));
            }
            if self.expand_in_item_position(Position::Items)? {
                continue;
            }
            items.push(self.parse_item()?);
        }
        Ok(items)
    }

    /// Does an item (rather than an expression) start at the current token?
    pub(super) fn at_item_start(&self) -> bool {
        match self.peek() {
            TokenKind::Pound => true,
            TokenKind::Keyword(
                Keyword::Pub
                | Keyword::Fn
                | Keyword::Struct
                | Keyword::Enum
                | Keyword::Trait
                | Keyword::Impl
                | Keyword::Mod
                | Keyword::Use
                | Keyword::Type
                | Keyword::Static
                | Keyword::Extern,
            ) => true,
            // `const NAME` is an item; `unsafe { }` is an expression.
            TokenKind::Keyword(Keyword::Const) => {
                !matches!(self.peek_nth(1), TokenKind::LBrace)
            }
            TokenKind::Keyword(Keyword::Unsafe) => matches!(
                self.peek_nth(1),
                TokenKind::Keyword(Keyword::Fn | Keyword::Impl | Keyword::Trait | Keyword::Extern)
            ),
            TokenKind::Ident(name) => {
                (name == "macro_rules" && *self.peek_nth(1) == TokenKind::Not)
                    || (name == "async" && *self.peek_nth(1) == TokenKind::Keyword(Keyword::Fn))
            }
            _ => false,
        }
    }

    pub(super) fn parse_item(&mut self) -> Result<Item> {
        let start = self.span();
        let attrs = self.parse_attributes()?;
        self.parse_item_after(attrs, start)
    }

    /// An item whose attributes, starting at `start`, have been parsed.
    pub(super) fn parse_item_after(&mut self, attrs: Vec<Attribute>, start: Span) -> Result<Item> {
        let is_pub = self.parse_visibility()?;

        let kind = match self.peek().clone() {
            TokenKind::Keyword(Keyword::Struct) => ItemKind::Struct(self.parse_struct()?),
            TokenKind::Keyword(Keyword::Enum) => ItemKind::Enum(self.parse_enum()?),
            TokenKind::Keyword(Keyword::Trait) => ItemKind::Trait(self.parse_trait()?),
            TokenKind::Keyword(Keyword::Impl) => ItemKind::Impl(self.parse_impl()?),
            TokenKind::Keyword(Keyword::Mod) => ItemKind::Mod(self.parse_module()?),
            TokenKind::Keyword(Keyword::Type) => ItemKind::TypeAlias(self.parse_type_alias()?),
            TokenKind::Keyword(Keyword::Static) => {
                self.bump();
                let mutable = self.eat_keyword(Keyword::Mut);
                ItemKind::Static(self.parse_const_body(mutable)?)
            }
            TokenKind::Keyword(Keyword::Const)
                if !matches!(self.peek_nth(1), TokenKind::Keyword(Keyword::Fn | Keyword::Unsafe)) =>
            {
                self.bump();
                ItemKind::Const(self.parse_const_body(false)?)
            }
            TokenKind::Keyword(Keyword::Use) => {
                self.bump();
                let tree = self.parse_use_tree()?;
                self.expect(&TokenKind::Semi)?;
                ItemKind::Use(tree)
            }
            TokenKind::Keyword(Keyword::Extern)
                if matches!(self.peek_nth(1), TokenKind::Str(_))
                    && *self.peek_nth(2) == TokenKind::LBrace =>
            {
                ItemKind::ExternBlock(self.parse_extern_block()?)
            }
            TokenKind::Keyword(Keyword::Unsafe)
                if matches!(
                    self.peek_nth(1),
                    TokenKind::Keyword(Keyword::Impl | Keyword::Trait)
                ) =>
            {
                self.bump();
                return self.parse_item();
            }
            TokenKind::Keyword(
                Keyword::Fn | Keyword::Const | Keyword::Unsafe | Keyword::Extern,
            ) => ItemKind::Fn(self.parse_function()?),
            TokenKind::Ident(name) if name == "async" => {
                return Err(self.unsupported("`async fn`"));
            }
            _ => return Err(self.unexpected("an item")),
        };
        Ok(Item { kind, attrs, is_pub, span: start.to(self.prev_span()) })
    }

    /// `#[name]`, `#[name(a, b)]`, `#![inner]` — unknown contents are skipped.
    pub(super) fn parse_attributes(&mut self) -> Result<Vec<Attribute>> {
        let mut attrs = Vec::new();
        while self.eat(&TokenKind::Pound) {
            self.eat(&TokenKind::Not);
            self.expect(&TokenKind::LBracket)?;
            let name = self.expect_ident()?.name;
            let mut args = Vec::new();
            let mut depth = 1usize;
            while depth > 0 {
                match self.bump().kind {
                    TokenKind::LBracket => depth += 1,
                    TokenKind::RBracket => depth -= 1,
                    TokenKind::Ident(arg) => args.push(arg),
                    TokenKind::Eof => return Err(self.unexpected("`]`")),
                    _ => {}
                }
            }
            attrs.push(Attribute { name, args });
        }
        Ok(attrs)
    }

    /// `pub`, `pub(crate)`, `pub(in path)`; returns whether the item is public.
    fn parse_visibility(&mut self) -> Result<bool> {
        if !self.eat_keyword(Keyword::Pub) {
            return Ok(false);
        }
        if self.eat(&TokenKind::LParen) {
            while !self.eat(&TokenKind::RParen) {
                if self.at(&TokenKind::Eof) {
                    return Err(self.unexpected("`)`"));
                }
                self.bump();
            }
        }
        Ok(true)
    }

    fn parse_generics_with_where(&mut self) -> Result<Generics> {
        Ok(Generics { params: self.parse_generic_params()?, where_clauses: Vec::new() })
    }

    // -- functions ---------------------------------------------------------------

    /// A function with a body, or a bodiless signature ending in `;`.
    pub(super) fn parse_function(&mut self) -> Result<Function> {
        let start = self.span();
        self.eat_keyword(Keyword::Const);
        let is_unsafe = self.eat_keyword(Keyword::Unsafe);
        let mut is_extern_c = false;
        if self.eat_keyword(Keyword::Extern) {
            is_extern_c = true;
            if matches!(self.peek(), TokenKind::Str(_)) {
                self.bump();
            }
        }
        self.expect_keyword(Keyword::Fn)?;
        let name = self.expect_ident()?;
        let mut generics = self.parse_generics_with_where()?;

        self.expect(&TokenKind::LParen)?;
        let self_param = self.parse_self_param()?;
        let mut params = Vec::new();
        let mut is_variadic = false;
        while !self.at(&TokenKind::RParen) {
            if self.eat(&TokenKind::DotDotDot) {
                is_variadic = true;
                break;
            }
            let pat = self.parse_pattern_no_alternatives()?;
            self.expect(&TokenKind::Colon)?;
            params.push(Param { pat, ty: self.parse_type()? });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen)?;

        let ret = self.parse_return_type()?;
        generics.where_clauses = self.parse_where_clauses()?;
        let body = if self.eat(&TokenKind::Semi) { None } else { Some(self.parse_block()?) };

        Ok(Function {
            name,
            generics,
            self_param,
            params,
            ret,
            body,
            is_unsafe,
            is_extern_c,
            is_variadic,
            is_pub: false,
            span: start.to(self.prev_span()),
        })
    }

    /// `self`, `mut self`, `&self`, `&'a mut self` at the head of a parameter list.
    fn parse_self_param(&mut self) -> Result<Option<SelfParam>> {
        let start = self.span();
        let mut ahead = 0;
        let by_ref = *self.peek() == TokenKind::And;
        if by_ref {
            ahead += 1;
            if matches!(self.peek_nth(ahead), TokenKind::Lifetime(_)) {
                ahead += 1;
            }
        }
        let mutable = *self.peek_nth(ahead) == TokenKind::Keyword(Keyword::Mut);
        if mutable {
            ahead += 1;
        }
        if *self.peek_nth(ahead) != TokenKind::Keyword(Keyword::SelfValue) {
            return Ok(None);
        }
        for _ in 0..=ahead {
            self.bump();
        }

        if self.eat(&TokenKind::Colon) {
            return Err(self.unsupported("an explicitly typed `self` parameter"));
        }
        if !self.at(&TokenKind::RParen) {
            self.expect(&TokenKind::Comma)?;
        }
        let kind = match (by_ref, mutable) {
            (false, _) => SelfKind::Value,
            (true, false) => SelfKind::Ref,
            (true, true) => SelfKind::RefMut,
        };
        Ok(Some(SelfParam { kind, mutable: mutable && !by_ref, span: start.to(self.prev_span()) }))
    }

    /// `extern "C" { fn name(args) -> ret; ... }`
    fn parse_extern_block(&mut self) -> Result<Vec<Function>> {
        self.expect_keyword(Keyword::Extern)?;
        self.bump(); // ABI string
        self.expect(&TokenKind::LBrace)?;
        let mut functions = Vec::new();
        while !self.eat(&TokenKind::RBrace) {
            self.parse_attributes()?;
            self.parse_visibility()?;
            let mut function = self.parse_function()?;
            if function.body.is_some() {
                bail!(function.span, "functions in an `extern` block cannot have a body");
            }
            function.is_extern_c = true;
            functions.push(function);
        }
        Ok(functions)
    }

    // -- data types --------------------------------------------------------------

    fn parse_struct(&mut self) -> Result<StructDef> {
        self.expect_keyword(Keyword::Struct)?;
        let name = self.expect_ident()?;
        let mut generics = self.parse_generics_with_where()?;
        generics.where_clauses = self.parse_where_clauses()?;
        let (fields, public_fields) = self.parse_variant_fields()?;
        if matches!(fields, VariantFields::Tuple(_)) {
            generics.where_clauses.extend(self.parse_where_clauses()?);
        }
        if !matches!(fields, VariantFields::Named(_)) {
            self.expect(&TokenKind::Semi)?;
        }
        Ok(StructDef { name, generics, fields, public_fields })
    }

    fn parse_enum(&mut self) -> Result<EnumDef> {
        self.expect_keyword(Keyword::Enum)?;
        let name = self.expect_ident()?;
        let mut generics = self.parse_generics_with_where()?;
        generics.where_clauses = self.parse_where_clauses()?;
        self.expect(&TokenKind::LBrace)?;
        let variants = self.comma_separated(&TokenKind::RBrace, |p| {
            let is_default = p.parse_attributes()?.iter().any(|attr| attr.name == "default");
            let name = p.expect_ident()?;
            let (fields, _) = p.parse_variant_fields()?;
            let discriminant = if p.eat(&TokenKind::Eq) { Some(p.parse_expr()?) } else { None };
            Ok(Variant { name, fields, discriminant, is_default })
        })?;
        Ok(EnumDef { name, generics, variants })
    }

    /// `{ name: Type, ... }`, `(Type, ...)` or nothing; with whether each
    /// field is declared `pub`.
    fn parse_variant_fields(&mut self) -> Result<(VariantFields, Vec<bool>)> {
        let mut public = Vec::new();
        let fields = if self.eat(&TokenKind::LBrace) {
            let fields = self.comma_separated(&TokenKind::RBrace, |p| {
                p.parse_attributes()?;
                public.push(p.parse_visibility()?);
                let name = p.expect_ident()?;
                p.expect(&TokenKind::Colon)?;
                Ok(FieldDef { name, ty: p.parse_type()? })
            })?;
            VariantFields::Named(fields)
        } else if self.eat(&TokenKind::LParen) {
            let types = self.comma_separated(&TokenKind::RParen, |p| {
                p.parse_attributes()?;
                public.push(p.parse_visibility()?);
                p.parse_type()
            })?;
            VariantFields::Tuple(types)
        } else {
            VariantFields::Unit
        };
        Ok((fields, public))
    }

    /// `NAME: Type = value;` after `const` / `static` has been consumed.
    fn parse_const_body(&mut self, mutable: bool) -> Result<ConstDef> {
        let name = match self.peek() {
            // `const _: () = ...;`
            TokenKind::Underscore => Ident { name: "_".to_string(), span: self.bump().span },
            _ => self.expect_ident()?,
        };
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let value = if self.eat(&TokenKind::Eq) { Some(self.parse_expr()?) } else { None };
        self.expect(&TokenKind::Semi)?;
        Ok(ConstDef { name, ty, value, mutable })
    }

    fn parse_type_alias(&mut self) -> Result<TypeAlias> {
        self.expect_keyword(Keyword::Type)?;
        let name = self.expect_ident()?;
        let generics = self.parse_generics_with_where()?;
        self.expect(&TokenKind::Eq)?;
        let ty = self.parse_type()?;
        self.expect(&TokenKind::Semi)?;
        Ok(TypeAlias { name, generics, ty })
    }

    // -- traits and impls ----------------------------------------------------------

    fn parse_trait(&mut self) -> Result<TraitDef> {
        self.expect_keyword(Keyword::Trait)?;
        let name = self.expect_ident()?;
        let mut generics = self.parse_generics_with_where()?;
        let supertraits =
            if self.eat(&TokenKind::Colon) { self.parse_bounds()? } else { Vec::new() };
        generics.where_clauses = self.parse_where_clauses()?;
        let items = self.parse_assoc_items()?;
        Ok(TraitDef { name, generics, supertraits, items })
    }

    fn parse_impl(&mut self) -> Result<ImplBlock> {
        self.expect_keyword(Keyword::Impl)?;
        let mut generics = self.parse_generics_with_where()?;
        let first = self.parse_type()?;
        let (trait_ref, self_ty) = if self.eat_keyword(Keyword::For) {
            let TypeKind::Path(trait_path) = first.kind else {
                bail!(first.span, "expected a trait name before `for`");
            };
            (Some(trait_path), self.parse_type()?)
        } else {
            (None, first)
        };
        generics.where_clauses = self.parse_where_clauses()?;
        let items = self.parse_assoc_items()?;
        Ok(ImplBlock { generics, trait_ref, self_ty, items })
    }

    /// The `{ ... }` body of a trait or impl.
    fn parse_assoc_items(&mut self) -> Result<Vec<AssocItem>> {
        self.expect(&TokenKind::LBrace)?;
        let mut items = Vec::new();
        while !self.eat(&TokenKind::RBrace) {
            if self.expand_in_item_position(Position::Items)? {
                continue;
            }
            self.parse_attributes()?;
            let is_pub = self.parse_visibility()?;
            let item = if self.eat_keyword(Keyword::Type) {
                let name = self.expect_ident()?;
                if self.eat(&TokenKind::Colon) {
                    self.parse_bounds()?;
                }
                let ty = if self.eat(&TokenKind::Eq) { Some(self.parse_type()?) } else { None };
                self.expect(&TokenKind::Semi)?;
                AssocItem::Type { name, ty }
            } else if self.at_keyword(Keyword::Const)
                && !matches!(self.peek_nth(1), TokenKind::Keyword(Keyword::Fn | Keyword::Unsafe))
            {
                self.bump();
                AssocItem::Const(self.parse_const_body(false)?)
            } else {
                let mut function = self.parse_function()?;
                function.is_pub = is_pub;
                AssocItem::Fn(function)
            };
            items.push(item);
        }
        Ok(items)
    }

    // -- modules and imports ---------------------------------------------------------

    /// `mod name { items }` or `mod name;` (which loads the module's file).
    fn parse_module(&mut self) -> Result<Module> {
        self.expect_keyword(Keyword::Mod)?;
        let name = self.expect_ident()?;
        let items = if self.eat(&TokenKind::Semi) {
            self.load_module_file(&name)?
        } else {
            self.expect(&TokenKind::LBrace)?;
            let items = self.parse_items_until(&TokenKind::RBrace)?;
            self.expect(&TokenKind::RBrace)?;
            items
        };
        Ok(Module { name, items })
    }

    /// `a::b::c`, `a::b as c`, `a::*`, `a::{b, c::d}`
    fn parse_use_tree(&mut self) -> Result<UseTree> {
        let mut prefix = Vec::new();
        self.eat(&TokenKind::PathSep);
        loop {
            if self.eat(&TokenKind::Star) {
                return Ok(UseTree { prefix, kind: UseKind::Glob });
            }
            if self.eat(&TokenKind::LBrace) {
                let nested = self.comma_separated(&TokenKind::RBrace, Self::parse_use_tree)?;
                return Ok(UseTree { prefix, kind: UseKind::Nested(nested) });
            }
            let keyword_name = match self.peek() {
                TokenKind::Keyword(Keyword::SelfValue) => Some("self"),
                TokenKind::Keyword(Keyword::Super) => Some("super"),
                TokenKind::Keyword(Keyword::Crate) => Some("crate"),
                _ => None,
            };
            prefix.push(match keyword_name {
                Some(name) => Ident { name: name.to_string(), span: self.bump().span },
                None => self.expect_ident()?,
            });
            if !self.eat(&TokenKind::PathSep) {
                break;
            }
        }
        let alias = if self.eat_keyword(Keyword::As) {
            match self.peek() {
                TokenKind::Underscore => Some(Ident { name: "_".to_string(), span: self.bump().span }),
                _ => Some(self.expect_ident()?),
            }
        } else {
            None
        };
        Ok(UseTree { prefix, kind: UseKind::Simple(alias) })
    }
}
