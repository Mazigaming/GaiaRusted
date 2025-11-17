use std::panic::{PanicHookInfo, catch_unwind, AssertUnwindSafe, UnwindSafe};
use std::cell::RefCell;

thread_local! {
    static PANIC_CONTEXT: RefCell<PanicContext> = RefCell::new(PanicContext::new());
}

pub struct PanicContext {
    message: Option<String>,
    backtrace: std::vec::Vec<String>,
    is_unwinding: bool,
    panic_count: usize,
}

impl PanicContext {
    fn new() -> Self {
        PanicContext {
            message: None,
            backtrace: std::vec::Vec::new(),
            is_unwinding: false,
            panic_count: 0,
        }
    }

    pub fn set_panic_message(&mut self, message: String) {
        self.message = Some(message);
        self.is_unwinding = true;
        self.panic_count += 1;
    }

    pub fn push_frame(&mut self, frame: String) {
        self.backtrace.push(frame);
    }

    pub fn pop_frame(&mut self) {
        if !self.backtrace.is_empty() {
            self.backtrace.pop();
        }
    }

    pub fn clear(&mut self) {
        self.message = None;
        self.backtrace.clear();
        self.is_unwinding = false;
    }

    pub fn get_message(&self) -> Option<&String> {
        self.message.as_ref()
    }

    pub fn is_panicking(&self) -> bool {
        self.is_unwinding
    }

    pub fn panic_count(&self) -> usize {
        self.panic_count
    }

    pub fn backtrace(&self) -> &[String] {
        &self.backtrace
    }
}

pub struct PanicHandler {
    enabled: bool,
    catch_panics: bool,
    handler_hook: Option<Box<dyn Fn(&PanicHookInfo) + Send + Sync>>,
}

