//! Cargo integration wrapper for GaiaRusted compiler
//!
//! Usage: cargo gaiarusted [args]
//! 
//! This binary integrates GaiaRusted with cargo, allowing it to be used
//! as the compiler backend through .cargo/config.
//!
//! To use with a project:
//! ```
//! # Create .cargo/config.toml in your project:
//! [build]
//! rustc = "cargo-gaiarusted"
//! ```

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "--version" {
        println!("cargo-gaiarusted 0.1.0");
        println!("A Cargo integration wrapper for GaiaRusted compiler");
        process::exit(0);
    }

    if args.len() > 1 && args[1] == "--help" {
        print_help();
        process::exit(0);
    }

    if args.len() > 1 && args[1] == "build" {
        handle_cargo_build(&args[2..]);
    } else if args.len() > 1 && args[1] == "run" {
        handle_cargo_run(&args[2..]);
    } else if args.len() > 1 && args[1] == "init" {
        handle_cargo_init(&args[2..]);
    } else if args.len() > 1 && args[1] == "add" {
        handle_cargo_add(&args[2..]);
    } else if args.len() > 1 && args[1] == "clean" {
        handle_cargo_clean();
    } else {
        handle_rustc_wrapper(&args[1..]);
    }
}

/// Handle cargo build command
fn handle_cargo_build(args: &[String]) {
    let mut release = false;
    let mut output_path: Option<String> = None;

    for i in 0..args.len() {
        match args[i].as_str() {
            "--release" => release = true,
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_path = Some(args[i + 1].clone());
                }
            }
            _ => {}
        }
    }

    println!("🔧 Building with GaiaRusted compiler...");
    
    let manifest_path = PathBuf::from("Cargo.toml");
    if !manifest_path.exists() {
        eprintln!("❌ Error: Cargo.toml not found");
        process::exit(1);
    }

    match gaiarusted::CargoAPI::build(".", gaiarusted::CargoBuildConfig {
        profile: if release {
            gaiarusted::BuildProfile::Release
        } else {
            gaiarusted::BuildProfile::Debug
        },
        opt_level: if release { 3 } else { 0 },
        target: "x86_64-unknown-linux-gnu".to_string(),
        features: Vec::new(),
        workspace_mode: false,
    }) {
        Ok(result) => {
            println!("✓ Build succeeded!");
            println!("  Output: {}", result.output_path.display());
            println!("  Artifacts: {}", result.artifacts.len());
        }
        Err(e) => {
            eprintln!("❌ Build failed: {}", e);
            process::exit(1);
        }
    }
}

/// Handle cargo init command
fn handle_cargo_init(args: &[String]) {
    let project_name = if !args.is_empty() {
        args[0].clone()
    } else {
        "my_project".to_string()
    };

    match gaiarusted::CargoAPI::init(".", &project_name) {
        Ok(project) => {
            println!("✓ Created binary (application) package");
            println!("  Package: {}", project.manifest.name);
            println!("  Version: {}", project.manifest.version);
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize project: {}", e);
            process::exit(1);
        }
    }
}

