//! GiaRusted Compiler CLI
//!
//! Command-line interface for the GiaRusted compiler library

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

use gaiarusted::{CompilationConfig, OutputFormat, compile_files, formatter};
use std::time::Instant;

#[derive(Debug)]
struct CliArgs {
    input: Vec<PathBuf>,
    output: PathBuf,
    output_format: OutputFormat,
    lib_paths: Vec<PathBuf>,
    libraries: Vec<String>,
    opt_level: u32,
    verbose: bool,
    debug: bool,
    discover_mode: bool,
    show_output: bool,
}

impl CliArgs {
    fn parse(args: Vec<String>) -> Result<Self, String> {
        let mut input = Vec::new();
        let mut output = PathBuf::from("output");
        let mut output_format = OutputFormat::Executable;
        let mut lib_paths = Vec::new();
        let mut libraries = Vec::new();
        let mut opt_level = 2;
        let mut verbose = false;
        let mut debug = false;
        let mut discover_mode = false;
        let mut show_output = false;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--help" | "-h" => {
                    Self::print_help();
                    process::exit(0);
                }
                "--help-formats" => {
                    Self::print_format_help();
                    process::exit(0);
                }
                "--version" => {
                    println!("GiaRusted Compiler v0.8.0");
                    process::exit(0);
                }
                "-o" | "--output" => {
                    if i + 1 >= args.len() {
                        return Err("-o requires an argument".to_string());
                    }
                    output = PathBuf::from(&args[i + 1]);
                    i += 2;
                }
                "--format" => {
                    if i + 1 >= args.len() {
                        return Err("--format requires an argument".to_string());
                    }
                    output_format = match args[i + 1].as_str() {
                        "asm" | "assembly" => OutputFormat::Assembly,
                        "obj" | "object" => OutputFormat::Object,
                        "exe" | "executable" => OutputFormat::Executable,
                        "bash" | "sh" => OutputFormat::BashScript,
                        "lib" | "library" => OutputFormat::Library,
                        other => return Err(format!("Unknown format: {}", other)),
                    };
                    i += 2;
                }
                "-L" => {
                    if i + 1 >= args.len() {
                        return Err("-L requires an argument".to_string());
                    }
                    lib_paths.push(PathBuf::from(&args[i + 1]));
                    i += 2;
                }
                "-l" => {
                    if i + 1 >= args.len() {
                        return Err("-l requires an argument".to_string());
                    }
                    libraries.push(args[i + 1].clone());
                    i += 2;
                }
                "-O" => {
                    if i + 1 >= args.len() {
                        return Err("-O requires an argument (0-3)".to_string());
                    }
                    opt_level = args[i + 1].parse::<u32>()
                        .map_err(|_| "Invalid optimization level".to_string())?;
                    i += 2;
                }
                "-v" | "--verbose" => {
                    verbose = true;
                    i += 1;
                }
                "-S" | "--show-output" => {
                    show_output = true;
                    i += 1;
                }
                "-g" | "--debug" => {
                    debug = true;
                    i += 1;
                }
                "--discover" => {
                    discover_mode = true;
                    i += 1;
                }
                arg if arg.starts_with('-') => {
                    return Err(format!("Unknown option: {}", arg));
                }
                arg => {
                    input.push(PathBuf::from(arg));
                    i += 1;
                }
            }
        }

        if input.is_empty() && !discover_mode {
            return Err("No input files specified".to_string());
        }

        Ok(CliArgs {
            input,
            output,
            output_format,
            lib_paths,
            libraries,
            opt_level,
            verbose,
            debug,
            discover_mode,
            show_output,
        })
    }

    fn print_help() {
        println!("GiaRusted Compiler - A Rust Compiler Built from Scratch (v0.8.0)");
        println!();
        println!("USAGE:");
        println!("    gaiarusted [OPTIONS] <FILES>...");
        println!("    gaiarusted --discover [OPTIONS] [DIRECTORY]");
        println!();
        println!("OPTIONS:");
        println!("    -o, --output <PATH>          Output file path (default: output)");
        println!("    --format <FORMAT>            Output format:");
        println!("                                   - asm:  x86-64 assembly (.s)");
        println!("                                   - obj:  ELF object file (.o)");
        println!("                                   - exe:  executable binary (default)");
        println!("                                   - bash: bash build script (.sh)");
        println!("                                   - lib:  static library (.a)");
        println!();
        println!("    -L <PATH>                    Add library search path");
        println!("    -l <LIB>                     Link library");
        println!("    -O <LEVEL>                   Optimization level (0-3, default: 2)");
        println!("    -v, --verbose                Verbose output");
        println!("    -S, --show-output            Display generated output in terminal");
        println!("    -g, --debug                  Include debug information");
        println!("    --discover                   Auto-discover .rs files in directory");
        println!("    -h, --help                   Print this help message");
        println!("    --help-formats               Show detailed format information");
        println!("    --version                    Print version");
        println!();
        println!("EXAMPLES:");
        println!("    # Compile single file to executable");
        println!("    gaiarusted main.rs");
        println!();
        println!("    # Compile to static library");
        println!("    gaiarusted lib.rs -o mylib --format lib");
        println!();
        println!("    # Show generated assembly in terminal");
        println!("    gaiarusted main.rs --format asm -S");
        println!();
        println!("    # Discover and compile all .rs files in src/ to assembly");
        println!("    gaiarusted --discover src/ --format asm");
        println!();
        println!("    # Compile with maximum optimization");
        println!("    gaiarusted main.rs -O 3 -v");
        println!();
        println!("For more details about output formats, use: gaiarusted --help-formats");
    }

    fn print_format_help() {
        println!("GiaRusted Output Formats");
        println!();
        println!("┌─ ASSEMBLY (.s)");
        println!("│  Generates x86-64 assembly code for inspection and debugging.");
        println!("│  Use for: Understanding generated code, manual optimization");
        println!("│  Example: gaiarusted main.rs --format asm");
        println!("│");
        println!("└─ Commands:");
        println!("   as output.s -o output.o    # Assemble");
        println!("   ld output.o -o output      # Link");
        println!();
        println!("┌─ OBJECT (.o)");
        println!("│  Generates ELF object files for linking with other code.");
        println!("│  Use for: Creating object files for larger projects");
        println!("│  Example: gaiarusted lib.rs --format obj");
        println!();
        println!("┌─ EXECUTABLE (no extension)");
        println!("│  Generates assembly and shows build instructions.");
        println!("│  Use for: Final applications, production binaries");
        println!("│  Example: gaiarusted main.rs --format exe");
        println!("│");
        println!("└─ Commands (shown after compilation):");
        println!("   as output.s -o output.o    # Assemble");
        println!("   ld output.o -o output      # Link");
        println!("   ./output                   # Run");
        println!();
        println!("┌─ BASH SCRIPT (.sh)");
        println!("│  Generates a self-contained shell script for automatic building.");
        println!("│  Use for: CI/CD pipelines, automated builds, reproducible builds");
        println!("│  Example: gaiarusted main.rs --format bash");
        println!("│");
        println!("└─ Usage:");
        println!("   chmod +x output.sh         # Make executable");
        println!("   ./output.sh                # Run automated build");
        println!();
        println!("┌─ LIBRARY (.a)");
        println!("│  Generates code for static library creation.");
        println!("│  Use for: Creating reusable libraries, code distribution");
        println!("│  Example: gaiarusted lib.rs --format lib");
        println!("│");
        println!("└─ Commands:");
        println!("   as output.s -o output.o    # Assemble");
        println!("   ar rcs output.a output.o   # Create library");
        println!("   ld program.o -L. -loutput -o program  # Link");
        println!();
        println!("═══════════════════════════════════════════════════════════════");
        println!("Format Comparison:");
        println!("  Assembly   │ For inspection and optimization");
        println!("  Object     │ For linking individual object files");
        println!("  Executable │ Manual build control (traditional approach)");
        println!("  Bash Script│ Automated building (recommended for CI/CD)");
        println!("  Library    │ Reusable code for other projects");
        println!();
        println!("See EXAMPLES_OUTPUT_FORMATS.md for more detailed examples.");
    }
}

