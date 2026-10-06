//! Sharing data between threads: reference counting with `Arc`, and locks.
//!
//! The locks are built on futexes: a thread that cannot take a lock asks
//! the kernel to put it to sleep until the lock's word changes.

use std::cell::UnsafeCell;
use std::cmp::Ordering as Order;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const SYS_FUTEX: i64 = 202;
const FUTEX_WAIT_PRIVATE: i64 = 128;
const FUTEX_WAKE_PRIVATE: i64 = 129;

/// Sleep while `word` still holds `expected`.
fn futex_wait(word: &AtomicU32, expected: u32) {
    unsafe {
        std::libc::syscall(SYS_FUTEX, word.as_ptr(), FUTEX_WAIT_PRIVATE, expected, 0usize, 0usize, 0u32);
    }
}

/// Wake up to `count` threads sleeping on `word`.
fn futex_wake(word: &AtomicU32, count: i32) {
    unsafe {
        std::libc::syscall(SYS_FUTEX, word.as_ptr(), FUTEX_WAKE_PRIVATE, count, 0usize, 0usize, 0u32);
    }
}

// -- Arc ------------------------------------------------------------------------------

/// The heap allocation behind every `Arc` and `Weak` to one value; the
/// counts work as `Rc`'s do, changed atomically.
struct ArcInner<T: ?Sized> {
    strong: AtomicUsize,
    weak: AtomicUsize,
    data: T,
}

/// A reference-counted pointer that threads can share.
#[rustc_nonnull_optimization_guaranteed]
pub struct Arc<T: ?Sized> {
    inner: *mut ArcInner<T>,
}

impl<T: ?Sized, U: ?Sized> std::ops::CoerceUnsized<Arc<U>> for Arc<T> {}

/// A reference to an `Arc`'s value that does not keep it alive.
pub struct Weak<T: ?Sized> {
    /// Null for a `Weak` made by `Weak::new`.
    inner: *mut ArcInner<T>,
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        let inner = ArcInner { strong: AtomicUsize::new(1), weak: AtomicUsize::new(1), data };
        Arc { inner: Box::into_raw(Box::new(inner)) }
    }

    /// The value, if this is its only strong reference.
    pub fn try_unwrap(this: Arc<T>) -> Result<T, Arc<T>> {
        if this.allocation().strong.compare_exchange(1, 0, Ordering::Acquire, Ordering::Relaxed).is_err() {
            return Err(this);
        }
        let inner = this.inner;
        std::mem::forget(this);
        unsafe {
            let data = std::ptr::read(&(*inner).data as *const T);
            release_weak(inner);
            Ok(data)
        }
    }

    pub fn into_inner(this: Arc<T>) -> Option<T> {
        Arc::try_unwrap(this).ok()
    }
}

/// Give up one weak reference to the allocation, freeing it with the last.
fn release_weak<T: ?Sized>(inner: *mut ArcInner<T>) {
    unsafe {
        if (*inner).weak.fetch_sub(1, Ordering::Release) == 1 {
            std::libc::free(inner as *mut u8);
        }
    }
}

impl<T: ?Sized> Arc<T> {
    fn allocation(&self) -> &ArcInner<T> {
        unsafe { &*self.inner }
    }

    pub fn strong_count(this: &Arc<T>) -> usize {
        this.allocation().strong.load(Ordering::SeqCst)
    }

    pub fn weak_count(this: &Arc<T>) -> usize {
        this.allocation().weak.load(Ordering::SeqCst) - 1
    }

    pub fn downgrade(this: &Arc<T>) -> Weak<T> {
        this.allocation().weak.fetch_add(1, Ordering::Acquire);
        Weak { inner: this.inner }
    }

    pub fn ptr_eq(this: &Arc<T>, other: &Arc<T>) -> bool {
        this.inner as *const u8 as usize == other.inner as *const u8 as usize
    }

