//! # Constraint Generation
//!
//! Walks the AST/HIR and generates type constraints (equations) that need to be solved.
//! This is the bridge between the parser output and the unification engine.
//!
//! ## Algorithm:
//! 1. Assign type variable to each expression
//! 2. Generate equations from operations (e.g., `+` requires both operands to be numeric)
//! 3. Collect all constraints
//! 4. Solve with unification engine

use super::types::{Type, TypeVar, StructId, GenericId};
use super::substitution::Substitution;
use super::unification::UnificationEngine;
use std::collections::HashMap;

/// A single type constraint (equation)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    /// Left-hand side of equation
    pub ty1: Type,
    /// Right-hand side of equation
    pub ty2: Type,
}

impl Constraint {
    /// Create a new constraint
    pub fn new(ty1: Type, ty2: Type) -> Self {
        Constraint { ty1, ty2 }
    }
}

/// Mapping from expressions/variables to their type variables
pub type ExprTypeMap = HashMap<String, TypeVar>;

/// Constraint generator: converts AST to type constraints
pub struct ConstraintGenerator {
    /// Current variable counter for fresh type variables
    var_counter: usize,
    /// Generated constraints
    pub constraints: Vec<Constraint>,
    /// Symbol table: maps names to type variables
    pub symbols: HashMap<String, TypeVar>,
    /// Struct definitions
    pub struct_defs: HashMap<String, StructDef>,
    /// Function definitions
    pub function_defs: HashMap<String, FunctionDef>,
}

/// Definition of a struct for constraint generation
#[derive(Debug, Clone)]
pub struct StructDef {
    pub id: StructId,
    pub fields: HashMap<String, Type>,
    pub generics: Vec<GenericId>,
}

/// Definition of a function for constraint generation
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub param_types: Vec<Type>,
    pub return_type: Type,
    pub generics: Vec<GenericId>,
}

/// Generic parameter information
#[derive(Debug, Clone)]
pub struct GenericParam {
    pub id: GenericId,
    pub name: String,
    pub bounds: Vec<String>, // Trait names
}

impl ConstraintGenerator {
    /// Create a new constraint generator
    pub fn new() -> Self {
        ConstraintGenerator {
            var_counter: 0,
            constraints: Vec::new(),
            symbols: HashMap::new(),
            struct_defs: HashMap::new(),
            function_defs: HashMap::new(),
        }
    }

    /// Generate a fresh type variable
    pub fn fresh_var(&mut self) -> TypeVar {
        let var = TypeVar(self.var_counter);
        self.var_counter += 1;
        var
    }

    /// Add a constraint
    pub fn add_constraint(&mut self, ty1: Type, ty2: Type) {
        self.constraints.push(Constraint::new(ty1, ty2));
    }

    /// Register a variable in the symbol table
    pub fn register_var(&mut self, name: String, var: TypeVar) {
        self.symbols.insert(name, var);
    }

    /// Look up a variable's type variable
    pub fn lookup_var(&self, name: &str) -> Option<TypeVar> {
        self.symbols.get(name).copied()
    }

    /// Register a struct definition
    pub fn register_struct(&mut self, id: StructId, fields: HashMap<String, Type>) {
        self.struct_defs.insert(
            format!("struct_{}", id.0),
            StructDef { id, fields, generics: Vec::new() },
        );
    }

    /// Register a struct definition with generics
    pub fn register_struct_with_generics(
        &mut self,
        id: StructId,
        fields: HashMap<String, Type>,
        generics: Vec<GenericId>,
    ) {
        self.struct_defs.insert(
            format!("struct_{}", id.0),
            StructDef { id, fields, generics },
        );
    }

    /// Register a function definition
    pub fn register_function(&mut self, name: String, param_types: Vec<Type>, return_type: Type) {
        self.function_defs.insert(name, FunctionDef { param_types, return_type, generics: Vec::new() });
    }

    /// Register a function definition with generics
    pub fn register_function_with_generics(
        &mut self,
        name: String,
        param_types: Vec<Type>,
        return_type: Type,
        generics: Vec<GenericId>,
    ) {
        self.function_defs.insert(name, FunctionDef { param_types, return_type, generics });
    }

