//! Compile-time evaluation of integer expressions: array lengths, const
//! generic arguments, enum discriminants and the bounds of range patterns.

use super::context::{Context, GenericEnv, TypeScope};
use super::defs::{Def, ModId, Resolution};
use super::infer::InferTable;
use super::ty::Ty;
use crate::syntax::ast::{BinOp, Expr, ExprKind, Path, UnOp};
use crate::syntax::diagnostic::{bail, Result};

pub fn eval_int(tcx: &Context, module: ModId, expr: &Expr) -> Result<i128> {
    eval_int_in(tcx, module, &GenericEnv::new(), expr)
}

/// Evaluate `expr` where the generic parameters `generics` are in scope;
/// a const parameter stands for its value.
pub fn eval_int_in(tcx: &Context, module: ModId, generics: &GenericEnv, expr: &Expr) -> Result<i128> {
    let eval_int = |tcx, module, expr| eval_int_in(tcx, module, generics, expr);
    let overflow = || crate::syntax::Diagnostic::new(expr.span, "constant evaluation overflowed");
    Ok(match &expr.kind {
        ExprKind::Int(value, _) => *value as i128,
        ExprKind::Char(c) => *c as i128,
        ExprKind::Byte(b) => *b as i128,
        ExprKind::Bool(b) => *b as i128,
        ExprKind::Cast(inner, _) => eval_int(tcx, module, inner)?,
        ExprKind::Unary(UnOp::Neg, inner) => {
            eval_int(tcx, module, inner)?.checked_neg().ok_or_else(overflow)?
        }
        ExprKind::Binary(op, lhs, rhs) => {
            let (l, r) = (eval_int(tcx, module, lhs)?, eval_int(tcx, module, rhs)?);
            let folded = match op {
                BinOp::Add => l.checked_add(r),
                BinOp::Sub => l.checked_sub(r),
                BinOp::Mul => l.checked_mul(r),
                BinOp::Div if r != 0 => l.checked_div(r),
                BinOp::Rem if r != 0 => l.checked_rem(r),
                BinOp::Div | BinOp::Rem => bail!(expr.span, "division by zero in a constant"),
                BinOp::BitAnd => Some(l & r),
                BinOp::BitOr => Some(l | r),
                BinOp::BitXor => Some(l ^ r),
                BinOp::Shl => u32::try_from(r).ok().and_then(|r| l.checked_shl(r)),
                BinOp::Shr => u32::try_from(r).ok().and_then(|r| l.checked_shr(r)),
                _ => bail!(expr.span, "this operator is not allowed in a constant integer expression"),
            };
            folded.ok_or_else(overflow)?
        }
        ExprKind::Block(block) if block.stmts.is_empty() && block.expr.is_some() => {
            eval_int(tcx, module, block.expr.as_deref().expect("checked just now"))?
        }
        ExprKind::Path(path) if path.as_ident().is_some_and(|name| generics.contains_key(&name.name)) => {
            let name = &path.as_ident().expect("checked just now").name;
            match generics[name] {
                Ty::Const(value) => value,
                _ => bail!(expr.span, "the value of `{name}` is not known here"),
            }
        }
        ExprKind::Path(path) => match tcx.defs.resolve_path(module, path) {
            Some(Resolution { def: Def::Const(id), rest: [] }) => {
                let constant = tcx.defs.const_def(id);
                match &constant.ast.value {
                    Some(value) => eval_int(tcx, constant.module, value)?,
                    None => bail!(expr.span, "this constant has no value"),
                }
            }
            // `i32::MAX`: a constant associated with a type.
            _ => match associated_const(tcx, module, path)? {
                Some((constant_module, value)) => eval_int(tcx, constant_module, value)?,
                None => bail!(expr.span, "expected a constant integer"),
            },
        },
        _ => bail!(expr.span, "expected a constant integer expression"),
    })
}

/// The value of `Type::NAME`, with the module it must be evaluated in.
fn associated_const<'a>(
    tcx: &Context<'a>,
    module: ModId,
    path: &Path,
) -> Result<Option<(ModId, &'a Expr)>> {
    let [.., name] = path.segments.as_slice() else { return Ok(None) };
    if path.segments.len() < 2 {
        return Ok(None);
    }
    let env = GenericEnv::new();
    let scope = TypeScope { module, generics: &env, self_ty: None, self_trait: None };
    let mut infer = InferTable::default();
    let Ok((ty, [_])) = tcx.lower_path_ty_prefix(scope, path, &mut infer, &mut Vec::new(), true) else {
        return Ok(None);
    };
    for &impl_id in tcx.defs.impls_by_member.get(&name.ident.name).into_iter().flatten() {
        let block = tcx.defs.impl_def(impl_id);
        let Some(&constant) = block.consts.get(&name.ident.name) else { continue };
        if tcx.match_impl(impl_id, &ty, &mut infer)?.is_some() {
            let constant = tcx.defs.const_def(constant);
            return Ok(constant.ast.value.as_ref().map(|value| (constant.module, value)));
        }
    }
    Ok(None)
}
