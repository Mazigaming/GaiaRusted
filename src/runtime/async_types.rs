//! # Phase 11: Async Types and Futures
//!
//! Provides type definitions and trait implementations for async/await support.
//!
//! ## Core Types
//! - **Future<Output = T>**: Trait for async computations
//! - **Pin<P>**: Pinning wrapper for self-referential types
//! - **Poll<T>**: Result of polling a future (Ready(T) or Pending)
//! - **Waker**: Task wakeup mechanism

use std::fmt;
use std::hash::Hash;

/// Trait representing an asynchronous computation
/// 
/// A Future represents a value that may not be ready yet but will eventually be available.
/// It can be polled to check if the value is ready or if execution should yield.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FutureType {
    /// Output type of the future when it completes
    pub output: Box<crate::typesystem::Type>,
}

impl FutureType {
    pub fn new(output: crate::typesystem::Type) -> Self {
        FutureType {
            output: Box::new(output),
        }
    }

    pub fn i32() -> Self {
        FutureType {
            output: Box::new(crate::typesystem::Type::I32),
        }
    }

    pub fn string() -> Self {
        FutureType {
            output: Box::new(crate::typesystem::Type::Str),
        }
    }

    pub fn unit() -> Self {
        FutureType {
            output: Box::new(crate::typesystem::Type::Unit),
        }
    }
}

impl fmt::Display for FutureType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "impl Future<Output = {}>", self.output)
    }
}

/// Result of polling a Future
/// 
/// Poll represents the result of attempting to progress an asynchronous task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Poll<T> {
    /// The value is ready
    Ready(T),
    /// The value is not yet ready
    Pending,
}

impl<T: fmt::Display> fmt::Display for Poll<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Poll::Ready(_) => write!(f, "Poll::Ready"),
            Poll::Pending => write!(f, "Poll::Pending"),
        }
    }
}

/// Pin is a wrapper type guaranteeing that a pointee will never be moved.
/// 
/// This is essential for futures and other self-referential types that cannot
/// be safely moved once their address is taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinType {
    /// The type being pinned
    pub inner: Box<crate::typesystem::Type>,
    /// Whether the pin is mutable
    pub mutable: bool,
}

impl PinType {
    pub fn new(inner: crate::typesystem::Type, mutable: bool) -> Self {
        PinType {
            inner: Box::new(inner),
            mutable,
        }
    }

    pub fn immutable(inner: crate::typesystem::Type) -> Self {
        PinType {
            inner: Box::new(inner),
            mutable: false,
        }
    }

    pub fn mutable(inner: crate::typesystem::Type) -> Self {
        PinType {
            inner: Box::new(inner),
            mutable: true,
        }
    }

    pub fn is_mutable(&self) -> bool {
        self.mutable
    }

    pub fn is_immutable(&self) -> bool {
        !self.mutable
    }
}

impl fmt::Display for PinType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.mutable {
            write!(f, "Pin<&mut {}>", self.inner)
        } else {
            write!(f, "Pin<&{}>", self.inner)
        }
    }
}

/// Waker is used to notify the executor that a future is ready to progress
#[derive(Debug, Clone)]
pub struct Waker {
    /// Task ID to wake
    pub task_id: usize,
    /// Whether this waker has been called
    pub called: bool,
}

impl Waker {
    pub fn new(task_id: usize) -> Self {
        Waker {
            task_id,
            called: false,
        }
    }

    pub fn wake(&mut self) {
        self.called = true;
    }

    pub fn was_called(&self) -> bool {
        self.called
    }
}

impl PartialEq for Waker {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
    }
}

impl Eq for Waker {}

impl Hash for Waker {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.task_id.hash(state);
    }
}

/// Context for executing async operations
/// 
/// Provides access to the current task's waker and other executor context.
#[derive(Debug, Clone)]
pub struct Context {
    /// Current task's waker
    pub waker: Waker,
}

impl Context {
    pub fn new(task_id: usize) -> Self {
        Context {
            waker: Waker::new(task_id),
        }
    }

    pub fn waker(&self) -> &Waker {
        &self.waker
    }

    pub fn waker_mut(&mut self) -> &mut Waker {
        &mut self.waker
    }
}

