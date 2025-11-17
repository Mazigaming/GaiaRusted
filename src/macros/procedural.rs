use std::collections::HashMap;
use crate::parser::ast::{Item, Attribute, EnumVariant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeriveMacro {
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
}

impl DeriveMacro {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Debug" => Some(DeriveMacro::Debug),
            "Clone" => Some(DeriveMacro::Clone),
            "Copy" => Some(DeriveMacro::Copy),
            "PartialEq" => Some(DeriveMacro::PartialEq),
            "Eq" => Some(DeriveMacro::Eq),
            "PartialOrd" => Some(DeriveMacro::PartialOrd),
            "Ord" => Some(DeriveMacro::Ord),
            "Hash" => Some(DeriveMacro::Hash),
            "Default" => Some(DeriveMacro::Default),
            _ => None,
        }
    }
}

pub struct ProceduralMacroProcessor {
    derives: HashMap<String, Vec<DeriveMacro>>,
}

impl ProceduralMacroProcessor {
    pub fn new() -> Self {
        ProceduralMacroProcessor {
            derives: HashMap::new(),
        }
    }

    pub fn process_derive_attribute(&mut self, item: &Item, attribute: &Attribute) -> Result<Vec<String>, String> {
        if attribute.name != "derive" {
            return Ok(Vec::new());
        }

        let derives: Vec<DeriveMacro> = attribute
            .args
            .iter()
            .filter_map(|name| DeriveMacro::from_name(name.trim()))
            .collect();

        if derives.is_empty() {
            return Err("No valid derive macros specified".to_string());
        }

        let mut generated_code = Vec::new();

        for derive in derives {
            match derive {
                DeriveMacro::Debug => {
                    generated_code.push(self.generate_debug_impl(item)?);
                }
                DeriveMacro::Clone => {
                    generated_code.push(self.generate_clone_impl(item)?);
                }
                DeriveMacro::Copy => {
                    generated_code.push(self.generate_copy_impl(item)?);
                }
                DeriveMacro::PartialEq => {
                    generated_code.push(self.generate_partial_eq_impl(item)?);
                }
                DeriveMacro::Eq => {
                    generated_code.push(self.generate_eq_impl(item)?);
                }
                DeriveMacro::Default => {
                    generated_code.push(self.generate_default_impl(item)?);
                }
                _ => {
                    return Err(format!("Derive macro {:?} not yet implemented", derive));
                }
            }
        }

        Ok(generated_code)
    }

    fn generate_debug_impl(&self, item: &Item) -> Result<String, String> {
        match item {
            Item::Struct {
                name,
                fields,
                ..
            } => {
                Ok(format!(
                    "impl std::fmt::Debug for {} {{\n  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{\n    f.debug_struct(\"{}\")\n{}\n      .finish()\n  }}\n}}",
                    name,
                    name,
                    fields.iter()
                        .map(|f| format!("      .field(\"{}\", &self.{})", f.name, f.name))
                        .collect::<Vec<_>>()
                        .join("\n")
                ))
            }
            Item::Enum { name, variants, .. } => {
                let variant_patterns = variants
                    .iter()
                    .map(|v| match v {
                        EnumVariant::Unit(v_name) => format!("{}::{} => write!(f, \"{}\")", name, v_name, v_name),
                        EnumVariant::Tuple(v_name, _) => format!("{}::{}(..) => write!(f, \"{}(..)\")", name, v_name, v_name),
                        EnumVariant::Struct(v_name, _) => format!("{}::{}{{\n          ..\n        }} => write!(f, \"{}{{ .. }}\")", name, v_name, v_name),
                    })
                    .collect::<Vec<_>>()
                    .join(",\n      ");

                Ok(format!(
                    "impl std::fmt::Debug for {} {{\n  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{\n    match self {{\n      {}\n    }}\n  }}\n}}",
                    name,
                    variant_patterns
                ))
            }
            _ => Err("Debug can only be derived for structs and enums".to_string()),
        }
    }

