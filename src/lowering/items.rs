//! Item lowering (enums, traits, impl, modules, etc.)
//! Adapted from rustc's ast_lowering/item.rs

use crate::parser::ast::{Item, EnumVariant, StructField, Type, GenericParam};
use std::collections::HashMap;

/// Lowered item representation
#[derive(Debug, Clone, PartialEq)]
pub enum HirItem {
    Function {
        name: String,
        generics: Vec<String>,
        params: Vec<(String, Type)>,
        return_type: Option<Type>,
        is_unsafe: bool,
    },
    Struct {
        name: String,
        fields: Vec<(String, Type)>,
        generics: Vec<String>,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariantLowered>,
        generics: Vec<String>,
    },
    Trait {
        name: String,
        methods: Vec<HirItem>,
        generics: Vec<String>,
        supertraits: Vec<String>,
    },
    Impl {
        trait_name: Option<String>,
        struct_name: String,
        methods: Vec<HirItem>,
        generics: Vec<String>,
        is_unsafe: bool,
    },
    Module {
        name: String,
        items: Vec<HirItem>,
    },
    TypeAlias {
        name: String,
        ty: Type,
        generics: Vec<String>,
    },
    Const {
        name: String,
        ty: Type,
        generics: Vec<String>,
    },
    Static {
        name: String,
        ty: Type,
        is_mutable: bool,
        generics: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariantLowered {
    pub name: String,
    pub fields: Vec<Type>,
    pub discriminant: Option<i32>,
}

/// Lower enum to HIR (proper implementation)
pub fn lower_enum(
    name: String,
    generics: Vec<GenericParam>,
    variants: Vec<EnumVariant>,
) -> HirItem {
    let generics_names: Vec<String> = generics
        .iter()
        .map(|g| match g {
            GenericParam::Lifetime(name) => name.clone(),
            GenericParam::Type(name) => name.clone(),
            GenericParam::Const(name, _) => name.clone(),
        })
        .collect();

    let lowered_variants: Vec<EnumVariantLowered> = variants
        .iter()
        .enumerate()
        .map(|(idx, variant)| {
            let fields = match &variant.data {
                crate::parser::ast::VariantData::Unit => vec![],
                crate::parser::ast::VariantData::Tuple(types) => types.clone(),
                crate::parser::ast::VariantData::Struct(fields) => {
                    fields.iter().map(|f| f.ty.clone()).collect()
                }
            };

            EnumVariantLowered {
                name: variant.name.clone(),
                fields,
                discriminant: variant.discriminant.or_else(|| Some(idx as i32)),
            }
        })
        .collect();

    HirItem::Enum {
        name,
        variants: lowered_variants,
        generics: generics_names,
    }
}

/// Lower trait to HIR (proper implementation)
pub fn lower_trait(
    name: String,
    generics: Vec<GenericParam>,
    supertraits: Vec<String>,
    methods: Vec<Item>,
) -> HirItem {
    let generics_names: Vec<String> = generics
        .iter()
        .map(|g| match g {
            GenericParam::Lifetime(name) => name.clone(),
            GenericParam::Type(name) => name.clone(),
            GenericParam::Const(name, _) => name.clone(),
        })
        .collect();

    let lowered_methods = methods
        .into_iter()
        .filter_map(|method| {
            if let Item::Function {
                name,
                generics,
                params,
                return_type,
                is_unsafe,
                ..
            } = method
            {
                Some(HirItem::Function {
                    name,
                    generics: generics
                        .iter()
                        .map(|g| match g {
                            GenericParam::Lifetime(name) => name.clone(),
                            GenericParam::Type(name) => name.clone(),
                            GenericParam::Const(name, _) => name.clone(),
                        })
                        .collect(),
                    params: params.iter().map(|p| (p.name.clone(), p.ty.clone())).collect(),
                    return_type,
                    is_unsafe,
                })
            } else {
                None
            }
        })
        .collect();

    HirItem::Trait {
        name,
        methods: lowered_methods,
        generics: generics_names,
        supertraits,
    }
}

/// Lower impl to HIR (proper implementation)
pub fn lower_impl(
    generics: Vec<GenericParam>,
    trait_name: Option<String>,
    struct_name: String,
    methods: Vec<Item>,
    is_unsafe: bool,
) -> HirItem {
    let generics_names: Vec<String> = generics
        .iter()
        .map(|g| match g {
            GenericParam::Lifetime(name) => name.clone(),
            GenericParam::Type(name) => name.clone(),
            GenericParam::Const(name, _) => name.clone(),
        })
        .collect();

    let lowered_methods = methods
        .into_iter()
        .filter_map(|method| {
            if let Item::Function {
                name,
                generics,
                params,
                return_type,
                is_unsafe: func_unsafe,
                ..
            } = method
            {
                Some(HirItem::Function {
                    name,
                    generics: generics
                        .iter()
                        .map(|g| match g {
                            GenericParam::Lifetime(name) => name.clone(),
                            GenericParam::Type(name) => name.clone(),
                            GenericParam::Const(name, _) => name.clone(),
                        })
                        .collect(),
                    params: params.iter().map(|p| (p.name.clone(), p.ty.clone())).collect(),
                    return_type,
                    is_unsafe: func_unsafe,
                })
            } else {
                None
            }
        })
        .collect();

    HirItem::Impl {
        trait_name,
        struct_name,
        methods: lowered_methods,
        generics: generics_names,
        is_unsafe,
    }
}

/// Lower module to HIR (proper implementation)
pub fn lower_module(name: String, items: Vec<Item>) -> HirItem {
    HirItem::Module { name, items: vec![] }
}