//! GiaRusted Compiler CLI
//!
//! Command-line interface for the GiaRusted compiler library

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

use gaiarusted::{CompilationConfig, OutputFormat, compile_files};

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
                    println!("GiaRusted Compiler v0.2.0");
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
        })
    }

    fn print_help() {
        println!("GiaRusted Compiler - A Rust Compiler Built from Scratch (v0.2.0)");
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
        println!("    # Compile to bash build script");
        println!("    gaiarusted main.rs --format bash");
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

fn print_detailed_error(error: &gaiarusted::CompileError, error_num: usize, total_errors: usize) {
    let location = match (error.line, error.column) {
        (Some(line), Some(col)) => format!("{}:{}", line, col),
        (Some(line), None) => format!("{}:1", line),
        _ => "unknown".to_string(),
    };
    
    if let Some(file) = &error.file {
        eprintln!("error[{}]: {} ({})", error_num, error.message, error.phase);
        eprintln!(" --> {}:{}", file.display(), location);
    } else {
        eprintln!("error[{}]: {} ({})", error_num, error.message, error.phase);
        eprintln!(" --> {}:{}", location, error.phase);
    }
    
    if let Some(file) = &error.file {
        if let Ok(source) = fs::read_to_string(file) {
            if let Some(line_num) = error.line {
                let lines: Vec<&str> = source.lines().collect();
                if line_num > 0 && line_num <= lines.len() {
                    let line_content = lines[line_num - 1];
                    let line_num_str = line_num.to_string();
                    let padding = " ".repeat(line_num_str.len());
                    
                    eprintln!(" {} |", padding);
                    eprintln!(" {} | {}", line_num_str, line_content);
                    
                    if let Some(col) = error.column {
                        let col = col.saturating_sub(1);
                        let pointer_pos = col.min(line_content.len());
                        let mut pointer_line = String::new();
                        pointer_line.push_str(&" ".repeat(pointer_pos));
                        pointer_line.push('^');
                        eprintln!(" {} | {}", padding, pointer_line);
                    }
                }
            }
        }
    }
    
    eprintln!("   = {}", error.phase);
    
    if let Some(sugg) = &error.suggestion {
        eprintln!("   = suggestion: {}", sugg);
    }
    
    if let Some(help) = &error.help {
        eprintln!("   = help: {}", help);
    }
    
    if error_num < total_errors {
        eprintln!();
    }
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

    println!("GiaRusted Compiler v0.2.0");
    println!("==================================================");
    println!("Input files: {}", config.source_files.len());
    for file in &config.source_files {
        println!("   - {}", file.display());
    }
    println!("Output: {} [{}]", 
        config.output_path.display(), 
        config.output_format);
    println!("Optimization level: {}", config.opt_level);
    if config.debug {
        println!("Debug info: enabled");
    }
    println!("==================================================");
    println!();

    // Compile
    match compile_files(&config) {
        Ok(result) => {
            if result.success {
                println!("Compilation successful!");
                println!();
                println!("Statistics:");
                println!("   Files compiled: {}", result.stats.files_compiled);
                println!("   Lines of code: {}", result.stats.total_lines);
                println!("   Assembly size: {} bytes", result.stats.assembly_size);
                println!();
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