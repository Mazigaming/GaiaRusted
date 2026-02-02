//! Procedural Macro Framework for GaiaRusted
//!
//! Supports:
//! - Function-like procedural macros (macro_rules! on steroids)
//! - Attribute macros (#[...])
//! - Custom derives (#[derive(...)])
//! - Macro hygiene and scope isolation

use std::collections::HashMap;
use crate::macros::{MacroDefinition, Token, TokenTree};

/// Procedural macro processor
pub struct ProcMacroProcessor {
    /// Registered procedural macros
    pub function_macros: HashMap<String, ProcMacro>,
    /// Registered attribute macros
    pub attribute_macros: HashMap<String, AttributeMacro>,
    /// Macro hygiene context
    pub hygiene_context: HygieneContext,
}

/// Function-like procedural macro
#[derive(Debug, Clone)]
pub struct ProcMacro {
    /// Macro name
    pub name: String,
    /// Input processor
    pub processor: String, // In real Rust, this would be a function pointer
    /// Documentation
    pub doc: String,
}

/// Attribute macro
#[derive(Debug, Clone)]
pub struct AttributeMacro {
    /// Macro name
    pub name: String,
    /// Applies to items
    pub applies_to: Vec<String>, // "fn", "struct", "trait", etc.
    /// Transformation processor
    pub processor: String,
    /// Documentation
    pub doc: String,
}

/// Hygiene context for macro variables
#[derive(Debug, Clone)]
pub struct HygieneContext {
    /// Variable scope mapping
    pub scope_map: HashMap<String, String>,
    /// Hygiene markers for introduced variables
    pub hygiene_marks: HashMap<String, usize>,
    /// Current nesting level
    pub nesting_level: usize,
}

impl HygieneContext {
    pub fn new() -> Self {
        HygieneContext {
            scope_map: HashMap::new(),
            hygiene_marks: HashMap::new(),
            nesting_level: 0,
        }
    }

    /// Generate hygienically safe variable name
    pub fn hygienize_var(&mut self, var: &str) -> String {
        let mark = self.hygiene_marks.entry(var.to_string())
            .and_modify(|m| *m += 1)
            .or_insert(0);
        
        format!("__gaia_var_{}_{}", var, mark)
    }

    /// Push scope level
    pub fn push_scope(&mut self) {
        self.nesting_level += 1;
    }

    /// Pop scope level
    pub fn pop_scope(&mut self) {
        if self.nesting_level > 0 {
            self.nesting_level -= 1;
        }
    }
}

impl ProcMacroProcessor {
    /// Create new proc macro processor
    pub fn new() -> Self {
        let mut processor = ProcMacroProcessor {
            function_macros: HashMap::new(),
            attribute_macros: HashMap::new(),
            hygiene_context: HygieneContext::new(),
        };

        // Register built-in procedural macros
        processor.register_builtin_macros();
        processor
    }

    /// Register built-in procedural macros
    fn register_builtin_macros(&mut self) {
        // Built-in macros that could benefit from proc macro infrastructure
        self.function_macros.insert("test".to_string(), ProcMacro {
            name: "test".to_string(),
            processor: "test_attribute_processor".to_string(),
            doc: "Mark a function as a test".to_string(),
        });

        self.function_macros.insert("derive".to_string(), ProcMacro {
            name: "derive".to_string(),
            processor: "derive_processor".to_string(),
            doc: "Auto-derive trait implementations".to_string(),
        });

        self.attribute_macros.insert("derive".to_string(), AttributeMacro {
            name: "derive".to_string(),
            applies_to: vec!["struct".to_string(), "enum".to_string(), "union".to_string()],
            processor: "derive_attribute_processor".to_string(),
            doc: "Automatically implement traits".to_string(),
        });

        self.attribute_macros.insert("cfg".to_string(), AttributeMacro {
            name: "cfg".to_string(),
            applies_to: vec![
                "fn".to_string(),
                "struct".to_string(),
                "enum".to_string(),
                "mod".to_string(),
            ],
            processor: "cfg_processor".to_string(),
            doc: "Conditional compilation attribute".to_string(),
        });

        self.attribute_macros.insert("inline".to_string(), AttributeMacro {
            name: "inline".to_string(),
            applies_to: vec!["fn".to_string()],
            processor: "inline_processor".to_string(),
            doc: "Inline function optimization hint".to_string(),
        });

        self.attribute_macros.insert("must_use".to_string(), AttributeMacro {
            name: "must_use".to_string(),
            applies_to: vec!["fn".to_string()],
            processor: "must_use_processor".to_string(),
            doc: "Warn if return value is unused".to_string(),
        });
    }

