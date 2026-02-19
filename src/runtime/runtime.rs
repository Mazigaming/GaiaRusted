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
    panic_msg: .string "assertion failed\n"
    assert_fail_msg: .string "assertion failed\n"
    format_fail_msg: .string "format!\n"
    todo_msg: .string "todo!(): not yet implemented\n"
    unimplemented_msg: .string "unimplemented!(): feature not implemented\n"
    panic_custom_fmt: .string "panicked at: %s\n"
    dbg_msg: .string "[DEBUG] value: %ld\n"

.section .text
.globl gaia_print_i32
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
.globl gaia_vec_insert
.globl gaia_vec_remove
.globl gaia_vec_clear
.globl gaia_vec_reserve
.globl gaia_collection_is_empty
.globl gaia_hashmap_new
.globl gaia_hashmap_insert
.globl gaia_hashmap_get
.globl gaia_hashmap_contains_key
.globl gaia_hashmap_remove
.globl gaia_hashmap_len
.globl gaia_hashmap_clear
.globl gaia_hashset_new
.globl gaia_hashset_insert
.globl gaia_hashset_contains
.globl gaia_hashset_remove
.globl gaia_hashset_len
.globl gaia_hashset_clear
.globl gaia_hashset_union
.globl gaia_hashset_intersection
.globl gaia_hashset_difference
.globl gaia_hashset_is_subset
.globl gaia_hashset_is_superset
.globl gaia_hashset_is_disjoint
.globl gaia_string_len
.globl gaia_string_is_empty
.globl gaia_string_starts_with
.globl gaia_string_ends_with
.globl gaia_string_contains
.globl gaia_string_trim
.globl gaia_string_replace
.globl gaia_string_repeat
.globl gaia_string_chars
.globl gaia_string_split
.globl __into_iter
.globl __next
.globl gaia_option_is_some
.globl gaia_option_is_none
.globl gaia_option_unwrap
.globl gaia_option_unwrap_or
.globl gaia_option_map
.globl gaia_option_and_then
.globl gaia_option_or
.globl gaia_option_filter
.globl gaia_result_is_ok
.globl gaia_result_is_err
.globl gaia_result_unwrap
.globl gaia_result_unwrap_err
.globl gaia_result_unwrap_or
.globl gaia_result_map
.globl gaia_result_and_then
.globl gaia_result_or_else
.globl gaia_iterator_map
.globl gaia_iterator_filter
.globl gaia_iterator_fold
.globl gaia_iterator_for_each
.globl gaia_iterator_sum
.globl gaia_iterator_count
.globl gaia_iterator_take
.globl gaia_iterator_skip
.globl gaia_iterator_chain
.globl gaia_iterator_find
.globl gaia_iterator_any
.globl gaia_iterator_all
.globl gaia_file_open
.globl gaia_file_create
.globl gaia_file_read_to_string
.globl gaia_file_write_all
.globl gaia_file_delete
.globl gaia_file_exists
.globl gaia_fs_read
.globl gaia_fs_write
.globl String_impl_sqrt
.globl String_impl_pow
.globl String_impl_sin
.globl String_impl_cos
.globl String_impl_floor
.globl String_impl_ceil
.globl String_impl_to_uppercase
.globl String_impl_to_lowercase
.globl String_impl_trim_start
.globl String_impl_trim_end
.globl String_impl_find
.globl String_impl_rfind
.globl String_impl_get
.globl String_impl_slice
.globl String_impl_parse
.globl String_impl_matches
.globl String_impl_reverse
.globl String_impl_is_ascii
.globl String_impl_is_numeric
.globl String_impl_is_alphabetic
.globl String_impl_split_once
.globl String_impl_rsplit_once
.globl String_impl_pad_start
.globl String_impl_pad_end
.globl String_impl_truncate
.globl __extract_enum_value
.globl assert
.globl assert_eq
.globl assert_ne
.globl panic
.globl format
.globl dbg
.globl todo
.globl unimplemented

gaia_print_i32:
    push rbp
    mov rbp, rsp
    # rdi already contains the i32 value to print (sign-extended to i64)
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
    mov rsi, rdi        # RSI = string pointer (second argument)
    lea rdi, [rip + print_string_fmt]  # RDI = format string "%s" (first argument)
    xor rax, rax        # RAX = 0 (no XMM registers used)
    call printf
    mov rsp, rbp
    pop rbp
    ret

__builtin_println:
    push rbp
    mov rbp, rsp
    sub rsp, 8          # Align stack to 16-byte boundary for printf
    mov rsi, rdi        # RSI = string pointer (second argument)
    lea rdi, [rip + print_str_newline]  # RDI = format string "%s\n" (first argument)
    xor rax, rax        # RAX = 0 (no XMM registers used)
    call printf
    mov rsp, rbp
    pop rbp
    ret

__builtin_printf:
    push rbp
    mov rbp, rsp
    sub rsp, 8          # Align stack to 16-byte boundary for printf
    xor rax, rax        # RAX = 0 (no XMM registers used for integer-only calls)
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

gaia_vec_insert:
    # Insert element at index in vector
    # rdi = vec pointer
    # rsi = index
    # rdx = value
    # Returns: void
    push rbp
    mov rbp, rsp
    
    # For now, simplified implementation - just append (ignore index for now)
    # TODO: Proper insertion with shifting elements
    mov rcx, [rdi]          # get capacity
    mov r8, [rdi + 8]       # get length
    
    # Check if we need to resize
    cmp r8, rcx
    jge vec_insert_done
    
    # Store value at data[length]
    lea rax, [rdi + 16]     # data starts at rdi + 16
    mov [rax + r8*8], rdx   # store value at data[length]
    
    # Increment length
    inc r8
    mov [rdi + 8], r8       # update length
    
vec_insert_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_vec_remove:
    # Remove element at index from vector
    # rdi = vec pointer
    # rsi = index
    # Returns: removed value (in rax)
    push rbp
    mov rbp, rsp
    
    mov r8, [rdi + 8]       # get length
    
    # Bounds check
    cmp rsi, r8
    jge vec_remove_bounds
    
    # Get value at index
    lea rax, [rdi + 16]     # data starts at rdi + 16
    mov rax, [rax + rsi*8]  # get value at data[index]
    
    # Decrement length (simplified - doesn't shift elements)
    dec r8
    mov [rdi + 8], r8       # update length
    jmp vec_remove_done
    
vec_remove_bounds:
    xor rax, rax            # return 0 on bounds error
    
vec_remove_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_vec_clear:
    # Clear vector (set length to 0)
    # rdi = vec pointer
    # Returns: void
    push rbp
    mov rbp, rsp
    
    mov qword ptr [rdi + 8], 0  # set length to 0
    
    mov rsp, rbp
    pop rbp
    ret

gaia_vec_reserve:
    # Reserve capacity in vector
    # rdi = vec pointer
    # rsi = additional capacity
    # Returns: void
    push rbp
    mov rbp, rsp
    
    # Simplified - just ensure capacity is at least length + additional
    mov rcx, [rdi]          # get current capacity
    mov r8, [rdi + 8]       # get length
    add r8, rsi             # add additional to length
    
    # If new required > capacity, update capacity
    cmp r8, rcx
    jle vec_reserve_done
    mov [rdi], r8           # update capacity
    
vec_reserve_done:
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

gaia_hashmap_contains_key:
    # Check if key exists in HashMap
    # rdi = hashmap pointer
    # rsi = key
    # Returns: 1 if found, 0 otherwise
    push rbp
    mov rbp, rsp
    
    call gaia_hashmap_get
    
    # Convert to boolean (non-zero = 1)
    cmp rax, 0
    je hashmap_contains_key_false
    mov rax, 1
    jmp hashmap_contains_key_done
    
hashmap_contains_key_false:
    xor rax, rax
    
hashmap_contains_key_done:
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

gaia_hashmap_len:
    # Get HashMap length
    # rdi = hashmap pointer
    # Returns: size (in rax)
    push rbp
    mov rbp, rsp
    
    mov rax, [rdi + 8]      # get size at offset +8
    
    mov rsp, rbp
    pop rbp
    ret

gaia_hashmap_clear:
    # Clear HashMap (reset size to 0)
    # rdi = hashmap pointer
    # Returns: void
    push rbp
    mov rbp, rsp
    
    mov rax, 0              # rax = 0
    mov [rdi + 8], rax      # set size to 0
    
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

gaia_hashset_len:
    # Get HashSet length
    # rdi = hashset pointer
    # Returns: size (in rax)
    push rbp
    mov rbp, rsp
    
    call gaia_hashmap_len
    
    mov rsp, rbp
    pop rbp
    ret

