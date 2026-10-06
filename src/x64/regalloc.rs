//! Register allocation: deciding which locals live in registers.
//!
//! A local is given one register for the whole function or stays in its
//! frame slot. Two locals can share a register unless one is written while
//! the other still holds a value that will be read, in which case they
//! *interfere*. Finding registers is then colouring a graph:
//!
//! 1. find the locals that can live in a register at all: scalars whose
//!    address is never taken;
//! 2. compute where each is live and from that who interferes with whom;
//! 3. hand out registers, the most heavily used locals first, each taking
//!    a register none of its interfering neighbours has. Whoever finds none
//!    stays in memory.
//!
//! A call destroys the caller-saved registers, so a local that must keep
//! its value across one only qualifies for a callee-saved register. There
//! are no callee-saved SSE registers; a float that lives across a call
//! stays in memory. A fat pointer takes two general registers at once.

use super::abi::{classify, ArgLoc, CallLayout, Class};
use super::reg::Reg;
use crate::ir::analysis::{loop_depths, BitSet, Liveness};
use crate::ir::*;
use crate::sema::context::Context;
use crate::sema::ty::Ty;
use crate::syntax::ast::BinOp;

/// Where a local is kept, if not in memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Register {
    Int(Reg),
    /// `xmm<n>`.
    Float(u8),
    /// A fat pointer: its data pointer and its extra word.
    Pair(Reg, Reg),
}

impl Register {
    /// The general registers this occupies.
    fn general(self) -> impl Iterator<Item = Reg> {
        let (first, second) = match self {
            Register::Int(reg) => (Some(reg), None),
            Register::Pair(data, extra) => (Some(data), Some(extra)),
            Register::Float(_) => (None, None),
        };
        first.into_iter().chain(second)
    }

    /// Whether this and `other` share a register.
    fn overlaps(self, other: Register) -> bool {
        match (self, other) {
            (Register::Float(a), Register::Float(b)) => a == b,
            _ => self.general().any(|reg| other.general().any(|theirs| theirs == reg)),
        }
    }
}

/// General registers a call leaves intact. Using one costs a save and a
/// restore per call of the function.
pub const CALLEE_SAVED: [Reg; 5] = [Reg::Rbx, Reg::R12, Reg::R13, Reg::R14, Reg::R15];
/// General registers free for the taking between calls. The remaining
/// ones are the code generator's scratch registers.
const CALLER_SAVED: [Reg; 4] = [Reg::Rsi, Reg::Rdi, Reg::R8, Reg::R9];
/// SSE registers that never carry arguments, so that moving parameters to
/// their homes cannot overwrite a parameter still to be moved.
const FLOAT_REGISTERS: std::ops::Range<u8> = 8..16;

/// One more level of loop nesting makes a use count this many times more.
const LOOP_WEIGHT: u64 = 10;
const MAX_WEIGHTED_DEPTH: u32 = 6;

pub struct Allocation {
    /// The register of each local; `None` for a local kept in memory.
    pub registers: Vec<Option<Register>>,
}

impl Allocation {
    /// Every local in memory.
    pub fn none(func: &Function) -> Allocation {
        Allocation { registers: vec![None; func.locals.len()] }
    }

    pub fn uses(&self, reg: Reg) -> bool {
        self.registers.iter().flatten().any(|register| register.general().any(|used| used == reg))
    }
}

/// Does executing the statement destroy the caller-saved registers, and
/// then carry on? A call that never returns (a panic) needs nothing kept.
fn clobbers_registers(func: &Function, tcx: &Context, statement: &Statement) -> bool {
    match statement {
        Statement::Call { dest, .. } => func.place_ty(tcx, dest) != Ty::Never,
        // The remainder of a float division is computed by the C library.
        Statement::Assign(_, Rvalue::Binary(BinOp::Rem, lhs, _)) => {
            matches!(classify(tcx, &func.operand_ty(tcx, lhs)), Class::Float { .. })
        }
        _ => false,
    }
}

/// The `bool`s that only decide the branch right after the comparison
/// that computes them. The code generator branches on the processor's
/// flags instead, so they never need a register.
fn branch_flags(func: &Function, tcx: &Context) -> BitSet {
    let count = func.locals.len();
    let mut mentions = vec![0usize; count];
    let mut fused_mentions = vec![0usize; count];
    for block in &func.blocks {
        for statement in &block.statements {
            statement.each_local(&mut |local, _| mentions[local.0 as usize] += 1);
        }
        if let Some(terminator) = &block.terminator {
            terminator.each_local(&mut |local, _| mentions[local.0 as usize] += 1);
        }
        let (Some(Statement::Assign(result, Rvalue::Binary(op, lhs, _))), Some(Terminator::Branch { cond, .. })) =
            (block.statements.last(), &block.terminator)
        else {
            continue;
        };
        let compares_integers = matches!(classify(tcx, &func.operand_ty(tcx, lhs)), Class::Int { .. });
        if let (Some(flag), Operand::Copy(cond)) = (result.as_local(), cond) {
            if op.is_comparison() && compares_integers && result == cond {
                fused_mentions[flag.0 as usize] += 2;
            }
        }
    }
    let mut flags = BitSet::new(count);
    for local in 0..count {
        if fused_mentions[local] > 0 && fused_mentions[local] == mentions[local] {
            flags.insert(local);
        }
    }
    flags
}

