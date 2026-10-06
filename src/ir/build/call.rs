//! Calls, compiler intrinsics, and the data that calls need: vtables,
//! statics, and function-pointer wrappers for closures.

use super::{FnBuilder, FuncKey, ProgramBuilder};
use crate::ir::layout::TagEncoding;
use crate::ir::*;
use crate::sema::context::{GenericEnv, TypeScope};
use crate::sema::defs::{ConstId, Def, FnKind, ModId, Resolution};
use crate::sema::infer::InferTable;
use crate::sema::thir::{self, Expr, Instance};
use crate::sema::ty::{ClosureId, TraitId, Ty};
use crate::syntax::ast;
use crate::syntax::diagnostic::{bail, Diagnostic, Result};

impl FnBuilder<'_, '_, '_> {
    pub fn call_into(&mut self, callee: &thir::Callee, args: &[Expr], call: &Expr, dest: Place) -> Result<()> {
        let callee = match callee {
            thir::Callee::Fn(instance) => {
                let def = self.tcx.defs.fn_def(instance.def);
                match def.kind {
                    FnKind::Intrinsic => return self.intrinsic_into(instance, args, call, dest),
                    FnKind::Extern => Callee::Extern {
                        symbol: def.ast.name.name.clone(),
                        variadic: def.ast.is_variadic,
                    },
                    FnKind::Defined => Callee::Direct(self.program.func_id(FuncKey::Instance(instance.clone()))),
                    FnKind::Required => {
                        bail!(call.span, "`{}` has no implementation for the type it is called on", def.path)
                    }
                }
            }
            thir::Callee::Pointer(pointer) => Callee::Indirect(self.expr_operand(pointer)?),
            thir::Callee::Closure(id) => Callee::Direct(self.program.func_id(FuncKey::Closure(*id))),
            thir::Callee::Virtual { slot } => Callee::Virtual { index: VTABLE_HEADER_WORDS + *slot },
        };

        let mut operands = Vec::with_capacity(args.len());
        for arg in args {
            operands.push(self.arg_operand(arg)?);
        }
        self.push(Statement::Call { dest, callee, args: operands });
        if call.ty == Ty::Never {
            self.terminate(Terminator::Unreachable);
        }
        Ok(())
    }

    /// An argument value. A callee owns its by-value parameters and may
    /// modify them, so a variable passed in memory is copied first; values
    /// that travel in registers are copies by nature.
    fn arg_operand(&mut self, arg: &Expr) -> Result<Operand> {
        let operand = self.expr_operand(arg)?;
        let in_memory = !self.tcx.is_register_value(&arg.ty) && !self.tcx.is_zero_sized(&arg.ty);
        if arg.is_place() && in_memory {
            let copy = self.temp(arg.ty.clone());
            self.assign(Place::local(copy), Rvalue::Use(operand));
            return Ok(Operand::Copy(Place::local(copy)));
        }
        Ok(operand)
    }

    /// Functions the standard library declares without a body: operations
    /// that cannot be written in the language itself.
    fn intrinsic_into(&mut self, instance: &Instance, args: &[Expr], call: &Expr, dest: Place) -> Result<()> {
        let name = self.tcx.defs.fn_def(instance.def).ast.name.name.as_str();
        let type_arg = || instance.substs.first().expect("intrinsic takes a type argument");
        let mut operands = Vec::new();
        for arg in args {
            operands.push(self.expr_operand(arg)?);
        }
        let mut operands = operands.into_iter();
        let mut next = || operands.next().expect("intrinsic called with too few arguments");

        let value = match name {
            "size_of" => Rvalue::Use(self.usize_const(self.tcx.size_of(type_arg()))),
            "type_name" => {
                let name = self.tcx.type_name(type_arg());
                let data = self.program.bytes(name.as_bytes());
                Rvalue::Use(Operand::Const(Const::Str(data, name.len() as u64)))
            }
            "align_of" => Rvalue::Use(self.usize_const(self.tcx.layout(type_arg()).align)),
            "slice_from_raw_parts" | "slice_from_raw_parts_mut" | "str_from_raw_parts" => {
                let data = next();
                Rvalue::MakeFat(data, next())
            }
            "slice_len" | "str_len" => Rvalue::FatExtra(next()),
            "slice_as_ptr" | "slice_as_mut_ptr" | "str_as_ptr" => Rvalue::FatData(next()),
            "sqrt" => Rvalue::Unary(UnaryOp::Sqrt, next()),
            "needs_drop" => {
                let needs = self.tcx.needs_drop(type_arg());
                Rvalue::Use(Operand::Const(Const::Int(needs as u128, Ty::Bool)))
            }
            // Reading and writing through a raw pointer moves bits and
            // nothing else: no value is dropped, no owner is told.
            "read" => {
                let Operand::Copy(pointer) = next() else {
                    bail!(call.span, "`read` needs a pointer held in a variable");
                };
                Rvalue::Use(Operand::Copy(pointer.deref()))
            }
            "write" => {
                let Operand::Copy(pointer) = next() else {
                    bail!(call.span, "`write` needs a pointer held in a variable");
                };
                let value = next();
                if !self.tcx.is_zero_sized(type_arg()) {
                    self.assign(pointer.deref(), Rvalue::Use(value));
                }
                return Ok(());
            }
            // The argument has been moved in; not dropping it is the point.
            "forget" => return Ok(()),
            // Whatever bits the destination already holds.
            "uninit" => return Ok(()),
            "drop_in_place" => {
                let pointer = next();
                let unit = self.temp(Ty::UNIT);
                match type_arg() {
                    // A trait object is dropped by what its vtable says.
                    Ty::Dyn(..) => self.push(Statement::Call {
                        dest: Place::local(unit),
                        callee: Callee::Virtual { index: VTABLE_DROP },
                        args: vec![pointer],
                    }),
                    pointee if self.tcx.needs_drop(pointee) => {
                        let glue = self.program.drop_glue(pointee);
                        self.push(Statement::Call { dest: Place::local(unit), callee: Callee::Direct(glue), args: vec![pointer] });
                    }
                    _ => {}
                }
                return Ok(());
            }
            "discriminant" => {
                let Operand::Copy(pointer) = next() else {
                    bail!(call.span, "`discriminant` needs a reference to an enum value");
                };
                let tag = self.temp(Ty::Int(self.tcx.tag_type(type_arg())));
                self.assign(Place::local(tag), Rvalue::Discriminant(pointer.deref()));
                let tag_ty = self.locals[tag.0 as usize].ty.clone();
                Rvalue::Cast(Operand::Copy(Place::local(tag)), tag_ty, call.ty.clone())
            }
            "unreachable" => {
                self.terminate(Terminator::Unreachable);
                return Ok(());
            }
            _ => bail!(call.span, "unknown compiler intrinsic `{name}`"),
        };
        self.assign(dest, value);
        Ok(())
    }

    /// The extra word a pointer gains when it is converted to a pointer to
    /// an unsized type: the array's length, or the vtable for the trait.
    pub fn unsize_extra<'t>(&mut self, from: &'t Ty, to: &'t Ty) -> Result<Operand> {
        // For `Box<T>` to `Box<dyn Trait>` the pointees are the type arguments.
        let pointee = |ty: &'t Ty| match ty {
            Ty::Adt(_, args) => args.first(),
            other => other.pointee(),
        };
        match (pointee(from), pointee(to)) {
            (Some(array @ Ty::Array(..)), Some(Ty::Slice(_))) => {
                Ok(self.usize_const(array.array_len().expect("array lengths are known after type checking")))
            }
            (Some(concrete), Some(Ty::Dyn(trait_id, _))) => {
                let vtable = self.program.vtable(concrete, *trait_id)?;
                Ok(Operand::Const(Const::DataAddr(vtable, Ty::USIZE)))
            }
            _ => unreachable!("no unsizing from `{}` to `{}`", self.tcx.display(from), self.tcx.display(to)),
        }
    }
}

