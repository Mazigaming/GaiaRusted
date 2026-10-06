//! Everything a program declares, gathered before any body is checked.
//!
//! [`collect`] walks the AST once and records each function, type, trait,
//! impl and constant under an id, builds the module tree, and resolves `use`
//! declarations. After that, "what does this name mean here?" is a table
//! lookup ([`Defs::resolve_path`]).

use super::ty::{AdtId, FnId, TraitId};
use crate::syntax::ast::{self, AssocItem, ItemKind, UseKind, VariantFields};
use crate::syntax::diagnostic::{bail, Diagnostic, Result};
use crate::syntax::span::Span;
use crate::syntax::visit;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImplId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConstId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AliasId(pub u32);

/// What a name can refer to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Def {
    Mod(ModId),
    Fn(FnId),
    Adt(AdtId),
    /// One variant of an enum, by index.
    Variant(AdtId, u32),
    Trait(TraitId),
    Const(ConstId),
    Alias(AliasId),
}

pub struct ModuleData {
    pub name: String,
    pub parent: Option<ModId>,
    pub names: HashMap<String, Def>,
    /// The names declared `pub`, which code outside the module may use.
    pub public: HashSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FnOwner {
    /// A free function.
    Free,
    /// A method or associated function in an `impl` block.
    Impl(ImplId),
    /// A method declared (and possibly given a default body) in a trait.
    Trait(TraitId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FnKind {
    /// Has a body to compile.
    Defined,
    /// Declared in an `extern "C"` block; lives in another object file.
    Extern,
    /// A bodiless standard-library declaration the compiler implements itself.
    Intrinsic,
    /// A trait method without a default body.
    Required,
}

pub struct FnDef<'a> {
    pub ast: &'a ast::Function,
    pub module: ModId,
    pub owner: FnOwner,
    pub kind: FnKind,
    /// Human-readable path, e.g. `geometry::Point::area`.
    pub path: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdtKind {
    Struct,
    Enum,
}

pub struct AdtDef<'a> {
    pub name: String,
    pub module: ModId,
    pub kind: AdtKind,
    pub generics: &'a ast::Generics,
    /// A struct has exactly one variant.
    pub variants: Vec<VariantDef<'a>>,
    /// The struct's first field is never zero, so an enum can use zero to
    /// stand for a variant without data: `#[rustc_nonnull_optimization_guaranteed]`.
    pub never_zero: bool,
    pub span: Span,
}

impl AdtDef<'_> {
    pub fn is_enum(&self) -> bool {
        self.kind == AdtKind::Enum
    }

    pub fn variant_index(&self, name: &str) -> Option<u32> {
        self.variants.iter().position(|v| v.name == name).map(|i| i as u32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariantShape {
    Unit,
    Tuple,
    Named,
}

pub struct VariantDef<'a> {
    pub name: String,
    pub shape: VariantShape,
    pub fields: Vec<FieldDef<'a>>,
    /// Explicit `= value` discriminant expression, if written.
    pub discriminant: Option<&'a ast::Expr>,
}

impl VariantDef<'_> {
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }
}

pub struct FieldDef<'a> {
    /// The field's name; tuple fields are named `0`, `1`, ...
    pub name: String,
    pub ty: &'a ast::Type,
    /// Visible outside the module that defines the type: declared `pub`,
    /// or a field of an enum variant.
    pub is_pub: bool,
}

pub struct TraitDef<'a> {
    pub name: String,
    pub module: ModId,
    pub ast: &'a ast::TraitDef,
    pub methods: HashMap<String, FnId>,
    pub assoc_types: Vec<String>,
    /// Associated constants, with their default values if they have any.
    pub consts: HashMap<String, ConstId>,
}

pub struct ImplDef<'a> {
    pub module: ModId,
    pub ast: &'a ast::ImplBlock,
    /// The implemented trait, for `impl Trait for Type`.
    pub trait_id: Option<TraitId>,
    pub methods: HashMap<String, FnId>,
    pub assoc_types: HashMap<String, &'a ast::Type>,
    pub consts: HashMap<String, ConstId>,
}

pub struct ConstDef<'a> {
    pub ast: &'a ast::ConstDef,
    pub module: ModId,
    /// The impl whose generics and `Self` are in scope, for associated constants.
    pub owner: Option<ImplId>,
    /// The trait that declares it, for a trait's constant: its value, if
    /// any, is the default for impls that do not give one.
    pub trait_owner: Option<TraitId>,
    pub is_static: bool,
}

