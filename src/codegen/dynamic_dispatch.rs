//! Dynamic Dispatch Code Generation for Trait Objects
//!
//! Generates assembly for calling methods through vtables (virtual method tables).
//! Implements dynamic polymorphism by indirecting method calls through function pointers.

use super::vtable_generation::{VtableEntry, VtableLayout, VtableLayoutEntry};

/// Dynamic dispatch code generator
pub struct DynamicDispatchCodegen;

impl DynamicDispatchCodegen {
    /// Generate code to load method pointer from vtable and call it
    ///
    /// This is the core of dynamic dispatch:
    /// 1. Load VTable pointer from fat pointer
    /// 2. Calculate method offset in VTable
    /// 3. Load method function pointer
    /// 4. Call the method
    /// 5. Store return value
    pub fn generate_trait_method_call(
        layout: &VtableLayout,
        method_name: &str,
        object_reg: &str, // Register containing fat pointer
        return_reg: &str, // Register for return value
    ) -> Option<String> {
        let entry = layout
            .entries
            .iter()
            .find(|e| e.method_name == method_name)?;

        let mut code = String::new();
        code.push_str(&format!(
            "    ;; Call {}::{} through vtable\n",
            layout.trait_name, method_name
        ));
        code.push_str(&format!(
            "    mov rax, [{}]               ;; Load vtable pointer from fat pointer\n",
            object_reg
        ));
        code.push_str(&format!(
            "    mov rbx, [rax + {}]         ;; Load method function pointer at offset {}\n",
            entry.offset * 8,
            entry.offset
        ));
        code.push_str(&format!(
            "    mov rdi, [{} + 8]           ;; Load data pointer as first argument (self)\n",
            object_reg
        ));
        code.push_str(&format!(
            "    call rbx                    ;; Dynamic dispatch call\n"
        ));
        code.push_str(&format!(
            "    mov {}, rax                 ;; Store return value\n",
            return_reg
        ));

        Some(code)
    }

    /// Generate code to construct a fat pointer (trait object)
    ///
    /// Fat pointer layout:
    /// - Offset 0: VTable pointer (8 bytes)
    /// - Offset 8: Data pointer (8 bytes)
    pub fn generate_fat_pointer_construction(
        data_ptr_reg: &str,  // Register containing data pointer
        vtable_symbol: &str, // VTable symbol name
        dest_reg: &str,      // Register for result fat pointer
    ) -> String {
        let mut code = String::new();
        code.push_str(&format!("    ;; Construct fat pointer for dyn Trait\n"));
        code.push_str(&format!("    lea rax, [rip + {}]\n", vtable_symbol));
        code.push_str(&format!(
            "    mov {}, rax                 ;; Store VTable pointer\n",
            dest_reg
        ));
        code.push_str(&format!("    mov [{} + 8], {}\n", dest_reg, data_ptr_reg));
        code.push_str(&format!(
            "    ;; Fat pointer now at [{}]: [vtable][data]\n",
            dest_reg
        ));
        code
    }

    /// Generate code to extract data pointer from fat pointer
    pub fn extract_data_ptr(fat_ptr_reg: &str, dest_reg: &str) -> String {
        format!(
            "    mov {}, [{} + 8]            ;; Extract data pointer from fat pointer\n",
            dest_reg, fat_ptr_reg
        )
    }

    /// Generate code to extract vtable pointer from fat pointer
    pub fn extract_vtable_ptr(fat_ptr_reg: &str, dest_reg: &str) -> String {
        format!(
            "    mov {}, [{}]                ;; Extract vtable pointer from fat pointer\n",
            dest_reg, fat_ptr_reg
        )
    }

    /// Generate code for Box<dyn Trait>::new()
    ///
    /// Steps:
    /// 1. Allocate heap memory for data
    /// 2. Copy data into heap
    /// 3. Allocate 16 bytes for fat pointer
    /// 4. Store VTable and data pointers
    /// 5. Return fat pointer in rax
    pub fn generate_box_new(
        impl_type: &str,
        trait_name: &str,
        vtable_symbol: &str,
        type_size: usize,
    ) -> String {
        let mut code = String::new();
        code.push_str(&format!(
            "    ;; Box<dyn {}>::new({})\n",
            trait_name, impl_type
        ));
        code.push_str(&format!(
            "    push rdi                    ;; Save data pointer\n"
        ));

        // Allocate data on heap
        code.push_str(&format!(
            "    mov rdi, {}                 ;; Allocate {} bytes\n",
            type_size, type_size
        ));
        code.push_str(&format!("    call malloc\n"));
        code.push_str(&format!(
            "    mov rbx, rax                ;; rbx = heap data pointer\n"
        ));

        // Allocate fat pointer
        code.push_str(&format!(
            "    mov rdi, 16                 ;; Allocate 16 bytes for fat pointer\n"
        ));
        code.push_str(&format!("    call malloc\n"));
        code.push_str(&format!(
            "    mov r14, rax                ;; r14 = fat pointer address\n"
        ));

        // Store VTable pointer
        code.push_str(&format!("    lea rax, [rip + {}]\n", vtable_symbol));
        code.push_str(&format!(
            "    mov [r14], rax              ;; Store vtable pointer at offset 0\n"
        ));

        // Store data pointer
        code.push_str(&format!(
            "    mov [r14 + 8], rbx          ;; Store data pointer at offset 8\n"
        ));
        code.push_str(&format!(
            "    mov rax, r14                ;; Return fat pointer in rax\n"
        ));

        code
    }