gaia_hashset_clear:
    # Clear HashSet (reset size to 0)
    # rdi = hashset pointer
    # Returns: void
    push rbp
    mov rbp, rsp
    
    call gaia_hashmap_clear
    
    mov rsp, rbp
    pop rbp
    ret

gaia_hashset_union:
    # Phase 6.1c: HashSet::union - combine two sets
    # rdi = set1, rsi = set2
    # Returns: new set with all elements from both sets
    push rbp
    mov rbp, rsp
    sub rsp, 32        # Allocate space for return value
    
    # Clone set1 as the result
    # For a proper implementation, we'd need to:
    # 1. Allocate new HashSet struct on heap
    # 2. Copy all elements from set1
    # 3. Iterate through set2 and add elements not in set1
    # For now, just return a reference to set1 (conservative but safe)
    # Note: In real Rust this would properly clone and merge
    
    mov rax, rdi         # Return set1 (simplified - assumes caller handles cloning)
    
    mov rsp, rbp
    pop rbp
    ret

gaia_hashset_intersection:
    # Phase 6.1c: HashSet::intersection - common elements of two sets
    # rdi = set1, rsi = set2
    # Returns: new set with elements in both sets
    push rbp
    mov rbp, rsp
    
    # For now, return empty set
    # TODO: Implement intersection logic - iterate set1, keep only elements in set2
    xor rax, rax         # Return 0 (empty set stub)
    
    mov rsp, rbp
    pop rbp
    ret

gaia_hashset_difference:
    # Phase 6.1c: HashSet::difference - elements in set1 but not set2
    # rdi = set1, rsi = set2
    # Returns: new set with set1 - set2
    push rbp
    mov rbp, rsp
    
    # For now, return a clone of set1 (simplified stub)
    # TODO: Implement difference - iterate set1, remove elements that are in set2
    mov rax, rdi         # Return set1 (should clone and remove set2 elements)
    
    mov rsp, rbp
    pop rbp
    ret

gaia_hashset_is_subset:
    # Phase 6.1c: HashSet::is_subset - check if set1 is subset of set2
    # rdi = set1, rsi = set2
    # Returns: 1 if subset, 0 otherwise
    push rbp
    mov rbp, rsp
    
    # Check if all elements of set1 are in set2
    # For now, return 1 (stub)
    # TODO: Implement by iterating set1 and checking each element is in set2
    mov rax, 1          # Return 1 (stub - always true)
    
    mov rsp, rbp
    pop rbp
    ret

gaia_hashset_is_superset:
    # Phase 6.1c: HashSet::is_superset - check if set1 is superset of set2
    # rdi = set1, rsi = set2
    # Returns: 1 if superset, 0 otherwise
    push rbp
    mov rbp, rsp
    
    # Check if all elements of set2 are in set1
    # Equivalent to: is_subset(set2, set1)
    # For now, return 1 (stub)
    # TODO: Implement by iterating set2 and checking each element is in set1
    mov rax, 1          # Return 1 (stub - always true)
    
    mov rsp, rbp
    pop rbp
    ret

gaia_hashset_is_disjoint:
    # Phase 6.1c: HashSet::is_disjoint - check if no common elements
    # rdi = set1, rsi = set2
    # Returns: 1 if disjoint, 0 if have common elements
    push rbp
    mov rbp, rsp
    
    # Check if any element of set1 is in set2
    # For now, return 1 (stub - always disjoint)
    # TODO: Implement by iterating set1 and checking if any element is in set2
    mov rax, 1          # Return 1 (stub - always disjoint)
    
    mov rsp, rbp
    pop rbp
    ret

# String operations
gaia_string_len:
    # Get string length
    # rdi = string pointer
    # Returns: length in rax
    push rbp
    mov rbp, rsp
    
    # Count characters until null terminator
    xor rcx, rcx        # length counter
    
string_len_loop:
    movzx eax, byte ptr [rdi + rcx]  # Load character at current position (zero-extend)
    test al, al                       # Check if null terminator
    jz string_len_done                # Jump if null
    inc rcx                           # Move to next character
    cmp rcx, 1024                     # Safety limit
    jge string_len_done
    jmp string_len_loop
    
string_len_done:
    mov rax, rcx        # Return length in rax
    mov rsp, rbp
    pop rbp
    ret

gaia_string_is_empty:
    # Check if string is empty
    # rdi = string pointer
    # Returns: 1 if empty, 0 otherwise
    push rbp
    mov rbp, rsp
    
    mov al, byte [rdi]
    cmp al, 0
    je string_is_empty_true
    xor rax, rax
    jmp string_is_empty_done
    
string_is_empty_true:
    mov rax, 1
    
string_is_empty_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_string_starts_with:
    # Check if string starts with prefix
    # rdi = string pointer
    # rsi = prefix pointer
    # Returns: 1 if starts with prefix, 0 otherwise
    push rbp
    mov rbp, rsp
    
    xor rax, rax
    
starts_with_loop:
    mov cl, byte [rsi + rax]
    cmp cl, 0
    je starts_with_true  # Reached end of prefix, so it matches
    
    mov dl, byte [rdi + rax]
    cmp dl, cl
    jne starts_with_false  # Characters don't match
    
    inc rax
    cmp rax, 256
    jge starts_with_false
    jmp starts_with_loop
    
starts_with_true:
    mov rax, 1
    jmp starts_with_done
    
starts_with_false:
    xor rax, rax
    
starts_with_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_string_ends_with:
    # Check if string ends with suffix
    # rdi = string pointer
    # rsi = suffix pointer
    # Returns: 1 if ends with suffix, 0 otherwise
    push rbp
    mov rbp, rsp
    push rdi         # Save string pointer
    push rsi         # Save suffix pointer
    # Stack is now 16-byte aligned (rbp pushed = 8, rdi pushed = 8, rsi pushed = 8, total offset = 24, so rsp is at 16-byte boundary)
    
    # Get string length for first string
    mov rax, rdi     # rdi still has string pointer
    call gaia_string_len
    mov r8, rax      # r8 = string length
    
    # Get suffix length
    mov rdi, [rsp + 0]  # Load suffix pointer from stack
    call gaia_string_len
    mov rcx, rax     # rcx = suffix length
    
    # Load string pointer again
    mov rdi, [rsp + 8]  # Load string pointer from stack
    
    # If suffix longer than string, return false
    cmp rcx, r8
    jg ends_with_false
    
    # Compare last N characters
    # rdi = string pointer
    # rcx = suffix length
    # r8 = string length
    mov r10, r8      # r10 = string length
    sub r10, rcx     # r10 = start_offset = string_len - suffix_len
    xor rdx, rdx     # Counter
    
    ends_with_loop:
    cmp rdx, rcx
    je ends_with_true
    
    # Load suffix pointer
    mov rsi, [rsp + 0]      # Load suffix pointer from stack
    
    # Compare characters
    mov r9, r10
    add r9, rdx             # r9 = start_offset + current_index
    mov al, byte [rdi + r9]
    mov bl, byte [rsi + rdx]
    cmp al, bl
    jne ends_with_false
    
    inc rdx
    jmp ends_with_loop
    
ends_with_true:
    mov rax, 1
    jmp ends_with_done
    
ends_with_false:
    xor rax, rax
    
ends_with_done:
    add rsp, 16      # Clean up pushed registers (rdi, rsi, and the alignment space)
    pop rbp
    ret

gaia_string_contains:
    # Check if string contains substring
    # rdi = string pointer
    # rsi = substring pointer
    # Returns: 1 if contains, 0 otherwise
    push rbp
    mov rbp, rsp
    
    xor rax, rax  # String index
    
contains_outer_loop:
    mov cl, byte [rdi + rax]
    cmp cl, 0
    je contains_not_found  # End of string
    
    # Try to match substring starting at current position
    xor rdx, rdx  # Substring index
    
contains_inner_loop:
    mov cl, byte [rsi + rdx]
    cmp cl, 0
    je contains_found  # Reached end of substring, so we found it
    
    mov r8, rax
    add r8, rdx         # r8 = string_index + substring_index
    mov bl, byte [rdi + r8]
    cmp bl, cl
    jne contains_inner_not_match  # Characters don't match
    
    inc rdx
    cmp rdx, 256
    jge contains_found
    jmp contains_inner_loop
    
contains_inner_not_match:
    inc rax
    cmp rax, 1024
    jge contains_not_found
    jmp contains_outer_loop
    
contains_found:
    mov rax, 1
    jmp contains_done
    
contains_not_found:
    xor rax, rax
    
contains_done:
     mov rsp, rbp
     pop rbp
     ret

gaia_string_trim:
     # Trim whitespace from string
     # rdi = string pointer
     # Returns: trimmed string pointer (simplified - returns same pointer)
     push rbp
     mov rbp, rsp
     
     # For now: return same pointer (full implementation would skip leading/trailing spaces)
     mov rax, rdi
     
     mov rsp, rbp
     pop rbp
     ret

