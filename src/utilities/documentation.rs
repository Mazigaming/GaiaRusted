//! Documentation Generation (Rustdoc-like)
//!
//! Generate HTML documentation from source code comments and attributes

use std::collections::HashMap;

/// Documentation comment
#[derive(Debug, Clone)]
pub struct DocComment {
    pub content: String,
    pub examples: Vec<String>,
    pub see_also: Vec<String>,
    pub deprecated: bool,
}

/// Item documentation
#[derive(Debug, Clone)]
pub struct ItemDoc {
    pub name: String,
    pub item_type: ItemType,
    pub doc: DocComment,
    pub visibility: Visibility,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemType {
    Module,
    Function,
    Struct,
    Enum,
    Trait,
    Type,
    Method,
    Field,
    Variant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    PublicCrate,
    PublicSuper,
    Private,
}

/// Documentation generator
pub struct DocumentationGenerator {
    items: HashMap<String, ItemDoc>,
    output_dir: String,
}

impl DocumentationGenerator {
    /// Create new documentation generator
    pub fn new(output_dir: &str) -> Self {
        DocumentationGenerator {
            items: HashMap::new(),
            output_dir: output_dir.to_string(),
        }
    }

    /// Register item for documentation
    pub fn register_item(&mut self, item: ItemDoc) {
        self.items.insert(item.name.clone(), item);
    }

    /// Generate HTML documentation
    pub fn generate_html(&self) -> String {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n");
        html.push_str("<html>\n");
        html.push_str("<head>\n");
        html.push_str("  <meta charset=\"UTF-8\">\n");
        html.push_str("  <title>Documentation</title>\n");
        html.push_str("  <style>\n");
        html.push_str("    body { font-family: Arial, sans-serif; margin: 20px; }\n");
        html.push_str("    h1 { color: #333; }\n");
        html.push_str("    .item { margin-bottom: 20px; padding: 10px; border-left: 3px solid #0066cc; }\n");
        html.push_str("    .doc { color: #666; white-space: pre-wrap; }\n");
        html.push_str("    .example { background: #f5f5f5; padding: 10px; margin: 10px 0; }\n");
        html.push_str("  </style>\n");
        html.push_str("</head>\n");
        html.push_str("<body>\n");
        html.push_str("  <h1>GaiaRusted Documentation</h1>\n");

        for (_, item) in &self.items {
            html.push_str(&self.generate_item_html(item));
        }

        html.push_str("</body>\n");
        html.push_str("</html>\n");

        html
    }

    /// Generate HTML for single item
    fn generate_item_html(&self, item: &ItemDoc) -> String {
        let mut html = String::new();

        html.push_str("<div class=\"item\">\n");
        html.push_str(&format!("  <h2>{}</h2>\n", item.name));
        html.push_str(&format!("  <p><strong>Type:</strong> {:?}</p>\n", item.item_type));
        html.push_str(&format!("  <p><strong>Visibility:</strong> {:?}</p>\n", item.visibility));

        if item.doc.deprecated {
            html.push_str("  <p style=\"color: red;\"><strong>⚠️ DEPRECATED</strong></p>\n");
        }

        html.push_str("  <div class=\"doc\">\n");
        html.push_str(&format!("    {}\n", html_escape(&item.doc.content)));
        html.push_str("  </div>\n");

        if !item.doc.examples.is_empty() {
            html.push_str("  <h3>Examples:</h3>\n");
            for example in &item.doc.examples {
                html.push_str("  <div class=\"example\">\n");
                html.push_str(&format!("    <pre>{}</pre>\n", html_escape(example)));
                html.push_str("  </div>\n");
            }
        }

        if !item.doc.see_also.is_empty() {
            html.push_str("  <h3>See also:</h3>\n");
            html.push_str("  <ul>\n");
            for see_also in &item.doc.see_also {
                html.push_str(&format!("    <li>{}</li>\n", html_escape(see_also)));
            }
            html.push_str("  </ul>\n");
        }

        html.push_str("</div>\n");

        html
    }

    /// Generate Markdown documentation
    pub fn generate_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# GaiaRusted Documentation\n\n");

        for (_, item) in &self.items {
            md.push_str(&self.generate_item_markdown(item));
        }

        md
    }

    /// Generate Markdown for single item
    fn generate_item_markdown(&self, item: &ItemDoc) -> String {
        let mut md = String::new();

        md.push_str(&format!("## {} ({})\n\n", item.name, format!("{:?}", item.item_type)));

        md.push_str(&format!("**Visibility:** {:?}\n\n", item.visibility));

        if item.doc.deprecated {
            md.push_str("⚠️ **DEPRECATED**\n\n");
        }

        md.push_str(&format!("{}\n\n", item.doc.content));

        if !item.doc.examples.is_empty() {
            md.push_str("### Examples\n\n");
            for example in &item.doc.examples {
                md.push_str("```rust\n");
                md.push_str(&format!("{}\n", example));
                md.push_str("```\n\n");
            }
        }

        if !item.doc.see_also.is_empty() {
            md.push_str("### See Also\n\n");
            for see_also in &item.doc.see_also {
                md.push_str(&format!("- {}\n", see_also));
            }
            md.push_str("\n");
        }

        md
    }

    /// Extract documentation from source
    pub fn extract_from_source(&mut self, source: &str) {
        // Parse doc comments and attributes
        let lines: Vec<&str> = source.lines().collect();
        for (_i, line) in lines.iter().enumerate() {
            if line.trim().starts_with("///") {
                let _content = line.trim_start_matches("///").trim().to_string();
                // Would parse documentation here
            }
        }
    }
}

/// HTML escape helper
fn html_escape(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_generator_creation() {
        let gen = DocumentationGenerator::new("./docs");
        assert_eq!(gen.items.len(), 0);
    }

    #[test]
    fn test_register_item() {
        let mut gen = DocumentationGenerator::new("./docs");
        let item = ItemDoc {
            name: "main".to_string(),
            item_type: ItemType::Function,
            doc: DocComment {
                content: "Entry point".to_string(),
                examples: vec![],
                see_also: vec![],
                deprecated: false,
            },
            visibility: Visibility::Public,
            parent: None,
        };
        gen.register_item(item);
        assert_eq!(gen.items.len(), 1);
    }

    #[test]
    fn test_html_generation() {
        let mut gen = DocumentationGenerator::new("./docs");
        let item = ItemDoc {
            name: "test_fn".to_string(),
            item_type: ItemType::Function,
            doc: DocComment {
                content: "Test function".to_string(),
                examples: vec!["fn test() {}".to_string()],
                see_also: vec!["other_fn".to_string()],
                deprecated: false,
            },
            visibility: Visibility::Public,
            parent: None,
        };
        gen.register_item(item);

        let html = gen.generate_html();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("test_fn"));
    }

    #[test]
    fn test_markdown_generation() {
        let mut gen = DocumentationGenerator::new("./docs");
        let item = ItemDoc {
            name: "my_struct".to_string(),
            item_type: ItemType::Struct,
            doc: DocComment {
                content: "A sample struct".to_string(),
                examples: vec!["let s = MyStruct {}".to_string()],
                see_also: vec![],
                deprecated: false,
            },
            visibility: Visibility::Public,
            parent: None,
        };
        gen.register_item(item);

        let md = gen.generate_markdown();
        assert!(md.contains("my_struct"));
        assert!(md.contains("```rust"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<test>"), "&lt;test&gt;");
        assert_eq!(html_escape("&"), "&amp;");
    }
}
