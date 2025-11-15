use std::collections::{HashMap, HashSet};
use crate::lowering::{HirExpression, HirStatement, HirType, ClosureTrait};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureKind {
    ByValue,
    ByRef,
    ByMutRef,
}

#[derive(Debug, Clone)]
pub struct ClosureInfo {
    pub id: usize,
    pub params: Vec<(String, HirType)>,
    pub return_type: HirType,
    pub body: Vec<HirStatement>,
    pub captures: HashMap<String, (CaptureKind, HirType)>,
    pub is_move: bool,
    pub fn_trait: FnTrait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnTrait {
    Fn,
    FnMut,
    FnOnce,
}

impl FnTrait {
    pub fn to_string(&self) -> &'static str {
        match self {
            FnTrait::Fn => "Fn",
            FnTrait::FnMut => "FnMut",
            FnTrait::FnOnce => "FnOnce",
        }
    }

    pub fn can_call_immutably(&self) -> bool {
        matches!(self, FnTrait::Fn)
    }

    pub fn can_call_mutably(&self) -> bool {
        matches!(self, FnTrait::Fn | FnTrait::FnMut)
    }

    pub fn can_call_by_value(&self) -> bool {
        true
    }

    pub fn consumes_on_call(&self) -> bool {
        matches!(self, FnTrait::FnOnce)
    }
}

pub struct ClosureInvocationTracker {
    pub closures: HashMap<usize, ClosureInfo>,
    pub closure_call_count: HashMap<usize, usize>,
}

impl ClosureInvocationTracker {
    pub fn new() -> Self {
        ClosureInvocationTracker {
            closures: HashMap::new(),
            closure_call_count: HashMap::new(),
        }
    }

    pub fn register_closure(&mut self, closure: ClosureInfo) {
        let id = closure.id;
        self.closures.insert(id, closure);
        self.closure_call_count.insert(id, 0);
    }

    pub fn record_call(&mut self, closure_id: usize) -> Result<(), String> {
        let closure = self.closures.get(&closure_id)
            .ok_or_else(|| format!("Closure {} not found", closure_id))?;

        if closure.fn_trait == FnTrait::FnOnce {
            let call_count = self.closure_call_count.get(&closure_id).unwrap_or(&0);
            if *call_count > 0 {
                return Err(format!("FnOnce closure {} already called", closure_id));
            }
        }

        *self.closure_call_count.entry(closure_id).or_insert(0) += 1;
        Ok(())
    }

    pub fn get_closure(&self, closure_id: usize) -> Option<&ClosureInfo> {
        self.closures.get(&closure_id)
    }
}

pub struct CaptureAnalyzer {
    next_closure_id: usize,
}

impl CaptureAnalyzer {
    pub fn new() -> Self {
        CaptureAnalyzer {
            next_closure_id: 0,
        }
    }

    pub fn analyze_closure(
        &mut self,
        params: Vec<(String, HirType)>,
        body: &[HirStatement],
        return_type: HirType,
        is_move: bool,
        available_bindings: &HashMap<String, HirType>,
    ) -> ClosureInfo {
        let closure_id = self.next_closure_id;
        self.next_closure_id += 1;

        let mut captures = HashMap::new();
        let mut used_vars = HashSet::new();

        self.collect_used_vars(body, &mut used_vars);

        for (param_name, _) in &params {
            used_vars.remove(param_name);
        }

        for var_name in used_vars {
            if let Some(var_type) = available_bindings.get(&var_name) {
                let capture_kind = self.determine_capture_kind(&var_name, body, is_move);
                captures.insert(var_name, (capture_kind, var_type.clone()));
            }
        }

        let fn_trait = self.determine_fn_trait(&captures, is_move);

        ClosureInfo {
            id: closure_id,
            params,
            return_type,
            body: body.to_vec(),
            captures,
            is_move,
            fn_trait,
        }
    }

    fn collect_used_vars(&self, statements: &[HirStatement], used_vars: &mut HashSet<String>) {
        for stmt in statements {
            self.collect_vars_from_stmt(stmt, used_vars);
        }
    }

