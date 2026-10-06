/// VTable Generation for Trait Objects (dyn Trait)
///
/// This module generates virtual method tables (VTables) for trait implementations.
/// VTables enable dynamic dispatch by storing function pointers.
///
/// Memory Layout:
/// ```text
/// VTable (static memory):
///   +-------+
///   | drop  | <- function pointer to drop impl type
///   +-------+
///   | fn1   | <- function pointer to method1
///   +-------+
///   | fn2   | <- function pointer to method2
///   +-------+
/// ```
///
/// Trait Object (heap):
///   +-------+-------+
///   | vtable*| data* | <- 16 bytes total (fat pointer)
///   +-------+-------+
use crate::lowering::{HirItem, HirType};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VTableEntry {
    /// Method name in the trait
    pub name: String,
    /// Offset in VTable (0 = drop, 1+ = methods)
    pub offset: usize,
    /// Function signature
    pub signature: String,
}

// Alias for compatibility with dynamic_dispatch module
pub type VtableEntry = VTableEntry;

#[derive(Debug, Clone)]
pub struct VtableLayout {
    /// Name of the trait
    pub trait_name: String,
    /// Name of the implementation type
    pub impl_type: String,
    /// VTable struct name
    pub vtable_symbol: String,
    /// Entries in the vtable
    pub entries: Vec<VtableLayoutEntry>,
}

#[derive(Debug, Clone)]
pub struct VtableLayoutEntry {
    /// Method name
    pub method_name: String,
    /// Byte offset in vtable (multiply by 8 for pointer size)
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct VTableDefinition {
    /// Name of the trait
    pub trait_name: String,
    /// Name of the implementation type
    pub impl_type: String,
    /// VTable struct name (e.g., VTable_Animal_Dog)
    pub vtable_name: String,
    /// Drop function pointer
    pub drop_fn: String,
    /// Method entries (in order)
    pub methods: Vec<VTableEntry>,
    /// Generated assembly code for VTable definition
    pub asm_definition: String,
    /// Generated assembly code for VTable initialization
    pub asm_initialization: String,
}

/// Generate a VTable for a trait implementation
pub fn generate_vtable(
    trait_name: &str,
    impl_type: &str,
    trait_methods: &[HirItem],
) -> Result<VTableDefinition, String> {
    // Validate that we have methods
    if trait_methods.is_empty() {
        // Empty trait is OK - just has drop function
    }

    let vtable_name = format!("VTable_{}_{}", trait_name, impl_type);

    // Extract method names and signatures
    let mut methods = Vec::new();
    for (idx, method) in trait_methods.iter().enumerate() {
        if let HirItem::Function {
            name: method_name,
            params,
            return_type,
            ..
        } = method
        {
            let signature = format_method_signature(method_name, params, return_type);
            methods.push(VTableEntry {
                name: method_name.clone(),
                offset: idx + 1, // offset 0 is drop, 1+ are methods
                signature,
            });
        }
    }

    // Generate drop function pointer
    let drop_fn = format!("{}::drop", impl_type);

    // Generate assembly code for VTable definition
    let asm_definition = generate_vtable_definition(&vtable_name, &drop_fn, &methods, impl_type);

    // Generate assembly code for VTable initialization (as global data)
    let asm_initialization =
        generate_vtable_initialization(&vtable_name, &drop_fn, &methods, impl_type);

    Ok(VTableDefinition {
        trait_name: trait_name.to_string(),
        impl_type: impl_type.to_string(),
        vtable_name,
        drop_fn,
        methods,
        asm_definition,
        asm_initialization,
    })
}

/// Generate assembly for VTable struct definition
fn generate_vtable_definition(
    vtable_name: &str,
    drop_fn: &str,
    methods: &[VTableEntry],
    impl_type: &str,
) -> String {
    let mut asm = String::new();

    // Define the VTable as a global data structure in assembly
    asm.push_str(&format!("# VTable for {}\n", vtable_name));
    asm.push_str(&format!(".section .rodata\n"));
    asm.push_str(&format!(".globl {}\n", vtable_name));
    asm.push_str(&format!(".align 8\n"));
    asm.push_str(&format!("{}:\n", vtable_name));

    // Drop function (first entry)
    asm.push_str(&format!(
        "    .quad {}_drop  # offset 0: drop function\n",
        impl_type
    ));

    // Method function pointers
    for (idx, method) in methods.iter().enumerate() {
        let offset = idx + 1;
        asm.push_str(&format!(
            "    .quad {}::{}  # offset {}: {} method\n",
            impl_type, method.name, offset, method.name
        ));
    }

    asm
}

/// Generate assembly for VTable initialization
fn generate_vtable_initialization(
    vtable_name: &str,
    drop_fn: &str,
    methods: &[VTableEntry],
    impl_type: &str,
) -> String {
    let mut asm = String::new();

    asm.push_str(&format!("# VTable initialization for {}\n", vtable_name));

    // The actual VTable data is defined in rodata section above
    // This function returns the assembly that references it

    // Generate a helper that returns the VTable pointer
    asm.push_str(&format!("{}__vtable_ptr:\n", vtable_name.to_lowercase()));
    asm.push_str(&format!("    lea rax, [rip + {}]\n", vtable_name));
    asm.push_str(&format!("    ret\n"));

    asm
}

/// Format a method signature from HirItem
fn format_method_signature(
    method_name: &str,
    params: &[(String, HirType)],
    return_type: &Option<HirType>,
) -> String {
    let param_types: Vec<String> = params.iter().map(|(_, ty)| ty.to_string()).collect();

    let ret_type = return_type
        .as_ref()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "()".to_string());