    pub fn get_mut(this: &mut Arc<T>) -> Option<&mut T> {
        if Arc::strong_count(this) == 1 && Arc::weak_count(this) == 0 {
            Some(unsafe { &mut (*this.inner).data })
        } else {
            None
        }
    }

    pub fn as_ptr(this: &Arc<T>) -> *const T {
        unsafe { &(*this.inner).data as *const T }
    }
}

impl<T: Clone> Arc<T> {
    /// The value, mutably: cloned first if other references share it.
    pub fn make_mut(this: &mut Arc<T>) -> &mut T {
        if Arc::strong_count(this) != 1 || Arc::weak_count(this) != 0 {
            *this = Arc::new((**this).clone());
        }
        unsafe { &mut (*this.inner).data }
    }

    pub fn unwrap_or_clone(this: Arc<T>) -> T {
        match Arc::try_unwrap(this) {
            Ok(data) => data,
            Err(shared) => (*shared).clone(),
        }
    }
}

impl<T: ?Sized> Clone for Arc<T> {
    fn clone(&self) -> Arc<T> {
        self.allocation().strong.fetch_add(1, Ordering::Relaxed);
        Arc { inner: self.inner }
    }
}

impl<T: ?Sized> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.allocation().strong.fetch_sub(1, Ordering::Release) == 1 {
            unsafe { std::ptr::drop_in_place(&mut (*self.inner).data as *mut T) }
            release_weak(self.inner);
        }
    }
}

impl<T: ?Sized> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.allocation().data
    }
}

impl<T: ?Sized> AsRef<T> for Arc<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> std::borrow::Borrow<T> for Arc<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: Default> Default for Arc<T> {
    fn default() -> Arc<T> {
        Arc::new(T::default())
    }
}

impl<T> From<T> for Arc<T> {
    fn from(data: T) -> Arc<T> {
        Arc::new(data)
    }
}

impl<T: ?Sized + PartialEq> PartialEq for Arc<T> {
    fn eq(&self, other: &Arc<T>) -> bool {
        **self == **other
    }
}

impl<T: ?Sized + Eq> Eq for Arc<T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for Arc<T> {
    fn partial_cmp(&self, other: &Arc<T>) -> Option<Order> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: ?Sized + Ord> Ord for Arc<T> {
    fn cmp(&self, other: &Arc<T>) -> Order {
        (**self).cmp(&**other)
    }
}

impl<T: ?Sized + Hash> Hash for Arc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Arc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for Arc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T> Weak<T> {
    pub fn new() -> Weak<T> {
        Weak { inner: std::ptr::null_mut() }
    }
}

impl<T: ?Sized> Weak<T> {
    /// An `Arc` to the value, if it has not been dropped yet.
    pub fn upgrade(&self) -> Option<Arc<T>> {
        if self.inner.is_null() {
            return None;
        }
        let strong = unsafe { &(*self.inner).strong };
        let increased = strong.fetch_update(Ordering::Acquire, Ordering::Relaxed, |count| {
            if count == 0 { None } else { Some(count + 1) }
        });
        match increased {
            Ok(_) => Some(Arc { inner: self.inner }),
            Err(_) => None,
        }
    }

    pub fn strong_count(&self) -> usize {
        if self.inner.is_null() { 0 } else { unsafe { (*self.inner).strong.load(Ordering::SeqCst) } }
    }
}

impl<T: ?Sized> Clone for Weak<T> {
    fn clone(&self) -> Weak<T> {
        if !self.inner.is_null() {
            unsafe { (*self.inner).weak.fetch_add(1, Ordering::Relaxed) };
        }
        Weak { inner: self.inner }
    }
}

impl<T: ?Sized> Drop for Weak<T> {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            release_weak(self.inner);
        }
    }
}

impl<T: ?Sized> fmt::Debug for Weak<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("(Weak)")
    }
}

// -- poisoning ------------------------------------------------------------------------

/// What taking a lock gives: the guard, or the guard of a lock whose last
/// holder panicked. A panic here ends the process, so locks are never
/// poisoned.
pub type LockResult<Guard> = Result<Guard, PoisonError<Guard>>;

