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
    /// Dereference: *ptr
    Deref(Box<Place>),
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Place::Local(name) => write!(f, "{}", name),
            Place::Field(place, field) => write!(f, "{}.{}", place, field),
            Place::Index(place, idx) => write!(f, "{}[{}]", place, idx),
            Place::Deref(place) => write!(f, "*{}", place),
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

/// Global constant or static variable
#[derive(Debug, Clone)]
pub struct GlobalItem {
    pub name: String,
    pub is_static: bool,
    pub is_mutable: bool,
    pub value: i64,  // simplified: support i64 values for now
    pub is_string: bool,  // if true, value is a string constant index
}

/// MIR for the entire program
#[derive(Debug, Clone)]
pub struct Mir {
    pub functions: Vec<MirFunction>,
    pub globals: Vec<GlobalItem>,
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
            eprintln!("[MIR-Builder] BEFORE: Block {} has {} statements", self.current_block, block.statements.len());
            block.statements.push(Statement { place: place.clone(), rvalue: rvalue.clone() });
            eprintln!("[MIR-Builder] AFTER: Block {} has {} statements", self.current_block, block.statements.len());
            eprintln!("[MIR-Builder] Added: {:?} = {:?}", place, rvalue);
        } else {
            eprintln!("[MIR-Builder] ERROR: Block {} does not exist!", self.current_block);
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
    available_functions: std::collections::HashSet<String>, // All functions that exist (including qualified names)
    local_types: std::collections::HashMap<String, String>, // Maps local variable names to their types
}

