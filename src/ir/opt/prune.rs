//! Dropping functions nothing refers to any more, such as those whose
//! every call was inlined.

use crate::ir::*;

pub fn run(program: &mut Program) {
    let count = program.functions.len();
    let mut reachable = vec![false; count];
    let mut pending = vec![program.entry];
    pending.extend(&program.initializers);
    // Whatever data mentions (a vtable's methods) may be called from anywhere.
    for item in program.data.iter().flat_map(|data| &data.items) {
        if let DataItem::FuncAddr(func) = item {
            pending.push(*func);
        }
    }
    while let Some(func) = pending.pop() {
        if std::mem::replace(&mut reachable[func.0 as usize], true) {
            continue;
        }
        each_function_reference(&mut program.functions[func.0 as usize], &mut |referenced| pending.push(*referenced));
    }

    let mut new_ids = Vec::with_capacity(count);
    let mut kept = 0;
    for &keep in &reachable {
        new_ids.push(FuncId(kept));
        kept += keep as u32;
    }
    let renumber = |func: &mut FuncId| *func = new_ids[func.0 as usize];

    let mut index = 0;
    program.functions.retain(|_| {
        index += 1;
        reachable[index - 1]
    });
    for function in &mut program.functions {
        each_function_reference(function, &mut |func| renumber(func));
    }
    for item in program.data.iter_mut().flat_map(|data| &mut data.items) {
        if let DataItem::FuncAddr(func) = item {
            renumber(func);
        }
    }
    renumber(&mut program.entry);
    program.initializers.iter_mut().for_each(renumber);
}

/// Visit every mention of a function in `func`: the calls and the
/// function addresses taken.
fn each_function_reference(func: &mut Function, visit: &mut dyn FnMut(&mut FuncId)) {
    let mut operand = |operand: &mut Operand| {
        if let Operand::Const(Const::FuncAddr(func, _)) = operand {
            visit(func);
        }
    };
    for block in &mut func.blocks {
        for statement in &mut block.statements {
            statement.each_operand_mut(&mut operand);
        }
        if let Some(terminator) = &mut block.terminator {
            terminator.each_operand_mut(&mut operand);
        }
    }
    for statement in func.blocks.iter_mut().flat_map(|block| &mut block.statements) {
        if let Statement::Call { callee: Callee::Direct(callee), .. } = statement {
            visit(callee);
        }
    }
}
