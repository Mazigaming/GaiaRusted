//! Loop-invariant code motion: computing once what a loop computes every
//! time around with the same result.
//!
//! A computation inside a loop whose operands do not change in the loop
//! gives the same value on every iteration. It can move to the block just
//! before the loop (the preheader) if moving it is invisible:
//!
//! * it is the only assignment to its local in the loop, and the loop does
//!   not read that local's value from before the loop;
//! * it cannot fail, since it may now run even if the loop body never
//!   does. A division by a non-zero constant is fine.
//!
//! Reading memory through a reference is a computation like any other in
//! a loop that writes no memory (no calls, no stores but to locals of its
//! own): it reads the same value every time around. A reference always
//! points to a valid value, so reading early cannot fault; a raw pointer
//! promises nothing, and reads through one stay where they are.
//!
//! Hoisting one computation can make another invariant, so the search
//! repeats. Inner loops are handled first, and what leaves an inner loop
//! may then leave the outer one too.

use crate::ir::analysis::{natural_loops, writes_memory, BitSet, Effect, Liveness};
use crate::ir::*;
use crate::sema::ty::Ty;
use crate::syntax::ast::BinOp;

/// Returns whether anything moved.
///
/// The analyses are done once for all loops. Hoisting keeps them valid: a
/// definition moved earlier only shortens the time its local is live, and
/// a preheader made for an inner loop is added to the bodies of the loops
/// around it, of which it is now a part.
pub fn run(func: &mut Function) -> bool {
    let loops = natural_loops(func);
    if loops.is_empty() {
        return false;
    }
    let whole = func.whole_locals();
    let liveness = Liveness::compute(func, &whole);
    // Room for one new preheader per loop.
    let capacity = func.blocks.len() + loops.len();
    let mut bodies: Vec<BitSet> = loops
        .iter()
        .map(|found| {
            let mut body = BitSet::new(capacity);
            found.body.iter().for_each(|block| {
                body.insert(block);
            });
            body
        })
        .collect();

    let mut changed = false;
    for (position, found) in loops.iter().enumerate() {
        let Some(preheader) = hoist_from(func, &whole, &liveness, found.header, &bodies[position]) else { continue };
        changed = true;
        // Loops come inner first: the enclosing ones are later in the list.
        for body in &mut bodies[position + 1..] {
            if body.contains(found.header.0 as usize) {
                body.insert(preheader.0 as usize);
            }
        }
    }
    changed
}

/// Hoist what can be hoisted out of one loop. Returns the preheader, if
/// anything went there.
fn hoist_from(func: &mut Function, whole: &BitSet, liveness: &Liveness, header: BlockId, body: &BitSet) -> Option<BlockId> {
    // How many times each local is written in the loop.
    let mut writes = vec![0u32; func.locals.len()];
    for block in body.iter() {
        for statement in &func.blocks[block].statements {
            for local in Effect::of_statement(statement).writes {
                writes[local.0 as usize] += 1;
            }
        }
    }
    let mut live_at_header = liveness.live_out[header.0 as usize].clone();
    liveness.walk_block(func, header, &mut live_at_header, &mut |_, _, _| {});

    let writes_memory = body.iter().any(|block| func.blocks[block].statements.iter().any(|statement| writes_memory(whole, statement)));
    let unchanged = |writes: &[u32], local: Local| whole.contains(local.0 as usize) && writes[local.0 as usize] == 0;
    // A place whose value is the same every time around the loop: a local
    // the loop does not change, or the value a reference that the loop
    // does not change points to, when the loop writes no memory.
    let invariant_place = |writes: &[u32], place: &Place| match place.projection.split_first() {
        None => unchanged(writes, place.local),
        Some((Projection::Deref, fields)) => {
            !writes_memory
                && matches!(func.locals[place.local.0 as usize].ty, Ty::Ref(..))
                && unchanged(writes, place.local)
                && fields.iter().all(|projection| matches!(projection, Projection::Field(_) | Projection::Downcast(_)))
        }
        Some(_) => false,
    };
    let invariant = |writes: &[u32], operand: &Operand| match operand {
        Operand::Const(_) => true,
        Operand::Copy(place) => invariant_place(writes, place),
    };
    let hoistable = |writes: &[u32], statement: &Statement| {
        let Statement::Assign(place, rvalue) = statement else { return false };
        let Some(local) = place.as_local() else { return false };
        let index = local.0 as usize;
        if !whole.contains(index) || writes[index] != 1 || live_at_header.contains(index) {
            return false;
        }
        let cannot_fail = match rvalue {
            Rvalue::Binary(BinOp::Div | BinOp::Rem, _, divisor) => {
                matches!(divisor, Operand::Const(Const::Int(bits, _)) if *bits != 0)
            }
            Rvalue::Use(_) | Rvalue::Binary(..) | Rvalue::Unary(..) | Rvalue::Cast(..) => true,
            Rvalue::MakeFat(..) | Rvalue::FatData(_) | Rvalue::FatExtra(_) => true,
            Rvalue::Discriminant(place) => invariant_place(writes, place),
            // An address computed from a pointer the loop does not change,
            // without reading memory on the way.
            Rvalue::AddrOf(place) => match place.projection.split_first() {
                None => true,
                Some((Projection::Deref, fields)) => {
                    unchanged(writes, place.local)
                        && fields.iter().all(|projection| matches!(projection, Projection::Field(_) | Projection::Downcast(_)))
                }
                Some(_) => false,
            },
        };
        let mut operands_invariant = true;
        rvalue.each_operand(&mut |operand| operands_invariant &= invariant(writes, operand));
        cannot_fail && operands_invariant
    };

    let mut hoisted = Vec::new();
    let mut changed = true;
    while changed {
        changed = false;
        for block in body.iter() {
            let statements = std::mem::take(&mut func.blocks[block].statements);
            let mut kept = Vec::with_capacity(statements.len());
            for statement in statements {
                if hoistable(&writes, &statement) {
                    for local in Effect::of_statement(&statement).writes {
                        writes[local.0 as usize] = 0;
                    }
                    hoisted.push(statement);
                    changed = true;
                } else {
                    kept.push(statement);
                }
            }
            func.blocks[block].statements = kept;
        }
    }
    if hoisted.is_empty() {
        return None;
    }
    let preheader = preheader(func, header, body);
    func.blocks[preheader.0 as usize].statements.extend(hoisted);
    Some(preheader)
}

/// The block every entry into the loop passes through just before the
/// header, made if there is none.
fn preheader(func: &mut Function, header: BlockId, body: &BitSet) -> BlockId {
    let predecessors = func.predecessors();
    let outside: Vec<BlockId> =
        predecessors[header.0 as usize].iter().copied().filter(|block| !body.contains(block.0 as usize)).collect();
    if let [only] = outside.as_slice() {
        if matches!(func.blocks[only.0 as usize].terminator, Some(Terminator::Goto(_))) {
            return *only;
        }
    }
    let preheader = BlockId(func.blocks.len() as u32);
    func.blocks.push(Block { statements: Vec::new(), terminator: Some(Terminator::Goto(header)) });
    for block in outside {
        if let Some(terminator) = &mut func.blocks[block.0 as usize].terminator {
            terminator.each_target_mut(&mut |target| {
                if *target == header {
                    *target = preheader;
                }
            });
        }
    }
    preheader
}
