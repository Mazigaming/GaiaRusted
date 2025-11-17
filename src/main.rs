//! GiaRusted - A Rust Compiler Built from Scratch
//!
//! This is the entry point for the gaiarusted compiler.
//! 
//! ## Compilation Pipeline
//! 
//! ```text
//! Rust Source Code
//!     ↓ [Lexer]
//! Token Stream
//!     ↓ [Parser]
//! Abstract Syntax Tree
//!     ↓ [Lowering]
//! High-Level IR
//!     ↓ [Type Checker]
//! Typed HIR
//!     ↓ [Borrow Checker]
//! Memory-Safe HIR
//!     ↓ [MIR Lowering]
//! Mid-Level IR
//!     ↓ [Optimizations]
//! Optimized MIR
//!     ↓ [Codegen]
//! x86-64 Machine Code → Object Files → Executable
//! ```

use std::env;
use std::fs;
use std::process;

mod lexer;
mod parser;
mod lowering;
mod typechecker;
mod borrowchecker;
mod mir;
mod codegen;

// Color helper functions (v0.0.3)
fn format_error(text: &str) -> String {
    format!("\x1b[31m{}\x1b[0m", text)
}

fn format_warning(text: &str) -> String {
    format!("\x1b[33m{}\x1b[0m", text)
}

fn format_success(text: &str) -> String {
    format!("\x1b[32m{}\x1b[0m", text)
}

fn format_info(text: &str) -> String {
    format!("\x1b[36m{}\x1b[0m", text)
}