impl PanicHandler {
    pub fn new() -> Self {
        PanicHandler {
            enabled: true,
            catch_panics: true,
            handler_hook: None,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_catch_panics(&mut self, catch: bool) {
        self.catch_panics = catch;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn can_catch_panics(&self) -> bool {
        self.catch_panics
    }

    pub fn handle_panic(&self, message: String) -> Result<(), String> {
        if !self.enabled {
            return Err(message);
        }

        PANIC_CONTEXT.with(|ctx| {
            let mut context = ctx.borrow_mut();
            context.set_panic_message(message.clone());
        });

        if self.catch_panics {
            Ok(())
        } else {
            Err(message)
        }
    }

    pub fn with_unwind_protection<F, R>(f: F) -> Result<R, String>
    where
        F: FnOnce() -> R + UnwindSafe,
    {
        match catch_unwind(AssertUnwindSafe(f)) {
            Ok(result) => Ok(result),
            Err(_) => {
                let message = PANIC_CONTEXT.with(|ctx| {
                    let context = ctx.borrow();
                    context.get_message().cloned().unwrap_or_else(|| "Unknown panic".to_string())
                });
                Err(message)
            }
        }
    }
}

pub struct UnwindStack {
    frames: std::vec::Vec<UnwindFrame>,
}

pub struct UnwindFrame {
    pub function_name: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl UnwindStack {
    pub fn new() -> Self {
        UnwindStack {
            frames: std::vec::Vec::new(),
        }
    }

    pub fn push_frame(&mut self, frame: UnwindFrame) {
        self.frames.push(frame);
    }

    pub fn pop_frame(&mut self) -> Option<UnwindFrame> {
        self.frames.pop()
    }

    pub fn current_frame(&self) -> Option<&UnwindFrame> {
        self.frames.last()
    }

    pub fn frames(&self) -> &[UnwindFrame] {
        &self.frames
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }

    pub fn format_backtrace(&self) -> String {
        let mut output = String::from("Stack backtrace:\n");
        for (idx, frame) in self.frames.iter().enumerate() {
            output.push_str(&format!(
                "  {} at {}:{}:{} in {}\n",
                idx, frame.file, frame.line, frame.column, frame.function_name
            ));
        }
        output
    }
}

pub enum UnwindReason {
    Panic(String),
    Return,
    Break,
    Continue,
}

pub struct UnwindResult {
    reason: UnwindReason,
    backtrace: UnwindStack,
}

impl UnwindResult {
    pub fn new_panic(message: String) -> Self {
        UnwindResult {
            reason: UnwindReason::Panic(message),
            backtrace: UnwindStack::new(),
        }
    }

    pub fn new_return() -> Self {
        UnwindResult {
            reason: UnwindReason::Return,
            backtrace: UnwindStack::new(),
        }
    }

    pub fn new_break() -> Self {
        UnwindResult {
            reason: UnwindReason::Break,
            backtrace: UnwindStack::new(),
        }
    }

    pub fn new_continue() -> Self {
        UnwindResult {
            reason: UnwindReason::Continue,
            backtrace: UnwindStack::new(),
        }
    }

    pub fn reason(&self) -> &UnwindReason {
        &self.reason
    }

    pub fn backtrace(&self) -> &UnwindStack {
        &self.backtrace
    }

    pub fn backtrace_mut(&mut self) -> &mut UnwindStack {
        &mut self.backtrace
    }

    pub fn is_panic(&self) -> bool {
        matches!(self.reason, UnwindReason::Panic(_))
    }

    pub fn panic_message(&self) -> Option<&String> {
        match &self.reason {
            UnwindReason::Panic(msg) => Some(msg),
            _ => None,
        }
    }
}

pub fn safe_call<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce() -> R + UnwindSafe,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => Ok(result),
        Err(err) => {
            let message = if let Some(s) = err.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = err.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic occurred".to_string()
            };
            Err(message)
        }
    }
}

pub fn global_panic_context() -> PanicContext {
    PANIC_CONTEXT.with(|ctx| ctx.borrow().clone())
}

impl Clone for PanicContext {
    fn clone(&self) -> Self {
        PanicContext {
            message: self.message.clone(),
            backtrace: self.backtrace.clone(),
            is_unwinding: self.is_unwinding,
            panic_count: self.panic_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panic_handler_creation() {
        let handler = PanicHandler::new();
        assert!(handler.is_enabled());
        assert!(handler.can_catch_panics());
    }

    #[test]
    fn test_panic_handler_set_enabled() {
        let mut handler = PanicHandler::new();
        handler.set_enabled(false);
        assert!(!handler.is_enabled());
        handler.set_enabled(true);
        assert!(handler.is_enabled());
    }

    #[test]
    fn test_panic_handler_catch_panics() {
        let mut handler = PanicHandler::new();
        handler.set_catch_panics(false);
        assert!(!handler.can_catch_panics());
        handler.set_catch_panics(true);
        assert!(handler.can_catch_panics());
    }

    #[test]
    fn test_panic_context_set_message() {
        let mut context = PanicContext::new();
        context.set_panic_message("Test panic".to_string());
        assert_eq!(context.get_message(), Some(&"Test panic".to_string()));
        assert!(context.is_panicking());
    }

    #[test]
    fn test_panic_context_panic_count() {
        let mut context = PanicContext::new();
        assert_eq!(context.panic_count(), 0);
        context.set_panic_message("First panic".to_string());
        assert_eq!(context.panic_count(), 1);
        context.set_panic_message("Second panic".to_string());
        assert_eq!(context.panic_count(), 2);
    }

    #[test]
    fn test_unwind_stack_operations() {
        let mut stack = UnwindStack::new();
        assert_eq!(stack.depth(), 0);

        let frame1 = UnwindFrame {
            function_name: "test_func".to_string(),
            file: "test.rs".to_string(),
            line: 42,
            column: 10,
        };

        stack.push_frame(frame1);
        assert_eq!(stack.depth(), 1);
        assert!(stack.current_frame().is_some());

        let popped = stack.pop_frame();
        assert!(popped.is_some());
        assert_eq!(stack.depth(), 0);
    }

    #[test]
    fn test_unwind_stack_backtrace_format() {
        let mut stack = UnwindStack::new();
        stack.push_frame(UnwindFrame {
            function_name: "main".to_string(),
            file: "main.rs".to_string(),
            line: 1,
            column: 0,
        });
        stack.push_frame(UnwindFrame {
            function_name: "foo".to_string(),
            file: "lib.rs".to_string(),
            line: 10,
            column: 5,
        });

        let backtrace = stack.format_backtrace();
        assert!(backtrace.contains("Stack backtrace"));
        assert!(backtrace.contains("main"));
        assert!(backtrace.contains("foo"));
    }

    #[test]
    fn test_unwind_result_panic() {
        let result = UnwindResult::new_panic("error message".to_string());
        assert!(result.is_panic());
        assert_eq!(result.panic_message(), Some(&"error message".to_string()));
    }

    #[test]
    fn test_unwind_result_return() {
        let result = UnwindResult::new_return();
        assert!(!result.is_panic());
        assert!(result.panic_message().is_none());
    }

    #[test]
    fn test_safe_call_success() {
        let result: Result<i32, String> = safe_call(|| 42);
        assert_eq!(result, Ok(42));
    }

    #[test]
    fn test_safe_call_with_function() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let result: Result<i32, String> = safe_call(|| add(3, 4));
        assert_eq!(result, Ok(7));
    }

    #[test]
    fn test_panic_handler_handle_panic() {
        let handler = PanicHandler::new();
        let result = handler.handle_panic("test error".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_panic_handler_handle_panic_disabled() {
        let mut handler = PanicHandler::new();
        handler.set_enabled(false);
        let result = handler.handle_panic("test error".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_unwind_stack_clear() {
        let mut stack = UnwindStack::new();
        stack.push_frame(UnwindFrame {
            function_name: "test".to_string(),
            file: "test.rs".to_string(),
            line: 1,
            column: 0,
        });
        assert_eq!(stack.depth(), 1);
        stack.clear();
        assert_eq!(stack.depth(), 0);
    }

    #[test]
    fn test_unwind_stack_multiple_frames() {
        let mut stack = UnwindStack::new();
        for i in 0..5 {
            stack.push_frame(UnwindFrame {
                function_name: format!("func_{}", i),
                file: format!("file_{}.rs", i),
                line: i as usize * 10,
                column: i as usize,
            });
        }
        assert_eq!(stack.depth(), 5);
        assert_eq!(stack.frames().len(), 5);
    }

    #[test]
    fn test_panic_context_backtrace() {
        let mut context = PanicContext::new();
        context.push_frame("frame1".to_string());
        context.push_frame("frame2".to_string());
        assert_eq!(context.backtrace().len(), 2);
        context.pop_frame();
        assert_eq!(context.backtrace().len(), 1);
    }

    #[test]
    fn test_unwind_result_variants() {
        let panic_result = UnwindResult::new_panic("test".to_string());
        let return_result = UnwindResult::new_return();
        let break_result = UnwindResult::new_break();
        let continue_result = UnwindResult::new_continue();

        assert!(panic_result.is_panic());
        assert!(!return_result.is_panic());
        assert!(!break_result.is_panic());
        assert!(!continue_result.is_panic());
    }
}
