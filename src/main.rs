//! GiaRusted - A Rust Compiler Built from Scratch
//!
//! This is the entry point for the gaiarusted compiler.

use std::env;
use std::fs;
use std::process;
use std::time::Instant;

mod lexer;
mod parser;
mod lowering;
mod typechecker;
mod borrowchecker;
mod mir;
mod codegen;
mod formatter;

use formatter::{Phase, Status, Colors};

fn main() {
    let total_start = Instant::now();
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("{}usage:{} gaiarusted <file.rs> [-o <output>]", Colors::BOLD, Colors::RESET);
        process::exit(1);
    }

    let input_file = &args[1];
    let mut output_file = "a.out".to_string();

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                if i + 1 < args.len() {
                    output_file = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("error: -o requires an argument");
                    process::exit(1);
                }
            }
            _ => {
                eprintln!("error: unknown argument '{}'", args[i]);
                process::exit(1);
            }
        }
    }

    let source = match fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(e) => {
            formatter::error(&format!("cannot read '{}': {}", input_file, e));
            process::exit(1);
        }
    };

    formatter::start_compilation(input_file);
    let mut phase_times = vec![];

    // Phase 1: Lexing
    let lex_start = Instant::now();
    formatter::progress(&Phase::LEXING);
    let tokens = match lexer::lex(&source) {
        Ok(tokens) => tokens,
        Err(e) => {
            formatter::error(&format!("lexer error: {}", e));
            process::exit(1);
        }
    };
    let lex_time = lex_start.elapsed();
    phase_times.push(("Lexing".to_string(), lex_time));
    println!();

    // Phase 2: Parsing
    let parse_start = Instant::now();
    formatter::progress(&Phase::PARSING);
    let ast = match parser::parse(tokens) {
        Ok(ast) => ast,
        Err(e) => {
            formatter::error(&format!("parser error: {}", e));
            process::exit(1);
        }
    };
    let parse_time = parse_start.elapsed();
    phase_times.push(("Parsing".to_string(), parse_time));
    println!();

    // Phase 2.5: Module resolution
    let base_dir = std::path::Path::new(&input_file)
        .parent()
        .map(|p| p.to_string_lossy().into_owned());
    let ast = match parser::resolve_file_modules(ast, base_dir.as_deref()) {
        Ok(ast) => ast,
        Err(e) => {
            formatter::error(&format!("module resolution error: {}", e));
            process::exit(1);
        }
    };

    // Phase 3: Lowering
    let lower_start = Instant::now();
    formatter::progress(&Phase::LOWERING);
    let hir = match lowering::lower(&ast) {
        Ok(hir) => hir,
        Err(e) => {
            formatter::error(&format!("lowering error: {}", e));
            process::exit(1);
        }
    };
    let lower_time = lower_start.elapsed();
    phase_times.push(("Lowering".to_string(), lower_time));
    println!();

    // Phase 4: Type Checking
    let tc_start = Instant::now();
    formatter::progress(&Phase::TYPECHECKING);
    if let Err(e) = typechecker::check_types(&hir) {
        formatter::error(&format!("type check error: {}", e));
        process::exit(1);
    }
    let tc_time = tc_start.elapsed();
    phase_times.push(("Type Checking".to_string(), tc_time));
    println!();

    // Phase 5: Borrow Checking
    let bc_start = Instant::now();
    formatter::progress(&Phase::BORROWCHECKING);
    if let Err(e) = borrowchecker::check_borrows(&hir) {
        formatter::error(&format!("borrow check error: {}", e));
        process::exit(1);
    }
    let bc_time = bc_start.elapsed();
    phase_times.push(("Borrow Checking".to_string(), bc_time));
    println!();

    // Phase 6: MIR Lowering
    let mir_start = Instant::now();
    formatter::progress(&Phase::MIR_LOWERING);
    let mir = match mir::lower_to_mir(&hir) {
        Ok(mir) => mir,
        Err(e) => {
            formatter::error(&format!("MIR error: {}", e));
            process::exit(1);
        }
    };
    let mir_time = mir_start.elapsed();
    phase_times.push(("MIR Lowering".to_string(), mir_time));
    println!();

    // Phase 7: Optimization
    let opt_start = Instant::now();
    formatter::progress(&Phase::OPTIMIZATION);
    let mut optimized_mir = mir.clone();
    if let Err(e) = mir::optimize_mir(&mut optimized_mir) {
        formatter::error(&format!("optimization error: {}", e));
        process::exit(1);
    }
    let opt_time = opt_start.elapsed();
    phase_times.push(("Optimization".to_string(), opt_time));
    println!();

    // Phase 8: Code Generation
    let cg_start = Instant::now();
    formatter::progress(&Phase::CODEGEN);
    let assembly = match codegen::generate_code(&optimized_mir) {
        Ok(asm) => asm,
        Err(e) => {
            formatter::error(&format!("codegen error: {}", e));
            process::exit(1);
        }
    };
    let cg_time = cg_start.elapsed();
    phase_times.push(("Code Generation".to_string(), cg_time));
    println!();

    // Write assembly file
    let asm_file = format!("{}.s", output_file);
    if let Err(e) = fs::write(&asm_file, &assembly) {
        formatter::error(&format!("cannot write assembly: {}", e));
        process::exit(1);
    }

    // Link
    if let Err(e) = codegen::object::link_assembly(&asm_file, &output_file) {
        formatter::error(&format!("linking failed: {}", e));
        process::exit(1);
    }

    let total_time = total_start.elapsed();
    formatter::success(&format!("compiled to '{}'", output_file));

    // Print summary
    println!();
    println!("{}summary:{}",Colors::DIM, Colors::RESET);
    println!("  {}• {} lines of code", Colors::CYAN, source.lines().count());
    println!("  {}• {} ms total", Colors::CYAN, total_time.as_millis());
    println!();
}