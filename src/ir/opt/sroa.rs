//! Scalar replacement of aggregates: one local per field.
//!
//! A struct, tuple or closure environment that nobody takes the address of
//! as a whole is just a group of values that happen to share a name. Split
//! into one local per field, each field is a value of its own: it can be
//! propagated, found dead, and kept in a register. An enum splits the same
//! way, into a local for its tag and one for each field of each variant;
//! setting and reading the tag become plain assignments. This is what turns
//! an iterator adapter, or an `Option` made by one function and taken apart
//! by its caller, into a handful of registers once inlining has put maker
//! and user side by side.
//!
//! A value that has to exist as a whole is put together where needed: a
//! copy of the aggregate becomes a copy of each field, an argument is
//! assembled in a temporary, and a result is taken apart after the call.

use crate::ir::*;
use crate::sema::context::Context;
use crate::sema::ty::Ty;

/// What a split aggregate has become.
struct Split {
    /// For an enum, the local holding the tag.
    tag: Option<Local>,
    /// The projections that select each part, and the part's own local.
    parts: Vec<(Vec<Projection>, Local)>,
}

/// Returns whether anything was split.
pub fn run(func: &mut Function, tcx: &Context) -> bool {
    /// Each round splits one level of nesting.
    const MAX_DEPTH: usize = 4;
    let mut changed = false;
    for _ in 0..MAX_DEPTH {
        if !split_once(func, tcx) {
            break;
        }
        changed = true;
    }
    changed
}

/// The types worth splitting, and those that can be.
fn is_aggregate(tcx: &Context, ty: &Ty) -> bool {
    matches!(ty, Ty::Tuple(_) | Ty::Closure(_) | Ty::Adt(..)) && !tcx.is_zero_sized(ty)
}

/// The projections that select each part of a value of type `ty`, with
/// the part's type.
fn parts_of(tcx: &Context, ty: &Ty) -> Vec<(Vec<Projection>, Ty)> {
    match ty {
        Ty::Adt(adt, _) if tcx.is_enum(*adt) => {
            let variants = tcx.defs.adt(*adt).variants.len() as u32;
            (0..variants)
                .flat_map(|variant| {
                    let fields = tcx.field_types(ty, Some(variant)).into_iter().enumerate();
                    fields.map(move |(index, field)| (vec![Projection::Downcast(variant), Projection::Field(index)], field))
                })
                .collect()
        }
        _ => tcx.field_types(ty, None).into_iter().enumerate().map(|(index, field)| (vec![Projection::Field(index)], field)).collect(),
    }
}

