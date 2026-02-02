//! Vtable Generation for Trait Objects
//! Generates virtual method tables for dyn Trait dynamic dispatch

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VtableEntry {
    pub method_name: String,
    pub function_pointer: String,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct VtableLayout {
    pub trait_name: String,
    pub concrete_type: String,
    pub entries: Vec<VtableEntry>,
    pub symbol: String,
    pub size: usize,
}

pub struct VtableGenerator {
    vtables: HashMap<String, VtableLayout>,
}

impl VtableGenerator {
    pub fn new() -> Self {
        VtableGenerator {
            vtables: HashMap::new(),
        }
    }

    pub fn generate_vtable(
        &mut self,
        trait_name: &str,
        concrete_type: &str,
        methods: Vec<String>,
    ) -> VtableLayout {
        let mut entries = Vec::new();

        for (idx, method) in methods.iter().enumerate() {
            entries.push(VtableEntry {
                method_name: method.clone(),
                function_pointer: format!("{}::{}", concrete_type, method),
                offset: idx * 8,
            });
        }

        let symbol = format!("_vtable_{}_{}", trait_name, concrete_type);
        let size = methods.len() * 8;

        let layout = VtableLayout {
            trait_name: trait_name.to_string(),
            concrete_type: concrete_type.to_string(),
            entries,
            symbol: symbol.clone(),
            size,
        };

        self.vtables.insert(symbol, layout.clone());
        layout
    }

    pub fn get_vtable(&self, symbol: &str) -> Option<&VtableLayout> {
        self.vtables.get(symbol)
    }

    pub fn generate_assembly(&self, layout: &VtableLayout) -> String {
        let mut asm = String::new();
        asm.push_str(&format!(".align 8\n"));
        asm.push_str(&format!(".globl {}\n", layout.symbol));
        asm.push_str(&format!("{}:\n", layout.symbol));

        for entry in &layout.entries {
            asm.push_str(&format!("    .quad {}\n", entry.function_pointer));
        }

        asm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vtable_creation() {
        let mut gen = VtableGenerator::new();
        let layout = gen.generate_vtable("Animal", "Dog", vec!["speak".to_string()]);
        assert_eq!(layout.entries.len(), 1);
        assert_eq!(layout.size, 8);
    }
}
