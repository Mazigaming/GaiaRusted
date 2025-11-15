//! Lowers AST to MIR
//!
//! Converts the high-level AST into MIR (control flow graph) form.
//! This is the key step before code generation.

use crate::parser::ast::{self, Item, Parameter, Block, Expression, Type};
use crate::parser::ast::Statement as AstStatement;
use crate::mir::{Mir, MirFunction, BasicBlock, Terminator, Operand, Constant, Rvalue, Place};
use crate::mir::Statement as MirStatement;
use crate::lowering::HirType;
use std::collections::HashMap;

pub struct MirLowerer {
    functions: Vec<MirFunction>,
    current_func_params: HashMap<String, bool>, // name -> is_mutable
}

impl MirLowerer {
    pub fn new() -> Self {
        MirLowerer {
            functions: Vec::new(),
            current_func_params: HashMap::new(),
        }
    }

    pub fn lower_program(&mut self, items: &[Item]) -> Mir {
        for item in items {
            if let Item::Function {
                name,
                params,
                body,
                return_type,
                ..
            } = item
            {
                self.lower_function(name.clone(), params.clone(), body.clone(), return_type.clone());
            }
        }

        Mir {
            functions: self.functions.clone(),
        }
    }

    fn lower_function(
        &mut self,
        name: String,
        params: Vec<Parameter>,
        body: Block,
        _return_type: Option<Type>,
    ) {
        let mut blocks = vec![BasicBlock {
            statements: Vec::new(),
            terminator: Terminator::Unreachable,
        }];

        let mut state = LoweringState::new();
        let param_types: Vec<(String, HirType)> = params
            .iter()
            .map(|p| (p.name.clone(), HirType::Int64)) // Simplified: assume i64
            .collect();

        // Store parameter names for reference
        for param in &params {
            self.current_func_params
                .insert(param.name.clone(), param.mutable);
        }

        // Lower statements
        let mut current_block = 0;
        for stmt in &body.statements {
            self.lower_statement(stmt, &mut blocks, &mut current_block, &mut state);
        }

        // Handle final expression (return value)
        if let Some(expr) = &body.expression {
            let (value, _) = self.lower_expression(expr, &mut blocks, &mut current_block, &mut state);
            if let Some(last_block) = blocks.get_mut(current_block) {
                last_block.terminator = Terminator::Return(Some(value));
            }
        } else if let Some(last_block) = blocks.get_mut(current_block) {
            last_block.terminator = Terminator::Return(None);
        }

        let return_type = HirType::Int64; // Simplified
        self.functions.push(MirFunction {
            name,
            params: param_types,
            return_type,
            basic_blocks: blocks,
        });
    }

    fn lower_statement(
        &mut self,
        stmt: &AstStatement,
        blocks: &mut Vec<BasicBlock>,
        current_block: &mut usize,
        state: &mut LoweringState,
    ) {
        match stmt {
            AstStatement::Let {
                name,
                mutable: _,
                ty: _,
                initializer,
                ..
            } => {
                let (init_val, _) = self.lower_expression(initializer, blocks, current_block, state);
                if let Some(block) = blocks.get_mut(*current_block) {
                    block.statements.push(MirStatement {
                        place: Place::Local(name.clone()),
                        rvalue: Rvalue::Use(init_val),
                    });
                }
            }
            AstStatement::Expression(expr) => {
                let (_, _) = self.lower_expression(expr, blocks, current_block, state);
            }
            AstStatement::Return(expr_opt) => {
                let term = if let Some(expr) = expr_opt {
                    let (val, _) = self.lower_expression(expr, blocks, current_block, state);
                    Terminator::Return(Some(val))
                } else {
                    Terminator::Return(None)
                };
                if let Some(block) = blocks.get_mut(*current_block) {
                    block.terminator = term;
                }
            }
            AstStatement::If {
                condition,
                then_body,
                else_body,
            } => {
                let (cond_val, _) = self.lower_expression(condition, blocks, current_block, state);
                let then_block_id = blocks.len();
                blocks.push(BasicBlock {
                    statements: Vec::new(),
                    terminator: Terminator::Unreachable,
                });

                let else_block_id = blocks.len();
                blocks.push(BasicBlock {
                    statements: Vec::new(),
                    terminator: Terminator::Unreachable,
                });

                let merge_block_id = blocks.len();
                blocks.push(BasicBlock {
                    statements: Vec::new(),
                    terminator: Terminator::Unreachable,
                });

                if let Some(block) = blocks.get_mut(*current_block) {
                    block.terminator = Terminator::If(cond_val, then_block_id, else_block_id);
                }

                // Lower then branch
                *current_block = then_block_id;
                for stmt in &then_body.statements {
                    self.lower_statement(stmt, blocks, current_block, state);
                }
                if let Some(expr) = &then_body.expression {
                    let (_, _) = self.lower_expression(expr, blocks, current_block, state);
                }
                if let Some(block) = blocks.get_mut(*current_block) {
                    if matches!(block.terminator, Terminator::Unreachable) {
                        block.terminator = Terminator::Goto(merge_block_id);
                    }
                }

                // Lower else branch
                *current_block = else_block_id;
                if let Some(else_stmt) = else_body {
                    if let AstStatement::If {
                        condition: cond,
                        then_body: then_b,
                        else_body: else_b,
                    } = &**else_stmt
                    {
                        self.lower_statement(
                            &AstStatement::If {
                                condition: cond.clone(),
                                then_body: then_b.clone(),
                                else_body: else_b.clone(),
                            },
                            blocks,
                            current_block,
                            state,
                        );
                    } else {
                        self.lower_statement(else_stmt, blocks, current_block, state);
                    }
                }
                if let Some(block) = blocks.get_mut(*current_block) {
                    if matches!(block.terminator, Terminator::Unreachable) {
                        block.terminator = Terminator::Goto(merge_block_id);
                    }
                }

                *current_block = merge_block_id;
            }
            _ => {}
        }
    }