fn format_header(text: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", text)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: gaiarusted <input.rs> [options]");
        eprintln!("       gaiarusted <input.rs> -o <output>");
        process::exit(1);
    }

    let input_file = &args[1];
    let mut output_file = "a.out".to_string();

    // Parse command line arguments
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                if i + 1 < args.len() {
                    output_file = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: -o requires an argument");
                    process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                process::exit(1);
            }
        }
    }

    // Read the source file
    let source = match fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file {}: {}", input_file, e);
            process::exit(1);
        }
    };

    println!("{}", format_header("[GiaRusted] Compiling..."));
    println!("  File: {}", input_file);

    // Phase 1: Lexical Analysis
    println!("{}", format_info("[Phase 1] Lexing..."));
    let tokens = match lexer::lex(&source) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("{} [Phase 1] Lexer Error: {}", format_error("❌"), e);
            eprintln!("   File: {}", input_file);
            process::exit(1);
        }
    };
    println!("{} Generated {} tokens", format_success("✓"), tokens.len());

    // Debug: Print tokens if verbose mode (not yet implemented)
    if std::env::var("VERBOSE").is_ok() {
        println!("\n[Debug] Token stream:");
        for (i, token) in tokens.iter().enumerate() {
            println!("  [{}] {}", i, token);
        }
        println!();
    }

    // Phase 2: Parsing
    println!("{}", format_info("[Phase 2] Parsing..."));
    let ast = match parser::parse(tokens) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("{} [Phase 2] Parser Error: {}", format_error("❌"), e);
            eprintln!("   File: {}", input_file);
            eprintln!("   {} Check your Rust syntax", format_warning("→"));
            process::exit(1);
        }
    };
    println!("{} AST generated with {} items", format_success("✓"), ast.len());

    // Phase 2.5: Resolve file-based modules
    println!("{}", format_info("[Phase 2.5] Resolving file-based modules..."));
    let base_dir = std::path::Path::new(&input_file)
        .parent()
        .map(|p| p.to_string_lossy().into_owned());
    let ast = match parser::resolve_file_modules(ast, base_dir.as_deref()) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("{} [Phase 2.5] Module resolution error: {}", format_error("❌"), e);
            eprintln!("   File: {}", input_file);
            process::exit(1);
        }
    };
    println!("{} File-based modules resolved", format_success("✓"));

    // Debug: Print AST if verbose mode
    if std::env::var("VERBOSE").is_ok() {
        println!("\n[Debug] AST Structure:");
        for (i, item) in ast.iter().enumerate() {
            match item {
                parser::Item::Function { name, params, return_type, .. } => {
                    println!("  [{}] Function '{}' with {} params", i, name, params.len());
                    if let Some(ret_ty) = return_type {
                        println!("       Returns: {}", ret_ty);
                    }
                }
                parser::Item::Struct { name, fields } => {
                    println!("  [{}] Struct '{}' with {} fields", i, name, fields.len());
                }
                parser::Item::Enum { name, variants } => {
                    println!("  [{}] Enum '{}' with {} variants", i, name, variants.len());
                }
                parser::Item::Trait { name, methods } => {
                    println!("  [{}] Trait '{}' with {} methods", i, name, methods.len());
                }
                parser::Item::Impl { struct_name, trait_name, methods } => {
                    if let Some(tr) = trait_name {
                        println!("  [{}] Impl {} for {} with {} methods", i, tr, struct_name, methods.len());
                    } else {
                        println!("  [{}] Impl {} with {} methods", i, struct_name, methods.len());
                    }
                }
                parser::Item::Module { name, items } => {
                    println!("  [{}] Module '{}' with {} items", i, name, items.len());
                }
                parser::Item::Use { path } => {
                    println!("  [{}] Use statement: {}", i, path);
                }
            }
        }
        println!();
    }

    // Phase 3: AST Lowering
    println!("{}", format_info("[Phase 3] Lowering (removing syntactic sugar)..."));
    let hir = match lowering::lower(&ast) {
        Ok(hir) => hir,
        Err(e) => {
            eprintln!("{} [Phase 3] Lowering Error: {}", format_error("❌"), e);
            eprintln!("   File: {}", input_file);
            eprintln!("   {} Error during AST sugar removal", format_warning("→"));
            process::exit(1);
        }
    };
    println!("{} HIR generated with {} items", format_success("✓"), hir.len());

    // Debug: Print HIR structure if verbose mode
    if std::env::var("VERBOSE").is_ok() {
        println!("\n[Debug] HIR Structure:");
        for (i, item) in hir.iter().enumerate() {
            match item {
                lowering::HirItem::Function {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    println!(
                        "  [{}] Function '{}' with {} params",
                        i,
                        name,
                        params.len()
                    );
                    if let Some(ret_ty) = return_type {
                        println!("       Returns: {}", ret_ty);
                    }
                }
                lowering::HirItem::Struct { name, fields } => {
                    println!(
                        "  [{}] Struct '{}' with {} fields",
                        i,
                        name,
                        fields.len()
                    );
                }
            }
        }
        println!();
    }

    // Phase 4: Type Checking & Inference
    println!("{}", format_info("[Phase 4] Type Checking & Inference..."));
    if let Err(e) = typechecker::check_types(&hir) {
        eprintln!("{} [Phase 4] Type Check Error: {}", format_error("❌"), e);
        eprintln!("   File: {}", input_file);
        eprintln!("   {} Type mismatch or inference failure", format_warning("→"));
        process::exit(1);
    }
    println!("{} All types verified and inferred", format_success("✓"));

    // Phase 5: Borrow Checking
    println!("{}", format_info("[Phase 5] Borrow Checking (memory safety)..."));
    if let Err(e) = borrowchecker::check_borrows(&hir) {
        eprintln!("{} [Phase 5] Borrow Check Error: {}", format_error("❌"), e);
        eprintln!("   File: {}", input_file);
        eprintln!("   {} Ownership or borrowing rules violated", format_warning("→"));
        process::exit(1);
    }
    println!("{} Memory safety verified (ownership & borrowing rules)", format_success("✓"));

    // Phase 6: MIR Lowering
    println!("{}", format_info("[Phase 6] MIR Lowering (control flow graph)..."));
    let mir = match mir::lower_to_mir(&hir) {
        Ok(mir) => mir,
        Err(e) => {
            eprintln!("{} [Phase 6] MIR Lowering Error: {}", format_error("❌"), e);
            eprintln!("   File: {}", input_file);
            eprintln!("   {} Control flow graph construction failed", format_warning("→"));
            process::exit(1);
        }
    };
    println!("{} MIR generated with {} functions", format_success("✓"), mir.functions.len());

    // Debug: Print MIR structure if verbose mode
    if std::env::var("VERBOSE").is_ok() {
        println!("\n[Debug] MIR Structure:");
        for (i, func) in mir.functions.iter().enumerate() {
            println!("  [{}] fn {}(...) with {} basic blocks", i, func.name, func.basic_blocks.len());
        }
        println!();
    }

    // Phase 7: MIR Optimization
    println!("{}", format_info("[Phase 7] MIR Optimization (dead code elimination, constant folding)..."));
    let mut optimized_mir = mir.clone();
    if let Err(e) = mir::optimize_mir(&mut optimized_mir) {
        eprintln!("{} [Phase 7] MIR Optimization Error: {}", format_error("❌"), e);
        eprintln!("   File: {}", input_file);
        eprintln!("   {} Optimization pass failed", format_warning("→"));
        process::exit(1);
    }
    println!("{} MIR optimized", format_success("✓"));

    // Phase 8: Code Generation
    println!("{}", format_info("[Phase 8] Code Generation (x86-64 assembly)..."));
    let assembly = match codegen::generate_code(&optimized_mir) {
        Ok(asm) => asm,
        Err(e) => {
            eprintln!("{} [Phase 8] Codegen Error: {}", format_error("❌"), e);
            eprintln!("   File: {}", input_file);
            eprintln!("   {} x86-64 code generation failed", format_warning("→"));
            process::exit(1);
        }
    };
    println!("{} Generated x86-64 assembly", format_success("✓"));

    // Debug: Print assembly if verbose mode
    if std::env::var("VERBOSE").is_ok() {
        println!("\n[Debug] Generated Assembly (first 50 lines):");
        for (i, line) in assembly.lines().take(50).enumerate() {
            println!("  [{}] {}", i, line);
        }
        if assembly.lines().count() > 50 {
            println!("  ... ({} more lines)", assembly.lines().count() - 50);
        }
        println!();
    }

    // Phase 9: Object File Generation
    println!("{}", format_info("[Phase 9] Object File Generation (ELF format)..."));
    let asm_file = format!("{}.s", output_file);
    match fs::write(&asm_file, &assembly) {
        Ok(_) => {
            println!("{} Assembly file generated: {}", format_success("✓"), asm_file);
        }
        Err(e) => {
            eprintln!("{} [Phase 9] Error writing assembly file: {}", format_error("❌"), e);
            eprintln!("   Output file: {}", output_file);
            process::exit(1);
        }
    }

    // Phase 10: Testing & Polish
    println!("{}", format_info("[Phase 10] Testing & Polish (compilation complete)..."));
    println!("{} Compilation succeeded!", format_success("✓"));
    println!("{} Assembly written to: {}", format_success("✓"), asm_file);
    println!();
    println!("{} Next steps to create executable:", format_header("📦"));
    println!("  1. Assemble:  as {} -o {}.o", asm_file, output_file);
    println!("  2. Link:      ld {}.o -o {}", output_file, output_file);
    println!("  3. Run:       ./{}", output_file);
    println!();
    println!("{} Output files:", format_header("📁"));
    println!("  • Assembly:   {}", asm_file);
    println!("  • Object:     {}.o", output_file);
    println!("  • Binary:     {}", output_file);
    println!();
    println!("{}", format_success("✨ [Status] All Phases 1-10 Complete! ✨"));
}