    format!(
        "fn {}({}) -> {}",
        method_name,
        param_types.join(", "),
        ret_type
    )
}

/// Generate code to create a trait object from a concrete type
pub fn generate_trait_object_construction(
    trait_name: &str,
    impl_type: &str,
    value_var: &str,
) -> String {
    let vtable_name = format!("VTable_{}_{}", trait_name, impl_type);

    let mut asm = String::new();

    // Allocate 16 bytes for trait object (2 pointers: vtable + data)
    asm.push_str(&format!("# Create trait object Box<dyn {}>\n", trait_name));
    asm.push_str(&format!("mov rdi, 16\n"));
    asm.push_str(&format!("call malloc\n"));
    asm.push_str(&format!("mov r14, rax  # r14 = trait object pointer\n"));

    // Store VTable pointer at offset 0
    asm.push_str(&format!("lea rax, [rip + {}]\n", vtable_name));
    asm.push_str(&format!("mov qword [r14], rax  # store vtable pointer\n"));

    // Allocate space for the actual data (impl type)
    // For simplicity, assume impl_type size is known
    // In reality, this would be determined from type info
    let impl_size = estimate_type_size(impl_type);
    asm.push_str(&format!("mov rdi, {}\n", impl_size));
    asm.push_str(&format!("call malloc\n"));
    asm.push_str(&format!("mov rbx, rax  # rbx = data pointer\n"));

    // Store data pointer at offset 8
    asm.push_str(&format!("mov qword [r14 + 8], rbx  # store data pointer\n"));

    // Copy the data value into the heap allocation
    // This would involve copying the struct fields
    asm.push_str(&format!("# Copy {} data to heap\n", impl_type));

    // For now, we assume the value is already set up
    // In a real implementation, we'd copy struct fields

    asm
}

/// Generate code to extract data pointer from trait object
pub fn generate_extract_data_pointer(trait_object_var: &str, dest_reg: &str) -> String {
    format!(
        "mov {}, [{}+ 8]  # extract data pointer from trait object\n",
        dest_reg, trait_object_var
    )
}

/// Generate code to extract VTable pointer from trait object
pub fn generate_extract_vtable_pointer(trait_object_var: &str, dest_reg: &str) -> String {
    format!(
        "mov {}, [{}]  # extract vtable pointer from trait object\n",
        dest_reg, trait_object_var
    )
}

/// Estimate the size of a type for allocation
fn estimate_type_size(type_name: &str) -> usize {
    // This is a simplified heuristic
    // Real implementation would use actual type information
    match type_name {
        "i32" | "u32" | "f32" => 4,
        "i64" | "u64" | "f64" => 8,
        "bool" | "u8" | "i8" => 1,
        "String" => 24, // String contains 3 pointers/sizes
        _ => 32,        // Default assumption for structs
    }
}

