//! Conformance: every program in `conformance/` must behave exactly as it
//! does when compiled by rustc.
//!
//! Each `name.rs` is compiled with the typed pipeline and run, once without
//! optimisation and once with; its standard output must equal `name.stdout`
//! both times. Those files are produced by rustc itself:
//!
//! ```text
//! BLESS=1 cargo test --test conformance
//! ```
//!
//! A `name.stdin` next to a program is given to it as standard input.
//!
//! To add a case, drop a program into `conformance/` and bless it.
//!
//! The programs in `conformance/reject/` are ones rustc refuses to compile.
//! Each starts with `// error: message`, and must be refused here too,
//! with an error saying `message`.

use gaiarusted::driver::{self, Options};
use gaiarusted::pipeline::OptLevel;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn programs() -> Vec<PathBuf> {
    programs_in("conformance")
}

fn programs_in(directory: &str) -> Vec<PathBuf> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join(directory);
    let mut programs: Vec<PathBuf> = std::fs::read_dir(&directory)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", directory.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect();
    programs.sort();
    programs
}

/// How long a program may run before it is taken to be stuck.
const TIME_LIMIT: Duration = Duration::from_secs(20);

fn run(executable: &Path, input: &Path) -> Result<String, String> {
    let stdin = match std::fs::File::open(input) {
        Ok(file) => Stdio::from(file),
        Err(_) => Stdio::null(),
    };
    let mut child = Command::new(executable)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run: {e}"))?;
    let started = Instant::now();
    while child.try_wait().map_err(|e| format!("cannot wait: {e}"))?.is_none() {
        if started.elapsed() > TIME_LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("still running after {} s", TIME_LIMIT.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let output = child.wait_with_output().map_err(|e| format!("cannot collect output: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("exited with {}: {}", output.status, stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// What rustc's build of the program prints.
fn reference_output(program: &Path, executable: &Path) -> Result<String, String> {
    let status = Command::new("rustc")
        .args(["--edition", "2021", "-A", "warnings", "-o"])
        .arg(executable)
        .arg(program)
        .output()
        .map_err(|e| format!("cannot run rustc: {e}"))?;
    if !status.status.success() {
        return Err(format!("rustc rejects it:\n{}", String::from_utf8_lossy(&status.stderr)));
    }
    run(executable, &program.with_extension("stdin"))
}

/// The optimisation levels every program is checked at.
const OPT_LEVELS: [u32; 2] = [0, 2];

/// What this compiler's build of the program prints.
fn actual_output(program: &Path, executable: &Path, opt_level: u32) -> Result<String, String> {
    let mut options = Options::executable(program, executable);
    options.opt_level = OptLevel::from_number(opt_level);
    driver::build(&options).map_err(|failure| format!("does not compile:\n{}", failure.render()))?;
    run(executable, &program.with_extension("stdin"))
}

#[test]
fn programs_behave_as_under_rustc() {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let bless = std::env::var_os("BLESS").is_some();
    let mut failures = Vec::new();

    for program in programs() {
        let name = program.file_stem().unwrap().to_string_lossy().into_owned();
        let expected_file = program.with_extension("stdout");
        let executable = scratch.join(&name);

        if bless {
            match reference_output(&program, &executable) {
                Ok(output) => std::fs::write(&expected_file, output).unwrap(),
                Err(problem) => failures.push(format!("{name}: {problem}")),
            }
            continue;
        }

        let Ok(expected) = std::fs::read_to_string(&expected_file) else {
            failures.push(format!("{name}: no expected output (run with BLESS=1)"));
            continue;
        };
        for opt_level in OPT_LEVELS {
            let name = format!("{name} (-O{opt_level})");
            match actual_output(&program, &executable, opt_level) {
                Ok(actual) if actual == expected => {}
                Ok(actual) => {
                    let first_difference = expected
                        .lines()
                        .zip(actual.lines())
                        .position(|(e, a)| e != a)
                        .unwrap_or_else(|| expected.lines().count().min(actual.lines().count()));
                    failures.push(format!(
                        "{name}: output differs at line {}\n  expected: {:?}\n  actual:   {:?}",
                        first_difference + 1,
                        expected.lines().nth(first_difference).unwrap_or("<end of output>"),
                        actual.lines().nth(first_difference).unwrap_or("<end of output>"),
                    ));
                }
                Err(problem) => failures.push(format!("{name}: {problem}")),
            }
        }
    }

    assert!(failures.is_empty(), "{} failing:\n\n{}\n", failures.len(), failures.join("\n\n"));
}

#[test]
fn invalid_programs_are_rejected_as_by_rustc() {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let mut failures = Vec::new();
    for program in programs_in("conformance/reject") {
        let name = program.file_stem().unwrap().to_string_lossy().into_owned();
        let source = std::fs::read_to_string(&program).unwrap();
        let Some(expected) = source.lines().next().and_then(|line| line.strip_prefix("// error: ")) else {
            failures.push(format!("{name}: no `// error: message` line to start with"));
            continue;
        };
        let executable = scratch.join(format!("reject-{name}"));
        let rustc = Command::new("rustc")
            .args(["--edition", "2021", "-A", "warnings", "-o"])
            .arg(&executable)
            .arg(&program)
            .output()
            .expect("rustc runs");
        if rustc.status.success() {
            failures.push(format!("{name}: rustc accepts it"));
            continue;
        }
        let messages = match driver::build(&Options::executable(&program, &executable)) {
            Ok(_) => {
                failures.push(format!("{name}: compiles, but should not"));
                continue;
            }
            Err(failure) => failure.render(),
        };
        if !messages.contains(expected) {
            failures.push(format!("{name}: rejected, but not with `{expected}`:\n{messages}"));
        }
    }
    assert!(failures.is_empty(), "{} failing:\n\n{}\n", failures.len(), failures.join("\n\n"));
}
