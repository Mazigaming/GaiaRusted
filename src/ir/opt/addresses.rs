//! Computing the address of an array or slice element once per use site.
//!
//! `items[i].a + items[i].b` addresses `items[i]` twice, and for an
//! element whose size is not 1, 2, 4 or 8 bytes each address costs a
//! multiplication (the processor's addressing modes scale an index by
//! those sizes only). When a block uses the same element several times,
//! its address is taken once into a pointer, and the element is reached
//! through that.

use crate::ir::analysis::Effect;
use crate::ir::*;
use crate::sema::context::Context;
use crate::sema::ty::{Mutability, Ty};
use std::collections::HashMap;

/// The part of a place that selects an element: `(*base)[index]` or
/// `base[index]`.
fn element_prefix(place: &Place) -> Option<usize> {
    match place.projection.as_slice() {
        [Projection::Deref, Projection::Index(_), ..] => Some(2),
        [Projection::Index(_), ..] => Some(1),
        _ => None,
    }
}

/// The locals an element's address depends on: the base and the index.
fn inputs(place: &Place, prefix: usize) -> [Local; 2] {
    let Projection::Index(index) = place.projection[prefix - 1] else { unreachable!("the prefix ends in an index") };
    [place.local, index]
}

/// Returns whether anything changed.
pub fn run(func: &mut Function, tcx: &Context) -> bool {
    let mut changed = false;
    for index in 0..func.blocks.len() {
        changed |= in_block(func, tcx, index);
    }
    changed
}

fn in_block(func: &mut Function, tcx: &Context, block: usize) -> bool {
    // Group the uses of each element, a group ending where its base or
    // index changes.
    #[derive(Default)]
    struct Group {
        statements: Vec<usize>,
    }
    let mut groups: Vec<(Place, Group)> = Vec::new();
    // The group still collecting uses of each element.
    let mut open: HashMap<Place, usize> = HashMap::new();

    for (position, statement) in func.blocks[block].statements.iter().enumerate() {
        statement.each_place(&mut |place, _| {
            let Some(prefix) = element_prefix(place) else { return };
            let element = Place { local: place.local, projection: place.projection[..prefix].to_vec() };
            let size = tcx.size_of(&func.place_ty(tcx, &element));
            if matches!(size, 0 | 1 | 2 | 4 | 8) {
                return;
            }
            let group = *open.entry(element.clone()).or_insert_with(|| {
                groups.push((element, Group::default()));
                groups.len() - 1
            });
            if groups[group].1.statements.last() != Some(&position) {
                groups[group].1.statements.push(position);
            }
        });
        let written = Effect::of_statement(statement).writes;
        open.retain(|_, group| {
            let element = &groups[*group].0;
            let prefix = element.projection.len();
            !inputs(element, prefix).iter().any(|input| written.contains(input))
        });
    }

    let mut insertions: Vec<(usize, Statement)> = Vec::new();
    let mut renames: Vec<(usize, Place, Local)> = Vec::new();
    for (element, group) in groups {
        if group.statements.len() < 2 {
            continue;
        }
        let ty = func.place_ty(tcx, &element);
        func.locals.push(LocalDecl { ty: Ty::Ptr(Box::new(ty), Mutability::Mut), name: None });
        let pointer = Local(func.locals.len() as u32 - 1);
        insertions.push((group.statements[0], Statement::Assign(Place::local(pointer), Rvalue::AddrOf(element.clone()))));
        for position in group.statements {
            renames.push((position, element.clone(), pointer));
        }
    }
    if insertions.is_empty() {
        return false;
    }

    let statements = &mut func.blocks[block].statements;
    for (position, element, pointer) in renames {
        statements[position].each_place_mut(&mut |place, _| {
            if place.local == element.local && place.projection.starts_with(&element.projection) {
                place.projection.splice(..element.projection.len(), [Projection::Deref]);
                place.local = pointer;
            }
        });
    }
    // Insert from the back so earlier positions stay valid.
    insertions.sort_by_key(|(position, _)| std::cmp::Reverse(*position));
    for (position, statement) in insertions {
        statements.insert(position, statement);
    }
    true
}