fn extract_type_info(message: &str) -> Option<(String, String, String)> {
    // Parse message like "mismatched types\nVARIABLE: x\nExpected: String\nFound: &str"
    let lines: Vec<&str> = message.lines().collect();
    let mut expected = String::new();
    let mut found = String::new();
    let mut variable = String::new();
    
    for line in lines {
        if line.starts_with("Expected:") {
            expected = line.replace("Expected:", "").trim().to_string();
        } else if line.starts_with("Found:") {
            found = line.replace("Found:", "").trim().to_string();
        } else if line.starts_with("VARIABLE:") {
            variable = line.replace("VARIABLE:", "").trim().to_string();
        }
    }
    
    if !expected.is_empty() && !found.is_empty() {
        Some((expected, found, variable))
    } else {
        None
    }
}

fn extract_borrow_info(message: &str) -> Option<(String, String, String)> {
    // Parse message like "use of moved value\nVARIABLE: x\nREASON: ownership transferred to y"
    let lines: Vec<&str> = message.lines().collect();
    let mut error_type = String::new();
    let mut variable = String::new();
    let mut reason = String::new();
    
    for line in lines {
        if line.starts_with("ERROR_TYPE:") {
            error_type = line.replace("ERROR_TYPE:", "").trim().to_string();
        } else if line.starts_with("VARIABLE:") {
            variable = line.replace("VARIABLE:", "").trim().to_string();
        } else if line.starts_with("REASON:") {
            reason = line.replace("REASON:", "").trim().to_string();
        }
    }
    
    if !variable.is_empty() {
        Some((error_type, variable, reason))
    } else {
        None
    }
}