    /// Solve all generated constraints
    pub fn solve(&self) -> Result<Substitution, String> {
        let mut engine = UnificationEngine::new();
        let mut subst = Substitution::new();

        for constraint in &self.constraints {
            engine.unify(&constraint.ty1, &constraint.ty2, &mut subst)?;
        }

        Ok(subst)
    }

    // === Expression Constraint Generation ===

    /// Generate constraints for a binary operation
    pub fn constrain_binary_op(
        &mut self,
        op: BinaryOp,
        left_ty: Type,
        right_ty: Type,
    ) -> Result<Type, String> {
        match op {
            // Arithmetic operators require numeric types
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                let left_var = TypeVar(self.var_counter);
                self.var_counter += 1;
                let right_var = TypeVar(self.var_counter);
                self.var_counter += 1;
                let result_var = TypeVar(self.var_counter);
                self.var_counter += 1;

                // left and right must be numeric
                self.add_constraint(left_ty, Type::Variable(left_var));
                self.add_constraint(right_ty, Type::Variable(right_var));

                // For now, result type matches operand type
                self.add_constraint(Type::Variable(left_var), Type::Variable(result_var));

                Ok(Type::Variable(result_var))
            }

            // Comparison operators return bool
            BinaryOp::Less | BinaryOp::LessEq | BinaryOp::Greater | BinaryOp::GreaterEq => {
                self.add_constraint(left_ty.clone(), right_ty);
                Ok(Type::Bool)
            }

            // Equality operators return bool
            BinaryOp::Equal | BinaryOp::NotEqual => {
                self.add_constraint(left_ty, right_ty);
                Ok(Type::Bool)
            }

            // Logical operators require bool operands
            BinaryOp::And | BinaryOp::Or => {
                self.add_constraint(left_ty, Type::Bool);
                self.add_constraint(right_ty, Type::Bool);
                Ok(Type::Bool)
            }

            // Bitwise operators
            BinaryOp::BitwiseAnd | BinaryOp::BitwiseOr | BinaryOp::BitwiseXor => {
                self.add_constraint(left_ty.clone(), right_ty);
                Ok(left_ty)
            }

            // Other operators
            BinaryOp::Modulo => {
                self.add_constraint(left_ty.clone(), right_ty);
                Ok(left_ty)
            }

            BinaryOp::LeftShift | BinaryOp::RightShift => {
                // Both operands should be integers
                self.add_constraint(left_ty.clone(), right_ty);
                Ok(left_ty)
            }
        }
    }

    /// Generate constraints for a unary operation
    pub fn constrain_unary_op(
        &mut self,
        op: UnaryOp,
        operand_ty: Type,
    ) -> Result<Type, String> {
        match op {
            UnaryOp::Negate => {
                // Negate works on numeric types
                Ok(operand_ty)
            }
            UnaryOp::Not => {
                // Not requires bool
                self.add_constraint(operand_ty, Type::Bool);
                Ok(Type::Bool)
            }
            UnaryOp::BitwiseNot => {
                // Bitwise not works on integers
                Ok(operand_ty)
            }
            UnaryOp::Reference => {
                // &T
                Ok(Type::Reference {
                    lifetime: None,
                    mutable: false,
                    inner: Box::new(operand_ty),
                })
            }
            UnaryOp::MutableReference => {
                // &mut T
                Ok(Type::Reference {
                    lifetime: None,
                    mutable: true,
                    inner: Box::new(operand_ty),
                })
            }
            UnaryOp::Dereference => {
                // Dereference: requires operand to be a reference or pointer
                let result_var = TypeVar(self.var_counter);
                self.var_counter += 1;
                let ref_type = Type::Reference {
                    lifetime: None,
                    mutable: false,
                    inner: Box::new(Type::Variable(result_var)),
                };
                self.add_constraint(operand_ty, ref_type);
                Ok(Type::Variable(result_var))
            }
        }
    }

    /// Generate constraints for a function call
    pub fn constrain_function_call(
        &mut self,
        func_name: &str,
        arg_types: Vec<Type>,
    ) -> Result<Type, String> {
        // Clone function def to avoid borrow checker issues
        let func_def = self.function_defs.get(func_name).cloned();
        
        if let Some(func_def) = func_def {
            // Check argument count
            if arg_types.len() != func_def.param_types.len() {
                return Err(format!(
                    "Function '{}' expects {} arguments, got {}",
                    func_name,
                    func_def.param_types.len(),
                    arg_types.len()
                ));
            }

            // Constrain arguments to match expected types
            for (arg_ty, param_ty) in arg_types.iter().zip(&func_def.param_types) {
                self.add_constraint(arg_ty.clone(), param_ty.clone());
            }

            Ok(func_def.return_type.clone())
        } else {
            // Unknown function - can't generate constraints
            Err(format!("Unknown function: '{}'", func_name))
        }
    }

    /// Generate constraints for field access
    /// Returns the field type if found
    pub fn constrain_field_access(
        &mut self,
        struct_type: &Type,
        field_name: &str,
    ) -> Result<Type, String> {
        match struct_type {
            Type::Struct(struct_id) => {
                let key = format!("struct_{}", struct_id.0);
                if let Some(struct_def) = self.struct_defs.get(&key) {
                    if let Some(field_type) = struct_def.fields.get(field_name) {
                        Ok(field_type.clone())
                    } else {
                        Err(format!(
                            "Struct {} has no field '{}'",
                            struct_id.0, field_name
                        ))
                    }
                } else {
                    Err(format!("Unknown struct: {}", struct_id.0))
                }
            }
            Type::Variable(_var) => {
                // Can't resolve field access on type variable yet
                // Return fresh variable for constraint resolution
                let fresh_var = self.fresh_var();
                Ok(Type::Variable(fresh_var))
            }
            _ => Err(format!(
                "Cannot access field '{}' on non-struct type: {:?}",
                field_name, struct_type
            )),
        }
    }

    /// Generate constraints for method call
    /// Methods are functions associated with a struct type
    pub fn constrain_method_call(
        &mut self,
        receiver_type: &Type,
        method_name: &str,
        arg_types: Vec<Type>,
    ) -> Result<Type, String> {
        // For now, method calls are treated like function calls with the receiver type constraint
        // In full implementation, we'd look up method signatures from trait impls
        
        // Generate method call name as struct_method
        let full_method_name = match receiver_type {
            Type::Struct(struct_id) => format!("{}_method_{}", struct_id.0, method_name),
            _ => format!("_method_{}", method_name),
        };

        // Try to find as a registered function
        if let Some(func_def) = self.function_defs.get(&full_method_name).cloned() {
            // Check argument count (excluding self)
            if arg_types.len() != func_def.param_types.len() {
                return Err(format!(
                    "Method '{}' expects {} arguments, got {}",
                    method_name,
                    func_def.param_types.len(),
                    arg_types.len()
                ));
            }

            // Constrain arguments
            for (arg_ty, param_ty) in arg_types.iter().zip(&func_def.param_types) {
                self.add_constraint(arg_ty.clone(), param_ty.clone());
            }

            Ok(func_def.return_type.clone())
        } else {
            // Method not found - return fresh variable for constraint resolution
            let fresh_var = self.fresh_var();
            Ok(Type::Variable(fresh_var))
        }
    }

    /// Instantiate a generic function with concrete types
    pub fn instantiate_generic_function(
        &self,
        func_name: &str,
        type_args: Vec<Type>,
    ) -> Result<(Vec<Type>, Type), String> {
        if let Some(func_def) = self.function_defs.get(func_name) {
            if func_def.generics.len() != type_args.len() {
                return Err(format!(
                    "Function '{}' expects {} type argument(s), got {}",
                    func_name,
                    func_def.generics.len(),
                    type_args.len()
                ));
            }

            // Create substitution mapping generic IDs to concrete types
            let mut generic_map: HashMap<GenericId, Type> = HashMap::new();
            for (generic_id, concrete_type) in func_def.generics.iter().zip(type_args.iter()) {
                generic_map.insert(*generic_id, concrete_type.clone());
            }

            // Apply substitution to parameter and return types
            let instantiated_params = func_def
                .param_types
                .iter()
                .map(|param_ty| self.substitute_generics(param_ty, &generic_map))
                .collect();

            let instantiated_ret = self.substitute_generics(&func_def.return_type, &generic_map);

            Ok((instantiated_params, instantiated_ret))
        } else {
            Err(format!("Unknown function: '{}'", func_name))
        }
    }

    /// Instantiate a generic struct with concrete types
    pub fn instantiate_generic_struct(
        &self,
        struct_id: StructId,
        type_args: Vec<Type>,
    ) -> Result<HashMap<String, Type>, String> {
        let key = format!("struct_{}", struct_id.0);
        if let Some(struct_def) = self.struct_defs.get(&key) {
            if struct_def.generics.len() != type_args.len() {
                return Err(format!(
                    "Struct {} expects {} type argument(s), got {}",
                    struct_id.0,
                    struct_def.generics.len(),
                    type_args.len()
                ));
            }

            // Create substitution mapping generic IDs to concrete types
            let mut generic_map: HashMap<GenericId, Type> = HashMap::new();
            for (generic_id, concrete_type) in struct_def.generics.iter().zip(type_args.iter()) {
                generic_map.insert(*generic_id, concrete_type.clone());
            }

            // Apply substitution to field types
            let instantiated_fields = struct_def
                .fields
                .iter()
                .map(|(name, field_ty)| {
                    (name.clone(), self.substitute_generics(field_ty, &generic_map))
                })
                .collect();

            Ok(instantiated_fields)
        } else {
            Err(format!("Unknown struct: {}", struct_id.0))
        }
    }

    /// Substitute generic type parameters with concrete types
    fn substitute_generics(&self, ty: &Type, generic_map: &HashMap<GenericId, Type>) -> Type {
        match ty {
            Type::Generic(generic_id) => {
                generic_map
                    .get(generic_id)
                    .cloned()
                    .unwrap_or_else(|| ty.clone())
            }
            Type::Array { element, size } => Type::Array {
                element: Box::new(self.substitute_generics(element, generic_map)),
                size: *size,
            },
            Type::Tuple(elements) => {
                Type::Tuple(
                    elements
                        .iter()
                        .map(|e| self.substitute_generics(e, generic_map))
                        .collect(),
                )
            }
            Type::Reference {
                lifetime,
                mutable,
                inner,
            } => Type::Reference {
                lifetime: *lifetime,
                mutable: *mutable,
                inner: Box::new(self.substitute_generics(inner, generic_map)),
            },
            Type::RawPointer { mutable, inner } => Type::RawPointer {
                mutable: *mutable,
                inner: Box::new(self.substitute_generics(inner, generic_map)),
            },
            Type::Function { params, ret } => Type::Function {
                params: params
                    .iter()
                    .map(|p| self.substitute_generics(p, generic_map))
                    .collect(),
                ret: Box::new(self.substitute_generics(ret, generic_map)),
            },
            _ => ty.clone(),
        }
    }
}