/// Trait implementation for async types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsyncTrait {
    /// Trait name (e.g., "Future", "Unpin")
    pub name: String,
    /// Type parameters (e.g., Output type for Future)
    pub params: Vec<crate::typesystem::Type>,
}

impl AsyncTrait {
    pub fn future(output: crate::typesystem::Type) -> Self {
        AsyncTrait {
            name: "Future".to_string(),
            params: vec![output],
        }
    }

    pub fn unpin() -> Self {
        AsyncTrait {
            name: "Unpin".to_string(),
            params: vec![],
        }
    }

    pub fn is_future(&self) -> bool {
        self.name == "Future"
    }

    pub fn is_unpin(&self) -> bool {
        self.name == "Unpin"
    }

    pub fn get_output_type(&self) -> Option<&crate::typesystem::Type> {
        if self.is_future() {
            self.params.first()
        } else {
            None
        }
    }
}

impl fmt::Display for AsyncTrait {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.params.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}<", self.name)?;
            for (i, param) in self.params.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", param)?;
            }
            write!(f, ">")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typesystem::Type;

    #[test]
    fn test_future_type_creation() {
        let fut = FutureType::new(Type::I32);
        assert_eq!(fut.output.as_ref(), &Type::I32);
    }

    #[test]
    fn test_future_type_display() {
        let fut = FutureType::i32();
        let display = format!("{}", fut);
        assert!(display.contains("Future"));
        assert!(display.contains("i32"));
    }

    #[test]
    fn test_poll_ready() {
        let poll: Poll<i32> = Poll::Ready(42);
        assert!(matches!(poll, Poll::Ready(42)));
    }

    #[test]
    fn test_poll_pending() {
        let poll: Poll<i32> = Poll::Pending;
        assert!(matches!(poll, Poll::Pending));
    }

    #[test]
    fn test_pin_immutable() {
        let pin = PinType::immutable(Type::I32);
        assert!(!pin.is_mutable());
        assert!(pin.is_immutable());
    }

    #[test]
    fn test_pin_mutable() {
        let pin = PinType::mutable(Type::I32);
        assert!(pin.is_mutable());
        assert!(!pin.is_immutable());
    }

    #[test]
    fn test_pin_display_immutable() {
        let pin = PinType::immutable(Type::I32);
        let display = format!("{}", pin);
        assert!(display.contains("Pin"));
        assert!(!display.contains("mut"));
    }

    #[test]
    fn test_pin_display_mutable() {
        let pin = PinType::mutable(Type::I32);
        let display = format!("{}", pin);
        assert!(display.contains("Pin"));
        assert!(display.contains("mut"));
    }

    #[test]
    fn test_waker_creation() {
        let waker = Waker::new(1);
        assert_eq!(waker.task_id, 1);
        assert!(!waker.was_called());
    }

    #[test]
    fn test_waker_wake() {
        let mut waker = Waker::new(1);
        waker.wake();
        assert!(waker.was_called());
    }

    #[test]
    fn test_waker_equality() {
        let waker1 = Waker::new(1);
        let waker2 = Waker::new(1);
        let waker3 = Waker::new(2);

        assert_eq!(waker1, waker2);
        assert_ne!(waker1, waker3);
    }

    #[test]
    fn test_context_creation() {
        let ctx = Context::new(1);
        assert_eq!(ctx.waker.task_id, 1);
    }

    #[test]
    fn test_async_trait_future() {
        let trait_ty = AsyncTrait::future(Type::I32);
        assert!(trait_ty.is_future());
        assert_eq!(trait_ty.get_output_type(), Some(&Type::I32));
    }

    #[test]
    fn test_async_trait_unpin() {
        let trait_ty = AsyncTrait::unpin();
        assert!(trait_ty.is_unpin());
        assert!(trait_ty.get_output_type().is_none());
    }

    #[test]
    fn test_async_trait_display() {
        let trait_ty = AsyncTrait::future(Type::I32);
        let display = format!("{}", trait_ty);
        assert!(display.contains("Future"));
    }

    #[test]
    fn test_poll_display() {
        let poll_ready = Poll::<i32>::Ready(0);
        assert!(format!("{}", poll_ready).contains("Ready"));

        let poll_pending = Poll::<i32>::Pending;
        assert!(format!("{}", poll_pending).contains("Pending"));
    }
}
