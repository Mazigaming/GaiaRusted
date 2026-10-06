//! Loop rotation: testing a loop's condition at the bottom.
//!
//! A `while` loop is lowered with its test at the top: every iteration
//! jumps back to the header, which tests and then jumps into the body or
//! out. Giving the end of the body its own copy of the test saves the jump
//! back: the loop then runs as
//!
//! ```text
//! if cond { loop { body; if !cond { break } } }
//! ```
//!
//! with a single branch per iteration. Only small headers are copied.

use crate::ir::analysis::natural_loops;
use crate::ir::*;

/// Headers up to this many statements are copied into the loop's latches.
const MAX_COPIED: usize = 4;

/// Returns whether any loop was rotated.
pub fn run(func: &mut Function) -> bool {
    let predecessors = func.predecessors();
    let mut changed = false;
    for found in natural_loops(func) {
        let header = &func.blocks[found.header.0 as usize];
        let tests = matches!(header.terminator, Some(Terminator::Branch { .. } | Terminator::Switch { .. }));
        if !tests || header.statements.len() > MAX_COPIED {
            continue;
        }
        let (statements, terminator) = (header.statements.clone(), header.terminator.clone());
        // The blocks that go back to the header to start another iteration.
        let latches = predecessors[found.header.0 as usize].iter().filter(|block| found.body.contains(block.0 as usize));
        for latch in latches {
            let latch = &mut func.blocks[latch.0 as usize];
            if latch.terminator != Some(Terminator::Goto(found.header)) {
                continue;
            }
            latch.statements.extend(statements.iter().cloned());
            latch.terminator = terminator.clone();
            changed = true;
        }
    }
    changed
}
