//! Propagation: using what is known about a value where it is used.
//!
//! After `b = a`, a later `b` can read `a` instead, so long as neither has
//! been changed in between, on any path. Likewise for constants, for
//! pointers known to hold the address of a particular place (`*p` is that
//! place), and for fat pointers built from known halves (their length is
//! that half). What is computed only to be passed along then goes unused,
//! and [`dce`](super::dce) removes it.
//!
//! Two more kinds of knowledge come from the same bookkeeping:
//!
//! * after `a = x < y`, a later `x < y` is `a` (common subexpressions), as
//!   long as none of `a`, `x` and `y` has changed;
//! * past `if c`, `c` is known to be true on one side and false on the
//!   other, so a test repeated there (a second bounds check of the same
//!   index) has a known outcome;
//! * after `a = (*p).f`, or `a = discriminant(*p)`, a later read of the
//!   same is `a`, until `p` changes or anything may write memory: a call,
//!   or a store anywhere but to a local of the function's own.
//!
//! Only the locals whose every use is visible qualify (see
//! [`Function::whole_locals`]): nothing can change them behind the
//! analysis's back. Aggregates are left alone, since an aggregate passed
//! to a call is handed over to be changed in place, and a copy made to be
//! handed over must stay a copy.
//!
//! Operations on constants are carried out on the spot, so a condition
//! that only depends on constants becomes one, and operations that do
//! nothing (`x * 1`) are reduced to their operand.

use super::fold;
use crate::ir::analysis::{writes_memory, BitSet, Effect};
use crate::ir::*;
use crate::sema::context::Context;
use crate::sema::ty::Ty;
use std::collections::HashMap;

/// Something known about the value of a local.
#[derive(Clone)]
enum Known {
    /// It is the same as this other local.
    Copy(Local),
    Const(Const),
    /// It is the address of this place, which stays where it is.
    Address(Place),
    /// It is a two-word pointer with these halves.
    Fat(Operand, Operand),
    /// It is what this computation gives.
    Computed(Rvalue),
}

/// That `local` holds `known`, as established by one statement or branch.
struct Fact {
    local: Local,
    known: Known,
    /// The locals `known` mentions: changing one ends the fact.
    inputs: Vec<Local>,
}

/// A place whose location does not depend on any value.
fn is_fixed(place: &Place) -> bool {
    place.projection.iter().all(|projection| matches!(projection, Projection::Field(_) | Projection::Downcast(_)))
}

/// A read of memory through a pointer: `*p`, `(*p).f`, `discriminant(*p)`.
fn loaded_place(rvalue: &Rvalue) -> Option<&Place> {
    match rvalue {
        Rvalue::Use(Operand::Copy(place)) | Rvalue::Discriminant(place)
            if place.projection.first() == Some(&Projection::Deref) =>
        {
            Some(place)
        }
        _ => None,
    }
}

/// The local a computation is filed under for looking it up: the pointer
/// a load goes through, or else its first operand that is a local.
fn first_input(rvalue: &Rvalue) -> Option<Local> {
    if let Some(place) = loaded_place(rvalue) {
        return Some(place.local);
    }
    let mut first = None;
    rvalue.each_operand(&mut |operand| {
        if let (None, Operand::Copy(place)) = (first, operand) {
            first = place.as_local();
        }
    });
    first
}

struct Facts {
    all: Vec<Fact>,
    /// The fact each statement establishes, by block and position.
    established: Vec<Vec<Option<usize>>>,
    /// For each block ending in a two-way branch on a local, the facts
    /// that its condition was true and that it was false.
    branch_outcomes: Vec<Option<(usize, usize)>>,
    /// For each local, the facts that changing it ends.
    ended_by: Vec<Vec<usize>>,
    /// For each local, the facts about it.
    about: Vec<Vec<usize>>,
    /// Facts about computations, filed under their first operand.
    computations: HashMap<Local, Vec<usize>>,
    /// The facts about values read from memory, which a write to memory ends.
    loads: Vec<usize>,
    /// The locals whose address is never taken.
    whole: BitSet,
}