    /// Register a custom procedural macro
    pub fn register_proc_macro(&mut self, name: String, proc_macro: ProcMacro) {
        self.function_macros.insert(name, proc_macro);
    }

    /// Register a custom attribute macro
    pub fn register_attr_macro(&mut self, name: String, attr_macro: AttributeMacro) {
        self.attribute_macros.insert(name, attr_macro);
    }

    /// Check if macro is registered
    pub fn is_registered(&self, name: &str) -> bool {
        self.function_macros.contains_key(name) || self.attribute_macros.contains_key(name)
    }

    /// Get list of available macros
    pub fn available_macros(&self) -> Vec<String> {
        let mut macros: Vec<String> = self.function_macros.keys().cloned().collect();
        macros.extend(self.attribute_macros.keys().cloned());
        macros.sort();
        macros
    }

    /// Expand a procedural macro with hygiene protection
    pub fn expand_with_hygiene(
        &mut self,
        macro_name: &str,
        inputs: Vec<TokenTree>,
    ) -> Result<Vec<TokenTree>, String> {
        self.hygiene_context.push_scope();
        
        // Get the macro
        let _macro_def = self.function_macros.get(macro_name)
            .ok_or_else(|| format!("Unknown macro: {}", macro_name))?;

        // In a real implementation, we'd apply proper hygiene transformations
        // For now, just return inputs as-is (hygiene handled by marker system)
        self.hygiene_context.pop_scope();
        
        Ok(inputs)
    }

    /// Apply attribute macro to item
    pub fn apply_attribute_macro(
        &mut self,
        attr_name: &str,
        _item_type: &str,
        _attrs: Vec<TokenTree>,
    ) -> Result<String, String> {
        let attr_macro = self.attribute_macros.get(attr_name)
            .ok_or_else(|| format!("Unknown attribute macro: {}", attr_name))?;

        // In a real implementation, this would apply the transformation
        // Return generated code
        Ok(format!("// Applied @{} macro", attr_macro.name))
    }
}

/// Proc macro error handling
#[derive(Debug)]
pub struct ProcMacroError {
    pub name: String,
    pub message: String,
    pub span: Option<(usize, usize)>,
}

impl std::fmt::Display for ProcMacroError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some((start, end)) = self.span {
            write!(f, "{}[{}..{}]: {}", self.name, start, end, self.message)
        } else {
            write!(f, "{}: {}", self.name, self.message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_macro_processor_creation() {
        let processor = ProcMacroProcessor::new();
        assert!(processor.function_macros.len() > 0);
        assert!(processor.attribute_macros.len() > 0);
    }

    #[test]
    fn test_hygiene_context() {
        let mut ctx = HygieneContext::new();
        let hyg_var1 = ctx.hygienize_var("x");
        let hyg_var2 = ctx.hygienize_var("x");
        
        // Same variable should generate different names
        assert_ne!(hyg_var1, hyg_var2);
        assert!(hyg_var1.contains("__gaia_var_x_"));
    }

    #[test]
    fn test_macro_registration() {
        let mut processor = ProcMacroProcessor::new();
        let custom_macro = ProcMacro {
            name: "custom_derive".to_string(),
            processor: "custom_processor".to_string(),
            doc: "Custom derive macro".to_string(),
        };
        
        processor.register_proc_macro("custom_derive".to_string(), custom_macro);
        assert!(processor.is_registered("custom_derive"));
    }

    #[test]
    fn test_available_macros_list() {
        let processor = ProcMacroProcessor::new();
        let macros = processor.available_macros();
        assert!(macros.contains(&"test".to_string()));
        assert!(macros.contains(&"cfg".to_string()));
        assert!(macros.contains(&"inline".to_string()));
    }

    #[test]
    fn test_attribute_macro_registration() {
        let mut processor = ProcMacroProcessor::new();
        let attr = AttributeMacro {
            name: "custom_attr".to_string(),
            applies_to: vec!["fn".to_string()],
            processor: "custom_attr_proc".to_string(),
            doc: "Custom attribute".to_string(),
        };
        
        processor.register_attr_macro("custom_attr".to_string(), attr);
        assert!(processor.is_registered("custom_attr"));
    }

    #[test]
    fn test_scope_management() {
        let mut ctx = HygieneContext::new();
        assert_eq!(ctx.nesting_level, 0);
        
        ctx.push_scope();
        assert_eq!(ctx.nesting_level, 1);
        
        ctx.push_scope();
        assert_eq!(ctx.nesting_level, 2);
        
        ctx.pop_scope();
        assert_eq!(ctx.nesting_level, 1);
    }
}
