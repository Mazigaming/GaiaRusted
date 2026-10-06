//! The system tools that finish a build: the assembler turns assembly into
//! an object file, the C compiler driver links executables (it knows where
//! the C runtime and `libgcc` are), and `ar` makes static libraries.

use super::Failure;
use std::collections::hash_map::RandomState;
use std::fs::DirBuilder;
use std::hash::{BuildHasher, Hasher};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

/// A folder for intermediate files, removed with everything in it when
/// dropped.
pub(super) struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    /// A new folder in the temporary directory, readable only by this
    /// user. Its name is unpredictable and it is created fresh, so another
    /// user of a shared temporary directory cannot plant one in advance.
    pub fn new() -> Result<Scratch, Failure> {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let mut builder = DirBuilder::new();
        builder.mode(0o700);
        for _ in 0..16 {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u32(NEXT.fetch_add(1, Ordering::Relaxed));
            let name = format!("gaiarusted-{}-{:016x}", std::process::id(), hasher.finish());
            let dir = std::env::temp_dir().join(name);
            match builder.create(&dir) {
                Ok(()) => return Ok(Scratch { dir }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(Failure::Tool(format!("cannot create `{}`: {e}", dir.display()))),
            }
        }
        Err(Failure::Tool("cannot create a scratch folder in the temporary directory".to_string()))
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Run a tool; on failure, its error output is the message.
fn run(command: &mut Command, tool: &str) -> Result<(), Failure> {
    let output = command.output().map_err(|e| Failure::Tool(format!("cannot run `{tool}`: {e}")))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(Failure::Tool(format!("`{tool}` failed:\n{}", stderr.trim_end())))
}

/// Assemble `assembly` into the object file `object`.
pub(super) fn assemble(assembly: &str, object: &Path) -> Result<(), Failure> {
    let scratch = Scratch::new()?;
    let source = scratch.path("program.s");
    super::write(&source, assembly)?;
    run(Command::new("as").arg("-o").arg(object).arg(&source), "as")
}

/// Link `object` into the executable `output`, with the C libraries asked
/// for, then the C library and the maths library.
pub(super) fn link_executable(object: &Path, output: &Path, paths: &[PathBuf], libraries: &[String]) -> Result<(), Failure> {
    let mut command = Command::new("gcc");
    command.arg("-no-pie").arg(object);
    for path in paths {
        command.arg("-L").arg(path);
    }
    for library in libraries {
        command.arg(format!("-l{library}"));
    }
    command.args(["-lc", "-lm", "-o"]).arg(output);
    run(&mut command, "gcc")
}

/// Make the static library `output` holding `object`. An existing archive
/// is replaced, not added to.
pub(super) fn archive(object: &Path, output: &Path) -> Result<(), Failure> {
    match std::fs::remove_file(output) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Failure::Tool(format!("cannot replace `{}`: {e}", output.display()))),
    }
    run(Command::new("ar").arg("rcs").arg(output).arg(object), "ar")
}

#[cfg(test)]
mod tests {
    use super::Scratch;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn scratch_folders_are_private_and_never_shared() {
        let first = Scratch::new().unwrap();
        let second = Scratch::new().unwrap();
        assert_ne!(first.dir, second.dir);
        let mode = std::fs::metadata(&first.dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700, "{:o}", mode);
    }
}
