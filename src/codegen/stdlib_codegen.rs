//! # Standard Library Code Generation
//!
//! Generates x86-64 code for String, Vec, and other stdlib types.
//! Handles heap allocation, memory layout, and cleanup.

use std::collections::HashMap;

/// Code generation for stdlib types
pub struct StdlibCodegen {
    /// Counter for unique allocation IDs
    allocation_counter: usize,
    /// Track allocated memory locations
    allocations: HashMap<String, AllocationInfo>,
}

/// Information about an allocation
#[derive(Clone, Debug)]
pub struct AllocationInfo {
    /// Name of the allocation
    pub name: String,
    /// Type of allocation (String, Vec<T>)
    pub alloc_type: AllocType,
    /// Whether allocation is still active
    pub active: bool,
    /// Stack location (register or memory offset)
    pub location: String,
}

/// Types of allocations
#[derive(Clone, Debug, PartialEq)]
pub enum AllocType {
    /// String allocation (24 bytes: ptr + len + cap)
    String,
    /// Vec allocation (24 bytes: ptr + len + cap)
    Vec { element_size: usize },
    /// Other allocations
    Other(String),
}

impl StdlibCodegen {
    /// Create a new stdlib codegen context
    pub fn new() -> Self {
        StdlibCodegen {
            allocation_counter: 0,
            allocations: HashMap::new(),
        }
    }

    /// Generate code for String::new()
    pub fn generate_string_new(&mut self) -> String {
        let alloc_id = self.allocation_counter;
        self.allocation_counter += 1;

        let code = format!(
            r#"
; String::new() -> String
; Returns: RAX = pointer to String (3 * 8 bytes: ptr, len, cap)
sub rsp, 24              ; allocate space for String struct
mov qword [rsp], 0       ; ptr = nullptr (null pointer)
mov qword [rsp+8], 0     ; len = 0
mov qword [rsp+16], 0    ; cap = 0
lea rax, [rsp]           ; load address into RAX
"#
        );

        self.allocations.insert(
            format!("string_{}", alloc_id),
            AllocationInfo {
                name: format!("string_{}", alloc_id),
                alloc_type: AllocType::String,
                active: true,
                location: "rax".to_string(),
            },
        );

        code
    }

    /// Generate code for String::from(ptr)
    pub fn generate_string_from(&mut self, src_ptr: &str) -> String {
        let alloc_id = self.allocation_counter;
        self.allocation_counter += 1;

        let code = format!(
            r#"
; String::from(src_ptr) -> String
; Input: {} = source string pointer
; Returns: RAX = pointer to String struct
push rax                 ; save return register
mov rax, {}              ; source pointer
call strlen              ; get length (assume strlen available)
mov rcx, rax             ; length in RCX

; Allocate heap memory
mov rdi, rcx             ; size to allocate
call malloc              ; allocate (assume malloc available)

; Create String struct on stack
sub rsp, 24
mov qword [rsp], rax     ; ptr = heap allocation
mov qword [rsp+8], rcx   ; len = strlen result
mov qword [rsp+16], rcx  ; cap = len (no extra capacity)

; Copy data
mov rdi, rax             ; destination (heap)
mov rsi, {}              ; source (input)
mov rcx, {}              ; length
rep movsb                ; copy bytes

lea rax, [rsp]
"#,
            src_ptr, src_ptr, src_ptr, src_ptr
        );

        self.allocations.insert(
            format!("string_{}", alloc_id),
            AllocationInfo {
                name: format!("string_{}", alloc_id),
                alloc_type: AllocType::String,
                active: true,
                location: "rax".to_string(),
            },
        );

        code
    }

