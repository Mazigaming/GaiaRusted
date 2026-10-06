//! Turning recursion at the end of a function into a loop.
//!
//! A call of the function itself whose result is returned as it is,
//! `return f(x)`, needs no new frame: assign the arguments to the
//! parameters and start over. The same goes for `return f(x) + e`, when
//! `+` is an operation whose operands can be regrouped and reordered: keep
//! a running total, add `e` to it, start over, and add the total to
//! whatever the function finally returns. This is how
//! `fib(n - 1) + fib(n - 2)` comes to make one call per level instead of two.
//!
//! Integer arithmetic here wraps, so integer `+`, `*`, `&`, `|` and `^`
//! all qualify. Floating point does not: regrouping changes its rounding.

use crate::ir::*;
use crate::sema::context::Context;
use crate::sema::ty::Ty;
use crate::syntax::ast::BinOp;

/// A place where the function calls itself and then returns.
struct Site {
    block: usize,
    args: Vec<Operand>,
    /// For `f(x) op e`, the operation and `e`.
    pending: Option<(BinOp, Operand)>,
    /// How many statements at the end of the block are the call and the
    /// operation, which the jump replaces.
    statements: usize,
}

/// The value that leaves anything unchanged under `op`.
fn identity(op: BinOp, ty: &Ty) -> Option<Const> {
    let Ty::Int(int) = ty else { return None };
    let bits = match op {
        BinOp::Add | BinOp::BitOr | BinOp::BitXor => 0,
        BinOp::Mul => 1,
        BinOp::BitAnd => u128::MAX >> (128 - int.size() * 8),
        _ => return None,
    };
    Some(Const::Int(bits, ty.clone()))
}

/// Returns whether any call was turned into a jump.
pub fn run(func: &mut Function, this: FuncId, tcx: &Context) -> bool {
    // Parameters that are handed over in memory would have to be copied
    // into that memory; keep to the ones that travel in registers.
    let params_fit = func.args().all(|param| {
        let ty = &func.locals[param.0 as usize].ty;
        tcx.is_register_value(ty) || tcx.is_zero_sized(ty)
    });
    if !params_fit {
        return false;
    }
    let sites = find_sites(func, this);
    // Every site that keeps a running total must keep it with the same
    // operation.
    let op = sites.iter().find_map(|site| site.pending.as_ref().map(|(op, _)| *op));
    let sites: Vec<Site> =
        sites.into_iter().filter(|site| site.pending.as_ref().is_none_or(|(other, _)| Some(*other) == op)).collect();
    if sites.is_empty() {
        return false;
    }

    // The old entry becomes the top of the loop; the new entry sets up the
    // running total.
    let top = BlockId(func.blocks.len() as u32);
    for block in &mut func.blocks {
        if let Some(terminator) = &mut block.terminator {
            terminator.each_target_mut(&mut |target| {
                if target.0 == 0 {
                    *target = top;
                }
            });
        }
    }
    let old_entry = std::mem::take(&mut func.blocks[0]);
    func.blocks.push(old_entry);
    let mut entry = Block { statements: Vec::new(), terminator: Some(Terminator::Goto(top)) };

    let total = op.map(|op| {
        let ty = func.ret_ty().clone();
        let start = identity(op, &ty).expect("only operations with an identity were accepted");
        func.locals.push(LocalDecl { ty, name: Some("total".to_string()) });
        let total = Local(func.locals.len() as u32 - 1);
        entry.statements.push(Statement::Assign(Place::local(total), Rvalue::Use(Operand::Const(start))));
        (op, total)
    });
    func.blocks[0] = entry;

    let params: Vec<Local> = func.args().collect();
    let mut site_blocks = Vec::new();
    for site in sites {
        // The site's block moved if it was the entry.
        let block = if site.block == 0 { top.0 as usize } else { site.block };
        site_blocks.push(block);
        let kept = func.blocks[block].statements.len() - site.statements;
        func.blocks[block].statements.truncate(kept);
        // The arguments are all computed before any parameter changes.
        let mut statements = Vec::new();
        let mut staged = Vec::new();
        for (param, arg) in params.iter().zip(site.args) {
            func.locals.push(func.locals[param.0 as usize].clone());
            let temp = Local(func.locals.len() as u32 - 1);
            statements.push(Statement::Assign(Place::local(temp), Rvalue::Use(arg)));
            staged.push((*param, temp));
        }
        if let (Some((op, operand)), Some((_, total))) = (site.pending, total) {
            let sum = Rvalue::Binary(op, Operand::Copy(Place::local(total)), operand);
            statements.push(Statement::Assign(Place::local(total), sum));
        }
        for (param, temp) in staged {
            statements.push(Statement::Assign(Place::local(param), Rvalue::Use(Operand::Copy(Place::local(temp)))));
        }
        let block = &mut func.blocks[block];
        block.statements.extend(statements);
        block.terminator = Some(Terminator::Goto(top));
    }

    // Every way out now adds the running total to the result.
    if let Some((op, total)) = total {
        for (index, block) in func.blocks.iter_mut().enumerate() {
            if matches!(block.terminator, Some(Terminator::Return)) && !site_blocks.contains(&index) {
                let result = Place::local(RETURN_LOCAL);
                let sum = Rvalue::Binary(op, Operand::Copy(Place::local(total)), Operand::Copy(result.clone()));
                block.statements.push(Statement::Assign(result, sum));
            }
        }
    }
    true
}

/// The places where the function calls itself and returns the result,
/// alone or combined with one other value.
fn find_sites(func: &Function, this: FuncId) -> Vec<Site> {
    let returns = |func: &Function, block: &Block| match &block.terminator {
        Some(Terminator::Return) => true,
        Some(Terminator::Goto(next)) => {
            let next = &func.blocks[next.0 as usize];
            next.statements.is_empty() && matches!(next.terminator, Some(Terminator::Return))
        }
        _ => false,
    };
    let ret_ty = func.ret_ty().clone();
    let mut sites = Vec::new();
    for index in 0..func.blocks.len() {
        let block = &func.blocks[index];
        if !returns(func, block) {
            continue;
        }
        let site = match block.statements.as_slice() {
            [.., Statement::Call { dest, callee: Callee::Direct(callee), args }]
                if *callee == this && dest.as_local() == Some(RETURN_LOCAL) =>
            {
                Some(Site { block: index, args: args.clone(), pending: None, statements: 1 })
            }
            [.., Statement::Call { dest, callee: Callee::Direct(callee), args }, Statement::Assign(result, Rvalue::Binary(op, lhs, rhs))]
                if *callee == this && result.as_local() == Some(RETURN_LOCAL) && identity(*op, &ret_ty).is_some() =>
            {
                let call_result = dest.as_local().filter(|&local| local != RETURN_LOCAL);
                let is_result = |operand: &Operand| matches!(operand, Operand::Copy(place) if place.as_local().is_some() && place.as_local() == call_result);
                let mentions_result = |operand: &Operand| matches!(operand, Operand::Copy(place) if Some(place.local) == call_result);
                let other = match (is_result(lhs), is_result(rhs)) {
                    (true, false) if !mentions_result(rhs) => Some(rhs.clone()),
                    (false, true) if !mentions_result(lhs) => Some(lhs.clone()),
                    _ => None,
                };
                other.map(|other| Site { block: index, args: args.clone(), pending: Some((*op, other)), statements: 2 })
            }
            _ => None,
        };
        sites.extend(site);
    }
    sites
}