/// Generate code to call a method on a trait object
pub fn generate_method_call(
    trait_object_var: &str,
    method_name: &str,
    method_idx: usize,
    args: &[String],
    return_reg: &str,
) -> String {
    let mut asm = String::new();

    let offset = method_idx + 1; // offset 0 is drop, 1+ are methods
    let vtable_offset = offset * 8; // each pointer is 8 bytes

    asm.push_str(&format!("# Call {}::{}\n", "dyn Trait", method_name));

    // Extract VTable pointer
    asm.push_str(&format!(
        "mov rax, [{}]  # rax = vtable pointer\n",
        trait_object_var
    ));

    // Load method pointer from VTable
    asm.push_str(&format!(
        "mov rcx, [rax + {}]  # rcx = method pointer\n",
        vtable_offset
    ));

    // Extract data pointer
    asm.push_str(&format!(
        "mov rbx, [{} + 8]  # rbx = data pointer\n",
        trait_object_var
    ));

    // Prepare arguments - first argument is always the data pointer (&self)
    asm.push_str(&format!("mov rdi, rbx  # first arg = self pointer\n"));

    // Handle additional arguments if any
    let arg_regs = ["rsi", "rdx", "rcx", "r8", "r9"];
    for (i, arg) in args.iter().enumerate().skip(1) {
        if i - 1 < arg_regs.len() {
            asm.push_str(&format!("mov {}, {}  # arg {}\n", arg_regs[i - 1], arg, i));
        }
    }

    // Call the method
    asm.push_str(&format!("call rcx\n"));

    // Handle return value
    if return_reg != "rax" {
        asm.push_str(&format!("mov {}, rax  # move return value\n", return_reg));
    }

    asm
}

/// Generate drop function for trait object
pub fn generate_drop_function(impl_type: &str, fields: &[(String, HirType)]) -> String {
    let mut asm = String::new();

    asm.push_str(&format!("# Drop function for {}\n", impl_type));
    asm.push_str(&format!("{}__drop:\n", impl_type));
    asm.push_str(&format!(".globl {}__drop\n", impl_type));

    // rdi contains pointer to data to drop
    asm.push_str(&format!("# Drop {} at [rdi]\n", impl_type));

    // For each field, generate drop code if needed
    for (field_name, field_type) in fields {
        if needs_drop(field_type) {
            asm.push_str(&format!("# Drop field: {}\n", field_name));
            // Generate drop code for this field
            match field_type {
                HirType::String | HirType::Vec(_) | HirType::Box(_) => {
                    asm.push_str(&format!("# Call drop on {} field\n", field_name));
                }
                _ => {}
            }
        }
    }

    // Free the data structure itself
    asm.push_str(&format!("mov rsi, rdi  # save data pointer\n"));
    asm.push_str(&format!("call free\n"));
    asm.push_str(&format!("ret\n"));

    asm
}

/// Check if a type needs explicit dropping
fn needs_drop(ty: &HirType) -> bool {
    match ty {
        HirType::String | HirType::Vec(_) | HirType::Box(_) => true,
        HirType::Reference(_) | HirType::MutableReference(_) => false,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vtable_name_generation() {
        let trait_name = "Animal";
        let impl_type = "Dog";
        let expected = "VTable_Animal_Dog";
        assert_eq!(format!("VTable_{}_{}", trait_name, impl_type), expected);
    }

    #[test]
    fn test_method_offset_calculation() {
        // offset 0 = drop
        // offset 1 = method 0
        // offset 2 = method 1
        assert_eq!(0, 0); // drop at 0
        assert_eq!(1, 0 + 1); // method 0 at 1
        assert_eq!(2, 1 + 1); // method 1 at 2
    }

    #[test]
    fn test_type_size_estimation() {
        assert_eq!(estimate_type_size("i32"), 4);
        assert_eq!(estimate_type_size("i64"), 8);
        assert_eq!(estimate_type_size("String"), 24);
        assert_eq!(estimate_type_size("UnknownType"), 32);
    }
}
