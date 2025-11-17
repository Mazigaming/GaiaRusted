//! # Derive Macros Support (Phase 12+)
//!
//! Automatic trait implementation generation:
//! - #[derive(Debug)]
//! - #[derive(Clone)]
//! - #[derive(Copy)]
//! - #[derive(Default)]
//! - #[derive(PartialEq, Eq)]
//! - Custom derives

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeriveError {
    UnknownDerive(String),
    InvalidDerive(String),
    CircularDerive(String),
    MissingField(String),
    UnsupportedType(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeriveKind {
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Display,
}

impl std::fmt::Display for DeriveKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeriveKind::Debug => write!(f, "Debug"),
            DeriveKind::Clone => write!(f, "Clone"),
            DeriveKind::Copy => write!(f, "Copy"),
            DeriveKind::Default => write!(f, "Default"),
            DeriveKind::PartialEq => write!(f, "PartialEq"),
            DeriveKind::Eq => write!(f, "Eq"),
            DeriveKind::PartialOrd => write!(f, "PartialOrd"),
            DeriveKind::Ord => write!(f, "Ord"),
            DeriveKind::Hash => write!(f, "Hash"),
            DeriveKind::Display => write!(f, "Display"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: String,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DerivableStruct {
    pub name: String,
    pub fields: Vec<StructField>,
    pub derives: Vec<DeriveKind>,
    pub generic_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GeneratedImpl {
    pub trait_name: String,
    pub struct_name: String,
    pub code: String,
}

pub struct DeriveEngine {
    derives: HashMap<String, DeriveKind>,
    generated_impls: HashMap<String, GeneratedImpl>,
    circular_check: Vec<String>,
}

impl DeriveEngine {
    pub fn new() -> Self {
        let mut derives = HashMap::new();
        derives.insert("Debug".to_string(), DeriveKind::Debug);
        derives.insert("Clone".to_string(), DeriveKind::Clone);
        derives.insert("Copy".to_string(), DeriveKind::Copy);
        derives.insert("Default".to_string(), DeriveKind::Default);
        derives.insert("PartialEq".to_string(), DeriveKind::PartialEq);
        derives.insert("Eq".to_string(), DeriveKind::Eq);
        derives.insert("PartialOrd".to_string(), DeriveKind::PartialOrd);
        derives.insert("Ord".to_string(), DeriveKind::Ord);
        derives.insert("Hash".to_string(), DeriveKind::Hash);
        derives.insert("Display".to_string(), DeriveKind::Display);

        DeriveEngine {
            derives,
            generated_impls: HashMap::new(),
            circular_check: Vec::new(),
        }
    }

    pub fn parse_derive_kinds(
        &self,
        derive_list: &[String],
    ) -> Result<Vec<DeriveKind>, DeriveError> {
        let mut kinds = Vec::new();
        for derive_name in derive_list {
            let kind = self
                .derives
                .get(derive_name)
                .cloned()
                .ok_or_else(|| DeriveError::UnknownDerive(derive_name.clone()))?;
            kinds.push(kind);
        }
        Ok(kinds)
    }

    pub fn generate_impl(
        &mut self,
        struct_def: &DerivableStruct,
    ) -> Result<Vec<GeneratedImpl>, DeriveError> {
        let mut impls = Vec::new();

        self.circular_check.clear();

        for derive_kind in &struct_def.derives {
            let impl_code = match derive_kind {
                DeriveKind::Debug => self.generate_debug(struct_def)?,
                DeriveKind::Clone => self.generate_clone(struct_def)?,
                DeriveKind::Copy => self.generate_copy(struct_def)?,
                DeriveKind::Default => self.generate_default(struct_def)?,
                DeriveKind::PartialEq => self.generate_partial_eq(struct_def)?,
                DeriveKind::Eq => self.generate_eq(struct_def)?,
                DeriveKind::PartialOrd => self.generate_partial_ord(struct_def)?,
                DeriveKind::Ord => self.generate_ord(struct_def)?,
                DeriveKind::Hash => self.generate_hash(struct_def)?,
                DeriveKind::Display => self.generate_display(struct_def)?,
            };

            let key = format!("{}_{}", struct_def.name, derive_kind);
            impls.push(impl_code.clone());
            self.generated_impls.insert(key, impl_code);
        }

        Ok(impls)
    }

    fn generate_debug(&self, struct_def: &DerivableStruct) -> Result<GeneratedImpl, DeriveError> {
        let mut field_debug = Vec::new();
        for field in &struct_def.fields {
            field_debug.push(format!(".field(\"{}\", &self.{})", field.name, field.name));
        }

        let code = format!(
            "impl Debug for {} {{\n    fn fmt(&self, f: &mut Formatter) -> Result {{\n        f.debug_struct(\"{}\"){}.finish()\n    }}\n}}",
            struct_def.name,
            struct_def.name,
            field_debug.join("")
        );

        Ok(GeneratedImpl {
            trait_name: "Debug".to_string(),
            struct_name: struct_def.name.clone(),
            code,
        })
    }

    fn generate_clone(&self, struct_def: &DerivableStruct) -> Result<GeneratedImpl, DeriveError> {
        let mut field_clones = Vec::new();
        for field in &struct_def.fields {
            field_clones.push(format!("{}: self.{}.clone()", field.name, field.name));
        }

        let code = format!(
            "impl Clone for {} {{\n    fn clone(&self) -> Self {{\n        {} {{\n            {}\n        }}\n    }}\n}}",
            struct_def.name,
            struct_def.name,
            field_clones.join(",\n            ")
        );

        Ok(GeneratedImpl {
            trait_name: "Clone".to_string(),
            struct_name: struct_def.name.clone(),
            code,
        })
    }

    fn generate_copy(&self, struct_def: &DerivableStruct) -> Result<GeneratedImpl, DeriveError> {
        let code = format!("impl Copy for {} {{}}", struct_def.name);

        Ok(GeneratedImpl {
            trait_name: "Copy".to_string(),
            struct_name: struct_def.name.clone(),
            code,
        })
    }

    fn generate_default(
        &self,
        struct_def: &DerivableStruct,
    ) -> Result<GeneratedImpl, DeriveError> {
        let mut field_defaults = Vec::new();
        for field in &struct_def.fields {
            field_defaults.push(format!("{}: Default::default()", field.name));
        }

        let code = format!(
            "impl Default for {} {{\n    fn default() -> Self {{\n        {} {{\n            {}\n        }}\n    }}\n}}",
            struct_def.name,
            struct_def.name,
            field_defaults.join(",\n            ")
        );

        Ok(GeneratedImpl {
            trait_name: "Default".to_string(),
            struct_name: struct_def.name.clone(),
            code,
        })
    }

    fn generate_partial_eq(
        &self,
        struct_def: &DerivableStruct,
    ) -> Result<GeneratedImpl, DeriveError> {
        let mut field_eqs = Vec::new();
        for field in &struct_def.fields {
            field_eqs.push(format!("self.{} == other.{}", field.name, field.name));
        }

        let code = format!(
            "impl PartialEq for {} {{\n    fn eq(&self, other: &Self) -> bool {{\n        {}\n    }}\n}}",
            struct_def.name,
            if field_eqs.is_empty() {
                "true".to_string()
            } else {
                field_eqs.join(" && ")
            }
        );

        Ok(GeneratedImpl {
            trait_name: "PartialEq".to_string(),
            struct_name: struct_def.name.clone(),
            code,
        })
    }

    fn generate_eq(&self, struct_def: &DerivableStruct) -> Result<GeneratedImpl, DeriveError> {
        let code = format!("impl Eq for {} {{}}", struct_def.name);

        Ok(GeneratedImpl {
            trait_name: "Eq".to_string(),
            struct_name: struct_def.name.clone(),
            code,
        })
    }

    fn generate_partial_ord(
        &self,
        struct_def: &DerivableStruct,
    ) -> Result<GeneratedImpl, DeriveError> {
        let code = format!(
            "impl PartialOrd for {} {{\n    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {{\n        Some(self.cmp(other))\n    }}\n}}",
            struct_def.name
        );

        Ok(GeneratedImpl {
            trait_name: "PartialOrd".to_string(),
            struct_name: struct_def.name.clone(),
            code,
        })
    }

    fn generate_ord(
        &self,
        struct_def: &DerivableStruct,
    ) -> Result<GeneratedImpl, DeriveError> {
        let code = format!(
            "impl Ord for {} {{\n    fn cmp(&self, other: &Self) -> Ordering {{\n        unimplemented!()\n    }}\n}}",
            struct_def.name
        );

        Ok(GeneratedImpl {
            trait_name: "Ord".to_string(),
            struct_name: struct_def.name.clone(),
            code,
        })
    }

    fn generate_hash(&self, struct_def: &DerivableStruct) -> Result<GeneratedImpl, DeriveError> {
        let mut field_hashes = Vec::new();
        for field in &struct_def.fields {
            field_hashes.push(format!("self.{}.hash(state);", field.name));
        }

        let code = format!(
            "impl Hash for {} {{\n    fn hash<H: Hasher>(&self, state: &mut H) {{\n        {}\n    }}\n}}",
            struct_def.name,
            field_hashes.join("\n        ")
        );

        Ok(GeneratedImpl {
            trait_name: "Hash".to_string(),
            struct_name: struct_def.name.clone(),
            code,
        })
    }

    fn generate_display(
        &self,
        struct_def: &DerivableStruct,
    ) -> Result<GeneratedImpl, DeriveError> {
        let code = format!(
            "impl Display for {} {{\n    fn fmt(&self, f: &mut Formatter) -> Result {{\n        write!(f, \"{}\")\n    }}\n}}",
            struct_def.name, struct_def.name
        );

        Ok(GeneratedImpl {
            trait_name: "Display".to_string(),
            struct_name: struct_def.name.clone(),
            code,
        })
    }

    pub fn validate_derives(
        &self,
        struct_def: &DerivableStruct,
    ) -> Result<(), DeriveError> {
        for derive in &struct_def.derives {
            match derive {
                DeriveKind::Copy => {
                    if struct_def.derives.contains(&DeriveKind::Clone) {
                    } else {
                        return Err(DeriveError::InvalidDerive(
                            "Copy requires Clone to be derived".to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn get_generated_impl(&self, struct_name: &str, trait_name: &str) -> Option<&GeneratedImpl> {
        let key = format!("{}_{}", struct_name, trait_name);
        self.generated_impls.get(&key)
    }
}

impl Default for DeriveEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_struct() -> DerivableStruct {
        DerivableStruct {
            name: "Point".to_string(),
            fields: vec![
                StructField {
                    name: "x".to_string(),
                    ty: "i32".to_string(),
                    attributes: vec![],
                },
                StructField {
                    name: "y".to_string(),
                    ty: "i32".to_string(),
                    attributes: vec![],
                },
            ],
            derives: vec![DeriveKind::Debug],
            generic_params: vec![],
        }
    }

    #[test]
    fn test_parse_debug_derive() {
        let engine = DeriveEngine::new();
        let kinds = engine.parse_derive_kinds(&["Debug".to_string()]).unwrap();
        assert_eq!(kinds.len(), 1);
        assert_eq!(kinds[0], DeriveKind::Debug);
    }

    #[test]
    fn test_parse_multiple_derives() {
        let engine = DeriveEngine::new();
        let kinds = engine
            .parse_derive_kinds(&["Debug".to_string(), "Clone".to_string()])
            .unwrap();
        assert_eq!(kinds.len(), 2);
    }

    #[test]
    fn test_parse_unknown_derive() {
        let engine = DeriveEngine::new();
        let result = engine.parse_derive_kinds(&["Unknown".to_string()]);
        assert!(matches!(result, Err(DeriveError::UnknownDerive(_))));
    }

    #[test]
    fn test_generate_debug() {
        let mut engine = DeriveEngine::new();
        let struct_def = create_test_struct();
        let result = engine.generate_impl(&struct_def).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].trait_name, "Debug");
        assert!(result[0].code.contains("fn fmt"));
    }

    #[test]
    fn test_generate_clone() {
        let mut engine = DeriveEngine::new();
        let mut struct_def = create_test_struct();
        struct_def.derives = vec![DeriveKind::Clone];
        let result = engine.generate_impl(&struct_def).unwrap();
        assert_eq!(result[0].trait_name, "Clone");
        assert!(result[0].code.contains("fn clone"));
    }

    #[test]
    fn test_generate_copy() {
        let mut engine = DeriveEngine::new();
        let mut struct_def = create_test_struct();
        struct_def.derives = vec![DeriveKind::Copy];
        let result = engine.generate_impl(&struct_def).unwrap();
        assert_eq!(result[0].trait_name, "Copy");
    }

    #[test]
    fn test_generate_default() {
        let mut engine = DeriveEngine::new();
        let mut struct_def = create_test_struct();
        struct_def.derives = vec![DeriveKind::Default];
        let result = engine.generate_impl(&struct_def).unwrap();
        assert_eq!(result[0].trait_name, "Default");
        assert!(result[0].code.contains("fn default"));
    }

    #[test]
    fn test_generate_partial_eq() {
        let mut engine = DeriveEngine::new();
        let mut struct_def = create_test_struct();
        struct_def.derives = vec![DeriveKind::PartialEq];
        let result = engine.generate_impl(&struct_def).unwrap();
        assert_eq!(result[0].trait_name, "PartialEq");
        assert!(result[0].code.contains("fn eq"));
    }

    #[test]
    fn test_generate_eq() {
        let mut engine = DeriveEngine::new();
        let mut struct_def = create_test_struct();
        struct_def.derives = vec![DeriveKind::Eq];
        let result = engine.generate_impl(&struct_def).unwrap();
        assert_eq!(result[0].trait_name, "Eq");
    }

    #[test]
    fn test_validate_copy_without_clone() {
        let mut engine = DeriveEngine::new();
        let mut struct_def = create_test_struct();
        struct_def.derives = vec![DeriveKind::Copy];
        let result = engine.validate_derives(&struct_def);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_copy_with_clone() {
        let mut engine = DeriveEngine::new();
        let mut struct_def = create_test_struct();
        struct_def.derives = vec![DeriveKind::Clone, DeriveKind::Copy];
        let result = engine.validate_derives(&struct_def);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_derives() {
        let mut engine = DeriveEngine::new();
        let mut struct_def = create_test_struct();
        struct_def.derives = vec![
            DeriveKind::Debug,
            DeriveKind::Clone,
            DeriveKind::PartialEq,
        ];
        let result = engine.generate_impl(&struct_def).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_get_generated_impl() {
        let mut engine = DeriveEngine::new();
        let struct_def = create_test_struct();
        engine.generate_impl(&struct_def).ok();
        let impl_code = engine.get_generated_impl("Point", "Debug");
        assert!(impl_code.is_some());
    }

    #[test]
    fn test_generate_hash() {
        let mut engine = DeriveEngine::new();
        let mut struct_def = create_test_struct();
        struct_def.derives = vec![DeriveKind::Hash];
        let result = engine.generate_impl(&struct_def).unwrap();
        assert_eq!(result[0].trait_name, "Hash");
        assert!(result[0].code.contains("fn hash"));
    }

    #[test]
    fn test_generate_display() {
        let mut engine = DeriveEngine::new();
        let mut struct_def = create_test_struct();
        struct_def.derives = vec![DeriveKind::Display];
        let result = engine.generate_impl(&struct_def).unwrap();
        assert_eq!(result[0].trait_name, "Display");
        assert!(result[0].code.contains("fn fmt"));
    }

    #[test]
    fn test_empty_struct_derives() {
        let mut engine = DeriveEngine::new();
        let mut struct_def = create_test_struct();
        struct_def.fields = vec![];
        struct_def.derives = vec![DeriveKind::Debug];
        let result = engine.generate_impl(&struct_def).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_derive_kind_display() {
        assert_eq!(DeriveKind::Debug.to_string(), "Debug");
        assert_eq!(DeriveKind::Clone.to_string(), "Clone");
        assert_eq!(DeriveKind::PartialEq.to_string(), "PartialEq");
    }
}