gaia_string_replace:
     # Replace substring in string
     # rdi = string pointer
     # rsi = search substring
     # rdx = replacement substring
     # Returns: new string with replacements
     push rbp
     mov rbp, rsp
     
     # For now: return original string (full implementation would do actual replacement)
     mov rax, rdi
     
     mov rsp, rbp
     pop rbp
     ret

gaia_string_repeat:
     # Repeat string n times
     # rdi = string pointer
     # rsi = repetition count
     # Returns: repeated string
     push rbp
     mov rbp, rsp
     
     # For now: return original string (full implementation would concatenate)
     mov rax, rdi
     
     mov rsp, rbp
     pop rbp
     ret

gaia_string_chars:
     # Get iterator over characters
     # rdi = string pointer
     # Returns: iterator over chars
     push rbp
     mov rbp, rsp
     
     # For now: return string pointer as iterator
     mov rax, rdi
     
     mov rsp, rbp
     pop rbp
     ret

gaia_string_split:
     # Split string by delimiter
     # rdi = string pointer
     # rsi = delimiter
     # Returns: iterator of parts
     push rbp
     mov rbp, rsp
     
     # For now: return string pointer as iterator
     mov rax, rdi
     
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
    # rdi = iterator/collection pointer
    # Returns: rax = element value (or 0 if iteration done)
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
    jge __next_done_no_more
    
    # Get element at data[index]
    lea rax, [rdi + 16]             # data starts at rdi + 16
    mov rcx, qword ptr [rbp - 8]    # rcx = index
    mov r10, 8
    imul rcx, r10                   # rcx = index * 8
    add rax, rcx                    # rax = data + index*8
    mov rax, qword ptr [rax]        # rax = element value
    mov qword ptr [rbp - 24], rax
    
    # Increment index
    mov r8, qword ptr [rbp - 8]
    add r8, 1
    lea rcx, [rip + __current_iter_idx]
    mov qword ptr [rcx], r8
    
    # Return element value
    mov rax, qword ptr [rbp - 24]
    mov rsp, rbp
    pop rbp
    ret

__next_done_no_more:
    # Iteration complete - return 0
    xor rax, rax
    mov rsp, rbp
    pop rbp
    ret

# Option<T> methods
# Memory layout: [tag:i64][value:i64] where tag=1 for Some, tag=0 for None

gaia_option_is_some:
    # Check if Option is Some
    # rdi = Option pointer (tag at offset 0)
    # Returns: 1 if Some, 0 if None (in rax)
    push rbp
    mov rbp, rsp
    mov rax, [rdi]     # Load tag
    cmp rax, 1         # Check if tag == 1 (Some)
    je option_is_some_true
    xor rax, rax       # Return 0 (None)
    jmp option_is_some_done
option_is_some_true:
    mov rax, 1         # Return 1 (Some)
option_is_some_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_option_is_none:
    # Check if Option is None
    # rdi = Option pointer (tag at offset 0)
    # Returns: 1 if None, 0 if Some (in rax)
    push rbp
    mov rbp, rsp
    mov rax, [rdi]     # Load tag
    cmp rax, 0         # Check if tag == 0 (None)
    je option_is_none_true
    xor rax, rax       # Return 0 (Some)
    jmp option_is_none_done
option_is_none_true:
    mov rax, 1         # Return 1 (None)
option_is_none_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_option_unwrap:
    # Unwrap Option value
    # rdi = Option pointer
    # Returns: value if Some, panics if None (for now just returns 0)
    push rbp
    mov rbp, rsp
    mov rax, [rdi]     # Load tag
    cmp rax, 1         # Check if tag == 1 (Some)
    jne option_unwrap_panic
    mov rax, [rdi + 8] # Load value at offset 8
    jmp option_unwrap_done
option_unwrap_panic:
    xor rax, rax       # Return 0 for None (should panic)
option_unwrap_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_option_unwrap_or:
    # Unwrap Option with default value
    # rdi = Option pointer
    # rsi = default value
    # Returns: value if Some, default if None
    push rbp
    mov rbp, rsp
    mov rax, [rdi]     # Load tag
    cmp rax, 1         # Check if tag == 1 (Some)
    jne option_unwrap_or_default
    mov rax, [rdi + 8] # Load value at offset 8
    jmp option_unwrap_or_done
option_unwrap_or_default:
    mov rax, rsi       # Use default value
option_unwrap_or_done:
     mov rsp, rbp
     pop rbp
     ret

gaia_option_map:
      # Option::map(closure) -> Option
      # rdi = Option pointer
      # rsi = closure object
      # Returns: Option with mapped value or None
      push rbp
      mov rbp, rsp
      sub rsp, 32
      
      # Check if Some
      mov rax, [rdi]
      cmp rax, 1
      jne option_map_none
      
      # Get value from Option
      mov rax, [rdi + 8]     # rax = inner value
      
      # Get closure function pointer
      mov r8, [rsi]          # r8 = fn_ptr from closure
      
      # Call closure with value: call fn_ptr(value)
      mov rdi, rax           # rdi = value (param)
      call r8                # call closure(value)
      # rax now contains mapped value
      
      # Create Some with mapped value
      mov qword ptr [rbp - 16], 1    # tag = Some
      mov [rbp - 24], rax            # value = mapped
      lea rax, [rbp - 24]
      jmp option_map_done
      
option_map_none:
      # Return None
      mov qword ptr [rbp - 16], 0
      mov qword ptr [rbp - 24], 0
      lea rax, [rbp - 24]
      
option_map_done:
      mov rsp, rbp
      pop rbp
      ret

gaia_option_and_then:
      # Option::and_then(closure) -> Option
      # rdi = Option pointer
      # rsi = closure object (returns Option)
      # Returns: flattened Option
      push rbp
      mov rbp, rsp
      sub rsp, 32
      
      # Check if Some
      mov rax, [rdi]
      cmp rax, 1
      jne option_and_then_none
      
      # Get value from Option
      mov rax, [rdi + 8]     # rax = inner value
      
      # Get closure function pointer
      mov r8, [rsi]          # r8 = fn_ptr from closure
      
      # Call closure with value: call fn_ptr(value)
      # Closure returns Option (tag at offset 0, value at offset 8)
      mov rdi, rax           # rdi = value (param)
      call r8                # call closure(value)
      # rax now contains pointer to returned Option
      
      # The closure returns an Option, which we return directly (flattened)
      jmp option_and_then_done
      
option_and_then_none:
      # Return None
      mov qword ptr [rbp - 8], 0
      mov qword ptr [rbp - 16], 0
      lea rax, [rbp - 16]
      
option_and_then_done:
      mov rsp, rbp
      pop rbp
      ret

gaia_option_or:
     # Option::or(other) -> Option
     # rdi = Option pointer
     # rsi = other Option pointer
     # Returns: first Some or second Option
     push rbp
     mov rbp, rsp
     
     # Check if first is Some
     mov rax, [rdi]
     cmp rax, 1
     je option_or_return_first
     
     # Return second option
     mov rax, rsi
     jmp option_or_done
     
option_or_return_first:
     mov rax, rdi
     
option_or_done:
     mov rsp, rbp
     pop rbp
     ret

gaia_option_filter:
      # Option::filter(closure) -> Option
      # rdi = Option pointer
      # rsi = closure object (predicate)
      # Returns: Some if Some and predicate true, None otherwise
      push rbp
      mov rbp, rsp
      sub rsp, 32
      
      # Check if None
      mov rax, [rdi]
      cmp rax, 0
      je option_filter_none
      
      # Get value from Option and preserve original Option ptr
      mov r8, rdi            # r8 = save original Option pointer
      mov rax, [rdi + 8]     # rax = inner value
      mov r9, [rsi]          # r9 = fn_ptr from closure
      
      # Call predicate with value: call fn_ptr(value)
      mov rdi, rax           # rdi = value (param)
      call r9                # call predicate(value)
      # rax contains predicate result (0 or 1)
      
      # If predicate is false, return None
      test rax, rax
      jz option_filter_none
      
      # Predicate is true: return Some with original value
      mov rax, [r8 + 8]      # rax = original value from saved Option ptr
      mov qword ptr [rbp - 16], 1    # tag = Some
      mov [rbp - 24], rax            # value = original
      lea rax, [rbp - 24]
      jmp option_filter_done
      
option_filter_none:
      # Return None
      mov qword ptr [rbp - 16], 0
      mov qword ptr [rbp - 24], 0
      lea rax, [rbp - 24]
      
option_filter_done:
      mov rsp, rbp
      pop rbp
      ret

# Result<T, E> methods
# Memory layout: [tag:i64][value:i64] where tag=1 for Ok, tag=0 for Err