    /// Generate code for method call with error handling
    pub fn generate_safe_method_call(
        layout: &VtableLayout,
        method_name: &str,
        object_reg: &str,
        return_reg: &str,
    ) -> Option<String> {
        let entry = layout
            .entries
            .iter()
            .find(|e| e.method_name == method_name)?;

        let mut code = String::new();
        code.push_str(&format!(
            "    ;; Safe call {}::{}\n",
            layout.trait_name, method_name
        ));
        code.push_str(&format!(
            "    test {}, {}                 ;; Verify fat pointer not null\n",
            object_reg, object_reg
        ));
        code.push_str(&format!("    jz .L_error_{}\n", method_name));
        code.push_str(&format!(
            "    mov rax, [{}]               ;; Load vtable pointer\n",
            object_reg
        ));
        code.push_str(&format!(
            "    test rax, rax               ;; Verify vtable not null\n"
        ));
        code.push_str(&format!("    jz .L_error_{}\n", method_name));
        code.push_str(&format!(
            "    mov rbx, [rax + {}]         ;; Load method pointer\n",
            entry.offset * 8
        ));
        code.push_str(&format!(
            "    test rbx, rbx               ;; Verify method not null\n"
        ));
        code.push_str(&format!("    jz .L_error_{}\n", method_name));
        code.push_str(&format!(
            "    mov rdi, [{} + 8]           ;; Load data pointer\n",
            object_reg
        ));
        code.push_str(&format!("    call rbx                    ;; Call method\n"));
        code.push_str(&format!(
            "    mov {}, rax                 ;; Store return\n",
            return_reg
        ));
        code.push_str(&format!("    jmp .L_success_{}\n", method_name));
        code.push_str(&format!(".L_error_{}:\n", method_name));
        code.push_str(&format!(
            "    mov {}, 0                   ;; Return null on error\n",
            return_reg
        ));
        code.push_str(&format!(".L_success_{}:\n", method_name));

        Some(code)
    }

    /// Calculate offset for a method in VTable (in bytes)
    pub fn calculate_method_offset(method_index: usize) -> usize {
        // Method 0 is at offset 8 (offset 0 is drop function)
        (method_index + 1) * 8
    }

    /// Verify VTable structure and method existence
    pub fn verify_vtable(layout: &VtableLayout, method_name: &str) -> Result<usize, String> {
        layout
            .entries
            .iter()
            .find(|e| e.method_name == method_name)
            .map(|e| e.offset)
            .ok_or_else(|| {
                format!(
                    "Method {} not found in trait {}",
                    method_name, layout.trait_name
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_offset_calculation() {
        assert_eq!(DynamicDispatchCodegen::calculate_method_offset(0), 8); // First method at offset 8
        assert_eq!(DynamicDispatchCodegen::calculate_method_offset(1), 16); // Second method at offset 16
        assert_eq!(DynamicDispatchCodegen::calculate_method_offset(2), 24); // Third method at offset 24
    }

    #[test]
    fn test_fat_pointer_construction() {
        let code = DynamicDispatchCodegen::generate_fat_pointer_construction(
            "rsi",
            "VTable_Display_String",
            "rax",
        );
        assert!(code.contains("dyn Trait"));
        assert!(code.contains("VTable_Display_String"));
        assert!(code.contains("lea"));
        assert!(code.contains("mov"));
    }

    #[test]
    fn test_pointer_extraction() {
        let data_code = DynamicDispatchCodegen::extract_data_ptr("rdi", "rax");
        assert!(data_code.contains("Extract data"));
        assert!(data_code.contains("rdi + 8"));

        let vtable_code = DynamicDispatchCodegen::extract_vtable_ptr("rdi", "rcx");
        assert!(vtable_code.contains("Extract vtable"));
        assert!(vtable_code.contains("[rdi]"));
    }

    #[test]
    fn test_box_new_generation() {
        let code =
            DynamicDispatchCodegen::generate_box_new("Dog", "Animal", "VTable_Animal_Dog", 8);
        assert!(code.contains("Box<dyn Animal>::new(Dog)"));
        assert!(code.contains("malloc"));
        assert!(code.contains("VTable_Animal_Dog"));
    }
}
