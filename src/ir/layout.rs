//! How values are arranged in memory.
//!
//! One rule for everything, in every context — stack, heap, behind a
//! pointer: fields are placed in declaration order at ascending offsets,
//! each aligned to its own alignment, and the whole is padded to a multiple
//! of the largest alignment. An enum is a tag followed by the fields of
//! whichever variant it holds — unless all its variants but one hold no
//! data and the one that does has a niche: a scalar that never takes some
//! of the values its bytes could hold, such as a reference, which is never
//! null. Then the enum is just that variant's fields, and the other
//! variants are stored as values the scalar never takes. This is how
//! `Option<&T>` and `Option<Box<T>>` are a single pointer, as under rustc.

use super::{Const, Function, Operand, Place, Projection};
use crate::sema::context::Context;
use crate::sema::ty::{IntTy, Ty};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub size: u64,
    pub align: u64,
}

impl Layout {
    const ZERO: Layout = Layout { size: 0, align: 1 };
    const WORD: Layout = Layout { size: 8, align: 8 };

    fn scalar(size: u64) -> Layout {
        Layout { size, align: size }
    }
}

/// A scalar inside a type that only ever holds the values
/// `start..=end`, a range that wraps around past the largest value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Niche {
    pub offset: u64,
    /// The scalar's size in bytes.
    pub size: u64,
    pub start: u128,
    pub end: u128,
}

impl Niche {
    const POINTER: Niche = Niche { offset: 0, size: 8, start: 1, end: u64::MAX as u128 };

    fn mask(&self) -> u128 {
        u128::MAX >> (128 - 8 * self.size)
    }

    /// How many values the scalar never holds.
    fn unused(&self) -> u128 {
        self.mask() - (self.end.wrapping_sub(self.start) & self.mask())
    }

    /// The first of `count` unused values set aside for an enum's variants,
    /// if there are that many.
    fn reserve(&self, count: u32) -> Option<u128> {
        (count as u128 <= self.unused()).then(|| self.end.wrapping_add(1) & self.mask())
    }

    fn at(self, offset: u64) -> Niche {
        Niche { offset: self.offset + offset, ..self }
    }
}

/// How an enum records which of its variants it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TagEncoding {
    /// A tag of its own, ahead of the fields, holding the discriminant.
    Direct(IntTy),
    /// The variants `first..first + count` other than `dataful` have no
    /// data; variant `first + i` is stored as `value + i` in the niche of
    /// the dataful variant's fields. A value of the niche's own range
    /// means `dataful`. Every discriminant is the variant's index.
    Niche { dataful: u32, first: u32, count: u32, niche: Niche, value: u128 },
}

/// The layout of a struct whose last field is unsized.
pub struct UnsizedTail {
    /// The offsets of the sized fields before the last.
    pub prefix_offsets: Vec<u64>,
    /// Where the sized fields end.
    pub prefix_end: u64,
    pub prefix_align: u64,
    /// The alignment the last field has whatever value it holds.
    pub static_align: u64,
    /// Whether a trait object in the last field may need more alignment:
    /// the last field then starts at `prefix_end` rounded up to the larger
    /// of `static_align` and the alignment in the object's vtable.
    pub dynamic: bool,
}

fn align_up(offset: u64, align: u64) -> u64 {
    offset.div_ceil(align) * align
}

/// Offsets of consecutive fields, and the layout of the struct they form.
fn sequence(fields: impl Iterator<Item = Layout>, start: u64) -> (Vec<u64>, Layout) {
    let mut offsets = Vec::new();
    let mut offset = start;
    let mut align = 1;
    for field in fields {
        offset = align_up(offset, field.align);
        offsets.push(offset);
        offset += field.size;
        align = align.max(field.align);
    }
    (offsets, Layout { size: align_up(offset, align), align })
}