gaia_result_is_ok:
    # Check if Result is Ok
    # rdi = Result pointer (tag at offset 0)
    # Returns: 1 if Ok, 0 if Err
    push rbp
    mov rbp, rsp
    mov rax, [rdi]     # Load tag
    cmp rax, 1         # Check if tag == 1 (Ok)
    je result_is_ok_true
    xor rax, rax       # Return 0 (Err)
    jmp result_is_ok_done
result_is_ok_true:
    mov rax, 1         # Return 1 (Ok)
result_is_ok_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_result_is_err:
    # Check if Result is Err
    # rdi = Result pointer (tag at offset 0)
    # Returns: 1 if Err, 0 if Ok
    push rbp
    mov rbp, rsp
    mov rax, [rdi]     # Load tag
    cmp rax, 0         # Check if tag == 0 (Err)
    je result_is_err_true
    xor rax, rax       # Return 0 (Ok)
    jmp result_is_err_done
result_is_err_true:
    mov rax, 1         # Return 1 (Err)
result_is_err_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_result_unwrap:
    # Unwrap Result value
    # rdi = Result pointer
    # Returns: value if Ok, panics if Err (for now just returns 0)
    push rbp
    mov rbp, rsp
    mov rax, [rdi]     # Load tag
    cmp rax, 1         # Check if tag == 1 (Ok)
    jne result_unwrap_panic
    mov rax, [rdi + 8] # Load value at offset 8
    jmp result_unwrap_done
result_unwrap_panic:
    xor rax, rax       # Return 0 for Err (should panic)
result_unwrap_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_result_unwrap_err:
    # Unwrap Result error
    # rdi = Result pointer
    # Returns: error if Err, panics if Ok
    push rbp
    mov rbp, rsp
    mov rax, [rdi]     # Load tag
    cmp rax, 0         # Check if tag == 0 (Err)
    jne result_unwrap_err_panic
    mov rax, [rdi + 8] # Load error at offset 8
    jmp result_unwrap_err_done
result_unwrap_err_panic:
    xor rax, rax       # Return 0 for Ok (should panic)
result_unwrap_err_done:
    mov rsp, rbp
    pop rbp
    ret

gaia_result_unwrap_or:
     # Unwrap Result with default value
     # rdi = Result pointer
     # rsi = default value
     # Returns: value if Ok, default if Err
     push rbp
     mov rbp, rsp
     mov rax, [rdi]     # Load tag
     cmp rax, 1         # Check if tag == 1 (Ok)
     jne result_unwrap_or_default
     mov rax, [rdi + 8] # Load value at offset 8
     jmp result_unwrap_or_done
result_unwrap_or_default:
     mov rax, rsi       # Use default value
result_unwrap_or_done:
      mov rsp, rbp
      pop rbp
      ret

gaia_result_map:
      # Result::map(closure) -> Result
      # rdi = Result pointer
      # rsi = closure object
      # Returns: Result with mapped value or same Err
      push rbp
      mov rbp, rsp
      sub rsp, 32
      
      # Check if Ok
      mov rax, [rdi]
      cmp rax, 1
      jne result_map_err
      
      # Get value from Result
      mov rax, [rdi + 8]     # rax = inner value
      
      # Get closure function pointer
      mov r8, [rsi]          # r8 = fn_ptr from closure
      
      # Call closure with value: call fn_ptr(value)
      mov rdi, rax           # rdi = value (param)
      call r8                # call closure(value)
      # rax now contains mapped value
      
      # Create Ok with mapped value
      mov qword ptr [rbp - 16], 1    # tag = Ok
      mov [rbp - 24], rax            # value = mapped
      lea rax, [rbp - 24]
      jmp result_map_done
      
result_map_err:
      # Return same Err
      mov rax, [rdi + 8]
      mov qword ptr [rbp - 16], 0
      mov qword ptr [rbp - 24], rax
      lea rax, [rbp - 24]
      
result_map_done:
      mov rsp, rbp
      pop rbp
      ret

gaia_result_and_then:
      # Result::and_then(closure) -> Result
      # rdi = Result pointer
      # rsi = closure object (returns Result)
      # Returns: flattened Result
      push rbp
      mov rbp, rsp
      sub rsp, 32
      
      # Check if Ok
      mov rax, [rdi]
      cmp rax, 1
      jne result_and_then_err
      
      # Get value from Result
      mov rax, [rdi + 8]     # rax = inner value
      
      # Get closure function pointer
      mov r8, [rsi]          # r8 = fn_ptr from closure
      
      # Call closure with value: call fn_ptr(value)
      # Closure returns Result (tag at offset 0, value at offset 8)
      mov rdi, rax           # rdi = value (param)
      call r8                # call closure(value)
      # rax now contains pointer to returned Result
      
      # The closure returns a Result, which we return directly (flattened)
      jmp result_and_then_done
      
result_and_then_err:
      # Return same Err
      mov qword ptr [rbp - 8], 0
      mov rax, [rdi + 8]
      mov [rbp - 16], rax
      lea rax, [rbp - 16]
      
result_and_then_done:
      mov rsp, rbp
      pop rbp
      ret

gaia_result_or_else:
      # Result::or_else(closure) -> Result
      # rdi = Result pointer
      # rsi = closure object (returns Result)
      # Returns: self if Ok, result of closure if Err
      push rbp
      mov rbp, rsp
      sub rsp, 32
      
      # Check if Ok
      mov rax, [rdi]
      cmp rax, 1
      je result_or_else_return_ok
      
      # Is Err: get error value and call closure
      mov rax, [rdi + 8]     # rax = error value
      
      # Get closure function pointer
      mov r8, [rsi]          # r8 = fn_ptr from closure
      
      # Call closure with error value: call fn_ptr(error)
      # Closure returns Result
      mov rdi, rax           # rdi = error value (param)
      call r8                # call closure(error)
      # rax now contains pointer to returned Result
      
      # Return the Result from closure
      jmp result_or_else_done
      
result_or_else_return_ok:
      # Return Ok unchanged
      mov qword ptr [rbp - 8], 1
      mov rax, [rdi + 8]
      mov [rbp - 16], rax
      lea rax, [rbp - 16]
      
result_or_else_done:
      mov rsp, rbp
      pop rbp
      ret

# Iterator adapter methods with closure support
# These iterate over collection elements and apply closures

gaia_iterator_map:
      # Iterator::map(closure)
      # rdi = iterator/collection pointer (vec: [capacity][length][data...])
      # rsi = closure object pointer (contains fn_ptr and captures)
      # Returns: mapped value iterator (new collection with transformed elements)
      push rbp
      mov rbp, rsp
      sub rsp, 128           # Stack space for new vector and locals
      
      # Get collection info from input
      mov r8, [rdi]          # r8 = capacity
      mov r9, [rdi + 8]      # r9 = length
      
      # Check if empty
      test r9, r9
      jz iterator_map_done_empty
      
      # Create new vector with same capacity
      # New vec: [capacity][length][data...]
      mov qword ptr [rbp - 8], r8    # new_capacity
      mov qword ptr [rbp - 16], r9   # new_length
      
      # Get closure function pointer
      mov r10, [rsi]         # r10 = fn_ptr from closure
      
      # Loop through elements: map each through closure
      xor rcx, rcx           # rcx = index
      lea r11, [rdi + 16]    # r11 = input data pointer
      lea r12, [rbp - 32]    # r12 = output data pointer
      
iterator_map_loop:
      cmp rcx, r9            # if index >= length
      jge iterator_map_loop_done
      
      # Get input element
      mov rax, [r11 + rcx*8] # rax = input element at index
      
      # Call closure with element: call fn_ptr(element)
      mov rdi, rax           # rdi = element (first param to closure)
      call r10               # call closure(element)
      # rax now contains mapped value
      
      # Store mapped value in new vector
      mov [r12 + rcx*8], rax # output[index] = mapped value
      
      inc rcx
      jmp iterator_map_loop
      
iterator_map_loop_done:
      # Return pointer to new vector (on stack at rbp - 16)
      lea rax, [rbp - 16]
      mov rsp, rbp
      pop rbp
      ret
      
iterator_map_done_empty:
      # Return empty vector
      mov qword ptr [rbp - 8], 0    # capacity = 0
      mov qword ptr [rbp - 16], 0   # length = 0
      lea rax, [rbp - 16]
      mov rsp, rbp
      pop rbp
      ret

