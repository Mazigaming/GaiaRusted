//! # Advanced Macro System
//!
//! Features:
//! - Procedural macro attributes
//! - Macro scoping and namespacing
//! - Macro rules with advanced pattern matching
//! - Macro error recovery
//! - Macro debugging and introspection

use std::collections::HashMap;
use std::fmt;

/// Advanced macro pattern matching
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvancedPattern {
    /// Literal token sequence
    Tokens(Vec<String>),
    /// Recursive pattern matching
    Recursive(Box<AdvancedPattern>),
    /// Alternative patterns
    Or(Vec<AdvancedPattern>),
    /// Optional pattern
    Optional(Box<AdvancedPattern>),
    /// Repetition with bounds
    Repeat {
        pattern: Box<AdvancedPattern>,
        min: usize,
        max: Option<usize>,
    },
    /// Capture with type constraint
    Capture {
        name: String,
        constraint: Option<String>,
    },
}

impl fmt::Display for AdvancedPattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AdvancedPattern::Tokens(tokens) => {
                write!(f, "tokens({})", tokens.join(" "))
            }
            AdvancedPattern::Recursive(pat) => {
                write!(f, "rec({})", pat)
            }
            AdvancedPattern::Or(patterns) => {
                let strs: Vec<_> = patterns.iter().map(|p| format!("{}", p)).collect();
                write!(f, "or({})", strs.join(" | "))
            }
            AdvancedPattern::Optional(pat) => {
                write!(f, "{}?", pat)
            }
            AdvancedPattern::Repeat { pattern, min, max } => {
                let max_str = max.map(|m| m.to_string()).unwrap_or_else(|| "*".to_string());
                write!(f, "{{{},{}..{}}}", pattern, min, max_str)
            }
            AdvancedPattern::Capture { name, constraint } => {
                if let Some(c) = constraint {
                    write!(f, "{}:{}", name, c)
                } else {
                    write!(f, "{}", name)
                }
            }
        }
    }
}

/// Advanced macro rule with metadata
#[derive(Debug, Clone)]
pub struct AdvancedMacroRule {
    /// Rule name/label
    pub name: String,
    /// Pattern to match
    pub pattern: AdvancedPattern,
    /// Replacement template
    pub template: String,
    /// Predicates that must be satisfied
    pub predicates: Vec<MacroPredicate>,
    /// Scope of the rule
    pub scope: MacroScope,
}

impl AdvancedMacroRule {
    /// Create a new advanced macro rule
    pub fn new(name: String, pattern: AdvancedPattern, template: String) -> Self {
        AdvancedMacroRule {
            name,
            pattern,
            template,
            predicates: Vec::new(),
            scope: MacroScope::Local,
        }
    }

    /// Add a predicate constraint
    pub fn add_predicate(&mut self, predicate: MacroPredicate) {
        self.predicates.push(predicate);
    }

    /// Set scope of the rule
    pub fn set_scope(&mut self, scope: MacroScope) {
        self.scope = scope;
    }
}

/// Scope of a macro definition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroScope {
    /// Local to current module
    Local,
    /// Public within crate
    Crate,
    /// Globally visible
    Global,
}

impl fmt::Display for MacroScope {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MacroScope::Local => write!(f, "local"),
            MacroScope::Crate => write!(f, "crate"),
            MacroScope::Global => write!(f, "global"),
        }
    }
}

/// Predicate for macro rule matching
#[derive(Debug, Clone)]
pub enum MacroPredicate {
    /// Type constraint on captured variable
    TypeConstraint { name: String, ty: String },
    /// Value constraint
    ValueRange { name: String, min: i64, max: i64 },
    /// Regex pattern matching
    PatternMatch { name: String, pattern: String },
}

/// Procedural macro attribute
#[derive(Debug, Clone)]
pub struct ProceduralMacroAttr {
    /// Attribute name
    pub name: String,
    /// Target items (derive, attribute, function_like)
    pub target: MacroTargetType,
    /// Function to execute
    pub handler: String,
    /// Configuration/options
    pub config: HashMap<String, String>,
}

impl ProceduralMacroAttr {
    /// Create a new procedural macro attribute
    pub fn new(name: String, target: MacroTargetType, handler: String) -> Self {
        ProceduralMacroAttr {
            name,
            target,
            handler,
            config: HashMap::new(),
        }
    }

    /// Add configuration option
    pub fn add_config(&mut self, key: String, value: String) {
        self.config.insert(key, value);
    }
}

/// Type of macro target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroTargetType {
    /// Derive macro (for structs/enums)
    Derive,
    /// Attribute macro (for attributes)
    Attribute,
    /// Function-like macro
    FunctionLike,
}

