//! Ownership at run time: running destructors when values go out of scope.
//!
//! Every local whose type needs dropping is *owned* by a scope and has a
//! hidden boolean, its drop flag: set when the local receives a value,
//! cleared when the value is moved out. Leaving a scope drops, in reverse
//! order of declaration, each of its locals whose flag is still set.
//!
//! Flags make every decision at run time. That is always correct, also when
//! a value is moved on only one of several paths; deciding statically where
//! possible is an optimisation left to a later pass.
//!
//! A field moved out of a local on its own (`let name = person.name;`, or
//! `pair().1`) gets a flag of its own, set while it is moved out. Dropping
//! the local then drops its fields one at a time, skipping those moved out.
//! Rust forbids moving out of a value whose type implements `Drop`, so
//! nothing is lost by not calling destructors on the way.
//!
//! Dropping a value means calling its type's *drop glue*, a function
//! generated per type: it runs the type's own `Drop::drop` if it has one,
//! then drops the fields.

use super::{FnBuilder, FuncKey, ProgramBuilder};
use crate::ir::*;
use crate::sema::defs::FnOwner;
use crate::sema::thir::{self, Expr, ExprKind};
use crate::sema::ty::{Mutability, Ty};
use crate::syntax::ast::BinOp;
use crate::syntax::diagnostic::Result;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// A block (or function body, match arm, ...): owns variables.
    Block,
    /// One statement: owns the temporaries created while evaluating it.
    Statement,
}

pub struct Scope {
    kind: ScopeKind,
    /// The locals to drop when the scope ends, in declaration order.
    owned: Vec<Local>,
}