    fn collect_vars_from_stmt(&self, stmt: &HirStatement, used_vars: &mut HashSet<String>) {
        match stmt {
            HirStatement::Expression(expr) => {
                self.collect_vars_from_expr(expr, used_vars);
            }
            HirStatement::Return(Some(expr)) => {
                self.collect_vars_from_expr(expr, used_vars);
            }
            _ => {}
        }
    }

    fn collect_vars_from_expr(&self, expr: &HirExpression, used_vars: &mut HashSet<String>) {
        match expr {
            HirExpression::Variable(name) => {
                used_vars.insert(name.clone());
            }
            HirExpression::BinaryOp { left, right, .. } => {
                self.collect_vars_from_expr(left, used_vars);
                self.collect_vars_from_expr(right, used_vars);
            }
            HirExpression::UnaryOp { operand, .. } => {
                self.collect_vars_from_expr(operand, used_vars);
            }
            HirExpression::Call { func, args } => {
                self.collect_vars_from_expr(func, used_vars);
                for arg in args {
                    self.collect_vars_from_expr(arg, used_vars);
                }
            }
            HirExpression::FieldAccess { object, .. } => {
                self.collect_vars_from_expr(object, used_vars);
            }
            HirExpression::Index { array, index } => {
                self.collect_vars_from_expr(array, used_vars);
                self.collect_vars_from_expr(index, used_vars);
            }
            HirExpression::Assign { target, value } => {
                self.collect_vars_from_expr(target, used_vars);
                self.collect_vars_from_expr(value, used_vars);
            }
            HirExpression::If {
                condition,
                then_body,
                else_body,
            } => {
                self.collect_vars_from_expr(condition, used_vars);
                self.collect_used_vars(then_body, used_vars);
                if let Some(else_stmts) = else_body {
                    self.collect_used_vars(else_stmts, used_vars);
                }
            }
            HirExpression::While { condition, body } => {
                self.collect_vars_from_expr(condition, used_vars);
                self.collect_used_vars(body, used_vars);
            }
            HirExpression::Match { scrutinee, arms } => {
                self.collect_vars_from_expr(scrutinee, used_vars);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.collect_vars_from_expr(guard, used_vars);
                    }
                    self.collect_used_vars(&arm.body, used_vars);
                }
            }
            HirExpression::Closure { .. } => {
            }
            _ => {}
        }
    }

    fn determine_capture_kind(
        &self,
        _var_name: &str,
        body: &[HirStatement],
        is_move: bool,
    ) -> CaptureKind {
        if is_move {
            return CaptureKind::ByValue;
        }

        let mut is_mutated = false;
        self.check_mutation(body, &mut is_mutated);

        if is_mutated {
            CaptureKind::ByMutRef
        } else {
            CaptureKind::ByRef
        }
    }

    fn check_mutation(&self, statements: &[HirStatement], is_mutated: &mut bool) {
        if *is_mutated {
            return;
        }

        for stmt in statements {
            match stmt {
                HirStatement::Expression(HirExpression::Assign { .. }) => {
                    *is_mutated = true;
                    return;
                }
                HirStatement::Expression(HirExpression::While { body, .. }) => {
                    self.check_mutation(body, is_mutated);
                }
                HirStatement::Expression(HirExpression::If { then_body, else_body, .. }) => {
                    self.check_mutation(then_body, is_mutated);
                    if let Some(else_stmts) = else_body {
                        self.check_mutation(else_stmts, is_mutated);
                    }
                }
                _ => {}
            }
        }
    }

    fn determine_fn_trait(&self, captures: &HashMap<String, (CaptureKind, HirType)>, is_move: bool) -> FnTrait {
        let has_mut_captures = captures.values().any(|(kind, _)| *kind == CaptureKind::ByMutRef);

        if is_move || captures.values().any(|(kind, _)| *kind == CaptureKind::ByValue) {
            if has_mut_captures {
                FnTrait::FnMut
            } else if is_move {
                FnTrait::FnOnce
            } else {
                FnTrait::Fn
            }
        } else if has_mut_captures {
            FnTrait::FnMut
        } else {
            FnTrait::Fn
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_capture_by_ref() {
        let mut analyzer = CaptureAnalyzer::new();
        let params = vec![("x".to_string(), HirType::Int32)];
        let body = vec![HirStatement::Expression(HirExpression::Variable("y".to_string()))];
        let mut bindings = HashMap::new();
        bindings.insert("y".to_string(), HirType::Int32);

        let closure = analyzer.analyze_closure(
            params,
            &body,
            HirType::Int32,
            false,
            &bindings,
        );

        assert_eq!(closure.fn_trait, FnTrait::Fn);
        assert_eq!(closure.captures.len(), 1);
        assert_eq!(
            closure.captures.get("y").unwrap().0,
            CaptureKind::ByRef
        );
    }

    #[test]
    fn test_move_closure() {
        let mut analyzer = CaptureAnalyzer::new();
        let params = vec![("x".to_string(), HirType::Int32)];
        let body = vec![HirStatement::Expression(HirExpression::Variable("y".to_string()))];
        let mut bindings = HashMap::new();
        bindings.insert("y".to_string(), HirType::Int32);

        let closure = analyzer.analyze_closure(
            params,
            &body,
            HirType::Int32,
            true,
            &bindings,
        );

        assert_eq!(closure.fn_trait, FnTrait::FnOnce);
        assert_eq!(closure.captures.len(), 1);
        assert_eq!(closure.captures.get("y").unwrap().0, CaptureKind::ByValue);
    }

    #[test]
    fn test_closure_with_mutation() {
        let mut analyzer = CaptureAnalyzer::new();
        let params = vec![("x".to_string(), HirType::Int32)];
        let body = vec![
            HirStatement::Expression(HirExpression::Assign {
                target: Box::new(HirExpression::Variable("y".to_string())),
                value: Box::new(HirExpression::Integer(5)),
            }),
        ];
        let mut bindings = HashMap::new();
        bindings.insert("y".to_string(), HirType::Int32);

        let closure = analyzer.analyze_closure(
            params,
            &body,
            HirType::Int32,
            false,
            &bindings,
        );

        assert_eq!(closure.fn_trait, FnTrait::FnMut);
        assert_eq!(closure.captures.len(), 1);
        assert_eq!(
            closure.captures.get("y").unwrap().0,
            CaptureKind::ByMutRef
        );
    }

    #[test]
    fn test_closure_id_increment() {
        let mut analyzer = CaptureAnalyzer::new();
        let params = vec![];
        let body = vec![];
        let bindings = HashMap::new();

        let closure1 = analyzer.analyze_closure(
            params.clone(),
            &body,
            HirType::Int32,
            false,
            &bindings,
        );
        let closure2 = analyzer.analyze_closure(
            params,
            &body,
            HirType::Int32,
            false,
            &bindings,
        );

        assert_eq!(closure1.id, 0);
        assert_eq!(closure2.id, 1);
    }

    #[test]
    fn test_fn_trait_call_permissions() {
        let fn_trait = FnTrait::Fn;
        assert!(fn_trait.can_call_immutably());
        assert!(fn_trait.can_call_mutably());
        assert!(fn_trait.can_call_by_value());
        assert!(!fn_trait.consumes_on_call());

        let fn_mut_trait = FnTrait::FnMut;
        assert!(!fn_mut_trait.can_call_immutably());
        assert!(fn_mut_trait.can_call_mutably());
        assert!(fn_mut_trait.can_call_by_value());
        assert!(!fn_mut_trait.consumes_on_call());

        let fn_once_trait = FnTrait::FnOnce;
        assert!(!fn_once_trait.can_call_immutably());
        assert!(!fn_once_trait.can_call_mutably());
        assert!(fn_once_trait.can_call_by_value());
        assert!(fn_once_trait.consumes_on_call());
    }

    #[test]
    fn test_closure_invocation_tracker_registration() {
        let mut tracker = ClosureInvocationTracker::new();
        let closure = ClosureInfo {
            id: 0,
            params: vec![],
            return_type: HirType::Int32,
            body: vec![],
            captures: HashMap::new(),
            is_move: false,
            fn_trait: FnTrait::Fn,
        };

        tracker.register_closure(closure);
        assert!(tracker.get_closure(0).is_some());
        assert_eq!(tracker.get_closure(0).unwrap().id, 0);
    }

    #[test]
    fn test_fn_once_only_callable_once() {
        let mut tracker = ClosureInvocationTracker::new();
        let closure = ClosureInfo {
            id: 0,
            params: vec![],
            return_type: HirType::Int32,
            body: vec![],
            captures: HashMap::new(),
            is_move: true,
            fn_trait: FnTrait::FnOnce,
        };

        tracker.register_closure(closure);

        assert!(tracker.record_call(0).is_ok());
        assert!(tracker.record_call(0).is_err());
    }

    #[test]
    fn test_fn_callable_multiple_times() {
        let mut tracker = ClosureInvocationTracker::new();
        let closure = ClosureInfo {
            id: 0,
            params: vec![],
            return_type: HirType::Int32,
            body: vec![],
            captures: HashMap::new(),
            is_move: false,
            fn_trait: FnTrait::Fn,
        };

        tracker.register_closure(closure);

        assert!(tracker.record_call(0).is_ok());
        assert!(tracker.record_call(0).is_ok());
        assert!(tracker.record_call(0).is_ok());
    }

    #[test]
    fn test_higher_order_closure() {
        let mut analyzer = CaptureAnalyzer::new();
        
        let inner_closure_type = HirType::Closure {
            params: vec![HirType::Int32],
            return_type: Box::new(HirType::Int32),
            trait_kind: ClosureTrait::Fn,
        };
        
        let params = vec![("f".to_string(), inner_closure_type)];
        let body = vec![HirStatement::Expression(HirExpression::Integer(42))];
        let bindings = HashMap::new();

        let closure = analyzer.analyze_closure(
            params,
            &body,
            HirType::Int32,
            false,
            &bindings,
        );

        assert_eq!(closure.id, 0);
        assert_eq!(closure.fn_trait, FnTrait::Fn);
    }

    #[test]
    fn test_closure_capturing_another_closure() {
        let mut analyzer = CaptureAnalyzer::new();
        let params = vec![];
        let body = vec![HirStatement::Expression(HirExpression::Variable("inner_closure".to_string()))];
        
        let mut bindings = HashMap::new();
        bindings.insert("inner_closure".to_string(), HirType::Closure {
            params: vec![HirType::Int32],
            return_type: Box::new(HirType::Int32),
            trait_kind: ClosureTrait::Fn,
        });

        let closure = analyzer.analyze_closure(
            params,
            &body,
            HirType::Int32,
            false,
            &bindings,
        );

        assert!(closure.captures.contains_key("inner_closure"));
        assert_eq!(closure.captures.get("inner_closure").unwrap().0, CaptureKind::ByRef);
    }

    #[test]
    fn test_nested_closure_move_semantics() {
        let mut analyzer = CaptureAnalyzer::new();
        let params = vec![];
        let body = vec![HirStatement::Expression(HirExpression::Variable("x".to_string()))];
        
        let mut bindings = HashMap::new();
        bindings.insert("x".to_string(), HirType::Int32);

        let outer_closure = analyzer.analyze_closure(
            params.clone(),
            &body,
            HirType::Int32,
            true,
            &bindings,
        );

        assert!(outer_closure.is_move);
        assert_eq!(outer_closure.fn_trait, FnTrait::FnOnce);
        assert!(outer_closure.captures.contains_key("x"));
        assert_eq!(outer_closure.captures.get("x").unwrap().0, CaptureKind::ByValue);
    }

    #[test]
    fn test_multiple_closure_instances_independent() {
        let mut analyzer = CaptureAnalyzer::new();
        let mut tracker1 = ClosureInvocationTracker::new();
        let mut tracker2 = ClosureInvocationTracker::new();
        
        let closure1 = analyzer.analyze_closure(
            vec![],
            &vec![],
            HirType::Int32,
            true,
            &HashMap::new(),
        );
        
        let closure2 = analyzer.analyze_closure(
            vec![],
            &vec![],
            HirType::Int32,
            true,
            &HashMap::new(),
        );
        
        let closure1_id = closure1.id;
        let closure2_id = closure2.id;
        
        tracker1.register_closure(closure1);
        tracker2.register_closure(closure2);
        
        assert!(tracker1.record_call(closure1_id).is_ok());
        assert!(tracker1.record_call(closure1_id).is_err());
        
        assert!(tracker2.record_call(closure2_id).is_ok());
        assert!(tracker2.record_call(closure2_id).is_err());
    }
}
