//! `#[derive(...)]`.
//!
//! A derive is expanded by writing the impl as Rust source and parsing it:
//! the generated code is exactly what a programmer would have typed, and it
//! goes through the same checking as everything else.
//!
//! Structs and enums share one code path: every impl is a `match` over the
//! type's variants, and a struct simply has one.

use crate::syntax::ast::{GenericParam, Generics, Item, ItemKind, VariantFields};
use crate::syntax::diagnostic::{bail, Result};
use crate::syntax::parser::parse_source;
use crate::syntax::span::SourceMap;
use std::fmt::Write;

/// Append the impls requested by `#[derive]` attributes to `items`
/// (recursively through inline modules).
pub fn expand(items: &mut Vec<Item>, sources: &mut SourceMap) -> Result<()> {
    let mut generated = Vec::new();
    for item in items.iter_mut() {
        if let ItemKind::Mod(module) = &mut item.kind {
            expand(&mut module.items, sources)?;
            continue;
        }
        let Some(target) = Target::of(item, sources) else { continue };
        for attr in item.attrs.iter().filter(|attr| attr.name == "derive") {
            for trait_name in &attr.args {
                let Some(source) = target.derive(trait_name) else {
                    bail!(item.span, "cannot derive `{trait_name}`: this derive is not supported");
                };
                let origin = format!("<derive({trait_name}) for {}>", target.name);
                generated.extend(parse_source(sources, &origin, &source)?);
            }
        }
    }
    items.extend(generated);
    Ok(())
}

/// The struct or enum a derive applies to.
struct Target<'a> {
    name: &'a str,
    params: Vec<Param<'a>>,
    variants: Vec<Variant<'a>>,
}

/// A generic parameter of the target: a type, which the derived trait's
/// impl requires to implement that trait too, or a constant.
struct Param<'a> {
    name: &'a str,
    /// For `const N: usize`, the declared type as written.
    const_ty: Option<String>,
}

struct Variant<'a> {
    /// How the variant is named in patterns and expressions: `Point`, `Shape::Circle`.
    path: String,
    name: &'a str,
    fields: Fields<'a>,
    /// The enum variant marked `#[default]`.
    is_default: bool,
}

enum Fields<'a> {
    Unit,
    Tuple(usize),
    Named(Vec<&'a str>),
}

impl Fields<'_> {
    fn len(&self) -> usize {
        match self {
            Fields::Unit => 0,
            Fields::Tuple(count) => *count,
            Fields::Named(names) => names.len(),
        }
    }
}

impl<'a> Fields<'a> {
    fn of(fields: &'a VariantFields) -> Fields<'a> {
        match fields {
            VariantFields::Unit => Fields::Unit,
            VariantFields::Tuple(types) => Fields::Tuple(types.len()),
            VariantFields::Named(fields) => Fields::Named(fields.iter().map(|f| f.name.name.as_str()).collect()),
        }
    }
}

impl<'a> Variant<'a> {
    /// The variant with one value per field: a pattern if the values are
    /// binding names, an expression if they are expressions.
    fn with_fields(&self, values: &[String]) -> String {
        match &self.fields {
            Fields::Unit => self.path.clone(),
            Fields::Tuple(_) => format!("{}({})", self.path, values.join(", ")),
            Fields::Named(names) => {
                let fields: Vec<String> = names.iter().zip(values).map(|(n, v)| format!("{n}: {v}")).collect();
                format!("{} {{ {} }}", self.path, fields.join(", "))
            }
        }
    }

    /// Binding names for the fields, `prefix0`, `prefix1`, ...
    fn bindings(&self, prefix: &str) -> Vec<String> {
        (0..self.fields.len()).map(|i| format!("{prefix}{i}")).collect()
    }