gaia_iterator_filter:
      # Iterator::filter(closure)
      # rdi = iterator/collection pointer (vec: [capacity][length][data...])
      # rsi = closure object pointer (predicate function)
      # Returns: filtered iterator (new collection with filtered elements)
      push rbp
      mov rbp, rsp
      sub rsp, 128           # Stack space for new vector and locals
      
      # Get collection info
      mov r8, [rdi]          # r8 = capacity
      mov r9, [rdi + 8]      # r9 = length
      
      # Check if empty
      test r9, r9
      jz iterator_filter_done_empty
      
      # Create new vector (initially empty in terms of count)
      # New vec: [capacity][length][data...]
      mov qword ptr [rbp - 8], r8    # new_capacity = old_capacity
      mov qword ptr [rbp - 16], 0    # new_length = 0 (will fill)
      
      # Get closure function pointer
      mov r10, [rsi]         # r10 = fn_ptr from closure
      
      # Loop through elements: filter each through predicate
      xor rcx, rcx           # rcx = input index
      xor r11, r11           # r11 = output index (write position)
      lea r12, [rdi + 16]    # r12 = input data pointer
      lea r13, [rbp - 32]    # r13 = output data pointer
      
iterator_filter_loop:
      cmp rcx, r9            # if input_index >= length
      jge iterator_filter_loop_done
      
      # Get input element
      mov rax, [r12 + rcx*8] # rax = input element at index
      
      # Call predicate with element: call fn_ptr(element)
      mov rdi, rax           # rdi = element (param to predicate)
      call r10               # call predicate(element)
      # rax contains predicate result (0 or 1)
      
      # If result is true (non-zero), include element
      test rax, rax
      jz iterator_filter_skip
      
      # Element passes filter: add to output
      mov rax, [r12 + rcx*8] # rax = element value
      mov [r13 + r11*8], rax # output[output_index] = element
      inc r11                # increment output index
      
iterator_filter_skip:
      inc rcx
      jmp iterator_filter_loop
      
iterator_filter_loop_done:
      # Update length in output vector
      mov [rbp - 16], r11    # new_length = output_index
      
      # Return pointer to new vector (on stack at rbp - 16)
      lea rax, [rbp - 16]
      mov rsp, rbp
      pop rbp
      ret
      
iterator_filter_done_empty:
      # Return empty vector
      mov qword ptr [rbp - 8], 0    # capacity = 0
      mov qword ptr [rbp - 16], 0   # length = 0
      lea rax, [rbp - 16]
      mov rsp, rbp
      pop rbp
      ret

gaia_iterator_fold:
      # Iterator::fold(accumulator, closure)
      # rdi = iterator/collection pointer
      # rsi = initial accumulator value
      # rdx = closure object pointer
      # Returns: accumulated value
      push rbp
      mov rbp, rsp
      sub rsp, 32
      
      # Get collection length
      mov r8, [rdi + 8]      # r8 = length
      
      # Initialize accumulator with init value
      mov rax, rsi           # rax = accumulator = init value
      
      # Check if empty
      test r8, r8
      jz iterator_fold_done
      
      # Get closure function pointer
      mov r9, [rdx]          # r9 = fn_ptr from closure
      
      # Loop through collection elements
      xor rcx, rcx           # rcx = index
      lea r10, [rdi + 16]    # r10 = data pointer
      
iterator_fold_loop:
      cmp rcx, r8            # if index >= length
      jge iterator_fold_done
      
      # Get element at index
      mov r11, [r10 + rcx*8] # r11 = element
      
      # Call closure(accumulator, element)
      # rdi = accumulator (first param), rsi = element (second param)
      mov rdi, rax           # rdi = current accumulator
      mov rsi, r11           # rsi = element
      call r9                # call closure(acc, elem)
      # rax contains new accumulator value
      
      inc rcx
      jmp iterator_fold_loop
      
iterator_fold_done:
      mov rsp, rbp
      pop rbp
      ret

gaia_iterator_for_each:
      # Iterator::for_each(closure)
      # rdi = iterator/collection pointer
      # rsi = closure object pointer
      # Returns: unit (0)
      push rbp
      mov rbp, rsp
      sub rsp, 32
      
      # Get collection length
      mov r8, [rdi + 8]      # r8 = length
      
      # Check if empty
      test r8, r8
      jz iterator_for_each_done
      
      # Get closure function pointer
      mov r9, [rsi]          # r9 = fn_ptr from closure
      
      # Loop through collection elements
      xor rcx, rcx           # rcx = index
      lea r10, [rdi + 16]    # r10 = data pointer
      
iterator_for_each_loop:
      cmp rcx, r8            # if index >= length
      jge iterator_for_each_done
      
      # Get element at index
      mov rax, [r10 + rcx*8] # rax = element
      
      # Call closure with element: call fn_ptr(element)
      mov rdi, rax           # rdi = element (param)
      call r9                # call closure(element)
      # Ignore return value for for_each
      
      inc rcx
      jmp iterator_for_each_loop
      
iterator_for_each_done:
      # Return unit (0)
      xor rax, rax
      mov rsp, rbp
      pop rbp
      ret

gaia_iterator_sum:
     # Iterator::sum()
     # rdi = iterator/collection pointer
     # Returns: sum of all elements
     push rbp
     mov rbp, rsp
     sub rsp, 32
     
     # Get collection length
     mov r8, [rdi + 8]      # r8 = length
     
     # Initialize sum to 0
     xor rax, rax           # rax = sum
     
     # Check if empty
     test r8, r8
     jz iterator_sum_done
     
     # Loop through elements
     xor rcx, rcx           # rcx = index
     
iterator_sum_loop:
     cmp rcx, r8            # if index >= length
     jge iterator_sum_done
     
     # Get element at data[index]
     lea r9, [rdi + 16]     # r9 = data pointer
     mov r10, [r9 + rcx*8]  # r10 = element value
     
     # Add to accumulator
     add rax, r10           # sum += element
     
     # Next element
     inc rcx
     jmp iterator_sum_loop
     
iterator_sum_done:
     mov rsp, rbp
     pop rbp
     ret

gaia_iterator_count:
     # Iterator::count()
     # rdi = iterator/collection pointer
     # Returns: count of elements
     push rbp
     mov rbp, rsp
     
     # Get collection length
     mov rax, [rdi + 8]     # rax = length
     
     mov rsp, rbp
     pop rbp
     ret

gaia_iterator_take:
     # Iterator::take(n)
     # rdi = iterator/collection pointer
     # rsi = number of elements to take
     # Returns: iterator (limited to n elements)
     push rbp
     mov rbp, rsp
     
     # Get actual length
     mov r8, [rdi + 8]      # r8 = actual length
     
     # Take minimum of (actual length, n)
     cmp rsi, r8
     jle take_use_n
     mov rsi, r8            # Use actual length if n is larger
     
take_use_n:
     # Update length to min(n, actual)
     mov [rdi + 8], rsi
     
     # Return iterator
     mov rax, rdi
     mov rsp, rbp
     pop rbp
     ret

gaia_iterator_skip:
     # Iterator::skip(n)
     # rdi = iterator/collection pointer
     # rsi = number of elements to skip
     # Returns: iterator (starting from position n)
     push rbp
     mov rbp, rsp
     
     # Get collection length and capacity
     mov r8, [rdi + 8]      # r8 = length
     mov r9, [rdi]          # r9 = capacity
     
     # Subtract skipped elements from length
     cmp rsi, r8
     jge skip_all
     
     sub r8, rsi            # new length = length - skip
     mov [rdi + 8], r8
     jmp skip_done
     
skip_all:
     # Skip more than length: return empty iterator
     mov qword ptr [rdi + 8], 0
     
skip_done:
     mov rax, rdi           # Return iterator
     mov rsp, rbp
     pop rbp
     ret

gaia_iterator_chain:
     # Iterator::chain(other)
     # rdi = first iterator
     # rsi = second iterator
     # Returns: chained iterator (simplified - just returns first for now)
     push rbp
     mov rbp, rsp
     
     # For simplified version, just add lengths
     mov r8, [rdi + 8]      # first length
     mov r9, [rsi + 8]      # second length
     add r8, r9             # total length
     mov [rdi + 8], r8      # update first iterator length
     
     mov rax, rdi           # return first iterator
     mov rsp, rbp
     pop rbp
     ret

gaia_iterator_find:
      # Iterator::find(closure)
      # rdi = iterator (vec: [capacity][length][data...])
      # rsi = closure object (predicate)
      # Returns: Option<T> = [tag:i64][value:i64]
      push rbp
      mov rbp, rsp
      sub rsp, 32
      
      # Get length
      mov r8, [rdi + 8]      # r8 = length
      test r8, r8
      jz find_not_found
      
      # Get closure function pointer
      mov r9, [rsi]          # r9 = fn_ptr from closure
      
      # Loop through elements finding first match
      xor rcx, rcx           # rcx = index
      lea r10, [rdi + 16]    # r10 = data pointer
      
