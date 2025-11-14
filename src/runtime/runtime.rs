//! Runtime library support
//!
//! Provides minimal runtime support needed by compiled programs:
//! - print/println functionality
//! - Memory management helpers
//! - String utilities

/// Generate runtime assembly that implements print functionality
pub fn generate_runtime_assembly() -> String {
    r#"
.section .rodata
    format_str: .string "%ld\n"
    format_str_bool: .string "%d\n"
    print_string_fmt: .string "%s"
    print_str_newline: .string "%s\n"

.section .text
.globl gaia_print_i64
.globl gaia_print_bool
.globl gaia_print_str
.globl __builtin_println

gaia_print_i64:
    push rbp
    mov rbp, rsp
    lea rsi, [rip + format_str]
    mov rdi, rsi
    mov rsi, [rbp + 16]
    call printf
    mov rsp, rbp
    pop rbp
    ret

gaia_print_bool:
    push rbp
    mov rbp, rsp
    lea rsi, [rip + format_str_bool]
    mov rdi, rsi
    mov rsi, [rbp + 16]
    call printf
    mov rsp, rbp
    pop rbp
    ret

gaia_print_str:
    push rbp
    mov rbp, rsp
    mov rsi, rdi
    lea rdi, [rip + print_string_fmt]
    call printf
    mov rsp, rbp
    pop rbp
    ret

__builtin_println:
    push rbp
    mov rbp, rsp
    mov rsi, rdi
    lea rdi, [rip + print_str_newline]
    call printf
    mov rsp, rbp
    pop rbp
    ret

__builtin_printf:
    push rbp
    mov rbp, rsp
    call printf
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