/// Decide which locals to split.
fn splittable(func: &Function, tcx: &Context) -> Vec<bool> {
    let count = func.locals.len();
    let is_enum = |local: Local| tcx.is_enum_ty(&func.locals[local.0 as usize].ty);
    // Parameters and the result are laid out by the calling convention.
    let mut possible: Vec<bool> = (0..count)
        .map(|index| index > func.arg_count && is_aggregate(tcx, &func.locals[index].ty))
        .collect();
    // Splitting pays when fields are computed with. A local that a call
    // fills and whose fields are only copied out would just be replaced
    // by a temporary that is used the same way.
    let mut computed_with = vec![false; count];
    let mut filled_by_call = vec![false; count];
    // A local nothing mentions has nothing to gain. Among those are the
    // aggregates split before, whose uses all went to their parts.
    let mut mentioned = vec![false; count];

    let reject = |local: Local, possible: &mut Vec<bool>| possible[local.0 as usize] = false;
    // A place inside an aggregate must select a whole part: a field, or
    // a field of a variant.
    let selects_part = |place: &Place| match place.projection.as_slice() {
        [] => true,
        [Projection::Downcast(_), Projection::Field(_), ..] => is_enum(place.local),
        [Projection::Field(_), ..] => !is_enum(place.local) && !matches!(func.locals[place.local.0 as usize].ty, Ty::Array(..)),
        _ => false,
    };

    for block in &func.blocks {
        for statement in &block.statements {
            statement.each_place(&mut |place, access| {
                mentioned[place.local.0 as usize] = true;
                if !selects_part(place) {
                    reject(place.local, &mut possible);
                }
                if access == visit::Access::Address && place.projection.is_empty() {
                    reject(place.local, &mut possible);
                }
            });
            match statement {
                Statement::Assign(dest, rvalue) => {
                    if let Some(local) = dest.as_local() {
                        if !matches!(rvalue, Rvalue::Use(Operand::Copy(_))) {
                            reject(local, &mut possible);
                        }
                    }
                    match rvalue {
                        // An enum is never read whole: putting one back
                        // together would mean writing a tag that is only
                        // known at run time.
                        Rvalue::Use(Operand::Copy(source)) => {
                            if let Some(local) = source.as_local().filter(|&local| is_enum(local)) {
                                reject(local, &mut possible);
                            }
                            let is_plain_copy = dest.as_local().is_some();
                            if !source.projection.is_empty() && !is_plain_copy {
                                computed_with[source.local.0 as usize] = true;
                            }
                        }
                        Rvalue::Discriminant(place) => computed_with[place.local.0 as usize] = true,
                        Rvalue::AddrOf(_) => {}
                        other => other.each_operand(&mut |operand| {
                            if let Operand::Copy(place) = operand {
                                if place.projection.is_empty() {
                                    reject(place.local, &mut possible);
                                } else {
                                    computed_with[place.local.0 as usize] = true;
                                }
                            }
                        }),
                    }
                }
                Statement::SetDiscriminant(..) => {}
                Statement::Call { dest, callee, args } => {
                    if let Some(local) = dest.as_local() {
                        filled_by_call[local.0 as usize] = true;
                    }
                    let pointer = match callee {
                        Callee::Indirect(pointer) => Some(pointer),
                        _ => None,
                    };
                    for operand in args.iter().chain(pointer) {
                        if let Operand::Copy(place) = operand {
                            match place.as_local() {
                                Some(local) if is_enum(local) => reject(local, &mut possible),
                                Some(_) => {}
                                None => computed_with[place.local.0 as usize] = true,
                            }
                        }
                    }
                }
            }
        }
        if let Some(terminator) = &block.terminator {
            terminator.each_place(&mut |place, _| {
                mentioned[place.local.0 as usize] = true;
                if place.projection.is_empty() {
                    reject(place.local, &mut possible);
                } else {
                    computed_with[place.local.0 as usize] = true;
                }
            });
        }
    }
    (0..count)
        .map(|local| possible[local] && mentioned[local] && (computed_with[local] || !filled_by_call[local]))
        .collect()
}

fn split_once(func: &mut Function, tcx: &Context) -> bool {
    let split = splittable(func, tcx);
    if !split.contains(&true) {
        return false;
    }
    let mut splits: Vec<Option<Split>> = Vec::with_capacity(func.locals.len());
    for local in 0..func.locals.len() {
        if !split[local] {
            splits.push(None);
            continue;
        }
        let decl = func.locals[local].clone();
        let part_name = |suffix: String| decl.name.as_ref().map(|name| format!("{name}.{suffix}"));
        let tag = tcx.is_enum_ty(&decl.ty).then(|| {
            func.locals.push(LocalDecl { ty: Ty::Int(tcx.tag_type(&decl.ty)), name: part_name("tag".into()) });
            Local(func.locals.len() as u32 - 1)
        });
        let mut parts = Vec::new();
        for (projection, ty) in parts_of(tcx, &decl.ty) {
            let suffix = projection.iter().map(|p| match p {
                Projection::Field(index) => index.to_string(),
                Projection::Downcast(variant) => format!("v{variant}"),
                _ => unreachable!("parts are selected by fields and variants"),
            });
            func.locals.push(LocalDecl { ty, name: part_name(suffix.collect::<Vec<_>>().join(".")) });
            parts.push((projection, Local(func.locals.len() as u32 - 1)));
        }
        splits.push(Some(Split { tag, parts }));
    }

    expand_whole_uses(func, tcx, &splits);
    rename_parts(func, &splits);
    true
}

