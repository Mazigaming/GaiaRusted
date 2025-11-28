use crate::parser::{Item, Statement, Expression};
use crate::macros::MacroExpander;

pub struct MacroExpansionPass {
    expander: MacroExpander,
}

impl MacroExpansionPass {
    pub fn new(expander: MacroExpander) -> Self {
        MacroExpansionPass { expander }
    }

    pub fn expand_items(&mut self, items: &[Item]) -> Result<Vec<Item>, String> {
        let mut expanded = Vec::new();
        for item in items {
            let item = self.expand_item(item)?;
            expanded.extend(item);
        }
        Ok(expanded)
    }

    fn expand_item(&mut self, item: &Item) -> Result<Vec<Item>, String> {
        match item {
            Item::MacroDefinition { name, rules, .. } => {
                let macro_def = crate::macros::MacroDefinition {
                    name: name.clone(),
                    rules: rules.iter().map(|_r| {
                        crate::macros::MacroRule {
                            pattern: vec![],
                            body: vec![],
                        }
                    }).collect(),
                };
                self.expander.define(macro_def);
                Ok(vec![])
            }
            Item::Function {
                name,
                params,
                return_type,
                body,
                generics,
                attributes,
                is_unsafe,
                is_async,
                is_pub,
                where_clause,
                abi,
            } => {
                let expanded_body = self.expand_block(body)?;
                Ok(vec![Item::Function {
                    name: name.clone(),
                    params: params.clone(),
                    return_type: return_type.clone(),
                    body: expanded_body,
                    generics: generics.clone(),
                    attributes: attributes.clone(),
                    is_unsafe: *is_unsafe,
                    is_async: *is_async,
                    is_pub: *is_pub,
                    where_clause: where_clause.clone(),
                    abi: None,
                }])
            }
            Item::Struct {
                name,
                fields,
                generics,
                attributes,
                is_pub,
                where_clause,
            } => {
                Ok(vec![Item::Struct {
                    name: name.clone(),
                    fields: fields.clone(),
                    generics: generics.clone(),
                    attributes: attributes.clone(),
                    is_pub: *is_pub,
                    where_clause: where_clause.clone(),
                }])
            }
            _ => Ok(vec![item.clone()]),
        }
    }

    fn expand_block(&mut self, block: &crate::parser::Block) -> Result<crate::parser::Block, String> {
        let mut expanded_stmts = Vec::new();
        for stmt in &block.statements {
            let stmts = self.expand_statement(stmt)?;
            expanded_stmts.extend(stmts);
        }

        let expanded_expr = if let Some(expr) = &block.expression {
            Some(Box::new(self.expand_expression(expr)?))
        } else {
            None
        };

        Ok(crate::parser::Block {
            statements: expanded_stmts,
            expression: expanded_expr,
        })
    }