    fn pattern(&self, prefix: &str) -> String {
        self.with_fields(&self.bindings(prefix))
    }
}

impl<'a> Target<'a> {
    fn of(item: &'a Item, sources: &SourceMap) -> Option<Target<'a>> {
        let params = |generics: &'a Generics| {
            let param = |p: &'a GenericParam| Param {
                name: p.name.name.as_str(),
                const_ty: p.const_ty.as_ref().map(|ty| sources.snippet(ty.span).to_string()),
            };
            generics.params.iter().map(param).collect()
        };
        match &item.kind {
            ItemKind::Struct(def) => {
                let name = def.name.name.as_str();
                let variant = Variant { path: name.to_string(), name, fields: Fields::of(&def.fields), is_default: true };
                Some(Target { name, params: params(&def.generics), variants: vec![variant] })
            }
            ItemKind::Enum(def) => {
                let name = def.name.name.as_str();
                let variants = def.variants.iter().map(|v| Variant {
                    path: format!("{name}::{}", v.name.name),
                    name: v.name.name.as_str(),
                    fields: Fields::of(&v.fields),
                    is_default: v.is_default,
                });
                Some(Target { name, params: params(&def.generics), variants: variants.collect() })
            }
            _ => None,
        }
    }

    /// `impl<T: Bound> Trait for Name<T>`
    fn header(&self, trait_path: &str) -> String {
        if self.params.is_empty() {
            return format!("impl {trait_path} for {}", self.name);
        }
        let declared: Vec<String> = self
            .params
            .iter()
            .map(|param| match &param.const_ty {
                Some(ty) => format!("const {}: {ty}", param.name),
                None => format!("{}: {trait_path}", param.name),
            })
            .collect();
        let names: Vec<&str> = self.params.iter().map(|param| param.name).collect();
        format!("impl<{}> {trait_path} for {}<{}>", declared.join(", "), self.name, names.join(", "))
    }

    /// `match SCRUTINEE { arm, arm, ... }` with one arm per variant.
    fn match_variants(&self, scrutinee: &str, mut arm: impl FnMut(&Variant) -> String) -> String {
        let arms: Vec<String> = self.variants.iter().map(|v| arm(v)).collect();
        format!("match {scrutinee} {{ {} }}", arms.join(" "))
    }

    /// The position of the value's variant, as an `isize` expression.
    fn variant_index(&self, value: &str) -> String {
        let mut index = 0;
        self.match_variants(value, |v| {
            index += 1;
            let ignore: Vec<String> = vec!["_".to_string(); v.fields.len()];
            format!("{} => {}isize,", v.with_fields(&ignore), index - 1)
        })
    }

    fn derive(&self, trait_name: &str) -> Option<String> {
        Some(match trait_name {
            "Clone" => self.derive_clone(),
            "Copy" => format!("{} {{}}", self.header("std::marker::Copy")),
            "Eq" => format!("{} {{}}", self.header("std::cmp::Eq")),
            "PartialEq" => self.derive_partial_eq(),
            "PartialOrd" => self.derive_ordering("std::cmp::PartialOrd", "partial_cmp"),
            "Ord" => self.derive_ordering("std::cmp::Ord", "cmp"),
            "Debug" => self.derive_debug(),
            "Default" => self.derive_default()?,
            "Hash" => self.derive_hash(),
            _ => return None,
        })
    }

    fn derive_clone(&self) -> String {
        let body = self.match_variants("self", |v| {
            let clones: Vec<String> =
                v.bindings("f").iter().map(|f| format!("std::clone::Clone::clone({f})")).collect();
            format!("{} => {},", v.pattern("f"), v.with_fields(&clones))
        });
        format!("{} {{ fn clone(&self) -> Self {{ {body} }} }}", self.header("std::clone::Clone"))
    }

    fn derive_partial_eq(&self) -> String {
        let mut body = self.match_variants("(self, other)", |v| {
            let equal: Vec<String> = (0..v.fields.len()).map(|i| format!("a{i} == b{i}")).collect();
            let all_equal = if equal.is_empty() { "true".to_string() } else { equal.join(" && ") };
            format!("({}, {}) => {all_equal},", v.pattern("a"), v.pattern("b"))
        });
        if self.variants.len() > 1 {
            // Two different variants are never equal.
            body.insert_str(body.len() - 1, "_ => false, ");
        }
        format!("{} {{ fn eq(&self, other: &Self) -> bool {{ {body} }} }}", self.header("std::cmp::PartialEq"))
    }

