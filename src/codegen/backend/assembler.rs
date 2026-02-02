//! Assembler and linker integration
//!
//! Takes x86-64 assembly and produces executable binaries using
//! system tools (as, ld) or embedded assembler/linker.

use std::process::Command;
use std::fs;
use std::path::{Path, PathBuf};

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
    pub fn assemble_to_object(
        &self,
        assembly: &str,
        output_obj: &Path,
    ) -> Result<(), String> {
        // Write assembly to temporary file
        let asm_file = self.output_dir.join("temp.s");
        fs::write(&asm_file, assembly)
            .map_err(|e| format!("Failed to write assembly: {}", e))?;

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

    /// Link object files into an executable
    pub fn link_executable(
        &self,
        object_files: &[&Path],
        output_exe: &Path,
        libraries: &[&str],
    ) -> Result<(), String> {
        let mut cmd = Command::new("ld");

        // Add standard C runtime
        cmd.arg("-dynamic-linker")
            .arg("/lib64/ld-linux-x86-64.so.2");

        // Add object files
        for obj_file in object_files {
            cmd.arg(obj_file);
        }

        // Add CRT startup files
        if std::path::Path::new("/usr/lib/crt1.o").exists() {
            cmd.arg("/usr/lib/crt1.o");
        } else if std::path::Path::new("/usr/lib/x86_64-linux-gnu/crt1.o").exists() {
            cmd.arg("/usr/lib/x86_64-linux-gnu/crt1.o");
        }

        if std::path::Path::new("/usr/lib/crti.o").exists() {
            cmd.arg("/usr/lib/crti.o");
        } else if std::path::Path::new("/usr/lib/x86_64-linux-gnu/crti.o").exists() {
            cmd.arg("/usr/lib/x86_64-linux-gnu/crti.o");
        }

        // Add libraries
        for lib in libraries {
            cmd.arg(format!("-l{}", lib));
        }

        // Add library paths
        cmd.arg("-L/lib")
            .arg("-L/usr/lib")
            .arg("-L/lib/x86_64-linux-gnu")
            .arg("-L/usr/lib/x86_64-linux-gnu")
            .arg("-L/usr/lib64");

        // Link with C library and math library
        cmd.arg("-lc");
        cmd.arg("-lm");  // Math library for sin, cos, pow, sqrt, etc.

        // Add CRT terminator files
        if std::path::Path::new("/usr/lib/crtn.o").exists() {
            cmd.arg("/usr/lib/crtn.o");
        } else if std::path::Path::new("/usr/lib/x86_64-linux-gnu/crtn.o").exists() {
            cmd.arg("/usr/lib/x86_64-linux-gnu/crtn.o");
        }

        cmd.arg("-o").arg(output_exe);

        let status = cmd.status()
            .map_err(|e| format!("Failed to invoke linker (ld): {}", e))?;

        if !status.success() {
            return Err("Linker failed".to_string());
        }

        Ok(())
    }

    /// Complete compilation pipeline: assembly → object → executable
    pub fn compile_to_executable(
        &self,
        assembly: &str,
        output_exe: &Path,
    ) -> Result<(), String> {
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