impl FnBuilder<'_, '_, '_> {
    pub fn push_scope(&mut self, kind: ScopeKind) {
        self.scopes.push(Scope { kind, owned: Vec::new() });
    }

    /// Leave the innermost scope, dropping what it still owns.
    pub fn exit_scope(&mut self) -> Result<()> {
        let scope = self.scopes.pop().expect("exit_scope without a matching push_scope");
        for &local in scope.owned.iter().rev() {
            self.drop_if_owned(local)?;
        }
        Ok(())
    }

    /// Drop what the scopes from `depth` inward own, without leaving them:
    /// the code for a `break`, `continue` or `return` that jumps out.
    pub fn drop_scopes_from(&mut self, depth: usize) -> Result<()> {
        let owned: Vec<Local> = self.scopes[depth..].iter().flat_map(|scope| scope.owned.iter().copied()).collect();
        for &local in owned.iter().rev() {
            self.drop_if_owned(local)?;
        }
        Ok(())
    }

    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    /// Make the innermost scope of the given kind responsible for `local`.
    /// The local counts as not holding a value until
    /// [`set_initialized`](Self::set_initialized) says otherwise.
    pub fn own(&mut self, local: Local, kind: ScopeKind) {
        if !self.tcx.needs_drop(&self.locals[local.0 as usize].ty) {
            return;
        }
        let flag = self.new_local(Ty::Bool, None);
        self.flags.insert(local, flag);
        let scope = match kind {
            ScopeKind::Statement => self.scopes.last_mut(),
            ScopeKind::Block => self.scopes.iter_mut().rev().find(|scope| scope.kind == ScopeKind::Block),
        };
        scope.expect("the function body is a block scope").owned.push(local);
    }

    fn set_flag(&mut self, local: Local, value: bool) {
        if let Some(&flag) = self.flags.get(&local) {
            let value = Operand::Const(Const::Int(value as u128, Ty::Bool));
            self.assign(Place::local(flag), Rvalue::Use(value));
        }
    }

    /// `local` now holds a value that must be dropped, all of it.
    pub fn set_initialized(&mut self, local: Local) {
        self.set_flag(local, true);
        self.parts_returned(local, &[]);
    }

    /// The parts of `local` at or inside `path` hold values again.
    fn parts_returned(&mut self, local: Local, path: &[usize]) {
        let flags: Vec<Local> = match self.moved_parts.get(&local) {
            Some(parts) => parts.iter().filter(|(part, _)| part.starts_with(path)).map(|&(_, flag)| flag).collect(),
            None => return,
        };
        for flag in flags {
            self.assign(Place::local(flag), Rvalue::Use(Operand::Const(Const::Int(0, Ty::Bool))));
        }
    }

    /// The field at `path` inside `local` has been moved out on its own.
    fn part_moved(&mut self, local: Local, path: Vec<usize>) {
        let existing = self.moved_parts.get(&local).and_then(|parts| parts.iter().find(|(part, _)| *part == path));
        let flag = match existing {
            Some(&(_, flag)) => flag,
            None => {
                let flag = self.new_local(Ty::Bool, None);
                self.moved_parts.entry(local).or_default().push((path, flag));
                flag
            }
        };
        self.assign(Place::local(flag), Rvalue::Use(Operand::Const(Const::Int(1, Ty::Bool))));
    }

    /// If `place` is a field (of a field...) of a local with a drop flag,
    /// that local, the field indices leading to the place, and its type; a
    /// field can be moved out on its own only through structs, tuples and
    /// closures without destructors of their own.
    pub fn local_part(&self, place: &Place) -> Option<(Local, Vec<usize>, Ty)> {
        if !self.flags.contains_key(&place.local) {
            return None;
        }
        let mut ty = self.locals[place.local.0 as usize].ty.clone();
        let mut path = Vec::new();
        for projection in &place.projection {
            let Projection::Field(index) = projection else { return None };
            let separable = matches!(ty, Ty::Tuple(_) | Ty::Closure(_))
                || matches!(ty, Ty::Adt(..) if !self.tcx.is_enum_ty(&ty) && self.tcx.drop_impl(&ty).is_none());
            if !separable {
                return None;
            }
            ty = self.tcx.field_types(&ty, None).swap_remove(*index);
            path.push(*index);
        }
        Some((place.local, path, ty))
    }

    /// The value of `local` has been moved elsewhere.
    pub fn set_moved(&mut self, local: Local) {
        self.set_flag(local, false);
    }

    /// Drop the value in `local` if it (still) holds one.
    pub fn drop_if_owned(&mut self, local: Local) -> Result<()> {
        if !self.flags.contains_key(&local) {
            return Ok(());
        }
        self.drop_part_if_owned(Place::local(local))?;
        self.set_moved(local);
        Ok(())
    }

    /// Drop what a local, or a field of one (see [`local_part`]), still
    /// holds; it holds nothing afterwards.
    ///
    /// [`local_part`]: Self::local_part
    pub fn drop_part_if_owned(&mut self, place: Place) -> Result<()> {
        let Some((local, path, ty)) = self.local_part(&place) else { unreachable!("a part of a local with a drop flag") };
        let flag = self.flags[&local];
        let moved = self.moved_parts.get(&local).cloned().unwrap_or_default();
        let (dropping, after) = (self.new_block(), self.new_block());
        self.branch(Operand::Copy(Place::local(flag)), dropping, after);
        self.switch_to(dropping);
        self.drop_unless_moved(place, &ty, &mut path.clone(), &moved)?;
        self.goto(after);
        self.switch_to(after);
        self.parts_returned(local, &path);
        Ok(())
    }

    /// Drop the value at `place`, which is at `path` in its local, except
    /// for the parts of it that `moved` says are moved out.
    fn drop_unless_moved(&mut self, place: Place, ty: &Ty, path: &mut Vec<usize>, moved: &[(Vec<usize>, Local)]) -> Result<()> {
        let Some(&(_, flag)) = moved.iter().find(|(part, _)| part == path) else {
            return self.drop_fields_unless_moved(place, ty, path, moved);
        };
        let (held, after) = (self.new_block(), self.new_block());
        self.branch(Operand::Copy(Place::local(flag)), after, held);
        self.switch_to(held);
        self.drop_fields_unless_moved(place, ty, path, moved)?;
        self.goto(after);
        self.switch_to(after);
        Ok(())
    }

    fn drop_fields_unless_moved(&mut self, place: Place, ty: &Ty, path: &mut Vec<usize>, moved: &[(Vec<usize>, Local)]) -> Result<()> {
        let inside = moved.iter().any(|(part, _)| part.len() > path.len() && part.starts_with(path));
        if !inside {
            return self.drop_place(place, ty);
        }
        // A field inside was moved out: the rest goes field by field.
        for (index, field_ty) in self.tcx.field_types(ty, None).into_iter().enumerate() {
            path.push(index);
            self.drop_unless_moved(place.clone().field(index), &field_ty, path, moved)?;
            path.pop();
        }
        Ok(())
    }

    /// Drop the value at `place` if the flag `guard` is set.
    pub fn drop_place_if(&mut self, place: Place, ty: &Ty, guard: Local) -> Result<()> {
        let (dropping, after) = (self.new_block(), self.new_block());
        self.branch(Operand::Copy(Place::local(guard)), dropping, after);
        self.switch_to(dropping);
        self.drop_place(place, ty)?;
        self.goto(after);
        self.switch_to(after);
        Ok(())
    }

    /// Drop the value at `place`, unconditionally.
    pub fn drop_place(&mut self, place: Place, ty: &Ty) -> Result<()> {
        if !self.tcx.needs_drop(ty) {
            return Ok(());
        }
        let glue = self.program.drop_glue(ty);
        let pointer = self.temp(Ty::Ptr(Box::new(ty.clone()), Mutability::Mut));
        let address = self.address_of(place);
        self.assign(Place::local(pointer), address);
        let unit = self.temp(Ty::UNIT);
        self.push(Statement::Call {
            dest: Place::local(unit),
            callee: Callee::Direct(glue),
            args: vec![Operand::Copy(Place::local(pointer))],
        });
        Ok(())
    }

    /// The local that owns the value a place expression reads from, when
    /// moving out of that place must be recorded: a variable itself, a field
    /// of one, or the contents of a `Box` held in one. Values reached
    /// through references and raw pointers are not owned here.
    pub fn owner_of(&self, expr: &Expr) -> Option<Local> {
        match &expr.kind {
            ExprKind::Local(id) => self.vars.get(id).and_then(Place::as_local),
            ExprKind::Field(base, _) => self.owner_of(base),
            ExprKind::Deref(pointer) => self.owner_of(self.box_behind_deref(pointer)?),
            _ => None,
        }
    }

    /// If `pointer` is `Deref::deref(&some_box)` (or `deref_mut`), the box.
    fn box_behind_deref<'e>(&self, pointer: &'e Expr) -> Option<&'e Expr> {
        let ExprKind::Call(thir::Callee::Fn(instance), args) = &pointer.kind else { return None };
        let FnOwner::Impl(impl_id) = self.tcx.defs.fn_def(instance.def).owner else { return None };
        let deref_trait = self.tcx.defs.impl_def(impl_id).trait_id?;
        let lang = &self.tcx.lang;
        if Some(deref_trait) != lang.deref && Some(deref_trait) != lang.deref_mut {
            return None;
        }
        match &args.first()?.kind {
            ExprKind::AddrOf(_, base) if matches!(&base.ty, Ty::Adt(adt, _) if Some(*adt) == lang.boxed) => Some(base),
            _ => None,
        }
    }

    /// A value has been read out of the place `expr` denotes. If that was a
    /// move (the type needs dropping), its owner no longer has it.
    ///
    /// Moving a part out of a variable gives up the whole variable: what
    /// remains of it is not dropped. That can leak, but never frees twice.
    pub fn moved_out_of(&mut self, expr: &Expr, place: &Place) {
        if self.tcx.needs_drop(&expr.ty) {
            if let Some((local, path, _)) = self.local_part(place) {
                if path.is_empty() {
                    self.set_moved(local);
                } else {
                    self.part_moved(local, path);
                }
            } else if let Some(owner) = self.owner_of(expr) {
                self.set_moved(owner);
            } else if let Some(flag) = captured_root(expr).and_then(|local| self.capture_flags.get(&local)) {
                // The closure no longer holds this capture.
                let flag = flag.clone();
                self.assign(flag, Rvalue::Use(Operand::Const(Const::Int(0, Ty::Bool))));
            }
        }
    }

    /// Start every drop flag out as "holds nothing".
    pub fn clear_flags_at_entry(&mut self) {
        let mut flags: Vec<Local> = self.flags.values().copied().collect();
        flags.sort();
        let cleared = flags.into_iter().map(|flag| {
            Statement::Assign(Place::local(flag), Rvalue::Use(Operand::Const(Const::Int(0, Ty::Bool))))
        });
        self.blocks[0].statements.splice(0..0, cleared);
    }
}

