//! Cargo projects: building the binaries a `Cargo.toml` describes.
//!
//! The manifest is read as far as building needs: the package's name, its
//! `[[bin]]` targets and whether it has dependencies, which cannot be built
//! yet. Binaries are also found where Cargo finds them by itself:
//! `src/main.rs`, named after the package, and each `src/bin/NAME.rs`.

use super::{build, Failure, Options};
use crate::pipeline::OptLevel;
use std::path::{Path, PathBuf};

/// What a `Cargo.toml` says.
#[derive(Debug, Default)]
pub struct Manifest {
    pub name: String,
    /// The `[[bin]]` entries, in order.
    pub binaries: Vec<Target>,
    /// The names in `[dependencies]` and `[dependencies.NAME]` tables.
    pub dependencies: Vec<String>,
}

/// One binary: its name and its crate root, relative to the project folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub name: String,
    pub path: PathBuf,
}

/// The table a manifest line belongs to.
enum Table {
    Package,
    Bin,
    Dependencies,
    Other,
}

pub fn read_manifest(dir: &Path) -> Result<Manifest, Failure> {
    let path = dir.join("Cargo.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| Failure::Project(format!("cannot read `{}`: {e}", path.display())))?;
    parse_manifest(&text)
}

/// Read a manifest's text. Only `key = "string"` values matter; other
/// values (inline tables, arrays, numbers) are passed over.
pub fn parse_manifest(text: &str) -> Result<Manifest, Failure> {
    let mut manifest = Manifest::default();
    let mut table = Table::Other;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(header) = line.strip_prefix("[[").and_then(|rest| rest.split("]]").next()) {
            table = match header.trim() {
                "bin" => {
                    manifest.binaries.push(Target { name: String::new(), path: PathBuf::new() });
                    Table::Bin
                }
                _ => Table::Other,
            };
            continue;
        }
        if let Some(header) = line.strip_prefix('[').and_then(|rest| rest.split(']').next()) {
            let header = header.trim();
            table = match header {
                "package" => Table::Package,
                "dependencies" => Table::Dependencies,
                _ => match header.strip_prefix("dependencies.") {
                    Some(name) => {
                        manifest.dependencies.push(name.trim().to_string());
                        Table::Other
                    }
                    None => Table::Other,
                },
            };
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        match table {
            Table::Package if key == "name" => manifest.name = string_value(value).unwrap_or_default(),
            Table::Bin => {
                let target = manifest.binaries.last_mut().expect("a [[bin]] header comes first");
                match key {
                    "name" => target.name = string_value(value).unwrap_or_default(),
                    "path" => target.path = PathBuf::from(string_value(value).unwrap_or_default()),
                    _ => {}
                }
            }
            Table::Dependencies => manifest.dependencies.push(key.to_string()),
            _ => {}
        }
    }
    if manifest.name.is_empty() {
        return Err(Failure::Project("`Cargo.toml` has no `[package]` name".to_string()));
    }
    Ok(manifest)
}

/// The text of a `"string"` value, ignoring what follows it.
fn string_value(value: &str) -> Option<String> {
    let rest = value.trim().strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Every binary of the project: `src/main.rs`, then the `[[bin]]` entries,
/// then the rest of `src/bin/*.rs`. A file an entry names is built once,
/// under the entry's name.
pub fn binaries(dir: &Path, manifest: &Manifest) -> Result<Vec<Target>, Failure> {
    let declared: Vec<Target> = manifest
        .binaries
        .iter()
        .map(|target| {
            let path = if target.path.as_os_str().is_empty() {
                PathBuf::from(format!("src/bin/{}.rs", target.name))
            } else {
                target.path.clone()
            };
            Target { name: target.name.clone(), path }
        })
        .collect();
    let is_declared = |path: &Path| declared.iter().any(|target| target.path == path);

    let mut targets = Vec::new();
    let main = Path::new("src/main.rs");
    if dir.join(main).is_file() && !is_declared(main) {
        targets.push(Target { name: manifest.name.clone(), path: main.to_path_buf() });
    }
    targets.extend(declared.iter().cloned());
    if let Ok(entries) = std::fs::read_dir(dir.join("src/bin")) {
        let mut files: Vec<PathBuf> = entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
        files.sort();
        for file in files.iter().filter(|file| file.extension().is_some_and(|extension| extension == "rs")) {
            let path = PathBuf::from("src/bin").join(file.file_name().unwrap());
            if !is_declared(&path) {
                let name = file.file_stem().unwrap().to_string_lossy().into_owned();
                targets.push(Target { name, path });
            }
        }
    }
    Ok(targets)
}

/// Build every binary of the Cargo project in `dir` into
/// `dir/target/gaiarusted/`. Returns the executables, in target order.
pub fn build_project(dir: &Path, opt_level: OptLevel) -> Result<Vec<PathBuf>, Failure> {
    let manifest = read_manifest(dir)?;
    if let Some(first) = manifest.dependencies.first() {
        return Err(Failure::Project(format!(
            "dependencies are not supported yet: `{}` depends on `{first}`",
            manifest.name
        )));
    }
    let targets = binaries(dir, &manifest)?;
    if targets.is_empty() {
        let message = if dir.join("src/lib.rs").is_file() {
            format!("`{}` is a library: library crates cannot be built yet, only binaries", manifest.name)
        } else {
            format!("`{}` has no binary to build (no `src/main.rs`, `src/bin/*.rs` or `[[bin]]`)", manifest.name)
        };
        return Err(Failure::Project(message));
    }
    let out = dir.join("target").join("gaiarusted");
    std::fs::create_dir_all(&out).map_err(|e| Failure::Tool(format!("cannot create `{}`: {e}", out.display())))?;
    let mut built = Vec::new();
    for target in targets {
        let mut options = Options::executable(dir.join(&target.path), out.join(&target.name));
        options.opt_level = opt_level;
        built.extend(build(&options)?);
    }
    Ok(built)
}

/// The crate root of a folder that is not a Cargo project: its `main.rs`,
/// else its only `.rs` file.
pub fn program_in(dir: &Path) -> Result<PathBuf, Failure> {
    let main = dir.join("main.rs");
    if main.is_file() {
        return Ok(main);
    }
    let entries = std::fs::read_dir(dir).map_err(|e| Failure::Project(format!("cannot read `{}`: {e}", dir.display())))?;
    let sources: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect();
    match sources.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(Failure::Project(format!("`{}` has no Cargo.toml and no `.rs` file", dir.display()))),
        _ => Err(Failure::Project(format!(
            "`{}` has several `.rs` files and no `main.rs`: name the one to build",
            dir.display()
        ))),
    }
}
