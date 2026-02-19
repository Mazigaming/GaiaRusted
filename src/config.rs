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
     /// Crate name (from Gaia.toml or Cargo.toml)
     pub crate_name: String,
     /// Crate version (from manifest)
     pub crate_version: String,
     /// Is this a library crate (lib.rs exists) or binary (main.rs exists)
     pub is_library: bool,
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
            crate_name: "unknown".to_string(),
            crate_version: "0.0.0".to_string(),
            is_library: false,
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

    /// Load configuration from a Gaia.toml file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Ok(CompilationConfig::new()); // No config file, use defaults
        }

        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read Gaia.toml: {}", e))?;

        Self::parse_toml(&contents)
    }

    /// Parse TOML configuration
    fn parse_toml(content: &str) -> Result<Self, String> {
        let mut config = CompilationConfig::new();
        let mut current_section = String::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse sections [package], [build], [dependencies]
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].to_string();
                continue;
            }

            // Parse key = value pairs
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim();

                match current_section.as_str() {
                    "package" => {
                        match key {
                            "name" => {
                                // Extract string value (remove quotes)
                                let name = value.trim_matches(|c| c == '"' || c == '\'');
                                config.crate_name = name.to_string();
                                config.output_path = PathBuf::from(name);
                            }
                            "version" => {
                                // Extract version string (remove quotes)
                                let version = value.trim_matches(|c| c == '"' || c == '\'');
                                config.crate_version = version.to_string();
                            }
                            _ => {}
                        }
                    }
                    "build" => {
                        match key {
                            "source" => {
                                let src = value.trim_matches(|c| c == '"' || c == '\'');
                                config.source_files.push(PathBuf::from(src));
                            }
                            "output" => {
                                let out = value.trim_matches(|c| c == '"' || c == '\'');
                                config.output_path = PathBuf::from(out);
                            }
                            "opt-level" => {
                                if let Ok(level) = value.parse::<u32>() {
                                    config.opt_level = level.min(3);
                                }
                            }
                            "debug" => {
                                config.debug = value.trim_matches(|c| c == '"' || c == '\'')
                                    .eq_ignore_ascii_case("true");
                            }
                            "lib-paths" => {
                                let paths = value.trim_matches(|c| c == '"' || c == '[' || c == ']' || c == ' ');
                                for path in paths.split(',') {
                                    let p = path.trim().trim_matches(|c| c == '"' || c == '\'');
                                    if !p.is_empty() {
                                        config.lib_paths.push(PathBuf::from(p));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    "dependencies" => {
                        // Store dependency name and version
                        let dep_version = value.trim_matches(|c| c == '"' || c == '\'');
                        config.libraries.push(format!("{} = {}", key, dep_version));
                    }
                    _ => {}
                }
            }
        }

        Ok(config)
    }
    
    /// Detect crate type (library vs binary) from source files
    pub fn detect_crate_type(mut self) -> Self {
        // Check if lib.rs exists
        let has_lib = self.source_files.iter()
            .any(|f| f.file_name().and_then(|n| n.to_str()) == Some("lib.rs"));
        
        // Check if main.rs exists
        let has_main = self.source_files.iter()
            .any(|f| f.file_name().and_then(|n| n.to_str()) == Some("main.rs"));
        
        // Library has priority if both exist
        self.is_library = has_lib;
        
        // Set output format based on crate type
        if !self.is_library {
            self.output_format = OutputFormat::Executable;
        }
        
        self
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

/// Builder pattern for creating CompilationConfig
#[derive(Debug, Clone)]
pub struct CompilerBuilder {
    verbose: bool,
    optimize: bool,
    output_format: String,
    output_path: PathBuf,
}

impl CompilerBuilder {
    /// Create a new builder with defaults
    pub fn new() -> Self {
        CompilerBuilder {
            verbose: false,
            optimize: false,
            output_format: "executable".to_string(),
            output_path: PathBuf::from("output"),
        }
    }

    /// Enable or disable verbose output
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Enable or disable optimization
    pub fn optimize(mut self, optimize: bool) -> Self {
        self.optimize = optimize;
        self
    }

    /// Set output format
    pub fn format(mut self, format: &str) -> Self {
        self.output_format = format.to_string();
        self
    }

    /// Set output path
    pub fn output(mut self, path: &str) -> Self {
        self.output_path = PathBuf::from(path);
        self
    }

    /// Build the configuration
    pub fn build(self) -> CompilerBuilderConfig {
        CompilerBuilderConfig {
            source_files: Vec::new(),
            is_verbose: self.verbose,
            is_optimized: self.optimize,
            output_format: self.output_format,
            output_path: self.output_path,
        }
    }
}

impl Default for CompilerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration built by CompilerBuilder
#[derive(Debug, Clone)]
pub struct CompilerBuilderConfig {
    pub source_files: Vec<PathBuf>,
    pub is_verbose: bool,
    pub is_optimized: bool,
    pub output_format: String,
    pub output_path: PathBuf,
}

impl CompilerBuilderConfig {
    /// Check if verbose output is enabled
    pub fn is_verbose(&self) -> bool {
        self.is_verbose
    }

    /// Check if optimization is enabled
    pub fn is_optimized(&self) -> bool {
        self.is_optimized
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        match self.output_format.as_str() {
            "assembly" | "object" | "executable" | "library" => {
                if self.source_files.is_empty() {
                    Err("No source files specified".to_string())
                } else {
                    Ok(())
                }
            }
            _ => Err(format!("Invalid output format: {}", self.output_format)),
        }
    }
}

/// Compilation metrics tracking
#[derive(Debug, Clone, Default)]
pub struct CompilationMetrics {
    pub total_time_ms: u64,
    pub lexer_time_ms: u64,
    pub parser_time_ms: u64,
    pub typechecker_time_ms: u64,
    pub codegen_time_ms: u64,
}

impl CompilationMetrics {
    /// Get phase breakdown as percentages
    pub fn phase_breakdown(&self) -> std::collections::HashMap<String, f64> {
        let mut breakdown = std::collections::HashMap::new();
        if self.total_time_ms > 0 {
            breakdown.insert("lexer".to_string(), (self.lexer_time_ms as f64 / self.total_time_ms as f64) * 100.0);
            breakdown.insert("parser".to_string(), (self.parser_time_ms as f64 / self.total_time_ms as f64) * 100.0);
            breakdown.insert("typechecker".to_string(), (self.typechecker_time_ms as f64 / self.total_time_ms as f64) * 100.0);
            breakdown.insert("codegen".to_string(), (self.codegen_time_ms as f64 / self.total_time_ms as f64) * 100.0);
        }
        breakdown
    }

    /// Find the slowest compilation phase
    pub fn slowest_phase(&self) -> Option<(String, u64)> {
        let phases = vec![
            ("lexer", self.lexer_time_ms),
            ("parser", self.parser_time_ms),
            ("typechecker", self.typechecker_time_ms),
            ("codegen", self.codegen_time_ms),
        ];
        phases.into_iter().max_by_key(|(_, time)| *time).map(|(name, time)| (name.to_string(), time))
    }
}

/// Default handler for compilation
#[derive(Debug, Clone, Copy)]
pub struct DefaultHandler;

impl DefaultHandler {
    /// Create a new default handler
    pub fn new() -> Self {
        DefaultHandler
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_gaia_toml() {
        let toml = r#"
[package]
name = "my_project"
version = "0.1.0"

[build]
source = "src/main.rs"
output = "my_app"
opt-level = 2
debug = false
"#;
        let config = CompilationConfig::parse_toml(toml).unwrap();
        assert_eq!(config.output_path.to_string_lossy(), "my_app");
        assert_eq!(config.opt_level, 2);
        assert!(!config.debug);
    }

    #[test]
    fn test_parse_gaia_toml_with_dependencies() {
        let toml = r#"
[package]
name = "my_app"

[dependencies]
serde = "1.0.0"
tokio = "1.0.0"
"#;
        let config = CompilationConfig::parse_toml(toml).unwrap();
        assert_eq!(config.libraries.len(), 2);
        assert!(config.libraries.iter().any(|l| l.contains("serde")));
        assert!(config.libraries.iter().any(|l| l.contains("tokio")));
    }

    #[test]
    fn test_parse_gaia_toml_with_lib_paths() {
        let toml = r#"
[build]
lib-paths = "/usr/lib, /usr/local/lib"
"#;
        let config = CompilationConfig::parse_toml(toml).unwrap();
        assert!(config.lib_paths.len() > 0);
    }

    #[test]
    fn test_compilation_config_validation() {
        let config = CompilationConfig::new();
        // Empty config should fail validation
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_output_format_extensions() {
        assert_eq!(OutputFormat::Assembly.extension(), ".s");
        assert_eq!(OutputFormat::Object.extension(), ".o");
        assert_eq!(OutputFormat::Executable.extension(), "");
        assert_eq!(OutputFormat::Library.extension(), ".a");
    }

    #[test]
    fn test_builder_pattern() {
        let config = CompilationConfig::new()
            .with_verbose(true)
            .with_debug(true)
            .set_opt_level(3);
        
        assert!(config.verbose);
        assert!(config.debug);
        assert_eq!(config.opt_level, 3);
    }
}