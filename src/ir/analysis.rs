//! Facts about a function that passes and the backend build on: the shape
//! of its control flow, which locals are live where, and how deeply each
//! block is nested in loops.

use super::visit::LocalUse;
use super::*;

/// A set of small integers.
#[derive(Clone, PartialEq, Eq)]
pub struct BitSet {
    words: Vec<u64>,
}

impl BitSet {
    pub fn new(capacity: usize) -> BitSet {
        BitSet { words: vec![0; capacity.div_ceil(64)] }
    }

    /// The set of all of `0..capacity`.
    pub fn full(capacity: usize) -> BitSet {
        let mut words = vec![u64::MAX; capacity.div_ceil(64)];
        if capacity % 64 != 0 {
            *words.last_mut().expect("a partial word exists") = (1 << (capacity % 64)) - 1;
        }
        BitSet { words }
    }

    /// Returns whether the element was absent.
    pub fn insert(&mut self, element: usize) -> bool {
        let (word, bit) = (element / 64, 1 << (element % 64));
        let absent = self.words[word] & bit == 0;
        self.words[word] |= bit;
        absent
    }

    pub fn remove(&mut self, element: usize) {
        self.words[element / 64] &= !(1 << (element % 64));
    }

    pub fn contains(&self, element: usize) -> bool {
        self.words[element / 64] & (1 << (element % 64)) != 0
    }

    /// Add every element of `other`. Returns whether anything was added.
    pub fn union_with(&mut self, other: &BitSet) -> bool {
        let mut changed = false;
        for (mine, theirs) in self.words.iter_mut().zip(&other.words) {
            changed |= *theirs & !*mine != 0;
            *mine |= *theirs;
        }
        changed
    }

    /// Keep only the elements also in `other`. Returns whether any were dropped.
    pub fn intersect_with(&mut self, other: &BitSet) -> bool {
        let mut changed = false;
        for (mine, theirs) in self.words.iter_mut().zip(&other.words) {
            changed |= *mine & !*theirs != 0;
            *mine &= *theirs;
        }
        changed
    }

    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.words.iter().enumerate().flat_map(|(index, &word)| {
            (0..64).filter(move |bit| word & (1 << bit) != 0).map(move |bit| index * 64 + bit)
        })
    }
}

impl Function {
    pub fn successors(&self, block: BlockId) -> Vec<BlockId> {
        match &self.blocks[block.0 as usize].terminator {
            Some(terminator) => terminator.successors(),
            None => Vec::new(),
        }
    }

    pub fn predecessors(&self) -> Vec<Vec<BlockId>> {
        let mut predecessors = vec![Vec::new(); self.blocks.len()];
        for index in 0..self.blocks.len() {
            let block = BlockId(index as u32);
            for successor in self.successors(block) {
                predecessors[successor.0 as usize].push(block);
            }
        }
        predecessors
    }

    /// The blocks reachable from the entry, each after all its predecessors
    /// except along loop back edges.
    pub fn reverse_postorder(&self) -> Vec<BlockId> {
        let mut postorder = Vec::with_capacity(self.blocks.len());
        let mut visited = BitSet::new(self.blocks.len());
        // (block, next successor to look at): an explicit stack, as
        // functions can be long chains of blocks.
        let mut stack = vec![(BlockId(0), 0)];
        visited.insert(0);
        while let Some((block, next)) = stack.last_mut() {
            let successors = self.successors(*block);
            match successors.get(*next) {
                Some(&successor) => {
                    *next += 1;
                    if visited.insert(successor.0 as usize) {
                        stack.push((successor, 0));
                    }
                }
                None => {
                    postorder.push(*block);
                    stack.pop();
                }
            }
        }
        postorder.reverse();
        postorder
    }

