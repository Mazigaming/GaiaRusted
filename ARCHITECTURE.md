# GaiaRusted Codebase Explanation & Architecture Guide

This is the guided tour of GaiaRusted: what happens to a Rust program from the moment it is read until it runs, where every part of the compiler lives, why it is built the way it is, and how to work on it. It describes **version 1.1.8**. For what the compiler supports and how to use it, see the [README](./README.md).

Every code excerpt, IR dump and assembly listing in this document is taken from the real source or produced by the real compiler.

---

## Contents

1. [What is GaiaRusted?](#what-is-gaiarusted)
2. [How the Compiler Works (Simple Explanation)](#how-the-compiler-works-simple-explanation)
3. [Architecture Flow](#architecture-flow)
4. [Directory Structure](#directory-structure)
5. [Following One Program Through the Compiler](#following-one-program-through-the-compiler)
6. [The Command Line and the Driver](#the-command-line-and-the-driver)
7. [The Pipeline](#the-pipeline)
8. [Stage 1: Syntax](#stage-1-syntax-srcsyntax)
9. [Stage 2: Semantic Analysis](#stage-2-semantic-analysis-srcsema)
10. [Stage 3: The Intermediate Representation](#stage-3-the-intermediate-representation-srcir)
11. [Stage 4: The Optimiser](#stage-4-the-optimiser-srciropt)
12. [Stage 5: The x86-64 Backend](#stage-5-the-x86-64-backend-srcx64)
13. [The Standard Library](#the-standard-library-library)
14. [Testing Strategy](#testing-strategy)
15. [Common Tasks](#common-tasks)
16. [Performance Considerations](#performance-considerations)
17. [Common Gotchas](#common-gotchas)
18. [Not There Yet](#not-there-yet)
19. [The Earlier Pipeline](#the-earlier-pipeline)
20. [Complete File Reference](#complete-file-reference)
21. [Build Your Own Compiler: A Tutorial](#build-your-own-compiler-a-tutorial)
22. [Glossary](#glossary)
23. [Resources](#resources)

---

## What is GaiaRusted?

GaiaRusted is a **Rust compiler written from scratch in Rust**, with no dependencies. It compiles Rust source to x86-64 assembly, object files, static libraries and Linux executables. It does not use LLVM, rustc or any other compiler infrastructure: lexing, parsing, type inference, trait resolution, monomorphisation, memory layout, optimisation, register allocation and instruction selection are all its own. Its standard library is written in Rust too, and compiled together with every program.

**Key facts (1.1.8):**

| | |
|---|---|
| Compiler (the typed pipeline) | about 22,000 lines in `src/syntax`, `src/sema`, `src/ir`, `src/x64`, `src/driver` |
| Standard library | about 20,700 lines of Rust in `library/` |
| Conformance | 64 programs whose output is byte-identical to rustc's build, at `-O0` and `-O2` |
| Feature coverage | 60 / 60 areas of the language and library |
| Benchmarks | 17 / 17 build and match rustc; geometric mean 1.6x the run time of `rustc -O` |
| Dependencies | none (GNU `as`, `ar` and `gcc` are used at run time to assemble and link) |

The rule the whole project is measured by: **a program works when it prints exactly what the rustc build of the same program prints.** Not "something reasonable", not "close": the same bytes.

---

## How the Compiler Works (Simple Explanation)

Think of the compiler as a pipeline that transforms your code step by step. Each step takes something rich and messy and hands the next step something simpler and more explicit:

```
Your Rust Code
    ↓
LEXER (turns text into tokens: `let`, `x`, `=`, `5`, `;`)
    ↓
PARSER (organises tokens into a syntax tree)
    ↓
DERIVE + NAME RESOLUTION (expands #[derive], finds what every name means)
    ↓
TYPE CHECKER (infers every type, picks every trait impl and method)
    ↓
IR BUILDER (turns the typed tree into a control-flow graph, one copy per generic instantiation)
    ↓
OPTIMISER (inlines, folds, propagates, hoists, removes dead code)
    ↓
BACKEND (allocates registers, selects x86-64 instructions)
    ↓
ASSEMBLER + LINKER (as + gcc, against the C library)
    ↓
Your Binary
```

Each stage catches the errors it can understand: the parser catches syntax errors, the type checker catches type errors and privacy violations. Once a program reaches the IR builder it is known to be well typed, and nothing after that point reports errors to the user.

---

## Architecture Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ SOURCE: your .rs files            +  library/*.rs (std, in Rust) │
└─────────────────────────────────────────────────────────────────┘
                                ↓
┌─────────────────────────────────────────────────────────────────┐
│ SYNTAX (src/syntax)                                              │
│ Lexer: text → tokens with spans                                  │
│ Parser: tokens → AST; macro_rules! expansion                     │
│ Error: "expected `;`, found `<`"                                 │
└─────────────────────────────────────────────────────────────────┘
                                ↓
┌─────────────────────────────────────────────────────────────────┐
│ SEMANTICS (src/sema)                                             │
│ derive → defs (names, modules, imports, visibility)              │
│ check: type inference, traits, methods, coercions, closures      │
│ Output: the typed tree (THIR), per generic instantiation         │
│ Error: "mismatched types: expected `i32`, found `&str`"          │
└─────────────────────────────────────────────────────────────────┘
                                ↓
┌─────────────────────────────────────────────────────────────────┐
│ IR (src/ir)                                                      │
│ build: THIR → control-flow graph over typed locals               │
│ layout: sizes, alignments, niches, unsized tails                 │
│ drop: drop flags, drop glue, rustc's drop order                  │
└─────────────────────────────────────────────────────────────────┘
                                ↓
┌─────────────────────────────────────────────────────────────────┐
│ OPTIMISER (src/ir/opt) — skipped at -O0                          │
│ inline (callees first) → tidy rounds:                            │
│   sroa · propagate · thread_jumps · simplify_cfg · dce · licm    │
│ → rotate · addresses · compact · tail_calls · prune              │
└─────────────────────────────────────────────────────────────────┘
                                ↓
┌─────────────────────────────────────────────────────────────────┐
│ BACKEND (src/x64)                                                │
│ abi: System V classification · regalloc: graph colouring         │
│ function: instruction selection → Intel-syntax assembly          │
└─────────────────────────────────────────────────────────────────┘
                                ↓
┌─────────────────────────────────────────────────────────────────┐
│ DRIVER (src/driver)                                              │
│ asm: write .s · obj: as · exe: as + gcc -no-pie · lib: as + ar   │
└─────────────────────────────────────────────────────────────────┘
                                ↓
┌─────────────────────────────────────────────────────────────────┐
│                    EXECUTABLE / OBJECT / LIBRARY                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Directory Structure

```
gaiarusted/
├── src/
│   ├── bin/
│   │   ├── gaiarusted.rs       # The command line (~180 LOC)
│   │   └── repl.rs             # Older REPL, to be rebuilt
│   ├── lib.rs                  # Library root: module declarations
│   ├── pipeline.rs             # The stages in order; embeds library/ (~180 LOC)
│   │
│   ├── driver/                 # Output formats, linking, Cargo projects (~430 LOC)
│   │   ├── mod.rs              # Format, Options, Failure, build, emit_ir
│   │   ├── link.rs             # Scratch folders; as, gcc, ar
│   │   └── project.rs          # Cargo.toml, binary discovery
│   │
│   ├── syntax/                 # Stage 1: source → AST (~4,500 LOC)
│   │   ├── lexer.rs, token.rs  # Tokenization
│   │   ├── parser/             # Recursive descent: items, exprs, patterns, types, macros
│   │   ├── ast.rs, build.rs    # The syntax tree, and building it by hand
│   │   ├── span.rs             # Source locations
│   │   ├── diagnostic.rs       # Errors, rendered like rustc's
│   │   └── visit.rs            # AST walking
│   │
│   ├── sema/                   # Stage 2: AST → typed tree (~8,800 LOC)
│   │   ├── derive.rs           # #[derive] expansion
│   │   ├── defs.rs             # Declarations, modules, imports, visibility
│   │   ├── context.rs          # Program-wide queries: signatures, fields, impls
│   │   ├── ty.rs               # Semantic types
│   │   ├── infer.rs            # Unification with an undo log
│   │   ├── check/              # Body type checking, split by construct
│   │   ├── thir.rs             # The typed tree
│   │   ├── const_eval.rs       # Array lengths, const items
│   │   └── impl_trait.rs       # impl Trait in argument position
│   │
│   ├── ir/                     # Stages 3–4: typed tree → optimised IR (~5,900 LOC)
│   │   ├── mod.rs              # Program, Function, Block, Statement, Terminator, Place
│   │   ├── build/              # Lowering: expressions, calls, patterns, drops
│   │   ├── layout.rs           # Memory layout, niches, unsized tails
│   │   ├── analysis.rs         # Predecessors, liveness, dominators, loops
│   │   ├── visit.rs            # Place walking
│   │   ├── display.rs          # --emit-ir
│   │   └── opt/                # The optimisation passes
│   │
│   ├── x64/                    # Stage 5: IR → x86-64 assembly (~2,400 LOC)
│   │   ├── abi.rs              # System V calling convention
│   │   ├── regalloc.rs         # Graph-colouring register allocation
│   │   ├── function.rs         # Instruction selection
│   │   ├── reg.rs              # Registers and operands
│   │   └── runtime.rs          # Atomic operations
│   │
│   └── (earlier pipeline: lexer/, parser/, lowering/, typechecker/, mir/, codegen/, …
│        still in the tree, not used by the gaiarusted command, removed in 1.2.0)
│
├── library/                    # std, written in Rust (~20,700 LOC, 52 files)
├── conformance/                # 64 programs + expected output; reject/ (10 programs)
├── coverage/                   # 60 programs, one per language area; run.py
├── benchmarks/                 # 17 programs; run.py compares with rustc
├── tests/                      # conformance.rs, driver.rs, scratch.rs, cli.rs
├── tools/gen_unicode.py        # Generates library/unicode.rs
├── examples/                   # Sample programs
├── Cargo.toml                  # No dependencies
├── README.md                   # Overview and usage
├── ARCHITECTURE.md             # This file
└── CONTRIBUTING.md             # Contribution guidelines
```

---

## Following One Program Through the Compiler

The fastest way to understand the compiler is to watch one program go through it. Here is the program:

```rust
struct Point {
    x: i64,
    y: i64,
}

fn norm2(p: &Point) -> i64 {
    p.x * p.x + p.y * p.y
}

fn sum_to(n: i64) -> i64 {
    let mut total = 0;
    let mut i = 1;
    while i <= n {
        total += i;
        i += 1;
    }
    total
}

fn main() {
    let p = Point { x: 3, y: 4 };
    println!("{} {}", norm2(&p), sum_to(10));
}
```

It prints `25 55`.

### 1. Tokens

The lexer turns `p.x * p.x` into five tokens, each with a span pointing back into the file:

```
Ident("p")  Dot  Ident("x")  Star  Ident("p") …
```

`TokenKind` (in `src/syntax/token.rs`) has one variant per kind of token; literals carry their value and suffix:

```rust
pub enum TokenKind {
    Ident(String),
    Keyword(Keyword),
    /// `'a`, without the quote.
    Lifetime(String),

    /// An integer literal and its optional type suffix (`42`, `0xff`, `7u8`).
    Int(u128, Option<String>),
    /// A float literal and its optional type suffix (`1.5`, `2e10`, `3f32`).
    Float(f64, Option<String>),
    Str(String),
    Char(char),
    // … punctuation: LParen, Comma, PathSep, Arrow, FatArrow, Plus, …
}
```

### 2. The syntax tree

The parser builds an `Item` for each declaration. `norm2`'s body becomes nested `ExprKind` nodes:

```
Binary(Add,
    Binary(Mul, Field(Path(p), x), Field(Path(p), x)),
    Binary(Mul, Field(Path(p), y), Field(Path(p), y)))
```

At this point `p` is only a name and `x` only an identifier: nobody knows yet that `p` is a reference or that `x` is the first field of `Point`. `println!` is still an unexpanded macro call.

### 3. The typed tree

Type checking turns the same body into THIR (`src/sema/thir.rs`). Every node now has a type, `p` is a local, `.x` is field 0 through a dereference, and `*` is a primitive multiplication on `i64`:

```
Binary(Add,                                   : i64
    Binary(Mul,                               : i64
        Field(Deref(Local(p)), 0),            : i64
        Field(Deref(Local(p)), 0)),           : i64
    Binary(Mul, …Field(…, 1)…))               : i64
```

`println!("{} {}", …)` has been expanded into calls on a `Formatter`: `fmt::i64::fmt` for each argument and `write_str` for the literal pieces.

### 4. The IR, unoptimised (`--emit-ir -O0`)

The IR builder produces one control-flow graph per function. Here is the real output for `norm2` and `sum_to`:

```
fn norm2(_1: &Point) -> i64 {
    let _2: i64;
    let _3: i64;
  bb0:
    _2 = (*_1).0 * (*_1).0
    _3 = (*_1).1 * (*_1).1
    _0 = _2 + _3
    return
  bb1:
    unreachable
}

fn sum_to(_1: i64) -> i64 {
    let _2: i64;  // total
    let _3: i64;  // i
    let _4: ();
    let _5: ();
    let _6: bool;
    let _7: ();
    let _8: ();
  bb0:
    _2 = 0
    _3 = 1
    goto bb1
  bb1:
    _6 = _3 <= _1
    if _6 goto bb3 else bb4
  bb2:
    _0 = _2
    return
  bb3:
    _2 = _2 + _3
    _3 = _3 + 1
    goto bb5
  bb4:
    goto bb2
  bb5:
    goto bb1
  bb6:
    goto bb5
  bb7:
    unreachable
}
```

Things to notice:

- `_0` is always the return value and `_1…` the arguments; the rest are locals, named after the source variable when there is one.
- `a + b * c` has become one operation per statement. `(*_1).0` is a *place*: field 0 of what local `_1` points to.
- The `while` loop is blocks and jumps. Lowering is deliberately naive and leaves empty blocks (`bb4`, `bb5`, `bb6`) and unreachable ones behind; cleaning up is the optimiser's job, which keeps the builder simple.

`main` shows a **drop flag** at work. `_10` records whether the `Formatter` in `_9` currently holds a value that must be dropped:

```
  bb0:
    _10 = 0
    _1.0 = 3
    _1.1 = 4
    _5 = &_1
    _4 = norm2(_5)
    …
    _9 = fmt::Formatter::new()
    _10 = 1
    …
    _2 = io::_print(_16)
    if _10 goto bb3 else bb4
  …
  bb3:
    _17 = &_9
    _18 = drop_in_place<Formatter>(_17)
    goto bb4
```

### 5. The IR, optimised (`--emit-ir`)

At `-O2` the picture changes completely. `norm2` and `sum_to` have been inlined into `main` and no longer exist as functions. Because the point is known to be `(3, 4)`, `norm2(&p)` has been computed at compile time: the formatter is called with the constant `25`. The loop of `sum_to` remains, rotated so that its test sits at the bottom:

```
fn main() -> () {
    …
    let _8: i64;  // total
    let _9: i64;  // i
    let _10: bool;
    …
  bb0:
    _8 = 0
    _9 = 1
    goto bb1
  bb1:
    _8 = _8 + _9
    _9 = _9 + 1
    _10 = _9 <= 10
    if _10 goto bb1 else bb2
  bb2:
    …
    _3 = fmt::fmt_unsigned(_2, 25, 1, 10, 0, str(_G.str.3, 0))
    …
```

The `Formatter` has been split into its fields (`_1.0.0 = …`, `_1.1 = 32`, …), and the call to `fmt::i64::fmt` has been inlined down to `fmt_unsigned`.

### 6. Assembly, unoptimised (`--format asm -O0`)

At `-O0` every local lives in its stack slot, and each IR statement is translated on its own. This is `sum_to`:

```asm
_G.sum_to:
    push rbp
    mov rbp, rsp
    sub rsp, 48
    mov qword ptr [rbp - 16], rdi
.Lf2_b0:
    mov qword ptr [rbp - 24], 0
    mov qword ptr [rbp - 32], 1
.Lf2_b1:
    mov rax, qword ptr [rbp - 32]
    mov rcx, qword ptr [rbp - 16]
    cmp rax, rcx
    jle .Lf2_b3
    jmp .Lf2_b4
.Lf2_b2:
    mov rax, qword ptr [rbp - 24]
    mov qword ptr [rbp - 8], rax
    mov rax, qword ptr [rbp - 8]
    leave
    ret
.Lf2_b3:
    mov rax, qword ptr [rbp - 24]
    mov rcx, qword ptr [rbp - 32]
    add rax, rcx
    mov qword ptr [rbp - 24], rax
    mov rax, qword ptr [rbp - 32]
    add rax, 1
    mov qword ptr [rbp - 32], rax
    jmp .Lf2_b5
.Lf2_b4:
    jmp .Lf2_b2
.Lf2_b5:
    jmp .Lf2_b1
.Lf2_b6:
    jmp .Lf2_b5
.Lf2_b7:
    ud2
```

Each IR block becomes a label `.Lf<function>_b<block>`. The argument arrives in `rdi` and is stored to its slot; the result leaves in `rax`. `unreachable` becomes `ud2`, which stops the program if it is ever reached.

### 7. Assembly, optimised (`--format asm`)

At `-O2`, with registers allocated, the whole of `sum_to(10)` inside `main` is four instructions in a loop:

```asm
.Lf0_b0:
    xor ebx, ebx
    mov rsi, 1
.Lf0_b1:
    add rbx, rsi
    add rsi, 1
    cmp rsi, 10
    jle .Lf0_b1
```

`total` lives in `rbx` and `i` in `rsi` for the whole function.

### 8. The program's entry point

The C library calls `main`. GaiaRusted's `main` first runs the initialisers of the statics that need code to compute (here the panic hook and the `catch_unwind` registry), then the user's `main`, which is the symbol `_G.main`, then returns 0:

```asm
main:
    push rbp
    mov rbp, rsp
    call _G.HOOK.init
    call _G.CATCHERS.init
    call _G.main
    xor eax, eax
    pop rbp
    ret
```

### 9. Assemble and link

The driver writes the assembly to a private scratch folder, runs `as` on it, links the object with `gcc -no-pie … -lc -lm`, and removes the scratch folder. The result is a 70 KB executable that prints `25 55`.

---

## The Command Line and the Driver

### `src/bin/gaiarusted.rs`

The binary is deliberately thin. It parses arguments into one `Command`:

```rust
enum Command {
    Build(Options),
    Project { dir: PathBuf, opt_level: OptLevel },
    EmitIr { sources: Vec<PathBuf>, opt_level: OptLevel },
    Help,
    Version,
}
```

Then it runs the command and reports the result:

| Situation | Output | Exit status |
|---|---|---|
| Success | nothing | 0 |
| Error in the program | the diagnostic, then ``error: could not compile `name` due to 1 previous error`` | 1 |
| A tool or file problem | `error: …` with the tool's message | 1 |
| A bad command line | `error: unknown option …` and a pointer to `--help` | 2 |

Without `-o`, the output is named after the first source file, as rustc does: `hello.rs` gives `hello`, `hello.s`, `hello.o` or `libhello.a`. A folder argument is a Cargo project if it has a `Cargo.toml`, and otherwise the program in its `main.rs` (or its only `.rs` file).

### `src/driver/mod.rs`

The driver is the library API for building:

```rust
pub enum Format { Assembly, Object, Executable, Library }

pub struct Options {
    /// The crate root first; each other file becomes a module named after it.
    pub sources: Vec<PathBuf>,
    pub output: PathBuf,
    pub format: Format,
    pub opt_level: OptLevel,
    /// Folders to search for the C libraries in `link_libraries`.
    pub link_paths: Vec<PathBuf>,
    pub link_libraries: Vec<String>,
}

pub enum Failure {
    /// The program has an error.
    Program(pipeline::Failure),
    /// A Cargo project cannot be built as it is described.
    Project(String),
    /// A file could not be read or written, or a tool (`as`, `gcc`, `ar`)
    /// is missing or failed.
    Tool(String),
}
```

`build` runs the pipeline, then finishes according to the format:

```rust
pub fn build(options: &Options) -> Result<Vec<PathBuf>, Failure> {
    let Some((root, modules)) = options.sources.split_first() else {
        return Err(Failure::Tool("no source files given".to_string()));
    };
    let assembly = pipeline::compile(root, modules, Emit::Assembly, options.opt_level).map_err(Failure::Program)?;
    let output = &options.output;
    match options.format {
        Format::Assembly => write(output, &assembly)?,
        Format::Object => link::assemble(&assembly, output)?,
        Format::Executable => {
            let scratch = link::Scratch::new()?;
            let object = scratch.path("program.o");
            link::assemble(&assembly, &object)?;
            link::link_executable(&object, output, &options.link_paths, &options.link_libraries)?;
        }
        Format::Library => {
            let scratch = link::Scratch::new()?;
            let object = scratch.path("program.o");
            link::assemble(&assembly, &object)?;
            link::archive(&object, output)?;
        }
    }
    Ok(vec![output.clone()])
}
```

`Failure::Program` keeps the error as a structured `Diagnostic` with its source map, not as printed text, so that whoever displays it decides how it looks.

### `src/driver/link.rs`

- **`Scratch`** is a folder for intermediate files. It is created in the temporary directory with a random name and owner-only permissions (0700), never reusing a folder that already exists, so another user of a shared `/tmp` cannot plant files in it. It is removed, with everything in it, when dropped.
- **`assemble`** runs `as`.
- **`link_executable`** runs `gcc -no-pie <object> -L… -l… -lc -lm -o <output>`. The C compiler driver is used for linking because it knows where the C runtime start files and `libgcc` are.
- **`archive`** runs `ar rcs`, after removing an existing archive: `ar` would otherwise add to it.

A failing tool's standard error becomes the `Failure::Tool` message, so a missing `-l` library reports the linker's own explanation.

### `src/driver/project.rs`

Reads just enough of `Cargo.toml` to build: `[package] name`, `[[bin]]` entries, and the names in `[dependencies]` and `[dependencies.NAME]`. Comments, inline tables and other sections are passed over. Binaries are found in Cargo's order:

1. `src/main.rs`, named after the package (unless a `[[bin]]` entry names that file)
2. the `[[bin]]` entries (a missing `path` means `src/bin/NAME.rs`)
3. the remaining `src/bin/*.rs`, in name order

Each binary is built into `target/gaiarusted/<name>`. A project with dependencies is refused with "dependencies are not supported yet", and one with only `src/lib.rs` is reported as a library crate that cannot be built yet.

---

## The Pipeline

`src/pipeline.rs` is where the stages meet:

```rust
pub fn compile(root_file: &Path, extra_files: &[PathBuf], emit: Emit, level: OptLevel) -> Result<String, Failure>
```

It:

1. parses every file of `library/` (embedded in the compiler as the `LIBRARY` table, one entry per module path such as `"collections::hash_map"`) and wraps them in a `std` module tree;
2. parses the program's root file, which loads `mod name;` files from the root's folder, and turns each extra file into a module named after it;
3. runs semantic analysis, IR building, optimisation at the requested level, and code generation;
4. returns assembly text, or an IR dump for `Emit::Ir`.

On failure it returns a `Failure`, holding the `Diagnostic` and the `SourceMap` it points into:

```rust
pub struct Failure {
    pub diagnostic: Diagnostic,
    pub sources: SourceMap,
}

impl Failure {
    /// The error with its source context, as the compiler prints it.
    pub fn render(&self) -> String {
        self.diagnostic.render(&self.sources)
    }
}
```

**Why embed the standard library as source?** Because generics need it. A `Vec<T>` method cannot be compiled before `T` is known, so a precompiled `std` would need a way to store generic code; compiling `std` together with the program sidesteps that entirely. Only what the program reaches from `main` is type-checked and generated, so an unused part of the library costs parsing time and nothing else.

---

## Stage 1: Syntax (`src/syntax/`)

**Purpose:** turn source text into a syntax tree. Nothing here knows what a name means or what type anything has.

```
source text ──lexer──▶ tokens ──parser──▶ AST
```

### The lexer (`lexer.rs`, `token.rs`)

- Keywords, identifiers, lifetimes and every kind of punctuation, including the multi-character ones (`::`, `->`, `=>`, `..=`, `<<=`).
- Every literal form: decimal, hex, octal and binary integers with `_` separators and suffixes (`7u8`, `0xffu32`); floats with exponents; strings with escapes (`\n`, `\u{1F980}`), raw strings (`r#"…"#`), byte strings and byte literals; chars.
- Line and block comments, nested block comments, and doc comments.
- Every token carries a `Span` (file and byte range) so errors can point at it.

### The parser (`parser/`)

A hand-written recursive-descent parser, split by construct:

| File | Parses |
|---|---|
| `parser/mod.rs` | The parser state, token lookahead, error helpers |
| `parser/item.rs` | Functions, structs, enums, traits, impls, consts, statics, type aliases, modules, `use` trees, `extern` blocks |
| `parser/expr.rs` | Expressions by precedence climbing, statements and blocks |
| `parser/pat.rs` | Patterns: literals, ranges, tuples, structs, slices, `@` bindings, or-patterns |
| `parser/ty.rs` | Types, paths with generic arguments, generic parameters, bounds, `where` clauses |
| `parser/macros.rs` | `macro_rules!` |

Two classic Rust parsing subtleties are handled the way rustc handles them:

- **Struct literals in conditions.** In `if x == S { … }` the `{` starts the block, not a struct literal. The parser carries a restriction flag while parsing conditions.
- **`>>` in generics.** `Vec<Vec<i32>>` closes two generic lists with one token; the parser splits it.

The syntax tree (`ast.rs`) mirrors the source:

```rust
pub struct Item {
    pub kind: ItemKind,
    pub attrs: Vec<Attribute>,
    pub is_pub: bool,
    pub span: Span,
}

pub enum ItemKind {
    Fn(Function),
    Struct(StructDef),
    Enum(EnumDef),
    Trait(TraitDef),
    Impl(ImplBlock),
    Const(ConstDef),
    Static(ConstDef),
    TypeAlias(TypeAlias),
    Mod(Module),
    Use(UseTree),
    /// `extern "C" { fn ...; }`
    ExternBlock(Vec<Function>),
}
```

### Macros by example (`parser/macros.rs`)

`macro_rules!` definitions are recorded when parsed, and invocations are expanded right away, during parsing. Expansion matches the invocation's tokens against each rule in order:

- **fragment specifiers**: `$e:expr`, `$t:ty`, `$i:ident`, `$p:pat`, `$b:block`, `$l:literal`, `$tt:tt` and the rest;
- **repetitions**: `$(…),*`, `$(…);+`, `$(…)?`, nested;

then substitutes the captured fragments into the rule's body and parses the result. The built-in macros (`println!`, `vec!`, `format!` and friends) are not `macro_rules!`: they are expanded during type checking, where the types of their arguments are known (see [`check/macros.rs`](#type-checking-check)).

### Diagnostics (`diagnostic.rs`, `span.rs`)

```rust
pub struct Diagnostic {
    pub message: String,
    pub span: Option<Span>,
    /// Extra context lines shown under the snippet (`= note: ...`).
    pub notes: Vec<String>,
}
```

`Diagnostic::render` prints an error in rustc's layout:

```
error: mismatched types: expected `i32`, found `&str`
 --> bad.rs:1:26
  |
1 | fn main() { let x: i32 = "s"; }
  |                          ^^^
```

### `build.rs` and `visit.rs`

`build.rs` constructs AST by hand, for code the compiler writes itself, such as derived impls. `visit.rs` walks the tree.

---

## Stage 2: Semantic Analysis (`src/sema/`)

**Purpose:** work out what everything means. Which item does each name refer to? What type does each expression have? Which trait impl and which method does each call use? The answer is the typed tree.

```
AST ──derive──▶ AST ──defs──▶ declarations ──check──▶ typed tree (THIR)
```

### Derive (`derive.rs`)

`#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]` is expanded into ordinary `impl` blocks, written as source text and parsed, before anything else runs. From then on a derived impl is indistinguishable from a handwritten one, which is why it needs no other support. `#[default]` on an enum variant is honoured.

### Declarations (`defs.rs`)

One walk over the whole AST records every function, struct, enum, trait, impl, constant and static under an id, builds the module tree, and resolves `use` declarations, including globs, renames and re-exports. After that, "what does this name mean here?" is a table lookup (`Defs::resolve_path`).

**Visibility is enforced here.** A private function, constant, module, method, associated function, field or tuple-struct constructor cannot be reached from outside the module that declares it, with the same rules and messages as rustc (`field `h` of struct `Rect` is private`).

### Types (`ty.rs`)

```rust
pub enum Ty {
    Int(IntTy),
    Float(FloatTy),
    Bool,
    Char,
    /// The unsized string slice; only ever seen behind a pointer (`&str`).
    Str,
    /// `!` — the type of expressions that never produce a value.
    Never,
    /// `(A, B)`; the unit type `()` is the empty tuple.
    Tuple(Vec<Ty>),
    /// `[T; N]`: the length is a [`Ty::Const`], or a variable until known.
    Array(Box<Ty>, Box<Ty>),
    /// The unsized slice `[T]`; only ever seen behind a pointer.
    Slice(Box<Ty>),
    /// `&T` / `&mut T`
    Ref(Box<Ty>, Mutability),
    /// `*const T` / `*mut T`
    Ptr(Box<Ty>, Mutability),
    /// A struct or enum with its type arguments.
    Adt(AdtId, Vec<Ty>),
    /// `fn(A, B) -> R` — a function pointer.
    FnPtr(Vec<Ty>, Box<Ty>),
    /// The zero-sized type of one specific function. Calling through it is
    /// a direct call; it coerces to [`Ty::FnPtr`] when stored as a pointer.
    FnItem(FnId, Vec<Ty>),
    /// The anonymous type of a closure: a struct holding its captures.
    Closure(ClosureId),
    /// `dyn Trait` with the trait's type arguments; only seen behind a pointer.
    Dyn(TraitId, Vec<Ty>),
    /// A constant in a type: the length of an array, or the value of a
    /// const generic parameter such as the `N` of `Buffer<N>`.
    Const(i128),
    /// Not known yet.
    Infer(InferVar),
}
```

Notice what is missing: there is no "type parameter" variant. Generic code is never type-checked in the abstract (see below), so a `T` is always replaced by a concrete type or an inference variable before anyone looks at it.

### The context (`context.rs`)

`Context` answers every question about declarations that type checking and code generation share:

- the signature of a function, *for given type arguments*;
- the type of a field, the variants of an enum;
- which impl of a trait applies to a type, and what an associated type is in it;
- whether a type is `Copy`, needs dropping, or implements a trait.

**Impl selection** is the hot spot of the whole compiler: every method call, every operator on a user type, every `for` loop (through `IntoIterator`) and every `?` asks it. It is made fast by:

- an index of impls by trait, and of impls by the associated type they define;
- a pre-filter on the *shape* of the self type (`Vec<_>`, `&_`, `[_; _]` …), so most impls are rejected without unification;
- caches for impl matches, layouts, and the concrete field types of variants.

**Traits the compiler needs to know about** (`Copy`, `Clone`, `Drop`, `Deref`, `DerefMut`, `Fn`, `FnMut`, `FnOnce`, `Iterator`, `IntoIterator`, `CoerceUnsized`, the operator traits …) are found by their path in the library, such as `std::ops::Deref`. The library defines them as ordinary Rust traits; there is no list of special attributes to keep in sync.

### Type inference (`infer.rs`)

Inference is unification over the `InferTable`:

```rust
pub struct InferTable {
    vars: Vec<VarState>,
    /// Every binding made, with the state the variable had before, so that
    /// a [`Snapshot`] can be returned to by undoing the bindings since.
    undo_log: Vec<(InferVar, VarState)>,
}
```

There are three kinds of variable: *general*, *integer literal* and *float literal*. An integer literal's variable can only become an integer type, and defaults to `i32` if nothing decides it; a float literal defaults to `f64`. The core of unification:

```rust
pub fn unify(&mut self, a: &Ty, b: &Ty) -> Result<(), Mismatch> {
    let a = self.shallow(a);
    let b = self.shallow(b);
    match (&a, &b) {
        (Ty::Infer(x), Ty::Infer(y)) if x == y => Ok(()),
        (Ty::Infer(x), Ty::Infer(y)) => {
            // Keep the more specific kind alive: bind the general one.
            let (x_kind, y_kind) = (self.unbound_kind(*x), self.unbound_kind(*y));
            match (x_kind, y_kind) {
                (Some(VarKind::General), _) => self.set(*x, VarState::Bound(b)),
                (_, Some(VarKind::General)) => self.set(*y, VarState::Bound(a)),
                _ if x_kind == y_kind => self.set(*x, VarState::Bound(b)),
                // An integer literal is never a float literal.
                _ => return Err(Mismatch),
            }
            Ok(())
        }
        (Ty::Infer(var), other) | (other, Ty::Infer(var)) => self.bind(*var, other.clone()),

        (Ty::Tuple(xs), Ty::Tuple(ys)) => self.unify_all(xs, ys),
        (Ty::Adt(x, xs), Ty::Adt(y, ys)) if x == y => self.unify_all(xs, ys),
        // … arrays, slices, references, pointers, function pointers …
        _ if a == b => Ok(()),
        _ => Err(Mismatch),
    }
}
```

The **undo log** makes trying things cheap: impl selection takes a snapshot, attempts to unify an impl's self type with the receiver, and rolls back if the impl does not apply, without copying the table.

### Type checking (`check/`)

`check_fn` takes one *function instance* — a function plus concrete types for its generic parameters — and produces its typed tree. The work is split by construct:

| File | Checks |
|---|---|
| `check/mod.rs` | The checker state, locals, scopes, expected-type propagation, finishing inference |
| `check/expr.rs` | Literals, operators (built-in and overloaded), fields, indexing, struct literals |
| `check/path.rs` | Names in expression position; calls of functions, associated functions and constructors |
| `check/method.rs` | Method calls: auto-deref and auto-borrow of the receiver, inherent before trait methods |
| `check/control.rs` | `if`, `match`, loops and labels, closures, `return`, `break` with values, `?` |
| `check/pat.rs` | Patterns and the types they bind, including binding modes (`ref`, default binding through references) |
| `check/coerce.rs` | The implicit conversions allowed at assignment points |
| `check/macros.rs` | The built-in macros |

**Method lookup** follows rustc's algorithm: for each step of auto-dereferencing the receiver type (`Box<Vec<T>>` → `Vec<T>` → `[T]`), try the type by value, then `&`, then `&mut`; inherent methods before trait methods. When the receiver's type is still an inference variable, the call is deferred until inference has learned more, rather than guessing.

**Coercions** (`coerce.rs`) are the conversions that happen without a cast:

- `&mut T` to `&T`; `&String` to `&str` and other deref coercions;
- unsizing: `&[T; N]` to `&[T]`, `&T` to `&dyn Trait`, and through the `CoerceUnsized` trait, `Box<T>`, `Rc<T>`, `Arc<T>` and `Weak<T>` to their `dyn` or slice forms;
- a capture-free closure or a function item to a function pointer;
- `!` to anything.

**Closures** get a fresh `Ty::Closure` each. Their captures are found by walking the body, and each is captured by reference, by mutable reference or by value according to how the body uses it (or by value for `move` closures). Which of `Fn`, `FnMut` and `FnOnce` a closure implements follows from its captures. When a closure is passed to a function bounded by `F: Fn(A) -> B`, or to a trait with a blanket impl over such functions, the bound supplies the parameter types before the body is checked, as in rustc.

**The built-in macros** (`macros.rs`) are expanded here because they need types. `println!("{} {:?}", a, b)` checks that `a` implements `Display` and `b` implements `Debug` and becomes calls on a `Formatter`. The family includes `print!`, `println!`, `eprint!`, `eprintln!`, `format!`, `write!`, `writeln!`, `panic!`, `assert!`, `assert_eq!`, `assert_ne!`, `debug_assert*!`, `vec!`, `matches!`, `dbg!`, `todo!`, `unimplemented!`, `unreachable!`, `stringify!`, `concat!`, `file!`, `line!`, `column!` and `module_path!`. Format strings support the full specification syntax: width, precision, fill and alignment, sign, `#`, `0`, positional and named arguments, and `{name}` captures.

### Generics are checked per instantiation

```
main ──calls──▶ largest::<i64>  ──checked as──▶ fn largest(xs: &[i64]) -> i64
     ──calls──▶ largest::<f64>  ──checked as──▶ fn largest(xs: &[f64]) -> f64
```

A generic function is type-checked once for each set of concrete type arguments it is used with, on demand, starting from `main`. This keeps the checker simple — inside a body every type is concrete or an inference variable — and is the same order of work monomorphisation needs anyway. The trade-off is in [Common Gotchas](#common-gotchas): an error inside a generic function that is never called is never reported.

### The typed tree (`thir.rs`)

Compared to the AST, everything implicit has been made explicit:

```rust
pub enum ExprKind {
    Int(u128),
    Float(f64),
    Bool(bool),
    Char(char),
    /// A string literal; its type is `&str`.
    Str(String),
    /// A value of a zero-sized type: `()`, a function item, a unit struct.
    ZeroSized,

    Local(LocalId),
    Static(ConstId),

    Unary(UnaryOp, Box<Expr>),
    /// A built-in operator on primitive operands. `&&` and `||` short-circuit.
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// A primitive conversion from the operand's type to this expression's type.
    Cast(Box<Expr>),

    AddrOf(Mutability, Box<Expr>),
    Deref(Box<Expr>),
    /// A field of a struct or tuple, by index.
    Field(Box<Expr>, usize),
    /// An element of an array or slice; bounds-checked.
    Index(Box<Expr>, Box<Expr>),

    Call(Callee, Vec<Expr>),
    /// A struct literal or enum variant constructor.
    Adt { variant: u32, fields: Vec<(usize, Expr)>, base: Option<Box<Expr>> },
    Closure(ClosureId),

    /// Pointer-to-sized into pointer-to-unsized: `&[T; N]` to `&[T]`,
    /// `&T` to `&dyn Trait`, `Box<T>` to `Box<dyn Trait>`.
    Unsize(Box<Expr>),
    /// A function item or capture-free closure used as a function pointer.
    ReifyFnPointer(Box<Expr>),

    Block(Block),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    /// `let PATTERN = value` as a condition: tests the pattern and binds its
    /// variables when it matches.
    Let(Box<Pat>, Box<Expr>),
    Match(Box<Expr>, Vec<Arm>),
    Loop(LoopId, Block),
    Break(LoopId, Option<Box<Expr>>),
    Continue(LoopId),
    Return(Option<Box<Expr>>),

    Assign(Box<Expr>, Box<Expr>),
    /// `place op= value` on primitive operands.
    AssignOp(BinOp, Box<Expr>, Box<Expr>),
    // … Tuple, Array, Repeat, ByteStr …
}
```

- Every expression and pattern carries its `Ty`.
- Method calls and overloaded operators are plain `Call`s, with the receiver's auto-borrow written out as `AddrOf`.
- `for` and `while` are `Loop`s with a `match` or `if` inside; `?` is a `match` with an early `Return`.
- Coercions are explicit nodes (`Unsize`, `ReifyFnPointer`).

### Supporting files

- **`const_eval.rs`** evaluates integer expressions at compile time, for array lengths, const generics and `const` items.
- **`impl_trait.rs`** turns `impl Trait` in argument position into an anonymous generic parameter.

---

## Stage 3: The Intermediate Representation (`src/ir/`)

**Purpose:** a form of the program where nothing is generic, implicit or nested, which can be analysed, optimised and turned into machine code.

### The data structures (`ir/mod.rs`)

```rust
pub struct Program {
    pub functions: Vec<Function>,
    pub data: Vec<Data>,
    /// The user's `main`.
    pub entry: FuncId,
    /// Functions that compute the statics that are not plain data; they
    /// run before `main`, in this order.
    pub initializers: Vec<FuncId>,
}

pub struct Function {
    /// The linker symbol.
    pub symbol: String,
    /// A readable name for dumps and diagnostics: `Vec<i64>::push`.
    pub name: String,
    pub locals: Vec<LocalDecl>,
    pub arg_count: usize,
    pub blocks: Vec<Block>,
}

pub struct Block {
    pub statements: Vec<Statement>,
    /// `None` only while the block is under construction.
    pub terminator: Option<Terminator>,
}
```

A **place** names a local or a part of one:

```rust
pub struct Place {
    pub local: Local,
    pub projection: Vec<Projection>,
}

pub enum Projection {
    /// A field of a struct, tuple or closure environment, or of the enum
    /// variant selected by a preceding [`Projection::Downcast`].
    Field(usize),
    /// What a reference or pointer points to.
    Deref,
    /// An element of an array or slice; the index is a `usize` local.
    Index(Local),
    /// View an enum as one specific variant.
    Downcast(u32),
}
```

`(*_1).0` is local `_1`, then `Deref`, then `Field(0)`. `(_41 as variant 1).0` is `Downcast(1)` then `Field(0)`.

**Statements** do things, **terminators** decide where to go next:

```rust
pub enum Statement {
    Assign(Place, Rvalue),
    /// Record which variant an enum value holds.
    SetDiscriminant(Place, u32),
    /// Arguments are passed by value. One that does not travel in
    /// registers is handed over, memory and all: it belongs to the callee
    /// for the duration of the call, which may change it in place, and the
    /// caller does not look at it again.
    Call { dest: Place, callee: Callee, args: Vec<Operand> },
}

pub enum Terminator {
    Goto(BlockId),
    /// Two-way branch on a `bool`.
    Branch { cond: Operand, then_block: BlockId, else_block: BlockId },
    /// Multi-way branch on an integer.
    Switch { value: Operand, arms: Vec<(u128, BlockId)>, otherwise: BlockId },
    Return,
    /// Control can never get here. Reaching it anyway is a compiler bug or
    /// undefined behaviour in `unsafe` code; the program is stopped.
    Unreachable,
}
```

Calls are statements, not terminators: there is no unwinding (see [Not There Yet](#not-there-yet)), so a call always continues with the next statement.

**Values** are computed by rvalues from operands:

```rust
pub enum Rvalue {
    Use(Operand),
    /// Arithmetic, bitwise and comparison operators on primitives. Never
    /// `&&` / `||`: those are control flow.
    Binary(BinOp, Operand, Operand),
    Unary(UnaryOp, Operand),
    /// A primitive conversion between the two types.
    Cast(Operand, Ty, Ty),
    /// The address of a place.
    AddrOf(Place),
    /// The tag saying which variant an enum value holds.
    Discriminant(Place),
    /// Build a two-word pointer from a data pointer and its extra word
    /// (a slice length or a vtable address).
    MakeFat(Operand, Operand),
    /// The data pointer of a two-word pointer.
    FatData(Operand),
    /// The extra word of a two-word pointer.
    FatExtra(Operand),
}

pub enum Operand {
    /// The current value of a place.
    Copy(Place),
    Const(Const),
}
```

A call goes to one of four kinds of **callee**:

```rust
pub enum Callee {
    /// A function of this program.
    Direct(FuncId),
    /// A foreign function, by its C symbol.
    Extern { symbol: String, variadic: bool },
    /// Through a function pointer.
    Indirect(Operand),
    /// Through word `index` of the vtable of the trait object that is the
    /// call's first argument.
    Virtual { index: usize },
}
```

### Building the IR (`ir/build/`)

Starting from `main`, every function that is reachable is built once per set of concrete type arguments it is used with. That is **monomorphisation**: `Vec<i64>::push` and `Vec<String>::push` are two separate functions in the output, each with its own symbol (`_G.` followed by the path, with the type arguments in the name).

| File | Lowers |
|---|---|
| `build/mod.rs` | The work queue of function instances, symbols, statics and their initialisers |
| `build/expr.rs` | Expressions, statements, blocks, `if`, loops, `break`/`continue` with values, assignments |
| `build/call.rs` | Calls, intrinsics, closures, vtables |
| `build/pattern.rs` | `match`, `if let`, `let` with patterns |
| `build/drop.rs` | Scopes, drop flags, drop glue |

**Closures** become a struct holding the captures (by value or as references) plus a function taking that struct as its first argument. Calling a closure through `Fn`, `FnMut` or `FnOnce` calls that function.

**Trait objects.** A `&dyn Trait` is a fat pointer: the data pointer and a vtable pointer. The vtable is constant data built per (concrete type, trait) pair:

```
word 0: size of the concrete type
word 1: alignment
word 2: drop glue (VTABLE_DROP)
word 3…: the trait's methods (VTABLE_HEADER_WORDS = 3 words precede them)
```

A method call on a trait object becomes `Callee::Virtual { index }`, which loads the function from the vtable.

**Patterns** (`pattern.rs`) are compiled into a sequence of tests — read the discriminant and switch on it, compare a value, check a range or a slice length — followed by the bindings. Or-patterns and guards fall through to the next arm on failure, exactly as written.

**Statics** that are plain data are emitted as data. A static whose value needs code to compute (such as a `Mutex::new(…)` that allocates, or the panic hook) gets an *initialiser* function, run before `main` in declaration order.

### Ownership at run time (`build/drop.rs`)

Every local whose type needs dropping is owned by a scope and has a hidden boolean, its **drop flag**:

- set when the local receives a value;
- cleared when the value is moved out;
- checked when the scope ends: each local whose flag is still set is dropped, in reverse order of declaration.

Flags make every decision at run time. That is always correct, also when a value is moved on only one of several paths; the optimiser removes the flags whose value it can work out.

A **partial move** — a field moved out on its own, as in `let name = person.name;` — gives that field a flag of its own. Dropping the local then drops its fields one at a time, skipping those moved out. Rust forbids moving out of a value whose type implements `Drop`, so no destructor is skipped by doing this.

Dropping a value means calling its type's **drop glue**, a function generated per type (`drop_in_place<Formatter>` in the walkthrough): it runs the type's own `Drop::drop` if it has one, then drops each field. The resulting drop order matches rustc's, including temporaries in `match` scrutinees and at the end of statements, which the conformance suite checks with `Drop` impls that print.

### Memory layout (`layout.rs`)

One rule for everything, in every context — stack, heap, behind a pointer:

- **Structs and tuples**: fields in declaration order at ascending offsets, each aligned to its own alignment; the whole padded to a multiple of the largest alignment.
- **Enums**: a tag followed by the fields of whichever variant the value holds — unless a niche can hold the tag (below).
- **Arrays**: elements back to back.

```rust
pub struct Layout {
    pub size: u64,
    pub align: u64,
}
```

**Niches.** When all of an enum's variants but one hold no data, and the one that does contains a scalar that never takes some of its possible values — a reference or `Box` (never null), a `bool` (only 0 and 1), a `char` (never above `0x10FFFF`), another enum's tag — the other variants are stored as those impossible values and no tag is needed:

```rust
pub enum TagEncoding {
    /// A tag of its own, ahead of the fields, holding the discriminant.
    Direct(IntTy),
    /// The variants `first..first + count` other than `dataful` have no
    /// data; variant `first + i` is stored as `value + i` in the niche of
    /// the dataful variant's fields. A value of the niche's own range
    /// means `dataful`. Every discriminant is the variant's index.
    Niche { dataful: u32, first: u32, count: u32, niche: Niche, value: u128 },
}
```

The library marks `Box`, `Rc`, `Arc` and the `NonZero` integers with `#[rustc_nonnull_optimization_guaranteed]` to say their pointer or value is never zero.

Measured sizes, compared with rustc:

| Type | GaiaRusted | rustc |
|---|---:|---:|
| `Option<&i32>` | 8 | 8 |
| `Option<Box<i32>>` | 8 | 8 |
| `Option<Rc<i32>>` | 8 | 8 |
| `Option<i64>` | 16 | 16 |
| `Option<char>` | 4 | 4 |
| `Option<bool>` | 1 | 1 |
| `&str`, `&dyn Display` | 16 | 16 |
| `Vec<u8>`, `String` | 24 | 24 |
| `enum Shape { Circle(f64), Rect(f64, f64), Empty }` | 24 | 24 |
| `(u8, u32, u16)` | 12 | 8 |

The last row is the one difference: rustc reorders fields to reduce padding, GaiaRusted keeps declaration order. Programs cannot observe field order without `unsafe` code that relies on unspecified layout, so behaviour is the same.

**Unsized types** — `str`, `[T]`, `dyn Trait`, and structs whose last field is one of them — are only reached through **fat pointers**, which carry a length or a vtable next to the data pointer. For a struct with an unsized tail, such as the `RcBox<dyn Trait>` behind an `Rc<dyn Trait>`, the tail's offset depends on the alignment stored in the vtable and is computed at run time.

### Analyses and tools

| File | Provides |
|---|---|
| `analysis.rs` | Predecessors, liveness, dominators and natural loops — the facts the passes and the register allocator build on |
| `visit.rs` | Walking every place a statement or terminator reads or writes |
| `display.rs` | The textual form shown by `--emit-ir` |

---

## Stage 4: The Optimiser (`src/ir/opt/`)

**Purpose:** rewrite the IR into IR that does the same thing faster.

Every pass is a function from a correct program to a correct program. Passes can therefore be read, tested and reordered independently, and a wrong result at `-O2` that is right at `-O0` can be narrowed down to one pass.

### The order

At `-O0` nothing runs. At `-O1` and above:

```rust
pub fn optimize(program: &mut Program, tcx: &Context, level: OptLevel) {
    if level == OptLevel::None {
        return;
    }
    // Inlining tidies each function as it finishes with it.
    inline::run(program, tcx);
    prune::run(program);
}
```

The inliner visits functions **callees first** and, as it finishes each, *tidies* it:

```rust
pub fn run(program: &mut Program, tcx: &Context) {
    let (order, recursive) = callees_first(program);
    for caller in order {
        inline_calls_in(program, tcx, caller, &recursive);
        let func = &mut program.functions[caller.0 as usize];
        tidy(func, tcx);
        if recursive[caller.0 as usize] && tail_calls::run(func, caller, tcx) {
            tidy(func, tcx);
        }
    }
}
```

```rust
pub fn tidy(func: &mut Function, tcx: &Context) {
    /// Enough for what lowering and inlining produce; more rounds find little.
    const ROUNDS: usize = 6;
    simplify_cfg::run(func);
    for _ in 0..ROUNDS {
        let split = sroa::run(func, tcx);
        let propagated = propagate::run(func, tcx);
        let threaded = thread_jumps::run(func);
        simplify_cfg::run(func);
        let removed = dce::run(func);
        let hoisted = licm::run(func);
        if !(split || propagated || threaded || removed || hoisted) {
            break;
        }
    }
    // Last, as they make the code bigger for the passes above to look at.
    if rotate::run(func) {
        simplify_cfg::run(func);
    }
    addresses::run(func, tcx);
    compact::run(func);
}
```

Because callees are finished first, a function is copied into its callers in its final, optimised form, already containing whatever was inlined into it.

### The passes

| Pass | File | What it does | In the walkthrough |
|---|---|---|---|
| Inlining | `inline.rs` | Replaces a call to a small function (up to 48 statements) by its body. A function that takes part in a recursion is never copied. A caller stops absorbing callees at 8,000 statements. | `norm2`, `sum_to` and the formatting calls disappear into `main` |
| CFG simplification | `simplify_cfg.rs` | Merges a block into its only predecessor, skips empty blocks, removes unreachable ones | `bb4`–`bb7` of `sum_to` vanish |
| SROA | `sroa.rs` | Scalar replacement of aggregates: a struct, tuple or enum local whose address is not taken becomes one local per field | `p` becomes two integers; the `Formatter` becomes its fields |
| Propagation | `propagate.rs` | Uses known copies, constants and addresses where they are read; folds constant operations; reuses a value already loaded through a pointer when nothing can have changed it (load CSE) | `p.x * p.x + p.y * p.y` becomes `25` |
| Jump threading | `thread_jumps.rs` | When a predecessor already decides a branch, jumps straight to its target | — |
| Dead code elimination | `dce.rs` | Removes assignments whose value is never read and that have no side effects | the unit locals `_4`, `_5`, `_7`, `_8` go |
| LICM | `licm.rs` | Loop-invariant code motion: moves computations that do not change in a loop to before it, including loads through references the loop does not write | — |
| Loop rotation | `rotate.rs` | Moves a loop's test from the top to the bottom, so each iteration runs one branch instead of two | `if _10 goto bb1 else bb2` at the bottom |
| Address hoisting | `addresses.rs` | Computes the address of an array or slice element once per use site | — |
| Compaction | `compact.rs` | Forgets locals that are no longer mentioned | — |
| Tail calls | `tail_calls.rs` | Turns a function that calls itself on its way out into a loop | — |
| Pruning | `prune.rs` | Drops functions nothing refers to any more | `norm2` and `sum_to` are gone from the output |

`fold.rs` evaluates operations on constants (with Rust's wrapping and shifting semantics) for the passes that meet them.

### What the optimiser does not do (yet)

- Vectorise loops (SIMD). This is why `matmul` runs at 3.4x the time of `rustc -O`.
- Remove bounds checks proven redundant by a loop's range.
- Global value numbering beyond the load CSE in `propagate`.

---

## Stage 5: The x86-64 Backend (`src/x64/`)

**Purpose:** turn IR into GNU assembler text in Intel syntax.

### The calling convention (`abi.rs`)

GaiaRusted follows the System V AMD64 ABI, the convention of C on Linux, for every function, so that calling C (and being called by it) needs no special code. Each type is classified:

```rust
pub enum Class {
    /// No bits at all: nothing to load, store or pass.
    Zst,
    /// An integer, `bool`, `char` or thin pointer, in a general register.
    Int { size: u64, signed: bool },
    /// A float, in an SSE register.
    Float { size: u64 },
    /// Two words in two general registers: a fat pointer, or a 128-bit
    /// integer (low word first, as System V passes `__int128`).
    Pair,
    /// Anything else: lives in memory and is handled by address.
    Memory { size: u64 },
}
```

- Integer and pointer arguments go in `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9`, then on the stack.
- Float arguments go in `xmm0`–`xmm7`.
- A `Pair` (fat pointer, `u128`) takes two consecutive integer registers.
- A `Memory` argument is passed by address; a `Memory` result is written to space the caller provides, whose address is a hidden first argument.
- Results come back in `rax` (and `rdx` for a pair) or `xmm0`.
- `rsp` is 16-byte aligned at every call.

### Register allocation (`regalloc.rs`)

A local is given one register for the whole function, or stays in its stack slot. The algorithm is graph colouring:

1. Find the locals that can live in a register at all: scalars whose address is never taken.
2. Compute where each is live, and from that which locals *interfere*: one is written while the other still holds a value that will be read.
3. Hand out registers, the most heavily used locals first (a use inside a loop counts ten times more per level of nesting), each taking a register none of its interfering neighbours has. A local that finds none stays in memory.

```rust
/// General registers a call leaves intact. Using one costs a save and a
/// restore per call of the function.
pub const CALLEE_SAVED: [Reg; 5] = [Reg::Rbx, Reg::R12, Reg::R13, Reg::R14, Reg::R15];
/// General registers free for the taking between calls. The remaining
/// ones are the code generator's scratch registers.
const CALLER_SAVED: [Reg; 4] = [Reg::Rsi, Reg::Rdi, Reg::R8, Reg::R9];
/// SSE registers that never carry arguments, so that moving parameters to
/// their homes cannot overwrite a parameter still to be moved.
const FLOAT_REGISTERS: std::ops::Range<u8> = 8..16;
```

A call destroys the caller-saved registers, so a local that must keep its value across a call can only take a callee-saved one. There are no callee-saved SSE registers in System V, so a float that lives across a call stays in memory. A fat pointer takes two registers at once.

At `-O0` the allocator is off and every local lives in memory; that is the reference build.

### Instruction selection (`function.rs`)

`function.rs` generates the code of one function:

- **Prologue and epilogue**: `push rbp; mov rbp, rsp; sub rsp, N`, saving the callee-saved registers the allocator used; `leave; ret`.
- **Statements**: loads and stores of places (following `Deref`, `Field`, `Index` and `Downcast` projections into address computations), arithmetic and comparisons on every integer width and on floats (SSE), casts between them, discriminant reads and writes for both tag encodings, fat-pointer construction and access, memory copies for aggregates.
- **Arithmetic** wraps on overflow, as rustc's optimised build does. Division by zero and out-of-bounds indexing call the library's panic functions (`rt::panic_div_zero`, `rt::panic_bounds_check`).
- **Calls**: argument placement per the ABI, stack alignment, the four callee kinds, variadic C calls (with `al` set to the number of vector registers used).
- **Terminators**: jumps, two-way branches folded into the preceding comparison, switches as compare-and-branch chains, `ret`, and `ud2` for `unreachable`.
- **Intrinsics**: generated at the call site.

**Naming.** Functions are `_G.` followed by their path with `.` separators (`_G.fmt.Formatter.new`); blocks are `.Lf<function>_b<block>`. The C entry point `main` runs the static initialisers and then `_G.main`, as shown in the [walkthrough](#8-the-programs-entry-point).

### Atomics (`runtime.rs`)

The atomic operations the library's `std::sync::atomic` needs are emitted as small routines using `lock`-prefixed instructions and `xchg`. Plain aligned loads and stores are atomic on x86-64, and locked instructions are full barriers, so every routine is sequentially consistent, whatever ordering the library asks for.

### x86-64 key concepts

**Registers:**

| Register | Role in GaiaRusted |
|---|---|
| `rax` | Return value; scratch |
| `rdx` | Second return word; argument 3; scratch |
| `rdi`, `rsi`, `rcx`, `r8`, `r9` | Arguments 1, 2, 4, 5, 6; `rsi`, `rdi`, `r8`, `r9` also allocatable between calls |
| `rbx`, `r12`–`r15` | Callee-saved: allocated to locals that live across calls |
| `r10`, `r11` | Scratch |
| `rbp` | Frame pointer |
| `rsp` | Stack pointer, 16-byte aligned at calls |
| `xmm0`–`xmm7` | Float arguments and results |
| `xmm8`–`xmm15` | Allocated to float locals |

**Stack frame:**

```
[higher addresses]
[rbp + 16]   7th integer argument and beyond
[rbp + 8]    return address
[rbp]        caller's rbp
[rbp - 8]    first stack slot (locals, saved callee-saved registers)
…
[rsp]        outgoing stack arguments
[lower addresses]
```

**String constants** live in `.rodata` and are reached with RIP-relative addressing (`lea rax, [rip + _G.str.3]`). Executables are linked without position independence (`-no-pie`).

---

## The Standard Library (`library/`)

GaiaRusted's `std` is written in Rust: about 20,700 lines in 52 files, compiled together with every program. Its module structure and signatures follow the real `std`, so programs written for rustc compile unchanged.

### How it fits together

- **`libc.rs`** declares the C functions everything else is built on: `malloc`, `realloc`, `free`, `write`, `read`, `open`, `pthread_create`, `setjmp`, `longjmp` and others.
- **`intrinsics.rs`** declares what the language cannot express, such as `size_of`, `align_of` and type identity. The compiler generates their code at the call site.
- **`prelude.rs`** lists the names every module sees without importing them: `Vec`, `String`, `Option`, `Some`, `None`, `Result`, `Ok`, `Err`, `Box`, the common traits.
- **`rt.rs`** holds what compiled code calls when something goes wrong: bounds check failures, division by zero, `unwrap` on `None`.

A typical piece of the library is ordinary Rust:

```rust
pub struct Vec<T> {
    pointer: *mut T,
    len: usize,
    capacity: usize,
}

    pub fn push(&mut self, value: T) {
        if self.len == self.capacity {
            self.reserve(1);
        }
        self.pointer.add(self.len).write(value);
        self.len += 1;
    }
```

### Panics

A panic prints its message through the panic hook, then returns to the innermost `catch_unwind` running on its thread with the panic's payload. With none active, the program exits with status 101, as rustc's does. `catch_unwind` is built on `setjmp`/`longjmp`:

```rust
pub fn catch_unwind<B: UnwindBody<R>, R>(body: B) -> std::thread::Result<R> {
    let mut catcher = Catcher { buffer: [0; 25], payload: None, outer: innermost_catcher() };
    let mut body = Some(body);
    let mut result = None;
    if run_caught(&mut catcher, &mut body, &mut result) {
        return Err(catcher.payload.take().expect("a panic leaves its payload with its catcher"));
    }
    match result {
        Some(value) => Ok(value),
        None => unreachable!("a body that did not panic has returned"),
    }
}
```

The jump back means the frames a panic leaves are not unwound: values they own are not dropped (their memory leaks), and a `Mutex` they locked stays locked rather than becoming poisoned. Real unwinding is planned for 1.2.0.

### Notable implementations

- **`collections/hash_map.rs`**: open addressing with a control byte per slot, as in hashbrown (rustc's `HashMap`), probing groups of slots; `HashSet` is built on it. **`hash.rs`** feeds every type's bytes exactly as rustc does and provides SipHash-1-3 for `DefaultHasher` with rustc's zero keys, so a hash a program prints is the one rustc's prints. `RandomState` also uses fixed keys, so a `HashMap`'s iteration order is the same on every run, where rustc's changes from run to run.
- **`collections/btree_map.rs`**: a real B-tree; `BTreeSet` is built on it.
- **`fmt.rs`**: the `Formatter`, every format specifier, `Display`/`Debug` for the built-in types (including `{:?}` escaping of strings and chars exactly as rustc prints them), and float formatting that prints the same digits as rustc (`0.30000000000000004`, `1000000000000000000000`).
- **`unicode.rs`**: generated by `tools/gen_unicode.py` from the Unicode tables in rustc's own `core`, so `char::is_alphabetic`, `to_uppercase` and the rest agree with rustc character for character.
- **`sync/`**, **`thread.rs`**: threads on `pthread`, `Arc`, `Mutex`, `RwLock`, `Condvar`, `Once`, atomics, and `mpsc` channels.

### Module reference

| File | Lines | Contents |
|---|---:|---|
| `any.rs` | 69 | `Any`, `TypeId`: looking at types as the program runs |
| `array.rs` | 266 | `[T; N]`: the traits arrays implement for every length |
| `borrow.rs` | 64 | `Borrow`, `ToOwned` |
| `boxed.rs` | 201 | `Box<T>` |
| `cell.rs` | 381 | `Cell`, `RefCell`, `UnsafeCell` |
| `char.rs` | 367 | `char` methods |
| `clone.rs` | 56 | `Clone` |
| `cmp.rs` | 392 | `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Ordering`, `Reverse`, `min`/`max` |
| `collections/binary_heap.rs` | 296 | `BinaryHeap<T>` |
| `collections/btree_map.rs` | 1,078 | `BTreeMap<K, V>` |
| `collections/btree_set.rs` | 483 | `BTreeSet<T>` |
| `collections/hash_map.rs` | 894 | `HashMap<K, V>` |
| `collections/hash_set.rs` | 379 | `HashSet<T>` |
| `collections/vec_deque.rs` | 621 | `VecDeque<T>` |
| `convert.rs` | 196 | `From`, `Into`, `TryFrom`, `TryInto`, `AsRef`, `AsMut` |
| `default.rs` | 65 | `Default` |
| `env.rs` | 119 | Environment variables, arguments, directories |
| `error.rs` | 70 | The `Error` trait |
| `f32.rs`, `f64.rs` | 27 each | Float constants |
| `ffi.rs` | 140 | `OsStr`, `OsString` |
| `fmt.rs` | 1,011 | Formatting |
| `fs.rs` | 284 | Files and the file system |
| `hash.rs` | 433 | `Hash`, `Hasher`, SipHash, `BuildHasher` |
| `intrinsics.rs` | 43 | Compiler intrinsics |
| `io.rs` | 840 | `stdin`/`stdout`/`stderr`, `Read`, `Write`, `BufRead`, `BufReader` |
| `iter.rs` | 1,575 | `Iterator`, its adapters, and the iterator traits |
| `libc.rs` | 115 | The C functions underneath |
| `marker.rs` | 75 | `Copy`, `Send`, `Sync`, `Sized`, `PhantomData` |
| `mem.rs` | 154 | `swap`, `replace`, `take`, `forget`, `size_of`, `ManuallyDrop`, `MaybeUninit` |
| `num.rs` | 759 | Integer and float methods, `NonZero*`, parsing |
| `ops.rs` | 322 | Operator traits, ranges, `Deref`, `Drop`, `Fn*`, `CoerceUnsized` |
| `option.rs` | 389 | `Option<T>` |
| `panic.rs` | 269 | Panics, `catch_unwind`, the panic hook |
| `path.rs` | 400 | `Path`, `PathBuf` |
| `prelude.rs` | 18 | The prelude |
| `process.rs` | 17 | `exit`, `abort` |
| `ptr.rs` | 106 | `null`, `read`, `write`, `drop_in_place` and the raw pointer methods |
| `rc.rs` | 348 | `Rc<T>`, `Weak<T>` |
| `result.rs` | 293 | `Result<T, E>` |
| `rt.rs` | 31 | Runtime failure entry points |
| `slice.rs` | 1,031 | Slice methods: sorting, searching, chunks, windows, iterators |
| `str.rs` | 1,229 | `str` methods: searching, splitting, trimming, parsing, chars |
| `string.rs` | 497 | `String` |
| `sync/atomic.rs` | 259 | Atomic types |
| `sync/mod.rs` | 788 | `Arc`, `Mutex`, `RwLock`, `Condvar`, `Once` |
| `sync/mpsc.rs` | 221 | Channels |
| `thread.rs` | 183 | `spawn`, `JoinHandle`, `sleep`, `scope` |
| `time.rs` | 412 | `Duration`, `Instant`, `SystemTime` |
| `unicode.rs` | 1,842 | Generated Unicode tables |
| `vec.rs` | 585 | `Vec<T>` |

---

## Testing Strategy

The compiler is tested the way it is meant to be used: by compiling real programs and comparing what they print with what rustc's build prints.

### Conformance (`conformance/`, `tests/conformance.rs`)

64 programs, each focused on an area (`closures`, `drop_order`, `hash_map_ownership`, `niche_layout`, `trait_objects`, `unicode_chars` …) and each written to print a lot. Next to each `NAME.rs` is `NAME.stdout`, the output of rustc's build, and optionally `NAME.stdin`, fed to the program.

`cargo test --release --test conformance` compiles every program at `-O0` and at `-O2`, runs both, and requires both outputs to equal the expected output byte for byte. A failure reports the first differing line:

```
closures: output differs at line 12
  expected: "captured 3 by move"
  actual:   "captured 0 by move"
```

To add a program, write `conformance/NAME.rs` and run the test once with `BLESS=1`, which records rustc's output as the expectation. Programs that run longer than 20 seconds are stopped.

**Rejection.** `conformance/reject/` holds programs rustc rejects. Each starts with a comment naming the error it must fail with:

```rust
// error: field `h` of struct `Rect` is private
```

The test checks that rustc rejects the program, that GaiaRusted rejects it too, and that GaiaRusted's message contains the expected text.

### The driver and the command line (`tests/`)

| File | Tests |
|---|---|
| `driver.rs` | Each output format produces its file and nothing else; linking a C library built in the test; a missing library reports the linker's error; an unwritable output path; rebuilding a library replaces it; Cargo projects with modules and several binaries; manifest forms; dependency and library-crate errors |
| `scratch.rs` | No scratch folders are left in the temporary directory (alone in its test binary, so no other build is in flight while it looks) |
| `cli.rs` | Exit codes 0, 1 and 2; silent success; the `could not compile` line; default output names; `--version` and `--help`; building a project folder |

### Coverage (`coverage/`)

60 programs, one per area of the language and library (basics, types, traits, memory, collections, strings, errors, patterns, iterators, modules, macros, threads, unsafe, I/O, files, Unicode …). `python3 coverage/run.py` compiles each with rustc as the oracle and with GaiaRusted at `-O0` and `-O2`, compares output and exit code, and flags programs that pass at `-O0` but fail at `-O2` as optimiser bugs.

`--harvest` is the tool for finding what is missing: for each error, it removes the statement the error points at and compiles again, until the program compiles, then lists every missing piece the program needed.

### Benchmarks (`benchmarks/`)

17 programs: 5 small ones and 12 realistic ones (binary trees, n-queens, Mandelbrot, word frequencies, matrix multiplication, BFS on a grid, a Brainfuck interpreter, LCS by dynamic programming, string sorting, a tokenizer, integer hash maps, dynamic dispatch over shapes). `python3 benchmarks/run.py --runs N` builds each with GaiaRusted, `rustc -O0` and `rustc -O`, checks the outputs agree, and reports the best run times and the build times.

### Running everything

```bash
cargo test --release --test conformance --test driver --test cli --test scratch
python3 coverage/run.py
python3 benchmarks/run.py --runs 3
```

The scripts need `python3` and `rustc`. On a machine with little memory, build with `-j 4`.

---

## Common Tasks

### Fixing a wrong result

1. **Reduce** the program until it is as small as it can be while still wrong. Compare with rustc's output each time.
2. **Is it the optimiser?** Build at `-O0` and `-O2`. If only `-O2` is wrong, the bug is in a pass.
   - Compare `--emit-ir -O0` with `--emit-ir` around the wrong value.
   - Turn passes off one at a time in `tidy` and `optimize` (`src/ir/opt/mod.rs`) until the output is right; the last pass turned off is the culprit, or exposes it.
3. **Otherwise** the bug is in IR building, layout or the backend. Read `--emit-ir -O0` first: if the IR is right, read `--format asm -O0`.
4. **Fix the cause**, not the symptom.
5. **Add the reduced program to `conformance/`** and bless it, so it stays fixed.

### Adding a language feature

Touch the stages in order, and stop as soon as the feature can be expressed with what already exists:

1. **Parser and AST** (`syntax/parser/`, `syntax/ast.rs`), if it is new syntax.
2. **Declarations** (`sema/defs.rs`), if it declares something.
3. **Type checking** (`sema/check/`). Prefer producing existing THIR nodes: `let else` is a `match`, `while let` is a `loop` with a `match`, `?` is a `match` with a `return`.
4. **IR building** (`ir/build/`), only if it needs new control flow or data.
5. **A conformance program** using the feature in as many ways as possible.

### Adding to the standard library

Write it in `library/` in ordinary Rust, with the real `std` signature, in the module where `std` has it. If you add a new file, add it to the `LIBRARY` table in `src/pipeline.rs`. Then run the conformance suite: the library is compiled with every test program, so a mistake anywhere shows up immediately.

### Adding an optimisation pass

1. Write it as `src/ir/opt/NAME.rs` with a `run(func: &mut Function, …) -> bool` that reports whether it changed anything.
2. Add it to `tidy` (if it should iterate with the others) or after the loop.
3. Run the conformance suite at both levels, and the benchmarks before and after.

### Adding a rejected program

Write `conformance/reject/NAME.rs` with a first line `// error: <the message you expect>` and run the conformance test.

---

## Performance Considerations

### Compile time

A program builds in 150–200 ms, most of it spent on the standard library, which is parsed with every program and type-checked as far as the program uses it. The type checker's time goes mostly to impl selection, which is why it is indexed and cached (see [The context](#the-context-contextrs)). Assembling and linking take a few tens of milliseconds.

### Run time

Against `rustc -O` (lower is better):

| Benchmark | Ratio | | Benchmark | Ratio |
|---|---:|---|---|---:|
| sieve | 0.89x | | binary_trees | 1.62x |
| shapes_dyn | 0.98x | | iterators | 1.62x |
| brainfuck | 1.28x | | sort_strings | 1.74x |
| mandelbrot | 1.28x | | nqueens | 1.76x |
| hashmap_ints | 1.39x | | bfs_grid | 1.77x |
| collections | 1.55x | | lcs_dp | 1.93x |
| nbody | 1.56x | | tokenizer | 2.07x |
| fib | 1.57x | | word_freq | 2.10x |
| | | | matmul | 3.39x |

Geometric mean 1.6x; every benchmark runs 1.3–14x faster than `rustc -O0`. The remaining gap comes from:

- **no vectorisation** (`matmul`);
- **bounds checks** inside loops that rustc proves redundant;
- **instruction selection**: string comparison loops, for example, are longer than LLVM's.

### Binary size

A hello-world executable is about 70 KB: only the library code the program uses is compiled into it, and it links against the shared C library.

---

## Common Gotchas

### 1. An error in an uncalled generic function is not reported

Generic functions are type-checked per instantiation. If nothing calls `fn helper<T>(…)`, its body is never checked, and a type error in it goes unnoticed until something does. rustc reports it immediately.

### 2. The standard library is part of every compilation

A mistake in `library/` breaks every program, and a slow construct in the library slows down every build. Run the conformance suite after any library change.

### 3. `-O0` is the reference

When `-O0` and `-O2` disagree, `-O0` is almost always right: it translates each statement on its own, with every local in memory. Look for the bug in the passes first.

### 4. Drop flags are set even when they seem unnecessary

The IR builder gives every droppable local a flag and leaves removing the redundant ones to the optimiser. Do not "optimise" the builder by deciding statically; it is easy to get wrong when a value is moved on only some paths.

### 5. Stack alignment

`rsp` must be 16-byte aligned at every `call`, or C functions such as `printf` crash on SSE instructions. Frames are sized to keep it aligned; code that pushes for a call must account for it.

### 6. Callee-saved registers

`rbx`, `rbp` and `r12`–`r15` must have their values back when a function returns. The prologue saves those the allocator used; generated code must never use one as a scratch register.

### 7. Floats across calls

There are no callee-saved SSE registers, so a float that lives across a call stays in memory. This is correct but slower; it shows in float-heavy code with calls in its loops.

### 8. Field order

Struct fields are laid out in declaration order; rustc reorders them. Safe code cannot tell the difference, but `unsafe` code that assumes a particular layout of a non-`#[repr(C)]` struct is relying on something neither compiler promises.

---

## Not There Yet

These are the pieces planned for 1.2.0, and where they will go.

### Safety analysis

Today the compiler accepts programs that rustc rejects for breaking Rust's safety rules. Valid programs are unaffected. The checks will run between type checking and IR building, on the typed tree and the IR:

| Check | Rejects |
|---|---|
| Mutability | `x = 2` on an immutable binding, `v.push(1)` on an immutable `Vec`, `&mut` of an immutable place, mutation through `&` |
| `unsafe` | Calling an `unsafe` or `extern` function, dereferencing a raw pointer, or touching a `static mut` outside `unsafe` |
| Exhaustiveness | A `match` that misses a case; a refutable pattern in `let` |
| Initialisation | Reading a variable before it is assigned on every path |
| Moves | Use after move, moving in a loop, using a value a `move` closure took |
| Borrows and lifetimes | Conflicting borrows, two `&mut` at once, references that outlive what they point to |
| `Send` / `Sync` | Sending an `Rc` or a `RefCell` reference to another thread |

### `async` / `await`

Lowering async functions to state machines in `ir/build`, with `Future`, `Pin`, `Context`, `Waker` and a simple executor in the library.

### Unwinding

Running destructors when a panic leaves a frame, and poisoning a `Mutex` held during a panic. This turns calls into terminators with a cleanup edge.

### Diagnostics

Reporting several errors per build instead of stopping at the first, and a redesigned presentation of errors and compiler output.

---

## The Earlier Pipeline

Releases up to 1.1.0 were built on an earlier compiler, which this one replaces. Its modules are still in `src/`:

| Module | Lines | Was |
|---|---:|---|
| `lexer` | 1,300 | Tokenizer |
| `parser` | 3,900 | Parser and AST |
| `lowering` | 5,200 | AST to HIR |
| `typechecker` | 6,200 | Type inference |
| `typesystem` | 16,800 | Types, constraints, traits |
| `borrowchecker` | 13,400 | Ownership and lifetime analyses |
| `mir` | 3,400 | MIR and its optimisations |
| `codegen` | 22,600 | x86-64 generation and optimisation modules |
| `runtime` | 7,800 | Runtime routines in assembly |
| `stdlib` | 8,600 | Built-in functions |
| `compiler`, `config` and about twenty more | | Orchestration, error display, Cargo API, incremental compilation, … |

In total about 122,000 lines. Why it was replaced, in short:

- **Untyped MIR.** Types were lost after type checking, so code generation guessed sizes and representations, and aggregates were laid out differently in different places.
- **Built-ins instead of a library.** Collections and strings were assembly routines with fixed layouts, so generic code over them could not be compiled faithfully.
- **No reference to measure against.** Its tests checked the compiler's internals, not whether programs printed what rustc's builds print.

The `gaiarusted` command no longer uses any of it. Only the older library API, `compile_files` in `src/compiler.rs`, still falls back to it when the typed pipeline rejects a program. The `repl` binary and `src/lsp` are separate older tools that will be rebuilt on the new pipeline.

All of this is removed in 1.2.0. Until then, new work goes into `syntax`, `sema`, `ir`, `x64`, `driver` and `library`.

---

## Complete File Reference

### Root files

| File | Purpose |
|---|---|
| `Cargo.toml` | Package manifest: the `gaiarusted` library, the `gaiarusted` and `repl` binaries, no dependencies |
| `Cargo.lock` | Lock file |
| `README.md` | Overview, usage, status |
| `ARCHITECTURE.md` | This file |
| `CONTRIBUTING.md` | Contribution guidelines |
| `CODE_OF_CONDUCT.md` | Community standards |
| `LICENSE` | MIT |

### Command line, driver, pipeline

| File | Lines | Purpose |
|---|---:|---|
| `src/bin/gaiarusted.rs` | 178 | Argument parsing, help, version, reporting, default output names |
| `src/driver/mod.rs` | 123 | `Format`, `Options`, `Failure`, `build`, `emit_ir` |
| `src/driver/link.rs` | 110 | `Scratch` folders; running `as`, `gcc`, `ar` |
| `src/driver/project.rs` | 196 | `Cargo.toml` reading, binary discovery, `build_project`, `program_in` |
| `src/pipeline.rs` | 176 | The stages in order; the embedded library; `Failure` |
| `src/lib.rs` | — | Module declarations |

### Syntax

| File | Lines | Purpose |
|---|---:|---|
| `src/syntax/mod.rs` | 20 | Module root |
| `src/syntax/token.rs` | 221 | `Token`, `TokenKind`, keywords |
| `src/syntax/lexer.rs` | 448 | Source text to tokens |
| `src/syntax/span.rs` | 114 | `Span`, `SourceMap`, line and column lookup |
| `src/syntax/diagnostic.rs` | 71 | `Diagnostic` and its rendering |
| `src/syntax/ast.rs` | 532 | The syntax tree |
| `src/syntax/build.rs` | 145 | Building AST by hand for generated code |
| `src/syntax/visit.rs` | 103 | AST walking |
| `src/syntax/parser/mod.rs` | 411 | Parser state and helpers |
| `src/syntax/parser/item.rs` | 474 | Items |
| `src/syntax/parser/expr.rs` | 708 | Expressions, statements, blocks |
| `src/syntax/parser/pat.rs` | 200 | Patterns |
| `src/syntax/parser/ty.rs` | 315 | Types, paths, generics, bounds |
| `src/syntax/parser/macros.rs` | 689 | `macro_rules!` |

### Semantic analysis

| File | Lines | Purpose |
|---|---:|---|
| `src/sema/mod.rs` | 28 | Module root |
| `src/sema/derive.rs` | 303 | `#[derive]` expansion |
| `src/sema/defs.rs` | 812 | Declarations, modules, imports, visibility |
| `src/sema/context.rs` | 1,626 | Signatures, fields, impl selection, caches |
| `src/sema/ty.rs` | 370 | `Ty` and friends |
| `src/sema/infer.rs` | 297 | `InferTable`, unification, snapshots |
| `src/sema/thir.rs` | 383 | The typed tree |
| `src/sema/const_eval.rs` | 100 | Compile-time integer evaluation |
| `src/sema/impl_trait.rs` | 91 | `impl Trait` in argument position |
| `src/sema/check/mod.rs` | 797 | Body checking driver |
| `src/sema/check/expr.rs` | 860 | Literals, operators, fields, indexing, struct literals |
| `src/sema/check/path.rs` | 893 | Names and calls |
| `src/sema/check/method.rs` | 445 | Method lookup |
| `src/sema/check/control.rs` | 583 | Control flow, closures, `?` |
| `src/sema/check/pat.rs` | 382 | Patterns |
| `src/sema/check/coerce.rs` | 263 | Coercions |
| `src/sema/check/macros.rs` | 601 | Built-in macros |

### Intermediate representation and optimiser

| File | Lines | Purpose |
|---|---:|---|
| `src/ir/mod.rs` | 284 | `Program`, `Function`, `Block`, `Statement`, `Terminator`, `Place`, `Rvalue` |
| `src/ir/layout.rs` | 426 | Layout, niches, unsized tails |
| `src/ir/analysis.rs` | 354 | Predecessors, liveness, dominators, loops |
| `src/ir/visit.rs` | 210 | Place walking |
| `src/ir/display.rs` | 119 | `--emit-ir` |
| `src/ir/build/mod.rs` | 385 | Instances, symbols, statics |
| `src/ir/build/expr.rs` | 622 | Expressions and control flow |
| `src/ir/build/call.rs` | 445 | Calls, intrinsics, closures, vtables |
| `src/ir/build/pattern.rs` | 342 | Pattern matching |
| `src/ir/build/drop.rs` | 468 | Drop flags and drop glue |
| `src/ir/opt/mod.rs` | 91 | `optimize`, `tidy`, `OptLevel` |
| `src/ir/opt/inline.rs` | 249 | Inlining |
| `src/ir/opt/sroa.rs` | 308 | Scalar replacement of aggregates |
| `src/ir/opt/propagate.rs` | 458 | Copy, constant and load propagation |
| `src/ir/opt/fold.rs` | 177 | Constant folding |
| `src/ir/opt/thread_jumps.rs` | 105 | Jump threading |
| `src/ir/opt/simplify_cfg.rs` | 135 | CFG simplification |
| `src/ir/opt/dce.rs` | 55 | Dead code elimination |
| `src/ir/opt/licm.rs` | 184 | Loop-invariant code motion |
| `src/ir/opt/rotate.rs` | 44 | Loop rotation |
| `src/ir/opt/addresses.rs` | 109 | Element address hoisting |
| `src/ir/opt/compact.rs` | 54 | Removing unused locals |
| `src/ir/opt/tail_calls.rs` | 173 | Tail recursion to loops |
| `src/ir/opt/prune.rs` | 70 | Removing unused functions |

### Backend

| File | Lines | Purpose |
|---|---:|---|
| `src/x64/mod.rs` | 74 | Program-level emission, entry point |
| `src/x64/abi.rs` | 125 | System V classification and call layout |
| `src/x64/regalloc.rs` | 264 | Graph-colouring register allocation |
| `src/x64/function.rs` | 1,792 | Instruction selection for one function |
| `src/x64/reg.rs` | 107 | Registers and memory operands |
| `src/x64/runtime.rs` | 65 | Atomic operations |

### Tests and tools

| File | Purpose |
|---|---|
| `tests/conformance.rs` | Runs `conformance/` and `conformance/reject/` |
| `tests/driver.rs` | Driver and Cargo project tests |
| `tests/scratch.rs` | Temporary-folder cleanup |
| `tests/cli.rs` | Command-line behaviour |
| `coverage/run.py` | Coverage runner and harvester |
| `benchmarks/run.py` | Benchmark runner |
| `tools/gen_unicode.py` | Generates `library/unicode.rs` from rustc's `core` |

---

## Build Your Own Compiler: A Tutorial

You have just read how GaiaRusted works. This section is the hands-on version: in about a thousand lines of Rust, with no dependencies, you will build **mini**, a compiler that turns a small subset of Rust into a real x86-64 Linux executable. It has the same stages as GaiaRusted — lexer, parser, checker, IR, optimiser, backend, driver — in their simplest working form, so when you are done, every part of GaiaRusted will look like a bigger version of something you wrote yourself.

Everything below has been compiled and tested: the programs in [Step 9](#step-9-test-against-rustc) print exactly what rustc's builds of them print.

### What mini compiles

A subset of Rust, chosen so that every mini program is also a valid Rust program:

```rust
// Fibonacci numbers, the slow way.
fn fib(n: i64) -> i64 {
    if n < 2 {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

fn main() {
    let mut i = 0;
    while i < 15 {
        print(fib(i));
        i = i + 1;
    }
}
```

- Types: `i64` and `bool`.
- Functions with up to six parameters; values are returned with `return`.
- `let`, `let mut`, assignment, `if` / `else if` / `else`, `while`.
- Arithmetic `+ - * / %`, comparisons, `&&`, `||` (short-circuiting), `!`, unary `-`.
- A built-in `print(x: i64)` that prints a number on its own line.

### What you need

- Rust (stable), to build mini.
- `gcc` and GNU binutils, to assemble and link what mini produces. On Linux they are usually installed already.
- Some familiarity with Rust enums and `match`. No prior compiler knowledge.

```bash
cargo new mini
cd mini
```

All the code goes into `src/main.rs`, in the order of the steps. Start the file with:

```rust
//! mini: a compiler for a small subset of Rust, from source to an x86-64
//! Linux executable, in one file.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
```

### The plan

```
source ─▶ lexer ─▶ tokens ─▶ parser ─▶ syntax tree ─▶ checker ─▶ (errors?)
                                                         │
                         assembly ◀─ backend ◀─ optimiser ◀─ IR ◀─ lowering
                            │
                            └──▶ gcc ──▶ executable
```

| Step | mini | GaiaRusted |
|---|---|---|
| 1. Lexer | `lex` | `src/syntax/lexer.rs` |
| 2. Syntax tree | `Expr`, `Stmt`, `Function` | `src/syntax/ast.rs` |
| 3. Parser | `Parser` | `src/syntax/parser/` |
| 4. Checker | `Checker` | `src/sema/` |
| 5. IR | `IrFunction`, `Lowerer` | `src/ir/`, `src/ir/build/` |
| 6. Optimiser | `fold_constants`, `remove_unreachable_blocks` | `src/ir/opt/` |
| 7. Backend | `emit_program` | `src/x64/` |
| 8. Driver | `compile`, `main` | `src/pipeline.rs`, `src/driver/` |

---

### Step 1: The lexer

A compiler cannot work on raw characters. The lexer groups them into **tokens**, the words and punctuation of the language: `while`, `i`, `<`, `15`, `{`. Whitespace and comments disappear here, and every token remembers its line, so errors later can say where they are.

The lexer is a loop over the characters with one branch per kind of token:

- a digit starts a number: take all the digits that follow;
- a letter or `_` starts a word: take letters, digits and `_`, then check whether the word is a keyword;
- otherwise it is punctuation. Two-character operators (`==`, `<=`, `->`, `&&`) are checked before one-character ones, so that `<=` does not become `<` followed by `=`.

```rust
#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Int(i64),
    Ident(String),
    Fn, Let, Mut, If, Else, While, Return, True, False,
    LParen, RParen, LBrace, RBrace, Comma, Semi, Colon, Arrow,
    Plus, Minus, Star, Slash, Percent, Assign,
    EqEq, NotEq, Lt, Le, Gt, Ge, AndAnd, OrOr, Not,
    Eof,
}

struct Token {
    tok: Tok,
    line: usize,
}

fn lex(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let (mut i, mut line) = (0, 1);
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let value = text.parse().map_err(|_| format!("line {line}: `{text}` is too large for i64"))?;
            tokens.push(Token { tok: Tok::Int(value), line });
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let tok = match word.as_str() {
                "fn" => Tok::Fn,
                "let" => Tok::Let,
                "mut" => Tok::Mut,
                "if" => Tok::If,
                "else" => Tok::Else,
                "while" => Tok::While,
                "return" => Tok::Return,
                "true" => Tok::True,
                "false" => Tok::False,
                _ => Tok::Ident(word),
            };
            tokens.push(Token { tok, line });
        } else {
            let pair: String = chars[i..(i + 2).min(chars.len())].iter().collect();
            let double = match pair.as_str() {
                "->" => Some(Tok::Arrow),
                "==" => Some(Tok::EqEq),
                "!=" => Some(Tok::NotEq),
                "<=" => Some(Tok::Le),
                ">=" => Some(Tok::Ge),
                "&&" => Some(Tok::AndAnd),
                "||" => Some(Tok::OrOr),
                _ => None,
            };
            let (tok, width) = match double {
                Some(tok) => (tok, 2),
                None => {
                    let tok = match c {
                        '(' => Tok::LParen,
                        ')' => Tok::RParen,
                        '{' => Tok::LBrace,
                        '}' => Tok::RBrace,
                        ',' => Tok::Comma,
                        ';' => Tok::Semi,
                        ':' => Tok::Colon,
                        '+' => Tok::Plus,
                        '-' => Tok::Minus,
                        '*' => Tok::Star,
                        '/' => Tok::Slash,
                        '%' => Tok::Percent,
                        '=' => Tok::Assign,
                        '<' => Tok::Lt,
                        '>' => Tok::Gt,
                        '!' => Tok::Not,
                        _ => return Err(format!("line {line}: unexpected character `{c}`")),
                    };
                    (tok, 1)
                }
            };
            tokens.push(Token { tok, line });
            i += width;
        }
    }
    tokens.push(Token { tok: Tok::Eof, line });
    Ok(tokens)
}
```

The last token is always `Eof`, so the parser can look one token ahead without running off the end.

> **In GaiaRusted** the lexer does the same with many more cases: hex, octal and binary numbers, type suffixes (`7u8`), floats, strings with escapes, raw strings, chars, lifetimes, nested comments. And instead of a line number, each token carries a `Span` — the exact byte range — which is how errors can underline the offending code with `^^^`.

---

### Step 2: The syntax tree

The parser's output is a tree that mirrors the structure of the program. `2 + 3 * 4` becomes:

```
Binary(Add, Int(2), Binary(Mul, Int(3), Int(4)))
```

Expressions produce values; statements do things; functions hold statements. Each is a Rust enum, and that is the main reason Rust is such a good language for writing compilers: a `match` over these enums forces you to handle every case.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum Type {
    Int,
    Bool,
    Unit,
}

impl Type {
    fn name(self) -> &'static str {
        match self {
            Type::Int => "i64",
            Type::Bool => "bool",
            Type::Unit => "()",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BinOp {
    Add, Sub, Mul, Div, Rem,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum UnOp {
    Neg,
    Not,
}

#[derive(Debug)]
enum Expr {
    Int(i64),
    Bool(bool),
    Var(String),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug)]
struct Stmt {
    kind: StmtKind,
    line: usize,
}

#[derive(Debug)]
enum StmtKind {
    Let { name: String, mutable: bool, value: Expr },
    Assign(String, Expr),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    Return(Option<Expr>),
    Expr(Expr),
}

#[derive(Debug)]
struct Function {
    name: String,
    params: Vec<(String, Type)>,
    ret: Type,
    body: Vec<Stmt>,
    line: usize,
}
```

Statements carry their line for error messages. `Type::Unit` is `()`, the type of a function that returns nothing.

---

### Step 3: The parser

The parser is **recursive descent**: one function per construct, each consuming the tokens of its construct and calling the others for the parts. `function` parses `fn name(params) -> type` and calls `block` for the body; `block` calls `statement` until it sees `}`; `statement` looks at the first token to decide what kind of statement it is.

The interesting part is expressions, because of **precedence**: `1 + 2 * 3` must be `1 + (2 * 3)`, and `1 - 2 - 3` must be `(1 - 2) - 3`. The technique is **precedence climbing**. Every binary operator gets a number — higher binds tighter — and `expr(min)` parses an expression whose operators all bind more tightly than `min`:

1. Parse one operand (a number, a name, a call, a parenthesised expression, or `-`/`!` followed by an operand).
2. While the next token is an operator binding more tightly than `min`: consume it, parse the right-hand side with `expr(its precedence)`, and combine.

Trace `1 - 2 * 3`, starting with `expr(0)`:

```
expr(0): operand 1
         sees `-` (4 > 0): right side is expr(4)
             expr(4): operand 2
                      sees `*` (5 > 4): right side is expr(5)
                          expr(5): operand 3, sees end → 3
                      → 2 * 3
         → 1 - (2 * 3)
```

And `1 - 2 - 3`: inside `expr(4)` the second `-` has precedence 4, which is not greater than 4, so the inner call stops and the outer loop takes it: `(1 - 2) - 3`, left-associative, as in Rust.

```rust
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

/// The operator a token stands for, and how tightly it binds.
fn binary_op(tok: &Tok) -> Option<(BinOp, u8)> {
    Some(match tok {
        Tok::OrOr => (BinOp::Or, 1),
        Tok::AndAnd => (BinOp::And, 2),
        Tok::EqEq => (BinOp::Eq, 3),
        Tok::NotEq => (BinOp::Ne, 3),
        Tok::Lt => (BinOp::Lt, 3),
        Tok::Le => (BinOp::Le, 3),
        Tok::Gt => (BinOp::Gt, 3),
        Tok::Ge => (BinOp::Ge, 3),
        Tok::Plus => (BinOp::Add, 4),
        Tok::Minus => (BinOp::Sub, 4),
        Tok::Star => (BinOp::Mul, 5),
        Tok::Slash => (BinOp::Div, 5),
        Tok::Percent => (BinOp::Rem, 5),
        _ => return None,
    })
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.tokens[self.pos].tok
    }

    fn line(&self) -> usize {
        self.tokens[self.pos].line
    }

    fn next(&mut self) -> Tok {
        let tok = self.tokens[self.pos].tok.clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, want: Tok) -> Result<(), String> {
        if *self.peek() == want {
            self.next();
            Ok(())
        } else {
            Err(format!("line {}: expected {:?}, found {:?}", self.line(), want, self.peek()))
        }
    }

    fn ident(&mut self) -> Result<String, String> {
        let line = self.line();
        match self.next() {
            Tok::Ident(name) => Ok(name),
            other => Err(format!("line {line}: expected a name, found {other:?}")),
        }
    }

    fn program(&mut self) -> Result<Vec<Function>, String> {
        let mut functions = Vec::new();
        while *self.peek() != Tok::Eof {
            functions.push(self.function()?);
        }
        Ok(functions)
    }

    fn function(&mut self) -> Result<Function, String> {
        let line = self.line();
        self.expect(Tok::Fn)?;
        let name = self.ident()?;
        self.expect(Tok::LParen)?;
        let mut params = Vec::new();
        while *self.peek() != Tok::RParen {
            let param = self.ident()?;
            self.expect(Tok::Colon)?;
            params.push((param, self.ty()?));
            if *self.peek() != Tok::RParen {
                self.expect(Tok::Comma)?;
            }
        }
        self.expect(Tok::RParen)?;
        let ret = if *self.peek() == Tok::Arrow {
            self.next();
            self.ty()?
        } else {
            Type::Unit
        };
        let body = self.block()?;
        Ok(Function { name, params, ret, body, line })
    }

    fn ty(&mut self) -> Result<Type, String> {
        let line = self.line();
        match self.ident()?.as_str() {
            "i64" => Ok(Type::Int),
            "bool" => Ok(Type::Bool),
            other => Err(format!("line {line}: unknown type `{other}`")),
        }
    }

    fn block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(Tok::LBrace)?;
        let mut stmts = Vec::new();
        while *self.peek() != Tok::RBrace {
            stmts.push(self.statement()?);
        }
        self.expect(Tok::RBrace)?;
        Ok(stmts)
    }

    fn statement(&mut self) -> Result<Stmt, String> {
        let line = self.line();
        let kind = match self.peek().clone() {
            Tok::Let => {
                self.next();
                let mutable = *self.peek() == Tok::Mut;
                if mutable {
                    self.next();
                }
                let name = self.ident()?;
                self.expect(Tok::Assign)?;
                let value = self.expr(0)?;
                self.expect(Tok::Semi)?;
                StmtKind::Let { name, mutable, value }
            }
            Tok::If => {
                self.next();
                self.if_rest()?
            }
            Tok::While => {
                self.next();
                let cond = self.expr(0)?;
                StmtKind::While(cond, self.block()?)
            }
            Tok::Return => {
                self.next();
                let value = if *self.peek() == Tok::Semi { None } else { Some(self.expr(0)?) };
                self.expect(Tok::Semi)?;
                StmtKind::Return(value)
            }
            Tok::Ident(name) if self.tokens[self.pos + 1].tok == Tok::Assign => {
                self.next();
                self.next();
                let value = self.expr(0)?;
                self.expect(Tok::Semi)?;
                StmtKind::Assign(name, value)
            }
            _ => {
                let expr = self.expr(0)?;
                self.expect(Tok::Semi)?;
                StmtKind::Expr(expr)
            }
        };
        Ok(Stmt { kind, line })
    }

    /// What follows `if`: the condition, the block, and any `else`.
    fn if_rest(&mut self) -> Result<StmtKind, String> {
        let cond = self.expr(0)?;
        let then_body = self.block()?;
        let mut else_body = Vec::new();
        if *self.peek() == Tok::Else {
            self.next();
            if *self.peek() == Tok::If {
                let line = self.line();
                self.next();
                else_body.push(Stmt { kind: self.if_rest()?, line });
            } else {
                else_body = self.block()?;
            }
        }
        Ok(StmtKind::If(cond, then_body, else_body))
    }

    /// An expression whose operators all bind more tightly than `min`.
    fn expr(&mut self, min: u8) -> Result<Expr, String> {
        let mut left = self.unary()?;
        while let Some((op, precedence)) = binary_op(self.peek()) {
            if precedence <= min {
                break;
            }
            self.next();
            let right = self.expr(precedence)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        let line = self.line();
        match self.next() {
            Tok::Int(value) => Ok(Expr::Int(value)),
            Tok::True => Ok(Expr::Bool(true)),
            Tok::False => Ok(Expr::Bool(false)),
            Tok::Minus => Ok(Expr::Unary(UnOp::Neg, Box::new(self.unary()?))),
            Tok::Not => Ok(Expr::Unary(UnOp::Not, Box::new(self.unary()?))),
            Tok::LParen => {
                let inner = self.expr(0)?;
                self.expect(Tok::RParen)?;
                Ok(inner)
            }
            Tok::Ident(name) if *self.peek() == Tok::LParen => {
                self.next();
                let mut args = Vec::new();
                while *self.peek() != Tok::RParen {
                    args.push(self.expr(0)?);
                    if *self.peek() != Tok::RParen {
                        self.expect(Tok::Comma)?;
                    }
                }
                self.expect(Tok::RParen)?;
                Ok(Expr::Call(name, args))
            }
            Tok::Ident(name) => Ok(Expr::Var(name)),
            other => Err(format!("line {line}: expected an expression, found {other:?}")),
        }
    }
}
```

Two details worth noticing:

- `statement` decides between an assignment (`x = …;`) and an expression statement (`print(x);`) by looking **two** tokens ahead: a name followed by `=`.
- `else if` is parsed as an `else` block holding a single `if` statement, so later stages only ever see `if`/`else`.

> **In GaiaRusted** the parser is split by construct across `src/syntax/parser/`, handles generics, patterns, closures, attributes and macros, and has to deal with Rust's famous ambiguities: in `if x == S { … }`, is `S { … }` a struct literal or is `{` the start of the block? (The block — the parser carries a flag that forbids struct literals in conditions.)

---

### Step 4: The checker

The parser accepts `let x = true + 1;` and `print(y);` with no `y` anywhere. Syntax is fine; meaning is not. The checker finds these errors before any code is generated:

- **Names**: every variable must be declared in an enclosing scope, every function must exist.
- **Types**: arithmetic takes `i64`, `if` and `while` take `bool`, arguments match parameters, `return` matches the function's return type.
- **Mutability**: assigning to a variable not declared `mut` is an error — the first of Rust's safety rules.
- **Returns**: a function that returns a value must do so on every path.

Scopes are a stack of maps: entering a block pushes one, leaving it pops it, and looking up a name searches from the innermost outwards. That stack also gives you **shadowing** (`let x = x * 2;`) for free.

Checking happens in two passes. The first records every function's signature, so that a function can call one defined later in the file (or itself). The second checks each body.

```rust
struct Signature {
    params: Vec<Type>,
    ret: Type,
}

struct Local {
    ty: Type,
    mutable: bool,
}

struct Checker<'a> {
    signatures: &'a HashMap<String, Signature>,
    scopes: Vec<HashMap<String, Local>>,
    ret: Type,
}

fn expect(want: Type, found: Type, line: usize) -> Result<(), String> {
    if want == found {
        Ok(())
    } else {
        Err(format!("line {line}: mismatched types: expected `{}`, found `{}`", want.name(), found.name()))
    }
}

/// Whether every path through `body` ends in `return`.
fn always_returns(body: &[Stmt]) -> bool {
    body.iter().any(|stmt| match &stmt.kind {
        StmtKind::Return(_) => true,
        StmtKind::If(_, then_body, else_body) => always_returns(then_body) && always_returns(else_body),
        _ => false,
    })
}

fn check_program(functions: &[Function]) -> Result<(), String> {
    let mut signatures = HashMap::new();
    signatures.insert("print".to_string(), Signature { params: vec![Type::Int], ret: Type::Unit });
    for f in functions {
        if f.params.len() > 6 {
            return Err(format!("line {}: `{}` has more than 6 parameters", f.line, f.name));
        }
        let signature = Signature { params: f.params.iter().map(|(_, ty)| *ty).collect(), ret: f.ret };
        if signatures.insert(f.name.clone(), signature).is_some() {
            return Err(format!("line {}: `{}` is defined more than once", f.line, f.name));
        }
    }
    match signatures.get("main") {
        Some(main) if main.params.is_empty() && main.ret == Type::Unit => {}
        _ => return Err("the program needs a `fn main()`".to_string()),
    }
    for f in functions {
        let mut checker = Checker { signatures: &signatures, scopes: vec![HashMap::new()], ret: f.ret };
        for (name, ty) in &f.params {
            checker.scopes[0].insert(name.clone(), Local { ty: *ty, mutable: false });
        }
        checker.block(&f.body)?;
        if f.ret != Type::Unit && !always_returns(&f.body) {
            return Err(format!("line {}: `{}` can reach its end without returning a value", f.line, f.name));
        }
    }
    Ok(())
}

impl Checker<'_> {
    fn lookup(&self, name: &str, line: usize) -> Result<&Local, String> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .ok_or_else(|| format!("line {line}: cannot find value `{name}` in this scope"))
    }

    fn block(&mut self, body: &[Stmt]) -> Result<(), String> {
        self.scopes.push(HashMap::new());
        for stmt in body {
            self.stmt(stmt)?;
        }
        self.scopes.pop();
        Ok(())
    }

    fn stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        let line = stmt.line;
        match &stmt.kind {
            StmtKind::Let { name, mutable, value } => {
                let ty = self.expr(value, line)?;
                if ty == Type::Unit {
                    return Err(format!("line {line}: `{name}` would hold `()`"));
                }
                self.scopes.last_mut().unwrap().insert(name.clone(), Local { ty, mutable: *mutable });
            }
            StmtKind::Assign(name, value) => {
                let ty = self.expr(value, line)?;
                let local = self.lookup(name, line)?;
                if !local.mutable {
                    return Err(format!("line {line}: cannot assign twice to immutable variable `{name}`"));
                }
                expect(local.ty, ty, line)?;
            }
            StmtKind::If(cond, then_body, else_body) => {
                expect(Type::Bool, self.expr(cond, line)?, line)?;
                self.block(then_body)?;
                self.block(else_body)?;
            }
            StmtKind::While(cond, body) => {
                expect(Type::Bool, self.expr(cond, line)?, line)?;
                self.block(body)?;
            }
            StmtKind::Return(value) => {
                let ty = match value {
                    Some(value) => self.expr(value, line)?,
                    None => Type::Unit,
                };
                expect(self.ret, ty, line)?;
            }
            StmtKind::Expr(expr) => {
                self.expr(expr, line)?;
            }
        }
        Ok(())
    }

    fn expr(&self, expr: &Expr, line: usize) -> Result<Type, String> {
        Ok(match expr {
            Expr::Int(_) => Type::Int,
            Expr::Bool(_) => Type::Bool,
            Expr::Var(name) => self.lookup(name, line)?.ty,
            Expr::Unary(UnOp::Neg, operand) => {
                expect(Type::Int, self.expr(operand, line)?, line)?;
                Type::Int
            }
            Expr::Unary(UnOp::Not, operand) => {
                expect(Type::Bool, self.expr(operand, line)?, line)?;
                Type::Bool
            }
            Expr::Binary(op, left, right) => {
                let (left, right) = (self.expr(left, line)?, self.expr(right, line)?);
                let (operands, result) = match op {
                    BinOp::And | BinOp::Or => (Type::Bool, Type::Bool),
                    BinOp::Eq | BinOp::Ne => (left, Type::Bool),
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => (Type::Int, Type::Bool),
                    _ => (Type::Int, Type::Int),
                };
                if operands == Type::Unit {
                    return Err(format!("line {line}: `()` cannot be compared"));
                }
                expect(operands, left, line)?;
                expect(operands, right, line)?;
                result
            }
            Expr::Call(name, args) => {
                let signature = self
                    .signatures
                    .get(name)
                    .ok_or_else(|| format!("line {line}: cannot find function `{name}`"))?;
                if args.len() != signature.params.len() {
                    let count = |n: usize| format!("{n} argument{}", if n == 1 { "" } else { "s" });
                    let (want, found) = (count(signature.params.len()), count(args.len()));
                    return Err(format!("line {line}: `{name}` expects {want}, found {found}"));
                }
                for (arg, want) in args.iter().zip(&signature.params) {
                    expect(*want, self.expr(arg, line)?, line)?;
                }
                signature.ret
            }
        })
    }
}
```

`always_returns` is a simplified version of Rust's rule: a list of statements always returns if one of them is a `return`, or an `if` whose two branches both always return.

> **In GaiaRusted** this stage is `src/sema/`, by far the largest part of the compiler. Types are not just `i64` and `bool` but structs, enums, references, generics and trait objects; types are *inferred* by unification instead of read from declarations; and every method call needs a search through trait impls. But the shape is the same: walk the tree with a stack of scopes, compute a type for every expression, compare it with what is expected.

---

### Step 5: The intermediate representation

You could generate assembly straight from the syntax tree. It works for a while, then everything gets hard: optimising a tree is awkward, and `while`, `if`, `&&` and `return` each need their own special code. Instead, compilers translate the tree into an **intermediate representation** that is closer to the machine but still independent of it.

mini's IR, like GaiaRusted's, is a **control-flow graph**:

- A function is a list of **basic blocks**.
- A block is a list of simple **instructions** — each computes one value into a **temp** — and ends with one **terminator** that says where control goes next: `goto`, a two-way branch, or `return`.
- There is no nesting. `total = total + i` is `t4 = t1 Add t2; t1 = t4`.

Every variable gets a temp, and so does every intermediate result. Lowering a `while` loop creates three blocks:

```
        ┌──────────────┐
        │ head:        │◀────────┐
        │ t3 = i <= n  │         │
        │ if t3 …      │         │
        └──┬────────┬──┘         │
     true  │        │ false      │
           ▼        ▼            │
   ┌────────────┐  ┌──────┐      │
   │ body:      │  │ exit │      │
   │ …          │──┼──────┼──────┘
   │ goto head  │  └──────┘
   └────────────┘
```

`&&` and `||` become control flow too, because `a && b` must not evaluate `b` when `a` is false — `n != 0 && 10 / n > 1` must not divide by zero. And after a `return`, lowering continues in a fresh block nothing jumps to; the optimiser deletes it.

```rust
/// A virtual register: every value the program computes lives in one.
type Temp = usize;
type BlockId = usize;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Value {
    Const(i64),
    Temp(Temp),
}

#[derive(Debug)]
enum Inst {
    Copy(Temp, Value),
    Unary(Temp, UnOp, Value),
    /// Never `&&` or `||`: those are control flow.
    Binary(Temp, BinOp, Value, Value),
    Call(Temp, String, Vec<Value>),
}

#[derive(Debug)]
enum Terminator {
    Goto(BlockId),
    Branch(Value, BlockId, BlockId),
    Return(Value),
}

struct Block {
    insts: Vec<Inst>,
    /// `None` only while the block is being filled.
    term: Option<Terminator>,
}

struct IrFunction {
    name: String,
    params: Vec<Temp>,
    temps: usize,
    blocks: Vec<Block>,
}

struct Lowerer {
    blocks: Vec<Block>,
    current: BlockId,
    temps: usize,
    scopes: Vec<HashMap<String, Temp>>,
}

fn lower_function(f: &Function) -> IrFunction {
    let mut lowerer = Lowerer { blocks: Vec::new(), current: 0, temps: 0, scopes: vec![HashMap::new()] };
    lowerer.new_block();
    let params = f
        .params
        .iter()
        .map(|(name, _)| {
            let temp = lowerer.new_temp();
            lowerer.scopes[0].insert(name.clone(), temp);
            temp
        })
        .collect();
    lowerer.block(&f.body);
    lowerer.terminate(Terminator::Return(Value::Const(0)));
    IrFunction { name: f.name.clone(), params, temps: lowerer.temps, blocks: lowerer.blocks }
}

impl Lowerer {
    fn new_temp(&mut self) -> Temp {
        self.temps += 1;
        self.temps - 1
    }

    fn new_block(&mut self) -> BlockId {
        self.blocks.push(Block { insts: Vec::new(), term: None });
        self.blocks.len() - 1
    }

    fn emit(&mut self, inst: Inst) {
        self.blocks[self.current].insts.push(inst);
    }

    fn terminate(&mut self, term: Terminator) {
        self.blocks[self.current].term = Some(term);
    }

    fn var(&self, name: &str) -> Temp {
        *self.scopes.iter().rev().find_map(|scope| scope.get(name)).expect("the checker resolved every name")
    }

    fn block(&mut self, body: &[Stmt]) {
        self.scopes.push(HashMap::new());
        for stmt in body {
            self.stmt(stmt);
        }
        self.scopes.pop();
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { name, value, .. } => {
                let value = self.expr(value);
                let temp = self.new_temp();
                self.emit(Inst::Copy(temp, value));
                self.scopes.last_mut().unwrap().insert(name.clone(), temp);
            }
            StmtKind::Assign(name, value) => {
                let value = self.expr(value);
                let temp = self.var(name);
                self.emit(Inst::Copy(temp, value));
            }
            StmtKind::If(cond, then_body, else_body) => {
                let cond = self.expr(cond);
                let (then_block, else_block, join) = (self.new_block(), self.new_block(), self.new_block());
                self.terminate(Terminator::Branch(cond, then_block, else_block));
                self.current = then_block;
                self.block(then_body);
                self.terminate(Terminator::Goto(join));
                self.current = else_block;
                self.block(else_body);
                self.terminate(Terminator::Goto(join));
                self.current = join;
            }
            StmtKind::While(cond, body) => {
                let (head, body_block, exit) = (self.new_block(), self.new_block(), self.new_block());
                self.terminate(Terminator::Goto(head));
                self.current = head;
                let cond = self.expr(cond);
                self.terminate(Terminator::Branch(cond, body_block, exit));
                self.current = body_block;
                self.block(body);
                self.terminate(Terminator::Goto(head));
                self.current = exit;
            }
            StmtKind::Return(value) => {
                let value = match value {
                    Some(value) => self.expr(value),
                    None => Value::Const(0),
                };
                self.terminate(Terminator::Return(value));
                // Whatever follows a `return` is unreachable; give it a block
                // of its own, which the optimiser will remove.
                self.current = self.new_block();
            }
            StmtKind::Expr(expr) => {
                self.expr(expr);
            }
        }
    }

    fn expr(&mut self, expr: &Expr) -> Value {
        match expr {
            Expr::Int(value) => Value::Const(*value),
            Expr::Bool(value) => Value::Const(*value as i64),
            Expr::Var(name) => Value::Temp(self.var(name)),
            Expr::Unary(op, operand) => {
                let operand = self.expr(operand);
                let temp = self.new_temp();
                self.emit(Inst::Unary(temp, *op, operand));
                Value::Temp(temp)
            }
            Expr::Binary(BinOp::And, left, right) => self.short_circuit(left, right, true),
            Expr::Binary(BinOp::Or, left, right) => self.short_circuit(left, right, false),
            Expr::Binary(op, left, right) => {
                let (left, right) = (self.expr(left), self.expr(right));
                let temp = self.new_temp();
                self.emit(Inst::Binary(temp, *op, left, right));
                Value::Temp(temp)
            }
            Expr::Call(name, args) => {
                let args = args.iter().map(|arg| self.expr(arg)).collect();
                let temp = self.new_temp();
                self.emit(Inst::Call(temp, name.clone(), args));
                Value::Temp(temp)
            }
        }
    }

    /// `a && b` evaluates `b` only when `a` is true, `a || b` only when it
    /// is false.
    fn short_circuit(&mut self, left: &Expr, right: &Expr, is_and: bool) -> Value {
        let result = self.new_temp();
        let left = self.expr(left);
        self.emit(Inst::Copy(result, left));
        let (rhs, join) = (self.new_block(), self.new_block());
        let (if_true, if_false) = if is_and { (rhs, join) } else { (join, rhs) };
        self.terminate(Terminator::Branch(Value::Temp(result), if_true, if_false));
        self.current = rhs;
        let right = self.expr(right);
        self.emit(Inst::Copy(result, right));
        self.terminate(Terminator::Goto(join));
        self.current = join;
        Value::Temp(result)
    }
}

fn dump(f: &IrFunction) -> String {
    let show = |value: &Value| match value {
        Value::Const(n) => n.to_string(),
        Value::Temp(t) => format!("t{t}"),
    };
    let params: Vec<String> = f.params.iter().map(|t| format!("t{t}")).collect();
    let mut out = format!("fn {}({}) {{\n", f.name, params.join(", "));
    for (id, block) in f.blocks.iter().enumerate() {
        writeln!(out, "  b{id}:").unwrap();
        for inst in &block.insts {
            let line = match inst {
                Inst::Copy(dest, value) => format!("t{dest} = {}", show(value)),
                Inst::Unary(dest, op, value) => format!("t{dest} = {op:?} {}", show(value)),
                Inst::Binary(dest, op, left, right) => format!("t{dest} = {} {op:?} {}", show(left), show(right)),
                Inst::Call(dest, name, args) => {
                    let args: Vec<String> = args.iter().map(show).collect();
                    format!("t{dest} = call {name}({})", args.join(", "))
                }
            };
            writeln!(out, "    {line}").unwrap();
        }
        let term = match &block.term {
            Some(Terminator::Goto(target)) => format!("goto b{target}"),
            Some(Terminator::Branch(cond, then_block, else_block)) => {
                format!("if {} goto b{then_block} else b{else_block}", show(cond))
            }
            Some(Terminator::Return(value)) => format!("return {}", show(value)),
            None => "(unfinished)".to_string(),
        };
        writeln!(out, "    {term}").unwrap();
    }
    out.push_str("}\n");
    out
}
```

`dump` prints the IR, so you can see what the compiler made of your program — GaiaRusted's `--emit-ir`.

---

### Step 6: The optimiser

The IR is easy to improve. mini does two classic optimisations:

**Constant folding.** Inside a block, remember which temps hold a known constant; replace their uses by the constant, and compute operations whose operands are all constant. `let x = 2 * 3 + 4;` becomes `x = 10` at compile time, and a branch on a known condition becomes a plain `goto`. The knowledge is reset at the start of each block: a temp assigned in a loop can hold different values each time control arrives.

Two operations are deliberately **not** folded: division by zero and `i64::MIN / -1`. Folding them would make the compiler crash instead of the program; the run-time behaviour belongs to the program.

**Removing unreachable blocks.** After folding, some blocks can no longer be reached — the dead block after a `return`, the `else` of an `if false`. A walk from the first block finds every reachable one; the rest are deleted and the blocks renumbered.

```rust
fn optimize(f: &mut IrFunction) {
    fold_constants(f);
    remove_unreachable_blocks(f);
}

/// The value of `left op right`, if it can be known now. Division by zero
/// and `i64::MIN / -1` are left for the program to fail on at run time.
fn fold(op: BinOp, left: i64, right: i64) -> Option<i64> {
    Some(match op {
        BinOp::Add => left.wrapping_add(right),
        BinOp::Sub => left.wrapping_sub(right),
        BinOp::Mul => left.wrapping_mul(right),
        BinOp::Div => left.checked_div(right)?,
        BinOp::Rem => left.checked_rem(right)?,
        BinOp::Eq => (left == right) as i64,
        BinOp::Ne => (left != right) as i64,
        BinOp::Lt => (left < right) as i64,
        BinOp::Le => (left <= right) as i64,
        BinOp::Gt => (left > right) as i64,
        BinOp::Ge => (left >= right) as i64,
        BinOp::And | BinOp::Or => unreachable!("lowered to branches"),
    })
}

/// Within each block, replace temps known to hold a constant by that
/// constant, and compute operations whose operands are all constants.
fn fold_constants(f: &mut IrFunction) {
    for block in &mut f.blocks {
        let mut known: HashMap<Temp, i64> = HashMap::new();
        let resolve = |value: Value, known: &HashMap<Temp, i64>| match value {
            Value::Temp(t) => known.get(&t).map_or(value, |&n| Value::Const(n)),
            constant => constant,
        };
        for inst in &mut block.insts {
            let folded = match inst {
                Inst::Copy(dest, value) => {
                    *value = resolve(*value, &known);
                    (*dest, *value)
                }
                Inst::Unary(dest, op, value) => {
                    *value = resolve(*value, &known);
                    match (*op, *value) {
                        (UnOp::Neg, Value::Const(n)) => (*dest, Value::Const(n.wrapping_neg())),
                        (UnOp::Not, Value::Const(n)) => (*dest, Value::Const(n ^ 1)),
                        _ => (*dest, Value::Temp(*dest)),
                    }
                }
                Inst::Binary(dest, op, left, right) => {
                    *left = resolve(*left, &known);
                    *right = resolve(*right, &known);
                    match (*left, *right) {
                        (Value::Const(a), Value::Const(b)) => match fold(*op, a, b) {
                            Some(n) => (*dest, Value::Const(n)),
                            None => (*dest, Value::Temp(*dest)),
                        },
                        _ => (*dest, Value::Temp(*dest)),
                    }
                }
                Inst::Call(dest, _, args) => {
                    for arg in args.iter_mut() {
                        *arg = resolve(*arg, &known);
                    }
                    (*dest, Value::Temp(*dest))
                }
            };
            match folded {
                (dest, Value::Const(n)) => {
                    *inst = Inst::Copy(dest, Value::Const(n));
                    known.insert(dest, n);
                }
                (dest, _) => {
                    known.remove(&dest);
                }
            }
        }
        block.term = match block.term.take() {
            Some(Terminator::Branch(cond, then_block, else_block)) => match resolve(cond, &known) {
                Value::Const(0) => Some(Terminator::Goto(else_block)),
                Value::Const(_) => Some(Terminator::Goto(then_block)),
                cond => Some(Terminator::Branch(cond, then_block, else_block)),
            },
            Some(Terminator::Return(value)) => Some(Terminator::Return(resolve(value, &known))),
            other => other,
        };
    }
}

fn successors(term: &Terminator) -> Vec<BlockId> {
    match term {
        Terminator::Goto(target) => vec![*target],
        Terminator::Branch(_, then_block, else_block) => vec![*then_block, *else_block],
        Terminator::Return(_) => Vec::new(),
    }
}

/// Drop the blocks control can never reach, and number the rest afresh.
fn remove_unreachable_blocks(f: &mut IrFunction) {
    let mut reachable = HashSet::from([0]);
    let mut work = vec![0];
    while let Some(id) = work.pop() {
        for next in successors(f.blocks[id].term.as_ref().unwrap()) {
            if reachable.insert(next) {
                work.push(next);
            }
        }
    }
    let mut renumber = HashMap::new();
    for id in 0..f.blocks.len() {
        if reachable.contains(&id) {
            renumber.insert(id, renumber.len());
        }
    }
    let blocks = std::mem::take(&mut f.blocks);
    for (id, mut block) in blocks.into_iter().enumerate() {
        if !reachable.contains(&id) {
            continue;
        }
        block.term = block.term.map(|term| match term {
            Terminator::Goto(target) => Terminator::Goto(renumber[&target]),
            Terminator::Branch(cond, a, b) => Terminator::Branch(cond, renumber[&a], renumber[&b]),
            other => other,
        });
        f.blocks.push(block);
    }
}
```

Here is the IR mini prints with `--ir` for this program:

```rust
fn sum_to(n: i64) -> i64 {
    let mut total = 0;
    let mut i = 1;
    while i <= n {
        total = total + i;
        i = i + 1;
    }
    return total;
}

fn main() {
    let x = 2 * 3 + 4;
    if x > 5 && sum_to(x) > 50 {
        print(sum_to(x));
    }
}
```

```
fn sum_to(t0) {
  b0:
    t1 = 0
    t2 = 1
    goto b1
  b1:
    t3 = t2 Le t0
    if t3 goto b2 else b3
  b2:
    t4 = t1 Add t2
    t1 = t4
    t5 = t2 Add 1
    t2 = t5
    goto b1
  b3:
    return t1
}
fn main() {
  b0:
    t0 = 6
    t1 = 10
    t2 = 10
    t4 = 1
    t3 = 1
    goto b1
  b1:
    t5 = call sum_to(t2)
    t6 = t5 Gt 50
    t3 = t6
    goto b2
  b2:
    if t3 goto b3 else b4
  b3:
    t7 = call sum_to(t2)
    t8 = call print(t7)
    goto b5
  b4:
    goto b5
  b5:
    return 0
}
```

In `main`, `2 * 3 + 4` was computed at compile time (`t1 = 10`), `x > 5` became `t4 = 1`, and the branch on it became `goto b1`: the compiler proved the first half of `&&` true and removed the test. The empty blocks `b4` and `b5` show what a next optimisation could do: merge a block into its only successor (GaiaRusted's `simplify_cfg`).

---

### Step 7: The backend

The backend turns IR into x86-64 assembly. mini uses the simplest strategy that works: **every temp lives in its own 8-byte slot on the stack**, and each instruction loads its operands into registers, computes, and stores the result back. It is slow but always correct — it is exactly what GaiaRusted does at `-O0`.

What you need to know about x86-64 on Linux (the **System V ABI**, the convention C uses, which lets mini call `printf`):

- The first six integer arguments arrive in `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9`; the result goes back in `rax`.
- `rbp` points to the current function's frame. Slot *n* is at `[rbp - 8*(n+1)]`.
- `rsp` must be a multiple of 16 at every `call`. `call` pushes 8 bytes and `push rbp` another 8, so making the frame a multiple of 16 keeps the alignment right.
- Comparisons set flags; `setl al` turns "less than" into a 0 or 1 in the low byte of `rax`, and `movzx` clears the rest.
- `idiv` divides `rdx:rax` by its operand; `cqo` sign-extends `rax` into `rdx` first. The quotient lands in `rax`, the remainder in `rdx`.

The program's functions are named `mini_<name>`, so that a mini function called `printf` or `exit` cannot collide with the C library. The C library calls `main`; mini's `main` calls `mini_main` and returns 0. `print` is a small hand-written function that calls `printf("%ld\n", x)`.

```rust
const ARG_REGS: [&str; 6] = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];

fn slot(temp: Temp) -> String {
    format!("qword ptr [rbp - {}]", 8 * (temp + 1))
}

fn load(out: &mut String, reg: &str, value: &Value) {
    match value {
        Value::Const(n) => writeln!(out, "    mov {reg}, {n}").unwrap(),
        Value::Temp(t) => writeln!(out, "    mov {reg}, {}", slot(*t)).unwrap(),
    }
}

fn emit_program(functions: &[IrFunction]) -> String {
    let mut out = String::from(".intel_syntax noprefix\n.text\n\n");
    // The C library calls `main`; ours calls the program's `main`.
    out.push_str(".globl main\nmain:\n    sub rsp, 8\n    call mini_main\n    add rsp, 8\n    xor eax, eax\n    ret\n\n");
    out.push_str("mini_print:\n    push rbp\n    mov rbp, rsp\n    mov rsi, rdi\n");
    out.push_str("    lea rdi, [rip + .Lformat]\n    xor eax, eax\n    call printf\n    pop rbp\n    ret\n\n");
    for f in functions {
        emit_function(&mut out, f);
    }
    out.push_str(".section .rodata\n.Lformat:\n    .string \"%ld\\n\"\n");
    out.push_str(".section .note.GNU-stack,\"\",@progbits\n");
    out
}

fn emit_function(out: &mut String, f: &IrFunction) {
    // One 8-byte slot per temp, rounded up to keep `rsp` 16-byte aligned.
    let frame = (8 * f.temps + 15) / 16 * 16;
    writeln!(out, "mini_{}:\n    push rbp\n    mov rbp, rsp\n    sub rsp, {frame}", f.name).unwrap();
    for (param, reg) in f.params.iter().zip(ARG_REGS) {
        writeln!(out, "    mov {}, {reg}", slot(*param)).unwrap();
    }
    for (id, block) in f.blocks.iter().enumerate() {
        writeln!(out, ".L{}_b{id}:", f.name).unwrap();
        for inst in &block.insts {
            emit_inst(out, inst);
        }
        match block.term.as_ref().unwrap() {
            Terminator::Goto(target) => writeln!(out, "    jmp .L{}_b{target}", f.name).unwrap(),
            Terminator::Branch(cond, then_block, else_block) => {
                load(out, "rax", cond);
                writeln!(out, "    cmp rax, 0").unwrap();
                writeln!(out, "    jne .L{}_b{then_block}", f.name).unwrap();
                writeln!(out, "    jmp .L{}_b{else_block}", f.name).unwrap();
            }
            Terminator::Return(value) => {
                load(out, "rax", value);
                writeln!(out, "    leave\n    ret").unwrap();
            }
        }
    }
    out.push('\n');
}

fn emit_inst(out: &mut String, inst: &Inst) {
    match inst {
        Inst::Copy(dest, value) => {
            load(out, "rax", value);
            writeln!(out, "    mov {}, rax", slot(*dest)).unwrap();
        }
        Inst::Unary(dest, op, value) => {
            load(out, "rax", value);
            let code = match op {
                UnOp::Neg => "neg rax",
                UnOp::Not => "xor rax, 1",
            };
            writeln!(out, "    {code}\n    mov {}, rax", slot(*dest)).unwrap();
        }
        Inst::Binary(dest, op, left, right) => {
            load(out, "rax", left);
            load(out, "rcx", right);
            let code = match op {
                BinOp::Add => "add rax, rcx",
                BinOp::Sub => "sub rax, rcx",
                BinOp::Mul => "imul rax, rcx",
                BinOp::Div => "cqo\n    idiv rcx",
                BinOp::Rem => "cqo\n    idiv rcx\n    mov rax, rdx",
                BinOp::Eq => "cmp rax, rcx\n    sete al\n    movzx eax, al",
                BinOp::Ne => "cmp rax, rcx\n    setne al\n    movzx eax, al",
                BinOp::Lt => "cmp rax, rcx\n    setl al\n    movzx eax, al",
                BinOp::Le => "cmp rax, rcx\n    setle al\n    movzx eax, al",
                BinOp::Gt => "cmp rax, rcx\n    setg al\n    movzx eax, al",
                BinOp::Ge => "cmp rax, rcx\n    setge al\n    movzx eax, al",
                BinOp::And | BinOp::Or => unreachable!("lowered to branches"),
            };
            writeln!(out, "    {code}\n    mov {}, rax", slot(*dest)).unwrap();
        }
        Inst::Call(dest, name, args) => {
            for (arg, reg) in args.iter().zip(ARG_REGS) {
                load(out, reg, arg);
            }
            writeln!(out, "    call mini_{name}\n    mov {}, rax", slot(*dest)).unwrap();
        }
    }
}
```

This is the code mini generates for `sum_to`:

```asm
mini_sum_to:
    push rbp
    mov rbp, rsp
    sub rsp, 48
    mov qword ptr [rbp - 8], rdi
.Lsum_to_b0:
    mov rax, 0
    mov qword ptr [rbp - 16], rax
    mov rax, 1
    mov qword ptr [rbp - 24], rax
    jmp .Lsum_to_b1
.Lsum_to_b1:
    mov rax, qword ptr [rbp - 24]
    mov rcx, qword ptr [rbp - 8]
    cmp rax, rcx
    setle al
    movzx eax, al
    mov qword ptr [rbp - 32], rax
    mov rax, qword ptr [rbp - 32]
    cmp rax, 0
    jne .Lsum_to_b2
    jmp .Lsum_to_b3
.Lsum_to_b2:
    mov rax, qword ptr [rbp - 16]
    mov rcx, qword ptr [rbp - 24]
    add rax, rcx
    mov qword ptr [rbp - 40], rax
    mov rax, qword ptr [rbp - 40]
    mov qword ptr [rbp - 16], rax
    mov rax, qword ptr [rbp - 24]
    mov rcx, 1
    add rax, rcx
    mov qword ptr [rbp - 48], rax
    mov rax, qword ptr [rbp - 48]
    mov qword ptr [rbp - 24], rax
    jmp .Lsum_to_b1
.Lsum_to_b3:
    mov rax, qword ptr [rbp - 16]
    leave
    ret
```

Compare it with GaiaRusted's optimised loop for the same function in the [walkthrough](#7-assembly-optimised---format-asm): four instructions, because a register allocator keeps `total` and `i` in registers instead of slots. That is the single biggest improvement a backend can make.

---

### Step 8: The driver

The driver connects the stages, writes the assembly next to the source file, and asks `gcc` to assemble and link it against the C library. `gcc` accepts `.s` files directly. `-no-pie` lets the generated code use absolute addresses for its own functions.

```rust
fn compile(source: &str, show_ir: bool) -> Result<String, String> {
    let tokens = lex(source)?;
    let functions = Parser { tokens, pos: 0 }.program()?;
    check_program(&functions)?;
    let mut program: Vec<IrFunction> = functions.iter().map(lower_function).collect();
    for f in &mut program {
        optimize(f);
        if show_ir {
            print!("{}", dump(f));
        }
    }
    Ok(emit_program(&program))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let show_ir = args.iter().any(|arg| arg == "--ir");
    let Some(path) = args.iter().find(|arg| !arg.starts_with("--")) else {
        eprintln!("usage: mini [--ir] FILE.rs");
        std::process::exit(2);
    };
    let path = std::path::Path::new(path);
    let source = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("error: cannot read `{}`: {e}", path.display());
        std::process::exit(1);
    });
    let assembly = compile(&source, show_ir).unwrap_or_else(|message| {
        eprintln!("error: {message}");
        std::process::exit(1);
    });
    let assembly_path = path.with_extension("s");
    std::fs::write(&assembly_path, assembly).expect("the assembly can be written");
    let linked = std::process::Command::new("gcc")
        .arg("-no-pie")
        .arg(&assembly_path)
        .arg("-o")
        .arg(path.with_extension(""))
        .status()
        .expect("gcc runs");
    std::process::exit(if linked.success() { 0 } else { 1 });
}
```

Build and try it:

```bash
cargo build --release
./target/release/mini fib.rs && ./fib
./target/release/mini --ir fib.rs     # print the IR too
```

And the errors:

```
error: line 3: cannot assign twice to immutable variable `x`
error: line 3: mismatched types: expected `bool`, found `i64`
error: line 1: cannot find value `y` in this scope
error: line 1: `f` can reach its end without returning a value
error: line 1: `print` expects 1 argument, found 2 arguments
error: the program needs a `fn main()`
```

---

### Step 9: Test against rustc

Here is the trick GaiaRusted is built on, and you can use it from day one: **every mini program is a Rust program**. Give rustc a definition of `print`, and rustc's build of the program tells you exactly what mini's build must print. You never write an expected output by hand.

```bash
#!/bin/sh
# check.sh FILE.rs — does mini's build print what rustc's build prints?
name=$(basename "$1" .rs)
./target/release/mini "$1" || exit 1
{ echo 'fn print(x: i64) { println!("{}", x); }'; cat "$1"; } > /tmp/oracle.rs
rustc -O -A warnings /tmp/oracle.rs -o /tmp/oracle || exit 1
if [ "$(./"${1%.rs}")" = "$(/tmp/oracle)" ]; then echo "ok $name"; else echo "FAIL $name"; fi
```

A program worth testing with exercises precedence, short-circuiting, recursion and negative division (Rust rounds towards zero, and so does `idiv`):

```rust
fn is_prime(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d = d + 1;
    }
    return true;
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        return a;
    }
    return gcd(b, a % b);
}

fn main() {
    let mut n = 0;
    let mut count = 0;
    while count < 10 {
        if is_prime(n) && n != 2 || n == 4 {
            print(n);
            count = count + 1;
        }
        n = n + 1;
    }
    print(gcd(1071, 462));
    print(-7 / 2);
    print(-7 % 3);
    print(1 - 2 - 3);
}
```

Collect such programs in a folder and run them all after every change. That folder is your conformance suite; GaiaRusted's has 64 programs and runs each at two optimisation levels.

One difference you will find is overflow. With `fn inc(x: i64) -> i64 { return x + 1; }`, `print(inc(9223372036854775807))` wraps around to `-9223372036854775808` in mini and in rustc's optimised build, but panics in rustc's debug build. (Write `9223372036854775807 + 1` directly and rustc refuses to compile it at all: it checks constant arithmetic for overflow. mini's constant folder quietly wraps — a check you could add.) Which run-time behaviour to choose is a design decision; GaiaRusted chose the optimised build's.

---

### Where to go from here

mini is about a thousand lines. GaiaRusted is about 40,000, with its standard library. Each of these extensions takes mini one step towards it, roughly in order of difficulty:

| Extension | What it teaches | Where GaiaRusted does it |
|---|---|---|
| Better errors: keep the column too, print the source line with a `^` under the problem | Spans and diagnostics | `src/syntax/span.rs`, `diagnostic.rs` |
| Report all errors instead of stopping at the first | Error recovery | — (planned) |
| Tail expressions (`fn f() -> i64 { 1 }`) and `if` as an expression | Expression-oriented lowering | `src/ir/build/expr.rs` |
| Merge a block into its only successor; drop `goto` to the next block | CFG simplification | `src/ir/opt/simplify_cfg.rs` |
| Copy propagation and dead-code elimination across blocks | Data-flow analysis, liveness | `src/ir/opt/propagate.rs`, `dce.rs` |
| Keep temps in registers | Liveness, interference, graph colouring | `src/x64/regalloc.rs` |
| Inline small functions | Call graphs, callee-first order | `src/ir/opt/inline.rs` |
| `u8`, `i32`, `f64` | Sized loads and stores, sign extension, SSE registers | `src/x64/function.rs`, `abi.rs` |
| Structs and tuples | Memory layout, places with field projections | `src/ir/layout.rs` |
| References `&` and `&mut` | Addresses, `Deref` projections | `src/ir/mod.rs` |
| Strings and `Vec` | A standard library written in your own language, on top of `malloc` | `library/` |
| Type inference (`let x = Vec::new();`) | Unification | `src/sema/infer.rs` |
| Generics | Monomorphisation | `src/ir/build/mod.rs` |
| Traits and methods | Impl search, vtables | `src/sema/context.rs`, `ir/build/call.rs` |
| `Drop` | Drop flags and drop glue | `src/ir/build/drop.rs` |
| Moves and borrows | The borrow checker | — (planned for 1.2.0) |

Two pieces of advice from building GaiaRusted:

1. **Keep a reference to compare against from the first day.** A compiler that "seems to work" hides bugs that a byte-for-byte comparison with rustc finds in seconds.
2. **Keep every stage simple, and let the next stage clean up.** mini's lowering leaves empty blocks and redundant copies everywhere, and that is fine: lowering stays obvious, and the optimiser removes the waste. GaiaRusted's IR builder works the same way.

Happy hacking! 🦀

---

## Glossary

| Term | Meaning |
|---|---|
| **AST** | Abstract syntax tree: the program as written, organised by the parser |
| **THIR** | Typed high-level IR: the tree after type checking, with every type and every implicit operation explicit |
| **IR** | GaiaRusted's intermediate representation: control-flow graphs over typed locals |
| **CFG** | Control-flow graph: basic blocks connected by jumps |
| **Basic block** | Straight-line statements with one terminator at the end |
| **Place** | A local, or a path into one (field, element, dereference, variant) |
| **Monomorphisation** | Compiling a generic function once per set of concrete type arguments |
| **Unification** | Making two types equal by binding inference variables |
| **Drop flag** | A hidden boolean saying whether a local still owns a value that must be dropped |
| **Drop glue** | The generated function that drops a value of a given type |
| **Niche** | Values a type's bytes can hold but the type never uses, available to store an enum's tag |
| **Fat pointer** | A two-word pointer: data plus length (slices, `str`) or vtable (`dyn Trait`) |
| **Vtable** | Constant table of a concrete type's drop glue, size, alignment and trait methods |
| **SROA** | Scalar replacement of aggregates: splitting a struct local into one local per field |
| **LICM** | Loop-invariant code motion: hoisting unchanging computations out of loops |
| **CSE** | Common-subexpression elimination: reusing a value already computed |
| **System V ABI** | The calling convention of C on x86-64 Linux |
| **Callee-saved** | Registers a function must restore before returning |
| **Conformance** | Producing the same output as rustc's build of the same program |
| **Bless** | Record rustc's output as a conformance program's expectation (`BLESS=1`) |

---

## Resources

- **The Rust Reference** — https://doc.rust-lang.org/reference/ — the language GaiaRusted implements
- **The Rustonomicon** — https://doc.rust-lang.org/nomicon/ — layout, drop order, unsafe
- **rustc dev guide** — https://rustc-dev-guide.rust-lang.org/ — how rustc does type checking, method lookup and MIR
- **System V AMD64 ABI** — the calling convention and type classification
- **Engineering a Compiler** (Cooper & Torczon) — data-flow analysis, optimisation, register allocation
- **Types and Programming Languages** (Pierce) — type inference and unification
- **MY GOD FORSAKEN BRAIN** imma overdose and lie on the ground iqmaxxing

---

*GaiaRusted v1.1.8 — a Rust compiler in Rust* 🦀