pub fn allocate(func: &Function, tcx: &Context, signature: &CallLayout) -> Allocation {
    let count = func.locals.len();
    let classes: Vec<Class> = func.locals.iter().map(|local| classify(tcx, &local.ty)).collect();
    let mut candidates = func.whole_locals();
    let flags = branch_flags(func, tcx);
    for (local, class) in classes.iter().enumerate() {
        if !matches!(class, Class::Int { .. } | Class::Float { .. } | Class::Pair) || flags.contains(local) {
            candidates.remove(local);
        }
    }

    let liveness = Liveness::compute(func, &candidates);
    let depths = loop_depths(func);
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); count];
    let mut weights = vec![0u64; count];
    let mut survives_call = BitSet::new(count);
    // Pairs of locals one of which is copied to the other: sharing a
    // register turns the copy into nothing.
    let mut copies: Vec<(usize, usize)> = Vec::new();

    let mut interfere = |a: usize, b: usize| {
        neighbours[a].push(b as u32);
        neighbours[b].push(a as u32);
    };

    // All parameters receive their values at once on entry.
    let params: Vec<usize> = func.args().map(|arg| arg.0 as usize).filter(|&arg| candidates.contains(arg)).collect();
    for (position, &a) in params.iter().enumerate() {
        for &b in &params[position + 1..] {
            interfere(a, b);
        }
    }

    for (index, block) in func.blocks.iter().enumerate() {
        let weight = LOOP_WEIGHT.pow(depths[index].min(MAX_WEIGHTED_DEPTH));
        let mut live = liveness.live_out[index].clone();
        liveness.walk_block(func, BlockId(index as u32), &mut live, &mut |position, effect, live_after| {
            let statement = block.statements.get(position);
            for local in effect.reads.iter().chain(&effect.writes) {
                weights[local.0 as usize] += weight;
            }

            // What a copy reads and what it writes hold the same value
            // afterwards: they may share a register.
            let copied = match statement {
                Some(Statement::Assign(to, Rvalue::Use(Operand::Copy(from)))) => {
                    to.as_local().zip(from.as_local()).map(|(to, from)| (to.0 as usize, from.0 as usize))
                }
                _ => None,
            };
            copies.extend(copied);

            for written in effect.writes.iter().map(|local| local.0 as usize) {
                if !candidates.contains(written) {
                    continue;
                }
                for other in live_after.iter() {
                    if other != written && copied != Some((written, other)) {
                        interfere(written, other);
                    }
                }
            }

            if statement.is_some_and(|statement| clobbers_registers(func, tcx, statement)) {
                // Whatever is live after the call must survive it, and
                // keeping what the call reads out of the caller-saved
                // registers leaves those free for setting up arguments.
                for local in live_after.iter() {
                    if !effect.writes.contains(&Local(local as u32)) {
                        survives_call.insert(local);
                    }
                }
                for local in &effect.reads {
                    survives_call.insert(local.0 as usize);
                }
            }
        });
    }

    // A parameter that arrives in a register is best left there.
    let mut preferred: Vec<Option<Register>> = vec![None; count];
    for (param, (_, location)) in func.args().zip(&signature.args) {
        if let ArgLoc::Regs(regs) = location {
            preferred[param.0 as usize] = Some(Register::Int(regs[0]));
        }
    }
    let mut partners: Vec<Vec<u32>> = vec![Vec::new(); count];
    for (a, b) in copies {
        partners[a].push(b as u32);
        partners[b].push(a as u32);
    }

    let mut order: Vec<usize> = candidates.iter().filter(|&local| weights[local] > 0).collect();
    order.sort_by_key(|&local| std::cmp::Reverse(weights[local]));

    let any_general: Vec<Reg> = CALLER_SAVED.iter().chain(&CALLEE_SAVED).copied().collect();
    let singles = |regs: &[Reg]| regs.iter().map(|&reg| Register::Int(reg)).collect::<Vec<_>>();
    let pairs = |regs: &[Reg]| {
        let mut pairs = Vec::new();
        for (index, &data) in regs.iter().enumerate() {
            pairs.extend(regs[index + 1..].iter().map(|&extra| Register::Pair(data, extra)));
        }
        pairs
    };
    let (any_single, safe_single) = (singles(&any_general), singles(&CALLEE_SAVED));
    let (any_pair, safe_pair) = (pairs(&any_general), pairs(&CALLEE_SAVED));
    let floats: Vec<Register> = FLOAT_REGISTERS.map(Register::Float).collect();

    let mut registers: Vec<Option<Register>> = vec![None; count];
    for local in order {
        let pool: &[Register] = match (classes[local], survives_call.contains(local)) {
            (Class::Int { .. }, false) => &any_single,
            (Class::Int { .. }, true) => &safe_single,
            (Class::Pair, false) => &any_pair,
            (Class::Pair, true) => &safe_pair,
            (Class::Float { .. }, false) => &floats,
            _ => continue,
        };
        let taken: Vec<Register> =
            neighbours[local].iter().filter_map(|&neighbour| registers[neighbour as usize]).collect();
        let available = |register: &Register| {
            pool.contains(register) && !taken.iter().any(|neighbour| neighbour.overlaps(*register))
        };

        let wished = partners[local].iter().filter_map(|&partner| registers[partner as usize]).chain(preferred[local]);
        registers[local] = wished.into_iter().find(available).or_else(|| pool.iter().copied().find(available));
    }
    Allocation { registers }
}
