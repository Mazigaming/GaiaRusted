//! # Advanced Error Handling System
//!
//! Features:
//! - Custom error types with error traits
//! - Error context and source tracking
//! - Error propagation helpers
//! - Try operator support
//! - Error conversion and wrapping

use std::fmt;

/// Custom error trait for user-defined errors
pub trait CustomError: fmt::Debug + fmt::Display + Send + Sync + 'static {
    /// Get error code if applicable
    fn code(&self) -> Option<String> {
        None
    }

    /// Get error source
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }


}

/// Error with context information
#[derive(Debug, Clone)]
pub struct ContextualError {
    /// Error message
    pub message: String,
    /// Error code
    pub code: String,
    /// Context information
    pub context: Vec<String>,
    /// Source location
    pub source_location: Option<String>,
}

impl ContextualError {
    /// Create a new contextual error
    pub fn new(message: String, code: String) -> Self {
        ContextualError {
            message,
            code,
            context: Vec::new(),
            source_location: None,
        }
    }

    /// Add context information
    pub fn with_context(mut self, context: String) -> Self {
        self.context.push(context);
        self
    }

    /// Set source location
    pub fn with_location(mut self, location: String) -> Self {
        self.source_location = Some(location);
        self
    }

    /// Get context stack
    pub fn get_context(&self) -> &[String] {
        &self.context
    }
}

impl fmt::Display for ContextualError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if !self.context.is_empty() {
            write!(f, "\nContext: {}", self.context.join(" -> "))?;
        }
        if let Some(loc) = &self.source_location {
            write!(f, " at {}", loc)?;
        }
        Ok(())
    }
}

/// Error chain for tracking error sources
#[derive(Debug)]
pub struct ErrorChain {
    /// Root error
    pub root: String,
    /// Chain of causes
    pub causes: Vec<String>,
}

impl ErrorChain {
    /// Create a new error chain
    pub fn new(root: String) -> Self {
        ErrorChain {
            root,
            causes: Vec::new(),
        }
    }

    /// Add a cause to the chain
    pub fn add_cause(&mut self, cause: String) {
        self.causes.push(cause);
    }

    /// Get full chain as string
    pub fn full_chain(&self) -> String {
        let mut chain = format!("Root: {}", self.root);
        for (i, cause) in self.causes.iter().enumerate() {
            chain.push_str(&format!("\nCause {}: {}", i + 1, cause));
        }
        chain
    }
}

impl fmt::Display for ErrorChain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.full_chain())
    }
}

/// Error result type
pub type ErrorResult<T, E> = Result<T, E>;

/// Error conversion trait
pub trait ErrorConversion: Sized {
    /// Convert from another error type
    fn from_error<E: fmt::Display>(error: E) -> Self;
}

/// Error wrapper for converting between error types
#[derive(Debug)]
pub struct WrappedError {
    /// Wrapped error message
    pub inner: String,
    /// Error category
    pub category: String,
}

impl WrappedError {
    /// Create a new wrapped error
    pub fn new(inner: String, category: String) -> Self {
        WrappedError { inner, category }
    }

    /// Wrap an error with context
    pub fn wrap<E: fmt::Display>(error: E, category: String) -> Self {
        WrappedError {
            inner: error.to_string(),
            category,
        }
    }
}

impl fmt::Display for WrappedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] {}", self.category, self.inner)
    }
}

/// Error recovery strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Panic on error
    Panic,
    /// Return default value
    Default,
    /// Retry operation
    Retry,
    /// Log and continue
    LogContinue,
    /// Custom handler
    Custom,
}

impl fmt::Display for RecoveryStrategy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RecoveryStrategy::Panic => write!(f, "panic"),
            RecoveryStrategy::Default => write!(f, "default"),
            RecoveryStrategy::Retry => write!(f, "retry"),
            RecoveryStrategy::LogContinue => write!(f, "log_continue"),
            RecoveryStrategy::Custom => write!(f, "custom"),
        }
    }
}

/// Try operator support
#[derive(Debug, Clone)]
pub struct TryOperator {
    /// Success flag
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Number of retry attempts
    pub retry_count: usize,
}

impl TryOperator {
    /// Create a successful try operation
    pub fn success() -> Self {
        TryOperator {
            success: true,
            error: None,
            retry_count: 0,
        }
    }