/// Binary operations for constraint generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Less,
    LessEq,
    Greater,
    GreaterEq,
    Equal,
    NotEqual,
    And,
    Or,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
}

/// Unary operations for constraint generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Negate,
    Not,
    BitwiseNot,
    Reference,
    MutableReference,
    Dereference,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_var_generation() {
        let mut gen = ConstraintGenerator::new();
        let v1 = gen.fresh_var();
        let v2 = gen.fresh_var();
        let v3 = gen.fresh_var();

        assert_eq!(v1, TypeVar(0));
        assert_eq!(v2, TypeVar(1));
        assert_eq!(v3, TypeVar(2));
    }

    #[test]
    fn test_add_constraint() {
        let mut gen = ConstraintGenerator::new();
        gen.add_constraint(Type::I32, Type::I32);
        gen.add_constraint(Type::I64, Type::I64);

        assert_eq!(gen.constraints.len(), 2);
        assert_eq!(gen.constraints[0], Constraint::new(Type::I32, Type::I32));
    }

    #[test]
    fn test_symbol_registration() {
        let mut gen = ConstraintGenerator::new();
        let var = TypeVar(0);

        gen.register_var("x".to_string(), var);
        assert_eq!(gen.lookup_var("x"), Some(var));
        assert_eq!(gen.lookup_var("y"), None);
    }

    #[test]
    fn test_binary_op_add() {
        let mut gen = ConstraintGenerator::new();
        let result = gen.constrain_binary_op(BinaryOp::Add, Type::I32, Type::I32).unwrap();
        
        // Result should be a type variable
        assert!(matches!(result, Type::Variable(_)));
        
        // Should have constraints
        assert!(!gen.constraints.is_empty());
    }

    #[test]
    fn test_binary_op_comparison() {
        let mut gen = ConstraintGenerator::new();
        let result = gen.constrain_binary_op(BinaryOp::Less, Type::I32, Type::I32).unwrap();
        
        // Comparison returns bool
        assert_eq!(result, Type::Bool);
    }

    #[test]
    fn test_unary_op_reference() {
        let mut gen = ConstraintGenerator::new();
        let result = gen.constrain_unary_op(UnaryOp::Reference, Type::I32).unwrap();
        
        assert!(matches!(result, Type::Reference { .. }));
    }

    #[test]
    fn test_function_call_constraint() {
        let mut gen = ConstraintGenerator::new();
        
        // Register a function: fn add(i32, i32) -> i32
        gen.register_function(
            "add".to_string(),
            vec![Type::I32, Type::I32],
            Type::I32,
        );

        // Call with matching types
        let result = gen.constrain_function_call("add", vec![Type::I32, Type::I32]).unwrap();
        assert_eq!(result, Type::I32);
    }

    #[test]
    fn test_function_call_wrong_arity() {
        let mut gen = ConstraintGenerator::new();
        
        gen.register_function(
            "add".to_string(),
            vec![Type::I32, Type::I32],
            Type::I32,
        );

        // Call with wrong number of arguments
        let result = gen.constrain_function_call("add", vec![Type::I32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_struct_registration() {
        let mut gen = ConstraintGenerator::new();
        let struct_id = StructId(0);
        let mut fields = HashMap::new();
        fields.insert("x".to_string(), Type::I32);
        fields.insert("y".to_string(), Type::I32);

        gen.register_struct(struct_id, fields);

        assert!(gen.struct_defs.contains_key("struct_0"));
    }

    #[test]
    fn test_solve_simple_constraints() {
        let mut gen = ConstraintGenerator::new();
        
        // Create constraint: X = i32
        gen.add_constraint(Type::Variable(TypeVar(0)), Type::I32);
        
        let subst = gen.solve().unwrap();
        assert_eq!(subst.apply(&Type::Variable(TypeVar(0))), Type::I32);
    }

    #[test]
    fn test_solve_contradictory_constraints() {
        let mut gen = ConstraintGenerator::new();
        
        // Create contradictory constraints: X = i32, X = bool
        gen.add_constraint(Type::Variable(TypeVar(0)), Type::I32);
        gen.add_constraint(Type::Variable(TypeVar(0)), Type::Bool);
        
        let result = gen.solve();
        assert!(result.is_err());
    }

    #[test]
    fn test_constrain_logical_and() {
        let mut gen = ConstraintGenerator::new();
        let result = gen.constrain_binary_op(BinaryOp::And, Type::Bool, Type::Bool).unwrap();
        
        assert_eq!(result, Type::Bool);
    }

    #[test]
    fn test_constrain_bitwise_ops() {
        let mut gen = ConstraintGenerator::new();
        
        let result = gen.constrain_binary_op(BinaryOp::BitwiseAnd, Type::I32, Type::I32).unwrap();
        assert_eq!(result, Type::I32);
        
        let result = gen.constrain_binary_op(BinaryOp::BitwiseOr, Type::I64, Type::I64).unwrap();
        assert_eq!(result, Type::I64);
    }
}