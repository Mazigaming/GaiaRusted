//! Builds leave no scratch folders behind. Alone in its test binary, so no
//! other build of this process is in flight while it looks.

use gaiarusted::driver::{self, Format, Options};
use std::path::Path;

#[test]
fn no_scratch_folders_are_left_behind() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("scratch");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.rs"), "fn main() { println!(\"hello\"); }\n").unwrap();
    for (format, name) in [(Format::Executable, "hello"), (Format::Library, "libhello.a"), (Format::Object, "hello.o")] {
        let mut options = Options::executable(dir.join("main.rs"), dir.join(name));
        options.format = format;
        driver::build(&options).unwrap();
    }
    let prefix = format!("gaiarusted-{}-", std::process::id());
    let left: Vec<_> = std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
        .map(|entry| entry.path())
        .collect();
    assert!(left.is_empty(), "left behind: {left:?}");
}
