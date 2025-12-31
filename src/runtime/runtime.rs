//! Runtime library support
//!
//! Provides minimal runtime support needed by compiled programs:
//! - print/println functionality
//! - Memory management helpers
//! - String utilities
//! - Collection operations (Vec, HashMap, HashSet)

/// Generate runtime assembly that implements print functionality and collection operations
pub fn generate_runtime_assembly() -> String {
    r#"
.section .rodata
    format_str: .string "%ld\n"
    format_str_bool: .string "%d\n"
    format_str_f64: .string "%f\n"
    print_string_fmt: .string "%s"
    print_str_newline: .string "%s\n"

.section .text
.globl gaia_print_i64
.globl gaia_print_bool
.globl gaia_print_f64
.globl gaia_print_str
.globl __builtin_println
.globl gaia_vec_new
.globl gaia_vec_push
.globl gaia_vec_pop
.globl gaia_vec_get
.globl gaia_vec_len
.globl gaia_collection_is_empty
.globl gaia_hashmap_new
.globl gaia_hashmap_insert
.globl gaia_hashmap_get
.globl gaia_hashmap_remove
.globl gaia_hashset_new
.globl gaia_hashset_insert
.globl gaia_hashset_contains
.globl gaia_hashset_remove
.globl __into_iter
.globl __next

gaia_print_i64:
    push rbp
    mov rbp, rsp
    # rdi already contains the i64 value to print
    lea rsi, [rip + format_str]
    mov rax, rdi          # Save the value in rax
    mov rdi, rsi          # format string in rdi
    mov rsi, rax          # value in rsi
    sub rsp, 8            # Align stack to 16 bytes (we pushed rbp, so subtract 8 more)
    call printf
    add rsp, 8
    mov rsp, rbp
    pop rbp
    ret

gaia_print_bool:
    push rbp
    mov rbp, rsp
    # rdi contains the bool value (0 or 1)
    lea rsi, [rip + format_str_bool]
    mov rax, rdi          # Save the value
    mov rdi, rsi          # format string in rdi
    mov rsi, rax          # value in rsi
    sub rsp, 8            # Align stack
    call printf
    add rsp, 8
    mov rsp, rbp
    pop rbp
    ret

gaia_print_f64:
    push rbp
    mov rbp, rsp
    # rdi contains the float value (64-bit, as i64 bits)
    # We need to move it to xmm0 and call printf with proper format
    lea rax, [rip + format_str_f64]
    movq xmm0, rdi        # Move 64-bit integer bits to xmm0 (as float bits)
    mov rdi, rax          # format string in rdi
    mov rax, 1            # printf needs 1 xmm argument
    sub rsp, 8            # Align stack to 16 bytes
    call printf
    add rsp, 8
    mov rsp, rbp
    pop rbp
    ret

gaia_print_str:
    push rbp
    mov rbp, rsp
    sub rsp, 8          # Align stack to 16-byte boundary for printf
    mov rsi, rdi
    lea rdi, [rip + print_string_fmt]
    call printf
    mov rsp, rbp
    pop rbp
    ret

__builtin_println:
    push rbp
    mov rbp, rsp
    sub rsp, 8          # Align stack to 16-byte boundary for printf
    mov rsi, rdi
    lea rdi, [rip + print_str_newline]
    call printf
    mov rsp, rbp
    pop rbp
    ret

__builtin_printf:
    push rbp
    mov rbp, rsp
    sub rsp, 8          # Align stack to 16-byte boundary for printf
    call printf
    mov rsp, rbp
    pop rbp
    ret

# gaia_printf_float: Helper for printing floats
# rdi = format string address
# rsi = float value as 64-bit integer (bits representation)
gaia_printf_float:
    push rbp
    mov rbp, rsp
    sub rsp, 16         # Allocate 16 bytes for alignment (8 for float, 8 for alignment)
    # Store the float bits to stack
    mov [rbp - 8], rsi  # Store float bits on stack
    # Load from stack into xmm0 as double-precision float
    movsd xmm0, [rbp - 8]  # Load 64-bit float into xmm0
    call printf
    mov rsp, rbp
    pop rbp
    ret

# Vec operations
# Vec memory layout: [capacity:i64][length:i64][...data...]
# Stack-based storage - metadata stored locally

gaia_vec_new:
    # Create new vector (stack-based)
    # This is a stub - actual Vec construction happens in codegen
    # Returns: 0 (success code)
    push rbp
    mov rbp, rsp
    xor rax, rax            # return 0
    mov rsp, rbp
    pop rbp
    ret

gaia_vec_push:
    # Push element to vector
    # rdi = vec pointer (ptr to capacity:i64, length:i64, ...data)
    # rsi = value
    # Returns: void
    push rbp
    mov rbp, rsp
    
    mov rcx, [rdi]          # get capacity
    mov r8, [rdi + 8]       # get length
    
    # Check if we need to resize (simplified - just fail if full)
    cmp r8, rcx
    jge vec_push_done
    
    # Store value at data[length]
    lea rax, [rdi + 16]     # data starts at rdi + 16
    mov [rax + r8*8], rsi   # store value at data[length]
    
    # Increment length
    inc r8
    mov [rdi + 8], r8       # update length
    
vec_push_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_vec_pop:
    # Pop element from vector
    # rdi = vec pointer
    # Returns: popped value (in rax)
    push rbp
    mov rbp, rsp
    
    mov r8, [rdi + 8]       # get length
    test r8, r8             # check if length > 0
    jz vec_pop_empty
    
    # Decrement length
    dec r8
    mov [rdi + 8], r8       # update length
    
    # Get value at data[length-1]
    lea rax, [rdi + 16]     # data starts at rdi + 16
    mov rax, [rax + r8*8]   # get value at data[length]
    jmp vec_pop_done
    
vec_pop_empty:
    xor rax, rax            # return 0 on empty
    
vec_pop_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_vec_get:
    # Get element from vector
    # rdi = vec pointer
    # rsi = index
    # Returns: value at index (in rax), or 0 if out of bounds
    push rbp
    mov rbp, rsp
    
    mov rcx, [rdi + 8]      # get length
    cmp rsi, rcx            # check if index < length
    jge vec_get_out_of_bounds
    
    lea rax, [rdi + 16]     # data starts at rdi + 16
    mov rax, [rax + rsi*8]  # get value at data[index]
    jmp vec_get_done
    
vec_get_out_of_bounds:
    xor rax, rax            # return 0 on bounds error
    
vec_get_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_vec_len:
    # Get length of vector
    # rdi = vec pointer
    # Returns: length (in rax)
    push rbp
    mov rbp, rsp
    
    mov rax, [rdi + 8]      # get length
    
    mov rsp, rbp
    pop rbp
    ret

gaia_collection_is_empty:
    # Check if any collection (Vec/HashMap/HashSet) is empty
    # All collections have size/length at offset +8
    # rdi = collection pointer
    # Returns: 1 if empty, 0 if not (in rax)
    push rbp
    mov rbp, rsp
    
    mov rax, [rdi + 8]      # get size/length (works for all collections)
    cmp rax, 0
    je collection_is_empty_true
    mov rax, 0              # not empty
    jmp collection_is_empty_done
collection_is_empty_true:
    mov rax, 1              # empty
collection_is_empty_done:
    
    mov rsp, rbp
    pop rbp
    ret

# HashMap operations (simplified)
# HashMap memory layout (stack-based): [capacity:i64][size:i64][...entries...]
# Each entry: [key:i64][value:i64]

gaia_hashmap_new:
    # Create new HashMap (stack-based stub)
    # Returns: 0 (success code)
    push rbp
    mov rbp, rsp
    xor rax, rax            # return 0
    mov rsp, rbp
    pop rbp
    ret

gaia_hashmap_insert:
    # Insert key-value pair into HashMap
    # rdi = hashmap pointer
    # rsi = key
    # rdx = value
    # Returns: void
    push rbp
    mov rbp, rsp
    
    mov rcx, [rdi + 8]      # get current size
    mov r8, rcx
    imul r8, 16             # each entry is 16 bytes
    
    # Store key and value at position size*16 + 16 (skip metadata)
    mov [rdi + 16 + r8], rsi     # key
    mov [rdi + 24 + r8], rdx     # value
    
    inc rcx
    mov [rdi + 8], rcx      # increment size
    
    mov rsp, rbp
    pop rbp
    ret

gaia_hashmap_get:
    # Get value from HashMap
    # rdi = hashmap pointer
    # rsi = key
    # Returns: value (in rax), or 0 if not found
    push rbp
    mov rbp, rsp
    
    mov rcx, [rdi + 8]      # get size
    xor r8, r8              # index = 0
    
hashmap_get_loop:
    cmp r8, rcx             # if index >= size
    jge hashmap_get_not_found
    
    # Check if key matches at position 16 + index*16
    mov r9, r8
    imul r9, 16
    mov r10, [rdi + 16 + r9] # get stored key
    cmp r10, rsi             # compare with lookup key
    je hashmap_get_found
    
    inc r8
    jmp hashmap_get_loop
    
hashmap_get_found:
    mov r9, r8
    imul r9, 16
    mov rax, [rdi + 24 + r9]  # get value
    jmp hashmap_get_done
    
hashmap_get_not_found:
    xor rax, rax            # return 0
    
hashmap_get_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_hashmap_remove:
    # Remove key from HashMap
    # rdi = hashmap pointer
    # rsi = key
    # Returns: void
    push rbp
    mov rbp, rsp
    
    # Simplified: mark as deleted (not implemented for now)
    
    mov rsp, rbp
    pop rbp
    ret

# HashSet operations (implemented using HashMap)

gaia_hashset_new:
    # Create new HashSet
    # Returns: 0 (success code)
    push rbp
    mov rbp, rsp
    xor rax, rax
    mov rsp, rbp
    pop rbp
    ret

gaia_hashset_insert:
    # Insert key into HashSet
    # rdi = hashset pointer
    # rsi = key
    # Returns: void
    push rbp
    mov rbp, rsp
    
    # Use hashmap_insert with dummy value
    mov rdx, 1              # value = 1 (arbitrary)
    call gaia_hashmap_insert
    
    mov rsp, rbp
    pop rbp
    ret

gaia_hashset_contains:
    # Check if key is in HashSet
    # rdi = hashset pointer
    # rsi = key
    # Returns: 1 if found, 0 otherwise
    push rbp
    mov rbp, rsp
    
    call gaia_hashmap_get
    
    # Convert to boolean (non-zero = 1)
    cmp rax, 0
    je hashset_contains_false
    mov rax, 1
    jmp hashset_contains_done
    
hashset_contains_false:
    xor rax, rax
    
hashset_contains_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_hashset_remove:
    # Remove key from HashSet
    # rdi = hashset pointer
    # rsi = key
    # Returns: void
    push rbp
    mov rbp, rsp
    
    call gaia_hashmap_remove
    
    mov rsp, rbp
    pop rbp
    ret

# Iterator protocol support
.data
    __current_iter_ptr: .quad 0   # Current iterator collection pointer
    __current_iter_idx: .quad 0   # Current index in iteration

.section .text

__into_iter:
    # Initialize iterator for a collection
    # rdi = collection pointer (vec metadata: capacity:i64, length:i64, data...)
    # Returns: collection pointer (same as input)
    push rbp
    mov rbp, rsp
    
    # Store the collection pointer in global state
    lea rax, [rip + __current_iter_ptr]
    mov qword ptr [rax], rdi
    
    # Initialize index to 0
    lea rax, [rip + __current_iter_idx]
    mov qword ptr [rax], 0
    
    # Return the collection pointer
    mov rax, rdi
    mov rsp, rbp
    pop rbp
    ret

__next:
    # Get next element from iterator
    # rdi = iterator/collection pointer (must match what was passed to __into_iter)
    # Returns: next element value or 0 (indicating end of iteration)
    push rbp
    mov rbp, rsp
    sub rsp, 32
    
    # Load current index
    lea rax, [rip + __current_iter_idx]
    mov r8, qword ptr [rax]
    mov qword ptr [rbp - 8], r8
    
    # Load collection length (at offset 8 from rdi)
    mov r9, qword ptr [rdi + 8]
    mov qword ptr [rbp - 16], r9
    
    # Check if index < length
    cmp r8, r9
    jge __next_end                  # if index >= length, return 0
    
    # Get element at data[index]
    # data starts at rdi + 16
    # element = *(rdi + 16 + index*8)
    lea rax, [rdi + 16]             # rax = data pointer
    mov rcx, qword ptr [rbp - 8]    # rcx = index
    mov r10, 8
    imul rcx, r10                   # rcx = index * 8
    add rax, rcx                    # rax = data + index*8
    mov rax, qword ptr [rax]        # rax = element value
    mov qword ptr [rbp - 24], rax
    
    # Increment and store index
    mov r8, qword ptr [rbp - 8]
    add r8, 1
    lea rcx, [rip + __current_iter_idx]
    mov qword ptr [rcx], r8
    
    # Return element
    mov rax, qword ptr [rbp - 24]
    mov rsp, rbp
    pop rbp
    ret
    
__next_end:
    # Return 0 to indicate end of iteration
    xor rax, rax
    mov rsp, rbp
    pop rbp
    ret
"#
    .to_string()
}

/// Generate a main function that calls the user's main entry point
pub fn generate_main_wrapper() -> String {
    r#"
.section .text
.globl main

main:
    push rbp
    mov rbp, rsp
    sub rsp, 8
    call gaia_main
    mov rsp, rbp
    pop rbp
    ret
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_generation() {
        let runtime = generate_runtime_assembly();
        assert!(runtime.contains("gaia_print_i64"));
        assert!(runtime.contains("printf"));
    }

    #[test]
    fn test_main_wrapper() {
        let main = generate_main_wrapper();
        assert!(main.contains("gaia_main"));
        assert!(main.contains("call gaia_main"));
    }
}
