//! The x86-64 backend: IR in, GNU assembler text (Intel syntax) out.
//!
//! * [`abi`] — how values are classified, passed and returned
//! * [`regalloc`] — which locals live in registers
//! * [`function`] — code for one function
//! * [`reg`] — registers and memory operands
//! * [`runtime`] — the atomic operations the standard library calls
//!
//! The output links against the C library, which supplies process startup
//! and the handful of system services the standard library builds on.

mod abi;
mod function;
mod reg;
mod regalloc;
mod runtime;

use crate::ir::opt::OptLevel;
use crate::ir::{Data, DataItem, Program};
use crate::sema::context::Context;
use std::fmt::Write;

/// Generate the assembly for a whole program.
pub fn emit(program: &Program, tcx: &Context, level: OptLevel) -> String {
    let mut out = String::from(".intel_syntax noprefix\n.text\n");

    // The C runtime calls `main`; it sets up the statics computed at run
    // time, then runs the program's own `main`.
    out.push_str("\n.globl main\nmain:\n    push rbp\n    mov rbp, rsp\n");
    for initializer in &program.initializers {
        let _ = writeln!(out, "    call {}", program.functions[initializer.0 as usize].symbol);
    }
    let entry = &program.functions[program.entry.0 as usize].symbol;
    let _ = write!(out, "    call {entry}\n    xor eax, eax\n    pop rbp\n    ret\n");

    for index in 0..program.functions.len() {
        function::FnEmitter::new(&mut out, tcx, program, index, level).emit();
    }
    runtime::emit(&mut out, program);

    for writable in [false, true] {
        out.push_str(if writable { "\n.data\n" } else { "\n.section .rodata\n" });
        for data in program.data.iter().filter(|data| data.writable == writable) {
            emit_data(&mut out, program, data);
        }
    }

    // Tell the linker this code never needs an executable stack.
    out.push_str("\n.section .note.GNU-stack,\"\",@progbits\n");
    out
}

fn emit_data(out: &mut String, program: &Program, data: &Data) {
    let _ = writeln!(out, ".balign {}\n{}:", data.align, data.symbol);
    for item in &data.items {
        match item {
            DataItem::Bytes(bytes) => {
                for line in bytes.chunks(16) {
                    let values: Vec<String> = line.iter().map(u8::to_string).collect();
                    let _ = writeln!(out, "    .byte {}", values.join(", "));
                }
            }
            DataItem::Word(value) => {
                let _ = writeln!(out, "    .quad {value}");
            }
            DataItem::FuncAddr(func) => {
                let _ = writeln!(out, "    .quad {}", program.functions[func.0 as usize].symbol);
            }
            DataItem::DataAddr(other) => {
                let _ = writeln!(out, "    .quad {}", program.data[other.0 as usize].symbol);
            }
        }
    }
}