impl Context<'_> {
    /// Size and alignment of a (concrete, sized) type.
    pub fn layout(&self, ty: &Ty) -> Layout {
        if let Some(&layout) = self.layouts.borrow().get(ty) {
            return layout;
        }
        let layout = self.compute_layout(ty);
        self.layouts.borrow_mut().insert(ty.clone(), layout);
        layout
    }

    fn compute_layout(&self, ty: &Ty) -> Layout {
        match ty {
            Ty::Int(int) => Layout::scalar(int.size()),
            Ty::Float(float) => Layout::scalar(float.size()),
            Ty::Bool => Layout::scalar(1),
            Ty::Char => Layout::scalar(4),
            Ty::Never | Ty::FnItem(..) => Layout::ZERO,
            Ty::FnPtr(..) => Layout::WORD,
            Ty::Ref(pointee, _) | Ty::Ptr(pointee, _) => {
                if self.is_unsized(pointee) {
                    Layout { size: 16, align: 8 }
                } else {
                    Layout::WORD
                }
            }
            Ty::Array(element, _) => {
                let len = ty.array_len().expect("array lengths are known after type checking");
                let element = self.layout(element);
                Layout { size: element.size * len, align: element.align }
            }
            Ty::Tuple(_) | Ty::Closure(_) => self.fields_layout(ty, None).1,
            Ty::Adt(adt, _) if self.is_enum(*adt) => self.enum_layout(ty),
            Ty::Adt(..) if self.is_unsized(ty) => {
                unreachable!("unsized type `{}` has no layout of its own", self.display(ty))
            }
            Ty::Adt(..) => self.fields_layout(ty, None).1,
            Ty::Str | Ty::Slice(_) | Ty::Dyn(..) => {
                unreachable!("unsized type `{}` has no layout of its own", self.display(ty))
            }
            Ty::Const(_) => unreachable!("a constant in a type has no layout"),
            Ty::Infer(_) => unreachable!("inference variable survived type checking"),
        }
    }

    pub fn size_of(&self, ty: &Ty) -> u64 {
        self.layout(ty).size
    }

    /// Can a value of this type travel in registers (one, or two for a
    /// fat pointer) instead of through memory?
    pub fn is_register_value(&self, ty: &Ty) -> bool {
        ty.is_scalar() || matches!(ty, Ty::Ref(..) | Ty::Ptr(..))
    }

    pub fn is_zero_sized(&self, ty: &Ty) -> bool {
        !self.is_unsized(ty) && self.size_of(ty) == 0
    }

    /// The types of the fields of a struct, tuple, closure environment, or
    /// (with `variant`) of one variant of an enum.
    pub fn field_types(&self, ty: &Ty, variant: Option<u32>) -> Vec<Ty> {
        match ty {
            Ty::Tuple(elements) => elements.clone(),
            Ty::Adt(adt, args) => self.concrete_variant_fields(*adt, args, variant.unwrap_or(0)),
            Ty::Closure(id) => {
                let closure = self.closure(*id).expect("closure was checked before being laid out");
                let captured = closure.captures.iter().map(|capture| {
                    if capture.by_ref {
                        Ty::mut_ref(capture.ty.clone())
                    } else {
                        capture.ty.clone()
                    }
                });
                let flags = self.closure_drop_flags(*id).into_iter().map(|_| Ty::Bool);
                captured.chain(flags).collect()
            }
            _ => unreachable!("`{}` has no fields", self.display(ty)),
        }
    }

    /// Field offsets and overall layout of a struct-like type or enum variant.
    ///
    /// For a struct whose last field is unsized, the layout is that of its
    /// sized fields, and the last field's offset is where it starts at the
    /// least; a trait object there may need more alignment, which only its
    /// vtable tells (see [`unsized_tail`](Self::unsized_tail)).
    pub fn fields_layout(&self, ty: &Ty, variant: Option<u32>) -> (Vec<u64>, Layout) {
        if variant.is_none() && matches!(ty, Ty::Adt(..)) && self.is_unsized(ty) {
            let tail = self.unsized_tail(ty);
            let mut offsets = tail.prefix_offsets;
            offsets.push(align_up(tail.prefix_end, tail.static_align));
            return (offsets, Layout { size: tail.prefix_end, align: tail.prefix_align });
        }
        let fields = self.field_types(ty, variant);
        let layouts = fields.iter().map(|field| self.layout(field));
        match variant {
            // A variant's fields come after the enum's tag.
            Some(_) if self.is_enum_ty(ty) => match self.tag_encoding(ty) {
                TagEncoding::Direct(tag) => sequence(layouts, tag.size()),
                TagEncoding::Niche { .. } => sequence(layouts, 0),
            },
            _ => sequence(layouts, 0),
        }
    }

    /// How a struct whose last field is unsized is laid out.
    pub fn unsized_tail(&self, ty: &Ty) -> UnsizedTail {
        let fields = self.field_types(ty, None);
        let (tail, prefix) = fields.split_last().expect("an unsized struct has a last field");
        let layouts: Vec<Layout> = prefix.iter().map(|field| self.layout(field)).collect();
        let (prefix_offsets, prefix_layout) = sequence(layouts.iter().copied(), 0);
        let prefix_end = prefix_offsets.last().map_or(0, |&offset| offset + layouts.last().map_or(0, |layout| layout.size));
        let (static_align, dynamic) = self.unsized_align(tail);
        UnsizedTail { prefix_offsets, prefix_end, prefix_align: prefix_layout.align, static_align, dynamic }
    }

    /// The alignment of an unsized type as far as the type tells it, and
    /// whether a trait object in it may need more: that is in its vtable.
    pub fn unsized_align(&self, ty: &Ty) -> (u64, bool) {
        match ty {
            Ty::Str => (1, false),
            Ty::Slice(element) => (self.layout(element).align, false),
            Ty::Dyn(..) => (1, true),
            _ => {
                let tail = self.unsized_tail(ty);
                (tail.prefix_align.max(tail.static_align), tail.dynamic)
            }
        }
    }

    pub fn field_offset(&self, ty: &Ty, variant: Option<u32>, index: usize) -> u64 {
        self.fields_layout(ty, variant).0[index]
    }

    pub fn is_enum_ty(&self, ty: &Ty) -> bool {
        matches!(ty, Ty::Adt(adt, _) if self.is_enum(*adt))
    }

    fn enum_layout(&self, ty: &Ty) -> Layout {
        let Ty::Adt(adt, _) = ty else { unreachable!() };
        let variants = self.defs.adt(*adt).variants.len() as u32;
        if variants == 0 {
            return Layout::ZERO;
        }
        let mut layout = match self.tag_encoding(ty) {
            TagEncoding::Direct(tag) => Layout::scalar(tag.size()),
            TagEncoding::Niche { .. } => Layout::ZERO,
        };
        for variant in 0..variants {
            let (_, of_variant) = self.fields_layout(ty, Some(variant));
            layout.size = layout.size.max(of_variant.size);
            layout.align = layout.align.max(of_variant.align);
        }
        layout.size = align_up(layout.size, layout.align);
        layout
    }

    /// The integer type of an enum's tag: the smallest that holds every
    /// discriminant.
    pub fn tag_type(&self, ty: &Ty) -> IntTy {
        let Ty::Adt(adt, _) = ty else { unreachable!("only enums have a tag") };
        let (min, max) = (0..self.defs.adt(*adt).variants.len() as u32)
            .map(|variant| self.discriminant(*adt, variant))
            .fold((0i128, 0i128), |(min, max), d| (min.min(d), max.max(d)));
        let candidates: &[IntTy] = if min < 0 {
            &[IntTy::I8, IntTy::I16, IntTy::I32, IntTy::I64]
        } else {
            &[IntTy::U8, IntTy::U16, IntTy::U32, IntTy::U64]
        };
        *candidates
            .iter()
            .find(|int| int.range().0 <= min && max <= int.range().1)
            .unwrap_or(&IntTy::I64)
    }

    /// How an enum records which variant it holds.
    pub fn tag_encoding(&self, ty: &Ty) -> TagEncoding {
        if let Some(&encoding) = self.tag_encodings.borrow().get(ty) {
            return encoding;
        }
        let encoding = self.choose_tag_encoding(ty);
        self.tag_encodings.borrow_mut().insert(ty.clone(), encoding);
        encoding
    }

    fn choose_tag_encoding(&self, ty: &Ty) -> TagEncoding {
        let Ty::Adt(adt, _) = ty else { unreachable!("only enums have a tag") };
        let direct = TagEncoding::Direct(self.tag_type(ty));
        let def = self.defs.adt(*adt);
        let variants = def.variants.len() as u32;
        if variants < 2 || def.variants.iter().any(|variant| variant.discriminant.is_some()) {
            return direct;
        }
        let mut with_data = (0..variants).filter(|&variant| {
            !self.field_types(ty, Some(variant)).iter().all(|field| self.is_zero_sized(field))
        });
        let (Some(dataful), None) = (with_data.next(), with_data.next()) else { return direct };

        let first = if dataful == 0 { 1 } else { 0 };
        let last = if dataful == variants - 1 { variants - 2 } else { variants - 1 };
        let count = last - first + 1;
        let fields = self.field_types(ty, Some(dataful));
        let (offsets, _) = sequence(fields.iter().map(|field| self.layout(field)), 0);
        match self.largest_niche(fields.iter().zip(offsets)) {
            Some(niche) => match niche.reserve(count) {
                Some(value) => TagEncoding::Niche { dataful, first, count, niche, value },
                None => direct,
            },
            None => direct,
        }
    }

    /// The scalar in a value of this type that leaves the most values unused.
    pub fn niche(&self, ty: &Ty) -> Option<Niche> {
        match ty {
            Ty::Bool => Some(Niche { offset: 0, size: 1, start: 0, end: 1 }),
            Ty::Char => Some(Niche { offset: 0, size: 4, start: 0, end: char::MAX as u128 }),
            Ty::Ref(..) | Ty::FnPtr(..) => Some(Niche::POINTER),
            Ty::Array(element, _) if ty.array_len() != Some(0) => self.niche(element),
            Ty::Adt(adt, _) if self.is_enum(*adt) => self.enum_niche(ty),
            Ty::Adt(adt, _) if self.defs.adt(*adt).never_zero => match self.field_types(ty, None).first()? {
                Ty::Int(int) => {
                    let niche = Niche { offset: 0, size: int.size(), start: 1, end: 0 };
                    Some(Niche { end: niche.mask(), ..niche })
                }
                Ty::Ptr(..) | Ty::Ref(..) => Some(Niche::POINTER),
                _ => None,
            },
            Ty::Tuple(_) | Ty::Closure(_) | Ty::Adt(..) => {
                let fields = self.field_types(ty, None);
                let (offsets, _) = self.fields_layout(ty, None);
                self.largest_niche(fields.iter().zip(offsets))
            }
            _ => None,
        }
    }

    fn largest_niche<'t>(&self, fields: impl Iterator<Item = (&'t Ty, u64)>) -> Option<Niche> {
        fields
            .filter_map(|(field, offset)| Some(self.niche(field)?.at(offset)))
            .filter(|niche| niche.unused() > 0)
            .max_by_key(|niche| (niche.unused(), std::cmp::Reverse(niche.offset)))
    }

    /// An enum's own niche: the values its tag never holds.
    fn enum_niche(&self, ty: &Ty) -> Option<Niche> {
        let Ty::Adt(adt, _) = ty else { unreachable!() };
        let variants = self.defs.adt(*adt).variants.len() as u32;
        let niche = match self.tag_encoding(ty) {
            TagEncoding::Direct(_) if variants == 0 => return None,
            TagEncoding::Direct(tag) => {
                let discriminants = (0..variants).map(|variant| self.discriminant(*adt, variant));
                let (min, max) = (discriminants.clone().min()?, discriminants.max()?);
                let niche = Niche { offset: 0, size: tag.size(), start: 0, end: 0 };
                Niche { start: min as u128 & niche.mask(), end: max as u128 & niche.mask(), ..niche }
            }
            TagEncoding::Niche { count, niche, value, .. } => {
                Niche { end: value.wrapping_add(count as u128 - 1) & niche.mask(), ..niche }
            }
        };
        (niche.unused() > 0).then_some(niche)
    }

    /// The tag value of a variant: its explicit `= value`, or one more than
    /// the previous variant's, starting from zero.
    pub fn discriminant(&self, adt: crate::sema::ty::AdtId, variant: u32) -> i128 {
        let def = self.defs.adt(adt);
        let mut value = -1;
        for candidate in &def.variants[..=variant as usize] {
            value = match candidate.discriminant {
                Some(expr) => crate::sema::const_eval::eval_int(self, def.module, expr)
                    .expect("discriminants were validated when the enum was first used"),
                None => value + 1,
            };
        }
        value
    }
}

