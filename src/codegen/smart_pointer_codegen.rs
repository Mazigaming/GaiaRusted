//! # Smart Pointer Code Generation
//!
//! Generates x86-64 assembly code for smart pointer operations:
//! - Box<T> allocation and deallocation
//! - Rc<T> reference counting
//! - Arc<T> atomic operations

use crate::typesystem::{BoxType, RcType, ArcType, Type};
use std::fmt;

/// Smart pointer code generation for x86-64
pub struct SmartPointerCodegen;

impl SmartPointerCodegen {
    /// Generate code to allocate and initialize a Box<T>
    ///
    /// # Assembly generated
    /// ```asm
    /// mov rdi, <size>          ; size of T
    /// call malloc
    /// mov [rax], <init_value>  ; initialize with value
    /// ; rax contains Box pointer
    /// ```
    pub fn generate_box_new(type_name: &str, size: usize) -> String {
        let mut code = String::new();
        code.push_str(&format!("// Box::new for {}\n", type_name));
        code.push_str(&format!("mov rdi, {}\n", size));
        code.push_str("call malloc\n");
        code.push_str("; rax now contains Box pointer\n");
        code
    }

    /// Generate code to dereference a Box<T>
    ///
    /// # Assembly generated  
    /// ```asm
    /// mov rax, [rdi]  ; load value from box (rdi = box pointer)
    /// ```
    pub fn generate_box_deref() -> String {
        "mov rax, [rdi]\n".to_string()
    }

    /// Generate code to drop a Box<T>
    ///
    /// # Assembly generated
    /// ```asm
    /// mov rdi, [rsi]  ; get pointer to free (rsi = box)
    /// call free
    /// ```
    pub fn generate_box_drop() -> String {
        let mut code = String::new();
        code.push_str("// Box drop\n");
        code.push_str("mov rdi, [rsi]\n");
        code.push_str("call free\n");
        code
    }

    /// Generate code to allocate an Rc<T>
    ///
    /// # Assembly generated
    /// ```asm
    /// mov rdi, <size + 4>      ; size + refcount header (4 bytes)
    /// call malloc
    /// mov dword [rax], 1       ; initialize refcount to 1
    /// ; rax contains Rc pointer (includes refcount header)
    /// ```
    pub fn generate_rc_new(type_name: &str, size: usize) -> String {
        let mut code = String::new();
        let total_size = size + 4; // +4 for refcount
        code.push_str(&format!("// Rc::new for {}\n", type_name));
        code.push_str(&format!("mov rdi, {}\n", total_size));
        code.push_str("call malloc\n");
        code.push_str("mov dword [rax], 1\n"); // refcount = 1
        code.push_str("; rax contains Rc pointer (refcount at offset 0, data at offset 4)\n");
        code
    }

    /// Generate code to clone an Rc<T> (increment refcount)
    ///
    /// # Assembly generated
    /// ```asm
    /// mov rax, [rdi]           ; load refcount address (rdi = Rc pointer)
    /// add dword [rax], 1       ; increment refcount
    /// ; rax contains Rc pointer
    /// ```
    pub fn generate_rc_clone() -> String {
        let mut code = String::new();
        code.push_str("// Rc clone (increment refcount)\n");
        code.push_str("mov rax, [rdi]\n");
        code.push_str("add dword [rax], 1\n");
        code
    }

    /// Generate code to drop an Rc<T> (decrement and possibly free)
    ///
    /// # Assembly generated
    /// ```asm
    /// mov rax, [rsi]           ; Rc pointer (rsi = Rc)
    /// mov ecx, [rax]           ; load refcount
    /// dec ecx                  ; decrement
    /// mov [rax], ecx           ; store back
    /// test ecx, ecx            ; check if zero
    /// jnz skip_free            ; if not zero, skip free
    /// mov rdi, rax             ; set up for free
    /// call free
    /// skip_free:
    /// ```
    pub fn generate_rc_drop() -> String {
        let mut code = String::new();
        code.push_str("// Rc drop (decrement refcount, free if 0)\n");
        code.push_str("mov rax, [rsi]\n");           // Rc pointer
        code.push_str("mov ecx, [rax]\n");           // refcount
        code.push_str("dec ecx\n");                  // decrement
        code.push_str("mov [rax], ecx\n");           // store back
        code.push_str("test ecx, ecx\n");            // check if zero
        code.push_str("jnz .rc_skip_free\n");        // if not zero, skip
        code.push_str("mov rdi, rax\n");             // prepare to free
        code.push_str("call free\n");
        code.push_str(".rc_skip_free:\n");
        code
    }

