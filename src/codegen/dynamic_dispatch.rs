//! Dynamic Dispatch Code Generation for Trait Objects
//! Generates assembly for calling methods through vtables

use super::vtable_generation::VtableLayout;

pub struct DynamicDispatchCodegen;

impl DynamicDispatchCodegen {
    /// Generate code to load method pointer from vtable and call it
    pub fn generate_trait_method_call(
        layout: &VtableLayout,
        method_name: &str,
        object_reg: &str,    // Register containing fat pointer
        return_reg: &str,    // Register for return value
    ) -> Option<String> {
        let entry = layout.entries.iter().find(|e| e.method_name == method_name)?;

        let mut code = String::new();
        code.push_str(&format!("    ;; Call {}::{} through vtable\n", layout.trait_name, method_name));
        code.push_str(&format!("    mov rax, [{} + 8]           ;; Load vtable pointer from fat pointer\n", object_reg));
        code.push_str(&format!("    mov rbx, [rax + {}]        ;; Load method function pointer\n", entry.offset));
        code.push_str(&format!("    call rbx                    ;; Call method\n"));
        code.push_str(&format!("    mov {}, rax                ;; Store return value\n", return_reg));

        Some(code)
    }

    /// Generate code to construct a fat pointer (trait object)
    pub fn generate_fat_pointer_construction(
        data_ptr_reg: &str,      // rdi
        vtable_symbol: &str,
        dest_reg: &str,          // Where to store fat pointer
    ) -> String {
        let mut code = String::new();
        code.push_str(&format!("    ;; Construct fat pointer for dyn Trait\n"));
        code.push_str(&format!("    mov rcx, offset {}\n", vtable_symbol));
        code.push_str(&format!("    ;; Fat pointer layout: [data_ptr({}), vtable_ptr(rcx)]\n", data_ptr_reg));
        code
    }

    /// Generate code to extract data pointer from fat pointer
    pub fn extract_data_ptr(fat_ptr_reg: &str, dest_reg: &str) -> String {
        format!("    mov {}, [{}]               ;; Extract data pointer from fat pointer\n", dest_reg, fat_ptr_reg)
    }

    /// Generate code to extract vtable pointer from fat pointer
    pub fn extract_vtable_ptr(fat_ptr_reg: &str, dest_reg: &str) -> String {
        format!("    mov {}, [{} + 8]           ;; Extract vtable pointer from fat pointer\n", dest_reg, fat_ptr_reg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::vtable_generation::{VtableGenerator, VtableEntry};

    #[test]
    fn test_trait_method_call_generation() {
        let mut gen = VtableGenerator::new();
        let layout = gen.generate_vtable("Display", "String", vec!["fmt".to_string()]);

        let code = DynamicDispatchCodegen::generate_trait_method_call(&layout, "fmt", "rdi", "rax");
        assert!(code.is_some());

        let asm = code.unwrap();
        assert!(asm.contains("mov rax"));
        assert!(asm.contains("call rbx"));
    }

    #[test]
    fn test_fat_pointer_construction() {
        let code = DynamicDispatchCodegen::generate_fat_pointer_construction("rdi", "_vtable_Display_String", "rax");
        assert!(code.contains("fat pointer"));
        assert!(code.contains("_vtable_Display_String"));
    }

    #[test]
    fn test_pointer_extraction() {
        let data_code = DynamicDispatchCodegen::extract_data_ptr("rdi", "rax");
        assert!(data_code.contains("Extract data"));

        let vtable_code = DynamicDispatchCodegen::extract_vtable_ptr("rdi", "rcx");
        assert!(vtable_code.contains("Extract vtable"));
    }
}
