//! Walking the AST.

use super::ast::*;

/// Call `visit` for every item declared inside `block`, at any nesting depth
/// (functions and types may be declared in the middle of a function body).
/// Items found this way are not searched again; their own bodies are the
/// caller's to visit.
pub fn nested_items<'a>(block: &'a Block, visit: &mut dyn FnMut(&'a Item)) {
    for stmt in &block.stmts {
        match &stmt.kind {
            StmtKind::Item(item) => visit(item),
            StmtKind::Expr(expr) => items_in_expr(expr, visit),
            StmtKind::Let { init, else_block, .. } => {
                if let Some(init) = init {
                    items_in_expr(init, visit);
                }
                if let Some(block) = else_block {
                    nested_items(block, visit);
                }
            }
        }
    }
    if let Some(tail) = &block.expr {
        items_in_expr(tail, visit);
    }
}

fn items_in_expr<'a>(expr: &'a Expr, visit: &mut dyn FnMut(&'a Item)) {
    let mut each = |exprs: &mut dyn Iterator<Item = &'a Expr>| {
        for expr in exprs {
            items_in_expr(expr, visit);
        }
    };
    match &expr.kind {
        ExprKind::Int(..)
        | ExprKind::Float(..)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Str(_)
        | ExprKind::Byte(_)
        | ExprKind::ByteStr(_)
        | ExprKind::Path(_)
        | ExprKind::Underscore
        | ExprKind::Continue { .. } => {}

        ExprKind::Unary(_, a)
        | ExprKind::AddrOf { expr: a, .. }
        | ExprKind::Cast(a, _)
        | ExprKind::Field(a, _)
        | ExprKind::TupleField(a, _)
        | ExprKind::Try(a)
        | ExprKind::Let(_, a) => each(&mut [&**a].into_iter()),

        ExprKind::Binary(_, a, b)
        | ExprKind::Assign(a, b)
        | ExprKind::AssignOp(_, a, b)
        | ExprKind::Index(a, b)
        | ExprKind::Repeat(a, b) => each(&mut [&**a, &**b].into_iter()),

        ExprKind::Call(callee, args) => each(&mut std::iter::once(&**callee).chain(args)),
        ExprKind::MethodCall { receiver, args, .. } => {
            each(&mut std::iter::once(&**receiver).chain(args))
        }
        ExprKind::Tuple(elements) | ExprKind::Array(elements) => each(&mut elements.iter()),
        ExprKind::Struct { fields, base, .. } => {
            each(&mut fields.iter().map(|field| &field.value).chain(base.as_deref()))
        }
        ExprKind::Range { lo, hi, .. } => each(&mut lo.as_deref().into_iter().chain(hi.as_deref())),
        ExprKind::Return(value) | ExprKind::Break { value, .. } => each(&mut value.as_deref().into_iter()),

        ExprKind::Block(block)
        | ExprKind::Unsafe(block)
        | ExprKind::Loop { body: block, .. }
        | ExprKind::LabeledBlock { body: block, .. } => {
            nested_items(block, visit)
        }
        ExprKind::If { cond, then_block, else_expr } => {
            each(&mut std::iter::once(&**cond).chain(else_expr.as_deref()));
            nested_items(then_block, visit);
        }
        ExprKind::While { cond, body, .. } => {
            items_in_expr(cond, visit);
            nested_items(body, visit);
        }
        ExprKind::For { iter, body, .. } => {
            items_in_expr(iter, visit);
            nested_items(body, visit);
        }
        ExprKind::Match { scrutinee, arms } => {
            let arm_exprs = arms.iter().flat_map(|arm| arm.guard.iter().chain([&arm.body]));
            each(&mut std::iter::once(&**scrutinee).chain(arm_exprs));
        }
        ExprKind::Closure { body, .. } => items_in_expr(body, visit),
        ExprKind::Macro(call) => match &call.args {
            MacroArgs::List(args) => each(&mut args.iter()),
            MacroArgs::Repeat(a, b) => each(&mut [&**a, &**b].into_iter()),
            MacroArgs::Matches(value, _, guard) => {
                each(&mut std::iter::once(&**value).chain(guard.as_deref()))
            }
        },
    }
}
