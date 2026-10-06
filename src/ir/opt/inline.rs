//! Inlining: replacing a call by the body of the function it calls.
//!
//! Abstractions are built from small functions (`Vec::len`, an iterator's
//! `next`, a closure), and calling one costs more than what it does. Once
//! its body sits in the caller the call is gone, and so is the wall between
//! the two: the other passes see through what used to be a call.
//!
//! Functions are visited callees first, so a body is copied into its
//! callers in its final form, already holding what was inlined into it. A
//! function that takes part in a recursion is never copied: there would be
//! no end to it.

use super::{tail_calls, tidy};
use crate::ir::*;
use crate::sema::context::Context;
use crate::sema::ty::Ty;

/// Functions up to this size are copied into their callers.
const INLINE_SIZE: usize = 48;
/// A function stops absorbing its callees when it gets this big.
const CALLER_SIZE_LIMIT: usize = 8000;

pub fn run(program: &mut Program, tcx: &Context) {
    let (order, recursive) = callees_first(program);
    for caller in order {
        inline_calls_in(program, tcx, caller, &recursive);
        let func = &mut program.functions[caller.0 as usize];
        tidy(func, tcx);
        if recursive[caller.0 as usize] && tail_calls::run(func, caller, tcx) {
            tidy(func, tcx);
        }
    }
}

/// A rough measure of how much code a function is.
fn size(func: &Function) -> usize {
    func.blocks.iter().map(|block| block.statements.len() + 1).sum()
}

fn direct_callees(func: &Function) -> Vec<FuncId> {
    let statements = func.blocks.iter().flat_map(|block| &block.statements);
    statements
        .filter_map(|statement| match statement {
            Statement::Call { callee: Callee::Direct(callee), .. } => Some(*callee),
            _ => None,
        })
        .collect()
}

/// All functions, each after the functions it calls, and for each function
/// whether it can reach a call of itself. (In a cycle, the order has to
/// break the rule somewhere.)
fn callees_first(program: &Program) -> (Vec<FuncId>, Vec<bool>) {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        Unseen,
        /// Being explored: reaching it again means a cycle.
        Open,
        Done,
    }
    let count = program.functions.len();
    let callees: Vec<Vec<FuncId>> = program.functions.iter().map(direct_callees).collect();
    let mut states = vec![State::Unseen; count];
    let mut recursive = vec![false; count];
    let mut order = Vec::with_capacity(count);

    for root in 0..count {
        if states[root] != State::Unseen {
            continue;
        }
        states[root] = State::Open;
        let mut path = vec![(root, 0)];
        while let Some((func, next)) = path.last_mut() {
            match callees[*func].get(*next) {
                Some(&callee) => {
                    *next += 1;
                    let callee = callee.0 as usize;
                    match states[callee] {
                        State::Unseen => {
                            states[callee] = State::Open;
                            path.push((callee, 0));
                        }
                        // Everything on the path from the callee to here
                        // is on a cycle through it.
                        State::Open => {
                            let start = path.iter().position(|(func, _)| *func == callee).expect("open functions are on the path");
                            path[start..].iter().for_each(|(func, _)| recursive[*func] = true);
                        }
                        State::Done => {}
                    }
                }
                None => {
                    states[*func] = State::Done;
                    order.push(FuncId(*func as u32));
                    path.pop();
                }
            }
        }
    }
    (order, recursive)
}

fn inline_calls_in(program: &mut Program, tcx: &Context, caller: FuncId, recursive: &[bool]) {
    let mut caller_size = size(&program.functions[caller.0 as usize]);
    let worthwhile = |program: &Program, callee: FuncId| {
        let func = &program.functions[callee.0 as usize];
        // A function that never returns is an error path: keep it out of line.
        callee != caller
            && !recursive[callee.0 as usize]
            && *func.ret_ty() != Ty::Never
            && size(func) <= INLINE_SIZE
            && !func.calls_returns_twice()
    };

    // Blocks added by inlining come after the ones present now. What was
    // inlined is final, so only the latter are searched for calls, along
    // with the remainders of the blocks that get split.
    for start in 0..program.functions[caller.0 as usize].blocks.len() {
        let mut block = BlockId(start as u32);
        let mut from = 0;
        loop {
            let statements = &program.functions[caller.0 as usize].blocks[block.0 as usize].statements;
            let call = statements[from..].iter().position(|statement| match statement {
                Statement::Call { callee: Callee::Direct(callee), .. } => worthwhile(program, *callee),
                _ => false,
            });
            let Some(offset) = call else { break };
            let index = from + offset;
            let Statement::Call { callee: Callee::Direct(callee), .. } = &statements[index] else {
                unreachable!("found to be a direct call above");
            };
            let callee = &program.functions[callee.0 as usize];
            caller_size += size(callee);
            if caller_size > CALLER_SIZE_LIMIT {
                return;
            }
            let body = Body { locals: callee.locals.clone(), arg_count: callee.arg_count, blocks: callee.blocks.clone() };
            block = splice(&mut program.functions[caller.0 as usize], tcx, block, index, body);
            from = 0;
        }
    }
}