impl fmt::Display for MacroTargetType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MacroTargetType::Derive => write!(f, "derive"),
            MacroTargetType::Attribute => write!(f, "attribute"),
            MacroTargetType::FunctionLike => write!(f, "function_like"),
        }
    }
}

/// Macro invocation with context
#[derive(Debug, Clone)]
pub struct MacroInvocation {
    /// Macro name being invoked
    pub name: String,
    /// Arguments to macro
    pub args: Vec<String>,
    /// Location in source
    pub location: String,
    /// Invocation context (module path)
    pub context: String,
}

impl MacroInvocation {
    /// Create a new macro invocation
    pub fn new(name: String, args: Vec<String>, context: String) -> Self {
        MacroInvocation {
            name,
            args,
            location: "unknown".to_string(),
            context,
        }
    }

    /// Set location information
    pub fn with_location(mut self, location: String) -> Self {
        self.location = location;
        self
    }
}

/// Macro expansion result
#[derive(Debug, Clone)]
pub struct MacroExpansionResult {
    /// Expanded code
    pub expanded: String,
    /// Number of expansions performed
    pub expansion_count: usize,
    /// Any warnings
    pub warnings: Vec<String>,
    /// Expansion trace (for debugging)
    pub trace: Vec<String>,
}

impl MacroExpansionResult {
    /// Create a new expansion result
    pub fn new(expanded: String) -> Self {
        MacroExpansionResult {
            expanded,
            expansion_count: 1,
            warnings: Vec::new(),
            trace: Vec::new(),
        }
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Add to expansion trace
    pub fn add_trace(&mut self, entry: String) {
        self.trace.push(entry);
    }

    /// Increment expansion count
    pub fn increment_expansions(&mut self) {
        self.expansion_count += 1;
    }
}

/// Advanced macro system
pub struct AdvancedMacroSystem {
    /// Macro rules
    rules: HashMap<String, Vec<AdvancedMacroRule>>,
    /// Procedural macros
    procedural: HashMap<String, ProceduralMacroAttr>,
    /// Macro invocation history (for debugging)
    invocation_history: Vec<MacroInvocation>,
    /// Expansion cache
    expansion_cache: HashMap<String, MacroExpansionResult>,
}

impl AdvancedMacroSystem {
    /// Create a new advanced macro system
    pub fn new() -> Self {
        AdvancedMacroSystem {
            rules: HashMap::new(),
            procedural: HashMap::new(),
            invocation_history: Vec::new(),
            expansion_cache: HashMap::new(),
        }
    }

    /// Register a macro rule
    pub fn register_rule(&mut self, macro_name: String, rule: AdvancedMacroRule) {
        self.rules
            .entry(macro_name)
            .or_insert_with(Vec::new)
            .push(rule);
    }

    /// Register a procedural macro
    pub fn register_procedural(
        &mut self,
        attr: ProceduralMacroAttr,
    ) -> Result<(), String> {
        if self.procedural.contains_key(&attr.name) {
            return Err(format!("Procedural macro {} already exists", attr.name));
        }
        self.procedural.insert(attr.name.clone(), attr);
        Ok(())
    }

    /// Get macro rules by name
    pub fn get_rules(&self, name: &str) -> Option<&Vec<AdvancedMacroRule>> {
        self.rules.get(name)
    }

    /// Get procedural macro by name
    pub fn get_procedural(&self, name: &str) -> Option<&ProceduralMacroAttr> {
        self.procedural.get(name)
    }

    /// Record macro invocation
    pub fn record_invocation(&mut self, invocation: MacroInvocation) {
        self.invocation_history.push(invocation);
    }

    /// Get invocation history
    pub fn get_invocation_history(&self) -> &[MacroInvocation] {
        &self.invocation_history
    }

    /// Clear invocation history
    pub fn clear_history(&mut self) {
        self.invocation_history.clear();
    }

    /// Cache expansion result
    pub fn cache_expansion(&mut self, key: String, result: MacroExpansionResult) {
        self.expansion_cache.insert(key, result);
    }

    /// Get cached expansion
    pub fn get_cached_expansion(&self, key: &str) -> Option<&MacroExpansionResult> {
        self.expansion_cache.get(key)
    }

    /// Clear expansion cache
    pub fn clear_cache(&mut self) {
        self.expansion_cache.clear();
    }

    /// Get total rules registered
    pub fn total_rules(&self) -> usize {
        self.rules.values().map(|v| v.len()).sum()
    }

    /// Get total procedural macros
    pub fn total_procedural(&self) -> usize {
        self.procedural.len()
    }
}

impl Default for AdvancedMacroSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_pattern_display() {
        let pat = AdvancedPattern::Tokens(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(pat.to_string(), "tokens(a b)");
    }

