//! High-level compiler API for multi-file compilation

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use crate::config::{CompilationConfig, OutputFormat};
use crate::lexer;
use crate::parser;
use crate::lowering;
use crate::typechecker;
use crate::borrowchecker;
use crate::mir;
use crate::codegen;
use crate::codegen::backend::assembler::Assembler;

/// Compilation error with detailed context
#[derive(Debug, Clone)]
pub struct CompileError {
    pub phase: String,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub suggestion: Option<String>,
    pub help: Option<String>,
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
#[derive(Debug, Clone)]
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
    pub compilation_time_ms: u128,
    pub lexing_time_ms: u128,
    pub parsing_time_ms: u128,
    pub lowering_time_ms: u128,
    pub typechecking_time_ms: u128,
    pub borrowchecking_time_ms: u128,
    pub mir_lowering_time_ms: u128,
    pub mir_optimization_time_ms: u128,
    pub codegen_time_ms: u128,
    pub output_time_ms: u128,
}

impl CompilationStats {
    pub fn new() -> Self {
        CompilationStats {
            files_compiled: 0,
            total_lines: 0,
            assembly_size: 0,
            compilation_time_ms: 0,
            lexing_time_ms: 0,
            parsing_time_ms: 0,
            lowering_time_ms: 0,
            typechecking_time_ms: 0,
            borrowchecking_time_ms: 0,
            mir_lowering_time_ms: 0,
            mir_optimization_time_ms: 0,
            codegen_time_ms: 0,
            output_time_ms: 0,
        }
    }
}

