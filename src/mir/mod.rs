//! # Phase 6 & 7: MIR LOWERING & OPTIMIZATION
//!
//! Converts HIR to MIR (Mid-Level IR) - a control flow graph representation.
//! Then optimizes the MIR.
//!
//! ## What we do:
//! - Convert HIR to MIR (basic blocks, control flow)
//! - Build control flow graph
//! - SSA-like form (each place assigned once)
//! - Terminator-based control flow
//!
//! ## MIR Structure:
//! - **Basic Block**: Sequence of statements ending with terminator
//! - **Statement**: Assignment or other effects
//! - **Terminator**: Control flow (goto, if, return, etc.)
//! - **Place**: Location of data (variable, field, index)
//! - **Operand**: Value source (move, copy, constant)

use crate::lowering::{HirExpression, HirItem, HirStatement, HirType, BinaryOp, UnaryOp};
use std::fmt;

/// MIR error
#[derive(Debug, Clone)]
pub struct MirError {
    pub message: String,
}

impl fmt::Display for MirError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

type MirResult<T> = Result<T, MirError>;

/// Represents a place (location of data)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Place {
    /// Local variable
    Local(String),
    /// Field of a struct
    Field(Box<Place>, String),
    /// Array index
    Index(Box<Place>, usize),
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Place::Local(name) => write!(f, "{}", name),
            Place::Field(place, field) => write!(f, "{}.{}", place, field),
            Place::Index(place, idx) => write!(f, "{}[{}]", place, idx),
        }
    }
}

/// Represents where a value comes from (operand)
#[derive(Debug, Clone)]
pub enum Operand {
    /// Move the value
    Move(Place),
    /// Copy the value
    Copy(Place),
    /// Constant value
    Constant(Constant),
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Operand::Move(place) => write!(f, "move {}", place),
            Operand::Copy(place) => write!(f, "copy {}", place),
            Operand::Constant(c) => write!(f, "{}", c),
        }
    }
}

/// Constant values
#[derive(Debug, Clone)]
pub enum Constant {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Unit,
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Constant::Integer(n) => write!(f, "{}", n),
            Constant::Float(n) => write!(f, "{}", n),
            Constant::String(s) => write!(f, "\"{}\"", s),
            Constant::Bool(b) => write!(f, "{}", b),
            Constant::Unit => write!(f, "()"),
        }
    }
}

/// Right-hand side of an assignment
#[derive(Debug, Clone)]
pub enum Rvalue {
    /// Use an operand
    Use(Operand),
    /// Binary operation
    BinaryOp(BinaryOp, Operand, Operand),
    /// Unary operation
    UnaryOp(UnaryOp, Operand),
    /// Function call
    Call(String, Vec<Operand>),
    /// Struct construction
    Aggregate(String, Vec<Operand>),
    /// Array construction
    Array(Vec<Operand>),
    /// Reference creation
    Ref(Place),
    /// Dereference
    Deref(Place),
    /// Field access
    Field(Place, String),
    /// Index access
    Index(Place, usize),
}

impl fmt::Display for Rvalue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Rvalue::Use(op) => write!(f, "{}", op),
            Rvalue::BinaryOp(op, l, r) => write!(f, "{:?} {:?} {:?}", l, op, r),
            Rvalue::UnaryOp(op, op_val) => write!(f, "{:?} {:?}", op, op_val),
            Rvalue::Call(name, args) => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Rvalue::Aggregate(name, fields) => {
                write!(f, "{} {{ ", name)?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", field)?;
                }
                write!(f, " }}")
            }
            Rvalue::Array(elems) => {
                write!(f, "[")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            Rvalue::Ref(place) => write!(f, "&{}", place),
            Rvalue::Deref(place) => write!(f, "*{}", place),
            Rvalue::Field(place, field) => write!(f, "{}.{}", place, field),
            Rvalue::Index(place, idx) => write!(f, "{}[{}]", place, idx),
        }
    }
}

/// Statement in a basic block
#[derive(Debug, Clone)]
pub struct Statement {
    pub place: Place,
    pub rvalue: Rvalue,
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} = {}", self.place, self.rvalue)
    }
}

/// Control flow terminator
#[derive(Debug, Clone)]
pub enum Terminator {
    /// Unconditional goto
    Goto(usize),
    /// Conditional branch
    If(Operand, usize, usize), // condition, then_block, else_block
    /// Return value
    Return(Option<Operand>),
    /// Unreachable code
    Unreachable,
}

impl fmt::Display for Terminator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Terminator::Goto(bb) => write!(f, "goto bb{}", bb),
            Terminator::If(cond, then_bb, else_bb) => {
                write!(f, "if {} {{ goto bb{} }} else {{ goto bb{} }}", cond, then_bb, else_bb)
            }
            Terminator::Return(Some(op)) => write!(f, "return {}", op),
            Terminator::Return(None) => write!(f, "return"),
            Terminator::Unreachable => write!(f, "unreachable"),
        }
    }
}

/// A basic block
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

