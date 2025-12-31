//! Scope-based lifetime and borrow tracking
//!
//! Manages lexical scopes and tracks which bindings are valid within each scope.
//! Provides foundation for Non-Lexical Lifetimes (NLL) by tracking:
//! - Scope entry/exit points
//! - Binding validity ranges
//! - Borrow validity within scopes
//! - Region-based analysis

use super::lifetimes::Lifetime;
use crate::lowering::HirType;
use std::collections::HashMap;

/// Unique identifier for a scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(usize);

impl ScopeId {
    pub fn new(id: usize) -> Self {
        ScopeId(id)
    }
}

/// Information about a binding within a scope
#[derive(Debug, Clone)]
pub struct ScopeBinding {
    /// Variable name
    pub name: String,
    /// Type of the binding
    pub ty: HirType,
    /// Whether binding is mutable
    pub is_mutable: bool,
    /// Lifetime of this binding (when it's valid)
    pub lifetime: Option<Lifetime>,
    /// Where binding was created (scope level)
    pub scope_level: usize,
}

/// Represents a lexical scope with its bindings
#[derive(Debug, Clone)]
pub struct Scope {
    /// Unique scope identifier
    pub id: ScopeId,
    /// Parent scope (if any)
    pub parent: Option<ScopeId>,
    /// Nesting depth
    pub depth: usize,
    /// Bindings in this scope
    pub bindings: HashMap<String, ScopeBinding>,
}

impl Scope {
    /// Create a new scope
    pub fn new(id: ScopeId, parent: Option<ScopeId>, depth: usize) -> Self {
        Scope {
            id,
            parent,
            depth,
            bindings: HashMap::new(),
        }
    }

    /// Add a binding to this scope
    pub fn add_binding(
        &mut self,
        name: String,
        ty: HirType,
        is_mutable: bool,
        lifetime: Option<Lifetime>,
    ) {
        self.bindings.insert(
            name.clone(),
            ScopeBinding {
                name,
                ty,
                is_mutable,
                lifetime,
                scope_level: self.depth,
            },
        );
    }

    /// Look up a binding in this scope (non-recursive)
    pub fn lookup_local(&self, name: &str) -> Option<&ScopeBinding> {
        self.bindings.get(name)
    }

    /// Check if binding exists in this scope
    pub fn contains(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }
}

/// Scope stack for managing lexical scopes
#[derive(Debug)]
pub struct ScopeStack {
    /// All scopes by ID
    scopes: HashMap<ScopeId, Scope>,
    /// Current scope stack (LIFO)
    stack: Vec<ScopeId>,
    /// Next available scope ID
    next_id: usize,
}

impl ScopeStack {
    /// Create a new scope stack with global scope
    pub fn new() -> Self {
        let global_scope = Scope::new(ScopeId(0), None, 0);
        let mut scopes = HashMap::new();
        scopes.insert(ScopeId(0), global_scope);

        ScopeStack {
            scopes,
            stack: vec![ScopeId(0)],
            next_id: 1,
        }
    }

    /// Get current scope
    pub fn current(&self) -> Option<&Scope> {
        self.stack.last().and_then(|id| self.scopes.get(id))
    }

    /// Get current scope mutably
    pub fn current_mut(&mut self) -> Option<&mut Scope> {
        let id = *self.stack.last()?;
        self.scopes.get_mut(&id)
    }

    /// Push a new child scope
    pub fn push_scope(&mut self) {
        // SAFETY: Stack is initialized with global scope in new(), and pop_scope() guards against empty stack
        // If this panics, it indicates a logic error in scope management (push/pop mismatch)
        let parent = match self.stack.last() {
            Some(id) => *id,
            None => {
                eprintln!("[ScopeStack] ERROR: push_scope() called but scope stack is empty - this indicates a push/pop mismatch");
                // Fallback: return without creating scope
                return;
            }
        };
        
        let current_depth = self.scopes[&parent].depth + 1;
        let new_id = ScopeId(self.next_id);
        self.next_id += 1;

        let new_scope = Scope::new(new_id, Some(parent), current_depth);
        self.scopes.insert(new_id, new_scope);
        self.stack.push(new_id);
    }