pub struct PoisonError<T> {
    guard: T,
}

impl<T> PoisonError<T> {
    pub fn new(guard: T) -> PoisonError<T> {
        PoisonError { guard }
    }

    pub fn into_inner(self) -> T {
        self.guard
    }

    pub fn get_ref(&self) -> &T {
        &self.guard
    }
}

impl<T> fmt::Debug for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("PoisonError { .. }")
    }
}

impl<T> fmt::Display for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("poisoned lock: another task failed inside")
    }
}

pub type TryLockResult<Guard> = Result<Guard, TryLockError<Guard>>;

pub enum TryLockError<T> {
    Poisoned(PoisonError<T>),
    WouldBlock,
}

impl<T> fmt::Debug for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TryLockError::Poisoned(_) => f.write_str("Poisoned(..)"),
            TryLockError::WouldBlock => f.write_str("WouldBlock"),
        }
    }
}

impl<T> fmt::Display for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TryLockError::Poisoned(_) => f.write_str("poisoned lock: another task failed inside"),
            TryLockError::WouldBlock => f.write_str("try_lock failed because the operation would block"),
        }
    }
}

// -- Mutex ----------------------------------------------------------------------------

const UNLOCKED: u32 = 0;
const LOCKED: u32 = 1;
/// Locked, and some thread may be asleep waiting for it.
const CONTENDED: u32 = 2;

/// Mutual exclusion: one thread at a time gets at the value.
pub struct Mutex<T: ?Sized> {
    state: AtomicU32,
    data: UnsafeCell<T>,
}

/// Access to a `Mutex`'s value; the lock is released when it is dropped.
pub struct MutexGuard<'a, T: ?Sized> {
    mutex: &'a Mutex<T>,
}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Mutex<T> {
        Mutex { state: AtomicU32::new(UNLOCKED), data: UnsafeCell::new(data) }
    }

    pub fn into_inner(self) -> LockResult<T> {
        Ok(self.data.into_inner())
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn lock(&self) -> LockResult<MutexGuard<'_, T>> {
        if self.state.compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed).is_err() {
            self.lock_contended();
        }
        Ok(MutexGuard { mutex: self })
    }

    /// Wait for the lock, saying there are waiters so that its holder wakes
    /// one when it lets go.
    fn lock_contended(&self) {
        while self.state.swap(CONTENDED, Ordering::Acquire) != UNLOCKED {
            futex_wait(&self.state, CONTENDED);
        }
    }

    pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> {
        match self.state.compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed) {
            Ok(_) => Ok(MutexGuard { mutex: self }),
            Err(_) => Err(TryLockError::WouldBlock),
        }
    }

    fn unlock(&self) {
        if self.state.swap(UNLOCKED, Ordering::Release) == CONTENDED {
            futex_wake(&self.state, 1);
        }
    }

    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        Ok(self.data.get_mut())
    }

    pub fn is_poisoned(&self) -> bool {
        false
    }

    pub fn clear_poison(&self) {}
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Mutex<T> {
        Mutex::new(T::default())
    }
}

impl<T> From<T> for Mutex<T> {
    fn from(data: T) -> Mutex<T> {
        Mutex::new(data)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut fields = f.debug_struct("Mutex");
        match self.try_lock() {
            Ok(guard) => fields.field("data", &&*guard),
            Err(_) => fields.field("data", &format_args!("<locked>")),
        };
        fields.field("poisoned", &false);
        fields.finish_non_exhaustive()
    }
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for MutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for MutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

// -- RwLock ---------------------------------------------------------------------------

/// The state word of a write-locked `RwLock`; otherwise it counts readers.
const WRITING: u32 = u32::MAX;

/// Any number of readers, or one writer, at a time.
pub struct RwLock<T: ?Sized> {
    state: AtomicU32,
    data: UnsafeCell<T>,
}

pub struct RwLockReadGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
}

pub struct RwLockWriteGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
}