/// Rewrite the statements that use a split aggregate as a whole into
/// statements on its parts (still named through the aggregate; the parts
/// get their own names afterwards).
fn expand_whole_uses(func: &mut Function, tcx: &Context, splits: &[Option<Split>]) {
    // Locals added along the way (parts, temporaries) are never split.
    let split_of = |place: &Place| place.as_local().and_then(|local| splits.get(local.0 as usize)?.as_ref());
    for index in 0..func.blocks.len() {
        let statements = std::mem::take(&mut func.blocks[index].statements);
        let mut rewritten = Vec::with_capacity(statements.len());
        for statement in statements {
            match statement {
                Statement::Assign(dest, Rvalue::Use(Operand::Copy(source)))
                    if split_of(&dest).is_some() || split_of(&source).is_some() =>
                {
                    let ty = func.place_ty(tcx, &dest);
                    copy_parts(tcx, &ty, &dest, &source, split_of(&dest), &mut rewritten);
                }
                Statement::SetDiscriminant(place, variant) if split_of(&place).is_some() => {
                    let tag = split_of(&place).and_then(|split| split.tag).expect("only enums have a discriminant");
                    let ty = func.place_ty(tcx, &place);
                    let Ty::Adt(adt, _) = &ty else { unreachable!("only enums have a discriminant") };
                    let value = tcx.discriminant(*adt, variant) as u128;
                    let constant = Const::Int(value, func.locals[tag.0 as usize].ty.clone());
                    rewritten.push(Statement::Assign(Place::local(tag), Rvalue::Use(Operand::Const(constant))));
                }
                Statement::Assign(dest, Rvalue::Discriminant(place)) if split_of(&place).is_some() => {
                    let tag = split_of(&place).and_then(|split| split.tag).expect("only enums have a discriminant");
                    rewritten.push(Statement::Assign(dest, Rvalue::Use(Operand::Copy(Place::local(tag)))));
                }
                Statement::Call { dest, callee, mut args } => {
                    // Arguments are assembled in temporaries of their own.
                    for arg in &mut args {
                        let Operand::Copy(source) = arg else { continue };
                        if split_of(source).is_none() {
                            continue;
                        }
                        let ty = func.place_ty(tcx, source);
                        let temp = Place::local(add_local(func, ty.clone()));
                        copy_parts(tcx, &ty, &temp, source, None, &mut rewritten);
                        *arg = Operand::Copy(temp);
                    }
                    // A result is received whole and then taken apart.
                    match split_of(&dest) {
                        Some(split) => {
                            let ty = func.place_ty(tcx, &dest);
                            let temp = Place::local(add_local(func, ty.clone()));
                            rewritten.push(Statement::Call { dest: temp.clone(), callee, args });
                            copy_parts(tcx, &ty, &dest, &temp, Some(split), &mut rewritten);
                        }
                        None => rewritten.push(Statement::Call { dest, callee, args }),
                    }
                }
                other => rewritten.push(other),
            }
        }
        func.blocks[index].statements = rewritten;
    }
}

fn add_local(func: &mut Function, ty: Ty) -> Local {
    func.locals.push(LocalDecl { ty, name: None });
    Local(func.locals.len() as u32 - 1)
}

/// `dest = source` for an aggregate of type `ty`, one part at a time.
/// `dest_split` is what `dest` was split into, if it was.
fn copy_parts(tcx: &Context, ty: &Ty, dest: &Place, source: &Place, dest_split: Option<&Split>, out: &mut Vec<Statement>) {
    if let Some(tag) = dest_split.and_then(|split| split.tag) {
        out.push(Statement::Assign(Place::local(tag), Rvalue::Discriminant(source.clone())));
    }
    for (projection, _) in parts_of(tcx, ty) {
        let select = |place: &Place| {
            let mut place = place.clone();
            place.projection.extend(projection.iter().cloned());
            place
        };
        out.push(Statement::Assign(select(dest), Rvalue::Use(Operand::Copy(select(source)))));
    }
}

/// Make every place inside a split aggregate refer to the part's local.
fn rename_parts(func: &mut Function, splits: &[Option<Split>]) {
    let rename = |place: &mut Place| {
        let Some(Some(split)) = splits.get(place.local.0 as usize) else { return };
        let (selector, part) = split
            .parts
            .iter()
            .find(|(selector, _)| place.projection.starts_with(selector))
            .expect("every place in a split aggregate selects a part");
        place.projection.drain(..selector.len());
        place.local = *part;
    };
    for block in &mut func.blocks {
        for statement in &mut block.statements {
            statement.each_place_mut(&mut |place, _| rename(place));
        }
        if let Some(terminator) = &mut block.terminator {
            terminator.each_place_mut(&mut |place, _| rename(place));
        }
    }
}