    /// Create a failed try operation
    pub fn failure(error: String) -> Self {
        TryOperator {
            success: false,
            error: Some(error),
            retry_count: 0,
        }
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// Check if operation succeeded
    pub fn is_ok(&self) -> bool {
        self.success
    }

    /// Check if operation failed
    pub fn is_err(&self) -> bool {
        !self.success
    }

    /// Get error message
    pub fn get_error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// Error handler for managing error recovery
pub struct ErrorHandler {
    /// Strategy to use
    strategy: RecoveryStrategy,
    /// Max retry attempts
    max_retries: usize,
    /// Error log
    error_log: Vec<String>,
}

impl ErrorHandler {
    /// Create a new error handler
    pub fn new(strategy: RecoveryStrategy) -> Self {
        ErrorHandler {
            strategy,
            max_retries: 3,
            error_log: Vec::new(),
        }
    }

    /// Set max retries
    pub fn set_max_retries(&mut self, max: usize) {
        self.max_retries = max;
    }

    /// Get recovery strategy
    pub fn get_strategy(&self) -> RecoveryStrategy {
        self.strategy
    }

    /// Log an error
    pub fn log_error(&mut self, error: String) {
        self.error_log.push(error);
    }

    /// Get error log
    pub fn get_error_log(&self) -> &[String] {
        &self.error_log
    }

    /// Clear error log
    pub fn clear_log(&mut self) {
        self.error_log.clear();
    }

    /// Handle an error according to strategy
    pub fn handle_error(&mut self, error: String) -> Result<(), String> {
        match self.strategy {
            RecoveryStrategy::Panic => {
                panic!("{}", error)
            }
            RecoveryStrategy::Default => {
                self.log_error(error);
                Ok(())
            }
            RecoveryStrategy::Retry => {
                if self.error_log.len() < self.max_retries {
                    self.log_error(error);
                    Ok(())
                } else {
                    Err(format!("Max retries exceeded: {}", error))
                }
            }
            RecoveryStrategy::LogContinue => {
                self.log_error(error);
                Ok(())
            }
            RecoveryStrategy::Custom => {
                self.log_error(error);
                Ok(())
            }
        }
    }
}

impl Default for ErrorHandler {
    fn default() -> Self {
        Self::new(RecoveryStrategy::LogContinue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contextual_error_creation() {
        let err = ContextualError::new("test error".to_string(), "E001".to_string());
        assert_eq!(err.message, "test error");
        assert_eq!(err.code, "E001");
    }

    #[test]
    fn test_contextual_error_with_context() {
        let err = ContextualError::new("error".to_string(), "E001".to_string())
            .with_context("context1".to_string());
        assert_eq!(err.context.len(), 1);
    }

    #[test]
    fn test_contextual_error_with_location() {
        let err = ContextualError::new("error".to_string(), "E001".to_string())
            .with_location("main.rs:42".to_string());
        assert!(err.source_location.is_some());
    }

    #[test]
    fn test_error_chain_creation() {
        let chain = ErrorChain::new("root error".to_string());
        assert_eq!(chain.root, "root error");
        assert_eq!(chain.causes.len(), 0);
    }

    #[test]
    fn test_error_chain_add_cause() {
        let mut chain = ErrorChain::new("root".to_string());
        chain.add_cause("cause1".to_string());
        chain.add_cause("cause2".to_string());
        assert_eq!(chain.causes.len(), 2);
    }

    #[test]
    fn test_wrapped_error_creation() {
        let wrapped = WrappedError::new("inner".to_string(), "IO".to_string());
        assert_eq!(wrapped.inner, "inner");
        assert_eq!(wrapped.category, "IO");
    }

    #[test]
    fn test_wrapped_error_wrap() {
        let error = "file not found";
        let wrapped = WrappedError::wrap(error, "FileIO".to_string());
        assert_eq!(wrapped.category, "FileIO");
    }

    #[test]
    fn test_recovery_strategy_display() {
        assert_eq!(RecoveryStrategy::Panic.to_string(), "panic");
        assert_eq!(RecoveryStrategy::Default.to_string(), "default");
        assert_eq!(RecoveryStrategy::Retry.to_string(), "retry");
        assert_eq!(RecoveryStrategy::LogContinue.to_string(), "log_continue");
        assert_eq!(RecoveryStrategy::Custom.to_string(), "custom");
    }

    #[test]
    fn test_try_operator_success() {
        let op = TryOperator::success();
        assert!(op.is_ok());
        assert!(!op.is_err());
    }

    #[test]
    fn test_try_operator_failure() {
        let op = TryOperator::failure("error".to_string());
        assert!(!op.is_ok());
        assert!(op.is_err());
        assert_eq!(op.get_error(), Some("error"));
    }

    #[test]
    fn test_try_operator_retry_count() {
        let mut op = TryOperator::success();
        op.increment_retry();
        op.increment_retry();
        assert_eq!(op.retry_count, 2);
    }

    #[test]
    fn test_error_handler_creation() {
        let handler = ErrorHandler::new(RecoveryStrategy::LogContinue);
        assert_eq!(handler.get_strategy(), RecoveryStrategy::LogContinue);
    }

    #[test]
    fn test_error_handler_log_error() {
        let mut handler = ErrorHandler::new(RecoveryStrategy::Default);
        handler.log_error("error1".to_string());
        assert_eq!(handler.get_error_log().len(), 1);
    }

    #[test]
    fn test_error_handler_max_retries() {
        let mut handler = ErrorHandler::new(RecoveryStrategy::Retry);
        handler.set_max_retries(5);
        assert_eq!(handler.max_retries, 5);
    }

    #[test]
    fn test_error_handler_handle_error_log_continue() {
        let mut handler = ErrorHandler::new(RecoveryStrategy::LogContinue);
        let result = handler.handle_error("error".to_string());
        assert!(result.is_ok());
        assert_eq!(handler.get_error_log().len(), 1);
    }

    #[test]
    fn test_error_handler_clear_log() {
        let mut handler = ErrorHandler::new(RecoveryStrategy::Default);
        handler.log_error("error1".to_string());
        assert_eq!(handler.get_error_log().len(), 1);
        handler.clear_log();
        assert_eq!(handler.get_error_log().len(), 0);
    }

    #[test]
    fn test_contextual_error_display() {
        let err = ContextualError::new("test".to_string(), "E001".to_string())
            .with_context("ctx1".to_string());
        let display = err.to_string();
        assert!(display.contains("E001"));
        assert!(display.contains("test"));
    }

    #[test]
    fn test_error_chain_display() {
        let mut chain = ErrorChain::new("root".to_string());
        chain.add_cause("cause1".to_string());
        let display = chain.to_string();
        assert!(display.contains("Root: root"));
    }

    #[test]
    fn test_wrapped_error_display() {
        let wrapped = WrappedError::new("io error".to_string(), "IO".to_string());
        let display = wrapped.to_string();
        assert!(display.contains("IO"));
        assert!(display.contains("io error"));
    }

    #[test]
    fn test_error_handler_default() {
        let handler = ErrorHandler::default();
        assert_eq!(handler.get_strategy(), RecoveryStrategy::LogContinue);
    }
}