    /// Generate code for String::push(char)
    pub fn generate_string_push(&self, string_ptr: &str, char_val: &str) -> String {
        format!(
            r#"
; String::push(char) - append character to string
; Input: {} = String pointer, {} = character
mov rax, {}              ; load String pointer
mov rcx, [rax+8]         ; load length
mov rdx, [rax+16]        ; load capacity

; Check if we need to grow
cmp rcx, rdx
jl .push_no_grow

; Need to grow: allocate double capacity
mov rdi, rdx
shl rdi, 1               ; new capacity = old * 2
call malloc              ; allocate
mov rsi, [{}+0]          ; source (old data)
mov rcx, [{}+8]          ; length
rep movsb                ; copy data

; Free old allocation (simplified)
; mov rdi, [{}+0]
; call free

; Update String struct
mov [{}+0], rax          ; update pointer
mov rdx, [{}+16]
shl rdx, 1
mov [{}+16], rdx         ; update capacity

.push_no_grow:
; Append character
mov rax, {}              ; reload String pointer
mov rcx, [rax+8]         ; reload length
mov byte [rax+rcx], {}   ; store char at [ptr + len]
inc qword [rax+8]        ; increment length
"#,
            string_ptr, char_val, string_ptr, string_ptr, string_ptr, string_ptr,
            string_ptr, string_ptr, string_ptr, string_ptr, char_val
        )
    }

    /// Generate code for String::len() -> usize
    pub fn generate_string_len(&self, string_ptr: &str) -> String {
        format!(
            r#"
; String::len() -> usize
; Input: {} = String pointer
; Returns: RAX = length
mov rax, {}              ; load String pointer
mov rax, [rax+8]         ; load length field
"#,
            string_ptr, string_ptr
        )
    }

    /// Generate code for String::pop() -> Option<char>
    pub fn generate_string_pop(&self, string_ptr: &str) -> String {
        format!(
            r#"
; String::pop() -> Option<char>
; Input: {} = String pointer
; Returns: RAX = Option (Some=char, None=0)
mov rax, {}              ; load String pointer
mov rcx, [rax+8]         ; load length
test rcx, rcx            ; check if empty
jz .pop_none

; Get last char
dec rcx                  ; length - 1
mov al, byte [rax+rcx]   ; load char at [ptr + len-1]
mov [rax+8], rcx         ; update length
jmp .pop_done

.pop_none:
xor rax, rax             ; return None (0)

.pop_done:
"#,
            string_ptr, string_ptr
        )
    }

    /// Generate code for Vec::new()
    pub fn generate_vec_new(&mut self, element_size: usize) -> String {
        let alloc_id = self.allocation_counter;
        self.allocation_counter += 1;

        let code = format!(
            r#"
; Vec<T>::new() -> Vec<T>
; Returns: RAX = pointer to Vec struct
sub rsp, 24              ; allocate space for Vec struct
mov qword [rsp], 0       ; ptr = nullptr
mov qword [rsp+8], 0     ; len = 0
mov qword [rsp+16], 0    ; cap = 0
lea rax, [rsp]           ; load address into RAX
"#
        );

        self.allocations.insert(
            format!("vec_{}", alloc_id),
            AllocationInfo {
                name: format!("vec_{}", alloc_id),
                alloc_type: AllocType::Vec {
                    element_size,
                },
                active: true,
                location: "rax".to_string(),
            },
        );

        code
    }

    /// Generate code for Vec::push(element)
    pub fn generate_vec_push(&self, vec_ptr: &str, elem_val: &str, elem_size: usize) -> String {
        format!(
            r#"
; Vec<T>::push(element)
; Input: {} = Vec pointer, {} = element value
mov rax, {}              ; load Vec pointer
mov rcx, [rax+8]         ; load length
mov rdx, [rax+16]        ; load capacity

; Check if we need to grow
cmp rcx, rdx
jl .push_vec_no_grow

; Need to grow
mov rdi, rdx
shl rdi, 1               ; new capacity = old * 2
imul rdi, {}             ; multiply by element_size
call malloc              ; allocate
mov rsi, [{}+0]          ; source (old data)
mov rcx, [{}+8]          ; length
imul rcx, {}             ; length * element_size
rep movsb                ; copy data

; Update Vec struct
mov [{}+0], rax          ; update pointer
mov rdx, [{}+16]
shl rdx, 1
mov [{}+16], rdx         ; update capacity

.push_vec_no_grow:
; Append element
mov rax, {}              ; reload Vec pointer
mov rcx, [rax+8]         ; reload length
imul rcx, {}             ; offset = length * element_size
mov qword [rax+rcx], {}  ; store element
inc qword [rax+8]        ; increment length
"#,
            vec_ptr, elem_val, vec_ptr, elem_size, vec_ptr, vec_ptr, elem_size, vec_ptr,
            vec_ptr, vec_ptr, vec_ptr, elem_size, elem_val
        )
    }

