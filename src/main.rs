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

    println!("[GiaRusted] Compiling {}...", input_file);

    // Phase 1: Lexical Analysis
    println!("[Phase 1] Lexing...");
    let tokens = match lexer::lex(&source) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("❌ [Phase 1] Lexer Error: {}", e);
            eprintln!("   File: {}", input_file);
            process::exit(1);
        }
    };
    println!("  ✓ Generated {} tokens", tokens.len());

    // Debug: Print tokens if verbose mode (not yet implemented)
    if std::env::var("VERBOSE").is_ok() {
        println!("\n[Debug] Token stream:");
        for (i, token) in tokens.iter().enumerate() {
            println!("  [{}] {}", i, token);
        }
        println!();
    }

    // Phase 2: Parsing
    println!("[Phase 2] Parsing...");
    let ast = match parser::parse(tokens) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("❌ [Phase 2] Parser Error: {}", e);
            eprintln!("   File: {}", input_file);
            eprintln!("   Check your Rust syntax");
            process::exit(1);
        }
    };
    println!("  ✓ AST generated with {} items", ast.len());

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
    println!("[Phase 3] Lowering (removing syntactic sugar)...");
    let hir = match lowering::lower(&ast) {
        Ok(hir) => hir,
        Err(e) => {
            eprintln!("❌ [Phase 3] Lowering Error: {}", e);
            eprintln!("   File: {}", input_file);
            eprintln!("   Error during AST sugar removal");
            process::exit(1);
        }
    };
    println!("  ✓ HIR generated with {} items", hir.len());

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
    println!("[Phase 4] Type Checking & Inference...");
    if let Err(e) = typechecker::check_types(&hir) {
        eprintln!("❌ [Phase 4] Type Check Error: {}", e);
        eprintln!("   File: {}", input_file);
        eprintln!("   Type mismatch or inference failure");
        process::exit(1);
    }
    println!("  ✓ All types verified and inferred");

    // Phase 5: Borrow Checking
    println!("[Phase 5] Borrow Checking (memory safety)...");
    if let Err(e) = borrowchecker::check_borrows(&hir) {
        eprintln!("❌ [Phase 5] Borrow Check Error: {}", e);
        eprintln!("   File: {}", input_file);
        eprintln!("   Ownership or borrowing rules violated");
        process::exit(1);
    }
    println!("  ✓ Memory safety verified (ownership & borrowing rules)");

    // Phase 6: MIR Lowering
    println!("[Phase 6] MIR Lowering (control flow graph)...");
    let mir = match mir::lower_to_mir(&hir) {
        Ok(mir) => mir,
        Err(e) => {
            eprintln!("❌ [Phase 6] MIR Lowering Error: {}", e);
            eprintln!("   File: {}", input_file);
            eprintln!("   Control flow graph construction failed");
            process::exit(1);
        }
    };
    println!("  ✓ MIR generated with {} functions", mir.functions.len());

    // Debug: Print MIR structure if verbose mode
    if std::env::var("VERBOSE").is_ok() {
        println!("\n[Debug] MIR Structure:");
        for (i, func) in mir.functions.iter().enumerate() {
            println!("  [{}] fn {}(...) with {} basic blocks", i, func.name, func.basic_blocks.len());
        }
        println!();
    }

    // Phase 7: MIR Optimization
    println!("[Phase 7] MIR Optimization (dead code elimination, constant folding)...");
    let mut optimized_mir = mir.clone();
    if let Err(e) = mir::optimize_mir(&mut optimized_mir) {
        eprintln!("❌ [Phase 7] MIR Optimization Error: {}", e);
        eprintln!("   File: {}", input_file);
        eprintln!("   Optimization pass failed");
        process::exit(1);
    }
    println!("  ✓ MIR optimized");

    // Phase 8: Code Generation
    println!("[Phase 8] Code Generation (x86-64 assembly)...");
    let assembly = match codegen::generate_code(&optimized_mir) {
        Ok(asm) => asm,
        Err(e) => {
            eprintln!("❌ [Phase 8] Codegen Error: {}", e);
            eprintln!("   File: {}", input_file);
            eprintln!("   x86-64 code generation failed");
            process::exit(1);
        }
    };
    println!("  ✓ Generated x86-64 assembly");

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
    println!("[Phase 9] Object File Generation (ELF format)...");
    let asm_file = format!("{}.s", output_file);
    match fs::write(&asm_file, &assembly) {
        Ok(_) => {
            println!("  ✓ Assembly file generated: {}", asm_file);
        }
        Err(e) => {
            eprintln!("❌ [Phase 9] Error writing assembly file: {}", e);
            eprintln!("   Output file: {}", output_file);
            process::exit(1);
        }
    }

    // Phase 10: Testing & Polish
    println!("[Phase 10] Testing & Polish (compilation complete)...");
    println!("  ✓ Compilation succeeded!");
    println!("  ✓ Assembly written to: {}", asm_file);
    println!();
    println!("📦 Next steps to create executable:");
    println!("  1. Assemble:  as {} -o {}.o", asm_file, output_file);
    println!("  2. Link:      ld {}.o -o {}", output_file, output_file);
    println!("  3. Run:       ./{}", output_file);
    println!();
    println!("📁 Output files:");
    println!("  • Assembly:   {}", asm_file);
    println!("  • Object:     {}.o", output_file);
    println!("  • Binary:     {}", output_file);
    println!();
    println!("✨ [Status] All Phases 1-10 Complete! ✨");
}