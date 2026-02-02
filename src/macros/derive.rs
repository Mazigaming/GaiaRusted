#[derive(Debug, Clone)]
pub struct DeriveAttribute {
    pub traits: Vec<String>,
}

pub struct DeriveCodeGenerator;

impl DeriveCodeGenerator {
    pub fn generate_derive_impl(
        struct_name: &str,
        fields: &[(String, String)],
        traits: &[String],
    ) -> Result<String, String> {
        let mut generated = String::new();

        for trait_name in traits {
            match trait_name.as_str() {
                "Debug" => {
                    generated.push_str(&Self::generate_debug(struct_name, fields));
                    generated.push('\n');
                }
                "Clone" => {
                    generated.push_str(&Self::generate_clone(struct_name, fields));
                    generated.push('\n');
                }
                "Default" => {
                    generated.push_str(&Self::generate_default(struct_name, fields));
                    generated.push('\n');
                }
                "PartialEq" => {
                    generated.push_str(&Self::generate_partial_eq(struct_name, fields));
                    generated.push('\n');
                }
                "Eq" => {
                    generated.push_str(&Self::generate_eq(struct_name));
                    generated.push('\n');
                }
                "Hash" => {
                    generated.push_str(&Self::generate_hash(struct_name, fields));
                    generated.push('\n');
                }
                "Copy" => {
                    generated.push_str(&Self::generate_copy(struct_name));
                    generated.push('\n');
                }
                _ => return Err(format!("Unknown derive trait: {}", trait_name)),
            }
        }

        Ok(generated)
    }

    fn generate_debug(struct_name: &str, fields: &[(String, String)]) -> String {
        let mut code = format!("impl std::fmt::Debug for {} {{\n", struct_name);
        code.push_str("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
        code.push_str(&format!("        f.debug_struct(\"{}\")\n", struct_name));

        for (field_name, _) in fields {
            code.push_str(&format!("            .field(\"{}\", &self.{})\n", field_name, field_name));
        }

        code.push_str("            .finish()\n");
        code.push_str("    }\n");
        code.push_str("}\n");
        code
    }

    fn generate_clone(struct_name: &str, fields: &[(String, String)]) -> String {
        let mut code = format!("impl Clone for {} {{\n", struct_name);
        code.push_str("    fn clone(&self) -> Self {\n");
        code.push_str(&format!("        {} {{\n", struct_name));

        for (field_name, _) in fields {
            code.push_str(&format!("            {}: self.{}.clone(),\n", field_name, field_name));
        }

        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n");
        code
    }

    fn generate_default(struct_name: &str, fields: &[(String, String)]) -> String {
        let mut code = format!("impl Default for {} {{\n", struct_name);
        code.push_str("    fn default() -> Self {\n");
        code.push_str(&format!("        {} {{\n", struct_name));

        for (field_name, _) in fields {
            code.push_str(&format!("            {}: Default::default(),\n", field_name));
        }

        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n");
        code
    }

    fn generate_partial_eq(struct_name: &str, fields: &[(String, String)]) -> String {
        let mut code = format!("impl PartialEq for {} {{\n", struct_name);
        code.push_str("    fn eq(&self, other: &Self) -> bool {\n");

        if fields.is_empty() {
            code.push_str("        true\n");
        } else {
            code.push_str("        self.fields == other.fields");
            for (field_name, _) in fields {
                code.push_str(&format!(" && self.{} == other.{}", field_name, field_name));
            }
            code.push('\n');
        }

        code.push_str("    }\n");
        code.push_str("}\n");
        code
    }

    fn generate_eq(struct_name: &str) -> String {
        format!("impl Eq for {} {{}}\n", struct_name)
    }

    fn generate_hash(struct_name: &str, fields: &[(String, String)]) -> String {
        let mut code = format!("impl std::hash::Hash for {} {{\n", struct_name);
        code.push_str("    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {\n");

        for (field_name, _) in fields {
            code.push_str(&format!("        self.{}.hash(state);\n", field_name));
        }

        code.push_str("    }\n");
        code.push_str("}\n");
        code
    }

    fn generate_copy(struct_name: &str) -> String {
        format!("impl Copy for {} {{}}\n", struct_name)
    }

    pub fn parse_derive_attribute(attr_str: &str) -> Result<DeriveAttribute, String> {
        let content = attr_str
            .trim_start_matches("#[derive(")
            .trim_end_matches(")]");

        let traits: Vec<String> = content.split(',').map(|s| s.trim().to_string()).collect();

        Ok(DeriveAttribute { traits })
    }

    pub fn is_derive_attribute(attr_str: &str) -> bool {
        attr_str.starts_with("#[derive(") && attr_str.ends_with(")]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_debug() {
        let fields = vec![
            ("x".to_string(), "i32".to_string()),
            ("y".to_string(), "i32".to_string()),
        ];
        let code = DeriveCodeGenerator::generate_debug("Point", &fields);
        assert!(code.contains("impl std::fmt::Debug for Point"));
        assert!(code.contains("debug_struct"));
    }

    #[test]
    fn test_generate_clone() {
        let fields = vec![("value".to_string(), "i32".to_string())];
        let code = DeriveCodeGenerator::generate_clone("Value", &fields);
        assert!(code.contains("impl Clone for Value"));
        assert!(code.contains("clone()"));
    }

    #[test]
    fn test_parse_derive_attribute() {
        let attr = "#[derive(Debug, Clone)]";
        let result = DeriveCodeGenerator::parse_derive_attribute(attr);
        assert!(result.is_ok());
        let derive = result.unwrap();
        assert_eq!(derive.traits.len(), 2);
        assert!(derive.traits.contains(&"Debug".to_string()));
        assert!(derive.traits.contains(&"Clone".to_string()));
    }

    #[test]
    fn test_is_derive_attribute() {
        assert!(DeriveCodeGenerator::is_derive_attribute("#[derive(Debug)]"));
        assert!(!DeriveCodeGenerator::is_derive_attribute("#[test]"));
    }

    #[test]
    fn test_generate_multiple_derives() {
        let fields = vec![("x".to_string(), "i32".to_string())];
        let result = DeriveCodeGenerator::generate_derive_impl(
            "MyStruct",
            &fields,
            &["Debug".to_string(), "Clone".to_string()],
        );
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("impl std::fmt::Debug"));
        assert!(code.contains("impl Clone"));
    }
}