impl Facts {
    fn collect(func: &Function, tcx: &Context, whole: &BitSet) -> Facts {
        let tracked = |local: Local| {
            let index = local.0 as usize;
            whole.contains(index) && tcx.is_register_value(&func.locals[index].ty)
        };
        // What a fact about `target` may mention: constants, and tracked
        // locals other than `target` itself (whose old value the fact
        // would be about).
        let inputs_of = |rvalue: &Rvalue, target: Local| {
            let mut inputs = Vec::new();
            let mut usable = true;
            rvalue.each_operand(&mut |operand| match operand {
                Operand::Const(_) => {}
                Operand::Copy(place) => match place.as_local() {
                    Some(local) if local != target && tracked(local) => inputs.push(local),
                    _ => usable = false,
                },
            });
            usable.then_some(inputs)
        };

        // What a load's value depends on: the pointer and any indices.
        let load_inputs = |place: &Place, target: Local| {
            let mut inputs = vec![place.local];
            for projection in &place.projection {
                if let Projection::Index(index) = projection {
                    inputs.push(*index);
                }
            }
            inputs.iter().all(|&input| input != target && tracked(input)).then_some(inputs)
        };

        let mut facts = Facts {
            all: Vec::new(),
            established: Vec::with_capacity(func.blocks.len()),
            branch_outcomes: Vec::with_capacity(func.blocks.len()),
            ended_by: vec![Vec::new(); func.locals.len()],
            about: vec![Vec::new(); func.locals.len()],
            computations: HashMap::new(),
            loads: Vec::new(),
            whole: whole.clone(),
        };
        for block in &func.blocks {
            let mut established = Vec::with_capacity(block.statements.len());
            for statement in &block.statements {
                let Statement::Assign(place, rvalue) = statement else {
                    established.push(None);
                    continue;
                };
                let fact = place.as_local().filter(|&local| tracked(local)).and_then(|local| {
                    if let Some(loaded) = loaded_place(rvalue) {
                        let inputs = load_inputs(loaded, local)?;
                        return Some(Fact { local, known: Known::Computed(rvalue.clone()), inputs });
                    }
                    let inputs = inputs_of(rvalue, local);
                    let known = match rvalue {
                        Rvalue::Use(Operand::Const(constant)) => Known::Const(constant.clone()),
                        Rvalue::Use(Operand::Copy(_)) => Known::Copy(*inputs.as_ref()?.first()?),
                        Rvalue::AddrOf(target) if is_fixed(target) => Known::Address(target.clone()),
                        Rvalue::MakeFat(data, extra) => {
                            inputs.as_ref()?;
                            Known::Fat(data.clone(), extra.clone())
                        }
                        Rvalue::Binary(..) | Rvalue::Unary(..) | Rvalue::Cast(..) | Rvalue::FatData(_) | Rvalue::FatExtra(_) => {
                            inputs.as_ref()?;
                            Known::Computed(rvalue.clone())
                        }
                        _ => return None,
                    };
                    Some(Fact { local, known, inputs: inputs.unwrap_or_default() })
                });
                established.push(fact.map(|fact| {
                    let is_load = matches!(&fact.known, Known::Computed(rvalue) if loaded_place(rvalue).is_some());
                    let id = facts.add(fact);
                    if is_load {
                        facts.loads.push(id);
                    }
                    id
                }));
            }
            facts.established.push(established);

            let outcomes = match &block.terminator {
                Some(Terminator::Branch { cond: Operand::Copy(cond), then_block, else_block }) if then_block != else_block => {
                    cond.as_local().filter(|&local| tracked(local)).map(|local| {
                        let outcome = |value: bool| Fact {
                            local,
                            known: Known::Const(Const::Int(value as u128, Ty::Bool)),
                            inputs: Vec::new(),
                        };
                        (facts.add(outcome(true)), facts.add(outcome(false)))
                    })
                }
                _ => None,
            };
            facts.branch_outcomes.push(outcomes);
        }
        facts
    }

    fn add(&mut self, fact: Fact) -> usize {
        let id = self.all.len();
        self.ended_by[fact.local.0 as usize].push(id);
        self.about[fact.local.0 as usize].push(id);
        for input in &fact.inputs {
            self.ended_by[input.0 as usize].push(id);
        }
        if let Known::Computed(rvalue) = &fact.known {
            if let Some(first) = first_input(rvalue) {
                self.computations.entry(first).or_default().push(id);
            }
        }
        self.all.push(fact);
        id
    }