    fn expand_statement(&mut self, stmt: &Statement) -> Result<Vec<Statement>, String> {
        match stmt {
            Statement::Let {
                name,
                mutable,
                ty,
                initializer,
                attributes,
                pattern,
            } => {
                let expanded_init = self.expand_expression(initializer)?;
                Ok(vec![Statement::Let {
                    name: name.clone(),
                    mutable: *mutable,
                    ty: ty.clone(),
                    initializer: expanded_init,
                    attributes: attributes.clone(),
                    pattern: pattern.clone(),
                }])
            }
            Statement::Expression(expr) => {
                let expanded = self.expand_expression(expr)?;
                Ok(vec![Statement::Expression(expanded)])
            }
            Statement::MacroInvocation { name, args } => {
                self.expand_macro_invocation(name, args)
            }
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                let expanded_cond = self.expand_expression(condition)?;
                let expanded_then = self.expand_block(then_body)?;
                let expanded_else = if let Some(else_stmt) = else_body {
                    let expanded_else_stmts = self.expand_statement(else_stmt)?;
                    if expanded_else_stmts.len() == 1 {
                        Some(Box::new(expanded_else_stmts.into_iter().next().unwrap()))
                    } else {
                        None
                    }
                } else {
                    None
                };

                Ok(vec![Statement::If {
                    condition: Box::new(expanded_cond),
                    then_body: expanded_then,
                    else_body: expanded_else,
                }])
            }
            _ => Ok(vec![stmt.clone()]),
        }
    }

    fn expand_expression(&mut self, expr: &Expression) -> Result<Expression, String> {
        match expr {
            Expression::MacroInvocation { name, args } => {
                self.expand_macro_invocation(name, args)?
                    .into_iter()
                    .next()
                    .map(|stmt| match stmt {
                        Statement::Expression(e) => Ok(e),
                        _ => Err("Macro expansion resulted in non-expression".to_string()),
                    })
                    .ok_or_else(|| "Macro expansion resulted in no output".to_string())?
            }
            Expression::Binary { left, op, right } => {
                let expanded_left = self.expand_expression(left)?;
                let expanded_right = self.expand_expression(right)?;
                Ok(Expression::Binary {
                    left: Box::new(expanded_left),
                    op: *op,
                    right: Box::new(expanded_right),
                })
            }
            Expression::Unary { op, operand } => {
                let expanded_operand = self.expand_expression(operand)?;
                Ok(Expression::Unary {
                    op: *op,
                    operand: Box::new(expanded_operand),
                })
            }
            Expression::FunctionCall { name, args } => {
                let mut expanded_args = Vec::new();
                for arg in args {
                    expanded_args.push(self.expand_expression(arg)?);
                }
                
                // Check if this is a macro call (builtin macros like vec!, println!, etc.)
                match name.as_str() {
                    "vec" | "println" | "print" | "eprintln" | "format" | "assert" | 
                    "assert_eq" | "assert_ne" | "panic" | "dbg" => {
                        // This is a macro, expand it
                        let expanded_stmts = self.expand_macro_invocation(name, &expanded_args)?;
                        expanded_stmts
                            .into_iter()
                            .next()
                            .map(|stmt| match stmt {
                                Statement::Expression(e) => Ok(e),
                                _ => Err("Macro expansion resulted in non-expression".to_string()),
                            })
                            .ok_or_else(|| "Macro expansion resulted in no output".to_string())?
                    }
                    _ => {
                        // Regular function call
                        Ok(Expression::FunctionCall {
                            name: name.clone(),
                            args: expanded_args,
                        })
                    }
                }
            }
            _ => Ok(expr.clone()),
        }
    }

    fn expand_macro_invocation(
        &mut self,
        name: &str,
        args: &[Expression],
    ) -> Result<Vec<Statement>, String> {
        match name {
            "println" => self.expand_println(args),
            "print" => self.expand_print(args),
            "eprintln" => self.expand_eprintln(args),
            "vec" => self.expand_vec(args),
            "format" => self.expand_format(args),
            "assert" => self.expand_assert(args),
            "assert_eq" => self.expand_assert_eq(args),
            "assert_ne" => self.expand_assert_ne(args),
            "panic" => self.expand_panic(args),
            "dbg" => self.expand_dbg(args),
            _ => Err(format!("Unknown macro: {}", name)),
        }
    }

    fn expand_println(&mut self, args: &[Expression]) -> Result<Vec<Statement>, String> {
        if args.is_empty() {
            return Err("println! requires at least a format string".to_string());
        }

        let println_call = Expression::FunctionCall {
            name: "__builtin_println".to_string(),
            args: args.to_vec(),
        };

        Ok(vec![Statement::Expression(println_call)])
    }

    fn expand_print(&mut self, args: &[Expression]) -> Result<Vec<Statement>, String> {
        if args.is_empty() {
            return Err("print! requires at least a format string".to_string());
        }

        let print_call = Expression::FunctionCall {
            name: "__builtin_print".to_string(),
            args: args.to_vec(),
        };

        Ok(vec![Statement::Expression(print_call)])
    }

    fn expand_eprintln(&mut self, args: &[Expression]) -> Result<Vec<Statement>, String> {
        if args.is_empty() {
            return Err("eprintln! requires at least a format string".to_string());
        }

        let eprintln_call = Expression::FunctionCall {
            name: "__builtin_eprintln".to_string(),
            args: args.to_vec(),
        };

        Ok(vec![Statement::Expression(eprintln_call)])
    }

    fn expand_vec(&mut self, args: &[Expression]) -> Result<Vec<Statement>, String> {
        if args.is_empty() {
            let vec_call = Expression::FunctionCall {
                name: "__builtin_vec_empty".to_string(),
                args: vec![],
            };
            return Ok(vec![Statement::Expression(vec_call)]);
        }

        let vec_call = Expression::FunctionCall {
            name: "__builtin_vec".to_string(),
            args: args.to_vec(),
        };

        Ok(vec![Statement::Expression(vec_call)])
    }

    fn expand_format(&mut self, args: &[Expression]) -> Result<Vec<Statement>, String> {
        if args.is_empty() {
            return Err("format! requires at least a format string".to_string());
        }

        let format_call = Expression::FunctionCall {
            name: "__builtin_format".to_string(),
            args: args.to_vec(),
        };

        Ok(vec![Statement::Expression(format_call)])
    }

    fn expand_assert(&mut self, args: &[Expression]) -> Result<Vec<Statement>, String> {
        if args.is_empty() {
            return Err("assert! requires at least a condition".to_string());
        }

        let assert_call = Expression::FunctionCall {
            name: "__builtin_assert".to_string(),
            args: args.to_vec(),
        };

        Ok(vec![Statement::Expression(assert_call)])
    }

    fn expand_assert_eq(&mut self, args: &[Expression]) -> Result<Vec<Statement>, String> {
        if args.len() != 2 {
            return Err("assert_eq! requires exactly two arguments".to_string());
        }

        let assert_call = Expression::FunctionCall {
            name: "__builtin_assert_eq".to_string(),
            args: args.to_vec(),
        };

        Ok(vec![Statement::Expression(assert_call)])
    }

    fn expand_assert_ne(&mut self, args: &[Expression]) -> Result<Vec<Statement>, String> {
        if args.len() != 2 {
            return Err("assert_ne! requires exactly two arguments".to_string());
        }

        let assert_call = Expression::FunctionCall {
            name: "__builtin_assert_ne".to_string(),
            args: args.to_vec(),
        };

        Ok(vec![Statement::Expression(assert_call)])
    }

    fn expand_panic(&mut self, args: &[Expression]) -> Result<Vec<Statement>, String> {
        if args.is_empty() {
            return Err("panic! requires a message".to_string());
        }

        let panic_call = Expression::FunctionCall {
            name: "__builtin_panic".to_string(),
            args: args.to_vec(),
        };

        Ok(vec![Statement::Expression(panic_call)])
    }

    fn expand_dbg(&mut self, args: &[Expression]) -> Result<Vec<Statement>, String> {
        if args.is_empty() {
            return Err("dbg! requires an expression".to_string());
        }

        let dbg_call = Expression::FunctionCall {
            name: "__builtin_dbg".to_string(),
            args: args.to_vec(),
        };

        Ok(vec![Statement::Expression(dbg_call)])
    }
}