impl MirLowerer {
    /// Create a new MIR lowerer
    pub fn new() -> Self {
        MirLowerer {
            builder: MirBuilder::new(),
            closure_counter: 0,
            generated_functions: Vec::new(),
            closure_vars: std::collections::HashMap::new(),
            available_functions: std::collections::HashSet::new(),
            local_types: std::collections::HashMap::new(),
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
        // First pass: collect all available function names (including qualified ones)
        self.collect_available_functions(items, "");
        
        let mut functions = Vec::new();
        let mut globals = Vec::new();
        
        // Collect global constants and statics
        self.collect_globals_recursive(items, &mut globals)?;
        
        self.lower_items_recursive(items, "", &mut functions)?;

        // Add any generated closure functions
        functions.extend(self.generated_functions.drain(..));

        Ok(Mir { functions, globals })
    }

    fn collect_available_functions(&mut self, items: &[HirItem], module_prefix: &str) {
        for item in items {
            match item {
                HirItem::Function { name, .. } => {
                    let full_name = if module_prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}::{}", module_prefix, name)
                    };
                    self.available_functions.insert(full_name);
                }
                HirItem::Module { name, items: module_items, .. } => {
                    let new_prefix = if module_prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}::{}", module_prefix, name)
                    };
                    self.collect_available_functions(module_items, &new_prefix);
                }
                HirItem::Impl { struct_name, methods, .. } => {
                    // Collect functions from impl blocks with qualified names
                    for method_item in methods {
                        if let HirItem::Function { name, .. } = method_item {
                            let qualified_name = format!("{}::{}", struct_name, name);
                            self.available_functions.insert(qualified_name);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Collect global constants and static variables recursively
    fn collect_globals_recursive(&mut self, items: &[HirItem], globals: &mut Vec<GlobalItem>) -> MirResult<()> {
        for item in items {
            match item {
                HirItem::Const { name, ty: _, is_public: _, generics: _ } => {
                    // For now, const values are compiled away (inlined)
                    // We still track them for future reference
                    globals.push(GlobalItem {
                        name: name.clone(),
                        is_static: false,
                        is_mutable: false,
                        value: 0,  // placeholder
                        is_string: false,
                    });
                }
                HirItem::Static { name, ty: _, is_mutable, is_public: _, generics: _ } => {
                    globals.push(GlobalItem {
                        name: name.clone(),
                        is_static: true,
                        is_mutable: *is_mutable,
                        value: 0,  // placeholder
                        is_string: false,
                    });
                }
                HirItem::Module { items: module_items, .. } => {
                    self.collect_globals_recursive(module_items, globals)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn lower_items_recursive(&mut self, items: &[HirItem], module_prefix: &str, functions: &mut Vec<MirFunction>) -> MirResult<()> {
        for item in items {
            match item {
                HirItem::Function {
                    name,
                    params,
                    return_type,
                    body,
                    ..
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

                    let full_name = if module_prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}::{}", module_prefix, name)
                    };

                    let func = MirFunction {
                        name: full_name,
                        params: params.clone(),
                        return_type: return_type.clone().unwrap_or(HirType::Unknown),
                        basic_blocks: mir_builder.finish(),
                    };
                    functions.push(func);
                }
                HirItem::Struct { .. } => {
                }
                HirItem::Module { name, items: module_items, .. } => {
                    let new_prefix = if module_prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}::{}", module_prefix, name)
                    };
                    self.lower_items_recursive(module_items, &new_prefix, functions)?;
                }
                HirItem::Const { .. } => {
                    // Constants don't generate code
                }
                HirItem::Static { .. } => {
                    // Statics don't generate code in our simplified implementation
                }
                HirItem::AssociatedType { .. } => {
                }
                HirItem::Use { .. } => {
                }
                HirItem::Impl { struct_name, methods, .. } => {
                    // For impl methods, prepend the struct name to create proper qualified names
                    // e.g., impl Nums { fn sum() } becomes "Nums::sum"
                    let new_prefix = if module_prefix.is_empty() {
                        struct_name.clone()
                    } else {
                        format!("{}::{}", module_prefix, struct_name)
                    };
                    self.lower_items_recursive(methods, &new_prefix, functions)?;
                }
                HirItem::Enum { .. } => {
                }
                HirItem::Trait { .. } => {
                }
            }
        }
        Ok(())
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
                    // Try to infer type from the initialization expression
                    let inferred_type = match init {
                        HirExpression::Call { func, .. } => {
                            if let HirExpression::Variable(func_name) = &**func {
                                // Extract struct name from functions like "Counter::new"
                                func_name.split("::").next().map(|s| s.to_string())
                            } else {
                                None
                            }
                        }
                        HirExpression::StructLiteral { name, .. } => {
                            Some(name.clone())
                        }
                        HirExpression::Variable(struct_name) => {
                            // Handle bare struct references (unit structs or constructors)
                            // If the variable starts with uppercase, it's likely a struct type
                            if struct_name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                                Some(struct_name.clone())
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    
                    if let Some(ty) = inferred_type {
                        self.local_types.insert(name.clone(), ty);
                    }
                    
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
                        
                        // Create loop blocks - use separate block for condition check
                        let current_block = builder.current_block;
                        let loop_cond = builder.create_block();
                        let loop_body = builder.create_block();
                        let loop_end = builder.create_block();
                        
                        // Terminate current block with jump to condition check
                        builder.blocks[current_block].terminator = Terminator::Goto(loop_cond);
                        
                        // Loop condition block (separate from initialization)
                        builder.current_block = loop_cond;
                        
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
                        let loop_body_end = builder.current_block;
                        let inc_expr = Rvalue::BinaryOp(
                            BinaryOp::Add,
                            Operand::Copy(loop_var_place.clone()),
                            Operand::Constant(Constant::Integer(1))
                        );
                        builder.add_statement(loop_var_place, inc_expr);
                        builder.blocks[loop_body_end].terminator = Terminator::Goto(loop_cond);
                        
                        // Continue after loop
                        builder.current_block = loop_end;
                    }
                    _ => {
                        // Implement iterator protocol: for var in iter { body }
                        // Desugars into:
                        // let mut __iter = iter.into_iter();
                        // loop {
                        //   match __iter.next() {
                        //     Some(var) => { body },
                        //     None => break,
                        //   }
                        // }
                        
                        let iter_var = format!("__iter_{}", var);
                        let iter_var_place = Place::Local(iter_var.clone());
                        
                        // Call into_iter() on the collection
                        let iter_temp = builder.gen_temp();
                        self.lower_expression_to_place(builder, iter, Place::Local(iter_temp.clone()))?;
                        
                        // Store the iterator result
                        builder.add_statement(
                            iter_var_place.clone(),
                            Rvalue::Call("__into_iter".to_string(), vec![Operand::Copy(Place::Local(iter_temp))])
                        );
                        
                        // Create loop blocks
                        let current_block = builder.current_block;
                        let loop_cond = builder.create_block();
                        let loop_body = builder.create_block();
                        let loop_end = builder.create_block();
                        
                        // Jump to loop condition
                        builder.blocks[current_block].terminator = Terminator::Goto(loop_cond);
                        
                        // Loop condition: call next() on iterator
                        builder.current_block = loop_cond;
                        let next_result = builder.gen_temp();
                        builder.add_statement(
                            Place::Local(next_result.clone()),
                            Rvalue::Call("__next".to_string(), vec![Operand::Copy(iter_var_place)])
                        );
                        
                        // Check if Some(value) or None
                        // For simplicity, treat any non-zero result as Some
                        let cond_check = builder.gen_temp();
                        builder.add_statement(
                            Place::Local(cond_check.clone()),
                            Rvalue::BinaryOp(
                                BinaryOp::NotEqual,
                                Operand::Copy(Place::Local(next_result.clone())),
                                Operand::Constant(Constant::Integer(0))
                            )
                        );
                        
                        builder.set_terminator(Terminator::If(
                            Operand::Copy(Place::Local(cond_check)),
                            loop_body,
                            loop_end,
                        ));
                        
                        // Loop body
                        builder.current_block = loop_body;
                        
                        // Bind loop variable to the value from next()
                        // For now, just use the result directly
                        builder.add_statement(
                            Place::Local(var.clone()),
                            Rvalue::Use(Operand::Copy(Place::Local(next_result)))
                        );
                        
                        // Execute loop body
                        for stmt in body {
                            self.lower_statement_in_builder(builder, stmt)?;
                        }
                        
                        // Jump back to condition
                        let loop_body_end = builder.current_block;
                        builder.blocks[loop_body_end].terminator = Terminator::Goto(loop_cond);
                        
                        // Continue after loop
                        builder.current_block = loop_end;
                    }
                }
            }

            HirStatement::While {
                condition,
                body,
            } => {
                // Lower while loop into proper control flow graph
                // Terminate current block and jump to condition check
                let current_block = builder.current_block;
                let loop_cond = builder.create_block();
                let loop_body = builder.create_block();
                let loop_end = builder.create_block();
                
                // Terminate current block with jump to condition
                builder.blocks[current_block].terminator = Terminator::Goto(loop_cond);
                
                // Loop condition check (in a separate block)
                builder.current_block = loop_cond;
                let cond_start = builder.current_block;
                let cond_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, condition, Place::Local(cond_temp.clone()))?;
                
                let cond_end = builder.current_block;
                if cond_end != cond_start {
                    // Condition evaluation created nested blocks (e.g., nested if expression)
                    // Set If terminator on the final block where cond_temp has its value
                    builder.blocks[cond_end].terminator = Terminator::If(
                        Operand::Copy(Place::Local(cond_temp)),
                        loop_body,
                        loop_end,
                    );
                } else {
                    // Simple condition that didn't create blocks
                    builder.blocks[loop_cond].terminator = Terminator::If(
                        Operand::Copy(Place::Local(cond_temp)),
                        loop_body,
                        loop_end,
                    );
                }
                
                // Loop body
                builder.current_block = loop_body;
                for stmt in body {
                    self.lower_statement_in_builder(builder, stmt)?;
                }
                let loop_body_end = builder.current_block;
                builder.blocks[loop_body_end].terminator = Terminator::Goto(loop_cond);
                
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
                
                // Condition check
                let cond_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, condition, Place::Local(cond_temp.clone()))?;
                
                let if_block = builder.current_block;
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();
                
                builder.blocks[if_block].terminator = Terminator::If(
                    Operand::Copy(Place::Local(cond_temp)),
                    then_block,
                    else_block,
                );
                
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

            HirStatement::Item(item) => {
                // Extract inner functions and add them to generated functions
                if let HirItem::Function {
                    name,
                    params,
                    return_type,
                    body,
                    ..
                } = &**item
                {
                    let mut inner_builder = MirBuilder::new();
                    for stmt in body {
                        self.lower_statement_in_builder(&mut inner_builder, stmt)?;
                    }
                    
                    if matches!(inner_builder.blocks[inner_builder.current_block].terminator, Terminator::Unreachable) {
                        inner_builder.set_terminator(Terminator::Return(None));
                    }
                    
                    let func = MirFunction {
                        name: name.clone(),
                        params: params.clone(),
                        return_type: return_type.clone().unwrap_or(HirType::Unknown),
                        basic_blocks: inner_builder.finish(),
                    };
                    
                    // Register the inner function as an available function
                    self.available_functions.insert(name.clone());
                    self.generated_functions.push(func);
                }
                // Other nested items are ignored for now
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
                eprintln!("[MIR] Variable: Lowering variable '{}' to place {:?}", name, place);
                builder.add_statement(place, Rvalue::Use(Operand::Copy(Place::Local(name.clone()))));
                eprintln!("[MIR] Variable: Added statement");
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
                // Special handling for Reference and MutableReference:
                // We need to pass the place itself, not evaluate the operand
                if matches!(op, crate::lowering::UnaryOp::Reference | crate::lowering::UnaryOp::MutableReference) {
                    // For references, extract the place from the operand
                    match &**operand {
                        HirExpression::Variable(var_name) => {
                            // Create reference to a variable directly
                            let rvalue = Rvalue::UnaryOp(*op, Operand::Copy(Place::Local(var_name.clone())));
                            builder.add_statement(place, rvalue);
                        }
                        _ => {
                            // For complex expressions, evaluate to temp first
                            let op_temp = builder.gen_temp();
                            self.lower_expression_to_place(builder, operand, Place::Local(op_temp.clone()))?;
                            let rvalue = Rvalue::UnaryOp(*op, Operand::Copy(Place::Local(op_temp)));
                            builder.add_statement(place, rvalue);
                        }
                    }
                } else {
                    // For other unary operations, evaluate the operand normally
                    let op_temp = builder.gen_temp();
                    self.lower_expression_to_place(builder, operand, Place::Local(op_temp.clone()))?;
                    
                    let rvalue = Rvalue::UnaryOp(*op, Operand::Copy(Place::Local(op_temp)));
                    builder.add_statement(place, rvalue);
                }
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
                
                // Check if this is an unresolved method call
                // Try to find a qualified version: if we call "foo" and it's not in available_functions,
                // check if it's actually a method call
                if !self.available_functions.contains(&func_name) && !func_name.contains("::") {
                    // This might be a method call. Check if first argument can resolve the type
                    if !args.is_empty() {
                        // Try to infer the type of the first argument from available qualified methods
                        // Look for Type::func_name pattern
                        for available_func in &self.available_functions {
                            if available_func.ends_with(&format!("::{}", func_name)) {
                                func_name = available_func.clone();
                                break;
                            }
                        }
                    }
                }
                
                for arg in args {
                    // Optimization: Skip creating temps for simple variable references and literals
                    match arg {
                        HirExpression::Variable(var_name) => {
                            // It's just a variable reference, use it directly
                            mir_args.push(Operand::Copy(Place::Local(var_name.clone())));
                        }
                        HirExpression::Integer(n) => {
                            // It's a constant integer, use directly without temp
                            mir_args.push(Operand::Constant(Constant::Integer(*n)));
                        }
                        HirExpression::Float(f) => {
                            // It's a constant float, use directly without temp
                            mir_args.push(Operand::Constant(Constant::Float(*f)));
                        }
                        HirExpression::String(s) => {
                            // It's a constant string, use directly without temp
                            mir_args.push(Operand::Constant(Constant::String(s.clone())));
                        }
                        HirExpression::Bool(b) => {
                            // It's a constant bool, use directly without temp
                            mir_args.push(Operand::Constant(Constant::Bool(*b)));
                        }
                        _ => {
                            // Need to evaluate the expression
                            let temp = builder.gen_temp();
                            self.lower_expression_to_place(builder, arg, Place::Local(temp.clone()))?;
                            mir_args.push(Operand::Copy(Place::Local(temp)));
                        }
                    }
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
                
                // Handle different assignment targets
                match &**target {
                    HirExpression::Variable(name) => {
                        // Simple variable assignment: x = value
                        builder.add_statement(Place::Local(name.clone()), Rvalue::Use(Operand::Copy(Place::Local(val_temp))));
                        builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
                    }
                    HirExpression::UnaryOp { op: UnaryOp::Dereference, operand } => {
                        // Dereference assignment: *ptr = value
                        // First evaluate the pointer
                        let ptr_temp = builder.gen_temp();
                        self.lower_expression_to_place(builder, operand, Place::Local(ptr_temp.clone()))?;
                        
                        // Then store through the pointer
                        // In a full implementation, this would create a Store instruction
                        // For now, we'll represent it as an assignment to the dereferenced place
                        builder.add_statement(Place::Deref(Box::new(Place::Local(ptr_temp))), Rvalue::Use(Operand::Copy(Place::Local(val_temp))));
                        builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
                    }
                    HirExpression::FieldAccess { object, field } => {
                        // Field assignment: obj.field = value
                        let obj_temp = builder.gen_temp();
                        self.lower_expression_to_place(builder, object, Place::Local(obj_temp.clone()))?;
                        
                        builder.add_statement(Place::Field(Box::new(Place::Local(obj_temp)), field.clone()), Rvalue::Use(Operand::Copy(Place::Local(val_temp))));
                        builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
                    }
                    _ => {
                        return Err(MirError { message: "Complex assignment targets not yet supported".to_string() });
                    }
                }
            }
            HirExpression::If { condition, then_body, else_body } => {
                // If expressions in MIR become branches
                let cond_start_block = builder.current_block;
                let cond_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, condition, Place::Local(cond_temp.clone()))?;
                
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();
                
                let cond_end_block = builder.current_block;
                if cond_end_block != cond_start_block {
                    // Condition evaluation created nested blocks
                    let if_block = builder.create_block();
                    builder.blocks[cond_end_block].terminator = Terminator::Goto(if_block);
                    builder.blocks[if_block].terminator = Terminator::If(
                        Operand::Copy(Place::Local(cond_temp)),
                        then_block,
                        else_block,
                    );
                } else {
                    // Simple condition: set If terminator on cond_start_block
                    builder.blocks[cond_start_block].terminator = Terminator::If(
                        Operand::Copy(Place::Local(cond_temp)),
                        then_block,
                        else_block,
                    );
                }
                
                // Determine target place for assignments
                let target_place = place.clone();
                
                // Lower then body
                builder.current_block = then_block;
                let then_len = then_body.len();
                for (idx, stmt) in then_body.iter().enumerate() {
                    if idx == then_len - 1 {
                        match stmt {
                            HirStatement::Expression(expr) => {
                                self.lower_expression_to_place(builder, expr, target_place.clone())?;
                            }
                            HirStatement::Return(Some(expr)) => {
                                self.lower_expression_to_place(builder, expr, target_place.clone())?;
                            }
                            HirStatement::Return(None) => {
                                builder.add_statement(target_place.clone(), Rvalue::Use(Operand::Constant(Constant::Unit)));
                            }
                            HirStatement::If { condition, then_body: if_then_body, else_body: if_else_body } => {
                                // Statement-level if that returns a value (implicitly the last expression)
                                // Convert it to expression-level if handling by recursively processing it
                                self.lower_expression_to_place(builder, &HirExpression::If {
                                    condition: condition.clone(),
                                    then_body: if_then_body.clone(),
                                    else_body: if_else_body.clone(),
                                }, target_place.clone())?;
                            }
                            _ => {
                                self.lower_statement_in_builder(builder, stmt)?;
                            }
                        }
                    } else {
                        self.lower_statement_in_builder(builder, stmt)?;
                    }
                }
                // Set terminator on the actual current block (could be different if nested expressions created blocks)
                let then_end_block = builder.current_block;
                builder.blocks[then_end_block].terminator = Terminator::Goto(merge_block);
                
                // Lower else body
                builder.current_block = else_block;
                if let Some(else_stmts) = else_body {
                    let else_len = else_stmts.len();
                    for (idx, stmt) in else_stmts.iter().enumerate() {
                        if idx == else_len - 1 {
                            match stmt {
                                HirStatement::Expression(expr) => {
                                    self.lower_expression_to_place(builder, expr, target_place.clone())?;
                                }
                                HirStatement::Return(Some(expr)) => {
                                    self.lower_expression_to_place(builder, expr, target_place.clone())?;
                                }
                                HirStatement::Return(None) => {
                                    builder.add_statement(target_place.clone(), Rvalue::Use(Operand::Constant(Constant::Unit)));
                                }
                                HirStatement::If { condition, then_body: if_then_body, else_body: if_else_body } => {
                                    // Statement-level if that returns a value (implicitly the last expression)
                                    // Convert it to expression-level if handling by recursively processing it
                                    self.lower_expression_to_place(builder, &HirExpression::If {
                                        condition: condition.clone(),
                                        then_body: if_then_body.clone(),
                                        else_body: if_else_body.clone(),
                                    }, target_place.clone())?;
                                }
                                _ => {
                                    self.lower_statement_in_builder(builder, stmt)?;
                                }
                            }
                        } else {
                            self.lower_statement_in_builder(builder, stmt)?;
                        }
                    }
                } else {
                    builder.add_statement(target_place.clone(), Rvalue::Use(Operand::Constant(Constant::Unit)));
                }
                // Set terminator on the actual current block (could be different if nested expressions created blocks)
                let else_end_block = builder.current_block;
                builder.blocks[else_end_block].terminator = Terminator::Goto(merge_block);
                
                // Continue at merge block
                builder.current_block = merge_block;
            }
            HirExpression::While { condition, body } => {
                let loop_cond = builder.create_block();
                let loop_body = builder.create_block();
                let loop_end = builder.create_block();
                
                // Transition from current block to loop condition
                let current_block = builder.current_block;
                builder.blocks[current_block].terminator = Terminator::Goto(loop_cond);
                
                // Loop condition check - use a fresh block so initial statements aren't in the loop
                builder.current_block = loop_cond;
                let cond_temp = builder.gen_temp();
                let cond_start = builder.current_block;
                self.lower_expression_to_place(builder, condition, Place::Local(cond_temp.clone()))?;
                
                let cond_end = builder.current_block;
                if cond_end != cond_start {
                    // Condition evaluation created nested blocks (e.g., nested if expression)
                    // The nested if handler already set loop_cond's terminator to jump to its branches.
                    // Those branches will assign to cond_temp and goto merge_block (cond_end).
                    // We just need to set the merge block to check cond_temp for the while loop.
                    builder.blocks[cond_end].terminator = Terminator::If(
                        Operand::Copy(Place::Local(cond_temp)),
                        loop_body,
                        loop_end,
                    );
                } else {
                    // Simple condition that didn't create blocks
                    builder.blocks[loop_cond].terminator = Terminator::If(
                        Operand::Copy(Place::Local(cond_temp)),
                        loop_body,
                        loop_end,
                    );
                }
                
                // Loop body
                builder.current_block = loop_body;
                for stmt in body {
                    self.lower_statement_in_builder(builder, stmt)?;
                }
                let loop_body_end = builder.current_block;
                builder.blocks[loop_body_end].terminator = Terminator::Goto(loop_cond);
                
                // After loop
                builder.current_block = loop_end;
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
            }
            HirExpression::FieldAccess { object, field } => {
                // Evaluate the object expression to a temporary
                let obj_temp = builder.gen_temp();
                eprintln!("[MIR] FieldAccess: Lowering object to temp '{}'", obj_temp);
                self.lower_expression_to_place(builder, object, Place::Local(obj_temp.clone()))?;
                eprintln!("[MIR] FieldAccess: Object lowered, now creating field access from '{}' to '{}'", obj_temp, field);
                
                // Then access the field from that temporary
                eprintln!("[MIR] FieldAccess: Adding statement: {} = Use(Copy(Field({}, {})))", 
                         if let Place::Local(n) = &place { n } else { "?" },
                         obj_temp, field);
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
                self.lower_expression_to_place(builder, array, Place::Local(arr_temp.clone()))?;
                
                // Extract the index value - it should be a constant or variable
                match index.as_ref() {
                    HirExpression::Integer(idx_val) => {
                        // Direct integer index - create Rvalue::Index
                        builder.add_statement(place, Rvalue::Index(Place::Local(arr_temp), *idx_val as usize));
                    }
                    HirExpression::Variable(var_name) => {
                        // Index from variable - we need to handle this differently
                        // For now, create a temporary and use it
                        let idx_temp = builder.gen_temp();
                        self.lower_expression_to_place(builder, index, Place::Local(idx_temp.clone()))?;
                        // Can't use variable index directly with Rvalue::Index, so treat as unit for now
                        builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
                    }
                    _ => {
                        // Complex index expression - evaluate it first
                        let idx_temp = builder.gen_temp();
                        self.lower_expression_to_place(builder, index, Place::Local(idx_temp.clone()))?;
                        // Treat as unit for complex expressions
                        builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Unit)));
                    }
                }
            }
            HirExpression::StructLiteral { name, fields } => {
                // Struct literals become Rvalue::Aggregate with field operands
                let mut operands = Vec::new();
                for (_field_name, field_expr) in fields {
                    let field_temp = builder.gen_temp();
                    self.lower_expression_to_place(builder, field_expr, Place::Local(field_temp.clone()))?;
                    operands.push(Operand::Copy(Place::Local(field_temp)));
                }
                // Create aggregate with proper struct name
                builder.add_statement(place, Rvalue::Aggregate(name.clone(), operands));
            }
            HirExpression::ArrayLiteral(elements) => {
                // Convert each array element to an operand
                let mut operands = Vec::new();
                for elem in elements {
                    let elem_temp = builder.gen_temp();
                    self.lower_expression_to_place(builder, elem, Place::Local(elem_temp.clone()))?;
                    operands.push(Operand::Copy(Place::Local(elem_temp)));
                }
                // Create array with proper Rvalue::Array
                builder.add_statement(place, Rvalue::Array(operands));
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
            HirExpression::Match { scrutinee, arms } => {
                // Match expressions: evaluate scrutinee, then process each arm
                let scrutinee_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, scrutinee, Place::Local(scrutinee_temp.clone()))?;
                
                let merge_block = builder.create_block();
                let curr = builder.current_block;
                
                for (arm_idx, arm) in arms.iter().enumerate() {
                    let arm_block = builder.create_block();
                    
                    if arm_idx == 0 {
                        builder.blocks[curr].terminator = Terminator::Goto(arm_block);
                    }
                    
                    builder.current_block = arm_block;
                    
                    let then_len = arm.body.len();
                    for (idx, stmt) in arm.body.iter().enumerate() {
                        if idx == then_len - 1 {
                            match stmt {
                                HirStatement::Expression(expr) => {
                                    self.lower_expression_to_place(builder, expr, place.clone())?;
                                }
                                _ => {
                                    self.lower_statement_in_builder(builder, stmt)?;
                                }
                            }
                        } else {
                            self.lower_statement_in_builder(builder, stmt)?;
                        }
                    }
                    
                    let arm_end = builder.current_block;
                    builder.blocks[arm_end].terminator = Terminator::Goto(merge_block);
                }
                
                builder.current_block = merge_block;
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
            HirExpression::EnumVariant { enum_name: _, variant_name: _, args } => {
                for arg in args {
                    let temp = builder.gen_temp();
                    self.lower_expression_to_place(builder, arg, Place::Local(temp))?;
                }
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Integer(0))));
            }
            HirExpression::EnumStructVariant { enum_name: _, variant_name: _, fields } => {
                for (_, field_expr) in fields {
                    let temp = builder.gen_temp();
                    self.lower_expression_to_place(builder, field_expr, Place::Local(temp))?;
                }
                builder.add_statement(place, Rvalue::Use(Operand::Constant(Constant::Integer(0))));
            }
            HirExpression::MethodCall { receiver, method, args } => {
                // Evaluate receiver to a temporary
                let receiver_temp = builder.gen_temp();
                self.lower_expression_to_place(builder, receiver, Place::Local(receiver_temp.clone()))?;
                
                // Try to infer receiver type from the expression
                let receiver_type = match &**receiver {
                    HirExpression::Variable(var_name) => {
                        // First check if it's a known local variable with a tracked type
                        if let Some(ty) = self.local_types.get(var_name).cloned() {
                            Some(ty)
                        } else {
                            // Otherwise, the variable name might be a struct type itself
                            // (e.g., unit structs used as values like `let dog = Dog;`)
                            // For now, assume the variable name is the type if it starts with uppercase
                            if var_name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                                Some(var_name.clone())
                            } else {
                                None
                            }
                        }
                    }
                    HirExpression::FieldAccess { object, field } => {
                        // For field accesses like self.items, check if field name is a collection
                        // e.g., if field is "items" it's likely a Vec or collection type
                        if method == "push" || method == "pop" || method == "get" || method == "len" {
                            Some("Vec".to_string())
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                
                // Map built-in collection methods to runtime functions
                let func_name = if let Some(struct_type) = receiver_type {
                    // Check if it's a built-in collection type
                    match struct_type.as_str() {
                        "Vec" => {
                            match method.as_str() {
                                "push" => "gaia_vec_push".to_string(),
                                "pop" => "gaia_vec_pop".to_string(),
                                "get" => "gaia_vec_get".to_string(),
                                "len" => "gaia_vec_len".to_string(),
                                _ => format!("{}::{}", struct_type, method),
                            }
                        }
                        _ => format!("{}::{}", struct_type, method),
                    }
                } else {
                    format!("__method_{}", method)
                };
                
                // Collect operands: receiver followed by method arguments
                let mut operands = vec![Operand::Copy(Place::Local(receiver_temp))];
                for arg in args {
                    let arg_temp = builder.gen_temp();
                    self.lower_expression_to_place(builder, arg, Place::Local(arg_temp.clone()))?;
                    operands.push(Operand::Copy(Place::Local(arg_temp)));
                }
                
                builder.add_statement(place, Rvalue::Call(func_name, operands));
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
        // Also mark the inner place of Deref assignments as used
        for block in blocks.iter() {
            for stmt in &block.statements {
                Self::collect_places_from_rvalue(&stmt.rvalue, &mut used_places);
                // If this statement assigns to a deref, mark the inner place as used
                if let Place::Deref(inner) = &stmt.place {
                    used_places.insert((**inner).clone());
                }
            }
        }

        // Second pass: remove statements that assign to unused places
        for block in blocks {
            block.statements.retain(|stmt| {
                // Keep statement if its target is used, if it has side effects, 
                // or if it's a dereference assignment (which has side effects: writes to memory)
                let is_deref = matches!(&stmt.place, crate::mir::Place::Deref(_));
                used_places.contains(&stmt.place) || Self::has_side_effects(&stmt.rvalue) || is_deref
            });
        }

        Ok(())
    }

    /// Collect places from an operand
    fn collect_places_from_operand(operand: &Operand, places: &mut HashSet<Place>) {
        match operand {
            Operand::Move(place) | Operand::Copy(place) => {
                // Recursively collect from the place structure
                Self::collect_places_from_place(place, places);
            }
            Operand::Constant(_) => {}
        }
    }
    
    /// Collect all places from a place structure (handles nested Field, Index, Deref)
    fn collect_places_from_place(place: &Place, places: &mut HashSet<Place>) {
        match place {
            Place::Local(_) => {
                places.insert(place.clone());
            }
            Place::Field(inner, _) | Place::Index(inner, _) | Place::Deref(inner) => {
                // Recursively collect from the inner place
                Self::collect_places_from_place(inner, places);
                // Also add this place itself
                places.insert(place.clone());
            }
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
            Rvalue::Array(_) => true,   // Array construction has side effects (allocates stack space)
            Rvalue::Aggregate(_, _) => true, // Struct construction has side effects (allocates stack space)
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
                // Don't follow chains through blocks with statements
                if !blocks[target].statements.is_empty() {
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