    /// Step over a statement: what it writes is no longer known, then
    /// what it establishes is.
    fn transfer(&self, holding: &mut BitSet, statement: &Statement, block: usize, index: usize) {
        for written in Effect::of_statement(statement).writes {
            for &fact in &self.ended_by[written.0 as usize] {
                holding.remove(fact);
            }
        }
        if writes_memory(&self.whole, statement) {
            for &fact in &self.loads {
                holding.remove(fact);
            }
        }
        if let Some(fact) = self.established[block][index] {
            holding.insert(fact);
        }
    }

    /// What holds on the way from `block` to `successor`, given what holds
    /// at the end of `block`.
    fn along_edge(&self, func: &Function, block: usize, successor: BlockId, holding: &BitSet) -> BitSet {
        let mut holding = holding.clone();
        if let (Some((when_true, when_false)), Some(Terminator::Branch { then_block, .. })) =
            (self.branch_outcomes[block], &func.blocks[block].terminator)
        {
            holding.insert(if successor == *then_block { when_true } else { when_false });
        }
        holding
    }

    /// For each block, the facts that hold when it starts.
    fn holding_at_entry(&self, func: &Function) -> Vec<BitSet> {
        let count = self.all.len();
        // A fact holds where it holds on every way in. Start from
        // "everything" and let the paths take away.
        let mut entry = vec![BitSet::full(count); func.blocks.len()];
        entry[0] = BitSet::new(count);
        let order = func.reverse_postorder();
        let mut changed = true;
        while changed {
            changed = false;
            for &block in &order {
                let index = block.0 as usize;
                let mut holding = entry[index].clone();
                for (position, statement) in func.blocks[index].statements.iter().enumerate() {
                    self.transfer(&mut holding, statement, index, position);
                }
                for successor in func.successors(block) {
                    let incoming = self.along_edge(func, index, successor, &holding);
                    changed |= entry[successor.0 as usize].intersect_with(&incoming);
                }
            }
        }
        entry
    }
}

/// The facts holding at one point, and what can be concluded from them.
struct Knowledge<'f> {
    facts: &'f Facts,
    holding: BitSet,
}