    /// `PartialOrd` and `Ord`: variants order by position, then fields left to right.
    fn derive_ordering(&self, trait_path: &str, method: &str) -> String {
        let partial = method == "partial_cmp";
        let (ret, equal) = if partial {
            ("std::option::Option<std::cmp::Ordering>", "std::option::Option::Some(std::cmp::Ordering::Equal)")
        } else {
            ("std::cmp::Ordering", "std::cmp::Ordering::Equal")
        };

        let mut body = String::new();
        if self.variants.len() > 1 {
            let _ = write!(
                body,
                "let self_index = {}; let other_index = {}; \
                 if self_index != other_index {{ return {trait_path}::{method}(&self_index, &other_index); }} ",
                self.variant_index("self"),
                self.variant_index("other"),
            );
        }
        let mut by_fields = self.match_variants("(self, other)", |v| {
            let mut steps = String::new();
            for i in 0..v.fields.len() {
                let _ = write!(
                    steps,
                    "match {trait_path}::{method}(a{i}, b{i}) {{ {equal} => {{}} unequal => return unequal, }} "
                );
            }
            format!("({}, {}) => {{ {steps}{equal} }}", v.pattern("a"), v.pattern("b"))
        });
        if self.variants.len() > 1 {
            by_fields.insert_str(by_fields.len() - 1, &format!("_ => {equal}, "));
        }
        body.push_str(&by_fields);
        format!("{} {{ fn {method}(&self, other: &Self) -> {ret} {{ {body} }} }}", self.header(trait_path))
    }

    fn derive_debug(&self) -> String {
        // The formatter's builders take care of `{:?}` versus `{:#?}`.
        let body = self.match_variants("self", |v| {
            let bindings = v.bindings("f");
            let call = match &v.fields {
                Fields::Unit => format!("f.write_str(\"{}\")", v.name),
                Fields::Tuple(_) => {
                    let fields: String = bindings.iter().map(|binding| format!(".field({binding})")).collect();
                    format!("f.debug_tuple(\"{}\"){fields}.finish()", v.name)
                }
                Fields::Named(names) => {
                    let fields: String =
                        bindings.iter().zip(names).map(|(binding, name)| format!(".field(\"{name}\", {binding})")).collect();
                    format!("f.debug_struct(\"{}\"){fields}.finish()", v.name)
                }
            };
            format!("{} => {{ {call} }}", v.pattern("f"))
        });
        format!(
            "{} {{ fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{ {body} }} }}",
            self.header("std::fmt::Debug")
        )
    }

    fn derive_default(&self) -> Option<String> {
        // A struct's default has every field's; an enum's is the unit
        // variant marked `#[default]`.
        let variant = match self.variants.as_slice() {
            [variant] if variant.path == self.name => variant,
            variants => variants.iter().find(|variant| variant.is_default && variant.fields.len() == 0)?,
        };
        let defaults = vec!["std::default::Default::default()".to_string(); variant.fields.len()];
        Some(format!(
            "{} {{ fn default() -> Self {{ {} }} }}",
            self.header("std::default::Default"),
            variant.with_fields(&defaults)
        ))
    }

    fn derive_hash(&self) -> String {
        let mut body = String::new();
        if self.variants.len() > 1 {
            let _ = write!(body, "std::hash::Hash::hash(&{}, state); ", self.variant_index("self"));
        }
        body.push_str(&self.match_variants("self", |v| {
            let hashes: Vec<String> =
                v.bindings("f").iter().map(|f| format!("std::hash::Hash::hash({f}, state);")).collect();
            format!("{} => {{ {} }}", v.pattern("f"), hashes.join(" "))
        }));
        format!(
            "{} {{ fn hash<H: std::hash::Hasher>(&self, state: &mut H) {{ {body} }} }}",
            self.header("std::hash::Hash")
        )
    }
}