impl fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for stmt in &self.statements {
            writeln!(f, "  {}", stmt)?;
        }
        write!(f, "  {}", self.terminator)
    }
}

/// A function in MIR form (control flow graph)
#[derive(Debug, Clone)]
pub struct MirFunction {
    pub name: String,
    pub params: Vec<(String, HirType)>,
    pub return_type: HirType,
    pub basic_blocks: Vec<BasicBlock>,
}

impl fmt::Display for MirFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "fn {}(...) -> {}", self.name, self.return_type)?;
        for (i, bb) in self.basic_blocks.iter().enumerate() {
            writeln!(f, "bb{}:", i)?;
            write!(f, "{}", bb)?;
        }
        Ok(())
    }
}

/// MIR for the entire program
#[derive(Debug, Clone)]
pub struct Mir {
    pub functions: Vec<MirFunction>,
}

/// MIR builder
pub struct MirBuilder {
    current_block: usize,
    blocks: Vec<BasicBlock>,
    next_var: usize,
}

impl MirBuilder {
    /// Create a new MIR builder
    pub fn new() -> Self {
        MirBuilder {
            current_block: 0,
            blocks: vec![BasicBlock {
                statements: Vec::new(),
                terminator: Terminator::Unreachable,
            }],
            next_var: 0,
        }
    }

    /// Generate a new temporary variable name
    pub fn gen_temp(&mut self) -> String {
        let name = format!("_t{}", self.next_var);
        self.next_var += 1;
        name
    }

    /// Add a statement to the current block
    pub fn add_statement(&mut self, place: Place, rvalue: Rvalue) {
        if let Some(block) = self.blocks.get_mut(self.current_block) {
            block.statements.push(Statement { place, rvalue });
        }
    }

    /// Set the terminator for the current block
    pub fn set_terminator(&mut self, terminator: Terminator) {
        if let Some(block) = self.blocks.get_mut(self.current_block) {
            block.terminator = terminator;
        }
    }

    /// Create a new basic block and return its index
    pub fn create_block(&mut self) -> usize {
        let idx = self.blocks.len();
        self.blocks.push(BasicBlock {
            statements: Vec::new(),
            terminator: Terminator::Unreachable,
        });
        idx
    }

    /// Switch to a different basic block
    pub fn switch_block(&mut self, block_idx: usize) {
        self.current_block = block_idx;
    }

    /// Get all basic blocks
    pub fn finish(self) -> Vec<BasicBlock> {
        self.blocks
    }
}

/// MIR lowerer: converts HIR to MIR
pub struct MirLowerer {
    builder: MirBuilder,
    closure_counter: usize,
    generated_functions: Vec<MirFunction>,
    closure_vars: std::collections::HashMap<String, (String, Vec<(String, HirType)>)>, // Maps variable name -> (function name, captures)
}

impl MirLowerer {
    /// Create a new MIR lowerer
    pub fn new() -> Self {
        MirLowerer {
            builder: MirBuilder::new(),
            closure_counter: 0,
            generated_functions: Vec::new(),
            closure_vars: std::collections::HashMap::new(),
        }
    }

    /// Generate a unique closure function name
    fn gen_closure_name(&mut self) -> String {
        let name = format!("__closure_{}", self.closure_counter);
        self.closure_counter += 1;
        name
    }

    /// Generate a closure function from a closure expression
    fn generate_closure_function(
        &mut self,
        params: &[(String, HirType)],
        body: &[HirStatement],
        return_type: &HirType,
        captures: &[(String, HirType)],
    ) -> MirResult<String> {
        let func_name = self.gen_closure_name();
        let mut builder = MirBuilder::new();

        for stmt in body {
            self.lower_statement_in_builder(&mut builder, stmt)?;
        }

        if matches!(builder.blocks[builder.current_block].terminator, Terminator::Unreachable) {
            builder.set_terminator(Terminator::Return(None));
        }

        let mut all_params = captures.to_vec();
        all_params.extend_from_slice(params);

        let func = MirFunction {
            name: func_name.clone(),
            params: all_params,
            return_type: return_type.clone(),
            basic_blocks: builder.finish(),
        };

        self.generated_functions.push(func);
        Ok(func_name)
    }

    /// Lower all items to MIR
    pub fn lower_items(&mut self, items: &[HirItem]) -> MirResult<Mir> {
        let mut functions = Vec::new();

        for item in items {
            match item {
                HirItem::Function {
                    name,
                    params,
                    return_type,
                    body,
                } => {
                    let mut mir_builder = MirBuilder::new();

                    // Lower function body
                    for stmt in body {
                        self.lower_statement_in_builder(&mut mir_builder, stmt)?;
                    }

                    // Ensure proper terminator
                    if matches!(mir_builder.blocks[mir_builder.current_block].terminator, Terminator::Unreachable) {
                        mir_builder.set_terminator(Terminator::Return(None));
                    }

                    let func = MirFunction {
                        name: name.clone(),
                        params: params.clone(),
                        return_type: return_type.clone().unwrap_or(HirType::Unknown),
                        basic_blocks: mir_builder.finish(),
                    };
                    functions.push(func);
                }
                HirItem::Struct { .. } => {
                }
                HirItem::AssociatedType { .. } => {
                }
                HirItem::Use { .. } => {
                }
            }
        }

        // Add any generated closure functions
        functions.extend(self.generated_functions.drain(..));

        Ok(Mir { functions })
    }

