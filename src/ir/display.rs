//! A readable dump of the IR, for `--emit ir` and for debugging the compiler.

use super::*;
use crate::sema::context::Context;
use std::fmt::Write;

/// Render the whole program.
pub fn dump(program: &Program, tcx: &Context) -> String {
    let mut out = String::new();
    for function in &program.functions {
        dump_function(&mut out, program, function, tcx);
        out.push('\n');
    }
    out
}

fn dump_function(out: &mut String, program: &Program, function: &Function, tcx: &Context) {
    let params: Vec<String> = function
        .args()
        .map(|arg| format!("_{}: {}", arg.0, tcx.display(&function.locals[arg.0 as usize].ty)))
        .collect();
    let _ = writeln!(out, "fn {}({}) -> {} {{", function.name, params.join(", "), tcx.display(function.ret_ty()));
    for (index, local) in function.locals.iter().enumerate().skip(function.arg_count + 1) {
        let name = local.name.as_deref().map(|name| format!("  // {name}")).unwrap_or_default();
        let _ = writeln!(out, "    let _{index}: {};{name}", tcx.display(&local.ty));
    }
    for (index, block) in function.blocks.iter().enumerate() {
        let _ = writeln!(out, "  bb{index}:");
        for statement in &block.statements {
            let _ = writeln!(out, "    {}", show_statement(program, statement));
        }
        let terminator = match &block.terminator {
            Some(terminator) => show_terminator(terminator),
            None => "<unterminated>".to_string(),
        };
        let _ = writeln!(out, "    {terminator}");
    }
    out.push_str("}\n");
}

fn show_place(place: &Place) -> String {
    let mut text = format!("_{}", place.local.0);
    for projection in &place.projection {
        text = match projection {
            Projection::Field(index) => format!("{text}.{index}"),
            Projection::Deref => format!("(*{text})"),
            Projection::Index(index) => format!("{text}[_{}]", index.0),
            Projection::Downcast(variant) => format!("({text} as variant {variant})"),
        };
    }
    text
}

fn show_operand(program: &Program, operand: &Operand) -> String {
    match operand {
        Operand::Copy(place) => show_place(place),
        Operand::Const(Const::Int(value, _)) => format!("{}", *value as i128 as i64),
        Operand::Const(Const::Float(value, _)) => format!("{value:?}"),
        Operand::Const(Const::Str(data, len)) => format!("str({}, {len})", program.data[data.0 as usize].symbol),
        Operand::Const(Const::DataAddr(data, _)) => format!("&{}", program.data[data.0 as usize].symbol),
        Operand::Const(Const::FuncAddr(func, _)) => format!("fn#{}", func.0),
        Operand::Const(Const::ZeroSized(_)) => "()".to_string(),
    }
}

fn show_statement(program: &Program, statement: &Statement) -> String {
    let operand = |operand| show_operand(program, operand);
    match statement {
        Statement::Assign(place, rvalue) => {
            let value = match rvalue {
                Rvalue::Use(value) => operand(value),
                Rvalue::Binary(op, lhs, rhs) => format!("{} {} {}", operand(lhs), op.symbol(), operand(rhs)),
                Rvalue::Unary(op, value) => format!("{op:?}({})", operand(value)),
                Rvalue::Cast(value, ..) => format!("cast({})", operand(value)),
                Rvalue::AddrOf(place) => format!("&{}", show_place(place)),
                Rvalue::Discriminant(place) => format!("discriminant({})", show_place(place)),
                Rvalue::MakeFat(data, extra) => format!("fat({}, {})", operand(data), operand(extra)),
                Rvalue::FatData(pointer) => format!("fat_data({})", operand(pointer)),
                Rvalue::FatExtra(pointer) => format!("fat_extra({})", operand(pointer)),
            };
            format!("{} = {value}", show_place(place))
        }
        Statement::SetDiscriminant(place, variant) => {
            format!("discriminant({}) = {variant}", show_place(place))
        }
        Statement::Call { dest, callee, args } => {
            let callee = match callee {
                Callee::Direct(func) => match program.functions.get(func.0 as usize) {
                    Some(function) => function.name.clone(),
                    None => format!("fn#{}", func.0),
                },
                Callee::Extern { symbol, .. } => format!("extern {symbol}"),
                Callee::Indirect(pointer) => format!("(*{})", operand(pointer)),
                Callee::Virtual { index } => format!("vtable[{index}]"),
            };
            let args: Vec<String> = args.iter().map(operand).collect();
            format!("{} = {callee}({})", show_place(dest), args.join(", "))
        }
    }
}

fn show_terminator(terminator: &Terminator) -> String {
    match terminator {
        Terminator::Goto(target) => format!("goto bb{}", target.0),
        Terminator::Branch { cond, then_block, else_block } => {
            let cond = match cond {
                Operand::Copy(place) => show_place(place),
                Operand::Const(_) => "const".to_string(),
            };
            format!("if {cond} goto bb{} else bb{}", then_block.0, else_block.0)
        }
        Terminator::Switch { arms, otherwise, .. } => {
            let arms: Vec<String> = arms.iter().map(|(value, block)| format!("{value} => bb{}", block.0)).collect();
            format!("switch [{}, _ => bb{}]", arms.join(", "), otherwise.0)
        }
        Terminator::Return => "return".to_string(),
        Terminator::Unreachable => "unreachable".to_string(),
    }
}