pub struct AliasDef<'a> {
    pub ast: &'a ast::TypeAlias,
    pub module: ModId,
}

/// The result of resolving a path as far as declarations go. Any `rest`
/// segments name something type-relative (`Vec::new`, `T::Item`) that only
/// the type checker can look up.
pub struct Resolution<'p> {
    pub def: Def,
    pub rest: &'p [ast::PathSegment],
}

pub struct Defs<'a> {
    pub modules: Vec<ModuleData>,
    pub fns: Vec<FnDef<'a>>,
    pub adts: Vec<AdtDef<'a>>,
    pub traits: Vec<TraitDef<'a>>,
    pub impls: Vec<ImplDef<'a>>,
    pub consts: Vec<ConstDef<'a>>,
    pub aliases: Vec<AliasDef<'a>>,
    /// Impl blocks that define a method or associated function of each name.
    pub impls_by_member: HashMap<String, Vec<ImplId>>,
    /// The module whose names are visible everywhere without a `use`.
    pub prelude: ModId,
    /// The program's crate name, which `std::any::type_name` writes in
    /// front of its own items: the root file's name.
    pub crate_name: String,
}

/// The standard library's root module; `std::`, `core::` and `alloc::` paths start here.
pub const STD_ROOT: ModId = ModId(0);
/// The root module of the program being compiled.
pub const CRATE_ROOT: ModId = ModId(1);

impl<'a> Defs<'a> {
    pub fn module(&self, id: ModId) -> &ModuleData {
        &self.modules[id.0 as usize]
    }