    /// Generate code to allocate an Arc<T>
    ///
    /// # Assembly generated
    /// ```asm
    /// mov rdi, <size + 8>      ; size + atomic refcount header (8 bytes)
    /// call malloc
    /// mov qword [rax], 1       ; initialize atomic refcount to 1
    /// ; rax contains Arc pointer
    /// ```
    pub fn generate_arc_new(type_name: &str, size: usize) -> String {
        let mut code = String::new();
        let total_size = size + 8; // +8 for atomic refcount
        code.push_str(&format!("// Arc::new for {}\n", type_name));
        code.push_str(&format!("mov rdi, {}\n", total_size));
        code.push_str("call malloc\n");
        code.push_str("mov qword [rax], 1\n"); // atomic refcount = 1
        code.push_str("; rax contains Arc pointer (atomic refcount at offset 0, data at offset 8)\n");
        code
    }

    /// Generate code to clone an Arc<T> (atomic increment)
    ///
    /// # Assembly generated
    /// ```asm
    /// mov rax, [rdi]                   ; Arc pointer (rdi = Arc)
    /// lock add qword [rax], 1          ; atomic increment refcount
    /// ; rax contains Arc pointer
    /// ```
    pub fn generate_arc_clone() -> String {
        let mut code = String::new();
        code.push_str("// Arc clone (atomic increment)\n");
        code.push_str("mov rax, [rdi]\n");
        code.push_str("lock add qword [rax], 1\n");  // atomic increment
        code
    }

    /// Generate code to drop an Arc<T> (atomic decrement with potential free)
    ///
    /// # Assembly generated
    /// ```asm
    /// mov rax, [rsi]                   ; Arc pointer (rsi = Arc)
    /// lock sub qword [rax], 1          ; atomic decrement
    /// jnz .arc_skip_free               ; if result != 0, skip free
    /// mov rdi, rax                     ; prepare for free
    /// call free
    /// .arc_skip_free:
    /// ```
    pub fn generate_arc_drop() -> String {
        let mut code = String::new();
        code.push_str("// Arc drop (atomic decrement, free if 0)\n");
        code.push_str("mov rax, [rsi]\n");
        code.push_str("lock sub qword [rax], 1\n");  // atomic decrement
        code.push_str("jnz .arc_skip_free\n");       // if not zero, skip
        code.push_str("mov rdi, rax\n");             // prepare for free
        code.push_str("call free\n");
        code.push_str(".arc_skip_free:\n");
        code
    }

    /// Generate code to dereference smart pointers
    pub fn generate_deref(ptr_type: &str) -> String {
        match ptr_type {
            "Box" => {
                // Box is direct pointer
                "mov rax, [rdi]\n".to_string()
            }
            "Rc" => {
                // Rc: data is at offset 4 from Rc pointer
                let mut code = String::new();
                code.push_str("mov rax, [rdi]\n");     // Get Rc pointer
                code.push_str("add rax, 4\n");         // Add data offset
                code
            }
            "Arc" => {
                // Arc: data is at offset 8 from Arc pointer
                let mut code = String::new();
                code.push_str("mov rax, [rdi]\n");     // Get Arc pointer
                code.push_str("add rax, 8\n");         // Add data offset
                code
            }
            _ => String::new(),
        }
    }

    /// Generate complete allocation + initialization for Box<T>
    pub fn generate_box_initialization(type_name: &str, size: usize, init_value: &str) -> String {
        let mut code = String::new();
        code.push_str(&format!("// Initialize Box<{}>\n", type_name));
        code.push_str(&format!("mov rdi, {}\n", size));
        code.push_str("call malloc\n");
        code.push_str(&format!("mov [rax], {}\n", init_value));
        code.push_str("; rax = Box<T> pointer\n");
        code
    }