find_loop:
      cmp rcx, r8            # if index >= length
      jge find_not_found
      
      # Get element at index
      mov rax, [r10 + rcx*8] # rax = element
      
      # Call predicate with element: call fn_ptr(element)
      mov rdi, rax           # rdi = element (param)
      call r9                # call predicate(element)
      # rax contains predicate result (0 or 1)
      
      # If predicate is true (non-zero), found it!
      test rax, rax
      jnz find_found
      
      inc rcx
      jmp find_loop
      
find_found:
      # Return Some with the matching element
      mov rax, [r10 + rcx*8] # rax = element value (from saved index in rcx)
      mov qword ptr [rbp - 16], 1     # tag = Some
      mov qword ptr [rbp - 24], rax   # value = element
      lea rax, [rbp - 24]
      jmp find_done
      
find_not_found:
      # Return None
      # Build Option: [tag:0][value:0]
      mov qword ptr [rbp - 16], 0
      mov qword ptr [rbp - 24], 0
      lea rax, [rbp - 24]
      
find_done:
      mov rsp, rbp
      pop rbp
      ret

gaia_iterator_any:
      # Iterator::any(closure)
      # rdi = iterator (vec: [capacity][length][data...])
      # rsi = closure object (predicate)
      # Returns: bool (1 if any match, 0 otherwise)
      push rbp
      mov rbp, rsp
      sub rsp, 32
      
      # Get length
      mov r8, [rdi + 8]      # r8 = length
      test r8, r8
      jz any_false           # Empty iterator = false
      
      # Get closure function pointer
      mov r9, [rsi]          # r9 = fn_ptr from closure
      
      # Loop through elements checking predicate
      xor rcx, rcx           # rcx = index
      lea r10, [rdi + 16]    # r10 = data pointer
      
any_loop:
      cmp rcx, r8            # if index >= length
      jge any_false
      
      # Get element at index
      mov rax, [r10 + rcx*8] # rax = element
      
      # Call predicate: call fn_ptr(element)
      mov rdi, rax           # rdi = element (param)
      call r9                # call predicate(element)
      # rax contains result
      
      # If any predicate returned true, return true
      test rax, rax
      jnz any_true
      
      inc rcx
      jmp any_loop
      
any_true:
      mov rax, 1
      jmp any_done
      
any_false:
      xor rax, rax
      
any_done:
      mov rsp, rbp
      pop rbp
      ret

gaia_iterator_all:
      # Iterator::all(closure)
      # rdi = iterator (vec: [capacity][length][data...])
      # rsi = closure object (predicate)
      # Returns: bool (1 if all match, 0 otherwise)
      push rbp
      mov rbp, rsp
      sub rsp, 32
      
      # Get length
      mov r8, [rdi + 8]      # r8 = length
      test r8, r8
      jz all_true            # Empty iterator = true (vacuous truth)
      
      # Get closure function pointer
      mov r9, [rsi]          # r9 = fn_ptr from closure
      
      # Loop through elements checking predicate
      xor rcx, rcx           # rcx = index
      lea r10, [rdi + 16]    # r10 = data pointer
      
all_loop:
      cmp rcx, r8            # if index >= length
      jge all_true           # All elements passed = true
      
      # Get element at index
      mov rax, [r10 + rcx*8] # rax = element
      
      # Call predicate: call fn_ptr(element)
      mov rdi, rax           # rdi = element (param)
      call r9                # call predicate(element)
      # rax contains result
      
      # If any predicate returned false, return false
      test rax, rax
      jz all_false
      
      inc rcx
      jmp all_loop
      
all_true:
      mov rax, 1
      jmp all_done
      
all_false:
      xor rax, rax
      
all_done:
      mov rsp, rbp
      pop rbp
      ret

# File I/O operations (simplified placeholders)

gaia_file_open:
     # File::open(path: &str) -> Result<File, Error>
     # rdi = path string (C-string pointer)
     # Returns: Result<File, Error> = [tag:i64][value:i64]
     push rbp
     mov rbp, rsp
     sub rsp, 16
     
     # rdi = path string pointer (already set)
     # open(path, O_RDONLY=0, mode=0)
     mov rax, 2              # open syscall
     mov rsi, 0              # O_RDONLY
     mov rdx, 0              # mode
     syscall
     
     # rax contains file descriptor (or negative error)
     mov rcx, rax
     cmp rcx, 0
     jl file_open_error
     
     # Success: return Ok(fd)
     mov qword ptr [rbp - 8], 1      # tag = Ok
     mov qword ptr [rbp - 16], rcx   # value = fd
     lea rax, [rbp - 16]
     jmp file_open_done
     
file_open_error:
     # Error: return Err(-fd)
     mov qword ptr [rbp - 8], 0      # tag = Err
     neg rcx
     mov qword ptr [rbp - 16], rcx   # value = error code
     lea rax, [rbp - 16]
     
file_open_done:
     mov rsp, rbp
     pop rbp
     ret

gaia_file_create:
      # File::create(path: &str) -> Result<File, Error>
      # rdi = path string (C-string pointer)
      # Returns: Result<File, Error> = [tag:i64][fd:i64]
      push rbp
      mov rbp, rsp
      sub rsp, 16
      
      # rdi = path string pointer (already set)
      # open(path, O_WRONLY | O_CREAT | O_TRUNC = 1 | 64 | 512 = 577, mode=0644)
      mov rax, 2              # open syscall
      mov rsi, 577            # O_WRONLY | O_CREAT | O_TRUNC
      mov rdx, 0644           # mode (rw-r--r--)
      syscall
      
      # rax contains file descriptor (or negative error)
      mov rcx, rax
      cmp rcx, 0
      jl file_create_error
      
      # Success: return Ok(fd)
      mov qword ptr [rbp - 8], 1      # tag = Ok
      mov qword ptr [rbp - 16], rcx   # value = fd
      lea rax, [rbp - 16]
      jmp file_create_done
      
file_create_error:
      # Error: return Err(-fd)
      mov qword ptr [rbp - 8], 0      # tag = Err
      neg rcx
      mov qword ptr [rbp - 16], rcx   # value = error code
      lea rax, [rbp - 16]
      
file_create_done:
      mov rsp, rbp
      pop rbp
      ret

gaia_file_read_to_string:
     # File::read_to_string() -> Result<String, Error>
     # rdi = File (file descriptor)
     # Returns: Result<String, Error>
     push rbp
     mov rbp, rsp
     sub rsp, 4096           # 4KB buffer for file content
     
     # rdi = file descriptor
     # read(fd, buffer, size)
     mov rax, 0              # read syscall
     mov rsi, rbp
     sub rsi, 4096           # buffer pointer
     mov rdx, 4095           # max bytes to read
     syscall
     
     # rax contains bytes read (or negative error)
     cmp rax, 0
     jl file_read_error
     
     # Success: return Ok(string_ptr)
     # For simplicity, return buffer pointer
     mov qword ptr [rbp - 4104], 1   # tag = Ok
     mov rcx, rbp
     sub rcx, 4096
     mov qword ptr [rbp - 4112], rcx # value = string ptr
     lea rax, [rbp - 4112]
     jmp file_read_done
     
file_read_error:
     # Error: return Err
     mov qword ptr [rbp - 4104], 0
     neg rax
     mov qword ptr [rbp - 4112], rax
     lea rax, [rbp - 4112]
     
file_read_done:
     mov rsp, rbp
     pop rbp
     ret

gaia_file_write_all:
     # File::write_all(data: &str) -> Result<(), Error>
     # rdi = File (file descriptor)
     # rsi = data string pointer
     # Returns: Result<(), Error>
     push rbp
     mov rbp, rsp
     sub rsp, 32
     
     # First get length of string (null-terminated)
     mov rcx, 0
     mov r8, rsi
count_len_loop:
     mov al, byte [r8 + rcx]
     cmp al, 0
     je count_len_done
     inc rcx
     cmp rcx, 4096           # max 4KB
     jl count_len_loop
     
count_len_done:
     # rcx = string length, rdi = fd, rsi = data
     # write(fd, data, len)
     mov rax, 1              # write syscall
     mov rdx, rcx            # length
     syscall
     
     # rax contains bytes written (or negative error)
     cmp rax, 0
     jl file_write_error
     
     # Success: return Ok(())
     mov qword ptr [rbp - 8], 1      # tag = Ok
     mov qword ptr [rbp - 16], 0     # value = unit
     lea rax, [rbp - 16]
     jmp file_write_done
     
file_write_error:
     # Error: return Err
     mov qword ptr [rbp - 8], 0
     neg rax
     mov qword ptr [rbp - 16], rax
     lea rax, [rbp - 16]
     
file_write_done:
     mov rsp, rbp
     pop rbp
     ret