    /// Can code in module `from` see the private items and fields of module
    /// `owner`? Only from `owner` itself and the modules inside it. The
    /// standard library is written as one unit and sees all of itself.
    pub fn sees_private(&self, owner: ModId, from: ModId) -> bool {
        let mut module = Some(from);
        let mut in_std = false;
        while let Some(current) = module {
            if current == owner {
                return true;
            }
            in_std |= current == STD_ROOT;
            module = self.module(current).parent;
        }
        in_std
    }
    pub fn fn_def(&self, id: FnId) -> &FnDef<'a> {
        &self.fns[id.0 as usize]
    }
    pub fn adt(&self, id: AdtId) -> &AdtDef<'a> {
        &self.adts[id.0 as usize]
    }
    pub fn trait_def(&self, id: TraitId) -> &TraitDef<'a> {
        &self.traits[id.0 as usize]
    }
    pub fn impl_def(&self, id: ImplId) -> &ImplDef<'a> {
        &self.impls[id.0 as usize]
    }
    pub fn const_def(&self, id: ConstId) -> &ConstDef<'a> {
        &self.consts[id.0 as usize]
    }
    pub fn alias(&self, id: AliasId) -> &AliasDef<'a> {
        &self.aliases[id.0 as usize]
    }

    /// A standard-library item by its path from the std root, e.g. `["string", "String"]`.
    pub fn std_item(&self, path: &[&str]) -> Option<Def> {
        let (last, modules) = path.split_last()?;
        let mut module = STD_ROOT;
        for name in modules {
            match self.module(module).names.get(*name)? {
                Def::Mod(next) => module = *next,
                _ => return None,
            }
        }
        self.module(module).names.get(*last).copied()
    }

    /// The names of the generic parameters in scope inside a function, outermost
    /// first: those of its impl or trait (a trait contributes `Self` too), then its own.
    pub fn fn_generics(&self, id: FnId) -> Vec<String> {
        let def = self.fn_def(id);
        let of_owner: Vec<&str> = match def.owner {
            FnOwner::Free => Vec::new(),
            FnOwner::Impl(impl_id) => self.impl_generics(impl_id),
            FnOwner::Trait(trait_id) => {
                let of_trait = self.trait_def(trait_id).ast.generics.params.iter();
                std::iter::once("Self").chain(of_trait.map(|p| p.name.name.as_str())).collect()
            }
        };
        of_owner.into_iter().map(str::to_string).chain(self.fn_own_generics(id)).collect()
    }

    /// A function's own generic parameters: the declared ones, then the
    /// anonymous ones that stand for `impl Trait` arguments.
    pub fn fn_own_generics(&self, id: FnId) -> Vec<String> {
        self.fn_def(id).ast.generics.params.iter().map(|p| p.name.name.clone()).collect()
    }

    /// The const generic parameters a function's body sees, its owner's and
    /// its own, with their declared types.
    pub fn fn_const_params(&self, id: FnId) -> Vec<(&'a str, &'a ast::Type)> {
        let def = self.fn_def(id);
        let of_owner = match def.owner {
            FnOwner::Free => None,
            FnOwner::Impl(impl_id) => Some(&self.impl_def(impl_id).ast.generics),
            FnOwner::Trait(trait_id) => Some(&self.trait_def(trait_id).ast.generics),
        };
        of_owner
            .into_iter()
            .chain([&def.ast.generics])
            .flat_map(|generics| generics.params.iter())
            .filter_map(|param| param.const_ty.as_ref().map(|ty| (param.name.name.as_str(), ty)))
            .collect()
    }

    pub fn impl_generics(&self, id: ImplId) -> Vec<&'a str> {
        self.impl_def(id).ast.generics.params.iter().map(|p| p.name.name.as_str()).collect()
    }

    /// Look a single name up the way an unqualified use sees it: the module's
    /// own names first, then the prelude.
    pub fn lookup(&self, module: ModId, name: &str) -> Option<Def> {
        self.module(module)
            .names
            .get(name)
            .or_else(|| self.module(self.prelude).names.get(name))
            .copied()
    }

    /// Resolve `path` as seen from `module`, as far as declared names go.
    /// Returns `None` if the first segment names nothing (it may be a local
    /// variable, a generic parameter or a primitive type).
    pub fn resolve_path<'p>(&self, module: ModId, path: &'p ast::Path) -> Option<Resolution<'p>> {
        if path.qself.is_some() {
            return None;
        }
        let segments = &path.segments;
        let mut def = self.resolve_first(module, path)?;
        let mut consumed = 1;
        for segment in &segments[1..] {
            let name = segment.ident.name.as_str();
            let next = match def {
                Def::Mod(inner) if name == "super" => self.module(inner).parent.map(Def::Mod),
                Def::Mod(inner) => self.module(inner).names.get(name).copied(),
                Def::Adt(adt) => self.adt(adt).variant_index(name).map(|v| Def::Variant(adt, v)),
                _ => None,
            };
            match next {
                Some(next) => def = next,
                // A module member must exist; anything else may be type-relative.
                None if matches!(def, Def::Mod(_)) => return None,
                None => break,
            }
            consumed += 1;
        }
        Some(Resolution { def, rest: &segments[consumed..] })
    }

    /// What the first segment of `path` names, seen from `module`.
    fn resolve_first(&self, module: ModId, path: &ast::Path) -> Option<Def> {
        let first = path.segments[0].ident.name.as_str();
        Some(match first {
            "crate" => Def::Mod(CRATE_ROOT),
            "self" if path.segments.len() > 1 => Def::Mod(module),
            "super" => Def::Mod(self.module(module).parent?),
            "std" | "core" | "alloc" if !self.module(module).names.contains_key(first) => Def::Mod(STD_ROOT),
            _ => self.lookup(module, first)?,
        })
    }

    /// An error if `path` reaches, from `module`, into another module for a
    /// name that module does not declare `pub`.
    pub fn check_visible(&self, module: ModId, path: &ast::Path) -> Result<()> {
        // A name on its own is one the module sees.
        if path.qself.is_some() || path.segments.len() < 2 {
            return Ok(());
        }
        let Some(mut def) = self.resolve_first(module, path) else { return Ok(()) };
        for segment in &path.segments[1..] {
            let name = segment.ident.name.as_str();
            let Def::Mod(inner) = def else { return Ok(()) };
            let next = match name {
                "super" => self.module(inner).parent.map(Def::Mod),
                _ => self.module(inner).names.get(name).copied(),
            };
            let Some(next) = next else { return Ok(()) };
            if name != "super" && !self.module(inner).public.contains(name) && !self.sees_private(inner, module) {
                bail!(segment.ident.span, "{} `{name}` is private", self.describe(next));
            }
            def = next;
        }
        Ok(())
    }

    /// What kind of item a definition is, as messages name it.
    fn describe(&self, def: Def) -> &'static str {
        match def {
            Def::Fn(_) => "function",
            Def::Adt(adt) if self.adt(adt).is_enum() => "enum",
            Def::Adt(_) => "struct",
            Def::Variant(..) => "variant",
            Def::Trait(_) => "trait",
            Def::Const(_) => "constant",
            Def::Mod(_) => "module",
            Def::Alias(_) => "type alias",
        }
    }

    /// The path of a module from its root, for messages and symbol names.
    pub fn module_path(&self, id: ModId) -> String {
        let module = self.module(id);
        match module.parent {
            Some(parent) if parent != CRATE_ROOT && parent != STD_ROOT => {
                format!("{}::{}", self.module_path(parent), module.name)
            }
            Some(_) => module.name.clone(),
            None => String::new(),
        }
    }
}

