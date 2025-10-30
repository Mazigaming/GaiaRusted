//! High-level compiler API for multi-file compilation

use std::fs;
use std::path::PathBuf;

use crate::config::{CompilationConfig, OutputFormat};
use crate::lexer;
use crate::parser;
use crate::lowering;
use crate::typechecker;
use crate::borrowchecker;
use crate::mir;
use crate::codegen;

/// Compilation error with detailed context
#[derive(Debug, Clone)]
pub struct CompileError {
    pub phase: String,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {} ({})", self.phase, self.message, 
            self.file.as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "unknown".to_string()))
    }
}

impl std::error::Error for CompileError {}

/// Result of compilation
pub struct CompilationResult {
    pub success: bool,
    pub output_files: Vec<PathBuf>,
    pub stats: CompilationStats,
    pub errors: Vec<CompileError>,
}

#[derive(Debug, Clone)]
pub struct CompilationStats {
    pub files_compiled: usize,
    pub total_lines: usize,
    pub assembly_size: usize,
}

impl CompilationStats {
    pub fn new() -> Self {
        CompilationStats {
            files_compiled: 0,
            total_lines: 0,
            assembly_size: 0,
        }
    }
}

/// Compile multiple files according to configuration
pub fn compile_files(config: &CompilationConfig) -> Result<CompilationResult, CompileError> {
    config.validate().map_err(|e| CompileError {
        phase: "Configuration".to_string(),
        message: e,
        file: None,
        line: None,
    })?;

    let mut stats = CompilationStats::new();
    let mut errors = Vec::new();
    let mut output_files = Vec::new();
    let mut all_hir_items = Vec::new();

    for source_file in &config.source_files {
        if config.verbose {
            println!("📝 Compiling: {}", source_file.display());
        }

        match compile_single_file(source_file, config) {
            Ok((hir_items, loc)) => {
                stats.files_compiled += 1;
                stats.total_lines += loc;
                all_hir_items.extend(hir_items);
            }
            Err(e) => {
                if config.verbose {
                    println!("❌ Error compiling {}: {}", source_file.display(), e.message);
                }
                errors.push(CompileError {
                    file: Some(source_file.clone()),
                    ..e
                });
            }
        }
    }

    if !errors.is_empty() {
        return Ok(CompilationResult {
            success: false,
            output_files: Vec::new(),
            stats,
            errors,
        });
    }

    if let Err(e) = typechecker::check_types(&all_hir_items) {
        errors.push(CompileError {
            phase: "Type Checking".to_string(),
            message: e.to_string(),
            file: None,
            line: None,
        });
    }

    if let Err(e) = borrowchecker::check_borrows(&all_hir_items) {
        errors.push(CompileError {
            phase: "Borrow Checking".to_string(),
            message: e.to_string(),
            file: None,
            line: None,
        });
    }

    if !errors.is_empty() {
        return Ok(CompilationResult {
            success: false,
            output_files: Vec::new(),
            stats,
            errors,
        });
    }

    match mir::lower_to_mir(&all_hir_items) {
        Ok(mir_items) => {
            let mut optimized_mir = mir_items.clone();
            if let Err(e) = mir::optimize_mir(&mut optimized_mir, config.opt_level) {
                errors.push(CompileError {
                    phase: "MIR Optimization".to_string(),
                    message: e.to_string(),
                    file: None,
                    line: None,
                });
            }

            if errors.is_empty() {
                match codegen::generate_code(&optimized_mir) {
                    Ok(assembly) => {
                        stats.assembly_size = assembly.len();
                        match write_output(&config, &assembly) {
                            Ok(files) => {
                                output_files = files;
                            }
                            Err(e) => {
                                errors.push(CompileError {
                                    phase: "Output Generation".to_string(),
                                    message: e,
                                    file: None,
                                    line: None,
                                });
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(CompileError {
                            phase: "Code Generation".to_string(),
                            message: e.to_string(),
                            file: None,
                            line: None,
                        });
                    }
                }
            }
        }
        Err(e) => {
            errors.push(CompileError {
                phase: "MIR Lowering".to_string(),
                message: e.to_string(),
                file: None,
                line: None,
            });
        }
    }

    Ok(CompilationResult {
        success: errors.is_empty(),
        output_files,
        stats,
        errors,
    })
}

/// Compile a single source file
fn compile_single_file(
    source_file: &std::path::Path,
    _config: &CompilationConfig,
) -> Result<(Vec<lowering::HirItem>, usize), CompileError> {
    let source = fs::read_to_string(source_file).map_err(|e| CompileError {
        phase: "File Reading".to_string(),
        message: format!("Failed to read file: {}", e),
        file: Some(source_file.to_path_buf()),
        line: None,
    })?;

    let loc = source.lines().count();

    let tokens = lexer::lex(&source).map_err(|e| CompileError {
        phase: "Lexing".to_string(),
        message: e.to_string(),
        file: Some(source_file.to_path_buf()),
        line: None,
    })?;

    let ast = parser::parse(tokens).map_err(|e| CompileError {
        phase: "Parsing".to_string(),
        message: e.to_string(),
        file: Some(source_file.to_path_buf()),
        line: None,
    })?;

    let hir = lowering::lower(&ast).map_err(|e| CompileError {
        phase: "Lowering".to_string(),
        message: e.to_string(),
        file: Some(source_file.to_path_buf()),
        line: None,
    })?;

    Ok((hir, loc))
}

/// Write output files based on configuration
fn write_output(config: &CompilationConfig, assembly: &str) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let output_path = config.output_path_with_extension();

    match config.output_format {
        OutputFormat::Assembly => {
            fs::write(&output_path, assembly)
                .map_err(|e| format!("Failed to write assembly file: {}", e))?;
            files.push(output_path);
        }
        OutputFormat::Object => {
            let asm_file = format!("{}.s", config.output_path.display());
            fs::write(&asm_file, assembly)
                .map_err(|e| format!("Failed to write assembly file: {}", e))?;
            files.push(PathBuf::from(&asm_file));
            files.push(output_path);
        }
        OutputFormat::Executable => {
            let asm_file = format!("{}.s", config.output_path.display());
            fs::write(&asm_file, assembly)
                .map_err(|e| format!("Failed to write assembly file: {}", e))?;
            files.push(PathBuf::from(&asm_file));
            files.push(output_path.clone());
            generate_build_script(config, &asm_file, &output_path)?;
        }
        OutputFormat::BashScript => {
            let asm_file = format!("{}.s", config.output_path.display());
            fs::write(&asm_file, assembly)
                .map_err(|e| format!("Failed to write assembly file: {}", e))?;
            files.push(PathBuf::from(&asm_file));
            let binary_file = config.output_path.clone();
            generate_bash_script(config, &asm_file, &output_path, &binary_file)?;
            files.push(output_path);
        }
        OutputFormat::Library => {
            let asm_file = format!("{}.s", config.output_path.display());
            fs::write(&asm_file, assembly)
                .map_err(|e| format!("Failed to write assembly file: {}", e))?;
            files.push(PathBuf::from(&asm_file));
            files.push(output_path);
        }
    }

    Ok(files)
}

/// Generate a bash script for building
fn generate_bash_script(
    _config: &CompilationConfig,
    asm_file: &str,
    script_path: &std::path::Path,
    binary_file: &std::path::Path,
) -> Result<(), String> {
    let binary_name = binary_file.display().to_string();
    let obj_file = format!("{}.o", binary_name);
    
    let script = format!(
        "#!/bin/bash\n\
        # Auto-generated build script by GiaRusted\n\
        set -e\n\
        \n\
        echo \"🔧 Building {}...\"\n\
        \n\
        # Assemble\n\
        echo \"📝 Assembling...\"\n\
        as {} -o {}\n\
        \n\
        # Link\n\
        echo \"🔗 Linking...\"\n\
        ld {} -o {}\n\
        \n\
        # Make executable\n\
        chmod +x {}\n\
        \n\
        echo \"✅ Build complete! Run with: ./{}\"\n\
        ",
        binary_name,
        asm_file,
        obj_file,
        obj_file,
        binary_name,
        binary_name,
        binary_name
    );

    fs::write(script_path, script)
        .map_err(|e| format!("Failed to write bash script: {}", e))?;

    Ok(())
}

/// Generate a build script for executable output
fn generate_build_script(
    _config: &CompilationConfig,
    asm_file: &str,
    output_file: &std::path::Path,
) -> Result<(), String> {
    // This writes instructions for the user
    let instructions = format!(
        "# Build instructions\n\
        # 1. Assemble:  as {} -o {}.o\n\
        # 2. Link:      ld {}.o -o {}\n\
        # 3. Run:       ./{}\n",
        asm_file,
        output_file.display(),
        output_file.display(),
        output_file.display(),
        output_file.display()
    );

    println!("{}", instructions);
    Ok(())
}