impl ProgramBuilder<'_, '_> {
    /// The table of `ty`'s implementations of a trait's methods, through
    /// which calls on `dyn Trait` are dispatched:
    ///
    /// ```text
    /// [ size of ty | alignment of ty | drop glue | method 0 | method 1 | ... ]
    /// ```
    fn vtable(&mut self, ty: &Ty, trait_id: TraitId) -> Result<DataId> {
        let key = (ty.clone(), trait_id);
        if let Some(&id) = self.vtables.get(&key) {
            return Ok(id);
        }
        let layout = self.tcx.layout(ty);
        let drop_glue = self.drop_glue(ty);
        let mut items = vec![DataItem::Word(layout.size), DataItem::Word(layout.align), DataItem::FuncAddr(drop_glue)];
        // A callable trait object has one entry: the call itself.
        if self.tcx.lang.callable_traits.contains(&trait_id) {
            let call = match ty {
                Ty::Closure(id) => FuncKey::Closure(*id),
                Ty::FnItem(def, substs) => {
                    FuncKey::FnItemAsClosure(Instance { def: *def, substs: substs.clone() })
                }
                other => bail!(
                    crate::syntax::Span::default(),
                    "`{}` cannot be made into a callable trait object yet",
                    self.tcx.display(other)
                ),
            };
            items.push(DataItem::FuncAddr(self.func_id(call)));
        }
        for (declaring_trait, method) in self.tcx.vtable_entries(trait_id) {
            // Methods that cannot be called on a trait object keep their
            // slot, so that slot numbers do not depend on the trait's shape.
            items.push(match self.tcx.resolve_trait_method(declaring_trait, method, ty)? {
                Some(instance) => DataItem::FuncAddr(self.func_id(FuncKey::Instance(instance))),
                None => DataItem::Word(0),
            });
        }
        let name = format!("vtable.{}.{}", self.tcx.defs.trait_def(trait_id).name, self.tcx.display(ty));
        let id = self.add_data(&name, items, 8, false);
        self.vtables.insert(key, id);
        Ok(id)
    }