impl Function {
    /// The type of the value a place refers to.
    pub fn place_ty(&self, tcx: &Context, place: &Place) -> Ty {
        let mut ty = self.locals[place.local.0 as usize].ty.clone();
        let mut variant = None;
        for projection in &place.projection {
            ty = match projection {
                Projection::Downcast(index) => {
                    variant = Some(*index);
                    continue;
                }
                Projection::Field(index) => match &ty {
                    Ty::Array(element, _) => (**element).clone(),
                    _ => tcx.field_types(&ty, variant).swap_remove(*index),
                },
                Projection::Deref => ty.pointee().expect("deref of a non-pointer").clone(),
                Projection::Index(_) => match &ty {
                    Ty::Array(element, _) | Ty::Slice(element) => (**element).clone(),
                    other => unreachable!("cannot index `{}`", tcx.display(other)),
                },
            };
            variant = None;
        }
        ty
    }

    pub fn operand_ty(&self, tcx: &Context, operand: &Operand) -> Ty {
        match operand {
            Operand::Copy(place) => self.place_ty(tcx, place),
            Operand::Const(Const::Int(_, ty) | Const::Float(_, ty) | Const::ZeroSized(ty)) => ty.clone(),
            Operand::Const(Const::DataAddr(_, ty) | Const::FuncAddr(_, ty)) => ty.clone(),
            Operand::Const(Const::Str(..)) => Ty::str_ref(),
        }
    }
}
