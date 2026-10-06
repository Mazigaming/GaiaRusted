//! Evaluating operations whose operands are constants.
//!
//! Arithmetic is done the way the target does it: on the bits of the
//! operand type, wrapping around. Anything whose outcome is not simply a
//! value (dividing by zero, shifting by the width or more) is left for the
//! program to do, so that it behaves the same optimised or not.

use crate::ir::*;
use crate::sema::ty::{IntTy, Ty};
use crate::syntax::ast::BinOp;

/// The width in bytes and signedness of an integer-like type.
fn integer(ty: &Ty) -> Option<(u64, bool)> {
    match ty {
        Ty::Int(int) => Some((int.size(), int.is_signed())),
        Ty::Bool => Some((1, false)),
        Ty::Char => Some((4, false)),
        _ => None,
    }
}

/// The value the bits of a constant of type `ty` stand for.
fn value(bits: u128, (size, signed): (u64, bool)) -> i128 {
    let shift = 128 - size * 8;
    if signed {
        ((bits << shift) as i128) >> shift
    } else {
        ((bits << shift) >> shift) as i128
    }
}

/// The bits of `value` as a constant of a type `size` bytes wide.
fn bits(value: i128, size: u64) -> u128 {
    (value as u128) & (u128::MAX >> (128 - size * 8))
}

/// The bits that identify an integer constant of type `ty`, whatever
/// form they were written in.
pub fn normalize(constant: u128, ty: &Ty) -> u128 {
    match integer(ty) {
        Some((size, _)) => bits(constant as i128, size),
        None => constant,
    }
}

/// A cheaper rvalue with the same value: `x * 1` is `x`, `x & 0` is `0`,
/// and for unsigned `x`, `x % 8` is `x & 7` and `x / 8` is `x >> 3`.
pub fn simplify(rvalue: &Rvalue) -> Option<Rvalue> {
    if let Some(constant) = self::rvalue(rvalue) {
        return Some(Rvalue::Use(Operand::Const(constant)));
    }
    let constant = |operand: &Operand| match operand {
        Operand::Const(Const::Int(bits, ty)) => integer(ty).map(|class| (value(*bits, class), ty.clone())),
        _ => None,
    };
    let keep = |operand: &Operand| Some(Rvalue::Use(operand.clone()));
    let zero = |ty: Ty| Some(Rvalue::Use(Operand::Const(Const::Int(0, ty))));
    match rvalue {
        Rvalue::Binary(op, lhs, rhs) => match (op, constant(lhs), constant(rhs)) {
            (BinOp::Add | BinOp::Sub | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr, _, Some((0, _))) => keep(lhs),
            (BinOp::Add | BinOp::BitOr | BinOp::BitXor, Some((0, _)), _) => keep(rhs),
            (BinOp::Mul | BinOp::Div, _, Some((1, _))) => keep(lhs),
            // Multiplying by a power of two is shifting, signed or not.
            (BinOp::Mul, _, Some((factor, ty))) if factor > 1 && factor.count_ones() == 1 => {
                let shift = Operand::Const(Const::Int(factor.trailing_zeros() as u128, ty));
                Some(Rvalue::Binary(BinOp::Shl, lhs.clone(), shift))
            }
            (BinOp::Mul, Some((1, _)), _) => keep(rhs),
            (BinOp::Mul | BinOp::BitAnd, _, Some((0, ty))) | (BinOp::Mul | BinOp::BitAnd, Some((0, ty)), _) => zero(ty),
            // Dividing by a power of two is shifting; its remainder is
            // the bits shifted out. (Not for signed numbers, which round
            // towards zero where a shift rounds down.)
            (BinOp::Div | BinOp::Rem, _, Some((divisor, ty)))
                if divisor > 0 && divisor.count_ones() == 1 && integer(&ty).is_some_and(|(_, signed)| !signed) =>
            {
                let (op, operand) = match op {
                    BinOp::Div => (BinOp::Shr, divisor.trailing_zeros() as u128),
                    _ => (BinOp::BitAnd, divisor as u128 - 1),
                };
                Some(Rvalue::Binary(op, lhs.clone(), Operand::Const(Const::Int(operand, ty))))
            }
            _ => None,
        },
        Rvalue::Cast(operand, from, to) if from == to || same_pointer_shape(from, to) => keep(operand),
        _ => None,
    }
}