    /// The locals whose value is only ever read or replaced as a whole and
    /// whose address is never taken. Nothing can observe where such a local
    /// is kept, so it may live in a register, be renamed or disappear.
    pub fn whole_locals(&self) -> BitSet {
        let mut whole = BitSet::new(self.locals.len());
        (0..self.locals.len()).for_each(|local| {
            whole.insert(local);
        });
        let mut note = |local: Local, how: LocalUse| {
            if how == LocalUse::InMemory {
                whole.remove(local.0 as usize);
            }
        };
        for block in &self.blocks {
            block.statements.iter().for_each(|statement| statement.each_local(&mut note));
            if let Some(terminator) = &block.terminator {
                terminator.each_local(&mut note);
            }
        }
        whole
    }
}

/// The locals a statement or terminator reads and the ones it replaces.
#[derive(Default)]
pub struct Effect {
    pub reads: Vec<Local>,
    pub writes: Vec<Local>,
}

impl Effect {
    fn note(&mut self, local: Local, how: LocalUse) {
        match how {
            LocalUse::Read => self.reads.push(local),
            LocalUse::Write => self.writes.push(local),
            LocalUse::InMemory => {}
        }
    }

    pub fn of_statement(statement: &Statement) -> Effect {
        let mut effect = Effect::default();
        statement.each_local(&mut |local, how| effect.note(local, how));
        effect
    }

    pub fn of_terminator(terminator: &Terminator) -> Effect {
        let mut effect = Effect::default();
        terminator.each_local(&mut |local, how| effect.note(local, how));
        effect
    }
}

/// Can the statement change memory that a pointer may point to? Only
/// writes to the function's own locals, whose address is never taken
/// (`whole`, see [`Function::whole_locals`]), cannot.
pub fn writes_memory(whole: &BitSet, statement: &Statement) -> bool {
    match statement {
        Statement::Call { .. } => true,
        Statement::Assign(place, _) | Statement::SetDiscriminant(place, _) => {
            !whole.contains(place.local.0 as usize) || place.projection.contains(&Projection::Deref)
        }
    }
}

/// For each block, the locals whose current value may still be read after
/// the block ends. Only the locals in `tracked` are followed.
pub struct Liveness {
    pub live_out: Vec<BitSet>,
    tracked: BitSet,
}

impl Liveness {
    pub fn compute(func: &Function, tracked: &BitSet) -> Liveness {
        let empty = BitSet::new(func.locals.len());
        let mut liveness = Liveness { live_out: vec![empty.clone(); func.blocks.len()], tracked: tracked.clone() };
        let mut live_in = vec![empty; func.blocks.len()];

        // Liveness flows against the direction of control, so visiting
        // blocks last-to-first settles it in few rounds.
        let order: Vec<BlockId> = func.reverse_postorder().into_iter().rev().collect();
        let mut changed = true;
        while changed {
            changed = false;
            for &block in &order {
                let index = block.0 as usize;
                for successor in func.successors(block) {
                    let incoming = live_in[successor.0 as usize].clone();
                    liveness.live_out[index].union_with(&incoming);
                }
                let mut live = liveness.live_out[index].clone();
                liveness.walk_block(func, block, &mut live, &mut |_, _, _| {});
                if live != live_in[index] {
                    live_in[index] = live;
                    changed = true;
                }
            }
        }
        liveness
    }

    /// Step backwards through a block. `visit` sees each terminator and
    /// statement (as a statement index, the terminator being one past the
    /// last) with its effect and the set of locals live just after it.
    /// `live` must start as the block's live-out set and ends as its
    /// live-in set.
    pub fn walk_block(
        &self,
        func: &Function,
        block: BlockId,
        live: &mut BitSet,
        visit: &mut dyn FnMut(usize, &Effect, &BitSet),
    ) {
        let block = &func.blocks[block.0 as usize];
        let mut step = |index: usize, effect: Effect, live: &mut BitSet| {
            visit(index, &effect, live);
            for local in &effect.writes {
                live.remove(local.0 as usize);
            }
            for local in &effect.reads {
                if self.tracked.contains(local.0 as usize) {
                    live.insert(local.0 as usize);
                }
            }
        };
        if let Some(terminator) = &block.terminator {
            step(block.statements.len(), Effect::of_terminator(terminator), live);
        }
        for (index, statement) in block.statements.iter().enumerate().rev() {
            step(index, Effect::of_statement(statement), live);
        }
    }
}