    /// Generate memory layout visualization for debugging
    pub fn generate_memory_layout_comment(ptr_type: &str, data_size: usize) -> String {
        match ptr_type {
            "Box" => {
                format!(
                    "// Memory layout for Box:\n\
                     // [Data: {} bytes]\n",
                    data_size
                )
            }
            "Rc" => {
                format!(
                    "// Memory layout for Rc:\n\
                     // [RefCount: 4 bytes] [Data: {} bytes]\n\
                     // Offset 0        Offset 4\n",
                    data_size
                )
            }
            "Arc" => {
                format!(
                    "// Memory layout for Arc:\n\
                     // [AtomicRefCount: 8 bytes] [Data: {} bytes]\n\
                     // Offset 0            Offset 8\n",
                    data_size
                )
            }
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_new_generation() {
        let code = SmartPointerCodegen::generate_box_new("i32", 4);
        assert!(code.contains("Box::new"));
        assert!(code.contains("malloc"));
    }

    #[test]
    fn test_box_deref_generation() {
        let code = SmartPointerCodegen::generate_box_deref();
        assert!(code.contains("mov rax, [rdi]"));
    }

    #[test]
    fn test_box_drop_generation() {
        let code = SmartPointerCodegen::generate_box_drop();
        assert!(code.contains("call free"));
    }

    #[test]
    fn test_rc_new_generation() {
        let code = SmartPointerCodegen::generate_rc_new("String", 24);
        assert!(code.contains("Rc::new"));
        assert!(code.contains("mov dword [rax], 1"));
        assert!(code.contains("28")); // 24 + 4
    }

    #[test]
    fn test_rc_clone_generation() {
        let code = SmartPointerCodegen::generate_rc_clone();
        assert!(code.contains("increment"));
        assert!(code.contains("add dword"));
    }

    #[test]
    fn test_rc_drop_generation() {
        let code = SmartPointerCodegen::generate_rc_drop();
        assert!(code.contains("dec ecx"));
        assert!(code.contains("call free"));
        assert!(code.contains(".rc_skip_free"));
    }

    #[test]
    fn test_arc_new_generation() {
        let code = SmartPointerCodegen::generate_arc_new("Vec", 48);
        assert!(code.contains("Arc::new"));
        assert!(code.contains("mov qword [rax], 1"));
        assert!(code.contains("56")); // 48 + 8
    }

    #[test]
    fn test_arc_clone_generation() {
        let code = SmartPointerCodegen::generate_arc_clone();
        assert!(code.contains("atomic"));
        assert!(code.contains("lock add"));
    }

    #[test]
    fn test_arc_drop_generation() {
        let code = SmartPointerCodegen::generate_arc_drop();
        assert!(code.contains("lock sub"));
        assert!(code.contains("call free"));
        assert!(code.contains(".arc_skip_free"));
    }

    #[test]
    fn test_deref_box() {
        let code = SmartPointerCodegen::generate_deref("Box");
        assert!(code.contains("mov rax, [rdi]"));
    }

    #[test]
    fn test_deref_rc() {
        let code = SmartPointerCodegen::generate_deref("Rc");
        assert!(code.contains("add rax, 4"));
    }

    #[test]
    fn test_deref_arc() {
        let code = SmartPointerCodegen::generate_deref("Arc");
        assert!(code.contains("add rax, 8"));
    }

    #[test]
    fn test_memory_layout_box() {
        let layout = SmartPointerCodegen::generate_memory_layout_comment("Box", 32);
        assert!(layout.contains("Box"));
        assert!(layout.contains("Data: 32 bytes"));
    }

    #[test]
    fn test_memory_layout_rc() {
        let layout = SmartPointerCodegen::generate_memory_layout_comment("Rc", 16);
        assert!(layout.contains("RefCount: 4 bytes"));
        assert!(layout.contains("Data: 16 bytes"));
        assert!(layout.contains("Offset 0"));
        assert!(layout.contains("Offset 4"));
    }

    #[test]
    fn test_memory_layout_arc() {
        let layout = SmartPointerCodegen::generate_memory_layout_comment("Arc", 24);
        assert!(layout.contains("AtomicRefCount"));
        assert!(layout.contains("Offset 8"));
    }
}
