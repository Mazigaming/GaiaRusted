//! Looking at types as the program runs.

/// The name of a type, written out in full: `alloc::vec::Vec<i32>`.
pub fn type_name<T: ?Sized>() -> &'static str {
    std::intrinsics::type_name::<T>()
}

/// The name of the type of the value `value` points to.
pub fn type_name_of_val<T: ?Sized>(value: &T) -> &'static str {
    std::intrinsics::type_name::<T>()
}

/// A type's identity at run time: the hash of its full name, which only
/// that type has.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TypeId {
    hash: u64,
}

impl TypeId {
    pub fn of<T: ?Sized>() -> TypeId {
        // FNV-1a over the name.
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in type_name::<T>().bytes() {
            hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
        }
        TypeId { hash }
    }
}

/// A value whose type can be asked for at run time, so that a `dyn Any`
/// can be turned back into the concrete type it holds.
pub trait Any {
    fn type_id(&self) -> TypeId;
}

impl<T: ?Sized> Any for T {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl dyn Any {
    pub fn is<T: Any>(&self) -> bool {
        self.type_id() == TypeId::of::<T>()
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        if self.is::<T>() {
            Some(unsafe { &*(self as *const dyn Any as *const T) })
        } else {
            None
        }
    }

    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        if self.is::<T>() {
            Some(unsafe { &mut *(self as *mut dyn Any as *mut T) })
        } else {
            None
        }
    }
}

impl std::fmt::Debug for dyn Any {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Any { .. }")
    }
}