/// For each block, the block that every path from the entry to it goes
/// through last (its immediate dominator); the entry's is itself.
/// Unreachable blocks have none.
pub fn immediate_dominators(func: &Function) -> Vec<Option<BlockId>> {
    // Cooper, Harvey and Kennedy, "A Simple, Fast Dominance Algorithm".
    let order = func.reverse_postorder();
    let mut position = vec![usize::MAX; func.blocks.len()];
    for (index, block) in order.iter().enumerate() {
        position[block.0 as usize] = index;
    }
    let predecessors = func.predecessors();
    let mut idom: Vec<Option<BlockId>> = vec![None; func.blocks.len()];
    idom[0] = Some(BlockId(0));

    let intersect = |idom: &[Option<BlockId>], mut a: BlockId, mut b: BlockId| {
        while a != b {
            while position[a.0 as usize] > position[b.0 as usize] {
                a = idom[a.0 as usize].expect("processed blocks have a dominator");
            }
            while position[b.0 as usize] > position[a.0 as usize] {
                b = idom[b.0 as usize].expect("processed blocks have a dominator");
            }
        }
        a
    };

    let mut changed = true;
    while changed {
        changed = false;
        for &block in order.iter().skip(1) {
            let mut processed = predecessors[block.0 as usize].iter().filter(|p| idom[p.0 as usize].is_some());
            let Some(&first) = processed.next() else { continue };
            let dominator = processed.fold(first, |a, &b| intersect(&idom, a, b));
            if idom[block.0 as usize] != Some(dominator) {
                idom[block.0 as usize] = Some(dominator);
                changed = true;
            }
        }
    }
    idom
}

/// A loop: a header that every way into the loop goes through, and the
/// blocks that can return to the header without leaving through it.
pub struct Loop {
    pub header: BlockId,
    pub body: BitSet,
}

/// The function's loops. Loops that share a header are one loop; a loop
/// nested in another comes before it.
pub fn natural_loops(func: &Function) -> Vec<Loop> {
    let idom = immediate_dominators(func);
    let dominates = |dominator: BlockId, mut block: BlockId| loop {
        if block == dominator {
            return true;
        }
        match idom[block.0 as usize] {
            Some(parent) if parent != block => block = parent,
            _ => return false,
        }
    };
    let predecessors = func.predecessors();

    // A jump back to a block that dominates its source closes a loop. The
    // loop's body is everything that can reach that jump without leaving
    // through the header.
    let mut bodies: Vec<Option<BitSet>> = vec![None; func.blocks.len()];
    for index in 0..func.blocks.len() {
        let source = BlockId(index as u32);
        if idom[index].is_none() {
            continue;
        }
        for header in func.successors(source) {
            if !dominates(header, source) {
                continue;
            }
            let body = bodies[header.0 as usize].get_or_insert_with(|| BitSet::new(func.blocks.len()));
            body.insert(header.0 as usize);
            let mut pending = vec![source];
            while let Some(block) = pending.pop() {
                if body.insert(block.0 as usize) {
                    pending.extend(predecessors[block.0 as usize].iter().copied());
                }
            }
        }
    }
    let mut loops: Vec<Loop> = bodies
        .into_iter()
        .enumerate()
        .filter_map(|(header, body)| Some(Loop { header: BlockId(header as u32), body: body? }))
        .collect();
    loops.sort_by_key(|found| found.body.iter().count());
    loops
}

/// How many loops each block is inside of.
pub fn loop_depths(func: &Function) -> Vec<u32> {
    let mut depths = vec![0; func.blocks.len()];
    for found in natural_loops(func) {
        found.body.iter().for_each(|block| depths[block] += 1);
    }
    depths
}
