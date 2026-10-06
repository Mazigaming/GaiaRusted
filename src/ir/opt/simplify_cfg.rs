//! Tidying the control-flow graph.
//!
//! Lowering builds blocks generously: every `if`, loop and scope exit gets
//! its own, and many end up empty or reachable from one place only. This
//! pass folds branches whose outcome is known, sends jumps straight to
//! where they end up, glues blocks that always run one after the other,
//! and drops whatever can no longer be reached. What is left is numbered
//! in reverse postorder, the order code is best laid out in, except that
//! blocks leading only to a panic go last: the hot path then falls through
//! its checks instead of jumping over the code for when they fail.

use super::fold;
use crate::ir::*;

pub fn run(func: &mut Function) {
    loop {
        let folded = fold_branches(func);
        let threaded = thread_jumps(func);
        let merged = merge_chains(func);
        if !(folded || threaded || merged) {
            break;
        }
    }
    remove_unreachable(func);
}

/// A branch that can only go one way is a jump.
fn fold_branches(func: &mut Function) -> bool {
    let mut changed = false;
    for block in &mut func.blocks {
        let target = match &block.terminator {
            Some(Terminator::Branch { then_block, else_block, .. }) if then_block == else_block => *then_block,
            Some(Terminator::Branch { cond: Operand::Const(Const::Int(value, _)), then_block, else_block }) => {
                if *value != 0 {
                    *then_block
                } else {
                    *else_block
                }
            }
            Some(Terminator::Switch { value: Operand::Const(Const::Int(value, ty)), arms, otherwise }) => {
                let value = fold::normalize(*value, ty);
                let arm = arms.iter().find(|(arm, _)| fold::normalize(*arm, ty) == value);
                arm.map_or(*otherwise, |(_, target)| *target)
            }
            _ => continue,
        };
        block.terminator = Some(Terminator::Goto(target));
        changed = true;
    }
    changed
}

/// A jump to an empty block that only jumps on goes to the final target
/// directly.
fn thread_jumps(func: &mut Function) -> bool {
    let forwards: Vec<Option<BlockId>> = func
        .blocks
        .iter()
        .map(|block| match &block.terminator {
            Some(Terminator::Goto(target)) if block.statements.is_empty() => Some(*target),
            _ => None,
        })
        .collect();
    let destination = |mut block: BlockId| {
        // An empty block that jumps to itself is an endless loop; stop
        // following a chain once it has been all the way round.
        for _ in 0..forwards.len() {
            match forwards[block.0 as usize] {
                Some(next) if next != block => block = next,
                _ => break,
            }
        }
        block
    };

    let mut changed = false;
    for block in &mut func.blocks {
        if let Some(terminator) = &mut block.terminator {
            terminator.each_target_mut(&mut |target| {
                let end = destination(*target);
                changed |= end != *target;
                *target = end;
            });
        }
    }
    changed
}

/// A block entered only by a jump from one other block is the rest of that
/// block.
fn merge_chains(func: &mut Function) -> bool {
    let mut entries: Vec<usize> = func.predecessors().iter().map(Vec::len).collect();
    // The entry block is also entered from outside.
    entries[0] += 1;

    let mut changed = false;
    for index in 0..func.blocks.len() {
        while let Some(Terminator::Goto(next)) = func.blocks[index].terminator {
            let next = next.0 as usize;
            if next == index || entries[next] != 1 {
                break;
            }
            let absorbed = std::mem::take(&mut func.blocks[next]);
            func.blocks[next].terminator = Some(Terminator::Unreachable);
            entries[next] = 0;
            func.blocks[index].statements.extend(absorbed.statements);
            func.blocks[index].terminator = absorbed.terminator;
            changed = true;
        }
    }
    changed
}

/// Keep only the blocks reachable from the entry, in reverse postorder
/// with the dead ends moved to the back.
fn remove_unreachable(func: &mut Function) {
    let is_dead_end = |block: &BlockId| matches!(func.blocks[block.0 as usize].terminator, Some(Terminator::Unreachable));
    let (dead_ends, mut order): (Vec<BlockId>, Vec<BlockId>) = func.reverse_postorder().into_iter().partition(is_dead_end);
    // The entry stays first.
    if order.is_empty() {
        order.push(BlockId(0));
    }
    order.extend(dead_ends.into_iter().filter(|block| block.0 != 0));
    let mut new_id = vec![BlockId(0); func.blocks.len()];
    for (position, block) in order.iter().enumerate() {
        new_id[block.0 as usize] = BlockId(position as u32);
    }
    let mut blocks: Vec<Block> = order.iter().map(|block| std::mem::take(&mut func.blocks[block.0 as usize])).collect();
    for block in &mut blocks {
        if let Some(terminator) = &mut block.terminator {
            terminator.each_target_mut(&mut |target| *target = new_id[target.0 as usize]);
        }
    }
    func.blocks = blocks;
}