/// Compile multiple files according to configuration
pub fn compile_files(config: &CompilationConfig) -> Result<CompilationResult, CompileError> {
    let total_start = Instant::now();
    
    config.validate().map_err(|e| CompileError {
        phase: "Configuration".to_string(),
        message: e,
        file: None,
        line: None,
        column: None,
        suggestion: None,
        help: None,
    })?;

    let mut stats = CompilationStats::new();
    let mut errors = Vec::new();
    let mut output_files = Vec::new();
    let mut all_hir_items = Vec::new();

    for source_file in &config.source_files {
        if config.verbose {
            println!("📝 Compiling: {}", source_file.display());
        }

        match compile_single_file(source_file, config, &mut stats) {
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
        let total_elapsed = total_start.elapsed().as_millis();
        stats.compilation_time_ms = total_elapsed;
        return Ok(CompilationResult {
            success: false,
            output_files: Vec::new(),
            stats,
            errors,
        });
    }

    let tc_start = Instant::now();
    if let Err(e) = typechecker::check_types(&all_hir_items) {
        errors.push(CompileError {
            phase: "Type Checking".to_string(),
            message: e.to_string(),
            file: None,
            line: None,
            column: None,
            suggestion: None,
            help: None,
        });
    }
    stats.typechecking_time_ms = tc_start.elapsed().as_millis();

    let bc_start = Instant::now();
    if let Err(e) = borrowchecker::check_borrows(&all_hir_items) {
        errors.push(CompileError {
            phase: "Borrow Checking".to_string(),
            message: e.to_string(),
            file: None,
            line: None,
            column: None,
            suggestion: None,
            help: None,
        });
    }
    stats.borrowchecking_time_ms = bc_start.elapsed().as_millis();

    if !errors.is_empty() {
        let total_elapsed = total_start.elapsed().as_millis();
        stats.compilation_time_ms = total_elapsed;
        return Ok(CompilationResult {
            success: false,
            output_files: Vec::new(),
            stats,
            errors,
        });
    }

    let mir_lower_start = Instant::now();
    match mir::lower_to_mir(&all_hir_items) {
        Ok(mir_items) => {
            stats.mir_lowering_time_ms = mir_lower_start.elapsed().as_millis();
            
            let mir_opt_start = Instant::now();
            let mut optimized_mir = mir_items.clone();
            if let Err(e) = mir::optimize_mir(&mut optimized_mir, config.opt_level) {
                errors.push(CompileError {
                    phase: "MIR Optimization".to_string(),
                    message: e.to_string(),
                    file: None,
                    line: None,
                    column: None,
                    suggestion: None,
                    help: None,
                });
            }
            stats.mir_optimization_time_ms = mir_opt_start.elapsed().as_millis();

            if errors.is_empty() {
                let codegen_start = Instant::now();
                match codegen::generate_code(&optimized_mir) {
                    Ok(assembly) => {
                        stats.codegen_time_ms = codegen_start.elapsed().as_millis();
                        stats.assembly_size = assembly.len();
                        
                        let output_start = Instant::now();
                        match write_output(&config, &assembly) {
                            Ok(files) => {
                                output_files = files;
                                stats.output_time_ms = output_start.elapsed().as_millis();
                            }
                            Err(e) => {
                                stats.output_time_ms = output_start.elapsed().as_millis();
                                errors.push(CompileError {
                                    phase: "Output Generation".to_string(),
                                    message: e,
                                    file: None,
                                    line: None,
                                    column: None,
                                    suggestion: None,
                                    help: None,
                                });
                            }
                        }
                    }
                    Err(e) => {
                        stats.codegen_time_ms = codegen_start.elapsed().as_millis();
                        errors.push(CompileError {
                            phase: "Code Generation".to_string(),
                            message: e.to_string(),
                            file: None,
                            line: None,
                            column: None,
                            suggestion: None,
                            help: None,
                        });
                    }
                }
            }
        }
        Err(e) => {
            stats.mir_lowering_time_ms = mir_lower_start.elapsed().as_millis();
            errors.push(CompileError {
                phase: "MIR Lowering".to_string(),
                message: e.to_string(),
                file: None,
                line: None,
                column: None,
                suggestion: None,
                help: None,
            });
        }
    }

    let total_elapsed = total_start.elapsed().as_millis();
    stats.compilation_time_ms = total_elapsed;

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
    stats: &mut CompilationStats,
) -> Result<(Vec<lowering::HirItem>, usize), CompileError> {
    let source = fs::read_to_string(source_file).map_err(|e| CompileError {
        phase: "File Reading".to_string(),
        message: format!("Failed to read file: {}", e),
        file: Some(source_file.to_path_buf()),
        line: None,
        column: None,
        suggestion: None,
        help: None,
    })?;

    let loc = source.lines().count();

    let lex_start = Instant::now();
    let tokens = lexer::lex(&source).map_err(|e| CompileError {
        phase: "Lexing".to_string(),
        message: e.to_string(),
        file: Some(source_file.to_path_buf()),
        line: None,
        column: None,
        suggestion: None,
        help: None,
    })?;
    stats.lexing_time_ms += lex_start.elapsed().as_millis();

    let parse_start = Instant::now();
    let ast = parser::parse(tokens).map_err(|e| CompileError {
        phase: "Parsing".to_string(),
        message: e.to_string(),
        file: Some(source_file.to_path_buf()),
        line: None,
        column: None,
        suggestion: None,
        help: None,
    })?;
    stats.parsing_time_ms += parse_start.elapsed().as_millis();

    let lower_start = Instant::now();
    let hir = lowering::lower(&ast).map_err(|e| CompileError {
        phase: "Lowering".to_string(),
        message: e.to_string(),
        file: Some(source_file.to_path_buf()),
        line: None,
        column: None,
        suggestion: None,
        help: None,
    })?;
    stats.lowering_time_ms += lower_start.elapsed().as_millis();

    Ok((hir, loc))
}

/// Write output files based on configuration
fn write_output(config: &CompilationConfig, assembly: &str) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let output_path = config.output_path_with_extension();
    let output_dir = config.output_path.parent().unwrap_or_else(|| std::path::Path::new("."));

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
            
            let assembler = Assembler::new(output_dir);
            assembler.assemble_to_object(assembly, &output_path)?;
            
            files.push(PathBuf::from(&asm_file));
            files.push(output_path);
        }
        OutputFormat::Executable => {
            let asm_file = format!("{}.s", config.output_path.display());
            fs::write(&asm_file, assembly)
                .map_err(|e| format!("Failed to write assembly file: {}", e))?;
            
            let assembler = Assembler::new(output_dir);
            assembler.compile_to_executable(assembly, &output_path)?;
            
            fs::set_permissions(&output_path, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Failed to set executable permissions: {}", e))?;
            
            files.push(PathBuf::from(&asm_file));
            files.push(output_path);
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
            
            let assembler = Assembler::new(output_dir);
            let obj_file = format!("{}.o", config.output_path.display());
            assembler.assemble_to_object(assembly, &PathBuf::from(&obj_file))?;
            
            let lib_file = format!("{}.a", config.output_path.display());
            create_static_library(&obj_file, &lib_file)?;
            
            files.push(PathBuf::from(&asm_file));
            files.push(PathBuf::from(&obj_file));
            files.push(PathBuf::from(&lib_file));
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

/// Create a static library from object files
fn create_static_library(obj_file: &str, lib_file: &str) -> Result<(), String> {
    let status = Command::new("ar")
        .arg("rcs")
        .arg(lib_file)
        .arg(obj_file)
        .status()
        .map_err(|e| format!("Failed to invoke ar: {}", e))?;

    if !status.success() {
        return Err("Failed to create static library".to_string());
    }

    Ok(())
}