/// Handle cargo run command
fn handle_cargo_run(args: &[String]) {
    let mut release = false;
    
    for arg in args {
        if arg == "--release" {
            release = true;
        }
    }

    println!("🚀 Running with GaiaRusted compiler...");
    
    let manifest_path = PathBuf::from("Cargo.toml");
    if !manifest_path.exists() {
        eprintln!("❌ Error: Cargo.toml not found");
        process::exit(1);
    }

    let build_config = gaiarusted::CargoBuildConfig {
        profile: if release {
            gaiarusted::BuildProfile::Release
        } else {
            gaiarusted::BuildProfile::Debug
        },
        opt_level: if release { 3 } else { 0 },
        target: "x86_64-unknown-linux-gnu".to_string(),
        features: Vec::new(),
        workspace_mode: false,
    };

    match gaiarusted::CargoAPI::build(".", build_config) {
        Ok(result) => {
            println!("✓ Built successfully!");
            if result.output_path.exists() {
                println!("📌 Running: {}", result.output_path.display());
                match Command::new(&result.output_path).status() {
                    Ok(status) => {
                        if status.success() {
                            println!("✓ Execution completed");
                        } else {
                            eprintln!("❌ Execution failed with status: {}", status);
                            process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to execute: {}", e);
                        process::exit(1);
                    }
                }
            } else {
                eprintln!("❌ Output binary not found: {}", result.output_path.display());
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("❌ Build failed: {}", e);
            process::exit(1);
        }
    }
}

/// Handle cargo add command
fn handle_cargo_add(args: &[String]) {
    if args.is_empty() {
        eprintln!("❌ Error: Dependency name required");
        eprintln!("Usage: cargo gaiarusted add <name> [--version VERSION]");
        process::exit(1);
    }

    let name = &args[0];
    let version = if args.len() > 2 && args[1] == "--version" {
        args[2].clone()
    } else {
        "0.1.0".to_string()
    };

    match gaiarusted::CargoAPI::add_dependency(".", name, &version) {
        Ok(_) => {
            println!("✓ Added dependency: {} = \"{}\"", name, version);
        }
        Err(e) => {
            eprintln!("❌ Failed to add dependency: {}", e);
            process::exit(1);
        }
    }
}

/// Handle cargo clean command
fn handle_cargo_clean() {
    println!("🧹 Cleaning build artifacts...");
    
    match gaiarusted::CargoProject::open(".") {
        Ok(project) => {
            let target_dir = &project.target_dir;
            if target_dir.exists() {
                match fs::remove_dir_all(target_dir) {
                    Ok(_) => println!("✓ Cleaned {}", target_dir.display()),
                    Err(e) => {
                        eprintln!("❌ Failed to clean: {}", e);
                        process::exit(1);
                    }
                }
            } else {
                println!("✓ Nothing to clean");
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to access project: {}", e);
            process::exit(1);
        }
    }
}

/// Handle rustc wrapper mode (for cargo integration)
fn handle_rustc_wrapper(args: &[String]) {
    let mut input_file: Option<String> = None;
    let mut output_file: Option<String> = None;
    let mut is_link = false;
    let mut emit = "link";

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--edition" | "--target" | "-L" | "-l" | "--cap-lints" => {
                i += 2;
                continue;
            }
            "--emit" => {
                if i + 1 < args.len() {
                    emit = &args[i + 1];
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            "-o" | "--out-dir" => {
                if i + 1 < args.len() {
                    output_file = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            "--crate-type" => {
                if i + 1 < args.len() {
                    is_link = args[i + 1] != "lib";
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            arg if arg.ends_with(".rs") => {
                input_file = Some(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    if let Some(input) = input_file {
        match gaiarusted::compile_files(
            &gaiarusted::CompilationConfig::new()
                .add_source_file(&input)
                .unwrap_or_default()
                .set_output(output_file.unwrap_or_else(|| "output".to_string()))
                .set_output_format(gaiarusted::OutputFormat::Object)
        ) {
            Ok(result) => {
                if result.success {
                    println!("cargo:info=Successfully compiled {}", input);
                    process::exit(0);
                } else {
                    for error in result.errors {
                        eprintln!("error: {}", error);
                    }
                    process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("error: {}", e);
                process::exit(1);
            }
        }
    } else {
        eprintln!("error: No input file specified");
        process::exit(1);
    }
}

fn print_help() {
    println!("cargo-gaiarusted - Cargo integration for GaiaRusted compiler");
    println!();
    println!("USAGE:");
    println!("    cargo gaiarusted <SUBCOMMAND>");
    println!();
    println!("SUBCOMMANDS:");
    println!("    build       Build the project with GaiaRusted");
    println!("    run         Build and run the project");
    println!("    clean       Remove build artifacts");
    println!("    init        Initialize a new project");
    println!("    add         Add a dependency to the project");
    println!("    --version   Print version information");
    println!("    --help      Print this help message");
    println!();
    println!("OPTIONS:");
    println!("    --release   Build in release mode (optimized)");
    println!();
    println!("EXAMPLES:");
    println!("    cargo gaiarusted build");
    println!("    cargo gaiarusted build --release");
    println!("    cargo gaiarusted run");
    println!("    cargo gaiarusted run --release");
    println!("    cargo gaiarusted clean");
    println!("    cargo gaiarusted init my_project");
    println!("    cargo gaiarusted add serde");
}