    #[test]
    fn test_advanced_macro_rule_creation() {
        let pattern = AdvancedPattern::Tokens(vec!["test".to_string()]);
        let rule = AdvancedMacroRule::new(
            "rule1".to_string(),
            pattern,
            "expanded".to_string(),
        );
        assert_eq!(rule.name, "rule1");
    }

    #[test]
    fn test_macro_scope_display() {
        assert_eq!(MacroScope::Local.to_string(), "local");
        assert_eq!(MacroScope::Crate.to_string(), "crate");
        assert_eq!(MacroScope::Global.to_string(), "global");
    }

    #[test]
    fn test_procedural_macro_attr() {
        let attr = ProceduralMacroAttr::new(
            "derive_clone".to_string(),
            MacroTargetType::Derive,
            "clone_handler".to_string(),
        );
        assert_eq!(attr.name, "derive_clone");
        assert_eq!(attr.target, MacroTargetType::Derive);
    }

    #[test]
    fn test_procedural_macro_config() {
        let mut attr = ProceduralMacroAttr::new(
            "test".to_string(),
            MacroTargetType::Attribute,
            "handler".to_string(),
        );
        attr.add_config("debug".to_string(), "true".to_string());
        assert_eq!(attr.config.len(), 1);
    }

    #[test]
    fn test_macro_invocation() {
        let invocation = MacroInvocation::new(
            "println".to_string(),
            vec!["hello".to_string()],
            "main".to_string(),
        );
        assert_eq!(invocation.name, "println");
        assert_eq!(invocation.args.len(), 1);
    }

    #[test]
    fn test_macro_expansion_result() {
        let mut result = MacroExpansionResult::new("expanded code".to_string());
        result.add_warning("warn1".to_string());
        result.increment_expansions();
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.expansion_count, 2);
    }

    #[test]
    fn test_advanced_macro_system_creation() {
        let sys = AdvancedMacroSystem::new();
        assert_eq!(sys.total_rules(), 0);
        assert_eq!(sys.total_procedural(), 0);
    }

    #[test]
    fn test_register_macro_rule() {
        let mut sys = AdvancedMacroSystem::new();
        let pattern = AdvancedPattern::Tokens(vec!["test".to_string()]);
        let rule = AdvancedMacroRule::new(
            "rule".to_string(),
            pattern,
            "expanded".to_string(),
        );
        sys.register_rule("test_macro".to_string(), rule);
        assert_eq!(sys.total_rules(), 1);
    }

    #[test]
    fn test_register_procedural_macro() {
        let mut sys = AdvancedMacroSystem::new();
        let attr = ProceduralMacroAttr::new(
            "derive".to_string(),
            MacroTargetType::Derive,
            "handler".to_string(),
        );
        assert!(sys.register_procedural(attr).is_ok());
        assert_eq!(sys.total_procedural(), 1);
    }

    #[test]
    fn test_record_invocation() {
        let mut sys = AdvancedMacroSystem::new();
        let invocation = MacroInvocation::new(
            "test".to_string(),
            vec![],
            "main".to_string(),
        );
        sys.record_invocation(invocation);
        assert_eq!(sys.get_invocation_history().len(), 1);
    }

    #[test]
    fn test_cache_expansion() {
        let mut sys = AdvancedMacroSystem::new();
        let result = MacroExpansionResult::new("code".to_string());
        sys.cache_expansion("key1".to_string(), result);
        assert!(sys.get_cached_expansion("key1").is_some());
    }

    #[test]
    fn test_macro_predicate_type_constraint() {
        let pred = MacroPredicate::TypeConstraint {
            name: "T".to_string(),
            ty: "i32".to_string(),
        };
        assert!(matches!(pred, MacroPredicate::TypeConstraint { .. }));
    }

    #[test]
    fn test_macro_target_type_display() {
        assert_eq!(MacroTargetType::Derive.to_string(), "derive");
        assert_eq!(MacroTargetType::Attribute.to_string(), "attribute");
        assert_eq!(MacroTargetType::FunctionLike.to_string(), "function_like");
    }

    #[test]
    fn test_get_rules() {
        let mut sys = AdvancedMacroSystem::new();
        let pattern = AdvancedPattern::Tokens(vec!["test".to_string()]);
        let rule = AdvancedMacroRule::new(
            "r1".to_string(),
            pattern,
            "exp".to_string(),
        );
        sys.register_rule("m1".to_string(), rule);
        assert!(sys.get_rules("m1").is_some());
        assert!(sys.get_rules("m2").is_none());
    }
}
