//! Jump threading: skipping a branch whose outcome a predecessor decides.
//!
//! A function that returns `Some(x)` or `None` and a caller that matches
//! on the result meet, after inlining, in a pattern like
//!
//! ```text
//! bb1: tag = 1; goto bb3        bb2: tag = 0; goto bb3
//! bb3: is_some = tag == 1; if is_some goto bb4 else bb5
//! ```
//!
//! Each way into `bb3` already knows where it will go from there. A
//! predecessor that ends in a jump to such a small block gets its own copy
//! of the block's statements and jumps straight to the right successor.

use super::fold;
use crate::ir::analysis::BitSet;
use crate::ir::*;
use std::collections::HashMap;

/// Blocks up to this many statements are copied into their predecessors.
const MAX_COPIED: usize = 4;

/// Returns whether any jump was threaded.
pub fn run(func: &mut Function) -> bool {
    let whole = func.whole_locals();
    let mut changed = false;
    for index in 0..func.blocks.len() {
        // Threading can expose another chance right away; the bound keeps
        // a pathological loop from going on forever.
        for _ in 0..func.blocks.len() {
            let Some(Terminator::Goto(next)) = func.blocks[index].terminator else { break };
            let next_block = &func.blocks[next.0 as usize];
            if next.0 as usize == index || next_block.statements.len() > MAX_COPIED {
                break;
            }
            let Some(target) = decided_target(&whole, &func.blocks[index], next_block) else { break };
            let copied = next_block.statements.clone();
            let block = &mut func.blocks[index];
            block.statements.extend(copied);
            block.terminator = Some(Terminator::Goto(target));
            changed = true;
        }
    }
    changed
}

/// Where `next` branches to when entered from `from`, if the constants
/// `from` leaves behind settle it.
fn decided_target(whole: &BitSet, from: &Block, next: &Block) -> Option<BlockId> {
    let mut constants: HashMap<Local, Const> = HashMap::new();
    for statement in from.statements.iter().chain(&next.statements) {
        step(whole, &mut constants, statement);
    }
    let value = |operand: &Operand| match operand {
        Operand::Const(constant) => Some(constant.clone()),
        Operand::Copy(place) => place.as_local().and_then(|local| constants.get(&local).cloned()),
    };
    match next.terminator.as_ref()? {
        Terminator::Branch { cond, then_block, else_block } => match value(cond)? {
            Const::Int(bits, _) => Some(if bits != 0 { *then_block } else { *else_block }),
            _ => None,
        },
        Terminator::Switch { value: scrutinee, arms, otherwise } => match value(scrutinee)? {
            Const::Int(bits, ty) => {
                let bits = fold::normalize(bits, &ty);
                let arm = arms.iter().find(|(arm, _)| fold::normalize(*arm, &ty) == bits);
                Some(arm.map_or(*otherwise, |(_, target)| *target))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Update which locals hold which constants after `statement`.
fn step(whole: &BitSet, constants: &mut HashMap<Local, Const>, statement: &Statement) {
    let written = match statement {
        Statement::Assign(place, rvalue) => place.as_local().map(|local| (local, Some(rvalue))),
        Statement::Call { dest, .. } => dest.as_local().map(|local| (local, None)),
        Statement::SetDiscriminant(..) => None,
    };
    let Some((local, rvalue)) = written else { return };
    constants.remove(&local);
    if !whole.contains(local.0 as usize) {
        return;
    }
    let Some(mut rvalue) = rvalue.cloned() else { return };
    rvalue.each_operand_mut(&mut |operand| {
        if let Some(constant) = operand_constant(constants, operand) {
            *operand = Operand::Const(constant);
        }
    });
    let constant = match &rvalue {
        Rvalue::Use(Operand::Const(constant)) => Some(constant.clone()),
        other => fold::rvalue(other),
    };
    if let Some(constant) = constant {
        constants.insert(local, constant);
    }
}

fn operand_constant(constants: &HashMap<Local, Const>, operand: &Operand) -> Option<Const> {
    let Operand::Copy(place) = operand else { return None };
    constants.get(&place.as_local()?).cloned()
}