impl<T> RwLock<T> {
    pub const fn new(data: T) -> RwLock<T> {
        RwLock { state: AtomicU32::new(0), data: UnsafeCell::new(data) }
    }

    pub fn into_inner(self) -> LockResult<T> {
        Ok(self.data.into_inner())
    }
}

impl<T: ?Sized> RwLock<T> {
    pub fn read(&self) -> LockResult<RwLockReadGuard<'_, T>> {
        loop {
            let readers = self.state.load(Ordering::Relaxed);
            if readers != WRITING
                && self.state.compare_exchange(readers, readers + 1, Ordering::Acquire, Ordering::Relaxed).is_ok()
            {
                return Ok(RwLockReadGuard { lock: self });
            }
            if readers == WRITING {
                futex_wait(&self.state, WRITING);
            }
        }
    }

    pub fn write(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
        loop {
            let readers = self.state.load(Ordering::Relaxed);
            if readers == 0 && self.state.compare_exchange(0, WRITING, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                return Ok(RwLockWriteGuard { lock: self });
            }
            futex_wait(&self.state, readers);
        }
    }

    pub fn try_read(&self) -> TryLockResult<RwLockReadGuard<'_, T>> {
        let readers = self.state.load(Ordering::Relaxed);
        if readers != WRITING && self.state.compare_exchange(readers, readers + 1, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            Ok(RwLockReadGuard { lock: self })
        } else {
            Err(TryLockError::WouldBlock)
        }
    }

    pub fn try_write(&self) -> TryLockResult<RwLockWriteGuard<'_, T>> {
        match self.state.compare_exchange(0, WRITING, Ordering::Acquire, Ordering::Relaxed) {
            Ok(_) => Ok(RwLockWriteGuard { lock: self }),
            Err(_) => Err(TryLockError::WouldBlock),
        }
    }

    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        Ok(self.data.get_mut())
    }

    pub fn is_poisoned(&self) -> bool {
        false
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> RwLock<T> {
        RwLock::new(T::default())
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut fields = f.debug_struct("RwLock");
        match self.try_read() {
            Ok(guard) => fields.field("data", &&*guard),
            Err(_) => fields.field("data", &format_args!("<locked>")),
        };
        fields.field("poisoned", &false);
        fields.finish_non_exhaustive()
    }
}

impl<'a, T: ?Sized> Deref for RwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        if self.lock.state.fetch_sub(1, Ordering::Release) == 1 {
            futex_wake(&self.lock.state, i32::MAX);
        }
    }
}

impl<'a, T: ?Sized> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.state.store(0, Ordering::Release);
        futex_wake(&self.lock.state, i32::MAX);
    }
}

// -- Condvar --------------------------------------------------------------------------

/// Waiting for a condition on data a `Mutex` guards.
///
/// Every notification bumps a counter; a waiter reads it before letting
/// go of the mutex and sleeps only while it is unchanged, so a
/// notification between the two is not lost.
pub struct Condvar {
    sequence: AtomicU32,
}

/// Whether `wait_timeout` gave up waiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitTimeoutResult(bool);

impl WaitTimeoutResult {
    pub fn timed_out(&self) -> bool {
        self.0
    }
}

impl Condvar {
    pub const fn new() -> Condvar {
        Condvar { sequence: AtomicU32::new(0) }
    }

    /// Release the mutex, sleep until notified, and take the mutex again.
    pub fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> LockResult<MutexGuard<'a, T>> {
        let mutex = guard.mutex;
        let sequence = self.sequence.load(Ordering::Relaxed);
        drop(guard);
        futex_wait(&self.sequence, sequence);
        mutex.lock()
    }

    /// Wait for as long as `condition` holds of the guarded value.
    pub fn wait_while<'a, T, F: FnMut(&mut T) -> bool>(
        &self,
        mut guard: MutexGuard<'a, T>,
        mut condition: F,
    ) -> LockResult<MutexGuard<'a, T>> {
        while condition(&mut *guard) {
            guard = self.wait(guard)?;
        }
        Ok(guard)
    }

    pub fn notify_one(&self) {
        self.sequence.fetch_add(1, Ordering::Release);
        futex_wake(&self.sequence, 1);
    }

    pub fn notify_all(&self) {
        self.sequence.fetch_add(1, Ordering::Release);
        futex_wake(&self.sequence, i32::MAX);
    }
}

