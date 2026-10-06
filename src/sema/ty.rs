//! The semantic representation of types.
//!
//! Unlike [`crate::syntax::ast::Type`], which records what the programmer
//! wrote, a [`Ty`] says what the type *is*: names are resolved, aliases are
//! expanded and generic parameters are replaced by concrete types or by
//! inference variables.

/// Identifies a struct or enum definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AdtId(pub u32);

/// Identifies a function, method or extern declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FnId(pub u32);

/// Identifies a trait definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TraitId(pub u32);

/// Identifies one closure expression inside one function instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClosureId(pub u32);

/// An inference variable: a type not known yet, to be solved by unification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InferVar(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IntTy {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
}

impl IntTy {
    pub const ALL: [IntTy; 12] = [
        IntTy::I8,
        IntTy::I16,
        IntTy::I32,
        IntTy::I64,
        IntTy::I128,
        IntTy::Isize,
        IntTy::U8,
        IntTy::U16,
        IntTy::U32,
        IntTy::U64,
        IntTy::U128,
        IntTy::Usize,
    ];

    pub fn name(self) -> &'static str {
        match self {
            IntTy::I8 => "i8",
            IntTy::I16 => "i16",
            IntTy::I32 => "i32",
            IntTy::I64 => "i64",
            IntTy::I128 => "i128",
            IntTy::Isize => "isize",
            IntTy::U8 => "u8",
            IntTy::U16 => "u16",
            IntTy::U32 => "u32",
            IntTy::U64 => "u64",
            IntTy::U128 => "u128",
            IntTy::Usize => "usize",
        }
    }

    pub fn from_name(name: &str) -> Option<IntTy> {
        IntTy::ALL.into_iter().find(|ty| ty.name() == name)
    }

    /// Size in bytes on the (only) supported target, x86-64.
    pub fn size(self) -> u64 {
        match self {
            IntTy::I8 | IntTy::U8 => 1,
            IntTy::I16 | IntTy::U16 => 2,
            IntTy::I32 | IntTy::U32 => 4,
            IntTy::I64 | IntTy::U64 | IntTy::Isize | IntTy::Usize => 8,
            IntTy::I128 | IntTy::U128 => 16,
        }
    }

    pub fn is_signed(self) -> bool {
        matches!(self, IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::I64 | IntTy::I128 | IntTy::Isize)
    }

    /// Smallest and largest representable values; `u128`'s largest is
    /// given as `i128::MAX`, all an `i128` can say.
    pub fn range(self) -> (i128, i128) {
        let bits = self.size() * 8;
        if bits == 128 {
            return if self.is_signed() { (i128::MIN, i128::MAX) } else { (0, i128::MAX) };
        }
        if self.is_signed() {
            (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
        } else {
            (0, (1i128 << bits) - 1)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FloatTy {
    F32,
    F64,
}

impl FloatTy {
    pub fn name(self) -> &'static str {
        match self {
            FloatTy::F32 => "f32",
            FloatTy::F64 => "f64",
        }
    }

    pub fn size(self) -> u64 {
        match self {
            FloatTy::F32 => 4,
            FloatTy::F64 => 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    Int(IntTy),
    Float(FloatTy),
    Bool,
    Char,
    /// The unsized string slice; only ever seen behind a pointer (`&str`).
    Str,
    /// `!` — the type of expressions that never produce a value.
    Never,
    /// `(A, B)`; the unit type `()` is the empty tuple.
    Tuple(Vec<Ty>),
    /// `[T; N]`: the length is a [`Ty::Const`], or a variable until known.
    Array(Box<Ty>, Box<Ty>),
    /// The unsized slice `[T]`; only ever seen behind a pointer.
    Slice(Box<Ty>),
    /// `&T` / `&mut T`
    Ref(Box<Ty>, Mutability),
    /// `*const T` / `*mut T`
    Ptr(Box<Ty>, Mutability),
    /// A struct or enum with its type arguments.
    Adt(AdtId, Vec<Ty>),
    /// `fn(A, B) -> R` — a function pointer.
    FnPtr(Vec<Ty>, Box<Ty>),
    /// The zero-sized type of one specific function. Calling through it is
    /// a direct call; it coerces to [`Ty::FnPtr`] when stored as a pointer.
    FnItem(FnId, Vec<Ty>),
    /// The anonymous type of a closure: a struct holding its captures.
    Closure(ClosureId),
    /// `dyn Trait` with the trait's type arguments; only seen behind a pointer.
    Dyn(TraitId, Vec<Ty>),
    /// A constant in a type: the length of an array, or the value of a
    /// const generic parameter such as the `N` of `Buffer<N>`.
    Const(i128),
    /// Not known yet.
    Infer(InferVar),
}

/// The outermost shape of a type: a struct, a reference, a tuple of three...
/// Two types of different shapes never unify, so an impl whose self type
/// has another shape than a type cannot apply to it, which is quick to see.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TyHead {
    Int(IntTy),
    Float(FloatTy),
    Bool,
    Char,
    Str,
    Never,
    Tuple(usize),
    Array,
    Slice,
    Ref(Mutability),
    Ptr(Mutability),
    Adt(AdtId),
    FnPtr(usize),
    FnItem(FnId),
    Closure(ClosureId),
    Dyn(TraitId),
    Const(i128),
}

impl Ty {
    /// The type's shape, or `None` while it is not known.
    pub fn head(&self) -> Option<TyHead> {
        Some(match self {
            Ty::Int(int) => TyHead::Int(*int),
            Ty::Float(float) => TyHead::Float(*float),
            Ty::Bool => TyHead::Bool,
            Ty::Char => TyHead::Char,
            Ty::Str => TyHead::Str,
            Ty::Never => TyHead::Never,
            Ty::Tuple(elements) => TyHead::Tuple(elements.len()),
            Ty::Array(..) => TyHead::Array,
            Ty::Slice(_) => TyHead::Slice,
            Ty::Ref(_, mutability) => TyHead::Ref(*mutability),
            Ty::Ptr(_, mutability) => TyHead::Ptr(*mutability),
            Ty::Adt(adt, _) => TyHead::Adt(*adt),
            Ty::FnPtr(params, _) => TyHead::FnPtr(params.len()),
            Ty::FnItem(def, _) => TyHead::FnItem(*def),
            Ty::Closure(id) => TyHead::Closure(*id),
            Ty::Dyn(trait_id, _) => TyHead::Dyn(*trait_id),
            Ty::Const(value) => TyHead::Const(*value),
            Ty::Infer(_) => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mutability {
    Not,
    Mut,
}

impl Mutability {
    pub fn from_bool(mutable: bool) -> Mutability {
        if mutable {
            Mutability::Mut
        } else {
            Mutability::Not
        }
    }

    pub fn is_mut(self) -> bool {
        self == Mutability::Mut
    }
}

impl Ty {
    pub const UNIT: Ty = Ty::Tuple(Vec::new());
    pub const I32: Ty = Ty::Int(IntTy::I32);
    pub const I64: Ty = Ty::Int(IntTy::I64);
    pub const U8: Ty = Ty::Int(IntTy::U8);
    pub const USIZE: Ty = Ty::Int(IntTy::Usize);
    pub const F64: Ty = Ty::Float(FloatTy::F64);

    pub fn shared_ref(inner: Ty) -> Ty {
        Ty::Ref(Box::new(inner), Mutability::Not)
    }

    pub fn mut_ref(inner: Ty) -> Ty {
        Ty::Ref(Box::new(inner), Mutability::Mut)
    }

    /// `[element; len]`
    pub fn array(element: Ty, len: u64) -> Ty {
        Ty::Array(Box::new(element), Box::new(Ty::Const(len as i128)))
    }

    /// The length of an array type, once known.
    pub fn array_len(&self) -> Option<u64> {
        match self {
            Ty::Array(_, len) => match **len {
                Ty::Const(len) => Some(len as u64),
                _ => None,
            },
            _ => None,
        }
    }

    /// `&str`
    pub fn str_ref() -> Ty {
        Ty::shared_ref(Ty::Str)
    }

    pub fn is_unit(&self) -> bool {
        matches!(self, Ty::Tuple(elements) if elements.is_empty())
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Ty::Int(_))
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Ty::Float(_))
    }

    /// Types whose values fit in one machine register.
    pub fn is_scalar(&self) -> bool {
        match self {
            Ty::Int(_) | Ty::Float(_) | Ty::Bool | Ty::Char | Ty::FnPtr(..) => true,
            Ty::Ref(pointee, _) | Ty::Ptr(pointee, _) => !pointee.is_unsized(),
            _ => false,
        }
    }

    /// Types with no compile-time size: a pointer to one carries extra data
    /// (a length or a vtable) and is two words wide.
    pub fn is_unsized(&self) -> bool {
        matches!(self, Ty::Str | Ty::Slice(_) | Ty::Dyn(..))
    }

    /// The type a reference or raw pointer points to.
    pub fn pointee(&self) -> Option<&Ty> {
        match self {
            Ty::Ref(pointee, _) | Ty::Ptr(pointee, _) => Some(pointee),
            _ => None,
        }
    }

    /// Does any inference variable occur in this type?
    pub fn has_infer(&self) -> bool {
        let mut found = false;
        self.walk(&mut |ty| found |= matches!(ty, Ty::Infer(_)));
        found
    }

    /// Visit this type and every type nested inside it.
    pub fn walk(&self, visit: &mut impl FnMut(&Ty)) {
        visit(self);
        match self {
            Ty::Tuple(elements) | Ty::Adt(_, elements) | Ty::FnItem(_, elements) | Ty::Dyn(_, elements) => {
                elements.iter().for_each(|ty| ty.walk(visit));
            }
            Ty::Array(inner, len) => {
                inner.walk(visit);
                len.walk(visit);
            }
            Ty::Slice(inner) | Ty::Ref(inner, _) | Ty::Ptr(inner, _) => {
                inner.walk(visit);
            }
            Ty::FnPtr(params, ret) => {
                params.iter().for_each(|ty| ty.walk(visit));
                ret.walk(visit);
            }
            Ty::Int(_)
            | Ty::Float(_)
            | Ty::Bool
            | Ty::Char
            | Ty::Str
            | Ty::Never
            | Ty::Closure(_)
            | Ty::Const(_)
            | Ty::Infer(_) => {}
        }
    }

    /// Rebuild this type, replacing every nested type by `f(nested)`,
    /// innermost first.
    pub fn map(&self, f: &mut dyn FnMut(Ty) -> Ty) -> Ty {
        fn each(types: &[Ty], f: &mut dyn FnMut(Ty) -> Ty) -> Vec<Ty> {
            types.iter().map(|ty| ty.map(f)).collect()
        }
        let rebuilt = match self {
            Ty::Tuple(elements) => Ty::Tuple(each(elements, f)),
            Ty::Adt(id, args) => Ty::Adt(*id, each(args, f)),
            Ty::FnItem(id, args) => Ty::FnItem(*id, each(args, f)),
            Ty::Dyn(id, args) => Ty::Dyn(*id, each(args, f)),
            Ty::Array(inner, len) => Ty::Array(Box::new(inner.map(f)), Box::new(len.map(f))),
            Ty::Slice(inner) => Ty::Slice(Box::new(inner.map(f))),
            Ty::Ref(inner, mutability) => Ty::Ref(Box::new(inner.map(f)), *mutability),
            Ty::Ptr(inner, mutability) => Ty::Ptr(Box::new(inner.map(f)), *mutability),
            Ty::FnPtr(params, ret) => Ty::FnPtr(each(params, f), Box::new(ret.map(f))),
            leaf => leaf.clone(),
        };
        f(rebuilt)
    }
}