impl ProgramBuilder<'_, '_> {
    /// The function that drops a value of type `ty` given a pointer to it.
    pub fn drop_glue(&mut self, ty: &Ty) -> FuncId {
        self.func_id(FuncKey::DropGlue(ty.clone()))
    }

    pub(super) fn build_drop_glue(&mut self, ty: &Ty) -> Result<Function> {
        let tcx = self.tcx;
        let mut glue = Glue {
            locals: vec![
                LocalDecl { ty: Ty::UNIT, name: None },
                LocalDecl { ty: Ty::Ptr(Box::new(ty.clone()), Mutability::Mut), name: Some("object".to_string()) },
            ],
            blocks: vec![Block::default()],
        };
        let object = Place::local(Local(1)).deref();

        // The type's own destructor runs first, while its fields are intact.
        if let Some(destructor) = tcx.drop_impl(ty) {
            let destructor = self.func_id(FuncKey::Instance(destructor));
            glue.call(0, destructor, Operand::Copy(Place::local(Local(1))));
        }

        match ty {
            Ty::Adt(adt, _) if tcx.is_enum(*adt) => {
                // Only the fields of the variant the value holds exist.
                let tag_ty = Ty::Int(tcx.tag_type(ty));
                let tag = glue.local(tag_ty.clone());
                glue.blocks[0].statements.push(Statement::Assign(Place::local(tag), Rvalue::Discriminant(object.clone())));
                let done = glue.block();
                let mut arms = Vec::new();
                for variant in 0..tcx.defs.adt(*adt).variants.len() as u32 {
                    let fields = tcx.field_types(ty, Some(variant));
                    if !fields.iter().any(|field| tcx.needs_drop(field)) {
                        continue;
                    }
                    let block = glue.block();
                    let payload = object.clone().project(Projection::Downcast(variant));
                    self.drop_fields(&mut glue, block, &payload, &fields);
                    glue.blocks[block].terminator = Some(Terminator::Goto(BlockId(done as u32)));
                    arms.push((tcx.discriminant(*adt, variant) as u128, BlockId(block as u32)));
                }
                glue.blocks[0].terminator = Some(Terminator::Switch {
                    value: Operand::Copy(Place::local(tag)),
                    arms,
                    otherwise: BlockId(done as u32),
                });
                glue.blocks[done].terminator = Some(Terminator::Return);
            }
            Ty::Adt(..) | Ty::Tuple(_) => {
                let fields = tcx.field_types(ty, None);
                self.drop_fields(&mut glue, 0, &object, &fields);
                glue.blocks[0].terminator = Some(Terminator::Return);
            }
            // A closure drops the captures its body has not moved out.
            Ty::Closure(id) => {
                let fields = tcx.field_types(ty, None);
                let mut current = 0;
                for (capture, flag) in tcx.closure_drop_flags(*id) {
                    let (dropping, next) = (glue.block(), glue.block());
                    glue.blocks[current].terminator = Some(Terminator::Branch {
                        cond: Operand::Copy(object.clone().field(flag)),
                        then_block: BlockId(dropping as u32),
                        else_block: BlockId(next as u32),
                    });
                    let field = fields[capture].clone();
                    let pointer = glue.local(Ty::Ptr(Box::new(field.clone()), Mutability::Mut));
                    let address = Rvalue::AddrOf(object.clone().field(capture));
                    glue.blocks[dropping].statements.push(Statement::Assign(Place::local(pointer), address));
                    let field_glue = self.drop_glue(&field);
                    glue.call(dropping, field_glue, Operand::Copy(Place::local(pointer)));
                    glue.blocks[dropping].terminator = Some(Terminator::Goto(BlockId(next as u32)));
                    current = next;
                }
                glue.blocks[current].terminator = Some(Terminator::Return);
            }
            Ty::Array(element, _) if tcx.needs_drop(element) => {
                let len = ty.array_len().expect("array lengths are known after type checking");
                self.drop_elements(&mut glue, &object, element, len);
            }
            _ => glue.blocks[0].terminator = Some(Terminator::Return),
        }

        let name = format!("drop_in_place<{}>", tcx.display(ty));
        Ok(Function { symbol: self.unique_symbol(&name), name, locals: glue.locals, arg_count: 1, blocks: glue.blocks })
    }

