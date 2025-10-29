//! Compilation configuration and file discovery system

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;

/// Output format for compiled code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// x86-64 assembly file (.s)
    Assembly,
    /// ELF object file (.o)
    Object,
    /// Executable binary
    Executable,
    /// Bash/shell script wrapper
    BashScript,
    /// Library (static or dynamic)
    Library,
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Assembly => ".s",
            OutputFormat::Object => ".o",
            OutputFormat::Executable => "",
            OutputFormat::BashScript => ".sh",
            OutputFormat::Library => ".a",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            OutputFormat::Assembly => "x86-64 Assembly",
            OutputFormat::Object => "ELF Object File",
            OutputFormat::Executable => "Executable Binary",
            OutputFormat::BashScript => "Bash Script",
            OutputFormat::Library => "Static Library",
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Configuration for compilation
#[derive(Debug, Clone)]
pub struct CompilationConfig {
    /// Source files to compile
    pub source_files: Vec<PathBuf>,
    /// Library paths for linking
    pub lib_paths: Vec<PathBuf>,
    /// Dependencies/external libraries
    pub libraries: Vec<String>,
    /// Output file path (without extension)
    pub output_path: PathBuf,
    /// Output format
    pub output_format: OutputFormat,
    /// Optimization level (0-3)
    pub opt_level: u32,
    /// Enable verbose output
    pub verbose: bool,
    /// Enable debug info
    pub debug: bool,
    /// Metadata about discovered modules
    pub module_map: HashMap<String, PathBuf>,
}

impl CompilationConfig {
    /// Create a new default configuration
    pub fn new() -> Self {
        CompilationConfig {
            source_files: Vec::new(),
            lib_paths: Vec::new(),
            libraries: Vec::new(),
            output_path: PathBuf::from("output"),
            output_format: OutputFormat::Executable,
            opt_level: 2,
            verbose: false,
            debug: false,
            module_map: HashMap::new(),
        }
    }

    /// Add a source file
    pub fn add_source_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(format!("Source file not found: {}", path.display()));
        }
        if !path.extension().map_or(false, |ext| ext == "rs") {
            return Err(format!("Source file must have .rs extension: {}", path.display()));
        }
        self.source_files.push(path);
        Ok(self)
    }

    /// Discover all .rs files in a directory
    pub fn discover_sources<P: AsRef<Path>>(mut self, dir: P) -> Result<Self, String> {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            return Err(format!("Directory not found: {}", dir.display()));
        }

        self.scan_directory(dir)?;
        Ok(self)
    }

    fn scan_directory(&mut self, dir: &Path) -> Result<(), String> {
        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry.map_err(|e| format!("Error reading directory: {}", e))?;
                    let path = entry.path();

                    if path.is_dir() {
                        // Skip common non-source directories
                        if let Some(name) = path.file_name() {
                            if let Some(name_str) = name.to_str() {
                                if name_str == "target" || name_str == ".git" 
                                    || name_str == "node_modules" || name_str.starts_with('.') {
                                    continue;
                                }
                            }
                        }
                        self.scan_directory(&path)?;
                    } else if let Some(ext) = path.extension() {
                        if ext == "rs" {
                            self.source_files.push(path.clone());
                            
                            // Track module
                            if let Some(file_name) = path.file_stem() {
                                if let Some(name_str) = file_name.to_str() {
                                    self.module_map.insert(name_str.to_string(), path);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return Err(format!("Error reading directory {}: {}", dir.display(), e));
            }
        }
        Ok(())
    }

    /// Set output file path
    pub fn set_output<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.output_path = path.as_ref().to_path_buf();
        self
    }

    /// Set output format
    pub fn set_output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// Add a library path
    pub fn add_lib_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.lib_paths.push(path.as_ref().to_path_buf());
        self
    }

    /// Add a library dependency
    pub fn add_library(mut self, name: String) -> Self {
        self.libraries.push(name);
        self
    }

    /// Set optimization level (0-3)
    pub fn set_opt_level(mut self, level: u32) -> Self {
        self.opt_level = level.min(3);
        self
    }

    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Enable debug info
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Load configuration from a file (Cargo.toml-like format)
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let _path = path.as_ref();
        // TODO: Implement configuration file parsing
        // For now, return default
        Ok(CompilationConfig::new())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.source_files.is_empty() {
            return Err("No source files specified".to_string());
        }

        for file in &self.source_files {
            if !file.exists() {
                return Err(format!("Source file not found: {}", file.display()));
            }
        }

        Ok(())
    }

    /// Get full output path with extension
    pub fn output_path_with_extension(&self) -> PathBuf {
        let mut path = self.output_path.clone();
        let extension = self.output_format.extension();
        if !extension.is_empty() {
            if let Some(file_name) = path.file_name() {
                let file_str = file_name.to_string_lossy();
                let new_name = format!("{}{}", file_str, extension);
                path.set_file_name(new_name);
            }
        }
        path
    }
}

impl Default for CompilationConfig {
    fn default() -> Self {
        Self::new()
    }
}