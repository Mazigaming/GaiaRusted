//! Assembler and linker integration
//!
//! Takes x86-64 assembly and produces executable binaries using
//! system tools (as, ld) or embedded assembler/linker.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Assembler {
    output_dir: PathBuf,
}

impl Assembler {
    pub fn new<P: AsRef<Path>>(output_dir: P) -> Self {
        Assembler {
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    /// Assemble x86-64 assembly to an object file using GNU as
    pub fn assemble_to_object(&self, assembly: &str, output_obj: &Path) -> Result<(), String> {
        // Write assembly to temporary file
        let asm_file = self.output_dir.join("temp.s");
        fs::write(&asm_file, assembly).map_err(|e| format!("Failed to write assembly: {}", e))?;

        // Invoke GNU as (assembler)
        let status = Command::new("as")
            .arg("-o")
            .arg(output_obj)
            .arg(&asm_file)
            .status()
            .map_err(|e| format!("Failed to invoke assembler (as): {}", e))?;

        if !status.success() {
            return Err("Assembler failed".to_string());
        }

        // Cleanup
        let _ = fs::remove_file(&asm_file);
        Ok(())
    }

    /// Link object files into an executable. The C compiler driver does
    /// it: it knows where the C runtime's start-up files are, and adds the
    /// support library (`libgcc`) that holds routines such as 128-bit
    /// division.
    pub fn link_executable(
        &self,
        object_files: &[&Path],
        output_exe: &Path,
        libraries: &[&str],
    ) -> Result<(), String> {
        let mut cmd = Command::new("gcc");
        cmd.arg("-no-pie");
        for obj_file in object_files {
            cmd.arg(obj_file);
        }
        for lib in libraries {
            cmd.arg(format!("-l{}", lib));
        }
        cmd.arg("-lc").arg("-lm").arg("-o").arg(output_exe);

        let output = cmd.output().map_err(|e| format!("Failed to invoke the linker (gcc): {}", e))?;
        if !output.status.success() {
            return Err(format!("Linker failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
        }
        Ok(())
    }

    /// Complete compilation pipeline: assembly → object → executable
    pub fn compile_to_executable(&self, assembly: &str, output_exe: &Path) -> Result<(), String> {
        let obj_file = self.output_dir.join("output.o");

        // Assemble
        self.assemble_to_object(assembly, &obj_file)?;

        // Link
        self.link_executable(&[&obj_file], output_exe, &[])?;

        // Cleanup
        let _ = fs::remove_file(&obj_file);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assembler_creation() {
        let asm = Assembler::new("/tmp");
        assert_eq!(asm.output_dir, PathBuf::from("/tmp"));
    }
}
