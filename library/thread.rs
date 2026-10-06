//! Threads of the operating system, each running a closure.
//!
//! A panic ends only the thread it happens on; `join` reports it as an
//! `Err` holding the panic's payload.

use std::any::Any;
use std::cell::UnsafeCell;
use std::fmt;
use std::sync::Mutex;
use std::time::Duration;

/// What `join` gives: the closure's result, or what it panicked with.
pub type Result<T> = std::result::Result<T, Box<dyn Any + Send + 'static>>;

/// Where a thread puts how its closure ended, for `join` to take.
type Slot<T> = UnsafeCell<Option<Result<T>>>;

/// The first thing a new thread runs: the closure handed to it and the
/// slot for its result, both made by `start_thread`.
fn start<F: FnOnce() -> T, T>(argument: *mut u8) -> *mut u8 {
    let (body, slot) = unsafe { std::ptr::read(argument as *const (F, *const Slot<T>)) };
    unsafe { std::libc::free(argument) };
    let result = std::panic::catch_unwind(body);
    unsafe { *(*slot).get() = Some(result) };
    std::ptr::null_mut()
}

/// Start a thread running `body`; returns its id and its result's slot.
fn start_thread<F: FnOnce() -> T, T>(body: F) -> (u64, *const Slot<T>) {
    let slot: *const Slot<T> = Box::into_raw(Box::new(UnsafeCell::new(None)));
    let argument = Box::into_raw(Box::new((body, slot))) as *mut u8;
    let entry: fn(*mut u8) -> *mut u8 = start::<F, T>;
    let mut thread = 0u64;
    let status = unsafe { std::libc::pthread_create(&mut thread, std::ptr::null(), entry, argument) };
    if status != 0 {
        panic!("failed to spawn thread: {}", std::io::Error::from_raw_os_error(status));
    }
    (thread, slot)
}

/// Wait for a thread to end and take its result.
fn finish_thread<T>(thread: u64, slot: *const Slot<T>) -> Result<T> {
    unsafe {
        std::libc::pthread_join(thread, std::ptr::null_mut());
        let slot = Box::from_raw(slot as *mut Slot<T>);
        match slot.into_inner() {
            Some(value) => value,
            None => std::intrinsics::unreachable(),
        }
    }
}

/// Run `body` on a new thread.
pub fn spawn<F: FnOnce() -> T + Send + 'static, T: Send + 'static>(body: F) -> JoinHandle<T> {
    let (thread, slot) = start_thread(body);
    JoinHandle { thread, slot, joined: false }
}

/// A running thread, to wait for and take the result of.
pub struct JoinHandle<T> {
    thread: u64,
    slot: *const Slot<T>,
    joined: bool,
}

impl<T> JoinHandle<T> {
    pub fn join(mut self) -> Result<T> {
        self.joined = true;
        finish_thread(self.thread, self.slot)
    }

    pub fn is_finished(&self) -> bool {
        unsafe { (*(*self.slot).get()).is_some() }
    }

    pub fn thread(&self) -> Thread {
        Thread { id: ThreadId(self.thread), name: None }
    }
}

impl<T> Drop for JoinHandle<T> {
    /// A thread not joined runs on by itself; its result is never taken.
    fn drop(&mut self) {
        if !self.joined {
            unsafe { std::libc::pthread_detach(self.thread) };
        }
    }
}

impl<T> fmt::Debug for JoinHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("JoinHandle { .. }")
    }
}

/// Threads that may borrow from the function that starts them, as all of
/// them have ended when `scope` returns.
pub fn scope<'env, F: FnOnce(&Scope<'_, 'env>) -> T, T>(body: F) -> T {
    let scope = Scope { running: Mutex::new(Vec::new()), _env: std::marker::PhantomData };
    let result = body(&scope);
    for thread in scope.running.into_inner().unwrap() {
        unsafe { std::libc::pthread_join(thread, std::ptr::null_mut()) };
    }
    result
}

pub struct Scope<'scope, 'env> {
    /// The threads not joined yet.
    running: Mutex<Vec<u64>>,
    _env: std::marker::PhantomData<&'env ()>,
}

impl<'scope, 'env> Scope<'scope, 'env> {
    pub fn spawn<F: FnOnce() -> T + Send, T: Send>(&self, body: F) -> ScopedJoinHandle<'scope, T> {
        let (thread, slot) = start_thread(body);
        self.running.lock().unwrap().push(thread);
        ScopedJoinHandle { thread, slot, running: &self.running }
    }
}

pub struct ScopedJoinHandle<'scope, T> {
    thread: u64,
    slot: *const Slot<T>,
    running: &'scope Mutex<Vec<u64>>,
}

impl<'scope, T> ScopedJoinHandle<'scope, T> {
    pub fn join(self) -> Result<T> {
        self.running.lock().unwrap().retain(|&thread| thread != self.thread);
        finish_thread(self.thread, self.slot)
    }
}

/// Put this thread to sleep for at least `duration`.
pub fn sleep(duration: Duration) {
    std::time::sleep_for(duration);
}

/// Give the processor to another thread that wants it.
pub fn yield_now() {
    unsafe { std::libc::sched_yield() };
}

/// A thread's identity.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ThreadId(u64);

/// A handle to a thread.
#[derive(Clone, Debug)]
pub struct Thread {
    id: ThreadId,
    name: Option<String>,
}

impl Thread {
    pub fn id(&self) -> ThreadId {
        self.id
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

/// The thread this is running on.
pub fn current() -> Thread {
    const GETPID: i64 = 39;
    const GETTID: i64 = 186;
    let id = unsafe { std::libc::pthread_self() };
    // The main thread is the one whose thread id is the process id.
    let is_main = unsafe { std::libc::syscall(GETTID) == std::libc::syscall(GETPID) };
    Thread { id: ThreadId(id), name: if is_main { Some(String::from("main")) } else { None } }
}

/// How many threads can usefully run at once: the processors online.
pub fn available_parallelism() -> std::io::Result<std::num::NonZeroUsize> {
    const PROCESSORS_ONLINE: i32 = 84;
    let count = unsafe { std::libc::sysconf(PROCESSORS_ONLINE) };
    match std::num::NonZeroUsize::new(count.max(1) as usize) {
        Some(count) => Ok(count),
        None => Err(std::io::Error::last_os_error()),
    }
}
