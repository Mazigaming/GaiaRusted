
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Null,
}

impl ConstantValue {
    pub fn add(&self, other: &ConstantValue) -> Option<ConstantValue> {
        match (self, other) {
            (ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                Some(ConstantValue::Integer(a + b))
            }
            (ConstantValue::Float(a), ConstantValue::Float(b)) => {
                Some(ConstantValue::Float(a + b))
            }
            _ => None,
        }
    }

    pub fn sub(&self, other: &ConstantValue) -> Option<ConstantValue> {
        match (self, other) {
            (ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                Some(ConstantValue::Integer(a - b))
            }
            (ConstantValue::Float(a), ConstantValue::Float(b)) => {
                Some(ConstantValue::Float(a - b))
            }
            _ => None,
        }
    }

    pub fn mul(&self, other: &ConstantValue) -> Option<ConstantValue> {
        match (self, other) {
            (ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                Some(ConstantValue::Integer(a * b))
            }
            (ConstantValue::Float(a), ConstantValue::Float(b)) => {
                Some(ConstantValue::Float(a * b))
            }
            _ => None,
        }
    }

    pub fn div(&self, other: &ConstantValue) -> Option<ConstantValue> {
        match (self, other) {
            (ConstantValue::Integer(a), ConstantValue::Integer(b)) if *b != 0 => {
                Some(ConstantValue::Integer(a / b))
            }
            (ConstantValue::Float(a), ConstantValue::Float(b)) if *b != 0.0 => {
                Some(ConstantValue::Float(a / b))
            }
            _ => None,
        }
    }

    pub fn modulo(&self, other: &ConstantValue) -> Option<ConstantValue> {
        match (self, other) {
            (ConstantValue::Integer(a), ConstantValue::Integer(b)) if *b != 0 => {
                Some(ConstantValue::Integer(a % b))
            }
            _ => None,
        }
    }

    pub fn bitwise_and(&self, other: &ConstantValue) -> Option<ConstantValue> {
        match (self, other) {
            (ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                Some(ConstantValue::Integer(a & b))
            }
            _ => None,
        }
    }

    pub fn bitwise_or(&self, other: &ConstantValue) -> Option<ConstantValue> {
        match (self, other) {
            (ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                Some(ConstantValue::Integer(a | b))
            }
            _ => None,
        }
    }

    pub fn bitwise_xor(&self, other: &ConstantValue) -> Option<ConstantValue> {
        match (self, other) {
            (ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                Some(ConstantValue::Integer(a ^ b))
            }
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            ConstantValue::Integer(n) => n.to_string(),
            ConstantValue::Float(f) => f.to_string(),
            ConstantValue::Boolean(b) => b.to_string(),
            ConstantValue::String(s) => format!("\"{}\"", s),
            ConstantValue::Null => "null".to_string(),
        }
    }
}

pub struct ConstantPropagator {
    constants: HashMap<String, ConstantValue>,
    statistics: PropagationStats,
}

#[derive(Debug, Clone, Copy)]
pub struct PropagationStats {
    pub constants_folded: usize,
    pub values_propagated: usize,
    pub instructions_eliminated: usize,
    pub savings_bytes: usize,
}

impl ConstantPropagator {
    pub fn new() -> Self {
        ConstantPropagator {
            constants: HashMap::new(),
            statistics: PropagationStats {
                constants_folded: 0,
                values_propagated: 0,
                instructions_eliminated: 0,
                savings_bytes: 0,
            },
        }
    }

    pub fn register_constant(&mut self, name: String, value: ConstantValue) {
        self.constants.insert(name, value);
    }

    pub fn get_constant(&self, name: &str) -> Option<ConstantValue> {
        self.constants.get(name).cloned()
    }

    pub fn fold_binary_op(
        &mut self,
        left_name: &str,
        op: &str,
        right_name: &str,
    ) -> Option<ConstantValue> {
        let left = self.constants.get(left_name).cloned()?;
        let right = self.constants.get(right_name).cloned()?;

        let result = match op {
            "add" => left.add(&right),
            "sub" => left.sub(&right),
            "mul" => left.mul(&right),
            "div" => left.div(&right),
            "mod" => left.modulo(&right),
            "and" => left.bitwise_and(&right),
            "or" => left.bitwise_or(&right),
            "xor" => left.bitwise_xor(&right),
            _ => None,
        }?;

        self.statistics.constants_folded += 1;
        Some(result)
    }

    pub fn propagate_ir(&mut self, ir: &str) -> String {
        let mut result = String::new();

        for line in ir.lines() {
            let processed = self.process_line(line);
            result.push_str(&processed);
            result.push('\n');
        }

        result.trim().to_string()
    }

    fn process_line(&mut self, line: &str) -> String {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            return line.to_string();
        }

        if parts.len() >= 5 && (parts[1] == "=" || parts[2] == "=") {
            if let Some(result) = self.try_fold_line(line) {
                self.statistics.values_propagated += 1;
                return result;
            }
        }

        line.to_string()
    }

    fn try_fold_line(&mut self, line: &str) -> Option<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 5 {
            return None;
        }

        let assign_idx = if parts[1] == "=" { 1 } else { 2 };
        let var_name = parts[assign_idx - 1];
        let op = parts[assign_idx + 1];

        if assign_idx + 3 < parts.len() {
            let left = parts[assign_idx + 2];
            let right = parts[assign_idx + 3];

            if let Some(result) = self.fold_binary_op(left, op, right) {
                self.constants
                    .insert(var_name.to_string(), result.clone());
                return Some(format!("{} = {}", var_name, result.to_string()));
            }
        }

        None
    }

    pub fn get_statistics(&self) -> PropagationStats {
        self.statistics
    }

    pub fn clear(&mut self) {
        self.constants.clear();
        self.statistics = PropagationStats {
            constants_folded: 0,
            values_propagated: 0,
            instructions_eliminated: 0,
            savings_bytes: 0,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_add() {
        let a = ConstantValue::Integer(10);
        let b = ConstantValue::Integer(20);
        assert_eq!(a.add(&b), Some(ConstantValue::Integer(30)));
    }

    #[test]
    fn test_integer_mul() {
        let a = ConstantValue::Integer(5);
        let b = ConstantValue::Integer(6);
        assert_eq!(a.mul(&b), Some(ConstantValue::Integer(30)));
    }

    #[test]
    fn test_propagator_register() {
        let mut prop = ConstantPropagator::new();
        prop.register_constant("x".to_string(), ConstantValue::Integer(42));
        assert_eq!(
            prop.get_constant("x"),
            Some(ConstantValue::Integer(42))
        );
    }

    #[test]
    fn test_binary_op_folding() {
        let mut prop = ConstantPropagator::new();
        prop.register_constant("a".to_string(), ConstantValue::Integer(10));
        prop.register_constant("b".to_string(), ConstantValue::Integer(20));

        let result = prop.fold_binary_op("a", "add", "b");
        assert_eq!(result, Some(ConstantValue::Integer(30)));
    }

    #[test]
    fn test_propagation_stats() {
        let prop = ConstantPropagator::new();
        let stats = prop.get_statistics();
        assert_eq!(stats.constants_folded, 0);
    }
}
