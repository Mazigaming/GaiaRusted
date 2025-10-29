A complete Rust compiler implementation built from scratch in pure Rust with zero external dependencies. Converts Rust source code to multiple output formats including Assembly, Object files, Executables, and Libraries.

**v0.0.1** | [Setup Guide](#building-from-source) | [Contributing](https://github.com/Mazigaming/GaiaRusted/blob/main/CONTRIBUTING.md) | [Architecture](#architecture)

* * *

What It Does
------------

Compiles custom Rust-like language through complete compilation pipeline:

*   **Lexer** - Tokenization and scanning
*   **Parser** - Syntax analysis and AST construction
*   **Type Checking** - Type inference and validation
*   **Lowering** - High-Level IR generation with syntactic sugar removal
*   **Borrow Checking** - Memory safety verification
*   **MIR Generation** - Mid-Level IR and control flow graph construction
*   **Code Generation** - Machine code and multiple output formats

Supports multiple output formats:

*   **Assembly** - Complete x86-64 disassembly (.s)
*   **Object** - ELF object files for linking (.o)
*   **Executable** - Standalone binary executables
*   **Bash Script** - Shell script wrappers (.sh)
*   **Library** - Static libraries for reuse (.a)

Quick Start
-----------

### Installation

```bash
# Clone repository
git clone https://github.com/Mazigaming/GaiaRusted.git
cd GaiaRusted/gaiarusted

# Build release
cargo build --release

# Run tests
cargo test --lib --tests
```

### Usage

```bash
# Compile a Rust file to assembly
./target/release/gaiarusted input.rs -o output.s --format assembly

# Compile to executable
./target/release/gaiarusted input.rs -o program --format executable

# Compile to object file
./target/release/gaiarusted input.rs -o program.o --format object
```

### Library Usage

Use GaiaRusted as a library in your Rust projects:

```rust
use gaiarusted::{CompilationConfig, OutputFormat, compile_files};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = CompilationConfig::new();
    config.output_format = OutputFormat::Executable;
    config.output_path = PathBuf::from("my_program");
    config.verbose = true;
    
    let result = compile_files(&config)?;
    println!("✓ Compilation successful: {:?}", result.output_path);
    
    Ok(())
}
```

* * *

Code Structure
--------------

### Core Components

```
src/
├── lib.rs                       # Public library exports
├── main.rs                      # Binary entry point
├── compiler.rs                  # Main compilation orchestrator
│
├── lexer/                       # Phase 1: Tokenization
│   ├── mod.rs                   # Lexer implementation
│   └── token.rs                 # Token definitions
│
├── parser/                      # Phase 2: Parsing
│   ├── mod.rs                   # Parser implementation
│   └── ast.rs                   # AST node definitions
│
├── lowering/                    # Phase 3: AST Lowering
│   └── mod.rs                   # HIR generation
│
├── typechecker/                 # Phase 4: Type Checking
│   └── mod.rs                   # Type inference & validation
│
├── borrowchecker/               # Phase 5: Borrow Checking
│   └── mod.rs                   # Ownership & borrow analysis
│
├── mir/                         # Phase 6 & 7: MIR & Optimization
│   └── mod.rs                   # Control flow graph construction
│
├── codegen/                     # Phase 8: Code Generation
│   ├── mod.rs                   # x86-64 code generation
│   └── object.rs                # ELF object file creation
│
└── config.rs                    # Configuration management
```

### Compilation Pipeline

```
Source Code (.rs)
    ↓
Lexer ──────────────→ Tokens
    ↓
Parser ─────────────→ Abstract Syntax Tree (AST)
    ↓
Lowering ───────────→ High-Level IR (HIR)
    ↓
Type Checker ───────→ Type-Checked HIR
    ↓
Borrow Checker ─────→ Memory-Safe HIR
    ↓
MIR Builder ────────→ Control Flow Graph (CFG)
    ↓
Code Generator ─────→ x86-64 Machine Code
    ↓
Object Writer ──────→ Output Format (ASM/OBJ/EXE/SH/LIB)
```

* * *

Key Features
------------

### Lexer (Phase 1)
*   ✅ Multi-character token recognition
*   ✅ String and numeric literal parsing
*   ✅ Keyword identification
*   ✅ Comment handling

### Parser (Phase 2)
*   ✅ Recursive descent parsing
*   ✅ Expression precedence handling
*   ✅ Function and struct definitions
*   ✅ Control flow constructs (if/else, loops)

### Lowering (Phase 3)
*   ✅ Syntactic sugar removal (for loops → while)
*   ✅ Pattern normalization
*   ✅ Explicit type annotations
*   ✅ Basic macro expansion

### Type Checking (Phase 4)
*   ✅ Type inference using Hindley-Milner algorithm
*   ✅ Type unification
*   ✅ Mismatch detection
*   ✅ Function signature validation

### Borrow Checking (Phase 5)
*   ✅ Ownership tracking
*   ✅ Move semantics enforcement
*   ✅ Borrow validation (immutable & mutable)
*   ✅ Use-after-move detection

### MIR (Phase 6 & 7)
*   ✅ Control flow graph construction
*   ✅ Basic block generation
*   ✅ SSA-like form (each place assigned once)
*   ✅ Terminator-based control flow

### Code Generation (Phase 8)
*   ✅ x86-64 instruction selection
*   ✅ Register allocation (simplified)
*   ✅ Stack frame management
*   ✅ Call convention compliance (System V AMD64 ABI)

### Output Formats
*   ✅ Intel syntax x86-64 assembly (.s)
*   ✅ ELF object files (.o)
*   ✅ Standalone executables
*   ✅ Bash script wrappers (.sh)
*   ✅ Static libraries (.a)

* * *

Building from Source
--------------------

### Requirements

*   **Rust:** Latest stable (install from [rustup.rs](https://rustup.rs/))
*   **Assembler:** `as` (GNU binutils)
*   **Linker:** `ld` or system linker

### Build Options

```bash
# Development build (faster compilation)
cargo build

# Release build (optimized binary)
cargo build --release

# Run tests
cargo test --lib --tests

# Generate documentation
cargo doc --open

# Check code quality
cargo fmt && cargo clippy -- -D warnings
```

### Platform Support

| Platform | Status | Requirements |
| --- | --- | --- |
| Linux (x86-64) | ✅ Stable | gcc, binutils |
| macOS (x86-64) | ⚠️ Partial | Xcode command line tools |
| Windows (x86-64) | ⚠️ Partial | MSVC or MinGW |

* * *

Architecture Overview
---------------------

### Phase Progression

1. **Lexer** (src/lexer/mod.rs)
   - Input: Raw source code string
   - Output: Vector of tokens
   - Algorithm: Scanning with lookahead

2. **Parser** (src/parser/mod.rs)
   - Input: Token stream
   - Output: Abstract Syntax Tree (AST)
   - Algorithm: Recursive descent parser with precedence climbing

3. **Lowering** (src/lowering/mod.rs)
   - Input: AST
   - Output: Higher-Level IR (HIR) with sugar removed
   - Desugaring: for loops → while loops

4. **Type Checker** (src/typechecker/mod.rs)
   - Input: HIR
   - Output: Type-annotated HIR + constraints
   - Algorithm: Hindley-Milner type inference

5. **Borrow Checker** (src/borrowchecker/mod.rs)
   - Input: Type-checked HIR
   - Output: Memory-safe HIR + borrow checks
   - Verification: Ownership rules enforcement

6. **MIR Builder** (src/mir/mod.rs)
   - Input: Validated HIR
   - Output: Control Flow Graph (CFG)
   - Construction: Basic block generation with explicit control flow

7. **Code Generator** (src/codegen/mod.rs)
   - Input: MIR/CFG
   - Output: x86-64 assembly or object code
   - Target: System V AMD64 ABI

8. **Object Writer** (src/codegen/object.rs)
   - Input: Machine code
   - Output: ELF object file or executable
   - Format: ELF64 with standard sections

### Data Structures

**AST Nodes** (parser/ast.rs)
- Expression, Statement, Item types
- Direct representation of source syntax

**HIR** (lowering/mod.rs)
- HirExpression, HirStatement, HirItem
- Normalized form without syntactic sugar

**MIR** (mir/mod.rs)
- BasicBlock, Terminator, Place, Operand
- Control flow explicit, SSA-like

**Type System** (typechecker/mod.rs)
- Type inference with unification
- Support for primitives and user-defined types

**Borrow State** (borrowchecker/mod.rs)
- OwnershipState: Owned, Moved, BorrowedImmutable, BorrowedMutable
- Track binding state through program execution

* * *

Examples
--------

### Example 1: Simple Function

```rust
fn main() {
    let x = 42;
    let y = x + 1;
    println!("Result: {}", y);
}
```

Compilation: `gaiarusted example.rs -o example --format executable`

### Example 2: Control Flow

```rust
fn fibonacci(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    let a = fibonacci(n - 1);
    let b = fibonacci(n - 2);
    a + b
}
```

### Example 3: Loops

```rust
fn sum_array() {
    let arr = [1, 2, 3, 4, 5];
    let sum = 0;
    for i in arr {
        sum = sum + i;
    }
}
```

### Example 4: Structs

```rust
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let p = Point { x: 10, y: 20 };
}
```

* * *

Performance
-----------

| Metric | Value |
| --- | --- |
| Compile Time (Debug) | ~0.5s typical |
| Compile Time (Release) | ~1.2s typical |
| Binary Size | ~5-10MB (release) |
| Memory Usage | ~50-100MB typical |
| x86-64 Code Generation Speed | ~1MB/s |

**Benchmarks:** Results vary by system and code complexity.

* * *

Testing
-------

### Run All Tests

```bash
cargo test --lib --tests
```

### Test Organization

*   **Unit Tests** - In individual modules (src/*/mod.rs)
*   **Integration Tests** - In tests/ directory
*   **Test Categories:**
    - `config_tests.rs` - Configuration API
    - `borrowchecker_tests.rs` - Ownership/borrow checking
    - `mir_tests.rs` - MIR representation
    - `codegen_tests.rs` - Code generation
    - `integration_tests.rs` - End-to-end compilation

### Current Test Coverage

- Lexer tests: ✅ 3 passing
- Parser tests: ✅ Framework in place
- Type checker tests: ✅ 3 passing
- Lowering tests: ✅ 2 passing
- Integration tests: ✅ 30+ passing

* * *

Supported Language Features
---------------------------

### ✅ Implemented

*   Primitive types: i32, i64, f64, bool, str
*   Variables and assignments
*   Arithmetic operators: +, -, *, /, %
*   Comparison operators: ==, !=, <, <=, >, >=
*   Logical operators: &&, ||, !
*   Control flow: if/else, while, for loops
*   Functions with parameters and return types
*   Struct definitions and literals
*   Array literals and indexing
*   Function calls
*   Comments

### 🚧 In Progress

*   Pattern matching
*   Trait definitions
*   Generics
*   Advanced type inference
*   Lifetimes

### 📋 Planned

*   Modules and visibility
*   Error handling (Option/Result)
*   Closures
*   Async/await
*   Macro system
*   Standard library

* * *

Roadmap
-------

### ✅ v0.0.1 (Complete)

*   Full compilation pipeline
*   Multiple output formats
*   Borrow checking
*   Type inference
*   MIR generation
*   Basic code generation

### 🚧 v0.0.2 (In Progress)

*   Optimization passes
*   Better error messages
*   Additional built-in functions
*   Performance improvements

### 📋 v0.1.0 (Planned)

*   Pattern matching support
*   Trait system
*   Generic type parameters
*   Module system

### 📋 v1.0.0 (Vision)

*   Full Rust compatibility subset
*   Standard library bindings
*   Package manager integration
*   Production-ready compiler

* * *

License
-------

MIT License - See [LICENSE](https://github.com/Mazigaming/GaiaRusted/blob/main/LICENSE)

**Educational Use** - This compiler is designed for learning compiler construction and understanding Rust internals. It implements a subset of Rust for educational purposes.

* * *

Quick Links
-----------

**Documentation**

*   📖 [Contributing Guide](https://github.com/Mazigaming/GaiaRusted/blob/main/CONTRIBUTING.md)
*   📚 [Full Architecture](docs/ARCHITECTURE.md)

**Resources**

*   🔧 [Build Instructions](#building-from-source)
*   🧪 [Test Guide](#testing)
*   💡 [Examples](#examples)

* * *

**Made with 🦀 Rust** | Built for compiler education and development