/// Gather the declarations of the standard library and of the user's crate.
pub fn collect<'a>(std_items: &'a [ast::Item], crate_items: &'a [ast::Item]) -> Result<Defs<'a>> {
    let mut collector = Collector {
        defs: Defs {
            modules: Vec::new(),
            fns: Vec::new(),
            adts: Vec::new(),
            traits: Vec::new(),
            impls: Vec::new(),
            consts: Vec::new(),
            aliases: Vec::new(),
            impls_by_member: HashMap::new(),
            prelude: STD_ROOT,
            crate_name: "main".to_string(),
        },
        imports: Vec::new(),
        pending_impls: Vec::new(),
    };
    let std_root = collector.new_module("std", None);
    let crate_root = collector.new_module("crate", None);
    debug_assert_eq!((std_root, crate_root), (STD_ROOT, CRATE_ROOT));

    collector.collect_items(STD_ROOT, std_items)?;
    collector.collect_items(CRATE_ROOT, crate_items)?;

    if let Some(Def::Mod(prelude)) = collector.defs.module(STD_ROOT).names.get("prelude") {
        collector.defs.prelude = *prelude;
    }
    collector.resolve_imports()?;
    collector.collect_impls()?;
    Ok(collector.defs)
}

struct PendingImport<'a> {
    module: ModId,
    /// `pub use`: the imported names are public in `module`.
    is_pub: bool,
    /// The full path from the `use` up to this leaf.
    path: Vec<&'a ast::Ident>,
    /// `None` for a glob import.
    binding: Option<&'a ast::Ident>,
}

struct Collector<'a> {
    defs: Defs<'a>,
    imports: Vec<PendingImport<'a>>,
    /// Impl blocks wait until imports are resolved, so that the traits they
    /// name can be found.
    pending_impls: Vec<(ModId, &'a ast::ImplBlock)>,
}

impl<'a> Collector<'a> {
    fn new_module(&mut self, name: &str, parent: Option<ModId>) -> ModId {
        self.defs.modules.push(ModuleData { name: name.to_string(), parent, names: HashMap::new(), public: HashSet::new() });
        ModId(self.defs.modules.len() as u32 - 1)
    }

    fn define(&mut self, module: ModId, name: &ast::Ident, def: Def) -> Result<()> {
        let names = &mut self.defs.modules[module.0 as usize].names;
        if names.insert(name.name.clone(), def).is_some() {
            bail!(name.span, "the name `{}` is defined multiple times", name.name);
        }
        Ok(())
    }

    fn qualified(&self, module: ModId, name: &str) -> String {
        match self.defs.module_path(module) {
            path if path.is_empty() || module == CRATE_ROOT || module == STD_ROOT => name.to_string(),
            path => format!("{path}::{name}"),
        }
    }

    fn collect_items(&mut self, module: ModId, items: &'a [ast::Item]) -> Result<()> {
        items.iter().try_for_each(|item| self.collect_item(module, item))
    }

    fn collect_item(&mut self, module: ModId, item: &'a ast::Item) -> Result<()> {
        self.collect_item_kind(module, item)?;
        let public: Vec<&str> = match &item.kind {
            // The C functions a block declares are used by name from anywhere.
            ItemKind::ExternBlock(functions) => functions.iter().map(|function| function.name.name.as_str()).collect(),
            _ if !item.is_pub => Vec::new(),
            ItemKind::Fn(function) => vec![&function.name.name],
            ItemKind::Struct(def) => vec![&def.name.name],
            ItemKind::Enum(def) => vec![&def.name.name],
            ItemKind::Trait(def) => vec![&def.name.name],
            ItemKind::Const(def) | ItemKind::Static(def) => vec![&def.name.name],
            ItemKind::TypeAlias(alias) => vec![&alias.name.name],
            ItemKind::Mod(inner) => vec![&inner.name.name],
            _ => Vec::new(),
        };
        let names = &mut self.defs.modules[module.0 as usize].public;
        names.extend(public.into_iter().map(str::to_string));
        Ok(())
    }

