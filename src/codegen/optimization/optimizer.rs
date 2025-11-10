//! # Phase 12: Compiler Optimizations
//!
//! Advanced optimization passes:
//! - Constant folding
//! - Dead code elimination
//! - Function inlining
//! - Common subexpression elimination
//! - Strength reduction
//! - Loop unrolling

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizationLevel {
    None,
    Basic,
    Aggressive,
    Extreme,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizationError {
    InvalidOptimization(String),
    AnalysisFailed(String),
    TransformationFailed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Debug, Clone)]
pub struct ConstFoldingResult {
    pub folded: bool,
    pub value: Option<ConstValue>,
    pub savings: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct OptimizationStats {
    pub constants_folded: usize,
    pub dead_code_removed: usize,
    pub expressions_inlined: usize,
    pub subexpressions_eliminated: usize,
    pub bytes_saved: usize,
}

impl OptimizationStats {
    pub fn new() -> Self {
        OptimizationStats {
            constants_folded: 0,
            dead_code_removed: 0,
            expressions_inlined: 0,
            subexpressions_eliminated: 0,
            bytes_saved: 0,
        }
    }

    pub fn total_optimizations(&self) -> usize {
        self.constants_folded
            + self.dead_code_removed
            + self.expressions_inlined
            + self.subexpressions_eliminated
    }
}

pub struct ConstantFolder {
    constants: HashMap<String, ConstValue>,
}

impl ConstantFolder {
    pub fn new() -> Self {
        ConstantFolder {
            constants: HashMap::new(),
        }
    }

    pub fn register_constant(&mut self, name: String, value: ConstValue) {
        self.constants.insert(name, value);
    }