    /// Lower a statement
    fn lower_statement_in_builder(&mut self, builder: &mut MirBuilder, stmt: &HirStatement) -> MirResult<()> {
        match stmt {
            HirStatement::Let { name, init, .. } => {
                if let HirExpression::Closure { params, body, return_type, is_move: _, captures } = init {
                    // Generate a closure function
                    let func_name = self.generate_closure_function(params, body, return_type, captures)?;
                    self.closure_vars.insert(name.clone(), (func_name, captures.clone()));
                    let place = Place::Local(name.clone());
                    builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
                } else {
                    let place = Place::Local(name.clone());
                    self.lower_expression_to_place(builder, init, place)?;
                }
            }
            HirStatement::Expression(expr) => {
                let temp = builder.gen_temp();
                let place = Place::Local(temp);
                self.lower_expression_to_place(builder, expr, place)?;
            }
            HirStatement::Return(Some(expr)) => {
                let temp = builder.gen_temp();
                let place = Place::Local(temp);
                self.lower_expression_to_place(builder, expr, place.clone())?;
                builder.set_terminator(Terminator::Return(Some(Operand::Copy(place))));
            }
            HirStatement::Return(None) => {
                builder.set_terminator(Terminator::Return(None));
            }
            HirStatement::Break | HirStatement::Continue => {
                // Simplified: treat as unreachable for now
                builder.set_terminator(Terminator::Unreachable);
            }

            HirStatement::For { var, iter, body } => {
                // Lower for loop into proper control flow graph
                // Desugar: for i in 0..n { body }
                // Into: let mut i = 0; while i < n { body; i = i + 1; }
                
                match &**iter {
                    HirExpression::Range { start, end, inclusive } => {
                        // Simple range iteration - desugar into explicit counter loop
                        let loop_var = var.clone();
                        let loop_var_place = Place::Local(loop_var.clone());
                        
                        // Initialize loop variable
                        if let Some(s) = start {
                            self.lower_expression_to_place(builder, s, loop_var_place.clone())?;
                        } else {
                            // Default start to 0
                            builder.add_statement(
                                loop_var_place.clone(),
                                Rvalue::Use(Operand::Constant(Constant::Integer(0)))
                            );
                        }
                        
                        // Create loop blocks
                        let loop_start = builder.current_block;
                        let loop_body = builder.create_block();
                        let loop_end = builder.create_block();
                        
                        // Loop condition: i < end (or i <= end if inclusive)
                        if let Some(e) = end {
                            let end_temp = builder.gen_temp();
                            self.lower_expression_to_place(builder, e, Place::Local(end_temp.clone()))?;
                            
                            let cond_op = Rvalue::BinaryOp(
                                if *inclusive { BinaryOp::LessEqual } else { BinaryOp::Less },
                                Operand::Copy(loop_var_place.clone()),
                                Operand::Copy(Place::Local(end_temp))
                            );
                            let cond_temp = builder.gen_temp();
                            builder.add_statement(Place::Local(cond_temp.clone()), cond_op);
                            
                            builder.set_terminator(Terminator::If(
                                Operand::Copy(Place::Local(cond_temp)),
                                loop_body,
                                loop_end,
                            ));
                        }
                        
                        // Loop body
                        builder.current_block = loop_body;
                        for stmt in body {
                            self.lower_statement_in_builder(builder, stmt)?;
                        }
                        
                        // Increment counter: i = i + 1
                        let inc_expr = Rvalue::BinaryOp(
                            BinaryOp::Add,
                            Operand::Copy(loop_var_place.clone()),
                            Operand::Constant(Constant::Integer(1))
                        );
                        builder.add_statement(loop_var_place, inc_expr);
                        builder.set_terminator(Terminator::Goto(loop_start));
                        
                        // Continue after loop
                        builder.current_block = loop_end;
                    }
                    _ => {
                        // Fallback: non-range iterator - for now just process body sequentially
                        // TODO: Implement proper iterator protocol
                        for stmt in body {
                            self.lower_statement_in_builder(builder, stmt)?;
                        }
                    }
                }
            }

            HirStatement::While {
                condition,
                body,
            } => {
                // Lower while loop into proper control flow graph
                // while cond { body } =>
                // loop_start:
                //   if cond { goto body_block } else { goto loop_end }
                // body_block:
                //   [body]
                //   goto loop_start
                // loop_end:
                
                let loop_start = builder.current_block;
                let loop_body = builder.create_block();
                let loop_end = builder.create_block();
                
                // Loop condition check
                let cond_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, condition, Place::Local(cond_temp.clone()))?;
                builder.set_terminator(Terminator::If(
                    Operand::Copy(Place::Local(cond_temp)),
                    loop_body,
                    loop_end,
                ));
                
                // Loop body
                builder.current_block = loop_body;
                for stmt in body {
                    self.lower_statement_in_builder(builder, stmt)?;
                }
                builder.set_terminator(Terminator::Goto(loop_start));
                
                // Continue after loop
                builder.current_block = loop_end;
            }

            HirStatement::If {
                condition,
                then_body,
                else_body,
            } => {
                // Lower if statement into proper control flow graph
                // if cond { then_body } else { else_body } =>
                // if_start:
                //   if cond { goto then_block } else { goto else_block }
                // then_block:
                //   [then_body]
                //   goto merge_block
                // else_block:
                //   [else_body]
                //   goto merge_block
                // merge_block:
                
                let _if_block = builder.current_block;
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();
                
                // Condition check
                let cond_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, condition, Place::Local(cond_temp.clone()))?;
                builder.set_terminator(Terminator::If(
                    Operand::Copy(Place::Local(cond_temp)),
                    then_block,
                    else_block,
                ));
                
                // Then branch
                builder.current_block = then_block;
                for stmt in then_body {
                    self.lower_statement_in_builder(builder, stmt)?;
                }
                builder.set_terminator(Terminator::Goto(merge_block));
                
                // Else branch
                builder.current_block = else_block;
                if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        self.lower_statement_in_builder(builder, stmt)?;
                    }
                }
                builder.set_terminator(Terminator::Goto(merge_block));
                
                // Continue after if
                builder.current_block = merge_block;
            }

            HirStatement::UnsafeBlock(stmts) => {
                // Unsafe blocks are treated as regular blocks in MIR
                // The safety guarantees are already checked in the borrowchecker
                for stmt in stmts {
                    self.lower_statement_in_builder(builder, stmt)?;
                }
            }

            HirStatement::Item(_) => {
                // Nested items are not lowered to MIR at this level
                // They are processed separately during compilation
            }
        }
        Ok(())
    }

    /// Lower an expression, storing result in place
    fn lower_expression_to_place(&mut self, builder: &mut MirBuilder, expr: &HirExpression, place: Place) -> MirResult<()> {
        match expr {
            HirExpression::Integer(n) => {
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Integer(*n))));
            }
            HirExpression::Float(n) => {
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Float(*n))));
            }
            HirExpression::String(s) => {
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::String(s.clone()))));
            }
            HirExpression::Bool(b) => {
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Bool(*b))));
            }
            HirExpression::Variable(name) => {
                builder.add_statement(place, Rvalue::Use(Operand::Copy(Place::Local(name.clone()))));
            }
            HirExpression::BinaryOp { op, left, right } => {
                let left_temp = builder.gen_temp();
                let right_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, left, Place::Local(left_temp.clone()))?;
                self.lower_expression_to_place(builder, right, Place::Local(right_temp.clone()))?;
                
                let rvalue = Rvalue::BinaryOp(*op, Operand::Copy(Place::Local(left_temp)), Operand::Copy(Place::Local(right_temp)));
                builder.add_statement(place, rvalue);
            }
            HirExpression::UnaryOp { op, operand } => {
                let op_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, operand, Place::Local(op_temp.clone()))?;
                
                let rvalue = Rvalue::UnaryOp(*op, Operand::Copy(Place::Local(op_temp)));
                builder.add_statement(place, rvalue);
            }
            HirExpression::Call { func, args } => {
                let mut func_name = match &**func {
                    HirExpression::Variable(name) => name.clone(),
                    _ => return Err(MirError { message: "Indirect calls not supported".to_string() }),
                };

                // Check if this is a call to a closure variable
                let mut mir_args = Vec::new();
                if let Some((actual_func_name, captures)) = self.closure_vars.get(&func_name).cloned() {
                    func_name = actual_func_name;
                    
                    for (capture_name, _) in captures {
                        let temp = builder.gen_temp();
                        let capture_place = Place::Local(capture_name);
                        builder.add_statement(Place::Local(temp.clone()), Rvalue::Use(Operand::Copy(capture_place)));
                        mir_args.push(Operand::Copy(Place::Local(temp)));
                    }
                }
                
                for arg in args {
                    let temp = builder.gen_temp();
                    self.lower_expression_to_place(builder, arg, Place::Local(temp.clone()))?;
                    mir_args.push(Operand::Copy(Place::Local(temp)));
                }

                builder.add_statement(place, Rvalue::Call(func_name, mir_args));
            }
            HirExpression::Range { start: _, end: _, inclusive: _ } => {
                // Ranges are simplified to unit in MIR
                // A full implementation would create range objects
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
            }
            HirExpression::Tuple(elements) => {
                // Tuples are simplified to unit in MIR
                // A full implementation would create tuple structures
                for _elem in elements {
                    // Could evaluate each element for side effects
                }
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
            }
            HirExpression::Assign { target, value } => {
                let val_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, value, Place::Local(val_temp.clone()))?;
                
                // For simple variable assignments
                if let HirExpression::Variable(name) = &**target {
                    builder.add_statement(Place::Local(name.clone()), Rvalue::Use(Operand::Copy(Place::Local(val_temp))));
                    builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
                } else {
                    return Err(MirError { message: "Complex assignment targets not yet supported".to_string() });
                }
            }
            HirExpression::If { condition, then_body, else_body } => {
                // If expressions in MIR become branches
                let cond_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, condition, Place::Local(cond_temp.clone()))?;
                
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();
                
                let curr = builder.current_block;
                builder.blocks[curr].terminator = Terminator::If(
                    Operand::Copy(Place::Local(cond_temp)),
                    then_block,
                    else_block,
                );
                
                // Lower then body
                builder.current_block = then_block;
                for stmt in then_body {
                    self.lower_statement_in_builder(builder, stmt)?;
                }
                builder.blocks[then_block].terminator = Terminator::Goto(merge_block);
                
                // Lower else body
                builder.current_block = else_block;
                if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        self.lower_statement_in_builder(builder, stmt)?;
                    }
                }
                builder.blocks[else_block].terminator = Terminator::Goto(merge_block);
                
                // Continue at merge block
                builder.current_block = merge_block;
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
            }
            HirExpression::While { condition, body } => {
                let loop_start = builder.current_block;
                let loop_body = builder.create_block();
                let loop_end = builder.create_block();
                
                // Loop condition check
                let cond_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, condition, Place::Local(cond_temp.clone()))?;
                builder.blocks[loop_start].terminator = Terminator::If(
                    Operand::Copy(Place::Local(cond_temp)),
                    loop_body,
                    loop_end,
                );
                
                // Loop body
                builder.current_block = loop_body;
                for stmt in body {
                    self.lower_statement_in_builder(builder, stmt)?;
                }
                builder.blocks[loop_body].terminator = Terminator::Goto(loop_start);
                
                // After loop
                builder.current_block = loop_end;
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
            }
            HirExpression::FieldAccess { object, field } => {
                let obj_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, object, Place::Local(obj_temp.clone()))?;
                builder.add_statement(place, Rvalue::Use(Operand::Copy(Place::Field(
                    Box::new(Place::Local(obj_temp)),
                    field.clone(),
                ))));
            }
            HirExpression::TupleAccess { object, index: _ } => {
                let obj_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, object, Place::Local(obj_temp.clone()))?;
                builder.add_statement(place, Rvalue::Use(Operand::Copy(Place::Local(obj_temp))));
            }
            HirExpression::Index { array, index } => {
                let arr_temp = builder.gen_temp();
                let idx_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, array, Place::Local(arr_temp.clone()))?;
                self.lower_expression_to_place(builder, index, Place::Local(idx_temp.clone()))?;
                
                // For now, just treat indexed access as unit
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
            }
            HirExpression::StructLiteral { name: _, fields: _ } => {
                // Struct literals become unit in simplified MIR
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
            }
            HirExpression::ArrayLiteral(elements) => {
                // Array literals become unit in simplified MIR
                for _elem in elements {
                    // Could evaluate each element for side effects
                }
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
            }
            HirExpression::Block(statements, expr) => {
                // Execute all statements
                for stmt in statements {
                    self.lower_statement_in_builder(builder, stmt)?;
                }
                // Execute final expression if present
                if let Some(final_expr) = expr {
                    self.lower_expression_to_place(builder, final_expr, place)?;
                } else {
                    builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
                }
            }
            HirExpression::Match { scrutinee: _, arms: _ } => {
                // Match expressions are complex - for now, treat as unit
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
            }
            HirExpression::Closure { params: _, body: _, return_type: _, is_move: _, captures: _ } => {
                // Closures are treated as unit in simplified MIR
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
            }

            HirExpression::Try { value } => {
                let temp_name = builder.gen_temp();
                let temp = Place::Local(temp_name.clone());
                self.lower_expression_to_place(builder, value, temp.clone())?;
                
                let ok_block = builder.create_block();
                let err_block = builder.create_block();
                let continue_block = builder.create_block();
                
                let is_ok_temp_name = builder.gen_temp();
                let is_ok_temp = Place::Local(is_ok_temp_name);
                builder.add_statement(is_ok_temp.clone(), Rvalue::Call(
                    "__builtin_is_ok".to_string(),
                    vec![Operand::Copy(temp.clone())],
                ));
                
                builder.set_terminator(Terminator::If(
                    Operand::Copy(is_ok_temp),
                    ok_block,
                    err_block,
                ));
                
                builder.switch_block(ok_block);
                let extract_temp_name = builder.gen_temp();
                let extract_temp = Place::Local(extract_temp_name);
                builder.add_statement(extract_temp.clone(), Rvalue::Call(
                    "__builtin_unwrap".to_string(),
                    vec![Operand::Copy(temp.clone())],
                ));
                builder.add_statement(place.clone(), Rvalue::Use(Operand::Copy(extract_temp)));
                builder.set_terminator(Terminator::Goto(continue_block));
                
                builder.switch_block(err_block);
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Integer(1))));
                builder.set_terminator(Terminator::Return(Some(Operand::Constant(Constant::Integer(1)))));
                
                builder.switch_block(continue_block);
            }
        }
        Ok(())
    }
}

