//! Virtual Table Generation for Dynamic Dispatch (dyn Trait)
//!
//! Generates vtables for trait objects, enabling runtime polymorphism.
//!
//! ## Vtable Layout
//! For trait object `Box<dyn Animal>`:
//! ```
//! struct VTable {
//!     drop_fn: fn(*mut ()),     // Destructor pointer
//!     size: usize,              // Size of concrete type
//!     align: usize,             // Alignment of concrete type
//!     methods: [fn_ptr; n],     // Method pointers
//! }
//! ```
//!
//! ## Object Layout
//! ```
//! struct TraitObject {
//!     data: *mut (),            // Pointer to concrete data
//!     vtable: *const VTable,    // Pointer to vtable
//! }
//! ```

use std::collections::HashMap;

/// Entry in a vtable (method pointer)
#[derive(Debug, Clone)]
pub struct VtableEntry {
    pub method_name: String,
    pub offset: usize,  // Offset in bytes from vtable start
}

/// Layout of a vtable for a specific concrete type implementing a trait
#[derive(Debug, Clone)]
pub struct VtableLayout {
    pub trait_name: String,
    pub concrete_type: String,
    pub entries: Vec<VtableEntry>,
    pub vtable_label: String,
}

/// Information about a vtable for a trait
#[derive(Debug, Clone)]
pub struct VTableInfo {
    /// Name of the trait
    pub trait_name: String,
    
    /// Methods in this vtable (method_name, method_index)
    pub methods: HashMap<String, usize>,
    
    /// Method count
    pub method_count: usize,
    
    /// Label for the vtable in assembly
    pub vtable_label: String,
}

/// Manages vtable generation for traits
pub struct VTableGenerator {
    /// Trait name -> VTable information
    vtables: HashMap<String, VTableInfo>,
    
    /// Counter for generating unique vtable labels
    vtable_counter: usize,
}

impl VTableGenerator {
    /// Create a new vtable generator
    pub fn new() -> Self {
        VTableGenerator {
            vtables: HashMap::new(),
            vtable_counter: 0,
        }
    }
    
    /// Register a trait and its methods for vtable generation
    pub fn register_trait(&mut self, trait_name: String, methods: Vec<String>) {
        let mut method_map = HashMap::new();
        for (idx, method) in methods.iter().enumerate() {
            method_map.insert(method.clone(), idx);
        }
        
        let vtable_label = format!("__vtable_{}", self.vtable_counter);
        self.vtable_counter += 1;
        
        let info = VTableInfo {
            trait_name: trait_name.clone(),
            methods: method_map,
            method_count: methods.len(),
            vtable_label,
        };
        
        self.vtables.insert(trait_name, info);
    }
    
    /// Get vtable info for a trait
    pub fn get_vtable(&self, trait_name: &str) -> Option<&VTableInfo> {
        self.vtables.get(trait_name)
    }
    
    /// Generate a vtable layout for a concrete type implementing a trait
    pub fn generate_vtable(&mut self, trait_name: &str, concrete_type: &str, methods: Vec<String>) -> VtableLayout {
        // Register the trait if not already registered
        if !self.vtables.contains_key(trait_name) {
            self.register_trait(trait_name.to_string(), methods.clone());
        }
        
        let vtable_info = self.vtables.get(trait_name).unwrap();
        
        let mut entries = Vec::new();
        for (idx, method) in methods.iter().enumerate() {
            entries.push(VtableEntry {
                method_name: method.clone(),
                offset: idx * 8, // Each vtable entry is 8 bytes (pointer)
            });
        }
        
        VtableLayout {
            trait_name: trait_name.to_string(),
            concrete_type: concrete_type.to_string(),
            entries,
            vtable_label: vtable_info.vtable_label.clone(),
        }
    }
    
    /// Generate assembly for all registered vtables
    pub fn generate_assembly(&self) -> String {
        let mut asm = String::new();
        asm.push_str("\n.section .data\n");
        
        for (_trait_name, vtable_info) in &self.vtables {
            // Generate vtable label and structure
            asm.push_str(&format!("{}:\n", vtable_info.vtable_label));
            // Placeholder: would generate actual vtable pointers here
            // For now, each vtable gets a size and method count
            asm.push_str(&format!("    .quad {}  # method count\n", vtable_info.method_count));
        }
        
        asm
    }
    
    /// Check if a trait is object-safe
    pub fn is_object_safe(&self, trait_name: &str) -> bool {
        self.vtables.contains_key(trait_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vtable_registration() {
        let mut gen = VTableGenerator::new();
        gen.register_trait(
            "Animal".to_string(),
            vec!["speak".to_string(), "move".to_string()]
        );
        
        let vtable = gen.get_vtable("Animal").unwrap();
        assert_eq!(vtable.trait_name, "Animal");
        assert_eq!(vtable.method_count, 2);
        assert_eq!(vtable.methods.get("speak"), Some(&0));
        assert_eq!(vtable.methods.get("move"), Some(&1));
    }
    
    #[test]
    fn test_vtable_generation() {
        let mut gen = VTableGenerator::new();
        let layout = gen.generate_vtable(
            "Display",
            "String",
            vec!["fmt".to_string()]
        );
        
        assert_eq!(layout.trait_name, "Display");
        assert_eq!(layout.concrete_type, "String");
        assert_eq!(layout.entries.len(), 1);
        assert_eq!(layout.entries[0].method_name, "fmt");
    }
    
    #[test]
    fn test_vtable_labels() {
        let mut gen = VTableGenerator::new();
        gen.register_trait("Trait1".to_string(), vec![]);
        gen.register_trait("Trait2".to_string(), vec![]);
        
        let t1 = gen.get_vtable("Trait1").unwrap();
        let t2 = gen.get_vtable("Trait2").unwrap();
        
        // Labels should be different
        assert_ne!(t1.vtable_label, t2.vtable_label);
    }
}