/// A copy of the function being inlined.
struct Body {
    locals: Vec<LocalDecl>,
    arg_count: usize,
    blocks: Vec<Block>,
}

/// What a local of the inlined function becomes in the caller.
enum Renamed {
    /// A local of its own.
    Local(Local),
    /// A place the caller already has: the memory of an argument handed
    /// over, or where the result is wanted.
    Place(Place),
}

/// A place whose location does not depend on any other value.
fn is_fixed(place: &Place) -> bool {
    place.projection.iter().all(|projection| matches!(projection, Projection::Field(_) | Projection::Downcast(_)))
}

/// Replace the call at `block.statements[index]` by `body`. The block ends
/// at the call from now on; returns the new block holding what followed it.
fn splice(func: &mut Function, tcx: &Context, block: BlockId, index: usize, body: Body) -> BlockId {
    let block = block.0 as usize;
    let rest = func.blocks[block].statements.split_off(index + 1);
    let Some(Statement::Call { dest, args, .. }) = func.blocks[block].statements.pop() else {
        unreachable!("the statement to replace is a call");
    };
    let continuation = BlockId(func.blocks.len() as u32);
    let terminator = func.blocks[block].terminator.take();
    func.blocks.push(Block { statements: rest, terminator });

    let new_local = |func: &mut Function, decl: &LocalDecl| {
        func.locals.push(decl.clone());
        Local(func.locals.len() as u32 - 1)
    };
    let mut renamed = Vec::with_capacity(body.locals.len());

    // The result. A value returned in memory is built where the caller
    // wants it, as a call would have it do. A value returned in registers
    // is the callee's own until it returns.
    let ret = &body.locals[0];
    let mentioned_by_args = args.iter().any(|arg| matches!(arg, Operand::Copy(place) if place.local == dest.local));
    if !tcx.is_register_value(&ret.ty) && is_fixed(&dest) && !mentioned_by_args {
        renamed.push(Renamed::Place(dest));
    } else {
        let result = new_local(func, ret);
        let hand_over = Statement::Assign(dest, Rvalue::Use(Operand::Copy(Place::local(result))));
        func.blocks[continuation.0 as usize].statements.insert(0, hand_over);
        renamed.push(Renamed::Local(result));
    }

    // The parameters. An argument handed over in memory is that memory;
    // one passed in registers is a copy.
    for (param, arg) in body.locals[1..=body.arg_count].iter().zip(args) {
        match arg {
            Operand::Copy(place) if !tcx.is_register_value(&param.ty) && is_fixed(&place) => {
                renamed.push(Renamed::Place(place));
            }
            arg => {
                let local = new_local(func, param);
                func.blocks[block].statements.push(Statement::Assign(Place::local(local), Rvalue::Use(arg)));
                renamed.push(Renamed::Local(local));
            }
        }
    }
    for decl in &body.locals[body.arg_count + 1..] {
        renamed.push(Renamed::Local(new_local(func, decl)));
    }

    let rename = |place: &mut Place| {
        for projection in &mut place.projection {
            if let Projection::Index(index) = projection {
                let Renamed::Local(local) = &renamed[index.0 as usize] else {
                    unreachable!("an index is a `usize`, which travels in a register");
                };
                *index = *local;
            }
        }
        match &renamed[place.local.0 as usize] {
            Renamed::Local(local) => place.local = *local,
            Renamed::Place(prefix) => {
                place.local = prefix.local;
                place.projection.splice(0..0, prefix.projection.iter().cloned());
            }
        }
    };

    let entry = func.blocks.len() as u32;
    func.blocks[block].terminator = Some(Terminator::Goto(BlockId(entry)));
    for mut inlined in body.blocks {
        for statement in &mut inlined.statements {
            statement.each_place_mut(&mut |place, _| rename(place));
        }
        let mut terminator = inlined.terminator.take().expect("inlined functions are complete");
        terminator.each_place_mut(&mut |place, _| rename(place));
        terminator.each_target_mut(&mut |target| target.0 += entry);
        if let Terminator::Return = terminator {
            terminator = Terminator::Goto(continuation);
        }
        inlined.terminator = Some(terminator);
        func.blocks.push(inlined);
    }
    continuation
}