impl Knowledge<'_> {
    /// Everything currently known about `local`.
    fn about(&self, local: Local) -> impl Iterator<Item = &Known> + '_ {
        let facts = self.facts.about.get(local.0 as usize).map_or(&[][..], |facts| &facts[..]);
        facts.iter().filter(|&&fact| self.holding.contains(fact)).map(|&fact| &self.facts.all[fact].known)
    }

    fn constant(&self, local: Local) -> Option<&Const> {
        self.about(local).find_map(|known| match known {
            Known::Const(constant) => Some(constant),
            _ => None,
        })
    }

    /// The local with the oldest copy of `local`'s current value.
    fn original(&self, mut local: Local) -> Local {
        // Each step goes to a strictly earlier assignment; the bound only
        // guards against a mistake turning this into an endless loop.
        for _ in 0..self.facts.all.len() {
            let source = self.about(local).find_map(|known| match known {
                Known::Copy(source) => Some(*source),
                _ => None,
            });
            match source {
                Some(source) => local = source,
                None => break,
            }
        }
        local
    }

    /// A local already holding what `rvalue` computes.
    fn computed_by(&self, rvalue: &Rvalue) -> Option<Local> {
        let candidates = self.facts.computations.get(&first_input(rvalue)?)?;
        candidates.iter().filter(|&&fact| self.holding.contains(fact)).find_map(|&fact| {
            let fact = &self.facts.all[fact];
            matches!(&fact.known, Known::Computed(known) if known == rvalue).then_some(fact.local)
        })
    }

    fn rewrite_operand(&self, operand: &mut Operand) -> bool {
        let Operand::Copy(place) = operand else { return false };
        let Some(local) = place.as_local() else {
            let changed = self.rewrite_place(place);
            // A value already read from the same place.
            let load = Rvalue::Use(Operand::Copy(place.clone()));
            if let Some(holder) = loaded_place(&load).and_then(|_| self.computed_by(&load)) {
                *operand = Operand::Copy(Place::local(holder));
                return true;
            }
            return changed;
        };
        let original = self.original(local);
        if let Some(constant) = self.constant(original).or_else(|| self.constant(local)) {
            *operand = Operand::Const(constant.clone());
            return true;
        }
        place.local = original;
        original != local
    }

    /// Rewrite the locals a place is located by. The place's own local, if
    /// it is the place, is not touched: that would change which place it is.
    fn rewrite_place(&self, place: &mut Place) -> bool {
        let mut changed = false;
        for projection in &mut place.projection {
            if let Projection::Index(index) = projection {
                let original = self.original(*index);
                changed |= original != *index;
                *index = original;
            }
        }
        if place.projection.first() == Some(&Projection::Deref) {
            let pointer = self.original(place.local);
            changed |= pointer != place.local;
            place.local = pointer;
            let address = self.about(pointer).find_map(|known| match known {
                Known::Address(target) => Some(target.clone()),
                _ => None,
            });
            if let Some(target) = address {
                // `*(&target)` is `target`.
                let rest = place.projection.split_off(1);
                *place = target;
                place.projection.extend(rest);
                changed = true;
            }
        }
        changed
    }

    fn rewrite_rvalue(&self, rvalue: &mut Rvalue) -> bool {
        // A half of a fat pointer whose halves are known.
        if let Rvalue::FatData(Operand::Copy(pointer)) | Rvalue::FatExtra(Operand::Copy(pointer)) = rvalue {
            let halves = pointer.as_local().and_then(|local| {
                self.about(self.original(local)).find_map(|known| match known {
                    Known::Fat(data, extra) => Some((data.clone(), extra.clone())),
                    _ => None,
                })
            });
            if let Some((data, extra)) = halves {
                let half = if matches!(rvalue, Rvalue::FatData(_)) { data } else { extra };
                *rvalue = Rvalue::Use(half);
                self.rewrite_rvalue(rvalue);
                return true;
            }
        }
        let mut changed = false;
        match rvalue {
            Rvalue::AddrOf(place) | Rvalue::Discriminant(place) => changed |= self.rewrite_place(place),
            _ => rvalue.each_operand_mut(&mut |operand| changed |= self.rewrite_operand(operand)),
        }
        if let Some(simpler) = fold::simplify(rvalue) {
            *rvalue = simpler;
            changed = true;
        }
        if !matches!(rvalue, Rvalue::Use(Operand::Const(_))) && !matches!(rvalue, Rvalue::Use(Operand::Copy(place)) if place.as_local().is_some()) {
            if let Some(local) = self.computed_by(rvalue) {
                *rvalue = Rvalue::Use(Operand::Copy(Place::local(local)));
                changed = true;
            }
        }
        changed
    }

    fn rewrite_statement(&self, statement: &mut Statement) -> bool {
        // Rewriting a destination that is a local on its own leaves it as
        // it is; one written through a pointer has that pointer rewritten.
        match statement {
            Statement::Assign(place, rvalue) => self.rewrite_rvalue(rvalue) | self.rewrite_place(place),
            Statement::SetDiscriminant(place, _) => self.rewrite_place(place),
            Statement::Call { dest, callee, args } => {
                let mut changed = false;
                if let Callee::Indirect(pointer) = callee {
                    changed |= self.rewrite_operand(pointer);
                }
                for arg in args {
                    changed |= self.rewrite_operand(arg);
                }
                changed | self.rewrite_place(dest)
            }
        }
    }
}

/// Returns whether anything changed.
pub fn run(func: &mut Function, tcx: &Context) -> bool {
    let whole = func.whole_locals();
    let facts = Facts::collect(func, tcx, &whole);
    if facts.all.is_empty() {
        return false;
    }
    let entry = facts.holding_at_entry(func);

    let mut changed = false;
    for (index, holding) in entry.into_iter().enumerate() {
        let mut knowledge = Knowledge { facts: &facts, holding };
        let block = &mut func.blocks[index];
        for (position, statement) in block.statements.iter_mut().enumerate() {
            changed |= knowledge.rewrite_statement(statement);
            facts.transfer(&mut knowledge.holding, statement, index, position);
        }
        if let Some(terminator) = &mut block.terminator {
            terminator.each_operand_mut(&mut |operand| changed |= knowledge.rewrite_operand(operand));
        }
    }
    changed
}
