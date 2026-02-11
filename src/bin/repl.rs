//! GiaRusted REPL (Read-Eval-Print Loop)
//! 
//! Interactive Rust code execution interpreter.
//! 
//! Usage:
//!   cargo run --bin repl

use gaiarusted::repl::Repl;

fn main() {
    let mut repl = Repl::new().with_verbose(false);
    repl.run();
}