fn print_detailed_error(error: &gaiarusted::CompileError, error_num: usize, total_errors: usize) {
    // Try to extract line/column from error message or use provided values
    let (line_num, col_num) = if let Some(line) = error.line {
        (line, error.column.unwrap_or(0))
    } else {
        (0, 0)
    };
    
    // Map error message to error code
    let error_code = gaiarusted::error_codes::get_error_code_for_message(&error.message)
        .and_then(|code| gaiarusted::error_codes::get_error_code(&code).map(|_| code));
    
    // Parse error message for type mismatch info
    let message = &error.message;
    let is_type_mismatch = message.to_lowercase().contains("type mismatch");
    let is_mismatched_types = message.to_lowercase().contains("mismatched types");
    let is_borrow_error = message.to_lowercase().contains("moved value") 
        || message.to_lowercase().contains("borrow")
        || message.to_lowercase().contains("ownership");
    
    // If it's a borrow error with file info, try to show rustc-style format
    if is_borrow_error && error.file.is_some() {
        if let Some((error_type, variable, _reason)) = extract_borrow_info(message) {
            let file_path = error.file.as_ref().unwrap().display().to_string();
            let borrow_error = gaiarusted::borrow_error_display::BorrowError::new(
                "E0382",
                &error_type,
                &variable,
                &file_path,
                line_num.max(1),
            );
            
            eprint!("{}", borrow_error.display());
            return;
        }
    }
    
    // If it's a type error with file info, try to show rustc-style format
    if (is_type_mismatch || is_mismatched_types) && error.file.is_some() {
        // Try to use source-aware formatter
        let file_path = error.file.as_ref().unwrap().display().to_string();
        
        // Extract expected/found types from message
        if let Some(caps) = extract_type_info(message) {
            let (expected, found, variable) = caps;
            let mut src_error = gaiarusted::source_display::SourceError::new(
                "E0308",
                "mismatched types",
                &file_path,
                line_num.max(1),
                col_num.max(1),
                expected.len().max(1),
                &expected,
                &found,
            );
            
            // Try to find the actual line by searching for the variable name
            if !variable.is_empty() {
                let _ = src_error.find_source_line_by_pattern(&variable);
            }
            
            // Add suggestions from message
            if message.contains("SUGGESTIONS:") {
                let suggestions: Vec<&str> = message.split("\n|").collect();
                for sugg in suggestions.iter().skip(1) {
                    src_error = src_error.with_suggestion(sugg.to_string());
                }
            }
            
            eprint!("{}", src_error.display());
            return;
        }
    }
    
    // Fall back to simple error display
    let location = match (error.line, error.column) {
        (Some(line), Some(col)) => format!("{}:{}", line, col),
        (Some(line), None) => format!("{}:1", line),
        _ => String::new(),
    };
    
    let file_location = if let Some(file) = &error.file {
        if location.is_empty() {
            file.display().to_string()
        } else {
            format!("{}:{}", file.display(), location)
        }
    } else if !location.is_empty() {
        location.clone()
    } else {
        "[unknown]".to_string()
    };
    
    let severity = match error.kind {
        gaiarusted::ErrorKind::CodeIssue => format_error("error"),
        gaiarusted::ErrorKind::CompilerLimitation => format_warning("limitation"),
        gaiarusted::ErrorKind::CompilerBug => format_error("bug"),
        gaiarusted::ErrorKind::InternalError => format_error("error"),
    };
    
    let kind_suffix = if error.kind != gaiarusted::ErrorKind::CodeIssue {
        format!(" [{}]", error.kind)
    } else {
        String::new()
    };
    
    let message_lines: Vec<&str> = error.message.split('\n').collect();
    
    // Display error with code if available
    let error_display = if let Some(code) = &error_code {
        format!("{}: {}[{}]: {}{}", 
            severity,
            file_location,
            code,
            message_lines[0],
            kind_suffix)
    } else {
        format!("{}: {}: {}{}", 
            severity,
            file_location,
            message_lines[0],
            kind_suffix)
    };
    
    eprintln!("{}", error_display);
    
    for line in &message_lines[1..] {
        eprintln!("  {}", line);
    }
    
    // Display suggestion if error code is available
    if let Some(code) = error_code {
        if let Some(error_detail) = gaiarusted::error_codes::get_error_code(&code) {
            eprintln!("  suggestion: {}", error_detail.suggestion);
        }
    }
    
    if let Some(file) = &error.file {
        if let Ok(source) = fs::read_to_string(file) {
            if let Some(line_num) = error.line {
                let lines: Vec<&str> = source.lines().collect();
                if line_num > 0 && line_num <= lines.len() {
                    let line_idx = line_num - 1;
                    let _line_content = lines[line_idx];
                    let line_num_str = line_num.to_string();
                    let padding = " ".repeat(line_num_str.len() + 2);
                    
                    let num_lines = lines.len();
                    let start_line = if line_num > 1 { line_num - 2 } else { line_num - 1 };
                    let end_line = std::cmp::min(line_num + 1, num_lines);
                    
                    eprintln!("  {} | ", " ".repeat(line_num_str.len()));
                    
                    for display_line in start_line..end_line {
                        let is_error_line = display_line + 1 == line_num;
                        let line_num_display = display_line + 1;
                        let line_str = lines[display_line];
                        let line_marker = if is_error_line { ">" } else { "|" };
                        
                        eprintln!("  {} {} {}", 
                            format!("{:>width$}", line_num_display, width = line_num_str.len()),
                            line_marker,
                            line_str);
                        
                        if is_error_line {
                            if let Some(col) = error.column {
                                let col = col.saturating_sub(1);
                                let pointer_pos = col.min(line_str.len());
                                let mut pointer_line = String::new();
                                pointer_line.push_str(&" ".repeat(pointer_pos));
                                pointer_line.push_str(&format_error("^"));
                                eprintln!("  {} | {}", padding, pointer_line);
                            }
                        }
                    }
                }
            }
        }
    }
    
    if let Some(sugg) = &error.suggestion {
        eprintln!("  {} = suggestion: {}", format_info("help"), sugg);
    }
    
    if let Some(help) = &error.help {
        eprintln!("  {} = {}", format_info("help"), help);
    }
    
    if error_num < total_errors {
        eprintln!();
    }
}

