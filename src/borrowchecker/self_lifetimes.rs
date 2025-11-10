//! Self reference lifetime validation
//!
//! Handles lifetime constraints for methods with `&self` or `&mut self` parameters.

use super::lifetimes::Lifetime;

/// Information about a method's self parameter and lifetime
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfReference {
    /// No self reference (not a method)
    None,
    /// Immutable self: &self (with implicit lifetime)
    Immutable,
    /// Mutable self: &mut self (with implicit lifetime)
    Mutable,
}

impl SelfReference {
    /// Check if this is a self reference
    pub fn is_self_reference(&self) -> bool {
        !matches!(self, SelfReference::None)
    }

    /// Get display string
    pub fn as_str(&self) -> &'static str {
        match self {
            SelfReference::None => "",
            SelfReference::Immutable => "&self",
            SelfReference::Mutable => "&mut self",
        }
    }
}

/// Validate self reference in method signature
pub fn validate_self_reference(
    param_count: usize,
    first_param_name: &str,
) -> Result<SelfReference, String> {
    if first_param_name == "self" && param_count > 0 {
        Ok(SelfReference::Immutable)
    } else if first_param_name == "mut_self" && param_count > 0 {
        Ok(SelfReference::Mutable)
    } else {
        Ok(SelfReference::None)
    }
}

/// Generate constraint for self reference
pub fn generate_self_constraint(
    self_ref: &SelfReference,
    return_lifetime: &Option<Lifetime>,
) -> Option<(Lifetime, Lifetime, String)> {
    if !self_ref.is_self_reference() {
        return None;
    }

    if let Some(ret_lt) = return_lifetime {
        let self_lt = match self_ref {
            SelfReference::Immutable => Lifetime::Named("self_lifetime".to_string()),
            SelfReference::Mutable => Lifetime::Named("self_lifetime".to_string()),
            SelfReference::None => return None,
        };

        let reason = match self_ref {
            SelfReference::Immutable => "return type must not outlive &self".to_string(),
            SelfReference::Mutable => "return type must not outlive &mut self".to_string(),
            SelfReference::None => return None,
        };

        Some((self_lt, ret_lt.clone(), reason))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_immutable() {
        let result = validate_self_reference(2, "self");
        assert_eq!(result.unwrap(), SelfReference::Immutable);
    }

    #[test]
    fn test_self_mutable() {
        let result = validate_self_reference(2, "mut_self");
        assert_eq!(result.unwrap(), SelfReference::Mutable);
    }

    #[test]
    fn test_self_none() {
        let result = validate_self_reference(1, "x");
        assert_eq!(result.unwrap(), SelfReference::None);
    }

    #[test]
    fn test_self_reference_is_self() {
        assert!(SelfReference::Immutable.is_self_reference());
        assert!(SelfReference::Mutable.is_self_reference());
        assert!(!SelfReference::None.is_self_reference());
    }

    #[test]
    fn test_self_as_str() {
        assert_eq!(SelfReference::Immutable.as_str(), "&self");
        assert_eq!(SelfReference::Mutable.as_str(), "&mut self");
        assert_eq!(SelfReference::None.as_str(), "");
    }

    #[test]
    fn test_self_constraint_generation() {
        let return_lt = Some(Lifetime::Named("a".to_string()));
        let constraint = generate_self_constraint(&SelfReference::Immutable, &return_lt);
        assert!(constraint.is_some());
        let (lhs, rhs, _) = constraint.unwrap();
        assert_eq!(lhs, Lifetime::Named("self_lifetime".to_string()));
        assert_eq!(rhs, Lifetime::Named("a".to_string()));
    }

    #[test]
    fn test_self_constraint_no_return_lifetime() {
        let return_lt = None;
        let constraint = generate_self_constraint(&SelfReference::Immutable, &return_lt);
        assert!(constraint.is_none());
    }

    #[test]
    fn test_self_constraint_no_self_ref() {
        let return_lt = Some(Lifetime::Named("a".to_string()));
        let constraint = generate_self_constraint(&SelfReference::None, &return_lt);
        assert!(constraint.is_none());
    }
}