impl Default for Condvar {
    fn default() -> Condvar {
        Condvar::new()
    }
}

impl fmt::Debug for Condvar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Condvar { .. }")
    }
}

// -- Once, OnceLock, Barrier ------------------------------------------------------------

const INCOMPLETE: u32 = 0;
const RUNNING: u32 = 1;
const COMPLETE: u32 = 2;

/// Running something exactly once, whichever thread gets there first.
pub struct Once {
    state: AtomicU32,
}

impl Once {
    pub const fn new() -> Once {
        Once { state: AtomicU32::new(INCOMPLETE) }
    }

    pub fn call_once<F: FnOnce()>(&self, body: F) {
        if self.state.load(Ordering::Acquire) == COMPLETE {
            return;
        }
        if self.state.compare_exchange(INCOMPLETE, RUNNING, Ordering::Acquire, Ordering::Acquire).is_ok() {
            body();
            self.state.store(COMPLETE, Ordering::Release);
            futex_wake(&self.state, i32::MAX);
            return;
        }
        while self.state.load(Ordering::Acquire) != COMPLETE {
            futex_wait(&self.state, RUNNING);
        }
    }

    pub fn is_completed(&self) -> bool {
        self.state.load(Ordering::Acquire) == COMPLETE
    }
}

/// A value set once and read afterwards from any thread.
pub struct OnceLock<T> {
    once: Once,
    value: UnsafeCell<Option<T>>,
}

impl<T> OnceLock<T> {
    pub const fn new() -> OnceLock<T> {
        OnceLock { once: Once::new(), value: UnsafeCell::new(None) }
    }

    pub fn get(&self) -> Option<&T> {
        if self.once.is_completed() {
            unsafe { (*self.value.get()).as_ref() }
        } else {
            None
        }
    }

    pub fn get_or_init<F: FnOnce() -> T>(&self, make: F) -> &T {
        self.once.call_once(|| unsafe { *self.value.get() = Some(make()) });
        self.get().expect("the value was just set")
    }

    /// Set the value, unless it is already set: then `value` is handed back.
    pub fn set(&self, value: T) -> Result<(), T> {
        let mut slot = Some(value);
        self.once.call_once(|| unsafe { *self.value.get() = slot.take() });
        match slot {
            None => Ok(()),
            Some(value) => Err(value),
        }
    }

    pub fn into_inner(self) -> Option<T> {
        self.value.into_inner()
    }
}

impl<T: fmt::Debug> fmt::Debug for OnceLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.get() {
            Some(value) => f.debug_tuple("OnceLock").field(value).finish(),
            None => f.write_str("OnceLock(<uninit>)"),
        }
    }
}

/// Makes a number of threads wait until all of them have arrived.
pub struct Barrier {
    state: Mutex<(usize, usize)>,
    arrived: Condvar,
    count: usize,
}

/// Whether this thread was the one that let the others go.
pub struct BarrierWaitResult(bool);

impl BarrierWaitResult {
    pub fn is_leader(&self) -> bool {
        self.0
    }
}

impl Barrier {
    pub fn new(count: usize) -> Barrier {
        Barrier { state: Mutex::new((0, 0)), arrived: Condvar::new(), count }
    }

    pub fn wait(&self) -> BarrierWaitResult {
        let mut state = self.state.lock().unwrap();
        let generation = state.1;
        state.0 += 1;
        if state.0 < self.count {
            while state.1 == generation {
                state = self.arrived.wait(state).unwrap();
            }
            BarrierWaitResult(false)
        } else {
            state.0 = 0;
            state.1 += 1;
            self.arrived.notify_all();
            BarrierWaitResult(true)
        }
    }
}
