//! Walking the places a statement or terminator touches.
//!
//! Every analysis and transformation of the IR starts from the same
//! question: which places does this statement read, which does it write,
//! and whose address does it take? The answer is given once, here, so that
//! a new kind of statement only has to be taught to one piece of code.

use super::*;

/// What a statement does with a place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    /// Its address is taken; it may be read or written through the
    /// resulting pointer at any later time.
    Address,
}

/// What a statement does with a local, derived from the access to a place
/// rooted at it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalUse {
    /// Its whole value is read.
    Read,
    /// Its whole value is replaced.
    Write,
    /// Part of it is read or written in place, or its address is taken:
    /// the local has to exist in memory.
    InMemory,
}

impl Place {
    /// Report how the locals of this place are used when the place itself
    /// is accessed as `access`.
    ///
    /// `(*p).field = x` does not write `p`: it reads `p` to find out where
    /// to write. Likewise every local used as an index is read.
    pub fn each_local(&self, access: Access, visit: &mut dyn FnMut(Local, LocalUse)) {
        for projection in &self.projection {
            if let Projection::Index(index) = projection {
                visit(*index, LocalUse::Read);
            }
        }
        let base = match (self.projection.first(), access) {
            (None, Access::Read) => LocalUse::Read,
            (None, Access::Write) => LocalUse::Write,
            (None, Access::Address) => LocalUse::InMemory,
            (Some(Projection::Deref), _) => LocalUse::Read,
            (Some(_), _) => LocalUse::InMemory,
        };
        visit(self.local, base);
    }
}

/// The traversal is the same whether the places are only looked at or also
/// rewritten, so it is written once and instantiated for `&` and `&mut`.
macro_rules! place_visitors {
    ($each_place:ident, $($mutable:tt)?) => {
        impl Operand {
            pub fn $each_place(&$($mutable)? self, visit: &mut dyn FnMut(&$($mutable)? Place, Access)) {
                if let Operand::Copy(place) = self {
                    visit(place, Access::Read);
                }
            }
        }

        impl Rvalue {
            pub fn $each_place(&$($mutable)? self, visit: &mut dyn FnMut(&$($mutable)? Place, Access)) {
                match self {
                    Rvalue::Use(operand)
                    | Rvalue::Unary(_, operand)
                    | Rvalue::Cast(operand, ..)
                    | Rvalue::FatData(operand)
                    | Rvalue::FatExtra(operand) => operand.$each_place(visit),
                    Rvalue::Binary(_, lhs, rhs) | Rvalue::MakeFat(lhs, rhs) => {
                        lhs.$each_place(visit);
                        rhs.$each_place(visit);
                    }
                    Rvalue::AddrOf(place) => visit(place, Access::Address),
                    Rvalue::Discriminant(place) => visit(place, Access::Read),
                }
            }
        }

        impl Statement {
            /// Visit the places of the statement: what it reads first, then
            /// what it writes.
            pub fn $each_place(&$($mutable)? self, visit: &mut dyn FnMut(&$($mutable)? Place, Access)) {
                match self {
                    Statement::Assign(place, rvalue) => {
                        rvalue.$each_place(visit);
                        visit(place, Access::Write);
                    }
                    Statement::SetDiscriminant(place, _) => visit(place, Access::Write),
                    Statement::Call { dest, callee, args } => {
                        if let Callee::Indirect(pointer) = callee {
                            pointer.$each_place(visit);
                        }
                        for arg in args {
                            arg.$each_place(visit);
                        }
                        visit(dest, Access::Write);
                    }
                }
            }
        }

        impl Terminator {
            pub fn $each_place(&$($mutable)? self, visit: &mut dyn FnMut(&$($mutable)? Place, Access)) {
                match self {
                    Terminator::Branch { cond: operand, .. } | Terminator::Switch { value: operand, .. } => {
                        operand.$each_place(visit)
                    }
                    Terminator::Goto(_) | Terminator::Return | Terminator::Unreachable => {}
                }
            }
        }
    };
}

place_visitors!(each_place,);
place_visitors!(each_place_mut, mut);

macro_rules! operand_visitors {
    ($each_operand:ident, $iterate:ident, $($mutable:tt)?) => {
        impl Rvalue {
            pub fn $each_operand(&$($mutable)? self, visit: &mut dyn FnMut(&$($mutable)? Operand)) {
                match self {
                    Rvalue::Use(operand)
                    | Rvalue::Unary(_, operand)
                    | Rvalue::Cast(operand, ..)
                    | Rvalue::FatData(operand)
                    | Rvalue::FatExtra(operand) => visit(operand),
                    Rvalue::Binary(_, lhs, rhs) | Rvalue::MakeFat(lhs, rhs) => {
                        visit(lhs);
                        visit(rhs);
                    }
                    Rvalue::AddrOf(_) | Rvalue::Discriminant(_) => {}
                }
            }
        }

        impl Statement {
            /// Visit the values the statement computes with.
            pub fn $each_operand(&$($mutable)? self, visit: &mut dyn FnMut(&$($mutable)? Operand)) {
                match self {
                    Statement::Assign(_, rvalue) => rvalue.$each_operand(visit),
                    Statement::SetDiscriminant(..) => {}
                    Statement::Call { callee, args, .. } => {
                        if let Callee::Indirect(pointer) = callee {
                            visit(pointer);
                        }
                        args.$iterate().for_each(visit);
                    }
                }
            }
        }

        impl Terminator {
            pub fn $each_operand(&$($mutable)? self, visit: &mut dyn FnMut(&$($mutable)? Operand)) {
                match self {
                    Terminator::Branch { cond: operand, .. } | Terminator::Switch { value: operand, .. } => visit(operand),
                    Terminator::Goto(_) | Terminator::Return | Terminator::Unreachable => {}
                }
            }
        }
    };
}

operand_visitors!(each_operand, iter,);
operand_visitors!(each_operand_mut, iter_mut, mut);

impl Statement {
    /// Report how each local the statement mentions is used.
    pub fn each_local(&self, visit: &mut dyn FnMut(Local, LocalUse)) {
        match self {
            // The tag is a part of the value: this writes into the place
            // the way a write through a pointer to it would.
            Statement::SetDiscriminant(place, _) => place.each_local(Access::Address, visit),
            _ => self.each_place(&mut |place, access| place.each_local(access, visit)),
        }
    }
}

impl Terminator {
    /// Report how each local the terminator mentions is used. Returning
    /// reads the return value.
    pub fn each_local(&self, visit: &mut dyn FnMut(Local, LocalUse)) {
        self.each_place(&mut |place, access| place.each_local(access, visit));
        if let Terminator::Return = self {
            visit(RETURN_LOCAL, LocalUse::Read);
        }
    }

    pub fn each_target_mut(&mut self, visit: &mut dyn FnMut(&mut BlockId)) {
        match self {
            Terminator::Goto(target) => visit(target),
            Terminator::Branch { then_block, else_block, .. } => {
                visit(then_block);
                visit(else_block);
            }
            Terminator::Switch { arms, otherwise, .. } => {
                arms.iter_mut().for_each(|(_, target)| visit(target));
                visit(otherwise);
            }
            Terminator::Return | Terminator::Unreachable => {}
        }
    }
}