    fn collect_item_kind(&mut self, module: ModId, item: &'a ast::Item) -> Result<()> {
        match &item.kind {
            ItemKind::Fn(function) => {
                let kind = if function.is_extern_c && function.body.is_none() {
                    FnKind::Extern
                } else if function.body.is_none() {
                    FnKind::Intrinsic
                } else {
                    FnKind::Defined
                };
                let path = self.qualified(module, &function.name.name);
                let id = self.add_fn(function, module, FnOwner::Free, kind, path)?;
                self.define(module, &function.name, Def::Fn(id))
            }
            ItemKind::ExternBlock(functions) => functions.iter().try_for_each(|function| {
                let path = function.name.name.clone();
                let id = self.add_fn(function, module, FnOwner::Free, FnKind::Extern, path)?;
                self.define(module, &function.name, Def::Fn(id))
            }),
            ItemKind::Struct(def) => {
                let mut variant = variant_def(def.name.name.clone(), &def.fields, None);
                for (field, &is_pub) in variant.fields.iter_mut().zip(&def.public_fields) {
                    field.is_pub = is_pub;
                }
                self.add_adt(module, &def.name, AdtKind::Struct, &def.generics, vec![variant], item)
            }
            ItemKind::Enum(def) => {
                let variants = def
                    .variants
                    .iter()
                    .map(|v| variant_def(v.name.name.clone(), &v.fields, v.discriminant.as_ref()))
                    .collect();
                self.add_adt(module, &def.name, AdtKind::Enum, &def.generics, variants, item)
            }
            ItemKind::Trait(def) => self.add_trait(module, def),
            ItemKind::Impl(block) => {
                self.pending_impls.push((module, block));
                for assoc in &block.items {
                    if let AssocItem::Fn(function) = assoc {
                        self.collect_nested_items(module, function)?;
                    }
                }
                Ok(())
            }
            ItemKind::Const(def) | ItemKind::Static(def) => {
                let is_static = matches!(item.kind, ItemKind::Static(_));
                self.defs.consts.push(ConstDef { ast: def, module, owner: None, trait_owner: None, is_static });
                let id = ConstId(self.defs.consts.len() as u32 - 1);
                if def.name.name == "_" {
                    return Ok(());
                }
                self.define(module, &def.name, Def::Const(id))
            }
            ItemKind::TypeAlias(alias) => {
                self.defs.aliases.push(AliasDef { ast: alias, module });
                let id = AliasId(self.defs.aliases.len() as u32 - 1);
                self.define(module, &alias.name, Def::Alias(id))
            }
            ItemKind::Mod(inner) => {
                let id = self.new_module(&inner.name.name, Some(module));
                self.define(module, &inner.name, Def::Mod(id))?;
                self.collect_items(id, &inner.items)
            }
            ItemKind::Use(tree) => {
                self.collect_use(module, tree, Vec::new(), item.is_pub);
                Ok(())
            }
        }
    }

    fn add_fn(
        &mut self,
        function: &'a ast::Function,
        module: ModId,
        owner: FnOwner,
        kind: FnKind,
        path: String,
    ) -> Result<FnId> {
        self.defs.fns.push(FnDef { ast: function, module, owner, kind, path });
        let id = FnId(self.defs.fns.len() as u32 - 1);
        self.collect_nested_items(module, function)?;
        Ok(id)
    }

    /// Items declared inside a function body belong to the enclosing module.
    fn collect_nested_items(&mut self, module: ModId, function: &'a ast::Function) -> Result<()> {
        let Some(body) = &function.body else { return Ok(()) };
        let mut nested = Vec::new();
        visit::nested_items(body, &mut |item| nested.push(item));
        nested.into_iter().try_for_each(|item| self.collect_item(module, item))
    }

