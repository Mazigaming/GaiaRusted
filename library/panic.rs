//! Panics, and catching them.
//!
//! A panic reports itself through the panic hook, then returns to the
//! innermost `catch_unwind` running on its thread with the panic's
//! payload; with none running, the program ends with status 101. That
//! return is a jump: the values owned by the functions a panic leaves are
//! not dropped. Their memory leaks, and a `Mutex` they locked stays locked.

use std::any::Any;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, OnceLock};

/// What `_setjmp` saves of the processor's state: 200 bytes on x86-64.
type JumpBuffer = [u64; 25];

/// A `catch_unwind` in progress, for a panic to return to.
struct Catcher {
    buffer: JumpBuffer,
    /// What the panic that returned here was raised with.
    payload: Option<Box<dyn Any + Send>>,
    /// The `catch_unwind` this one runs inside of, on the same thread.
    outer: *mut Catcher,
}

/// The thread-specific key under which each thread keeps its innermost
/// catcher.
static CATCHERS: OnceLock<u32> = OnceLock::new();

fn catchers_key() -> u32 {
    *CATCHERS.get_or_init(|| {
        let mut key = 0u32;
        unsafe { std::libc::pthread_key_create(&mut key, std::ptr::null()) };
        key
    })
}

fn innermost_catcher() -> *mut Catcher {
    unsafe { std::libc::pthread_getspecific(catchers_key()) as *mut Catcher }
}

fn set_innermost_catcher(catcher: *mut Catcher) {
    unsafe { std::libc::pthread_setspecific(catchers_key(), catcher as *const u8) };
}

/// Code that `catch_unwind` can run: a closure, or one wrapped in
/// [`AssertUnwindSafe`].
pub trait UnwindBody<R> {
    fn run(self) -> R;
}

impl<R, F: FnOnce() -> R> UnwindBody<R> for F {
    fn run(self) -> R {
        self()
    }
}

impl<R, F: FnOnce() -> R> UnwindBody<R> for AssertUnwindSafe<F> {
    fn run(self) -> R {
        (self.0)()
    }
}

/// Run `body`; a panic in it comes back as `Err` with its payload.
pub fn catch_unwind<B: UnwindBody<R>, R>(body: B) -> std::thread::Result<R> {
    let mut catcher = Catcher { buffer: [0; 25], payload: None, outer: innermost_catcher() };
    let mut body = Some(body);
    let mut result = None;
    if run_caught(&mut catcher, &mut body, &mut result) {
        return Err(catcher.payload.take().expect("a panic leaves its payload with its catcher"));
    }
    match result {
        Some(value) => Ok(value),
        None => unreachable!("a body that did not panic has returned"),
    }
}

/// Run the body with `catcher` innermost; returns whether a panic ended
/// it. Everything here that outlives the jump back lives in the caller's
/// frame, behind the pointers.
fn run_caught<B: UnwindBody<R>, R>(catcher: *mut Catcher, body: &mut Option<B>, result: &mut Option<R>) -> bool {
    unsafe {
        if std::libc::_setjmp((*catcher).buffer.as_mut_ptr()) != 0 {
            set_innermost_catcher((*catcher).outer);
            return true;
        }
    }
    set_innermost_catcher(catcher);
    if let Some(body) = body.take() {
        *result = Some(body.run());
    }
    set_innermost_catcher(unsafe { (*catcher).outer });
    false
}

/// Report a panic through the hook, then leave for the innermost
/// `catch_unwind`.
pub fn begin_panic(payload: Box<dyn Any + Send>, location: &str) -> ! {
    {
        let info = PanicHookInfo { payload: &*payload, location: Location::parse(location) };
        let hook = HOOK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match &*hook {
            Some(hook) => hook(&info),
            None => default_hook(&info),
        }
    }
    resume_unwind(payload)
}

