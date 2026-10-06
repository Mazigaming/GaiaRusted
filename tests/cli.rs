//! The `gaiarusted` command: what it prints and the status it exits with.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn gaiarusted(args: &[&str], dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gaiarusted")).args(args).current_dir(dir).output().unwrap()
}

fn workspace(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_successful_build_prints_nothing_and_names_the_executable_after_the_file() {
    let dir = workspace("success");
    std::fs::write(dir.join("hello.rs"), "fn main() { println!(\"hi\"); }\n").unwrap();
    let output = gaiarusted(&["hello.rs"], &dir);
    assert!(output.status.success());
    assert!(output.stdout.is_empty() && output.stderr.is_empty(), "{output:?}");
    let run = Command::new(dir.join("hello")).output().unwrap();
    assert_eq!(String::from_utf8(run.stdout).unwrap(), "hi\n");
    assert!(!dir.join("hello.s").exists());
}

#[test]
fn a_program_error_exits_with_1_and_says_so() {
    let dir = workspace("failure");
    std::fs::write(dir.join("broken.rs"), "fn main() {\n    let x: i32 = \"text\";\n}\n").unwrap();
    let output = gaiarusted(&["broken.rs"], &dir);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("mismatched types"), "{stderr}");
    assert!(stderr.ends_with("error: could not compile `broken` due to 1 previous error\n"), "{stderr}");
}

#[test]
fn unknown_options_exit_with_2() {
    let dir = workspace("unknown");
    for option in ["--pipeline", "--format=bash", "-S"] {
        let output = gaiarusted(&[option, "x.rs"], &dir);
        assert_eq!(output.status.code(), Some(2), "{option}");
        assert!(String::from_utf8(output.stderr).unwrap().contains("unknown option"), "{option}");
    }
    let output = gaiarusted(&["--format", "bash", "x.rs"], &dir);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn version_and_help() {
    let dir = workspace("version");
    let version = gaiarusted(&["--version"], &dir);
    assert_eq!(String::from_utf8(version.stdout).unwrap(), format!("gaiarusted {}\n", env!("CARGO_PKG_VERSION")));
    let help = String::from_utf8(gaiarusted(&["--help"], &dir).stdout).unwrap();
    assert!(help.contains("GaiaRusted") && help.contains("--format") && !help.contains("--pipeline"), "{help}");
}

#[test]
fn a_cargo_project_folder_builds() {
    let dir = workspace("project");
    std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"app\"\nversion = \"0.1.0\"\n").unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/main.rs"), "fn main() { println!(\"app\"); }\n").unwrap();
    let output = gaiarusted(&["."], &dir);
    assert!(output.status.success(), "{output:?}");
    assert!(dir.join("target/gaiarusted/app").exists());
    let output = gaiarusted(&["--discover", "."], &dir);
    assert!(output.status.success(), "{output:?}");
}