    fn generate_clone_impl(&self, item: &Item) -> Result<String, String> {
        match item {
            Item::Struct { name, fields, .. } => {
                let field_clones = fields
                    .iter()
                    .map(|f| format!("{}: self.{}.clone()", f.name, f.name))
                    .collect::<Vec<_>>()
                    .join(", ");

                Ok(format!(
                    "impl Clone for {} {{\n  fn clone(&self) -> Self {{\n    {} {{\n      {}\n    }}\n  }}\n}}",
                    name,
                    name,
                    field_clones
                ))
            }
            Item::Enum { name, variants, .. } => {
                let variant_clones = variants
                    .iter()
                    .map(|v| match v {
                        EnumVariant::Unit(v_name) => format!("{}::{} => {}::{}", name, v_name, name, v_name),
                        EnumVariant::Tuple(v_name, _) => format!("{}::{}(ref items) => {}::{}(items.iter().map(|i| i.clone()).collect::<Vec<_>>())", name, v_name, name, v_name),
                        EnumVariant::Struct(v_name, fields) => {
                            let field_clones = fields
                                .iter()
                                .map(|f| format!("{}: ref_{}.clone()", f.name, f.name))
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!("{}::{} {{ {} }} => {}::{} {{ {} }}", name, v_name, fields.iter().map(|f| format!("ref_{}", f.name)).collect::<Vec<_>>().join(", "), name, v_name, field_clones)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(",\n      ");

                Ok(format!(
                    "impl Clone for {} {{\n  fn clone(&self) -> Self {{\n    match self {{\n      {}\n    }}\n  }}\n}}",
                    name,
                    variant_clones
                ))
            }
            _ => Err("Clone can only be derived for structs and enums".to_string()),
        }
    }

    fn generate_copy_impl(&self, item: &Item) -> Result<String, String> {
        match item {
            Item::Struct { name, .. } => {
                Ok(format!("impl Copy for {} {{}}", name))
            }
            Item::Enum { name, .. } => {
                Ok(format!("impl Copy for {} {{}}", name))
            }
            _ => Err("Copy can only be derived for structs and enums".to_string()),
        }
    }

    fn generate_partial_eq_impl(&self, item: &Item) -> Result<String, String> {
        match item {
            Item::Struct { name, fields, .. } => {
                let field_comparisons = fields
                    .iter()
                    .map(|f| format!("self.{} == other.{}", f.name, f.name))
                    .collect::<Vec<_>>()
                    .join(" && ");

                Ok(format!(
                    "impl PartialEq for {} {{\n  fn eq(&self, other: &Self) -> bool {{\n    {}\n  }}\n}}",
                    name,
                    field_comparisons
                ))
            }
            Item::Enum { name, .. } => {
                Ok(format!(
                    "impl PartialEq for {} {{\n  fn eq(&self, other: &Self) -> bool {{\n    std::mem::discriminant(self) == std::mem::discriminant(other)\n  }}\n}}",
                    name
                ))
            }
            _ => Err("PartialEq can only be derived for structs and enums".to_string()),
        }
    }

    fn generate_eq_impl(&self, item: &Item) -> Result<String, String> {
        match item {
            Item::Struct { name, .. } => {
                Ok(format!("impl Eq for {} {{}}", name))
            }
            Item::Enum { name, .. } => {
                Ok(format!("impl Eq for {} {{}}", name))
            }
            _ => Err("Eq can only be derived for structs and enums".to_string()),
        }
    }

    fn generate_default_impl(&self, item: &Item) -> Result<String, String> {
        match item {
            Item::Struct { name, fields, .. } => {
                let field_defaults = fields
                    .iter()
                    .map(|f| format!("{}: Default::default()", f.name))
                    .collect::<Vec<_>>()
                    .join(",\n      ");

                Ok(format!(
                    "impl Default for {} {{\n  fn default() -> Self {{\n    {} {{\n      {}\n    }}\n  }}\n}}",
                    name,
                    name,
                    field_defaults
                ))
            }
            _ => Err("Default can only be derived for structs".to_string()),
        }
    }
}

pub struct AttributeMacroProcessor {
    macros: HashMap<String, AttributeMacroFn>,
}

pub type AttributeMacroFn = fn(&Item, &Attribute) -> Result<String, String>;

impl AttributeMacroProcessor {
    pub fn new() -> Self {
        let mut processor = AttributeMacroProcessor {
            macros: HashMap::new(),
        };
        
        processor.register_standard_attributes();
        processor
    }

    fn register_standard_attributes(&mut self) {
        self.macros.insert("cfg".to_string(), Self::process_cfg);
        self.macros.insert("test".to_string(), Self::process_test);
        self.macros.insert("allow".to_string(), Self::process_allow);
        self.macros.insert("deprecated".to_string(), Self::process_deprecated);
        self.macros.insert("doc".to_string(), Self::process_doc);
    }

    pub fn process_attribute(&self, item: &Item, attribute: &Attribute) -> Result<Option<String>, String> {
        if let Some(processor) = self.macros.get(&attribute.name) {
            let result = processor(item, attribute)?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    fn process_cfg(_item: &Item, attribute: &Attribute) -> Result<String, String> {
        Ok(format!("// #[cfg({})]", attribute.args.join(", ")))
    }

    fn process_test(_item: &Item, _attribute: &Attribute) -> Result<String, String> {
        Ok("// #[test] - test function".to_string())
    }

    fn process_allow(_item: &Item, attribute: &Attribute) -> Result<String, String> {
        Ok(format!("// #[allow({})]", attribute.args.join(", ")))
    }

    fn process_deprecated(_item: &Item, attribute: &Attribute) -> Result<String, String> {
        let reason = attribute.args.first().map(|s| s.as_str()).unwrap_or("deprecated");
        Ok(format!("// #[deprecated(\"{}\")]", reason))
    }

    fn process_doc(_item: &Item, attribute: &Attribute) -> Result<String, String> {
        let doc = attribute.args.join(" ");
        Ok(format!("/// {}", doc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{StructField, Type};

    #[test]
    fn test_derive_macro_from_name() {
        assert_eq!(DeriveMacro::from_name("Debug"), Some(DeriveMacro::Debug));
        assert_eq!(DeriveMacro::from_name("Clone"), Some(DeriveMacro::Clone));
        assert_eq!(DeriveMacro::from_name("Copy"), Some(DeriveMacro::Copy));
        assert_eq!(DeriveMacro::from_name("Unknown"), None);
    }

    #[test]
    fn test_debug_impl_generation() {
        let processor = ProceduralMacroProcessor::new();
        let item = Item::Struct {
            name: "Point".to_string(),
            generics: Vec::new(),
            fields: vec![
                StructField {
                    name: "x".to_string(),
                    ty: Type::Named("i32".to_string()),
                    attributes: Vec::new(),
                },
                StructField {
                    name: "y".to_string(),
                    ty: Type::Named("i32".to_string()),
                    attributes: Vec::new(),
                },
            ],
            is_pub: false,
            attributes: Vec::new(),
            where_clause: Vec::new(),
        };

        let result = processor.generate_debug_impl(&item);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("impl std::fmt::Debug for Point"));
        assert!(code.contains("debug_struct"));
    }

    #[test]
    fn test_clone_impl_generation() {
        let processor = ProceduralMacroProcessor::new();
        let item = Item::Struct {
            name: "Data".to_string(),
            generics: Vec::new(),
            fields: vec![
                StructField {
                    name: "value".to_string(),
                    ty: Type::Named("i32".to_string()),
                    attributes: Vec::new(),
                },
            ],
            is_pub: false,
            attributes: Vec::new(),
            where_clause: Vec::new(),
        };

        let result = processor.generate_clone_impl(&item);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("impl Clone for Data"));
        assert!(code.contains(".clone()"));
    }

    #[test]
    fn test_partial_eq_impl_generation() {
        let processor = ProceduralMacroProcessor::new();
        let item = Item::Struct {
            name: "Pair".to_string(),
            generics: Vec::new(),
            fields: vec![
                StructField {
                    name: "a".to_string(),
                    ty: Type::Named("i32".to_string()),
                    attributes: Vec::new(),
                },
                StructField {
                    name: "b".to_string(),
                    ty: Type::Named("i32".to_string()),
                    attributes: Vec::new(),
                },
            ],
            is_pub: false,
            attributes: Vec::new(),
            where_clause: Vec::new(),
        };

        let result = processor.generate_partial_eq_impl(&item);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("impl PartialEq for Pair"));
        assert!(code.contains("self.a == other.a"));
        assert!(code.contains("self.b == other.b"));
    }

    #[test]
    fn test_attribute_macro_processor_creation() {
        let processor = AttributeMacroProcessor::new();
        assert!(processor.macros.contains_key("cfg"));
        assert!(processor.macros.contains_key("test"));
        assert!(processor.macros.contains_key("allow"));
    }

    #[test]
    fn test_process_cfg_attribute() {
        let item = Item::Struct {
            name: "Test".to_string(),
            generics: Vec::new(),
            fields: Vec::new(),
            is_pub: false,
            attributes: Vec::new(),
            where_clause: Vec::new(),
        };

        let attr = Attribute {
            name: "cfg".to_string(),
            args: vec!["test".to_string()],
            is_macro: true,
        };

        let processor = AttributeMacroProcessor::new();
        let result = processor.process_attribute(&item, &attr);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.is_some());
        assert!(output.unwrap().contains("cfg"));
    }

    #[test]
    fn test_copy_impl_generation() {
        let processor = ProceduralMacroProcessor::new();
        let item = Item::Struct {
            name: "Simple".to_string(),
            generics: Vec::new(),
            fields: Vec::new(),
            is_pub: false,
            attributes: Vec::new(),
            where_clause: Vec::new(),
        };

        let result = processor.generate_copy_impl(&item);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert_eq!(code, "impl Copy for Simple {}");
    }

    #[test]
    fn test_eq_impl_generation() {
        let processor = ProceduralMacroProcessor::new();
        let item = Item::Struct {
            name: "Value".to_string(),
            generics: Vec::new(),
            fields: Vec::new(),
            is_pub: false,
            attributes: Vec::new(),
            where_clause: Vec::new(),
        };

        let result = processor.generate_eq_impl(&item);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert_eq!(code, "impl Eq for Value {}");
    }

    #[test]
    fn test_default_impl_generation() {
        let processor = ProceduralMacroProcessor::new();
        let item = Item::Struct {
            name: "Config".to_string(),
            generics: Vec::new(),
            fields: vec![
                StructField {
                    name: "enabled".to_string(),
                    ty: Type::Named("bool".to_string()),
                    attributes: Vec::new(),
                },
            ],
            is_pub: false,
            attributes: Vec::new(),
            where_clause: Vec::new(),
        };

        let result = processor.generate_default_impl(&item);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("impl Default for Config"));
        assert!(code.contains("Default::default()"));
    }
}
