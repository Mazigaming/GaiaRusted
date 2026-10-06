//! Dead code elimination: removing assignments whose value is never read.
//!
//! An assignment to a local is dead if no path from it reads the value
//! before it is overwritten or the function ends. Only locals whose every
//! use is visible can be judged; the others may be read through a pointer.
//! Computing a value has no other effect, so the assignment goes, and with
//! it maybe the last use of whatever it read: the pass repeats until
//! nothing more dies.

use crate::ir::analysis::{BitSet, Effect, Liveness};
use crate::ir::*;

/// Returns whether anything was removed.
pub fn run(func: &mut Function) -> bool {
    let mut removed_any = false;
    loop {
        let whole = func.whole_locals();
        let liveness = Liveness::compute(func, &whole);
        let mut removed = false;
        for (index, block) in func.blocks.iter_mut().enumerate() {
            let mut live = liveness.live_out[index].clone();
            let step = |effect: Effect, live: &mut BitSet| {
                effect.writes.iter().for_each(|local| live.remove(local.0 as usize));
                effect.reads.iter().filter(|local| whole.contains(local.0 as usize)).for_each(|local| {
                    live.insert(local.0 as usize);
                });
            };
            if let Some(terminator) = &block.terminator {
                step(Effect::of_terminator(terminator), &mut live);
            }
            let mut keep = vec![true; block.statements.len()];
            for (position, statement) in block.statements.iter().enumerate().rev() {
                if let Statement::Assign(place, _) = statement {
                    let dead = place.as_local().is_some_and(|local| {
                        whole.contains(local.0 as usize) && !live.contains(local.0 as usize)
                    });
                    if dead {
                        keep[position] = false;
                        continue;
                    }
                }
                step(Effect::of_statement(statement), &mut live);
            }
            if keep.contains(&false) {
                let mut kept = keep.into_iter();
                block.statements.retain(|_| kept.next().expect("one flag per statement"));
                removed = true;
            }
        }
        if !removed {
            return removed_any;
        }
        removed_any = true;
    }
}
