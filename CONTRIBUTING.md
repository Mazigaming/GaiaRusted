# Contributing to GaiaRusted

Thank you for your interest in contributing! This guide will help you get started with development.

## Table of Contents

- [Setup](#setup)
- [Code Style](#code-style)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Architecture Guide](#architecture-guide)
- [Debugging](#debugging)
- [Areas for Contribution](#areas-for-contribution)

---

## Setup

### Prerequisites

- Rust 1.70+ ([Install rustup](https://rustup.rs/))
- GNU binutils (for assembly/linking)
- Git

### Local Development

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/GaiaRusted.git
cd GaiaRusted/gaiarusted

# Build the project
cargo build

# Run tests
cargo test --lib --tests

# Build documentation
cargo doc --open
```

### Development Environment

Recommended IDE setup:
- **VS Code** with rust-analyzer
- **IntelliJ IDEA** with Rust plugin
- **Vim/Neovim** with rust.vim

---

## Code Style

### Formatting

We follow Rust's standard formatting conventions:

```bash
# Format your code
cargo fmt

# Check formatting (CI requirement)
cargo fmt -- --check
```

### Linting

All code must pass clippy checks:

```bash
# Run clippy
cargo clippy -- -D warnings

# Fix common issues automatically
cargo fix --allow-dirty
```

### Documentation

Document public APIs with doc comments:

```rust
/// Brief description of the item
/// 
/// More detailed explanation if needed.
/// 
/// # Examples
/// 
/// ```
/// let x = my_function();
/// ```
pub fn my_function() -> Result<i32, Box<dyn std::error::Error>> {
    Ok(42)
}
```

### Naming Conventions

- **Types**: `PascalCase` (e.g., `HirExpression`, `BorrowChecker`)
- **Functions**: `snake_case` (e.g., `compile_files`, `check_types`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_DEPTH`)
- **Variables**: `snake_case` (e.g., `source_file`, `result`)
- **Modules**: `snake_case` (e.g., `lexer`, `type_checker`)

---

## Making Changes

### Branching Strategy

Create a feature branch for your work:

```bash
# Create and switch to feature branch
git checkout -b feature/description-of-change

# Keep feature branch updated with main
git rebase origin/main
```

### Branch Naming

Use descriptive branch names:
- `feature/add-pattern-matching` - New feature
- `fix/borrow-checker-crash` - Bug fix
- `docs/api-reference` - Documentation
- `refactor/simplify-codegen` - Code improvement
- `test/add-mir-tests` - Test additions

### Commit Messages

Write clear, concise commit messages:

```
Short description (50 chars max)

Detailed explanation of changes (if needed)
- Explain the "why" not just the "what"
- Reference issue numbers: Fixes #123
- Keep lines under 72 characters
```

Good examples:
- `Add pattern matching to type checker (Fixes #45)`
- `Fix off-by-one error in register allocation`
- `Refactor MIR builder for clarity`

### Code Changes

Keep changes focused and testable:

1. **Make one logical change per commit**
2. **Add tests for new functionality**
3. **Update documentation**
4. **Don't mix formatting and logic changes**

---

## Testing

### Writing Tests

Place tests in the appropriate location:

**Unit tests** (in source modules):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_functionality() {
        let result = my_function(42);
        assert_eq!(result, expected_value);
    }
}
```

**Integration tests** (in `tests/` directory):
```rust
use gaiarusted::compiler;

#[test]
fn test_end_to_end_compilation() {
    let config = CompilationConfig::new();
    let result = compile_files(&config);
    assert!(result.is_ok());
}
```

### Running Tests

```bash
# Run all tests
cargo test --lib --tests

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run with specific thread count
cargo test -- --test-threads=1
```

### Test Coverage

Aim for high coverage in:
- **Lexer**: Token recognition, edge cases
- **Parser**: AST construction, error recovery
- **Type Checker**: Type inference, mismatch detection
- **Borrow Checker**: Ownership rules, move semantics
- **MIR**: Control flow, basic blocks
- **Code Generation**: Instruction selection, register allocation

### Expected Test Results

After running `cargo test --lib --tests`:
- All unit tests should pass
- All integration tests should pass
- No warnings in test code

---

## Submitting Changes

### Pre-submission Checklist

- [ ] Code compiles without warnings
- [ ] All tests pass: `cargo test --lib --tests`
- [ ] Code formatted: `cargo fmt`
- [ ] Clippy passes: `cargo clippy -- -D warnings`
- [ ] Documentation updated
- [ ] Commit messages are clear
- [ ] Branch rebased on latest main

### Creating a Pull Request

1. Push your feature branch to your fork
2. Go to GitHub and create a Pull Request
3. Fill out the PR template:

```markdown
## Description
Brief description of changes

## Related Issues
Fixes #123

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
How to test these changes

## Checklist
- [ ] Tests pass
- [ ] Documentation updated
- [ ] No new warnings
```

### PR Review Process

- Maintainers will review within 48 hours
- Address feedback and push updates
- Once approved, maintainers will merge
- Thank you for contributing!

---

## Architecture Guide

### Compilation Phases

Understanding the compilation flow helps with development:

```
Phase 1: Lexer (src/lexer/)
  Input: Source code string
  Output: Token stream
  Key types: Token, TokenKind

Phase 2: Parser (src/parser/)
  Input: Token stream
  Output: Abstract Syntax Tree (AST)
  Key types: Item, Statement, Expression

Phase 3: Lowering (src/lowering/)
  Input: AST
  Output: High-Level IR (HIR)
  Key types: HirItem, HirStatement, HirExpression

Phase 4: Type Checking (src/typechecker/)
  Input: HIR
  Output: Type-annotated HIR
  Key types: HirType, TypeEnv

Phase 5: Borrow Checking (src/borrowchecker/)
  Input: Type-checked HIR
  Output: Memory-safe HIR
  Key types: OwnershipState, BorrowEnv

Phase 6-7: MIR (src/mir/)
  Input: Validated HIR
  Output: Control Flow Graph
  Key types: BasicBlock, Terminator, Place

Phase 8: Code Generation (src/codegen/)
  Input: MIR
  Output: Machine code
  Key types: Instruction, Register
```

### Navigation Quick Reference

**Working on lexical analysis?**
- Start: `src/lexer/mod.rs`
- Token definitions: `src/lexer/token.rs`

**Working on parsing?**
- Start: `src/parser/mod.rs`
- AST definitions: `src/parser/ast.rs`

**Working on type checking?**
- Start: `src/typechecker/mod.rs`
- Related: `src/lowering/mod.rs` (HirType definition)

**Working on borrow checking?**
- Start: `src/borrowchecker/mod.rs`
- Related: Look at BorrowEnv and OwnershipState

**Working on code generation?**
- Start: `src/codegen/mod.rs`
- Object files: `src/codegen/object.rs`
- Related: `src/mir/mod.rs` (MIR input)

**Integration?**
- Start: `src/compiler.rs` (orchestrator)
- Config: `src/config.rs`

### Key Algorithms

**Type Inference** (src/typechecker/mod.rs)
- Hindley-Milner algorithm
- Constraint collection and unification
- See `fn unify()` and `fn infer()`

**Borrow Checking** (src/borrowchecker/mod.rs)
- Ownership state tracking per binding
- Move/borrow validation at each use
- See `fn check_statement()` and `BorrowEnv`

**Code Generation** (src/codegen/mod.rs)
- Tree-to-code selection
- Register allocation (simplified)
- See `fn generate_instruction()`

---

## Debugging

### Using Cargo Debug Output

```bash
# Verbose output
cargo build -vv

# With backtrace for panics
RUST_BACKTRACE=1 cargo run -- input.rs

# Full backtrace
RUST_BACKTRACE=full cargo run -- input.rs
```

### Adding Debug Output

Use `eprintln!()` for debug output (goes to stderr):

```rust
eprintln!("Debug: {:?}", some_value);
```

Better yet, use the `dbg!()` macro:

```rust
let x = dbg!(some_computation());  // Prints and returns value
```

### Using a Debugger

With VS Code + rust-analyzer:
1. Install CodeLLDB extension
2. Create `.vscode/launch.json`:
```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug",
            "cargo": {
                "args": ["build", "--bin=gaiarusted", "--release"]
            }
        }
    ]
}
```
3. Press F5 to debug

### Profiling

```bash
# Time compilation
time cargo build --release

# Analyze code with clippy
cargo clippy --release -- -D warnings

# Generate profile-guided optimizations
cargo build --release --profile-guided
```

---

## Areas for Contribution

### High Priority 🔴

These areas would have the most impact:

1. **Pattern Matching** (src/parser/ + src/lowering/)
   - Add match expression parsing
   - Implement exhaustiveness checking
   - Generate MIR for patterns

2. **Better Error Messages**
   - Add line/column tracking to AST nodes
   - Implement error context spans
   - Create helpful error messages with suggestions

3. **Optimization Passes** (src/mir/)
   - Dead code elimination
   - Constant folding
   - Common subexpression elimination

### Medium Priority 🟡

Good contributions that round out functionality:

1. **More Built-in Functions**
   - String manipulation
   - Array operations
   - Math functions

2. **Test Coverage**
   - Add tests for error cases
   - Edge case testing
   - Integration test scenarios

3. **Performance**
   - Profile and optimize hot paths
   - Improve compilation speed
   - Reduce memory usage

### Low Priority 🟢

Nice-to-have improvements:

1. **Documentation**
   - Better inline comments
   - Architecture documentation
   - API documentation examples

2. **Code Cleanup**
   - Refactor duplicated code
   - Simplify complex functions
   - Improve module organization

3. **Examples**
   - Add more example programs
   - Create tutorial walkthroughs
   - Document language features

---

## Questions?

- Open an issue with the `question` label
- Check existing issues for answers
- Join discussions for design decisions
- Comment on PRs for specific code questions

---

**Happy contributing! 🚀**

Made with 🦀 Rust | Building the future of compiler design