    fn add_adt(
        &mut self,
        module: ModId,
        name: &ast::Ident,
        kind: AdtKind,
        generics: &'a ast::Generics,
        variants: Vec<VariantDef<'a>>,
        item: &ast::Item,
    ) -> Result<()> {
        let never_zero = item.attrs.iter().any(|attr| attr.name == "rustc_nonnull_optimization_guaranteed");
        let span = item.span;
        self.defs.adts.push(AdtDef { name: name.name.clone(), module, kind, generics, variants, never_zero, span });
        let id = AdtId(self.defs.adts.len() as u32 - 1);
        self.define(module, name, Def::Adt(id))
    }

    fn add_trait(&mut self, module: ModId, def: &'a ast::TraitDef) -> Result<()> {
        let id = TraitId(self.defs.traits.len() as u32);
        self.defs.traits.push(TraitDef {
            name: def.name.name.clone(),
            module,
            ast: def,
            methods: HashMap::new(),
            assoc_types: Vec::new(),
            consts: HashMap::new(),
        });
        self.define(module, &def.name, Def::Trait(id))?;

        for item in &def.items {
            match item {
                AssocItem::Fn(function) => {
                    let kind =
                        if function.body.is_some() { FnKind::Defined } else { FnKind::Required };
                    let path = format!("{}::{}", def.name.name, function.name.name);
                    let fn_id = self.add_fn(function, module, FnOwner::Trait(id), kind, path)?;
                    self.defs.traits[id.0 as usize].methods.insert(function.name.name.clone(), fn_id);
                }
                AssocItem::Type { name, .. } => {
                    self.defs.traits[id.0 as usize].assoc_types.push(name.name.clone());
                }
                AssocItem::Const(constant) => {
                    self.defs.consts.push(ConstDef {
                        ast: constant,
                        module,
                        owner: None,
                        trait_owner: Some(id),
                        is_static: false,
                    });
                    let const_id = ConstId(self.defs.consts.len() as u32 - 1);
                    self.defs.traits[id.0 as usize].consts.insert(constant.name.name.clone(), const_id);
                }
            }
        }
        Ok(())
    }

    /// Flatten a `use` tree into one pending import per leaf.
    fn collect_use(&mut self, module: ModId, tree: &'a ast::UseTree, mut path: Vec<&'a ast::Ident>, is_pub: bool) {
        path.extend(&tree.prefix);
        match &tree.kind {
            UseKind::Simple(alias) => {
                // `use a::b::{self}` imports `b` itself.
                if path.last().is_some_and(|last| last.name == "self") {
                    path.pop();
                }
                let Some(&last) = path.last() else { return };
                let binding = Some(alias.as_ref().unwrap_or(last));
                self.imports.push(PendingImport { module, path, binding, is_pub });
            }
            UseKind::Glob => self.imports.push(PendingImport { module, path, binding: None, is_pub }),
            UseKind::Nested(children) => {
                for child in children {
                    self.collect_use(module, child, path.clone(), is_pub);
                }
            }
        }
    }

    /// Resolve imports to a fixed point: one `use` may depend on a name that
    /// another `use` brings in.
    fn resolve_imports(&mut self) -> Result<()> {
        let mut pending = std::mem::take(&mut self.imports);
        loop {
            let before = pending.len();
            let mut unresolved = Vec::new();
            for import in pending {
                if !self.try_import(&import)? {
                    unresolved.push(import);
                }
            }
            pending = unresolved;
            if pending.is_empty() {
                return Ok(());
            }
            if pending.len() == before {
                let import = &pending[0];
                let text: Vec<&str> = import.path.iter().map(|ident| ident.name.as_str()).collect();
                let span = import.path.last().map_or(Span::default(), |ident| ident.span);
                return Err(Diagnostic::new(span, format!("unresolved import `{}`", text.join("::"))));
            }
        }
    }

