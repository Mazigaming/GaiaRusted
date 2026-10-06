//! Machine-level support routines the standard library calls like C
//! functions: the atomic operations, which need a `lock` prefix or an
//! exchange instruction that no other construct of the IR produces.
//!
//! Each takes the address of the value in `rdi` and further operands in
//! `rsi` and `rdx`, as System V passes them, and returns in `rax`. Plain
//! loads and stores of aligned words are atomic on x86-64, and the locked
//! instructions are full barriers, so every routine is sequentially
//! consistent, whatever ordering the library asks for.

use crate::ir::{Callee, Program, Statement};
use std::fmt::Write;

/// The operations, as instruction sequences over a value of the size
/// given by the suffix's register names.
const OPERATIONS: [(&str, &str); 5] = [
    // load(address) -> value
    ("load", "mov {rax}, {mem}"),
    // store(address, value): an exchange, so that it is a barrier too.
    ("store", "xchg {mem}, {rsi}"),
    // swap(address, value) -> previous
    ("swap", "xchg {mem}, {rsi}\n    mov {rax}, {rsi}"),
    // fetch_add(address, value) -> previous
    ("fetch_add", "lock xadd {mem}, {rsi}\n    mov {rax}, {rsi}"),
    // compare_exchange(address, expected, new) -> previous; it succeeded
    // if the previous value is the expected one.
    ("compare_exchange", "mov {rax}, {rsi}\n    lock cmpxchg {mem}, {rdx}"),
];

/// The value sizes in bytes, with the names of `rax`, `rsi`, `rdx` and of
/// a memory operand at that size.
const SIZES: [(u32, &str, &str, &str, &str); 3] = [
    (1, "al", "sil", "dl", "byte ptr [rdi]"),
    (4, "eax", "esi", "edx", "dword ptr [rdi]"),
    (8, "rax", "rsi", "rdx", "qword ptr [rdi]"),
];

/// The symbol of one routine: `__gaia_atomic_fetch_add_8`.
fn symbol(operation: &str, size: u32) -> String {
    format!("__gaia_atomic_{operation}_{size}")
}

/// Append the routines the program calls.
pub fn emit(out: &mut String, program: &Program) {
    let mut used = std::collections::HashSet::new();
    for function in &program.functions {
        for block in &function.blocks {
            for statement in &block.statements {
                if let Statement::Call { callee: Callee::Extern { symbol, .. }, .. } = statement {
                    used.insert(symbol.as_str());
                }
            }
        }
    }
    for (operation, template) in OPERATIONS {
        for (size, rax, rsi, rdx, mem) in SIZES {
            let name = symbol(operation, size);
            if !used.contains(name.as_str()) {
                continue;
            }
            let body = template.replace("{rax}", rax).replace("{rsi}", rsi).replace("{rdx}", rdx).replace("{mem}", mem);
            let _ = write!(out, "\n{name}:\n    {body}\n    ret\n");
        }
    }
}