    /// Pop current scope
    pub fn pop_scope(&mut self) -> Option<ScopeId> {
        if self.stack.len() > 1 {
            self.stack.pop()
        } else {
            None
        }
    }

    /// Add binding to current scope
    pub fn add_binding(
        &mut self,
        name: String,
        ty: HirType,
        is_mutable: bool,
        lifetime: Option<Lifetime>,
    ) -> Result<(), String> {
        if let Some(scope) = self.current_mut() {
            scope.add_binding(name, ty, is_mutable, lifetime);
            Ok(())
        } else {
            Err("No current scope".to_string())
        }
    }

    /// Look up a binding from current scope upward (with parent lookup)
    pub fn lookup(&self, name: &str) -> Option<&ScopeBinding> {
        let mut current = *self.stack.last()?;
        loop {
            if let Some(scope) = self.scopes.get(&current) {
                if let Some(binding) = scope.lookup_local(name) {
                    return Some(binding);
                }
                current = scope.parent?;
            } else {
                return None;
            }
        }
    }

    /// Check if a binding exists (searches up the scope chain)
    pub fn contains(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Get all bindings in current scope
    pub fn current_bindings(&self) -> Option<Vec<&ScopeBinding>> {
        self.current().map(|scope| {
            scope
                .bindings
                .values()
                .collect::<Vec<_>>()
        })
    }

    /// Get current scope depth
    pub fn current_depth(&self) -> usize {
        self.current().map(|s| s.depth).unwrap_or(0)
    }

    /// Get current scope ID
    pub fn current_id(&self) -> Option<ScopeId> {
        self.stack.last().copied()
    }
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_stack_creation() {
        let stack = ScopeStack::new();
        assert!(stack.current().is_some());
    }

    #[test]
    fn test_push_and_pop_scopes() {
        let mut stack = ScopeStack::new();
        assert_eq!(stack.current_depth(), 0);

        stack.push_scope();
        assert_eq!(stack.current_depth(), 1);

        stack.push_scope();
        assert_eq!(stack.current_depth(), 2);

        stack.pop_scope();
        assert_eq!(stack.current_depth(), 1);
    }

    #[test]
    fn test_binding_addition() {
        let mut stack = ScopeStack::new();
        stack
            .add_binding(
                "x".to_string(),
                HirType::Int32,
                false,
                None,
            )
            .unwrap();
        assert!(stack.contains("x"));
    }

    #[test]
    fn test_binding_lookup_with_parent() {
        let mut stack = ScopeStack::new();
        stack
            .add_binding(
                "x".to_string(),
                HirType::Int32,
                false,
                None,
            )
            .unwrap();

        stack.push_scope();
        // Should find x in parent scope
        assert!(stack.contains("x"));

        stack
            .add_binding(
                "y".to_string(),
                HirType::Bool,
                true,
                None,
            )
            .unwrap();
        assert!(stack.contains("y"));
        assert!(stack.contains("x"));
    }

    #[test]
    fn test_binding_scope_isolation() {
        let mut stack = ScopeStack::new();
        stack.push_scope();
        stack
            .add_binding(
                "x".to_string(),
                HirType::Int32,
                false,
                None,
            )
            .unwrap();

        stack.pop_scope();
        // x should not be accessible after scope exits
        let result = stack.lookup("x");
        // After popping the scope, x should not be found
        assert!(result.is_none());
    }

    #[test]
    fn test_mutable_binding_tracking() {
        let mut stack = ScopeStack::new();
        stack
            .add_binding(
                "x".to_string(),
                HirType::Int32,
                true,
                None,
            )
            .unwrap();

        let binding = stack.lookup("x").unwrap();
        assert!(binding.is_mutable);
    }

    #[test]
    fn test_lifetime_association() {
        let mut stack = ScopeStack::new();
        let lifetime = Some(Lifetime::Static);
        stack
            .add_binding(
                "s".to_string(),
                HirType::String,
                false,
                lifetime.clone(),
            )
            .unwrap();

        let binding = stack.lookup("s").unwrap();
        assert_eq!(binding.lifetime, lifetime);
    }
}