    /// The memory image of a constant of type `ty`, written as `value`:
    /// exactly as many bytes as the type is large, with fields at their
    /// offsets and zeros in between.
    fn constant_data(&mut self, ty: &Ty, value: &ast::Expr, module: ModId) -> Result<Vec<DataItem>> {
        let size = self.tcx.size_of(ty) as usize;
        // A name for another constant stands for its value.
        if let ast::ExprKind::Path(path) = &value.kind {
            if let Some(Resolution { def: Def::Const(other), rest: [] }) = self.tcx.defs.resolve_path(module, path) {
                let other = self.tcx.defs.const_def(other);
                if let Some(other_value) = &other.ast.value {
                    if !matches!(ty, Ty::Adt(..)) || !matches!(other_value.kind, ast::ExprKind::Path(_)) {
                        return self.constant_data(ty, other_value, other.module);
                    }
                }
            }
        }
        Ok(match (ty, &value.kind) {
            (Ty::Int(_) | Ty::Bool | Ty::Char, _) => {
                let bits = crate::sema::const_eval::eval_int(self.tcx, module, value)?;
                vec![DataItem::Bytes(bits.to_le_bytes()[..size].to_vec())]
            }
            (Ty::Float(_), _) => {
                let number = constant_float(value)
                    .ok_or_else(|| Diagnostic::new(value.span, "this kind of float constant is not supported yet"))?;
                match size {
                    4 => vec![DataItem::Bytes((number as f32).to_le_bytes().to_vec())],
                    _ => vec![DataItem::Bytes(number.to_le_bytes().to_vec())],
                }
            }
            (Ty::Ref(pointee, _), ast::ExprKind::Str(text)) if **pointee == Ty::Str => {
                let bytes = self.bytes(text.as_bytes());
                vec![DataItem::DataAddr(bytes), DataItem::Word(text.len() as u64)]
            }
            // `&[a, b, c]`: the array in data of its own.
            (Ty::Ref(pointee, _), ast::ExprKind::AddrOf { expr: inner, .. }) if matches!(**pointee, Ty::Slice(_)) => {
                let Ty::Slice(element) = &**pointee else { unreachable!() };
                let count = match &inner.kind {
                    ast::ExprKind::Array(elements) => elements.len() as u64,
                    ast::ExprKind::Repeat(_, count) => crate::sema::const_eval::eval_int(self.tcx, module, &**count)? as u64,
                    _ => bail!(inner.span, "this kind of constant slice is not supported yet"),
                };
                let array = Ty::array((**element).clone(), count);
                let items = self.constant_data(&array, inner, module)?;
                let align = self.tcx.layout(&array).align;
                let data = self.add_data("constant", items, align, false);
                vec![DataItem::DataAddr(data), DataItem::Word(count)]
            }
            (Ty::Array(element, _), ast::ExprKind::Array(elements)) => {
                let mut items = Vec::new();
                for element_value in elements {
                    items.extend(self.constant_data(element, element_value, module)?);
                }
                items
            }
            (Ty::Array(element, _), ast::ExprKind::Repeat(element_value, _)) => {
                let count = ty.array_len().expect("array lengths are known after type checking");
                let one = self.constant_data(element, element_value, module)?;
                let mut items = Vec::new();
                for _ in 0..count {
                    items.extend(one.iter().cloned());
                }
                items
            }
            (Ty::Tuple(_), ast::ExprKind::Tuple(elements)) => {
                let fields: Vec<&ast::Expr> = elements.iter().collect();
                self.constant_fields(ty, None, &fields, module)?
            }
            (Ty::Adt(adt, _), ast::ExprKind::Struct { fields, base: None, .. }) if !self.tcx.is_enum(*adt) => {
                let variant = &self.tcx.defs.adt(*adt).variants[0];
                let mut ordered: Vec<Option<&ast::Expr>> = vec![None; variant.fields.len()];
                for field in fields {
                    let index = variant
                        .field_index(&field.name.name)
                        .ok_or_else(|| Diagnostic::new(field.name.span, format!("no field `{}`", field.name.name)))?;
                    ordered[index] = Some(&field.value);
                }
                let values: Option<Vec<&ast::Expr>> = ordered.into_iter().collect();
                let values = values.ok_or_else(|| Diagnostic::new(value.span, "a field of this constant has no value"))?;
                self.constant_fields(ty, None, &values, module)?
            }
            // A variant without fields is its tag.
            (Ty::Adt(adt, _), ast::ExprKind::Path(path)) if self.tcx.is_enum(*adt) => {
                let variant_name = &path.last().ident.name;
                let variant = self
                    .tcx
                    .defs
                    .adt(*adt)
                    .variant_index(variant_name)
                    .ok_or_else(|| Diagnostic::new(path.span, format!("no variant `{variant_name}`")))?;
                let (offset, bits, width) = match self.tcx.tag_encoding(ty) {
                    TagEncoding::Direct(tag) => (0, self.tcx.discriminant(*adt, variant) as u128, tag.size()),
                    TagEncoding::Niche { first, niche, value, .. } => {
                        (niche.offset, value.wrapping_add((variant - first) as u128), niche.size)
                    }
                };
                let (offset, width) = (offset as usize, width as usize);
                let mut bytes = vec![0; size];
                bytes[offset..offset + width].copy_from_slice(&bits.to_le_bytes()[..width]);
                vec![DataItem::Bytes(bytes)]
            }
            _ => bail!(value.span, "this kind of constant initialiser is not supported yet"),
        })
    }