    /// Generate code for Vec::len() -> usize
    pub fn generate_vec_len(&self, vec_ptr: &str) -> String {
        format!(
            r#"
; Vec<T>::len() -> usize
; Input: {} = Vec pointer
; Returns: RAX = length
mov rax, {}              ; load Vec pointer
mov rax, [rax+8]         ; load length field
"#,
            vec_ptr, vec_ptr
        )
    }

    /// Generate code for cleanup/drop
    pub fn generate_cleanup(&self, alloc_name: &str) -> String {
        match self.allocations.get(alloc_name) {
            Some(info) => {
                match &info.alloc_type {
                    AllocType::String | AllocType::Vec { .. } => {
                        format!(
                            r#"
; Cleanup allocation: {}
; Free heap memory
mov rax, [{}+0]          ; load pointer
test rax, rax            ; check if null
jz .cleanup_skip         ; skip if null
mov rdi, rax
call free                ; free memory

.cleanup_skip:
"#,
                            alloc_name, &info.location
                        )
                    }
                    AllocType::Other(_) => String::new(),
                }
            }
            None => String::new(),
        }
    }

    /// Mark allocation as completed
    pub fn mark_complete(&mut self, alloc_name: &str) {
        if let Some(info) = self.allocations.get_mut(alloc_name) {
            info.active = false;
        }
    }

    /// Get all active allocations
    pub fn get_active_allocations(&self) -> Vec<String> {
        self.allocations
            .iter()
            .filter(|(_, info)| info.active)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

impl Default for StdlibCodegen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_new() {
        let mut codegen = StdlibCodegen::new();
        let code = codegen.generate_string_new();
        assert!(code.contains("String::new"));
        assert!(code.contains("mov qword"));
    }

    #[test]
    fn test_string_from() {
        let mut codegen = StdlibCodegen::new();
        let code = codegen.generate_string_from("rsi");
        assert!(code.contains("String::from"));
        assert!(code.contains("strlen"));
    }

    #[test]
    fn test_string_push() {
        let codegen = StdlibCodegen::new();
        let code = codegen.generate_string_push("rax", "rcx");
        assert!(code.contains("push"));
        assert!(code.contains("character"));
    }

    #[test]
    fn test_string_len() {
        let codegen = StdlibCodegen::new();
        let code = codegen.generate_string_len("rax");
        assert!(code.contains("len"));
    }

    #[test]
    fn test_string_pop() {
        let codegen = StdlibCodegen::new();
        let code = codegen.generate_string_pop("rax");
        assert!(code.contains("pop"));
    }

    #[test]
    fn test_vec_new() {
        let mut codegen = StdlibCodegen::new();
        let code = codegen.generate_vec_new(8);
        assert!(code.contains("Vec<T>::new"));
    }

    #[test]
    fn test_vec_push() {
        let codegen = StdlibCodegen::new();
        let code = codegen.generate_vec_push("rax", "rcx", 8);
        assert!(code.contains("push"));
    }

    #[test]
    fn test_vec_len() {
        let codegen = StdlibCodegen::new();
        let code = codegen.generate_vec_len("rax");
        assert!(code.contains("len"));
    }

    #[test]
    fn test_allocation_tracking() {
        let mut codegen = StdlibCodegen::new();
        codegen.generate_string_new();
        assert_eq!(codegen.get_active_allocations().len(), 1);
    }

    #[test]
    fn test_cleanup_generation() {
        let mut codegen = StdlibCodegen::new();
        codegen.generate_string_new();
        let allocs = codegen.get_active_allocations();
        let cleanup = codegen.generate_cleanup(&allocs[0]);
        assert!(cleanup.contains("Cleanup"));
        assert!(cleanup.contains("free"));
    }

    #[test]
    fn test_multiple_allocations() {
        let mut codegen = StdlibCodegen::new();
        codegen.generate_string_new();
        codegen.generate_vec_new(8);
        assert_eq!(codegen.get_active_allocations().len(), 2);
    }
}