    fn try_import(&mut self, import: &PendingImport<'a>) -> Result<bool> {
        let segments = import
            .path
            .iter()
            .map(|&ident| ast::PathSegment { ident: ident.clone(), args: Vec::new(), bindings: Vec::new() })
            .collect();
        let path = ast::Path::new(segments, Span::default());
        let Some(Resolution { def, rest: [] }) = self.defs.resolve_path(import.module, &path) else {
            return Ok(false);
        };
        self.defs.check_visible(import.module, &path)?;

        let imported: Vec<(String, Def)> = match (import.binding, def) {
            (Some(binding), _) => vec![(binding.name.clone(), def)],
            // A glob brings in what the importing module may see.
            (None, Def::Mod(source)) => {
                let source_module = self.defs.module(source);
                let visible = |name: &String| source_module.public.contains(name) || self.defs.sees_private(source, import.module);
                source_module.names.iter().filter(|(name, _)| visible(name)).map(|(name, def)| (name.clone(), *def)).collect()
            }
            (None, Def::Adt(adt)) => {
                let variants = self.defs.adt(adt).variants.iter().enumerate();
                variants.map(|(i, v)| (v.name.clone(), Def::Variant(adt, i as u32))).collect()
            }
            (None, _) => return Ok(false),
        };
        let module = &mut self.defs.modules[import.module.0 as usize];
        for (name, def) in imported {
            if name != "_" {
                if import.is_pub {
                    module.public.insert(name.clone());
                }
                // A module's own declarations win over what a glob brings in.
                module.names.entry(name).or_insert(def);
            }
        }
        Ok(true)
    }

    fn collect_impls(&mut self) -> Result<()> {
        for (module, block) in std::mem::take(&mut self.pending_impls) {
            let trait_id = match &block.trait_ref {
                None => None,
                Some(path) => match self.defs.resolve_path(module, path) {
                    Some(Resolution { def: Def::Trait(id), rest: [] }) => Some(id),
                    _ => bail!(path.span, "cannot find trait `{}`", path.last().ident.name),
                },
            };
            let id = ImplId(self.defs.impls.len() as u32);
            let type_name = match &block.self_ty.kind {
                ast::TypeKind::Path(path) => path.last().ident.name.clone(),
                _ => "impl".to_string(),
            };
            let mut def = ImplDef {
                module,
                ast: block,
                trait_id,
                methods: HashMap::new(),
                assoc_types: HashMap::new(),
                consts: HashMap::new(),
            };

            for item in &block.items {
                let member = match item {
                    AssocItem::Fn(function) => {
                        let path = self.qualified(module, &format!("{type_name}::{}", function.name.name));
                        self.defs.fns.push(FnDef {
                            ast: function,
                            module,
                            owner: FnOwner::Impl(id),
                            kind: if function.body.is_some() { FnKind::Defined } else { FnKind::Intrinsic },
                            path,
                        });
                        let fn_id = FnId(self.defs.fns.len() as u32 - 1);
                        if def.methods.insert(function.name.name.clone(), fn_id).is_some() {
                            bail!(function.name.span, "duplicate definition of `{}`", function.name.name);
                        }
                        &function.name
                    }
                    AssocItem::Type { name, ty } => {
                        let Some(ty) = ty else {
                            bail!(name.span, "associated type `{}` needs a value", name.name);
                        };
                        def.assoc_types.insert(name.name.clone(), ty);
                        continue;
                    }
                    AssocItem::Const(constant) => {
                        self.defs.consts.push(ConstDef {
                            ast: constant,
                            module,
                            owner: Some(id),
                            trait_owner: None,
                            is_static: false,
                        });
                        let const_id = ConstId(self.defs.consts.len() as u32 - 1);
                        def.consts.insert(constant.name.name.clone(), const_id);
                        &constant.name
                    }
                };
                self.defs.impls_by_member.entry(member.name.clone()).or_default().push(id);
            }
            self.defs.impls.push(def);
        }
        Ok(())
    }
}

fn variant_def<'a>(
    name: String,
    fields: &'a VariantFields,
    discriminant: Option<&'a ast::Expr>,
) -> VariantDef<'a> {
    let (shape, fields) = match fields {
        VariantFields::Unit => (VariantShape::Unit, Vec::new()),
        VariantFields::Tuple(types) => {
            let fields = types.iter().enumerate();
            (VariantShape::Tuple, fields.map(|(i, ty)| FieldDef { name: i.to_string(), ty, is_pub: true }).collect())
        }
        VariantFields::Named(named) => {
            let fields = named.iter().map(|f| FieldDef { name: f.name.name.clone(), ty: &f.ty, is_pub: true });
            (VariantShape::Named, fields.collect())
        }
    };
    VariantDef { name, shape, fields, discriminant }
}
