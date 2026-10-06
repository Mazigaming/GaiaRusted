//! Forgetting the locals nothing mentions any more.
//!
//! Passes leave locals behind: the ones whose last use was propagated
//! away, aggregates that were split, the leftovers of inlined bodies. They
//! cost nothing at run time, but every later analysis is sized by the
//! number of locals, and an inlined function brings all of its own along.

use crate::ir::visit::LocalUse;
use crate::ir::*;

pub fn run(func: &mut Function) {
    let mut used = vec![false; func.locals.len()];
    // The result and the parameters are part of the signature.
    used[..=func.arg_count].fill(true);
    let mut note = |local: Local, _: LocalUse| used[local.0 as usize] = true;
    for block in &func.blocks {
        block.statements.iter().for_each(|statement| statement.each_local(&mut note));
        if let Some(terminator) = &block.terminator {
            terminator.each_local(&mut note);
        }
    }
    if !used.contains(&false) {
        return;
    }

    let mut renumbered = Vec::with_capacity(used.len());
    let mut kept = 0;
    for &is_used in &used {
        renumbered.push(Local(kept));
        kept += is_used as u32;
    }
    let mut index = 0;
    func.locals.retain(|_| {
        index += 1;
        used[index - 1]
    });

    let rename = |place: &mut Place| {
        place.local = renumbered[place.local.0 as usize];
        for projection in &mut place.projection {
            if let Projection::Index(index) = projection {
                *index = renumbered[index.0 as usize];
            }
        }
    };
    for block in &mut func.blocks {
        for statement in &mut block.statements {
            statement.each_place_mut(&mut |place, _| rename(place));
        }
        if let Some(terminator) = &mut block.terminator {
            terminator.each_place_mut(&mut |place, _| rename(place));
        }
    }
}