    /// In `block`, drop each field of `owner` that needs it.
    fn drop_fields(&mut self, glue: &mut Glue, block: usize, owner: &Place, fields: &[Ty]) {
        for (index, field) in fields.iter().enumerate() {
            if !self.tcx.needs_drop(field) {
                continue;
            }
            let pointer = glue.local(Ty::Ptr(Box::new(field.clone()), Mutability::Mut));
            let address = Rvalue::AddrOf(owner.clone().field(index));
            glue.blocks[block].statements.push(Statement::Assign(Place::local(pointer), address));
            let field_glue = self.drop_glue(field);
            glue.call(block, field_glue, Operand::Copy(Place::local(pointer)));
        }
    }

    /// A loop over the elements of an array, dropping each.
    fn drop_elements(&mut self, glue: &mut Glue, array: &Place, element: &Ty, len: u64) {
        let usize_const = |value: u64| Operand::Const(Const::Int(value as u128, Ty::USIZE));
        let index = glue.local(Ty::USIZE);
        let more = glue.local(Ty::Bool);
        let pointer = glue.local(Ty::Ptr(Box::new(element.clone()), Mutability::Mut));
        let (head, body, done) = (glue.block(), glue.block(), glue.block());
        let index_value = Operand::Copy(Place::local(index));

        glue.blocks[0].statements.push(Statement::Assign(Place::local(index), Rvalue::Use(usize_const(0))));
        glue.blocks[0].terminator = Some(Terminator::Goto(BlockId(head as u32)));

        let in_range = Rvalue::Binary(BinOp::Lt, index_value.clone(), usize_const(len));
        glue.blocks[head].statements.push(Statement::Assign(Place::local(more), in_range));
        glue.blocks[head].terminator = Some(Terminator::Branch {
            cond: Operand::Copy(Place::local(more)),
            then_block: BlockId(body as u32),
            else_block: BlockId(done as u32),
        });

        let address = Rvalue::AddrOf(array.clone().project(Projection::Index(index)));
        glue.blocks[body].statements.push(Statement::Assign(Place::local(pointer), address));
        let element_glue = self.drop_glue(element);
        glue.call(body, element_glue, Operand::Copy(Place::local(pointer)));
        let next = Rvalue::Binary(BinOp::Add, index_value, usize_const(1));
        glue.blocks[body].statements.push(Statement::Assign(Place::local(index), next));
        glue.blocks[body].terminator = Some(Terminator::Goto(BlockId(head as u32)));
        glue.blocks[done].terminator = Some(Terminator::Return);
    }
}

/// The body of a drop glue function under construction.
struct Glue {
    locals: Vec<LocalDecl>,
    blocks: Vec<Block>,
}

impl Glue {
    fn local(&mut self, ty: Ty) -> Local {
        self.locals.push(LocalDecl { ty, name: None });
        Local(self.locals.len() as u32 - 1)
    }

    fn block(&mut self) -> usize {
        self.blocks.push(Block::default());
        self.blocks.len() - 1
    }

    /// Call a one-argument function for its effect.
    fn call(&mut self, block: usize, function: FuncId, argument: Operand) {
        self.blocks[block].statements.push(Statement::Call {
            dest: Place::local(RETURN_LOCAL),
            callee: Callee::Direct(function),
            args: vec![argument],
        });
    }
}

/// The variable an expression such as `pair.0.name` takes a part of.
fn captured_root(expr: &Expr) -> Option<thir::LocalId> {
    match &expr.kind {
        ExprKind::Local(local) => Some(*local),
        ExprKind::Field(base, _) => captured_root(base),
        _ => None,
    }
}