fn format_error(text: &str) -> String {
    format!("\x1b[1;31m{}\x1b[0m", text)
}

fn format_warning(text: &str) -> String {
    format!("\x1b[1;33m{}\x1b[0m", text)
}

fn format_info(text: &str) -> String {
    format!("\x1b[1;36m{}\x1b[0m", text)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let cli_args = match CliArgs::parse(args) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Use --help for usage information");
            process::exit(1);
        }
    };

    // Build configuration
    let mut config = CompilationConfig::new()
        .set_output(&cli_args.output)
        .set_output_format(cli_args.output_format)
        .set_opt_level(cli_args.opt_level)
        .with_verbose(cli_args.verbose)
        .with_debug(cli_args.debug);

    // Add libraries and library paths
    for lib_path in cli_args.lib_paths {
        config = config.add_lib_path(lib_path);
    }
    for lib in cli_args.libraries {
        config = config.add_library(lib);
    }

    // Handle discovery mode or explicit files
    if cli_args.discover_mode {
        let discover_path = if cli_args.input.is_empty() {
            PathBuf::from("src")
        } else {
            cli_args.input[0].clone()
        };

        if cli_args.verbose {
            println!("Discovering .rs files in: {}", discover_path.display());
        }

        match config.discover_sources(&discover_path) {
            Ok(new_config) => {
                config = new_config;
                if cli_args.verbose {
                    println!("Found {} source files", config.source_files.len());
                    for file in &config.source_files {
                        println!("  - {}", file.display());
                    }
                }
            }
            Err(e) => {
                eprintln!("Discovery error: {}", e);
                process::exit(1);
            }
        }
    } else {
        for input_file in cli_args.input {
            match config.add_source_file(&input_file) {
                Ok(new_config) => config = new_config,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
    }

    let start = Instant::now();
    formatter::start_compilation(&format!("{} file(s)", config.source_files.len()));

    // Compile
    match compile_files(&config) {
        Ok(result) => {
            let total_time = start.elapsed();
            if result.success {
                formatter::success(&format!("compiled to '{}'", config.output_path.display()));
                println!();
                println!("{}summary{}", formatter::Colors::DIM, formatter::Colors::RESET);
                println!("  {}•{} {} lines of code", formatter::Colors::CYAN, formatter::Colors::RESET, result.stats.total_lines);
                println!("  {}•{} {} ms total", formatter::Colors::CYAN, formatter::Colors::RESET, total_time.as_millis());
                println!();
                
                if cli_args.show_output {
                    let asm_file = format!("{}.s", config.output_path.display());
                    if let Ok(asm_content) = fs::read_to_string(&asm_file) {
                        println!("==================================================");
                        println!("Generated Assembly Output:");
                        println!("==================================================");
                        println!("{}", asm_content);
                        println!("==================================================");
                        println!();
                    }
                }
                
                println!("Output files:");
                for file in &result.output_files {
                    if file.exists() {
                        let size = fs::metadata(file)
                            .map(|m| m.len())
                            .unwrap_or(0);
                        println!("   - {} ({} bytes)", file.display(), size);
                    } else {
                        println!("   - {} (to be generated)", file.display());
                    }
                }
                println!();

                if matches!(config.output_format, OutputFormat::Executable | OutputFormat::BashScript) {
                    println!("Next steps:");
                    let asm_file = format!("{}.s", config.output_path.display());
                    let obj_file = format!("{}.o", config.output_path.display());
                    let out_file = config.output_path.display();
                    println!("   1. Assemble:  as {} -o {}", asm_file, obj_file);
                    println!("   2. Link:      ld {} -o {}", obj_file, out_file);
                    println!("   3. Run:       ./{}", out_file);
                }
            } else {
                eprintln!("error: compilation failed with {} error{}",
                    result.errors.len(),
                    if result.errors.len() == 1 { "" } else { "s" });
                eprintln!();
                
                for (idx, error) in result.errors.iter().enumerate() {
                    print_detailed_error(error, idx + 1, result.errors.len());
                }
                
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Fatal compilation error: {}", e);
            process::exit(1);
        }
    }
}