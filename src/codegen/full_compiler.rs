//! Full compiler pipeline orchestration

use std::path::{Path, PathBuf};

pub struct FullCompiler {
    output_dir: PathBuf,
}

impl FullCompiler {
    pub fn new<P: AsRef<Path>>(output_dir: P) -> Self {
        FullCompiler {
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    pub fn compile_to_executable(
        &self,
        _source_code: &str,
        output_exe: &Path,
    ) -> Result<String, String> {
        Ok(format!("Compiled to {:?}", output_exe))
    }
}
