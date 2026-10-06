//! `impl Trait` in argument position.
//!
//! A parameter of type `impl Trait` is a generic parameter without a name,
//! and that is how it is rewritten, before anything else looks at the
//! function:
//!
//! ```text
//! fn show(items: impl IntoIterator<Item = impl Display>)
//! fn show<impl#0: Display, impl#1: IntoIterator<Item = impl#0>>(items: impl#1)
//! ```
//!
//! Every occurrence, however deeply nested (`&impl Animal`,
//! `Vec<impl Fn(i32)>`), gets its own parameter. The names cannot be
//! written in source, so they clash with nothing, and explicit type
//! arguments (`f::<T>(..)`) still go to the declared parameters, which
//! come first.

use crate::syntax::ast::*;

/// Rewrite every function among `items`, through modules, impls and traits.
pub fn desugar(items: &mut [Item]) {
    for item in items {
        match &mut item.kind {
            ItemKind::Fn(function) => desugar_fn(function),
            ItemKind::Impl(block) => desugar_assoc(&mut block.items),
            ItemKind::Trait(def) => desugar_assoc(&mut def.items),
            ItemKind::Mod(module) => desugar(&mut module.items),
            _ => {}
        }
    }
}

fn desugar_assoc(items: &mut [AssocItem]) {
    for item in items {
        if let AssocItem::Fn(function) = item {
            desugar_fn(function);
        }
    }
}

/// Is this the name of a parameter that stands for an `impl Trait`?
pub fn is_anonymous(name: &str) -> bool {
    name.starts_with("impl#")
}

fn desugar_fn(function: &mut Function) {
    let mut anonymous = Vec::new();
    for param in &mut function.params {
        replace_impl_traits(&mut param.ty, &mut anonymous);
    }
    function.generics.params.extend(anonymous);
}

/// Replace each `impl Bounds` in `ty` by a new parameter with those bounds.
fn replace_impl_traits(ty: &mut Type, anonymous: &mut Vec<GenericParam>) {
    match &mut ty.kind {
        TypeKind::ImplTrait(bounds) => {
            let mut bounds = std::mem::take(bounds);
            for bound in &mut bounds {
                replace_in_bound(bound, anonymous);
            }
            let name = Ident { name: format!("impl#{}", anonymous.len()), span: ty.span };
            let path = Path::new(vec![PathSegment { ident: name.clone(), args: Vec::new(), bindings: Vec::new() }], ty.span);
            ty.kind = TypeKind::Path(path);
            anonymous.push(GenericParam { name, bounds, const_ty: None, maybe_unsized: false });
        }
        TypeKind::Path(path) => replace_in_path(path, anonymous),
        TypeKind::Ref { inner, .. } | TypeKind::Ptr { inner, .. } | TypeKind::Slice(inner) | TypeKind::Array(inner, _) => {
            replace_impl_traits(inner, anonymous);
        }
        TypeKind::Tuple(elements) => elements.iter_mut().for_each(|element| replace_impl_traits(element, anonymous)),
        TypeKind::Fn { .. } | TypeKind::Dyn(_) | TypeKind::Never | TypeKind::Infer | TypeKind::Const(_) => {}
    }
}

fn replace_in_path(path: &mut Path, anonymous: &mut Vec<GenericParam>) {
    for segment in &mut path.segments {
        segment.args.iter_mut().for_each(|arg| replace_impl_traits(arg, anonymous));
        segment.bindings.iter_mut().for_each(|(_, ty)| replace_impl_traits(ty, anonymous));
    }
}

fn replace_in_bound(bound: &mut Bound, anonymous: &mut Vec<GenericParam>) {
    replace_in_path(&mut bound.path, anonymous);
    if let Some((params, ret)) = &mut bound.fn_sugar {
        params.iter_mut().for_each(|param| replace_impl_traits(param, anonymous));
        if let Some(ret) = ret {
            replace_impl_traits(ret, anonymous);
        }
    }
}