use std::collections::{HashMap, HashSet};

/// Simple MIR optimizer with multiple passes based on optimization level
pub struct MirOptimizer;

impl MirOptimizer {
    /// Optimize MIR with passes based on optimization level (1-3)
    pub fn optimize(mir: &mut Mir, opt_level: u32) -> MirResult<()> {
        if opt_level == 0 {
            return Ok(()); // No optimizations
        }

        for func in &mut mir.functions {
            // O1+ passes
            Self::constant_fold(&mut func.basic_blocks)?;
            Self::dead_code_elimination(&mut func.basic_blocks)?;

            // O2+ passes
            if opt_level >= 2 {
                Self::simplify_control_flow(&mut func.basic_blocks)?;
            }

            // O3 passes
            if opt_level >= 3 {
                Self::copy_propagation(&mut func.basic_blocks)?;
            }
        }
        Ok(())
    }

    /// O1 Pass: Constant Folding - Evaluate constant expressions at compile time
    fn constant_fold(blocks: &mut [BasicBlock]) -> MirResult<()> {
        for block in blocks {
            for stmt in &mut block.statements {
                if let Rvalue::BinaryOp(op, left, right) = &stmt.rvalue {
                    // Only fold if both operands are constants
                    if let (Operand::Constant(l), Operand::Constant(r)) = (left, right) {
                        if let Some(result) = Self::fold_binary_op(op, l, r) {
                            stmt.rvalue = Rvalue::Use(Operand::Constant(result));
                        }
                    }
                } else if let Rvalue::UnaryOp(op, operand) = &stmt.rvalue {
                    if let Operand::Constant(val) = operand {
                        if let Some(result) = Self::fold_unary_op(op, val) {
                            stmt.rvalue = Rvalue::Use(Operand::Constant(result));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Fold binary operations with constant operands
    fn fold_binary_op(op: &BinaryOp, left: &Constant, right: &Constant) -> Option<Constant> {
        match (left, right) {
            (Constant::Integer(l), Constant::Integer(r)) => {
                Some(match op {
                    BinaryOp::Add => Constant::Integer(l + r),
                    BinaryOp::Subtract => Constant::Integer(l - r),
                    BinaryOp::Multiply => Constant::Integer(l * r),
                    BinaryOp::Divide => {
                        if *r != 0 {
                            Constant::Integer(l / r)
                        } else {
                            return None;
                        }
                    }
                    BinaryOp::Modulo => {
                        if *r != 0 {
                            Constant::Integer(l % r)
                        } else {
                            return None;
                        }
                    }
                    BinaryOp::Equal => Constant::Bool(l == r),
                    BinaryOp::NotEqual => Constant::Bool(l != r),
                    BinaryOp::Less => Constant::Bool(l < r),
                    BinaryOp::LessEqual => Constant::Bool(l <= r),
                    BinaryOp::Greater => Constant::Bool(l > r),
                    BinaryOp::GreaterEqual => Constant::Bool(l >= r),
                    BinaryOp::And => Constant::Bool(*l != 0 && *r != 0),
                    BinaryOp::Or => Constant::Bool(*l != 0 || *r != 0),
                    BinaryOp::BitwiseXor => Constant::Integer(l ^ r),
                    BinaryOp::BitwiseAnd => Constant::Integer(l & r),
                    BinaryOp::BitwiseOr => Constant::Integer(l | r),
                    BinaryOp::LeftShift => Constant::Integer(l << r),
                    BinaryOp::RightShift => Constant::Integer(l >> r),
                })
            }
            (Constant::Float(l), Constant::Float(r)) => {
                Some(match op {
                    BinaryOp::Add => Constant::Float(l + r),
                    BinaryOp::Subtract => Constant::Float(l - r),
                    BinaryOp::Multiply => Constant::Float(l * r),
                    BinaryOp::Divide => {
                        if *r != 0.0 {
                            Constant::Float(l / r)
                        } else {
                            return None;
                        }
                    }
                    BinaryOp::Equal => Constant::Bool((l - r).abs() < f64::EPSILON),
                    BinaryOp::NotEqual => Constant::Bool((l - r).abs() >= f64::EPSILON),
                    BinaryOp::Less => Constant::Bool(l < r),
                    BinaryOp::LessEqual => Constant::Bool(l <= r),
                    BinaryOp::Greater => Constant::Bool(l > r),
                    BinaryOp::GreaterEqual => Constant::Bool(l >= r),
                    _ => return None, // Other ops don't apply to floats
                })
            }
            (Constant::String(l), Constant::String(r)) => {
                match op {
                    BinaryOp::Add => Some(Constant::String(format!("{}{}", l, r))),
                    BinaryOp::Equal => Some(Constant::Bool(l == r)),
                    BinaryOp::NotEqual => Some(Constant::Bool(l != r)),
                    _ => None,
                }
            }
            (Constant::Bool(l), Constant::Bool(r)) => {
                match op {
                    BinaryOp::And => Some(Constant::Bool(*l && *r)),
                    BinaryOp::Or => Some(Constant::Bool(*l || *r)),
                    BinaryOp::Equal => Some(Constant::Bool(l == r)),
                    BinaryOp::NotEqual => Some(Constant::Bool(l != r)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Fold unary operations with constant operands
    fn fold_unary_op(op: &UnaryOp, val: &Constant) -> Option<Constant> {
        match (op, val) {
            (UnaryOp::Negate, Constant::Integer(n)) => Some(Constant::Integer(-n)),
            (UnaryOp::Negate, Constant::Float(f)) => Some(Constant::Float(-f)),
            (UnaryOp::Not, Constant::Bool(b)) => Some(Constant::Bool(!b)),
            (UnaryOp::BitwiseNot, Constant::Integer(n)) => Some(Constant::Integer(!n)),
            _ => None,
        }
    }

    /// O1 Pass: Dead Code Elimination - Remove unused variable assignments
    fn dead_code_elimination(blocks: &mut [BasicBlock]) -> MirResult<()> {
        // First pass: collect all used places
        let mut used_places = HashSet::new();

        // Mark all operands in terminators as used
        for block in blocks.iter() {
            match &block.terminator {
                Terminator::If(cond, _, _) => {
                    Self::collect_places_from_operand(cond, &mut used_places);
                }
                Terminator::Return(Some(operand)) => {
                    Self::collect_places_from_operand(operand, &mut used_places);
                }
                _ => {}
            }
        }

        // Mark all operands in statements (right-hand side)
        for block in blocks.iter() {
            for stmt in &block.statements {
                Self::collect_places_from_rvalue(&stmt.rvalue, &mut used_places);
            }
        }

        // Second pass: remove statements that assign to unused places
        for block in blocks {
            block.statements.retain(|stmt| {
                // Keep statement if its target is used, or if it has side effects
                used_places.contains(&stmt.place) || Self::has_side_effects(&stmt.rvalue)
            });
        }

        Ok(())
    }

    /// Collect places from an operand
    fn collect_places_from_operand(operand: &Operand, places: &mut HashSet<Place>) {
        match operand {
            Operand::Move(place) | Operand::Copy(place) => {
                places.insert(place.clone());
            }
            Operand::Constant(_) => {}
        }
    }

    /// Collect places from an rvalue
    fn collect_places_from_rvalue(rvalue: &Rvalue, places: &mut HashSet<Place>) {
        match rvalue {
            Rvalue::Use(op) => Self::collect_places_from_operand(op, places),
            Rvalue::BinaryOp(_, l, r) => {
                Self::collect_places_from_operand(l, places);
                Self::collect_places_from_operand(r, places);
            }
            Rvalue::UnaryOp(_, op) => Self::collect_places_from_operand(op, places),
            Rvalue::Call(_, args) => {
                for arg in args {
                    Self::collect_places_from_operand(arg, places);
                }
            }
            Rvalue::Aggregate(_, operands) | Rvalue::Array(operands) => {
                for operand in operands {
                    Self::collect_places_from_operand(operand, places);
                }
            }
            Rvalue::Ref(place) | Rvalue::Deref(place) | Rvalue::Field(place, _) | Rvalue::Index(place, _) => {
                places.insert(place.clone());
            }
        }
    }

    /// Check if an rvalue has side effects (function calls, memory operations)
    fn has_side_effects(rvalue: &Rvalue) -> bool {
        match rvalue {
            Rvalue::Call(_, _) => true, // Function calls always have potential side effects
            Rvalue::Ref(_) => true,     // Creating references has side effects
            _ => false,
        }
    }

    /// O2 Pass: Simplify Control Flow - Remove redundant gotos and merge blocks
    fn simplify_control_flow(blocks: &mut Vec<BasicBlock>) -> MirResult<()> {
        let mut changed = true;
        while changed {
            changed = false;

            // Remove chains of gotos (goto -> goto -> dest becomes goto -> dest)
            for i in 0..blocks.len() {
                if let Terminator::Goto(target) = blocks[i].terminator {
                    if target < blocks.len() {
                        let final_target = Self::follow_goto_chain(blocks, target);
                        if final_target != target {
                            blocks[i].terminator = Terminator::Goto(final_target);
                            changed = true;
                        }
                    }
                }
            }

            // Merge consecutive blocks when possible
            let mut to_merge = Vec::new();
            for i in 0..blocks.len() {
                if let Terminator::Goto(target) = blocks[i].terminator {
                    if target == i + 1 && i + 1 < blocks.len() {
                        // Check if only this block jumps to the next one
                        let only_predecessor = blocks.iter().enumerate().all(|(j, b)| {
                            if j == i {
                                true
                            } else {
                                match &b.terminator {
                                    Terminator::Goto(t) => *t != i + 1,
                                    Terminator::If(_, t, e) => *t != i + 1 && *e != i + 1,
                                    _ => true,
                                }
                            }
                        });

                        if only_predecessor && blocks[i].statements.iter().all(|s| !Self::has_side_effects(&s.rvalue)) {
                            to_merge.push(i);
                            changed = true;
                        }
                    }
                }
            }

            // Perform merges in reverse order to maintain indices
            for i in to_merge.iter().rev() {
                if *i + 1 < blocks.len() {
                    let mut next_block = blocks.remove(*i + 1);
                    blocks[*i].statements.append(&mut next_block.statements);
                    blocks[*i].terminator = next_block.terminator;
                }
            }

            // Update references after merging
            Self::update_block_references(blocks);
        }

        Ok(())
    }

    /// Follow a chain of goto statements to find the final target
    fn follow_goto_chain(blocks: &[BasicBlock], mut target: usize) -> usize {
        loop {
            if target >= blocks.len() {
                break;
            }
            if let Terminator::Goto(next) = blocks[target].terminator {
                if next == target {
                    // Infinite loop, stop
                    break;
                }
                target = next;
            } else {
                break;
            }
        }
        target
    }

    /// Update block reference indices after removing blocks
    fn update_block_references(blocks: &mut [BasicBlock]) {
        // This is a simplified version - in practice would need to track removed blocks
        let max_idx = blocks.len().saturating_sub(1);
        for block in blocks.iter_mut() {
            match &mut block.terminator {
                Terminator::Goto(ref mut t) => {
                    *t = (*t).min(max_idx);
                }
                Terminator::If(_, ref mut then_bb, ref mut else_bb) => {
                    *then_bb = (*then_bb).min(max_idx);
                    *else_bb = (*else_bb).min(max_idx);
                }
                _ => {}
            }
        }
    }

    /// O3 Pass: Copy Propagation - Replace variables with their definitions
    fn copy_propagation(blocks: &mut [BasicBlock]) -> MirResult<()> {
        // Build a map of: place -> place it was assigned from (for simple copies)
        let mut copy_map: HashMap<Place, Operand> = HashMap::new();

        // First pass: identify copy assignments
        for block in blocks.iter() {
            for stmt in &block.statements {
                match &stmt.rvalue {
                    Rvalue::Use(op @ (Operand::Copy(_) | Operand::Move(_))) => {
                        copy_map.insert(stmt.place.clone(), op.clone());
                    }
                    _ => {} // Not a simple copy
                }
            }
        }

        // Second pass: replace uses of copied variables
        for block in blocks {
            for stmt in &mut block.statements {
                Self::substitute_operands(&mut stmt.rvalue, &copy_map);
            }

            match &mut block.terminator {
                Terminator::If(op, _, _) => {
                    if let Operand::Move(ref mut place) | Operand::Copy(ref mut place) = op {
                        if let Some(Operand::Copy(orig) | Operand::Move(orig)) = copy_map.get(place) {
                            *place = orig.clone();
                        }
                    }
                }
                Terminator::Return(Some(op)) => {
                    if let Operand::Move(ref mut place) | Operand::Copy(ref mut place) = op {
                        if let Some(Operand::Copy(orig) | Operand::Move(orig)) = copy_map.get(place) {
                            *place = orig.clone();
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Substitute operands in an rvalue using the copy map
    fn substitute_operands(rvalue: &mut Rvalue, copy_map: &HashMap<Place, Operand>) {
        match rvalue {
            Rvalue::Use(op) => Self::substitute_operand(op, copy_map),
            Rvalue::BinaryOp(_, l, r) => {
                Self::substitute_operand(l, copy_map);
                Self::substitute_operand(r, copy_map);
            }
            Rvalue::UnaryOp(_, op) => Self::substitute_operand(op, copy_map),
            Rvalue::Call(_, args) => {
                for arg in args {
                    Self::substitute_operand(arg, copy_map);
                }
            }
            Rvalue::Aggregate(_, operands) | Rvalue::Array(operands) => {
                for operand in operands {
                    Self::substitute_operand(operand, copy_map);
                }
            }
            _ => {} // No operand substitution needed for Ref, Deref, Field, Index
        }
    }

    /// Substitute an operand if it's a copied variable
    fn substitute_operand(operand: &mut Operand, copy_map: &HashMap<Place, Operand>) {
        if let Operand::Copy(place) | Operand::Move(place) = operand {
            if let Some(replacement) = copy_map.get(place) {
                *operand = replacement.clone();
            }
        }
    }
}

/// Public API: Lower HIR to MIR
pub fn lower_to_mir(items: &[HirItem]) -> MirResult<Mir> {
    let mut lowerer = MirLowerer::new();
    lowerer.lower_items(items)
}

/// Public API: Optimize MIR with specified optimization level (1-3)
pub fn optimize_mir(mir: &mut Mir, opt_level: u32) -> MirResult<()> {
    MirOptimizer::optimize(mir, opt_level)
}