    /// The image of a struct or tuple from the values of its fields.
    fn constant_fields(&mut self, ty: &Ty, variant: Option<u32>, values: &[&ast::Expr], module: ModId) -> Result<Vec<DataItem>> {
        let field_tys = self.tcx.field_types(ty, variant);
        let (offsets, layout) = self.tcx.fields_layout(ty, variant);
        let mut items = Vec::new();
        let mut at = 0u64;
        for ((field_ty, offset), value) in field_tys.iter().zip(&offsets).zip(values) {
            if *offset > at {
                items.push(DataItem::Bytes(vec![0; (*offset - at) as usize]));
            }
            items.extend(self.constant_data(field_ty, value, module)?);
            at = *offset + self.tcx.size_of(field_ty);
        }
        if layout.size > at {
            items.push(DataItem::Bytes(vec![0; (layout.size - at) as usize]));
        }
        Ok(items)
    }

    /// The storage of a `static` item, initialised from its constant value.
    pub fn static_data(&mut self, id: ConstId) -> Result<DataId> {
        if let Some(&data) = self.statics.get(&id) {
            return Ok(data);
        }
        let item = self.tcx.defs.const_def(id);
        let (name, span) = (&item.ast.name.name, item.ast.name.span);
        let env = GenericEnv::new();
        let scope = TypeScope { module: item.module, generics: &env, self_ty: None, self_trait: None };
        let ty = self.tcx.lower_ty(scope, &item.ast.ty, &mut InferTable::default(), &mut Vec::new())?;
        let Some(value) = &item.ast.value else { bail!(span, "static `{name}` has no value") };

        // A value that is not plain data, such as a call to a `const fn`,
        // is computed when the program starts, into zeroed memory.
        let layout = self.tcx.layout(&ty);
        let items = match self.constant_data(&ty, value, item.module) {
            Ok(items) => items,
            Err(_) => {
                let initializer = self.func_id(FuncKey::StaticInit(id));
                self.initializers.push(initializer);
                vec![DataItem::Bytes(vec![0; layout.size as usize])]
            }
        };
        let data = self.add_data(&format!("static.{name}"), items, layout.align, true);
        self.statics.insert(id, data);
        Ok(data)
    }