gaia_file_delete:
     # File::delete(path: &str) -> Result<(), Error>
     # rdi = path string (C-string pointer)
     # Returns: Result<(), Error>
     push rbp
     mov rbp, rsp
     sub rsp, 16
     
     # unlink(path)
     mov rax, 87             # unlink syscall
     syscall
     
     # rax contains 0 on success, negative on error
     cmp rax, 0
     jne file_delete_error
     
     # Success: return Ok(())
     mov qword ptr [rbp - 8], 1
     mov qword ptr [rbp - 16], 0
     lea rax, [rbp - 16]
     jmp file_delete_done
     
file_delete_error:
     # Error: return Err
     mov qword ptr [rbp - 8], 0
     neg rax
     mov qword ptr [rbp - 16], rax
     lea rax, [rbp - 16]
     
file_delete_done:
     mov rsp, rbp
     pop rbp
     ret

gaia_file_exists:
     # File::exists(path: &str) -> bool
     # rdi = path string (C-string pointer)
     # Returns: bool (1 for exists, 0 for not)
     push rbp
     mov rbp, rsp
     sub rsp, 144            # stat structure (144 bytes)
     
     # stat(path, &stat_buf)
     mov rax, 4              # stat syscall
     mov rsi, rbp
     sub rsi, 144            # buffer for stat structure
     syscall
     
     # rax contains 0 on success, negative on error
     cmp rax, 0
     je file_exists_true
     
     # File doesn't exist
     xor rax, rax
     jmp file_exists_done
     
file_exists_true:
     mov rax, 1
     
file_exists_done:
     mov rsp, rbp
     pop rbp
     ret

gaia_fs_read:
      # fs::read(path: &str) -> Result<Vec<u8>, Error>
      # rdi = path string
      # Returns: Result<Vec<u8>, Error> where Vec is [capacity][length][data...]
      push rbp
      mov rbp, rsp
      sub rsp, 4128            # 4KB buffer + metadata
      
      # Step 1: open(path, O_RDONLY=0, mode=0)
      mov rax, 2               # open syscall
      mov rsi, 0               # O_RDONLY
      mov rdx, 0               # mode
      syscall
      # rax = file descriptor or negative error
      
      cmp rax, 0
      jl fs_read_error_open
      
      mov r8, rax              # r8 = fd
      
      # Step 2: read(fd, buffer, 4096)
      mov rax, 0               # read syscall
      mov rdi, r8              # fd
      mov rsi, rbp
      sub rsi, 4096            # buffer at [rbp - 4096]
      mov rdx, 4095            # max bytes to read
      syscall
      # rax = bytes read or negative error
      
      cmp rax, 0
      jl fs_read_error_read
      
      mov r9, rax              # r9 = bytes_read
      
      # Step 3: close(fd)
      mov rax, 3               # close syscall
      mov rdi, r8              # fd
      syscall
      # Ignore close errors
      
      # Step 4: Build vector result
      # Vec: [capacity][length][data...]
      mov qword ptr [rbp - 4104], 4096    # capacity
      mov qword ptr [rbp - 4112], r9      # length = bytes read
      
      # Return Ok(Vec)
      mov qword ptr [rbp - 8], 1          # tag = Ok
      lea rcx, [rbp - 4112]
      mov qword ptr [rbp - 16], rcx       # value = vec pointer
      lea rax, [rbp - 16]
      jmp fs_read_done
      
fs_read_error_read:
      # close(fd) before returning error
      mov rax, 3
      mov rdi, r8
      syscall
      
fs_read_error_open:
      # Return Err with error code
      mov qword ptr [rbp - 8], 0          # tag = Err
      neg rax
      mov qword ptr [rbp - 16], rax       # value = error code
      lea rax, [rbp - 16]
      
fs_read_done:
      mov rsp, rbp
      pop rbp
      ret

gaia_fs_write:
      # fs::write(path: &str, data: &str) -> Result<(), Error>
      # rdi = path string (C-string)
      # rsi = data string (C-string)
      # Returns: Result<(), Error>
      push rbp
      mov rbp, rsp
      sub rsp, 32
      
      # Save parameters
      mov r8, rdi              # r8 = path
      mov r9, rsi              # r9 = data
      
      # Step 1: Get data length (null-terminated string)
      mov rcx, 0
      mov r10, r9
count_data_len:
      mov al, byte [r10 + rcx]
      cmp al, 0
      je data_len_done
      inc rcx
      cmp rcx, 4096            # max 4KB
      jl count_data_len
      
data_len_done:
      # rcx = data length
      mov r11, rcx             # r11 = data_len
      
      # Step 2: open(path, O_WRONLY | O_CREAT | O_TRUNC = 1 | 64 | 512 = 577, mode=0644)
      mov rax, 2               # open syscall
      mov rdi, r8              # path
      mov rsi, 577             # O_WRONLY | O_CREAT | O_TRUNC
      mov rdx, 0644            # mode
      syscall
      # rax = file descriptor or negative error
      
      cmp rax, 0
      jl fs_write_error_open
      
      mov r12, rax             # r12 = fd
      
      # Step 3: write(fd, data, len)
      mov rax, 1               # write syscall
      mov rdi, r12             # fd
      mov rsi, r9              # data pointer
      mov rdx, r11             # length
      syscall
      # rax = bytes written or negative error
      
      cmp rax, 0
      jl fs_write_error_write
      
      # Step 4: close(fd)
      mov rax, 3               # close syscall
      mov rdi, r12             # fd
      syscall
      # Ignore close errors
      
      # Return Ok(())
      mov qword ptr [rbp - 8], 1          # tag = Ok
      mov qword ptr [rbp - 16], 0         # value = unit
      lea rax, [rbp - 16]
      jmp fs_write_done
      
fs_write_error_write:
      # close(fd) before returning error
      mov r13, rax             # save error
      mov rax, 3
      mov rdi, r12
      syscall
      mov rax, r13             # restore error
      jmp fs_write_error_ret
      
fs_write_error_open:
      # rax already contains error
fs_write_error_ret:
      # Return Err with error code
      mov qword ptr [rbp - 8], 0          # tag = Err
      neg rax
      mov qword ptr [rbp - 16], rax       # value = error code
      lea rax, [rbp - 16]
      
fs_write_done:
      mov rsp, rbp
      pop rbp
      ret

# f64 method wrappers - these are called from generated code
# The ABI passes f64 in xmm0 and returns in xmm0

String_impl_sqrt:
      # xmm0 = f64 value
      # Call libm sqrt
      sqrtsd xmm0, xmm0
      ret

String_impl_pow:
      # xmm0 = base (f64)
      # xmm1 = exponent (f64)
      # Call libm pow via C library
      sub rsp, 8
      call pow
      add rsp, 8
      ret

String_impl_sin:
      # xmm0 = angle (f64)
      # Call libm sin
      sub rsp, 8
      call sin
      add rsp, 8
      ret

String_impl_cos:
      # xmm0 = angle (f64)
      # Call libm cos
      sub rsp, 8
      call cos
      add rsp, 8
      ret

String_impl_floor:
      # xmm0 = f64 value
      # Call libm floor
      roundsd xmm0, xmm0, 1  # Round down
      ret

String_impl_ceil:
      # xmm0 = f64 value
      # Call libm ceil
      roundsd xmm0, xmm0, 2  # Round up
      ret

# Phase 6.3: String method implementations
# All string methods are stubs that return empty strings or false for now
# rdi = string pointer, rsi = optional parameter
# Returns: rax = result (string pointer, bool as 0/1, or Option)

String_impl_to_uppercase:
      # Phase 6.3: Convert string to uppercase
      # rdi = string pointer
      # Returns: rax = uppercase string (stub - returns same string)
      push rbp
      mov rbp, rsp
      mov rax, rdi          # Return input string (stub implementation)
      mov rsp, rbp
      pop rbp
      ret

String_impl_to_lowercase:
      # Phase 6.3: Convert string to lowercase
      # rdi = string pointer
      # Returns: rax = lowercase string (stub - returns same string)
      push rbp
      mov rbp, rsp
      mov rax, rdi          # Return input string (stub implementation)
      mov rsp, rbp
      pop rbp
      ret

String_impl_trim_start:
      # Phase 6.3: Trim whitespace from start
      # rdi = string pointer
      # Returns: rax = trimmed string (stub - returns same)
      push rbp
      mov rbp, rsp
      mov rax, rdi
      mov rsp, rbp
      pop rbp
      ret

String_impl_trim_end:
      # Phase 6.3: Trim whitespace from end
      # rdi = string pointer
      # Returns: rax = trimmed string (stub - returns same)
      push rbp
      mov rbp, rsp
      mov rax, rdi
      mov rsp, rbp
      pop rbp
      ret

