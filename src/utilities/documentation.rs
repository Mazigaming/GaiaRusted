use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DocComment {
    pub summary: String,
    pub description: Option<String>,
    pub examples: Vec<String>,
    pub params: HashMap<String, String>,
    pub returns: Option<String>,
    pub panics: Option<String>,
    pub safety: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DocumentationItem {
    pub name: String,
    pub item_type: ItemType,
    pub doc: DocComment,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemType {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
    Constant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visibility {
    Public,
    Private,
}

pub struct DocumentationGenerator {
    items: Vec<DocumentationItem>,
}

impl DocumentationGenerator {
    pub fn new() -> Self {
        DocumentationGenerator {
            items: Vec::new(),
        }
    }

    pub fn register_item(&mut self, item: DocumentationItem) {
        self.items.push(item);
    }

    pub fn parse_doc_comment(comment: &str) -> DocComment {
        let lines: Vec<&str> = comment.lines().collect();
        let mut summary = String::new();
        let mut description = String::new();
        let mut examples = Vec::new();
        let mut params = HashMap::new();
        let mut returns = None;
        let mut panics = None;
        let mut safety = None;

        let mut current_section = Section::Summary;
        let mut example_buffer = String::new();

        for line in lines {
            let trimmed = line.trim_start_matches("///").trim();

            match trimmed {
                line if line.starts_with("# Examples") => {
                    current_section = Section::Examples;
                    if !example_buffer.is_empty() {
                        examples.push(example_buffer.clone());
                        example_buffer.clear();
                    }
                }
                line if line.starts_with("# Arguments") || line.starts_with("# Parameters") => {
                    current_section = Section::Parameters;
                }
                line if line.starts_with("# Returns") => {
                    current_section = Section::Returns;
                }
                line if line.starts_with("# Panics") => {
                    current_section = Section::Panics;
                }
                line if line.starts_with("# Safety") => {
                    current_section = Section::Safety;
                }
                line if line.is_empty() && current_section == Section::Summary => {
                    if !summary.is_empty() {
                        current_section = Section::Description;
                    }
                }
                _ => {
                    match current_section {
                        Section::Summary => {
                            if !summary.is_empty() {
                                summary.push(' ');
                            }
                            summary.push_str(trimmed);
                        }
                        Section::Description => {
                            if !description.is_empty() {
                                description.push('\n');
                            }
                            description.push_str(trimmed);
                        }
                        Section::Examples => {
                            example_buffer.push_str(trimmed);
                            example_buffer.push('\n');
                        }
                        Section::Parameters => {
                            if let Some((param_name, param_desc)) = Self::parse_param_line(trimmed)
                            {
                                params.insert(param_name, param_desc);
                            }
                        }
                        Section::Returns => {
                            if returns.is_none() {
                                returns = Some(trimmed.to_string());
                            }
                        }
                        Section::Panics => {
                            if panics.is_none() {
                                panics = Some(trimmed.to_string());
                            }
                        }
                        Section::Safety => {
                            if safety.is_none() {
                                safety = Some(trimmed.to_string());
                            }
                        }
                    }
                }
            }
        }

        if !example_buffer.is_empty() {
            examples.push(example_buffer);
        }

        DocComment {
            summary: summary.trim().to_string(),
            description: if description.is_empty() {
                None
            } else {
                Some(description)
            },
            examples,
            params,
            returns,
            panics,
            safety,
        }
    }

    fn parse_param_line(line: &str) -> Option<(String, String)> {
        if let Some(dash_pos) = line.find('-') {
            let before_dash = line[..dash_pos].trim();
            let after_dash = line[dash_pos + 1..].trim();

            let param_name = before_dash
                .split_whitespace()
                .next()?
                .to_string();
            let param_desc = after_dash.to_string();

            Some((param_name, param_desc))
        } else {
            None
        }
    }

    pub fn generate_markdown(&self) -> String {
        let mut output = String::from("# API Documentation\n\n");

        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut traits = Vec::new();
        let mut modules = Vec::new();

        for item in &self.items {
            match item.item_type {
                ItemType::Function => functions.push(item),
                ItemType::Struct => structs.push(item),
                ItemType::Enum => enums.push(item),
                ItemType::Trait => traits.push(item),
                ItemType::Module => modules.push(item),
                _ => {}
            }
        }

        if !modules.is_empty() {
            output.push_str("## Modules\n\n");
            for item in modules {
                Self::append_item_docs(&mut output, item);
            }
        }

        if !structs.is_empty() {
            output.push_str("## Structs\n\n");
            for item in structs {
                Self::append_item_docs(&mut output, item);
            }
        }

        if !enums.is_empty() {
            output.push_str("## Enums\n\n");
            for item in enums {
                Self::append_item_docs(&mut output, item);
            }
        }

        if !traits.is_empty() {
            output.push_str("## Traits\n\n");
            for item in traits {
                Self::append_item_docs(&mut output, item);
            }
        }

        if !functions.is_empty() {
            output.push_str("## Functions\n\n");
            for item in functions {
                Self::append_item_docs(&mut output, item);
            }
        }

        output
    }

    fn append_item_docs(output: &mut String, item: &DocumentationItem) {
        output.push_str(&format!("### {}\n\n", item.name));
        output.push_str(&format!("**Type:** {}\n\n", Self::item_type_str(item.item_type)));

        if item.visibility == Visibility::Private {
            output.push_str("**Visibility:** Private\n\n");
        }

        output.push_str(&format!("{}\n\n", item.doc.summary));

        if let Some(desc) = &item.doc.description {
            output.push_str(desc);
            output.push_str("\n\n");
        }

        if !item.doc.params.is_empty() {
            output.push_str("**Parameters:**\n\n");
            for (param_name, param_desc) in &item.doc.params {
                output.push_str(&format!("- `{}`: {}\n", param_name, param_desc));
            }
            output.push('\n');
        }

        if let Some(returns) = &item.doc.returns {
            output.push_str(&format!("**Returns:** {}\n\n", returns));
        }

        if let Some(safety) = &item.doc.safety {
            output.push_str("**Safety:**\n\n");
            output.push_str(safety);
            output.push_str("\n\n");
        }

        if let Some(panics) = &item.doc.panics {
            output.push_str("**Panics:**\n\n");
            output.push_str(panics);
            output.push_str("\n\n");
        }

        if !item.doc.examples.is_empty() {
            output.push_str("**Examples:**\n\n```rust\n");
            for example in &item.doc.examples {
                output.push_str(example);
            }
            output.push_str("```\n\n");
        }
    }

    fn item_type_str(item_type: ItemType) -> &'static str {
        match item_type {
            ItemType::Function => "Function",
            ItemType::Struct => "Struct",
            ItemType::Enum => "Enum",
            ItemType::Trait => "Trait",
            ItemType::Module => "Module",
            ItemType::Constant => "Constant",
        }
    }

    pub fn get_items(&self) -> &[DocumentationItem] {
        &self.items
    }

    pub fn find_item(&self, name: &str) -> Option<&DocumentationItem> {
        self.items.iter().find(|item| item.name == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Section {
    Summary,
    Description,
    Parameters,
    Returns,
    Examples,
    Panics,
    Safety,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_doc_comment() {
        let comment = "/// This is a function\n/// that does something";
        let doc = DocumentationGenerator::parse_doc_comment(comment);
        assert_eq!(doc.summary, "This is a function that does something");
    }

    #[test]
    fn test_parse_doc_with_examples() {
        let comment = "/// Add two numbers\n/// # Examples\n/// let result = add(1, 2);";
        let doc = DocumentationGenerator::parse_doc_comment(comment);
        assert_eq!(doc.summary, "Add two numbers");
        assert!(!doc.examples.is_empty());
    }

    #[test]
    fn test_parse_doc_with_params() {
        let comment = "/// Multiply numbers\n/// # Parameters\n/// x - first number\n/// y - second number";
        let doc = DocumentationGenerator::parse_doc_comment(comment);
        assert!(!doc.params.is_empty());
        assert!(doc.params.len() >= 1);
    }

    #[test]
    fn test_generate_markdown() {
        let mut gen = DocumentationGenerator::new();
        gen.register_item(DocumentationItem {
            name: "test_fn".to_string(),
            item_type: ItemType::Function,
            doc: DocComment {
                summary: "A test function".to_string(),
                description: None,
                examples: vec![],
                params: HashMap::new(),
                returns: Some("i32".to_string()),
                panics: None,
                safety: None,
            },
            visibility: Visibility::Public,
        });

        let markdown = gen.generate_markdown();
        assert!(markdown.contains("test_fn"));
        assert!(markdown.contains("A test function"));
    }
}
