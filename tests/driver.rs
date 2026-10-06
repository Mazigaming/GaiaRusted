//! The driver: building programs into each output format, linking C
//! libraries, and Cargo projects.

use gaiarusted::driver::{self, Failure, Format, Options};
use gaiarusted::pipeline::OptLevel;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A fresh, empty folder for one test.
fn workspace(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("driver").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn run(executable: &Path) -> String {
    let output = Command::new(executable).output().expect("the program runs");
    assert!(output.status.success(), "{} failed", executable.display());
    String::from_utf8(output.stdout).unwrap()
}

const HELLO: &str = "fn main() { println!(\"hello {}\", 6 * 7); }\n";

fn options(dir: &Path, format: Format, output: &str) -> Options {
    write(&dir.join("main.rs"), HELLO);
    let mut options = Options::executable(dir.join("main.rs"), dir.join(output));
    options.format = format;
    options
}

#[test]
fn each_format_produces_its_file_and_nothing_else() {
    for (format, name) in [(Format::Executable, "hello"), (Format::Assembly, "hello.s"), (Format::Object, "hello.o"), (Format::Library, "libhello.a")] {
        let dir = workspace(&format!("format-{name}"));
        let built = driver::build(&options(&dir, format, name)).unwrap();
        assert_eq!(built, vec![dir.join(name)]);
        let mut entries: Vec<String> =
            std::fs::read_dir(&dir).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect();
        entries.sort();
        let mut expected = vec!["main.rs".to_string(), name.to_string()];
        expected.sort();
        assert_eq!(entries, expected, "{name}");
    }
    let dir = workspace("format-run");
    driver::build(&options(&dir, Format::Executable, "hello")).unwrap();
    assert_eq!(run(&dir.join("hello")), "hello 42\n");
}

#[test]
fn a_program_error_is_a_program_failure() {
    let dir = workspace("program-error");
    write(&dir.join("main.rs"), "fn main() {\n    let x: i32 = \"text\";\n}\n");
    let failure = driver::build(&Options::executable(dir.join("main.rs"), dir.join("out"))).unwrap_err();
    let Failure::Program(program) = &failure else { panic!("expected a program failure, got {failure:?}") };
    assert!(program.message().contains("mismatched types"), "{}", program.message());
    assert!(failure.render().contains("main.rs:2"), "{}", failure.render());
    assert!(!dir.join("out").exists());
}

#[test]
fn links_a_c_library() {
    let dir = workspace("c-library");
    write(&dir.join("adder.c"), "int add(int a, int b) { return a + b; }\n");
    let status = Command::new("cc").args(["-c", "adder.c", "-o", "adder.o"]).current_dir(&dir).status().unwrap();
    assert!(status.success());
    let status = Command::new("ar").args(["rcs", "libadder.a", "adder.o"]).current_dir(&dir).status().unwrap();
    assert!(status.success());
    write(
        &dir.join("main.rs"),
        "extern \"C\" { fn add(a: i32, b: i32) -> i32; }\nfn main() { println!(\"{}\", unsafe { add(2, 3) }); }\n",
    );
    let mut options = Options::executable(dir.join("main.rs"), dir.join("sum"));
    options.link_paths.push(dir.clone());
    options.link_libraries.push("adder".to_string());
    driver::build(&options).unwrap();
    assert_eq!(run(&dir.join("sum")), "5\n");
}

#[test]
fn missing_c_library_reports_the_linker_error() {
    let dir = workspace("missing-library");
    let mut options = options(&dir, Format::Executable, "hello");
    options.link_libraries.push("no_such_library_anywhere".to_string());
    let failure = driver::build(&options).unwrap_err();
    assert!(matches!(failure, Failure::Tool(_)), "{failure:?}");
    assert!(failure.render().contains("no_such_library_anywhere"), "{}", failure.render());
}

#[test]
fn output_into_missing_folder_is_an_error() {
    let dir = workspace("missing-folder");
    let options = options(&dir, Format::Assembly, "no/such/folder/hello.s");
    let failure = driver::build(&options).unwrap_err();
    assert!(matches!(failure, Failure::Tool(_)), "{failure:?}");
    assert!(failure.render().contains("no/such/folder/hello.s"), "{}", failure.render());
}

#[test]
fn rebuilding_a_library_replaces_it() {
    let dir = workspace("rebuild-library");
    let options = options(&dir, Format::Library, "libhello.a");
    driver::build(&options).unwrap();
    driver::build(&options).unwrap();
    let listing = Command::new("ar").arg("t").arg(dir.join("libhello.a")).output().unwrap();
    assert_eq!(String::from_utf8(listing.stdout).unwrap().lines().count(), 1);
}

#[test]
fn emits_ir() {
    let dir = workspace("emit-ir");
    write(&dir.join("main.rs"), HELLO);
    let ir = driver::emit_ir(&[dir.join("main.rs")], OptLevel::Full).unwrap();
    assert!(ir.contains("fn main"), "{ir}");
}

use gaiarusted::driver::project;

const MANIFEST: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";

#[test]
fn builds_a_project_with_modules_and_two_binaries() {
    let dir = workspace("project");
    write(
        &dir.join("Cargo.toml"),
        &format!("{MANIFEST}\n[[bin]]\nname = \"other\"\npath = \"src/other.rs\"\n"),
    );
    write(&dir.join("src/main.rs"), "mod greet;\nfn main() { println!(\"{}\", greet::hello()); }\n");
    write(&dir.join("src/greet.rs"), "pub fn hello() -> String { String::from(\"hi from a module\") }\n");
    write(&dir.join("src/other.rs"), "fn main() { println!(\"other\"); }\n");
    let built = project::build_project(&dir, OptLevel::Full).unwrap();
    let out = dir.join("target/gaiarusted");
    assert_eq!(built, vec![out.join("demo"), out.join("other")]);
    assert_eq!(run(&out.join("demo")), "hi from a module\n");
    assert_eq!(run(&out.join("other")), "other\n");
}

#[test]
fn dependencies_are_rejected() {
    let dir = workspace("project-dependencies");
    write(&dir.join("Cargo.toml"), &format!("{MANIFEST}\n[dependencies]\nrand = \"0.8\"\n"));
    write(&dir.join("src/main.rs"), HELLO);
    let failure = project::build_project(&dir, OptLevel::Full).unwrap_err();
    assert!(matches!(failure, Failure::Project(_)), "{failure:?}");
    assert!(failure.render().contains("dependencies are not supported yet"), "{}", failure.render());
    assert!(failure.render().contains("rand"), "{}", failure.render());
}

#[test]
fn manifest_forms() {
    let text = "# A comment\n[package]\nname = \"forms\" # trailing comment\nversion = \"1.0.0\"\n\n\
                [dev-dependencies]\ntest-helper = \"1\"\n\n\
                [dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\n\n\
                [dependencies.tokio]\nversion = \"1\"\n";
    let manifest = project::parse_manifest(text).unwrap();
    assert_eq!(manifest.name, "forms");
    assert_eq!(manifest.dependencies, vec!["serde".to_string(), "tokio".to_string()]);
    assert!(manifest.binaries.is_empty());
}

#[test]
fn binaries_in_src_bin_are_found() {
    let dir = workspace("project-src-bin");
    write(&dir.join("Cargo.toml"), MANIFEST);
    write(&dir.join("src/bin/tool.rs"), "fn main() { println!(\"tool\"); }\n");
    let built = project::build_project(&dir, OptLevel::Full).unwrap();
    assert_eq!(built, vec![dir.join("target/gaiarusted/tool")]);
    assert_eq!(run(&built[0]), "tool\n");
}

#[test]
fn library_only_project_is_reported() {
    let dir = workspace("project-library");
    write(&dir.join("Cargo.toml"), MANIFEST);
    write(&dir.join("src/lib.rs"), "pub fn f() {}\n");
    let failure = project::build_project(&dir, OptLevel::Full).unwrap_err();
    assert!(failure.render().contains("library crates cannot be built yet"), "{}", failure.render());
}

#[test]
fn finds_the_program_in_a_plain_folder() {
    let dir = workspace("plain-folder");
    write(&dir.join("only.rs"), HELLO);
    assert_eq!(project::program_in(&dir).unwrap(), dir.join("only.rs"));
    write(&dir.join("main.rs"), HELLO);
    assert_eq!(project::program_in(&dir).unwrap(), dir.join("main.rs"));
    let empty = workspace("empty-folder");
    assert!(project::program_in(&empty).is_err());
}
