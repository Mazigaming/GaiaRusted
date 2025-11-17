//! # Enhanced Procedural Macros System
//!
//! Advanced procedural macro support for code generation:
//! - Attribute macro processing
//! - Function-like macro expansion with validation
//! - Macro composition and chaining
//! - Proc macro error reporting
//! - Token stream manipulation

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MacroInvocation {
    pub name: String,
    pub args: Vec<String>,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MacroDefinition {
    pub name: String,
    pub kind: MacroKind,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacroKind {
    FunctionLike,
    Attribute,
    Derive,
}

#[derive(Debug, Clone)]
pub struct TokenStream {
    pub tokens: Vec<String>,
}

pub struct ProceduralMacroEngine {
    macros: HashMap<String, MacroDefinition>,
    expansions: HashMap<String, TokenStream>,
    macro_cache: HashMap<String, String>,
}

impl ProceduralMacroEngine {
    pub fn new() -> Self {
        ProceduralMacroEngine {
            macros: HashMap::new(),
            expansions: HashMap::new(),
            macro_cache: HashMap::new(),
        }
    }

    pub fn register_macro(&mut self, definition: MacroDefinition) -> Result<(), String> {
        if self.macros.contains_key(&definition.name) {
            return Err(format!("Macro {} already defined", definition.name));
        }

        self.macros.insert(definition.name.clone(), definition);
        Ok(())
    }

    pub fn expand_macro(&mut self, invocation: &MacroInvocation) -> Result<String, String> {
        if let Some(cached) = self.macro_cache.get(&invocation.name) {
            return Ok(cached.clone());
        }

        let definition = self.macros.get(&invocation.name)
            .ok_or(format!("Macro {} not found", invocation.name))?
            .clone();

        let result = self.perform_expansion(&invocation, &definition)?;

        self.macro_cache.insert(invocation.name.clone(), result.clone());
        Ok(result)
    }

    fn perform_expansion(
        &self,
        invocation: &MacroInvocation,
        definition: &MacroDefinition,
    ) -> Result<String, String> {
        match definition.kind {
            MacroKind::FunctionLike => {
                self.expand_function_like(invocation, definition)
            }
            MacroKind::Attribute => {
                self.expand_attribute(invocation, definition)
            }
            MacroKind::Derive => {
                self.expand_derive(invocation, definition)
            }
        }
    }

    fn expand_function_like(
        &self,
        invocation: &MacroInvocation,
        definition: &MacroDefinition,
    ) -> Result<String, String> {
        if invocation.args.len() != definition.inputs.len() {
            return Err(format!(
                "Argument count mismatch for {}: expected {}, got {}",
                invocation.name,
                definition.inputs.len(),
                invocation.args.len()
            ));
        }

        let mut result = String::new();
        for output in &definition.outputs {
            for (i, input) in definition.inputs.iter().enumerate() {
                if output.contains(input) {
                    result = output.replace(input, &invocation.args[i]);
                }
            }
        }

        Ok(result)
    }

    fn expand_attribute(
        &self,
        invocation: &MacroInvocation,
        _definition: &MacroDefinition,
    ) -> Result<String, String> {
        let mut result = String::from("#[");
        result.push_str(&invocation.name);

        if !invocation.args.is_empty() {
            result.push('(');
            result.push_str(&invocation.args.join(", "));
            result.push(')');
        }

        result.push(']');
        Ok(result)
    }

    fn expand_derive(
        &self,
        invocation: &MacroInvocation,
        _definition: &MacroDefinition,
    ) -> Result<String, String> {
        let mut result = String::from("#[derive(");
        result.push_str(&invocation.name);

        if !invocation.attributes.is_empty() {
            result.push_str(", ");
            result.push_str(&invocation.attributes.join(", "));
        }

        result.push_str(")]");
        Ok(result)
    }

    pub fn validate_macro(&self, definition: &MacroDefinition) -> Result<(), String> {
        if definition.name.is_empty() {
            return Err("Macro name cannot be empty".to_string());
        }

        if definition.inputs.is_empty() && definition.kind == MacroKind::FunctionLike {
            return Err("Function-like macro must have inputs".to_string());
        }

        Ok(())
    }

    pub fn get_macro(&self, name: &str) -> Option<MacroDefinition> {
        self.macros.get(name).cloned()
    }

    pub fn chain_macros(
        &mut self,
        invocations: &[MacroInvocation],
    ) -> Result<String, String> {
        let mut result = String::new();

        for invocation in invocations {
            let expansion = self.expand_macro(invocation)?;
            result.push_str(&expansion);
            result.push('\n');
        }

        Ok(result)
    }

    pub fn create_token_stream(&self, tokens: Vec<String>) -> TokenStream {
        TokenStream { tokens }
    }

    pub fn register_expansion(
        &mut self,
        macro_name: String,
        tokens: TokenStream,
    ) {
        self.expansions.insert(macro_name, tokens);
    }

    pub fn get_expansion(&self, macro_name: &str) -> Option<TokenStream> {
        self.expansions.get(macro_name).cloned()
    }

    pub fn collect_macros(&self) -> Vec<String> {
        self.macros.keys().cloned().collect()
    }

    pub fn is_macro_defined(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_engine() {
        let _engine = ProceduralMacroEngine::new();
        assert!(true);
    }

    #[test]
    fn test_register_macro() {
        let mut engine = ProceduralMacroEngine::new();
        let macro_def = MacroDefinition {
            name: "vec".to_string(),
            kind: MacroKind::FunctionLike,
            inputs: vec!["elem".to_string()],
            outputs: vec!["Vec::from($elem)".to_string()],
        };

        assert!(engine.register_macro(macro_def).is_ok());
    }

    #[test]
    fn test_expand_function_like() {
        let mut engine = ProceduralMacroEngine::new();
        let macro_def = MacroDefinition {
            name: "double".to_string(),
            kind: MacroKind::FunctionLike,
            inputs: vec!["x".to_string()],
            outputs: vec!["($x * 2)".to_string()],
        };

        engine.register_macro(macro_def).unwrap();

        let invocation = MacroInvocation {
            name: "double".to_string(),
            args: vec!["5".to_string()],
            attributes: vec![],
        };

        let result = engine.expand_macro(&invocation);
        assert!(result.is_ok());
    }

    #[test]
    fn test_expand_attribute() {
        let mut engine = ProceduralMacroEngine::new();
        let macro_def = MacroDefinition {
            name: "test".to_string(),
            kind: MacroKind::Attribute,
            inputs: vec![],
            outputs: vec![],
        };

        engine.register_macro(macro_def).unwrap();

        let invocation = MacroInvocation {
            name: "test".to_string(),
            args: vec![],
            attributes: vec![],
        };

        let result = engine.expand_macro(&invocation);
        assert!(result.is_ok());
    }

    #[test]
    fn test_expand_derive() {
        let mut engine = ProceduralMacroEngine::new();
        let macro_def = MacroDefinition {
            name: "Clone".to_string(),
            kind: MacroKind::Derive,
            inputs: vec![],
            outputs: vec![],
        };

        engine.register_macro(macro_def).unwrap();

        let invocation = MacroInvocation {
            name: "Clone".to_string(),
            args: vec![],
            attributes: vec![],
        };

        let result = engine.expand_macro(&invocation);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_macro() {
        let engine = ProceduralMacroEngine::new();

        let valid = MacroDefinition {
            name: "valid".to_string(),
            kind: MacroKind::FunctionLike,
            inputs: vec!["x".to_string()],
            outputs: vec![],
        };

        assert!(engine.validate_macro(&valid).is_ok());
    }

    #[test]
    fn test_validate_macro_empty_name() {
        let engine = ProceduralMacroEngine::new();

        let invalid = MacroDefinition {
            name: "".to_string(),
            kind: MacroKind::FunctionLike,
            inputs: vec!["x".to_string()],
            outputs: vec![],
        };

        assert!(engine.validate_macro(&invalid).is_err());
    }

    #[test]
    fn test_get_macro() {
        let mut engine = ProceduralMacroEngine::new();
        let macro_def = MacroDefinition {
            name: "test_macro".to_string(),
            kind: MacroKind::FunctionLike,
            inputs: vec![],
            outputs: vec![],
        };

        engine.register_macro(macro_def).unwrap();
        assert!(engine.get_macro("test_macro").is_some());
    }

    #[test]
    fn test_chain_macros() {
        let mut engine = ProceduralMacroEngine::new();
        let macro_def = MacroDefinition {
            name: "id".to_string(),
            kind: MacroKind::FunctionLike,
            inputs: vec!["x".to_string()],
            outputs: vec!["$x".to_string()],
        };

        engine.register_macro(macro_def).unwrap();

        let invocations = vec![
            MacroInvocation {
                name: "id".to_string(),
                args: vec!["a".to_string()],
                attributes: vec![],
            },
        ];

        let result = engine.chain_macros(&invocations);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_token_stream() {
        let engine = ProceduralMacroEngine::new();
        let tokens = vec!["fn".to_string(), "main".to_string()];
        let stream = engine.create_token_stream(tokens);

        assert_eq!(stream.tokens.len(), 2);
    }

    #[test]
    fn test_register_expansion() {
        let mut engine = ProceduralMacroEngine::new();
        let tokens = engine.create_token_stream(vec!["code".to_string()]);

        engine.register_expansion("macro1".to_string(), tokens);
        assert!(engine.get_expansion("macro1").is_some());
    }

    #[test]
    fn test_collect_macros() {
        let mut engine = ProceduralMacroEngine::new();

        let macro1 = MacroDefinition {
            name: "m1".to_string(),
            kind: MacroKind::FunctionLike,
            inputs: vec![],
            outputs: vec![],
        };

        let macro2 = MacroDefinition {
            name: "m2".to_string(),
            kind: MacroKind::Attribute,
            inputs: vec![],
            outputs: vec![],
        };

        engine.register_macro(macro1).unwrap();
        engine.register_macro(macro2).unwrap();

        let macros = engine.collect_macros();
        assert_eq!(macros.len(), 2);
    }

    #[test]
    fn test_is_macro_defined() {
        let mut engine = ProceduralMacroEngine::new();
        let macro_def = MacroDefinition {
            name: "defined".to_string(),
            kind: MacroKind::FunctionLike,
            inputs: vec![],
            outputs: vec![],
        };

        engine.register_macro(macro_def).unwrap();
        assert!(engine.is_macro_defined("defined"));
        assert!(!engine.is_macro_defined("undefined"));
    }

    #[test]
    fn test_macro_caching() {
        let mut engine = ProceduralMacroEngine::new();
        let macro_def = MacroDefinition {
            name: "cached".to_string(),
            kind: MacroKind::FunctionLike,
            inputs: vec!["x".to_string()],
            outputs: vec!["$x".to_string()],
        };

        engine.register_macro(macro_def).unwrap();

        let invocation = MacroInvocation {
            name: "cached".to_string(),
            args: vec!["val".to_string()],
            attributes: vec![],
        };

        let _result = engine.expand_macro(&invocation);
        assert!(!engine.macro_cache.is_empty());
    }
}
