//! Lowering the typed tree to IR.
//!
//! Starting from `main`, every function that is reachable gets built once
//! per set of concrete type arguments it is used with (monomorphisation).
//! Building one function turns its expression tree into basic blocks:
//!
//! * [`expr`] — expressions, statements and control flow
//! * [`call`] — calls, compiler intrinsics, closures and vtables
//! * [`pattern`] — matching a value against a pattern
//! * [`drop`] — scopes, and dropping values when they end

mod call;
mod drop;
mod expr;
mod pattern;

use super::*;
use crate::sema::context::Context;
use crate::sema::defs::{ConstId, Def, FnKind, CRATE_ROOT};
use crate::sema::thir::{self, Instance, LocalId, LoopId};
use crate::sema::ty::{ClosureId, TraitId, Ty};
use crate::syntax::diagnostic::{bail, Diagnostic, Result};
use drop::ScopeKind;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

/// Build the IR of the whole program.
pub fn build(tcx: &Context) -> Result<Program> {
    let Some(Def::Fn(main)) = tcx.defs.module(CRATE_ROOT).names.get("main").copied() else {
        return Err(Diagnostic::global("no `main` function found in the crate root"));
    };
    let main_def = tcx.defs.fn_def(main);
    if !main_def.ast.generics.params.is_empty() || !main_def.ast.params.is_empty() {
        bail!(main_def.ast.name.span, "`main` must take no arguments and have no generic parameters");
    }

    let mut program = ProgramBuilder {
        tcx,
        functions: Vec::new(),
        func_ids: HashMap::new(),
        queue: VecDeque::new(),
        data: Vec::new(),
        strings: HashMap::new(),
        vtables: HashMap::new(),
        statics: HashMap::new(),
        initializers: Vec::new(),
        symbols: HashMap::new(),
    };
    let entry = program.func_id(FuncKey::Instance(Instance { def: main, substs: Vec::new() }));
    while let Some((id, key)) = program.queue.pop_front() {
        let function = program.build_function(&key)?;
        program.functions[id.0 as usize] = Some(function);
    }

    let functions = program.functions.into_iter().map(|f| f.expect("every queued function is built")).collect();
    Ok(Program { functions, data: program.data, entry, initializers: program.initializers })
}

/// Something that becomes one function in the output.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum FuncKey {
    Instance(Instance),
    Closure(ClosureId),
    /// A capture-free closure wrapped so it can be called through a plain
    /// function pointer (which passes no environment).
    ClosureAsFnPointer(ClosureId),
    /// A plain function wrapped so it can be called like a closure (which
    /// is passed an environment).
    FnItemAsClosure(Instance),
    /// The function that drops a value of a type in place.
    DropGlue(Ty),
    /// The function that computes a static's value when the program starts,
    /// for a static whose value is not plain data.
    StaticInit(ConstId),
}

pub(super) struct ProgramBuilder<'c, 'a> {
    pub tcx: &'c Context<'a>,
    functions: Vec<Option<Function>>,
    func_ids: HashMap<FuncKey, FuncId>,
    /// Functions that have an id but are not built yet.
    queue: VecDeque<(FuncId, FuncKey)>,
    data: Vec<Data>,
    strings: HashMap<Vec<u8>, DataId>,
    vtables: HashMap<(Ty, TraitId), DataId>,
    statics: HashMap<ConstId, DataId>,
    /// The functions to run before `main`, in order.
    initializers: Vec<FuncId>,
    /// How many times each symbol base name has been handed out.
    symbols: HashMap<String, usize>,
}

impl<'c, 'a> ProgramBuilder<'c, 'a> {
    /// The id of a function, scheduling it to be built on first request.
    pub fn func_id(&mut self, key: FuncKey) -> FuncId {
        if let Some(&id) = self.func_ids.get(&key) {
            return id;
        }
        let id = FuncId(self.functions.len() as u32);
        self.functions.push(None);
        self.func_ids.insert(key.clone(), id);
        self.queue.push_back((id, key));
        id
    }

    pub fn add_data(&mut self, name: &str, items: Vec<DataItem>, align: u64, writable: bool) -> DataId {
        let symbol = self.unique_symbol(name);
        self.data.push(Data { symbol, items, align, writable });
        DataId(self.data.len() as u32 - 1)
    }

    /// Constant bytes; identical contents share one copy.
    pub fn bytes(&mut self, bytes: &[u8]) -> DataId {
        if let Some(&id) = self.strings.get(bytes) {
            return id;
        }
        let id = self.add_data("str", vec![DataItem::Bytes(bytes.to_vec())], 1, false);
        self.strings.insert(bytes.to_vec(), id);
        id
    }