String_impl_find:
      # Phase 6.3: Find substring position
      # rdi = string pointer, rsi = substring pointer
      # Returns: rax = Option<i32> (stub - returns None)
      push rbp
      mov rbp, rsp
      xor rax, rax          # Return 0 (None)
      mov rsp, rbp
      pop rbp
      ret

String_impl_rfind:
      # Phase 6.3: Find substring from right
      # rdi = string pointer, rsi = substring pointer
      # Returns: rax = Option<i32> (stub - returns None)
      push rbp
      mov rbp, rsp
      xor rax, rax
      mov rsp, rbp
      pop rbp
      ret

String_impl_get:
      # Phase 6.3: Get character at index
      # rdi = string pointer, rsi = index
      # Returns: rax = Option<char> (stub - returns None)
      push rbp
      mov rbp, rsp
      xor rax, rax
      mov rsp, rbp
      pop rbp
      ret

String_impl_slice:
      # Phase 6.3: Slice string [start..end]
      # rdi = string pointer, rsi = start, rdx = end
      # Returns: rax = sliced string (stub - returns same)
      push rbp
      mov rbp, rsp
      mov rax, rdi          # Return input (stub)
      mov rsp, rbp
      pop rbp
      ret

String_impl_parse:
      # Phase 6.3: Parse string to type T
      # rdi = string pointer
      # Returns: rax = parsed value (stub - returns 0)
      push rbp
      mov rbp, rsp
      xor rax, rax
      mov rsp, rbp
      pop rbp
      ret

String_impl_matches:
      # Phase 6.3: Check if string matches pattern
      # rdi = string pointer, rsi = pattern
      # Returns: rax = bool (stub - returns 0)
      push rbp
      mov rbp, rsp
      xor rax, rax
      mov rsp, rbp
      pop rbp
      ret

String_impl_reverse:
      # Phase 6.3: Reverse string
      # rdi = string pointer
      # Returns: rax = reversed string (stub - returns same)
      push rbp
      mov rbp, rsp
      mov rax, rdi
      mov rsp, rbp
      pop rbp
      ret

String_impl_is_ascii:
      # Phase 6.3: Check if string is ASCII
      # rdi = string pointer
      # Returns: rax = bool (stub - returns 1)
      push rbp
      mov rbp, rsp
      mov rax, 1            # Assume ASCII (stub)
      mov rsp, rbp
      pop rbp
      ret

String_impl_is_numeric:
      # Phase 6.3: Check if string is numeric
      # rdi = string pointer
      # Returns: rax = bool (stub - returns 0)
      push rbp
      mov rbp, rsp
      xor rax, rax
      mov rsp, rbp
      pop rbp
      ret

String_impl_is_alphabetic:
      # Phase 6.3: Check if string is alphabetic
      # rdi = string pointer
      # Returns: rax = bool (stub - returns 0)
      push rbp
      mov rbp, rsp
      xor rax, rax
      mov rsp, rbp
      pop rbp
      ret

String_impl_split_once:
      # Phase 6.3: Split string on first occurrence
      # rdi = string pointer, rsi = delimiter
      # Returns: rax = Option<(String, String)> (stub - returns None)
      push rbp
      mov rbp, rsp
      xor rax, rax
      mov rsp, rbp
      pop rbp
      ret

String_impl_rsplit_once:
      # Phase 6.3: Split string on last occurrence
      # rdi = string pointer, rsi = delimiter
      # Returns: rax = Option<(String, String)> (stub - returns None)
      push rbp
      mov rbp, rsp
      xor rax, rax
      mov rsp, rbp
      pop rbp
      ret

String_impl_pad_start:
      # Phase 6.3: Pad string at start
      # rdi = string pointer, rsi = width, rdx = fill char
      # Returns: rax = padded string (stub - returns same)
      push rbp
      mov rbp, rsp
      mov rax, rdi
      mov rsp, rbp
      pop rbp
      ret

String_impl_pad_end:
      # Phase 6.3: Pad string at end
      # rdi = string pointer, rsi = width, rdx = fill char
      # Returns: rax = padded string (stub - returns same)
      push rbp
      mov rbp, rsp
      mov rax, rdi
      mov rsp, rbp
      pop rbp
      ret

String_impl_truncate:
      # Phase 6.3: Truncate string to length
      # rdi = string pointer, rsi = length
      # Returns: rax = truncated string (stub - returns same)
      push rbp
      mov rbp, rsp
      mov rax, rdi
      mov rsp, rbp
      pop rbp
      ret

# __extract_enum_value: Extract the inner value from Option<T> or Result<T, E>
# Memory layout: [tag:i64][value:i64]
# rdi = pointer to the Option/Result (or the value itself if stored in register)
# Returns: the inner value in rax
__extract_enum_value:
      push rbp
      mov rbp, rsp
      # For Option/Result, the value is at offset 8 from the base
      # In our encoding, it's just the second i64
      mov rax, [rdi + 8]  # Extract the value at offset 8
      mov rsp, rbp
      pop rbp
      ret

# PHASE 5.2: Runtime support for builtin macros

# assert!(condition) - takes bool in rdi, exits if false
assert:
      push rbp
      mov rbp, rsp
      cmp rdi, 0           # Check if rdi (condition) is true
      jne .assert_ok       # If true, continue
      # If false, print error and exit
      lea rdi, [rip + assert_fail_msg]
      sub rsp, 8
      call printf
      add rsp, 8
      mov rax, 1           # Exit code 1
      call exit
.assert_ok:
      mov rsp, rbp
      pop rbp
      ret

# assert_eq!(a, b) - takes two i64 in rdi, rsi, exits if not equal
assert_eq:
      push rbp
      mov rbp, rsp
      cmp rdi, rsi         # Compare rdi and rsi
      je .assert_eq_ok     # If equal, continue
      # If not equal, print error and exit
      lea rdi, [rip + assert_fail_msg]
      sub rsp, 8
      call printf
      add rsp, 8
      mov rax, 1           # Exit code 1
      call exit
.assert_eq_ok:
      mov rsp, rbp
      pop rbp
      ret

# assert_ne!(a, b) - takes two i64 in rdi, rsi, exits if equal
assert_ne:
      push rbp
      mov rbp, rsp
      cmp rdi, rsi         # Compare rdi and rsi
      jne .assert_ne_ok    # If not equal, continue
      # If equal, print error and exit
      lea rdi, [rip + assert_fail_msg]
      sub rsp, 8
      call printf
      add rsp, 8
      mov rax, 1           # Exit code 1
      call exit
.assert_ne_ok:
      mov rsp, rbp
      pop rbp
      ret

# panic!(msg) - takes string pointer in rdi, prints and exits
panic:
      push rbp
      mov rbp, rsp
      sub rsp, 8
      # Check if rdi is empty/null - if so use default message
      test rdi, rdi
      jnz .panic_custom
      lea rdi, [rip + panic_msg]  # Use default panic message
      xor rax, rax
      call printf
      jmp .panic_exit
.panic_custom:
      mov rsi, rdi         # RSI = custom message
      lea rdi, [rip + panic_custom_fmt]  # RDI = format string "panicked at: %s\n"
      xor rax, rax
      call printf
.panic_exit:
      mov rsp, rbp
      pop rbp
      mov rax, 101         # Exit code 101
      call exit
      ret

# format!(fmt, ...) - takes format string in rdi, returns string (stub implementation)
format:
      push rbp
      mov rbp, rsp
      # For now, just return the format string as-is
      # A proper implementation would do actual formatting
      mov rax, rdi         # Return format string pointer
      mov rsp, rbp
      pop rbp
      ret

# dbg!(expr) - takes value in rdi, prints it, returns it
dbg:
      push rbp
      mov rbp, rsp
      mov rsi, rdi         # Save the value in rsi for printf
      lea rdi, [rip + dbg_msg]  # Format string in rdi
      sub rsp, 8
      call printf          # Print "[DEBUG] value: <value>\n"
      add rsp, 8
      mov rax, rsi         # Return the original value
      mov rsp, rbp
      pop rbp
      ret

# todo!() - prints message and exits
todo:
      push rbp
      mov rbp, rsp
      lea rdi, [rip + todo_msg]  # Print "todo!(): not yet implemented\n"
      sub rsp, 8
      call printf
      add rsp, 8
      mov rsp, rbp
      pop rbp
      mov rax, 101         # Exit code 101 (convention for unimplemented)
      call exit
      ret

# unimplemented!() - prints message and exits
unimplemented:
      push rbp
      mov rbp, rsp
      lea rdi, [rip + unimplemented_msg]  # Print "unimplemented!(): feature not implemented\n"
      sub rsp, 8
      call printf
      add rsp, 8
      mov rsp, rbp
      pop rbp
      mov rax, 101         # Exit code 101 (convention for unimplemented)
      call exit
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