/// Continue a panic without reporting it again: return to the innermost
/// `catch_unwind`, or end the program.
pub fn resume_unwind(payload: Box<dyn Any + Send>) -> ! {
    let catcher = innermost_catcher();
    if catcher.is_null() {
        unsafe { std::libc::exit(101) }
    }
    unsafe {
        (*catcher).payload = Some(payload);
        std::libc::longjmp((*catcher).buffer.as_mut_ptr(), 1)
    }
}

/// Panic with any value as the payload.
pub fn panic_any<M: Any + Send>(payload: M) -> ! {
    begin_panic(Box::new(payload), "<unknown>")
}

type Hook = Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>;

/// What reports panics; `None` for the default report.
static HOOK: Mutex<Option<Hook>> = Mutex::new(None);

/// Report panics with `hook` from now on.
pub fn set_hook(hook: Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>) {
    *HOOK.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(hook);
}

/// Go back to the default report, returning the hook that was set.
pub fn take_hook() -> Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static> {
    match HOOK.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take() {
        Some(hook) => hook,
        None => Box::new(default_hook),
    }
}

/// The report rustc's programs give: the thread, where, and the message.
fn default_hook(info: &PanicHookInfo<'_>) {
    let thread = std::thread::current();
    let name = thread.name().unwrap_or("<unnamed>");
    eprintln!("thread '{}' {}", name, info);
    eprintln!("note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace");
}

/// A panic, as the hook is told about it.
pub struct PanicHookInfo<'a> {
    payload: &'a (dyn Any + Send),
    location: Location<'a>,
}

pub type PanicInfo<'a> = PanicHookInfo<'a>;

impl<'a> PanicHookInfo<'a> {
    pub fn payload(&self) -> &(dyn Any + Send) {
        self.payload
    }

    /// The message, when the panic was raised with one.
    pub fn payload_as_str(&self) -> Option<&str> {
        if let Some(message) = self.payload.downcast_ref::<&'static str>() {
            Some(*message)
        } else if let Some(message) = self.payload.downcast_ref::<String>() {
            Some(message.as_str())
        } else {
            None
        }
    }

    pub fn location(&self) -> Option<&Location<'a>> {
        Some(&self.location)
    }
}

impl fmt::Display for PanicHookInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "panicked at {}:", self.location)?;
        if let Some(message) = self.payload_as_str() {
            write!(f, "\n{}", message)?;
        }
        Ok(())
    }
}

impl fmt::Debug for PanicHookInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PanicHookInfo").field("location", &self.location).finish_non_exhaustive()
    }
}

/// Where in the source a panic was raised.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Location<'a> {
    file: &'a str,
    line: u32,
    col: u32,
}

impl<'a> Location<'a> {
    /// A location written `file:line:column`.
    fn parse(text: &'a str) -> Location<'a> {
        let mut parts = text.rsplitn(3, ':');
        let col = parts.next().and_then(|col| col.parse().ok());
        let line = parts.next().and_then(|line| line.parse().ok());
        match (parts.next(), line, col) {
            (Some(file), Some(line), Some(col)) => Location { file, line, col },
            _ => Location { file: text, line: 0, col: 0 },
        }
    }

    pub fn file(&self) -> &'a str {
        self.file
    }

    pub fn line(&self) -> u32 {
        self.line
    }

    pub fn column(&self) -> u32 {
        self.col
    }
}

impl fmt::Display for Location<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.col)
    }
}

/// Types whose values are left consistent when a panic interrupts code
/// using them. Every type counts here, as nothing is checked.
pub trait UnwindSafe {}

/// Types whose shared references are [`UnwindSafe`].
pub trait RefUnwindSafe {}

impl<T: ?Sized> UnwindSafe for T {}
impl<T: ?Sized> RefUnwindSafe for T {}

/// A value declared unwind safe whatever its type.
pub struct AssertUnwindSafe<T>(pub T);

impl<T> Deref for AssertUnwindSafe<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for AssertUnwindSafe<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: fmt::Debug> fmt::Debug for AssertUnwindSafe<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("AssertUnwindSafe").field(&self.0).finish()
    }
}