    /// The function that stores a static's value in its place.
    pub(super) fn build_static_init(&mut self, id: ConstId) -> Result<Function> {
        let checked = crate::sema::check::check_static_init(self.tcx, id)?;
        let body = thir::Body { ret_ty: Ty::UNIT, ..checked };
        let data = self.static_data(id)?;
        let name = format!("{}.init", self.tcx.defs.const_def(id).ast.name.name);
        let mut builder = FnBuilder::new(self, &body, name, None);
        let pointer_ty = Ty::mut_ref(body.value.ty.clone());
        let pointer = builder.temp(pointer_ty.clone());
        builder.assign(Place::local(pointer), Rvalue::Use(Operand::Const(Const::DataAddr(data, pointer_ty))));
        builder.finish_into(Place::local(pointer).deref())
    }

    /// A plain function given the calling convention of a closure — an
    /// ignored environment pointer, then the arguments — so that it can sit
    /// in the vtable of a `dyn Fn`.
    pub(super) fn build_fn_item_as_closure(&mut self, instance: &Instance) -> Result<Function> {
        let sig = self.tcx.fn_sig(instance.def, &instance.substs, &mut InferTable::default(), &mut Vec::new())?;
        let body = self.tcx.body(instance)?;
        let target = self.func_id(FuncKey::Instance(instance.clone()));

        let mut locals = vec![LocalDecl { ty: body.ret_ty.clone(), name: None }];
        locals.push(LocalDecl { ty: Ty::Ptr(Box::new(Ty::UNIT), crate::sema::ty::Mutability::Not), name: Some("env".to_string()) });
        let mut args = Vec::new();
        for ty in &sig.params {
            locals.push(LocalDecl { ty: ty.clone(), name: None });
            args.push(Operand::Copy(Place::local(Local(locals.len() as u32 - 1))));
        }
        let arg_count = locals.len() - 1;
        let block = Block {
            statements: vec![Statement::Call { dest: Place::local(RETURN_LOCAL), callee: Callee::Direct(target), args }],
            terminator: Some(Terminator::Return),
        };
        let name = format!("{}.as_closure", self.tcx.defs.fn_def(instance.def).path);
        Ok(Function { symbol: self.unique_symbol(&name), name, locals, arg_count, blocks: vec![block] })
    }

    /// A function with the closure's own parameters that calls the closure
    /// with an empty environment, so its address can serve as a `fn` pointer.
    pub(super) fn build_closure_fn_pointer(&mut self, id: ClosureId) -> Result<Function> {
        let closure = self.closure(id);
        let body = &closure.body;
        let target = self.func_id(FuncKey::Closure(id));

        let mut locals = vec![LocalDecl { ty: body.ret_ty.clone(), name: None }];
        let mut args = Vec::new();
        for param in &body.params {
            locals.push(LocalDecl { ty: body.locals[param.0 as usize].ty.clone(), name: None });
            args.push(Operand::Copy(Place::local(Local(locals.len() as u32 - 1))));
        }
        let arg_count = args.len();
        locals.push(LocalDecl { ty: Ty::Closure(id), name: Some("env".to_string()) });
        let env = Local(locals.len() as u32 - 1);
        locals.push(LocalDecl { ty: Ty::mut_ref(Ty::Closure(id)), name: None });
        let env_pointer = Local(locals.len() as u32 - 1);
        args.insert(0, Operand::Copy(Place::local(env_pointer)));

        let block = Block {
            statements: vec![
                Statement::Assign(Place::local(env_pointer), Rvalue::AddrOf(Place::local(env))),
                Statement::Call { dest: Place::local(RETURN_LOCAL), callee: Callee::Direct(target), args },
            ],
            terminator: Some(Terminator::Return),
        };
        let name = format!("closure#{}.as_fn", id.0);
        Ok(Function { symbol: self.unique_symbol(&name), name, locals, arg_count, blocks: vec![block] })
    }
}

/// The value of a float constant written as a literal, possibly negated.
fn constant_float(value: &ast::Expr) -> Option<f64> {
    match &value.kind {
        ast::ExprKind::Float(number, _) => Some(*number),
        ast::ExprKind::Int(number, _) => Some(*number as f64),
        ast::ExprKind::Unary(ast::UnOp::Neg, inner) => constant_float(inner).map(|number| -number),
        _ => None,
    }
}