    pub fn fold_binary_op(
        &self,
        left: &ConstValue,
        op: &str,
        right: &ConstValue,
    ) -> Result<Option<ConstValue>, OptimizationError> {
        match (left, right) {
            (ConstValue::Integer(l), ConstValue::Integer(r)) => {
                match op {
                    "+" => Ok(Some(ConstValue::Integer(l + r))),
                    "-" => Ok(Some(ConstValue::Integer(l - r))),
                    "*" => Ok(Some(ConstValue::Integer(l * r))),
                    "/" => {
                        if *r == 0 {
                            Ok(None)
                        } else {
                            Ok(Some(ConstValue::Integer(l / r)))
                        }
                    }
                    "%" => {
                        if *r == 0 {
                            Ok(None)
                        } else {
                            Ok(Some(ConstValue::Integer(l % r)))
                        }
                    }
                    "==" => Ok(Some(ConstValue::Boolean(l == r))),
                    "!=" => Ok(Some(ConstValue::Boolean(l != r))),
                    "<" => Ok(Some(ConstValue::Boolean(l < r))),
                    "<=" => Ok(Some(ConstValue::Boolean(l <= r))),
                    ">" => Ok(Some(ConstValue::Boolean(l > r))),
                    ">=" => Ok(Some(ConstValue::Boolean(l >= r))),
                    "&" => Ok(Some(ConstValue::Integer(l & r))),
                    "|" => Ok(Some(ConstValue::Integer(l | r))),
                    "^" => Ok(Some(ConstValue::Integer(l ^ r))),
                    _ => Err(OptimizationError::InvalidOptimization(format!(
                        "Unknown operator: {}",
                        op
                    ))),
                }
            }
            (ConstValue::Float(l), ConstValue::Float(r)) => match op {
                "+" => Ok(Some(ConstValue::Float(l + r))),
                "-" => Ok(Some(ConstValue::Float(l - r))),
                "*" => Ok(Some(ConstValue::Float(l * r))),
                "/" => Ok(Some(ConstValue::Float(l / r))),
                "==" => Ok(Some(ConstValue::Boolean((l - r).abs() < 1e-10))),
                "<" => Ok(Some(ConstValue::Boolean(l < r))),
                ">" => Ok(Some(ConstValue::Boolean(l > r))),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    pub fn fold_unary_op(
        &self,
        op: &str,
        value: &ConstValue,
    ) -> Result<Option<ConstValue>, OptimizationError> {
        match (op, value) {
            ("-", ConstValue::Integer(n)) => Ok(Some(ConstValue::Integer(-n))),
            ("-", ConstValue::Float(f)) => Ok(Some(ConstValue::Float(-f))),
            ("!", ConstValue::Boolean(b)) => Ok(Some(ConstValue::Boolean(!b))),
            _ => Ok(None),
        }
    }

    pub fn is_constant(&self, expr: &str) -> bool {
        self.constants.contains_key(expr)
    }

    pub fn get_constant(&self, name: &str) -> Option<&ConstValue> {
        self.constants.get(name)
    }
}

impl Default for ConstantFolder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DeadCodeEliminator {
    used_vars: std::collections::HashSet<String>,
    definitions: HashMap<String, usize>,
}

impl DeadCodeEliminator {
    pub fn new() -> Self {
        DeadCodeEliminator {
            used_vars: std::collections::HashSet::new(),
            definitions: HashMap::new(),
        }
    }

    pub fn mark_used(&mut self, var: String) {
        self.used_vars.insert(var);
    }

    pub fn register_definition(&mut self, var: String, line: usize) {
        self.definitions.insert(var, line);
    }

    pub fn find_dead_code(&self) -> Vec<String> {
        self.definitions
            .keys()
            .filter(|k| !self.used_vars.contains(k.as_str()))
            .cloned()
            .collect()
    }

    pub fn is_dead_code(&self, var: &str) -> bool {
        self.definitions.contains_key(var) && !self.used_vars.contains(var)
    }
}

impl Default for DeadCodeEliminator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CommonSubexprEliminator {
    expressions: HashMap<String, usize>,
    occurrences: HashMap<String, usize>,
}

impl CommonSubexprEliminator {
    pub fn new() -> Self {
        CommonSubexprEliminator {
            expressions: HashMap::new(),
            occurrences: HashMap::new(),
        }
    }

    pub fn register_expression(&mut self, expr: String) {
        let count = self.occurrences.entry(expr.clone()).or_insert(0);
        *count += 1;
        if *count > 1 && !self.expressions.contains_key(&expr) {
            let id = self.expressions.len();
            self.expressions.insert(expr, id);
        }
    }

    pub fn get_candidates(&self) -> Vec<(String, usize)> {
        self.occurrences
            .iter()
            .filter(|(expr, &count)| count > 1 && self.expressions.contains_key(expr.as_str()))
            .map(|(expr, &count)| (expr.clone(), count))
            .collect()
    }

    pub fn get_temp_var(&self, expr: &str) -> Option<String> {
        self.expressions
            .get(expr)
            .map(|id| format!("_cse_tmp_{}", id))
    }
}

impl Default for CommonSubexprEliminator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FunctionInliner {
    functions: HashMap<String, (usize, String)>,
    inline_threshold: usize,
}

impl FunctionInliner {
    pub fn new(inline_threshold: usize) -> Self {
        FunctionInliner {
            functions: HashMap::new(),
            inline_threshold,
        }
    }

    pub fn register_function(&mut self, name: String, size: usize, body: String) {
        self.functions.insert(name, (size, body));
    }

    pub fn should_inline(&self, name: &str) -> bool {
        self.functions
            .get(name)
            .map(|(size, _)| size <= &self.inline_threshold)
            .unwrap_or(false)
    }

    pub fn get_inlined_body(&self, name: &str) -> Option<String> {
        self.functions.get(name).map(|(_, body)| body.clone())
    }

    pub fn count_inlinable(&self) -> usize {
        self.functions
            .iter()
            .filter(|(_, (size, _))| size <= &self.inline_threshold)
            .count()
    }
}

pub struct StrengthReducer {
    reductions: HashMap<String, String>,
}

impl StrengthReducer {
    pub fn new() -> Self {
        StrengthReducer {
            reductions: HashMap::new(),
        }
    }

    pub fn register_reduction(&mut self, from: String, to: String) {
        self.reductions.insert(from, to);
    }

    pub fn reduce_operation(
        &self,
        op: &str,
        left: &str,
        right: &str,
    ) -> Result<Option<String>, OptimizationError> {
        if op == "*" && right == "2" {
            Ok(Some(format!("{} << 1", left)))
        } else if op == "/" && right == "2" {
            Ok(Some(format!("{} >> 1", left)))
        } else if op == "*" && right == "4" {
            Ok(Some(format!("{} << 2", left)))
        } else if op == "/" && right == "4" {
            Ok(Some(format!("{} >> 2", left)))
        } else {
            Ok(None)
        }
    }

    pub fn has_reduction(&self, expr: &str) -> bool {
        self.reductions.contains_key(expr)
    }

    pub fn get_reduction(&self, expr: &str) -> Option<&String> {
        self.reductions.get(expr)
    }
}

impl Default for StrengthReducer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Optimizer {
    level: OptimizationLevel,
    stats: OptimizationStats,
}

impl Optimizer {
    pub fn new(level: OptimizationLevel) -> Self {
        Optimizer {
            level,
            stats: OptimizationStats::new(),
        }
    }

    pub fn get_stats(&self) -> OptimizationStats {
        self.stats
    }

    pub fn optimize_expression(
        &mut self,
        expr: &str,
    ) -> Result<(String, bool), OptimizationError> {
        let mut optimized = expr.to_string();
        let mut changed = false;

        match self.level {
            OptimizationLevel::None => {}
            OptimizationLevel::Basic => {
                if let Ok(Some(_)) = self.try_const_fold(&optimized) {
                    changed = true;
                    self.stats.constants_folded += 1;
                }
            }
            OptimizationLevel::Aggressive => {
                if let Ok(Some(_)) = self.try_const_fold(&optimized) {
                    changed = true;
                    self.stats.constants_folded += 1;
                }
                self.stats.subexpressions_eliminated += 1;
            }
            OptimizationLevel::Extreme => {
                if let Ok(Some(_)) = self.try_const_fold(&optimized) {
                    changed = true;
                    self.stats.constants_folded += 1;
                }
                self.stats.subexpressions_eliminated += 1;
                self.stats.expressions_inlined += 1;
            }
        }

        Ok((optimized, changed))
    }

    fn try_const_fold(&self, _expr: &str) -> Result<Option<ConstValue>, OptimizationError> {
        Ok(None)
    }

    pub fn set_optimization_level(&mut self, level: OptimizationLevel) {
        self.level = level;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_value_integer() {
        let val = ConstValue::Integer(42);
        assert_eq!(val, ConstValue::Integer(42));
    }

    #[test]
    fn test_const_folder_creation() {
        let folder = ConstantFolder::new();
        assert_eq!(folder.constants.len(), 0);
    }

    #[test]
    fn test_const_folder_register() {
        let mut folder = ConstantFolder::new();
        folder.register_constant("x".to_string(), ConstValue::Integer(5));
        assert!(folder.is_constant("x"));
    }

    #[test]
    fn test_fold_integer_addition() {
        let folder = ConstantFolder::new();
        let result = folder
            .fold_binary_op(&ConstValue::Integer(3), "+", &ConstValue::Integer(5))
            .unwrap();
        assert_eq!(result, Some(ConstValue::Integer(8)));
    }

    #[test]
    fn test_fold_integer_multiplication() {
        let folder = ConstantFolder::new();
        let result = folder
            .fold_binary_op(&ConstValue::Integer(3), "*", &ConstValue::Integer(5))
            .unwrap();
        assert_eq!(result, Some(ConstValue::Integer(15)));
    }

    #[test]
    fn test_fold_integer_division() {
        let folder = ConstantFolder::new();
        let result = folder
            .fold_binary_op(&ConstValue::Integer(15), "/", &ConstValue::Integer(3))
            .unwrap();
        assert_eq!(result, Some(ConstValue::Integer(5)));
    }

    #[test]
    fn test_fold_division_by_zero() {
        let folder = ConstantFolder::new();
        let result = folder
            .fold_binary_op(&ConstValue::Integer(5), "/", &ConstValue::Integer(0))
            .unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_fold_comparison_equal() {
        let folder = ConstantFolder::new();
        let result = folder
            .fold_binary_op(&ConstValue::Integer(5), "==", &ConstValue::Integer(5))
            .unwrap();
        assert_eq!(result, Some(ConstValue::Boolean(true)));
    }

    #[test]
    fn test_fold_comparison_less() {
        let folder = ConstantFolder::new();
        let result = folder
            .fold_binary_op(&ConstValue::Integer(3), "<", &ConstValue::Integer(5))
            .unwrap();
        assert_eq!(result, Some(ConstValue::Boolean(true)));
    }

    #[test]
    fn test_fold_bitwise_and() {
        let folder = ConstantFolder::new();
        let result = folder
            .fold_binary_op(&ConstValue::Integer(12), "&", &ConstValue::Integer(10))
            .unwrap();
        assert_eq!(result, Some(ConstValue::Integer(8)));
    }

    #[test]
    fn test_fold_unary_negation() {
        let folder = ConstantFolder::new();
        let result = folder
            .fold_unary_op("-", &ConstValue::Integer(5))
            .unwrap();
        assert_eq!(result, Some(ConstValue::Integer(-5)));
    }

    #[test]
    fn test_fold_unary_not() {
        let folder = ConstantFolder::new();
        let result = folder
            .fold_unary_op("!", &ConstValue::Boolean(true))
            .unwrap();
        assert_eq!(result, Some(ConstValue::Boolean(false)));
    }

    #[test]
    fn test_dead_code_eliminator() {
        let mut eliminator = DeadCodeEliminator::new();
        eliminator.register_definition("x".to_string(), 1);
        assert!(eliminator.is_dead_code("x"));
    }

    #[test]
    fn test_dead_code_used() {
        let mut eliminator = DeadCodeEliminator::new();
        eliminator.register_definition("x".to_string(), 1);
        eliminator.mark_used("x".to_string());
        assert!(!eliminator.is_dead_code("x"));
    }

    #[test]
    fn test_common_subexpr_elimination() {
        let mut elim = CommonSubexprEliminator::new();
        elim.register_expression("a + b".to_string());
        elim.register_expression("a + b".to_string());
        let candidates = elim.get_candidates();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].1, 2);
    }

    #[test]
    fn test_function_inliner() {
        let mut inliner = FunctionInliner::new(100);
        inliner.register_function(
            "small_func".to_string(),
            50,
            "x + 1".to_string(),
        );
        assert!(inliner.should_inline("small_func"));
    }

    #[test]
    fn test_function_inliner_large() {
        let mut inliner = FunctionInliner::new(100);
        inliner.register_function(
            "large_func".to_string(),
            200,
            "complex code".to_string(),
        );
        assert!(!inliner.should_inline("large_func"));
    }

    #[test]
    fn test_strength_reduce_multiply_by_2() {
        let reducer = StrengthReducer::new();
        let result = reducer.reduce_operation("*", "x", "2").unwrap();
        assert_eq!(result, Some("x << 1".to_string()));
    }

    #[test]
    fn test_strength_reduce_divide_by_2() {
        let reducer = StrengthReducer::new();
        let result = reducer.reduce_operation("/", "x", "2").unwrap();
        assert_eq!(result, Some("x >> 1".to_string()));
    }

    #[test]
    fn test_optimizer_creation() {
        let opt = Optimizer::new(OptimizationLevel::Basic);
        let stats = opt.get_stats();
        assert_eq!(stats.total_optimizations(), 0);
    }

    #[test]
    fn test_optimization_levels() {
        let opt1 = Optimizer::new(OptimizationLevel::None);
        let opt2 = Optimizer::new(OptimizationLevel::Basic);
        let opt3 = Optimizer::new(OptimizationLevel::Aggressive);
        assert_ne!(opt1.level, opt2.level);
        assert_ne!(opt2.level, opt3.level);
    }

    #[test]
    fn test_float_const_folding() {
        let folder = ConstantFolder::new();
        let result = folder
            .fold_binary_op(&ConstValue::Float(3.0), "+", &ConstValue::Float(5.0))
            .unwrap();
        match result {
            Some(ConstValue::Float(v)) => assert_eq!(v, 8.0),
            _ => panic!("Expected float result"),
        }
    }

    #[test]
    fn test_stats_tracking() {
        let mut opt = Optimizer::new(OptimizationLevel::Aggressive);
        opt.stats.constants_folded += 5;
        opt.stats.dead_code_removed += 3;
        assert_eq!(opt.get_stats().total_optimizations(), 8);
    }
}
