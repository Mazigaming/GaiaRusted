//! The driver: from source files to the file a user asked for.
//!
//! [`build`] runs the compiler ([`crate::pipeline`]) on a crate and writes
//! its assembly, an object file, an executable or a static library;
//! [`project`] builds the binaries of a Cargo project.

mod link;
pub mod project;

use crate::pipeline::{self, Emit, OptLevel};
use std::path::{Path, PathBuf};

/// What a build produces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Assembly,
    Object,
    Executable,
    Library,
}

impl Format {
    /// The format a `--format` value names.
    pub fn from_name(name: &str) -> Option<Format> {
        match name {
            "asm" | "assembly" => Some(Format::Assembly),
            "obj" | "object" => Some(Format::Object),
            "exe" | "executable" => Some(Format::Executable),
            "lib" | "library" => Some(Format::Library),
            _ => None,
        }
    }
}

/// What to build, and how.
#[derive(Clone, Debug)]
pub struct Options {
    /// The crate root first; each other file becomes a module named after it.
    pub sources: Vec<PathBuf>,
    pub output: PathBuf,
    pub format: Format,
    pub opt_level: OptLevel,
    /// Folders to search for the C libraries in `link_libraries`.
    pub link_paths: Vec<PathBuf>,
    pub link_libraries: Vec<String>,
}

impl Options {
    /// An optimised executable at `output` from the crate rooted at `root`.
    pub fn executable(root: impl Into<PathBuf>, output: impl Into<PathBuf>) -> Options {
        Options {
            sources: vec![root.into()],
            output: output.into(),
            format: Format::Executable,
            opt_level: OptLevel::Full,
            link_paths: Vec::new(),
            link_libraries: Vec::new(),
        }
    }
}

/// Why a build failed.
#[derive(Debug)]
pub enum Failure {
    /// The program has an error.
    Program(pipeline::Failure),
    /// A Cargo project cannot be built as it is described.
    Project(String),
    /// A file could not be read or written, or a tool (`as`, `gcc`, `ar`)
    /// is missing or failed.
    Tool(String),
}

impl Failure {
    /// The failure as the compiler prints it, ending with a newline.
    pub fn render(&self) -> String {
        match self {
            Failure::Program(failure) => {
                let rendered = failure.render();
                if rendered.ends_with('\n') { rendered } else { rendered + "\n" }
            }
            Failure::Project(message) | Failure::Tool(message) => format!("error: {message}\n"),
        }
    }
}

/// Build what `options` ask for. Returns the files written.
pub fn build(options: &Options) -> Result<Vec<PathBuf>, Failure> {
    let Some((root, modules)) = options.sources.split_first() else {
        return Err(Failure::Tool("no source files given".to_string()));
    };
    let assembly = pipeline::compile(root, modules, Emit::Assembly, options.opt_level).map_err(Failure::Program)?;
    let output = &options.output;
    match options.format {
        Format::Assembly => write(output, &assembly)?,
        Format::Object => link::assemble(&assembly, output)?,
        Format::Executable => {
            let scratch = link::Scratch::new()?;
            let object = scratch.path("program.o");
            link::assemble(&assembly, &object)?;
            link::link_executable(&object, output, &options.link_paths, &options.link_libraries)?;
        }
        Format::Library => {
            let scratch = link::Scratch::new()?;
            let object = scratch.path("program.o");
            link::assemble(&assembly, &object)?;
            link::archive(&object, output)?;
        }
    }
    Ok(vec![output.clone()])
}

/// The optimised intermediate representation of a crate, as text.
pub fn emit_ir(sources: &[PathBuf], opt_level: OptLevel) -> Result<String, Failure> {
    let Some((root, modules)) = sources.split_first() else {
        return Err(Failure::Tool("no source files given".to_string()));
    };
    pipeline::compile(root, modules, Emit::Ir, opt_level).map_err(Failure::Program)
}

fn write(path: &Path, contents: &str) -> Result<(), Failure> {
    std::fs::write(path, contents).map_err(|e| Failure::Tool(format!("cannot write `{}`: {e}", path.display())))
}
