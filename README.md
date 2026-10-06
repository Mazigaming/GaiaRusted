**GaiaRusted** 🦀
================

A Rust compiler written from scratch in Rust, with no dependencies. GaiaRusted reads Rust source and produces x86-64 Linux executables, object files, static libraries or assembly, through its own front end, type checker, optimiser and code generator. It does not use LLVM or rustc, and it brings its own standard library, written in Rust.

**v1.1.8 — Reforged: The Typed Pipeline** · on the way to 1.2.0

[README](#gaiarusted-) | [Architecture](./ARCHITECTURE.md) | [Code of Conduct](./CODE_OF_CONDUCT.md) | [Contributing](./CONTRIBUTING.md) | [MIT License](./LICENSE)

* * *

Status
------

GaiaRusted compiles real Rust programs, and every program in its test suites prints exactly what the rustc build of the same program prints. It is not a complete Rust compiler yet: most notably, it does not run the borrow checker, so it accepts some programs rustc rejects (see [Known limitations](#known-limitations)).

| Measure | Result |
| --- | --- |
| Conformance suite: programs whose output must match rustc byte for byte, at `-O0` and `-O2` | ✅ 64 / 64 |
| Programs that must be rejected as rustc rejects them | ✅ 10 / 10 |
| Feature coverage: one program per area of the language and library | ✅ 60 / 60 |
| Benchmarks that build and match rustc's output | ✅ 17 / 17 |
| Run time against `rustc -O` (geometric mean over the benchmarks) | 1.6x |
| Progress through the 1.2.0 plan | about 79% |

* * *

Quick Start
-----------

### Requirements

*   **Linux on x86-64.** Other platforms are not supported.
*   **Rust** (stable) to build the compiler.
*   **GNU binutils and gcc** (`as`, `ar`, `gcc`) to assemble and link what it compiles, against the system C library.

### Build

```bash
git clone https://github.com/Mazigaming/GaiaRusted.git
cd GaiaRusted/gaiarusted
cargo build --release
```

### Compile a program

```bash
# An executable named after the file: ./hello
./target/release/gaiarusted hello.rs

# Choose the output and the format: exe (default), asm, obj or lib
./target/release/gaiarusted hello.rs -o build/hello
./target/release/gaiarusted hello.rs --format asm      # hello.s
./target/release/gaiarusted hello.rs --format lib      # libhello.a

# Optimisation: -O2 is the default, -O0 turns the optimiser off
./target/release/gaiarusted hello.rs -O0

# Link C libraries
./target/release/gaiarusted ffi.rs -L ./native -l mylib

# Show the optimised intermediate representation
./target/release/gaiarusted hello.rs --emit-ir
```

A successful build prints nothing, as rustc does. An error is reported against the source:

```
error: mismatched types: expected `i32`, found `&str`
 --> bad.rs:1:26
  |
1 | fn main() { let x: i32 = "s"; }
  |                          ^^^
error: could not compile `bad` due to 1 previous error
```

### Cargo projects

Give the compiler a folder with a `Cargo.toml` and it builds every binary in it, into `target/gaiarusted/`:

```bash
./target/release/gaiarusted path/to/project
```

It finds binaries where Cargo does: `src/main.rs`, `src/bin/*.rs` and `[[bin]]` entries. Modules declared with `mod` are loaded from their files. Crates with dependencies and library-only crates cannot be built yet.

* * *

An Example
----------

```rust
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Number(i64),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Word(w) => write!(f, "word `{}`", w),
            Token::Number(n) => write!(f, "number {}", n),
        }
    }
}

fn tokenize(text: &str) -> Vec<Token> {
    text.split_whitespace()
        .map(|piece| match piece.parse::<i64>() {
            Ok(n) => Token::Number(n),
            Err(_) => Token::Word(piece.to_lowercase()),
        })
        .collect()
}

fn main() {
    let tokens = tokenize("The quick fox jumps 3 times over the lazy dog 42 times");
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for token in &tokens {
        if let Token::Word(w) = token {
            *counts.entry(w.as_str()).or_insert(0) += 1;
        }
    }
    let mut repeated: Vec<_> = counts.into_iter().filter(|&(_, n)| n > 1).collect();
    repeated.sort();
    let total: i64 = tokens.iter().filter_map(|t| match t { Token::Number(n) => Some(*n), _ => None }).sum();
    println!("{}", tokens[1]);
    println!("{:?}", repeated);
    println!("numbers add up to {}", total);
}
```

```
$ gaiarusted words.rs && ./words
word `quick`
[("the", 2), ("times", 2)]
numbers add up to 45
```

The same output as rustc's build, from a 72 KB executable built in about 0.2 seconds.

* * *

What Works
----------

### Language

*   ✅ All integer types up to `u128`/`i128`, `f32`/`f64`, `bool`, `char`, `str` and `String` (UTF-8 throughout)
*   ✅ Structs, tuple structs, enums with data, tuples, arrays and slices
*   ✅ Generics, monomorphised per instantiation, with `where` clauses and const generics
*   ✅ Traits: default methods, associated types and constants, generic and blanket impls, operator overloading, `impl Trait` in argument and return position
*   ✅ Trait objects (`dyn Trait`) with vtables, including unsized types such as `Rc<dyn Trait>`
*   ✅ Closures (`Fn`, `FnMut`, `FnOnce`, `move`), returned and stored in structs
*   ✅ Pattern matching: ranges, slices, guards, `@` bindings, or-patterns, `if let`, `while let`, `let else`
*   ✅ Error handling with `?`, `Option` and `Result`
*   ✅ Ownership at run time: `Drop` in rustc's order, including partial moves
*   ✅ Modules, `use`, and visibility (private items and fields are enforced)
*   ✅ `macro_rules!` and the built-in macros; `#[derive]` for `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash` and `Default`
*   ✅ `const` and `static` items, including statics initialised at run time
*   ✅ `unsafe`, raw pointers and `extern "C"` functions
*   ✅ Threads, panics and `catch_unwind`

### Standard library

The standard library lives in [`library/`](./library), written in Rust and compiled with every program:

*   ✅ `Vec`, `VecDeque`, `HashMap` and `HashSet` (SipHash, control-byte tables), `BTreeMap`, `BTreeSet`, `BinaryHeap`
*   ✅ `Box`, `Rc`, `Arc`, `Weak`, `Cell`, `RefCell`
*   ✅ Formatting with every format specifier, `Display` and `Debug`
*   ✅ Iterators and their adapters, and the full set of string, slice, number and `char` methods; Unicode tables generated from rustc's own
*   ✅ `std::io` (including `stdin` and `BufRead`), `fs`, `env`, `process`, `time`
*   ✅ `std::thread`, `std::sync` (`Mutex`, atomics, `mpsc`), `std::panic`

### Optimiser

At `-O1` and above the program is optimised on a typed control-flow graph:

*   ✅ Inlining, scalar replacement of aggregates and constant folding
*   ✅ Copy, constant and load propagation, and common-subexpression elimination
*   ✅ Jump threading, control-flow simplification and dead-code elimination
*   ✅ Loop-invariant code motion, loop rotation, and turning tail recursion into loops
*   ✅ Register allocation by graph colouring, using the System V calling convention

* * *

Performance
-----------

Run time of the benchmark programs in [`benchmarks/`](./benchmarks), against rustc's optimised build (lower is better):

| Benchmark | gaiarusted | rustc -O | Ratio |
| --- | ---: | ---: | ---: |
| sieve | 144 ms | 161 ms | 0.89x |
| shapes_dyn | 161 ms | 164 ms | 0.98x |
| brainfuck | 272 ms | 212 ms | 1.28x |
| mandelbrot | 304 ms | 237 ms | 1.28x |
| hashmap_ints | 135 ms | 97 ms | 1.39x |
| collections | 137 ms | 88 ms | 1.55x |
| nbody | 143 ms | 92 ms | 1.56x |
| fib | 159 ms | 101 ms | 1.57x |
| binary_trees | 739 ms | 457 ms | 1.62x |
| iterators | 101 ms | 62 ms | 1.62x |
| sort_strings | 240 ms | 138 ms | 1.74x |
| nqueens | 576 ms | 328 ms | 1.76x |
| bfs_grid | 176 ms | 99 ms | 1.77x |
| lcs_dp | 383 ms | 199 ms | 1.93x |
| tokenizer | 160 ms | 77 ms | 2.07x |
| word_freq | 177 ms | 84 ms | 2.10x |
| matmul | 250 ms | 74 ms | 3.39x |

Every benchmark runs 1.3–14x faster than rustc's unoptimised (`-O0`) build. `matmul` is the outlier because the compiler does not vectorise loops yet. Building a program takes 150–200 ms, including the standard library.

* * *

Known Limitations
-----------------

These are the main things missing before 1.2.0:

*   **No borrow checker or other safety analysis.** The compiler accepts programs that rustc rejects for use after move, conflicting borrows, assigning to an immutable variable, a missing `unsafe`, a non-exhaustive `match`, reading an uninitialised variable, or sending a non-`Send` value to another thread. Valid programs are unaffected.
*   **No `async`/`await`.**
*   **Panics do not unwind.** A panic caught by `catch_unwind` skips the destructors of the frames it leaves, and a `Mutex` held during a panic is not poisoned.
*   **Not yet supported:** `thread_local!`, generic associated types, trait upcasting, and a few library methods (for example `sort_by_cached_key`).
*   **One error at a time.** Compilation stops at the first error.
*   **No loop vectorisation.**
*   **Linux x86-64 only**, and Cargo projects without dependencies.

* * *

How It Works
------------

```
source ─▶ syntax ─▶ sema ─▶ ir ─▶ ir::opt ─▶ x64 ─▶ assembly ─▶ as + gcc
```

| Stage | Where | What it does |
| --- | --- | --- |
| Syntax | `src/syntax/` | Lexer, recursive-descent parser, `macro_rules!`, diagnostics |
| Semantics | `src/sema/` | Name resolution, type inference by unification, trait and method resolution, coercions; produces the typed tree |
| IR | `src/ir/` | A typed control-flow graph per monomorphised function, with memory layouts and drop elaboration |
| Optimiser | `src/ir/opt/` | The passes listed above |
| Backend | `src/x64/` | Instruction selection and register allocation into x86-64 assembly |
| Driver | `src/driver/` | Output formats, linking, Cargo projects |
| Standard library | `library/` | `std`, written in Rust |

[ARCHITECTURE.md](./ARCHITECTURE.md) explains each stage in detail, follows one program through the whole compiler, and ends with a tutorial: [build your own compiler](./ARCHITECTURE.md#build-your-own-compiler-a-tutorial), a small Rust-subset compiler with the same stages, in about a thousand lines.

* * *

Testing
-------

```bash
# The test suites: rustc's output is the expected output.
cargo test --release --test conformance --test driver --test cli --test scratch

# Feature coverage, and benchmarks against rustc (needs python3 and rustc)
python3 coverage/run.py
python3 benchmarks/run.py --runs 3
```

*   **`conformance/`**: 64 programs, each with the output rustc's build prints, checked at `-O0` and `-O2`. `conformance/reject/` holds programs that must fail to compile. Add a program and run the test once with `BLESS=1` to record its expected output.
*   **`coverage/`**: one program per area of the language and library, compared with rustc.
*   **`benchmarks/`**: run time against `rustc -O0` and `rustc -O`.
*   **`tests/`**: the driver and the command line.

* * *

Roadmap
-------

### 1.2.0

*   Safety analysis: mutability, `unsafe`, exhaustiveness, initialisation, moves, and borrow checking with lifetimes
*   `async`/`await`
*   Unwinding panics that run destructors
*   Removing the earlier pipeline (still in `src/`, no longer used by the compiler)
*   A new terminal experience: redesigned diagnostics and output, with themes

### Later

*   Loop vectorisation
*   Reporting several errors per build
*   Dependencies in Cargo projects, and library crates
*   A rebuilt REPL and language server on the new pipeline

* * *

Release History
---------------

### ✅ v1.1.8 (CURRENT) - Reforged: The Typed Pipeline

A new compiler, built beside the old one and now the only one the `gaiarusted` command uses. Correctness is measured against rustc itself: a program counts as working only when its output is byte-identical to rustc's build.

**New compiler pipeline** ✅
*   ✅ Typed front to back: syntax, semantic analysis with type inference, a typed IR, an optimiser and an x86-64 backend
*   ✅ Monomorphised generics, trait resolution with blanket impls, trait objects, closures and `impl Trait`
*   ✅ Memory layouts like rustc's, including niche-optimised enums (`Option<Box<T>>` is 8 bytes) and unsized types
*   ✅ Drop elaboration with drop flags, matching rustc's drop order

**Standard library in Rust** ✅
*   ✅ About 20,000 lines in `library/`: collections, smart pointers, formatting, iterators, strings, Unicode, I/O, threads and synchronisation, panics

**Optimiser and backend** ✅
*   ✅ Inlining, SROA, propagation and CSE, LICM, jump threading, tail-recursion elimination
*   ✅ Graph-colouring register allocation
*   ✅ Benchmarks run at 0.9–3.4x the time of rustc's optimised build

**Command line and driver** ✅
*   ✅ Rewritten `gaiarusted` command: rustc-style output, `-O0`–`-O3`, `--format exe|asm|obj|lib`, `-L`/`-l`, `--emit-ir`
*   ✅ Cargo projects: every binary of a `Cargo.toml` project in one command
*   ✅ Temporary files kept in private, unpredictable folders and always removed

**Testing** ✅
*   ✅ A conformance suite checked against rustc's output at two optimisation levels, plus programs that must be rejected
*   ✅ A coverage suite over the whole language, and benchmarks against rustc

### Earlier releases

Releases up to 1.1.0 were built on the earlier pipeline, which remains in `src/` until 1.2.0 removes it.

| Version | Date | Name |
| --- | --- | --- |
| 1.1.0 | 2026-02-20 | Dynamic Array Indexing & Full Feature Consolidation |
| 1.0.2 | 2026-02-11 | Binary Real Overhaul and Revamp & Performance Validation |
| 1.0.1.5 | 2026-02-08 | Pattern Extraction Alchemy |
| 1.0.1 | 2026-02-06 | Array-of-Structs Return Type Support |
| 1.0.0 | 2026-02-05 | First 1.0 release |
| 0.14.0 | 2026-01-31 | Offsets and runtime |
| 0.13.0 | 2026-01-11 | Advanced Memory, Types & Patterns |
| 0.12.0 | 2026-01-05 | Memory optimisation and profiling |
| 0.11.0 | 2026-01-05 | Optimisations and SIMD |
| 0.9.0 | 2025-12-31 | Struct fields |
| 0.8.0 | 2025-11-24 | Enums, variants, structs and iterator optimisation |
| 0.7.0 | 2025-11-20 | Compiler logic, macros and DWARF |
| 0.6.0 | 2025-11-18 | Async/await, LSP and ecosystem |
| 0.5.0 | 2025-11-17 | Fresh start in a new repository |

* * *

License
-------

MIT License - See [LICENSE](./LICENSE)

* * *

**Made with 🦀 Rust** | Built in memory of Terry Davis and my mental insanity | GaiaRusted v1.1.8
