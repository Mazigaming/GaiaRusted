//! Optimisation: rewriting the IR into IR that does the same thing faster.
//!
//! Each pass is a function from a correct program to a correct program, so
//! passes can be read, tested and reordered independently:
//!
//! * [`simplify_cfg`] — removes the jumps and blocks that lowering leaves behind
//! * [`sroa`] — splits structs, tuples and enums into one local per field
//! * [`propagate`] — uses known copies, constants and addresses where they are read
//! * [`thread_jumps`] — sends jumps past branches whose outcome is already known
//! * [`licm`] — moves computations that do not change out of loops
//! * [`rotate`] — moves a loop's test from the top to the bottom
//! * [`addresses`] — computes the address of an element used repeatedly once
//! * [`dce`] — removes assignments nobody reads
//! * [`compact`] — forgets locals that are no longer mentioned
//! * [`inline`] — replaces calls of small functions by their bodies
//! * [`tail_calls`] — turns a function calling itself on its way out into a loop
//! * [`prune`] — drops functions that are no longer referred to
//!
//! [`fold`] evaluates operations on constants for the passes that meet them.

pub mod addresses;
pub mod compact;
pub mod dce;
pub mod fold;
pub mod inline;
pub mod licm;
pub mod propagate;
pub mod rotate;
pub mod prune;
pub mod simplify_cfg;
pub mod sroa;
pub mod tail_calls;
pub mod thread_jumps;

use super::{Function, Program};
use crate::sema::context::Context;

/// How hard to try.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptLevel {
    /// Translate the program as written: every local in memory, nothing
    /// rearranged. The reference that optimised output is tested against.
    None,
    /// Everything that pays for itself.
    Full,
}

impl OptLevel {
    pub fn from_number(level: u32) -> OptLevel {
        match level {
            0 => OptLevel::None,
            _ => OptLevel::Full,
        }
    }
}

pub fn optimize(program: &mut Program, tcx: &Context, level: OptLevel) {
    if level == OptLevel::None {
        return;
    }
    // Inlining tidies each function as it finishes with it.
    inline::run(program, tcx);
    prune::run(program);
}

/// The passes that clean up within one function, repeated while they find
/// something to do. Each can give the others new work: a constant
/// condition becomes a jump, a merged block brings a copy and its use
/// together, a removed assignment ends a local's last use.
pub fn tidy(func: &mut Function, tcx: &Context) {
    /// Enough for what lowering and inlining produce; more rounds find little.
    const ROUNDS: usize = 6;
    simplify_cfg::run(func);
    for _ in 0..ROUNDS {
        let split = sroa::run(func, tcx);
        let propagated = propagate::run(func, tcx);
        let threaded = thread_jumps::run(func);
        simplify_cfg::run(func);
        let removed = dce::run(func);
        let hoisted = licm::run(func);
        if !(split || propagated || threaded || removed || hoisted) {
            break;
        }
    }
    // Last, as they make the code bigger for the passes above to look at.
    if rotate::run(func) {
        simplify_cfg::run(func);
    }
    addresses::run(func, tcx);
    compact::run(func);
}