    fn lower_expression(
        &mut self,
        expr: &Expression,
        blocks: &mut Vec<BasicBlock>,
        current_block: &mut usize,
        state: &mut LoweringState,
    ) -> (Operand, String) {
        // Returns (operand, place_name)
        match expr {
            Expression::Integer(n) => (Operand::Constant(Constant::Integer(*n)), String::new()),
            Expression::Float(f) => (Operand::Constant(Constant::Float(*f)), String::new()),
            Expression::Bool(b) => (Operand::Constant(Constant::Bool(*b)), String::new()),
            Expression::String(s) => (Operand::Constant(Constant::String(s.clone())), String::new()),
            Expression::Variable(name) => (Operand::Copy(Place::Local(name.clone())), name.clone()),
            Expression::Binary { left, op, right } => {
                let (left_val, _) = self.lower_expression(left, blocks, current_block, state);
                let (right_val, _) = self.lower_expression(right, blocks, current_block, state);

                let temp_name = state.gen_temp();
                let mir_op = match op {
                    ast::BinaryOp::Add => crate::lowering::BinaryOp::Add,
                    ast::BinaryOp::Subtract => crate::lowering::BinaryOp::Subtract,
                    ast::BinaryOp::Multiply => crate::lowering::BinaryOp::Multiply,
                    ast::BinaryOp::Divide => crate::lowering::BinaryOp::Divide,
                    ast::BinaryOp::Modulo => crate::lowering::BinaryOp::Modulo,
                    ast::BinaryOp::Equal => crate::lowering::BinaryOp::Equal,
                    ast::BinaryOp::NotEqual => crate::lowering::BinaryOp::NotEqual,
                    ast::BinaryOp::Less => crate::lowering::BinaryOp::Less,
                    ast::BinaryOp::LessEq => crate::lowering::BinaryOp::LessEqual,
                    ast::BinaryOp::Greater => crate::lowering::BinaryOp::Greater,
                    ast::BinaryOp::GreaterEq => crate::lowering::BinaryOp::GreaterEqual,
                    ast::BinaryOp::And => crate::lowering::BinaryOp::And,
                    ast::BinaryOp::Or => crate::lowering::BinaryOp::Or,
                    ast::BinaryOp::BitwiseAnd => crate::lowering::BinaryOp::BitwiseAnd,
                    ast::BinaryOp::BitwiseOr => crate::lowering::BinaryOp::BitwiseOr,
                    ast::BinaryOp::BitwiseXor => crate::lowering::BinaryOp::BitwiseXor,
                    ast::BinaryOp::LeftShift => crate::lowering::BinaryOp::LeftShift,
                    ast::BinaryOp::RightShift => crate::lowering::BinaryOp::RightShift,
                };

                if let Some(block) = blocks.get_mut(*current_block) {
                    block.statements.push(MirStatement {
                        place: Place::Local(temp_name.clone()),
                        rvalue: Rvalue::BinaryOp(mir_op, left_val, right_val),
                    });
                }
                (Operand::Copy(Place::Local(temp_name.clone())), temp_name)
            }
            Expression::FunctionCall { name, args } => {
                let mut arg_vals = Vec::new();
                for arg in args {
                    let (val, _) = self.lower_expression(arg, blocks, current_block, state);
                    arg_vals.push(val);
                }

                let temp_name = state.gen_temp();
                if let Some(block) = blocks.get_mut(*current_block) {
                    block.statements.push(MirStatement {
                        place: Place::Local(temp_name.clone()),
                        rvalue: Rvalue::Call(name.clone(), arg_vals),
                    });
                }
                (Operand::Copy(Place::Local(temp_name.clone())), temp_name)
            }
            Expression::Block(block) => {
                let mut result = Operand::Constant(Constant::Unit);
                for stmt in &block.statements {
                    self.lower_statement(stmt, blocks, current_block, state);
                }
                if let Some(expr) = &block.expression {
                    result = self.lower_expression(expr, blocks, current_block, state).0;
                }
                (result, String::new())
            }
            _ => (Operand::Constant(Constant::Unit), String::new()),
        }
    }
}

struct LoweringState {
    temp_counter: usize,
}

impl LoweringState {
    fn new() -> Self {
        LoweringState { temp_counter: 0 }
    }

    fn gen_temp(&mut self) -> String {
        let name = format!("_t{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mir_lowerer_creation() {
        let lowerer = MirLowerer::new();
        assert_eq!(lowerer.functions.len(), 0);
    }
}
