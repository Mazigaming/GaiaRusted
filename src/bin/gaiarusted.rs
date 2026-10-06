//! The `gaiarusted` command: compile a Rust program, or the binaries of a
//! Cargo project.

use gaiarusted::driver::{self, project, Failure, Format, Options};
use gaiarusted::pipeline::OptLevel;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = "\
GaiaRusted, a Rust compiler written in Rust

Usage:
    gaiarusted [OPTIONS] FILE.rs [MODULE.rs ...]
    gaiarusted [OPTIONS] FOLDER          a Cargo project, or a folder with a main.rs
    gaiarusted --discover [FOLDER]

Options:
    -o PATH           Where to write the output (default: named after the first file)
    --format FORMAT   exe (the default), asm, obj or lib
    -O LEVEL          0 for no optimisation, 1 to 3 for all of it (default 2); also -O0 .. -O3
    -L PATH           Search PATH for the libraries given with -l
    -l NAME           Link the C library NAME
    --emit-ir         Print the optimised intermediate representation instead of building
    --discover        Build the folder given (default: this one)
    -h, --help        Show this help
    --version         Show the version
";

enum Command {
    Build(Options),
    Project { dir: PathBuf, opt_level: OptLevel },
    EmitIr { sources: Vec<PathBuf>, opt_level: OptLevel },
    Help,
    Version,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match parse(&args) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("error: {message}\n\nFor more information, try `gaiarusted --help`.");
            return ExitCode::from(2);
        }
    };
    match command {
        Command::Help => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Command::Version => {
            println!("gaiarusted {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Command::Build(options) => {
            let name = crate_name(&options.sources[0]);
            finish(driver::build(&options).map(drop), &name)
        }
        Command::Project { dir, opt_level } => {
            let name = project::read_manifest(&dir).map(|manifest| manifest.name).unwrap_or_else(|_| crate_name(&dir));
            finish(project::build_project(&dir, opt_level).map(drop), &name)
        }
        Command::EmitIr { sources, opt_level } => match driver::emit_ir(&sources, opt_level) {
            Ok(ir) => {
                print!("{ir}");
                ExitCode::SUCCESS
            }
            Err(failure) => finish(Err(failure), &crate_name(&sources[0])),
        },
    }
}

/// Report how a build ended.
fn finish(result: Result<(), Failure>, name: &str) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprint!("{}", failure.render());
            if matches!(failure, Failure::Program(_)) {
                eprintln!("error: could not compile `{name}` due to 1 previous error");
            }
            ExitCode::from(1)
        }
    }
}

fn parse(args: &[String]) -> Result<Command, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut output = None;
    let mut format = Format::Executable;
    let mut opt_level = OptLevel::Full;
    let mut link_paths = Vec::new();
    let mut link_libraries = Vec::new();
    let mut emit_ir = false;
    let mut discover = None;

    let value = |index: &mut usize, flag: &str| -> Result<String, String> {
        *index += 1;
        args.get(*index).cloned().ok_or_else(|| format!("`{flag}` needs a value"))
    };
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "-h" | "--help" => return Ok(Command::Help),
            "--version" => return Ok(Command::Version),
            "-o" => output = Some(PathBuf::from(value(&mut index, arg)?)),
            "--format" => {
                let name = value(&mut index, arg)?;
                format = Format::from_name(&name)
                    .ok_or_else(|| format!("unknown format `{name}`: expected exe, asm, obj or lib"))?;
            }
            "-O" => opt_level = opt_level_named(&value(&mut index, arg)?)?,
            "-L" => link_paths.push(PathBuf::from(value(&mut index, arg)?)),
            "-l" => link_libraries.push(value(&mut index, arg)?),
            "--emit-ir" => emit_ir = true,
            "--discover" => {
                discover = Some(match args.get(index + 1) {
                    Some(next) if !next.starts_with('-') => {
                        index += 1;
                        PathBuf::from(next)
                    }
                    _ => PathBuf::from("."),
                });
            }
            _ if arg.len() == 3 && arg.starts_with("-O") => opt_level = opt_level_named(&arg[2..])?,
            _ if arg.starts_with('-') => return Err(format!("unknown option `{arg}`")),
            _ => files.push(PathBuf::from(arg)),
        }
        index += 1;
    }

    // A folder: a Cargo project, or a plain folder with a program in it.
    let folder = match (discover, files.as_slice()) {
        (Some(dir), _) => Some(dir),
        (None, [only]) if only.is_dir() => Some(only.clone()),
        _ => None,
    };
    if let Some(dir) = folder {
        if dir.join("Cargo.toml").is_file() {
            return Ok(Command::Project { dir, opt_level });
        }
        files = vec![project::program_in(&dir).map_err(|failure| failure.render().trim_end().trim_start_matches("error: ").to_string())?];
    }
    if files.is_empty() {
        return Err("no input file: give a `.rs` file or a project folder".to_string());
    }
    if emit_ir {
        return Ok(Command::EmitIr { sources: files, opt_level });
    }
    let output = output.unwrap_or_else(|| default_output(&files[0], format));
    Ok(Command::Build(Options { sources: files, output, format, opt_level, link_paths, link_libraries }))
}

fn opt_level_named(level: &str) -> Result<OptLevel, String> {
    match level {
        "0" => Ok(OptLevel::None),
        "1" | "2" | "3" => Ok(OptLevel::Full),
        _ => Err(format!("unknown optimisation level `{level}`: expected 0, 1, 2 or 3")),
    }
}

/// Where the output goes without `-o`: in the current folder, named after
/// the crate root, as rustc does.
fn default_output(root: &Path, format: Format) -> PathBuf {
    let name = crate_name(root);
    PathBuf::from(match format {
        Format::Executable => name,
        Format::Assembly => format!("{name}.s"),
        Format::Object => format!("{name}.o"),
        Format::Library => format!("lib{name}.a"),
    })
}

fn crate_name(path: &Path) -> String {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    path.file_stem().map_or_else(|| "main".to_string(), |stem| stem.to_string_lossy().into_owned())
}