    /// A linker symbol derived from a readable name. Symbols get a prefix
    /// that keeps them apart from C library names and assembler keywords.
    fn unique_symbol(&mut self, name: &str) -> String {
        let mut symbol = String::from("_G.");
        for c in name.chars() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '.' => symbol.push(c),
                ':' => symbol.push('.'),
                '<' => symbol.push_str(".of."),
                '>' | ' ' => {}
                ',' => symbol.push_str(".and."),
                '&' => symbol.push_str("ref."),
                '*' => symbol.push_str("ptr."),
                other => symbol.push_str(&format!(".x{:x}.", other as u32)),
            }
        }
        let symbol = symbol.replace("..", ".");
        let uses = self.symbols.entry(symbol.clone()).or_insert(0);
        *uses += 1;
        match *uses {
            1 => symbol,
            n => format!("{symbol}.{n}"),
        }
    }

    fn instance_name(&self, instance: &Instance) -> String {
        let path = &self.tcx.defs.fn_def(instance.def).path;
        if instance.substs.is_empty() {
            return path.clone();
        }
        let args: Vec<String> = instance.substs.iter().map(|ty| self.tcx.display(ty)).collect();
        format!("{path}<{}>", args.join(", "))
    }

    fn build_function(&mut self, key: &FuncKey) -> Result<Function> {
        match key {
            FuncKey::Instance(instance) => {
                let def = self.tcx.defs.fn_def(instance.def);
                if def.kind != FnKind::Defined {
                    bail!(def.ast.span, "`{}` has no body and cannot be called here", def.path);
                }
                let body = self.tcx.body(instance)?;
                let name = self.instance_name(instance);
                FnBuilder::new(self, &body, name, None).finish()
            }
            FuncKey::Closure(closure_id) => {
                let closure = self.closure(*closure_id);
                let name = format!("closure#{}", closure_id.0);
                FnBuilder::new(self, &closure.body, name, Some((*closure_id, &*closure))).finish()
            }
            FuncKey::ClosureAsFnPointer(closure_id) => self.build_closure_fn_pointer(*closure_id),
            FuncKey::FnItemAsClosure(instance) => self.build_fn_item_as_closure(instance),
            FuncKey::DropGlue(ty) => self.build_drop_glue(ty),
            FuncKey::StaticInit(id) => self.build_static_init(*id),
        }
    }

    pub fn closure(&self, id: ClosureId) -> Rc<thir::ClosureDef> {
        self.tcx.closure(id).expect("a closure is registered when its defining function is checked")
    }
}

/// Where `break` and `continue` of one loop go.
struct LoopScope {
    id: LoopId,
    break_block: BlockId,
    continue_block: BlockId,
    /// Where `break value` stores the loop's result.
    result: Place,
    /// How many scopes were open when the loop began; leaving the loop
    /// drops what the scopes opened since then own.
    scope_depth: usize,
}

/// Builds the IR of one function.
pub(super) struct FnBuilder<'p, 'c, 'a> {
    pub program: &'p mut ProgramBuilder<'c, 'a>,
    pub tcx: &'c Context<'a>,
    body: &'p thir::Body,
    name: String,
    locals: Vec<LocalDecl>,
    arg_count: usize,
    blocks: Vec<Block>,
    current: BlockId,
    /// The place each variable of the typed tree lives in.
    vars: HashMap<LocalId, Place>,
    loops: Vec<LoopScope>,
    /// The open scopes, innermost last.
    scopes: Vec<drop::Scope>,
    /// The drop flag of each local that needs dropping.
    flags: HashMap<Local, Local>,
    /// For a local some fields of which were moved out on their own: each
    /// such field, as the indices that lead to it, with a flag set while it
    /// is moved out. The rest of the local is still dropped with it.
    moved_parts: HashMap<Local, Vec<(Vec<usize>, Local)>>,
    /// In a closure's body: for each capture it holds by value and must
    /// drop, the flag in its environment saying it still holds it.
    capture_flags: HashMap<thir::LocalId, Place>,
    /// See [`expr_into`](Self::expr_into).
    extend_temporaries: bool,
}