/// Are both types pointers to the same type, differing at most in being
/// a reference or a raw pointer and in mutability? A cast between them
/// changes nothing. (A cast to a pointer to some other type changes what a
/// dereference finds there, so it is not a copy in a typed IR.)
fn same_pointer_shape(from: &Ty, to: &Ty) -> bool {
    match (from, to) {
        (Ty::Ref(a, _) | Ty::Ptr(a, _), Ty::Ref(b, _) | Ty::Ptr(b, _)) => a == b,
        _ => false,
    }
}

/// The constant an rvalue evaluates to, if its operands are constants.
pub fn rvalue(rvalue: &Rvalue) -> Option<Const> {
    let int = |bits: u128, ty: &Ty| Some(Const::Int(bits, ty.clone()));
    match rvalue {
        Rvalue::Binary(op, Operand::Const(Const::Int(lhs, ty)), Operand::Const(Const::Int(rhs, _)))
            if *ty == Ty::Int(IntTy::U128) =>
        {
            // The one type whose values do not all fit in an `i128`.
            let (a, b) = (*lhs, *rhs);
            let result = match op {
                BinOp::Add => a.wrapping_add(b),
                BinOp::Sub => a.wrapping_sub(b),
                BinOp::Mul => a.wrapping_mul(b),
                BinOp::BitAnd => a & b,
                BinOp::BitOr => a | b,
                BinOp::BitXor => a ^ b,
                BinOp::Shl if b < 128 => a << b,
                BinOp::Shr if b < 128 => a >> b,
                BinOp::Div if b != 0 => a / b,
                BinOp::Rem if b != 0 => a % b,
                BinOp::Eq => return int((a == b) as u128, &Ty::Bool),
                BinOp::Ne => return int((a != b) as u128, &Ty::Bool),
                BinOp::Lt => return int((a < b) as u128, &Ty::Bool),
                BinOp::Le => return int((a <= b) as u128, &Ty::Bool),
                BinOp::Gt => return int((a > b) as u128, &Ty::Bool),
                BinOp::Ge => return int((a >= b) as u128, &Ty::Bool),
                _ => return None,
            };
            int(result, ty)
        }
        Rvalue::Binary(op, Operand::Const(Const::Int(lhs, ty)), Operand::Const(Const::Int(rhs, _))) => {
            let class = integer(ty)?;
            let (size, signed) = class;
            let (a, b) = (value(*lhs, class), value(*rhs, class));
            let width = size as i128 * 8;
            let result = match op {
                BinOp::Add => a.wrapping_add(b),
                BinOp::Sub => a.wrapping_sub(b),
                BinOp::Mul => a.wrapping_mul(b),
                BinOp::BitAnd => a & b,
                BinOp::BitOr => a | b,
                BinOp::BitXor => a ^ b,
                BinOp::Shl if (0..width).contains(&b) => a << b,
                BinOp::Shr if (0..width).contains(&b) => a >> b,
                BinOp::Div | BinOp::Rem if b == 0 || (signed && b == -1 && a == value(1 << (width - 1), class)) => {
                    return None
                }
                BinOp::Div => a / b,
                BinOp::Rem => a % b,
                BinOp::Eq => return int((a == b) as u128, &Ty::Bool),
                BinOp::Ne => return int((a != b) as u128, &Ty::Bool),
                BinOp::Lt => return int((a < b) as u128, &Ty::Bool),
                BinOp::Le => return int((a <= b) as u128, &Ty::Bool),
                BinOp::Gt => return int((a > b) as u128, &Ty::Bool),
                BinOp::Ge => return int((a >= b) as u128, &Ty::Bool),
                _ => return None,
            };
            int(bits(result, size), ty)
        }
        Rvalue::Unary(op, Operand::Const(Const::Int(operand, ty))) => {
            let class = integer(ty)?;
            let a = value(*operand, class);
            let result = match op {
                UnaryOp::Neg => a.wrapping_neg(),
                UnaryOp::Not if *ty == Ty::Bool => a ^ 1,
                UnaryOp::Not => !a,
                UnaryOp::Sqrt => return None,
            };
            int(bits(result, class.0), ty)
        }
        Rvalue::Cast(Operand::Const(Const::Int(operand, _)), from, to @ Ty::Int(_)) => {
            let a = value(*operand, integer(from)?);
            int(bits(a, integer(to)?.0), to)
        }
        Rvalue::FatExtra(Operand::Const(Const::Str(_, len))) => int(*len as u128, &Ty::USIZE),
        _ => None,
    }
}
