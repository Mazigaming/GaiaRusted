use gaiarusted::config::{CompilationConfig, OutputFormat};
use std::path::PathBuf;

#[test]
fn test_config_creation() {
    let config = CompilationConfig::new();
    assert_eq!(config.output_format, OutputFormat::Executable);
    assert_eq!(config.opt_level, 2);
}

#[test]
fn test_config_with_output_path() {
    let mut config = CompilationConfig::new();
    config.output_path = PathBuf::from("/tmp/test");
    assert_eq!(config.output_path, PathBuf::from("/tmp/test"));
}

#[test]
fn test_config_set_verbose() {
    let mut config = CompilationConfig::new();
    config.verbose = true;
    assert!(config.verbose);
}

#[test]
fn test_output_format_assembly_extension() {
    let fmt = OutputFormat::Assembly;
    assert_eq!(fmt.extension(), ".s");
}

#[test]
fn test_output_format_object_extension() {
    let fmt = OutputFormat::Object;
    assert_eq!(fmt.extension(), ".o");
}