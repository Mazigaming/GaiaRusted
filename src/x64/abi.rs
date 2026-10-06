//! The calling convention.
//!
//! Scalars follow the System V AMD64 convention, so compiled code can call
//! C functions and be called by them: integers and pointers in `rdi, rsi,
//! rdx, rcx, r8, r9`, floats in `xmm0..xmm7`, the rest on the stack. On top
//! of that:
//!
//! * a fat pointer (`&str`, `&[T]`, `&dyn Trait`) travels as two integers;
//! * any other aggregate is passed as a pointer to memory owned by the
//!   callee for the duration of the call, and returned through a pointer
//!   the caller supplies as a hidden first argument.

use super::reg::Reg;
use crate::sema::context::Context;
use crate::sema::ty::Ty;

/// How a value is stored in registers or memory, which decides how it is
/// loaded, stored, passed and returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// No bits at all: nothing to load, store or pass.
    Zst,
    /// An integer, `bool`, `char` or thin pointer, in a general register.
    Int { size: u64, signed: bool },
    /// A float, in an SSE register.
    Float { size: u64 },
    /// Two words in two general registers: a fat pointer, or a 128-bit
    /// integer (low word first, as System V passes `__int128`).
    Pair,
    /// Anything else: lives in memory and is handled by address.
    Memory { size: u64 },
}

pub fn classify(tcx: &Context, ty: &Ty) -> Class {
    match ty {
        Ty::Int(int) if int.size() == 16 => Class::Pair,
        Ty::Int(int) => Class::Int { size: int.size(), signed: int.is_signed() },
        Ty::Bool => Class::Int { size: 1, signed: false },
        Ty::Char => Class::Int { size: 4, signed: false },
        Ty::FnPtr(..) => Class::Int { size: 8, signed: false },
        Ty::Float(float) => Class::Float { size: float.size() },
        Ty::Ref(pointee, _) | Ty::Ptr(pointee, _) => {
            if tcx.is_unsized(pointee) {
                Class::Pair
            } else {
                Class::Int { size: 8, signed: false }
            }
        }
        _ => match tcx.size_of(ty) {
            0 => Class::Zst,
            size => Class::Memory { size },
        },
    }
}

/// Where one argument is found at the moment of the call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArgLoc {
    /// Not passed at all (zero-sized).
    None,
    /// In general registers: one, or two for a fat pointer.
    Regs(Vec<Reg>),
    /// In the SSE register with this number.
    Xmm(u8),
    /// On the stack, starting at this word above the return address.
    Stack(usize),
}

pub struct CallLayout {
    /// The class and location of each argument, in order.
    pub args: Vec<(Class, ArgLoc)>,
    pub ret: Class,
    /// Number of 8-byte words of stack the arguments occupy.
    pub stack_words: usize,
    /// Number of SSE registers used; C variadic functions are told this in `al`.
    pub xmm_used: u8,
}

const INT_ARG_REGS: [Reg; 6] = [Reg::Rdi, Reg::Rsi, Reg::Rdx, Reg::Rcx, Reg::R8, Reg::R9];
const XMM_ARG_COUNT: u8 = 8;

impl CallLayout {
    /// A value returned in memory is written through a pointer passed as a
    /// hidden first integer argument.
    pub fn returns_in_memory(&self) -> bool {
        matches!(self.ret, Class::Memory { .. })
    }
}

pub fn layout_call(tcx: &Context, arg_tys: &[Ty], ret_ty: &Ty) -> CallLayout {
    let ret = classify(tcx, ret_ty);
    let mut int_regs = INT_ARG_REGS.iter().copied();
    if matches!(ret, Class::Memory { .. }) {
        int_regs.next();
    }
    let mut xmm_used = 0;
    let mut stack_words = 0;

    let mut args = Vec::with_capacity(arg_tys.len());
    for ty in arg_tys {
        let class = classify(tcx, ty);
        let words = match class {
            Class::Zst => {
                args.push((class, ArgLoc::None));
                continue;
            }
            Class::Float { .. } if xmm_used < XMM_ARG_COUNT => {
                args.push((class, ArgLoc::Xmm(xmm_used)));
                xmm_used += 1;
                continue;
            }
            Class::Pair => 2,
            _ => 1,
        };
        // A fat pointer goes in two registers or not in registers at all.
        let loc = if !matches!(class, Class::Float { .. }) && int_regs.len() >= words {
            ArgLoc::Regs(int_regs.by_ref().take(words).collect())
        } else {
            stack_words += words;
            ArgLoc::Stack(stack_words - words)
        };
        args.push((class, loc));
    }
    CallLayout { args, ret, stack_words, xmm_used }
}
