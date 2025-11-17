use gaiarusted::config::{CompilationConfig, OutputFormat};

#[test]
fn test_config_builder_default() {
    let config = CompilationConfig::new();
    let _ = config;
}

#[test]
fn test_output_format_assembly() {
    let fmt = OutputFormat::Assembly;
    let _ = fmt;
}

#[test]
fn test_output_format_object() {
    let fmt = OutputFormat::Object;
    let _ = fmt;
}

#[test]
fn test_output_format_executable() {
    let fmt = OutputFormat::Executable;
    let _ = fmt;
}

#[test]
fn test_output_format_library() {
    let fmt = OutputFormat::Library;
    let _ = fmt;
}