impl<'p, 'c, 'a> FnBuilder<'p, 'c, 'a> {
    fn new(
        program: &'p mut ProgramBuilder<'c, 'a>,
        body: &'p thir::Body,
        name: String,
        closure: Option<(ClosureId, &'p thir::ClosureDef)>,
    ) -> FnBuilder<'p, 'c, 'a> {
        let tcx = program.tcx;
        let mut builder = FnBuilder {
            program,
            tcx,
            body,
            name,
            locals: vec![LocalDecl { ty: body.ret_ty.clone(), name: None }],
            arg_count: 0,
            blocks: vec![Block::default()],
            current: BlockId(0),
            vars: HashMap::new(),
            loops: Vec::new(),
            scopes: Vec::new(),
            flags: HashMap::new(),
            moved_parts: HashMap::new(),
            capture_flags: HashMap::new(),
            extend_temporaries: false,
        };
        builder.push_scope(ScopeKind::Block);

        // A closure's first parameter points to its captured variables.
        if let Some((id, def)) = closure {
            let env = builder.new_local(Ty::mut_ref(Ty::Closure(id)), Some("env"));
            for (index, capture) in def.captures.iter().enumerate() {
                let slot = Place::local(env).deref().field(index);
                let place = if capture.by_ref { slot.deref() } else { slot };
                builder.vars.insert(capture.local, place);
            }
            for (capture, flag) in tcx.closure_drop_flags(id) {
                let flag = Place::local(env).deref().field(flag);
                builder.capture_flags.insert(def.captures[capture].local, flag);
            }
        }
        let mut params = Vec::new();
        for &param in &body.params {
            let decl = &body.locals[param.0 as usize];
            let local = builder.new_local(decl.ty.clone(), Some(&decl.name));
            builder.vars.insert(param, Place::local(local));
            params.push(local);
        }
        builder.arg_count = builder.locals.len() - 1;
        // A function owns its parameters and drops them when it returns.
        for local in params {
            builder.own(local, ScopeKind::Block);
            builder.set_initialized(local);
        }
        builder
    }

    fn finish(self) -> Result<Function> {
        self.finish_into(Place::local(RETURN_LOCAL))
    }

    /// Build the body, storing its value in `dest`.
    fn finish_into(mut self, dest: Place) -> Result<Function> {
        let body = self.body;
        self.expr_into(&body.value, dest)?;
        self.exit_scope()?;
        self.terminate(Terminator::Return);
        self.clear_flags_at_entry();
        // Blocks left open hold code that follows a `return` or `break`.
        for block in &mut self.blocks {
            block.terminator.get_or_insert(Terminator::Unreachable);
        }
        let symbol = self.program.unique_symbol(&self.name);
        Ok(Function {
            symbol,
            name: self.name,
            locals: self.locals,
            arg_count: self.arg_count,
            blocks: self.blocks,
        })
    }

    // -- locals -----------------------------------------------------------------

    pub fn new_local(&mut self, ty: Ty, name: Option<&str>) -> Local {
        self.locals.push(LocalDecl { ty, name: name.map(str::to_string) });
        Local(self.locals.len() as u32 - 1)
    }

    pub fn temp(&mut self, ty: Ty) -> Local {
        self.new_local(ty, None)
    }

    /// The place of a source variable, created the first time it is mentioned.
    pub fn var(&mut self, id: LocalId) -> Place {
        if let Some(place) = self.vars.get(&id) {
            return place.clone();
        }
        let decl = &self.body.locals[id.0 as usize];
        let local = self.new_local(decl.ty.clone(), Some(&decl.name));
        self.vars.insert(id, Place::local(local));
        self.own(local, ScopeKind::Block);
        Place::local(local)
    }

    // -- blocks -------------------------------------------------------------------

    pub fn new_block(&mut self) -> BlockId {
        self.blocks.push(Block::default());
        BlockId(self.blocks.len() as u32 - 1)
    }

    pub fn switch_to(&mut self, block: BlockId) {
        self.current = block;
    }

    pub fn push(&mut self, statement: Statement) {
        self.blocks[self.current.0 as usize].statements.push(statement);
    }

    pub fn assign(&mut self, place: Place, rvalue: Rvalue) {
        self.push(Statement::Assign(place, rvalue));
    }

    /// End the current block. Code that follows (after a `return`, say) can
    /// never run; it goes into a fresh block nothing jumps to.
    pub fn terminate(&mut self, terminator: Terminator) {
        let block = &mut self.blocks[self.current.0 as usize];
        if block.terminator.is_none() {
            block.terminator = Some(terminator);
        }
        self.current = self.new_block();
    }

    /// End the current block with a jump and continue building at its target.
    pub fn goto(&mut self, target: BlockId) {
        let block = &mut self.blocks[self.current.0 as usize];
        if block.terminator.is_none() {
            block.terminator = Some(Terminator::Goto(target));
        }
    }

    pub fn branch(&mut self, cond: Operand, then_block: BlockId, else_block: BlockId) {
        let block = &mut self.blocks[self.current.0 as usize];
        if block.terminator.is_none() {
            block.terminator = Some(Terminator::Branch { cond, then_block, else_block });
        }
    }

    // -- helpers --------------------------------------------------------------------

    pub fn usize_const(&self, value: u64) -> Operand {
        Operand::Const(Const::Int(value as u128, Ty::USIZE))
    }

    /// A function from the standard library's runtime support module.
    pub fn runtime_fn(&mut self, name: &str) -> Result<FuncId> {
        match self.tcx.defs.std_item(&["rt", name]) {
            Some(Def::Fn(def)) => Ok(self.program.func_id(FuncKey::Instance(Instance { def, substs: Vec::new() }))),
            _ => Err(Diagnostic::global(format!("the standard library is missing `rt::{name}`"))),
        }
    }
}
