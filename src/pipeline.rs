//! The typed compilation pipeline, end to end.
//!
//! ```text
//! source ─▶ syntax ─▶ sema ─▶ ir ─▶ ir::opt ─▶ x64 ─▶ assembly
//! ```
//!
//! The standard library is not a separate binary: its source (the
//! `library/` directory) is embedded in the compiler and compiled together
//! with every program, so only the parts a program uses end up in it.

use crate::ir;
pub use crate::ir::opt::OptLevel;
use crate::sema::{defs, derive, impl_trait, Context};
use crate::syntax::ast::{Ident, Item, ItemKind, Module};
use crate::syntax::parser::{parse_crate, parse_source};
use crate::syntax::{Diagnostic, SourceMap, Span};
use crate::x64;
use std::path::{Path, PathBuf};

/// The standard library: one module per file. A path such as
/// `collections::hash_map` names a submodule; its parent comes first.
const LIBRARY: &[(&str, &str)] = &[
    ("any", include_str!("../library/any.rs")),
    ("array", include_str!("../library/array.rs")),
    ("borrow", include_str!("../library/borrow.rs")),
    ("boxed", include_str!("../library/boxed.rs")),
    ("cell", include_str!("../library/cell.rs")),
    ("char", include_str!("../library/char.rs")),
    ("clone", include_str!("../library/clone.rs")),
    ("cmp", include_str!("../library/cmp.rs")),
    ("collections", include_str!("../library/collections/mod.rs")),
    ("collections::binary_heap", include_str!("../library/collections/binary_heap.rs")),
    ("collections::btree_map", include_str!("../library/collections/btree_map.rs")),
    ("collections::btree_set", include_str!("../library/collections/btree_set.rs")),
    ("collections::hash_map", include_str!("../library/collections/hash_map.rs")),
    ("collections::hash_set", include_str!("../library/collections/hash_set.rs")),
    ("collections::vec_deque", include_str!("../library/collections/vec_deque.rs")),
    ("convert", include_str!("../library/convert.rs")),
    ("default", include_str!("../library/default.rs")),
    ("env", include_str!("../library/env.rs")),
    ("error", include_str!("../library/error.rs")),
    ("f32", include_str!("../library/f32.rs")),
    ("f64", include_str!("../library/f64.rs")),
    ("ffi", include_str!("../library/ffi.rs")),
    ("fmt", include_str!("../library/fmt.rs")),
    ("fs", include_str!("../library/fs.rs")),
    ("hash", include_str!("../library/hash.rs")),
    ("intrinsics", include_str!("../library/intrinsics.rs")),
    ("io", include_str!("../library/io.rs")),
    ("iter", include_str!("../library/iter.rs")),
    ("libc", include_str!("../library/libc.rs")),
    ("marker", include_str!("../library/marker.rs")),
    ("mem", include_str!("../library/mem.rs")),
    ("num", include_str!("../library/num.rs")),
    ("ops", include_str!("../library/ops.rs")),
    ("option", include_str!("../library/option.rs")),
    ("path", include_str!("../library/path.rs")),
    ("prelude", include_str!("../library/prelude.rs")),
    ("panic", include_str!("../library/panic.rs")),
    ("process", include_str!("../library/process.rs")),
    ("ptr", include_str!("../library/ptr.rs")),
    ("rc", include_str!("../library/rc.rs")),
    ("result", include_str!("../library/result.rs")),
    ("rt", include_str!("../library/rt.rs")),
    ("slice", include_str!("../library/slice.rs")),
    ("str", include_str!("../library/str.rs")),
    ("string", include_str!("../library/string.rs")),
    ("sync", include_str!("../library/sync/mod.rs")),
    ("sync::atomic", include_str!("../library/sync/atomic.rs")),
    ("sync::mpsc", include_str!("../library/sync/mpsc.rs")),
    ("thread", include_str!("../library/thread.rs")),
    ("time", include_str!("../library/time.rs")),
    ("unicode", include_str!("../library/unicode.rs")),
    ("vec", include_str!("../library/vec.rs")),
];

/// What to produce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Emit {
    /// x86-64 assembly, ready for the assembler.
    Assembly,
    /// A readable dump of the intermediate representation.
    Ir,
}

/// A failed compilation: the error, and the source text it points into.
pub struct Failure {
    pub diagnostic: Diagnostic,
    pub sources: SourceMap,
}

impl Failure {
    /// The error with its source context, as the compiler prints it.
    pub fn render(&self) -> String {
        self.diagnostic.render(&self.sources)
    }

    pub fn message(&self) -> &str {
        &self.diagnostic.message
    }
}

impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Failure").field("diagnostic", &self.diagnostic).finish_non_exhaustive()
    }
}

/// Compile the crate rooted at `root_file`. Each of `extra_files` becomes a
/// module of the crate named after the file, as if declared with `mod`.
pub fn compile(root_file: &Path, extra_files: &[PathBuf], emit: Emit, level: OptLevel) -> Result<String, Failure> {
    let mut sources = SourceMap::new();
    match run(&mut sources, root_file, extra_files, emit, level) {
        Ok(output) => Ok(output),
        Err(diagnostic) => Err(Failure { diagnostic, sources }),
    }
}

fn module(name: &str, items: Vec<Item>) -> Item {
    let name = Ident { name: name.to_string(), span: Span::default() };
    Item { kind: ItemKind::Mod(Module { name, items }), attrs: Vec::new(), is_pub: true, span: Span::default() }
}

fn run(
    sources: &mut SourceMap,
    root_file: &Path,
    extra_files: &[PathBuf],
    emit: Emit,
    level: OptLevel,
) -> Result<String, Diagnostic> {
    let mut library: Vec<Item> = Vec::new();
    for (path, text) in LIBRARY {
        let file = format!("<std>/{}.rs", path.replace("::", "/"));
        let items = parse_source(sources, &file, text)?;
        let mut parents = path.split("::").collect::<Vec<_>>();
        let name = parents.pop().unwrap_or(path);
        let mut siblings = &mut library;
        for parent in parents {
            siblings = match siblings.iter_mut().find_map(|item| match &mut item.kind {
                ItemKind::Mod(m) if m.name.name == parent => Some(m),
                _ => None,
            }) {
                Some(m) => &mut m.items,
                None => unreachable!("library module `{path}` is listed before its parent"),
            };
        }
        siblings.push(module(name, items));
    }

    let mut program = parse_crate(sources, root_file)?;
    for file in extra_files {
        let name = file.file_stem().and_then(|stem| stem.to_str()).unwrap_or("module");
        let declared = program.iter().any(|item| matches!(&item.kind, ItemKind::Mod(m) if m.name.name == name));
        if !declared {
            let items = parse_crate(sources, file)?;
            program.push(module(name, items));
        }
    }

    derive::expand(&mut library, sources)?;
    derive::expand(&mut program, sources)?;
    impl_trait::desugar(&mut library);
    impl_trait::desugar(&mut program);

    let mut defs = defs::collect(&library, &program)?;
    if let Some(stem) = root_file.file_stem().and_then(|stem| stem.to_str()) {
        defs.crate_name = stem.replace('-', "_");
    }
    let tcx = Context::new(defs, sources);
    let mut ir = ir::build::build(&tcx)?;
    ir::opt::optimize(&mut ir, &tcx, level);
    Ok(match emit {
        Emit::Assembly => x64::emit(&ir, &tcx, level),
        Emit::Ir => ir::display::dump(&ir, &tcx),
    })
}
