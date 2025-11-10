//! # Phase 11: Async Synchronization Primitives
//!
//! Provides channels, mutexes, and other primitives for async task coordination.
//!
//! ## Components
//! - **Channel**: Multi-producer, single-consumer message queue
//! - **Mutex**: Async-safe mutual exclusion lock
//! - **RwLock**: Async-safe read-write lock
//! - **Barrier**: Synchronization barrier for N tasks

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex as StdMutex};

/// Error type for sync operations
#[derive(Debug, Clone)]
pub struct SyncError {
    pub message: String,
    pub kind: SyncErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncErrorKind {
    /// Channel is closed
    ChannelClosed,
    /// Mutex is poisoned
    MutexPoisoned,
    /// Lock acquisition failed
    LockFailed,
    /// Timeout occurred
    Timeout,
    /// Buffer is full
    BufferFull,
    /// Buffer is empty
    BufferEmpty,
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SyncError: {}", self.message)
    }
}

pub type SyncResult<T> = Result<T, SyncError>;

/// Multi-producer, single-consumer channel
pub struct MpscChannel<T> {
    sender_count: usize,
    queue: Arc<StdMutex<VecDeque<T>>>,
    closed: Arc<StdMutex<bool>>,
    capacity: usize,
}

impl<T> MpscChannel<T> {
    pub fn new(capacity: usize) -> Self {
        MpscChannel {
            sender_count: 1,
            queue: Arc::new(StdMutex::new(VecDeque::new())),
            closed: Arc::new(StdMutex::new(false)),
            capacity,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn is_closed(&self) -> SyncResult<bool> {
        self.closed.lock()
            .map(|guard| *guard)
            .map_err(|_| SyncError {
                message: "mutex poisoned".to_string(),
                kind: SyncErrorKind::MutexPoisoned,
            })
    }

    pub fn sender_count(&self) -> usize {
        self.sender_count
    }
}

impl<T: Clone> Clone for MpscChannel<T> {
    fn clone(&self) -> Self {
        MpscChannel {
            sender_count: self.sender_count + 1,
            queue: Arc::clone(&self.queue),
            closed: Arc::clone(&self.closed),
            capacity: self.capacity,
        }
    }
}

/// Sender half of the channel
pub struct Sender<T> {
    channel: MpscChannel<T>,
}

impl<T> Sender<T> {
    pub fn new(channel: MpscChannel<T>) -> Self {
        Sender { channel }
    }

    pub fn send(&self, value: T) -> SyncResult<()> {
        let closed = self.channel.is_closed()?;
        if closed {
            return Err(SyncError {
                message: "channel is closed".to_string(),
                kind: SyncErrorKind::ChannelClosed,
            });
        }

        let mut queue = self.channel.queue.lock().map_err(|_| SyncError {
            message: "mutex poisoned".to_string(),
            kind: SyncErrorKind::MutexPoisoned,
        })?;

        if queue.len() >= self.channel.capacity {
            return Err(SyncError {
                message: "channel buffer is full".to_string(),
                kind: SyncErrorKind::BufferFull,
            });
        }

        queue.push_back(value);
        Ok(())
    }

    pub fn close(&self) -> SyncResult<()> {
        self.channel.closed.lock()
            .map(|mut guard| *guard = true)
            .map_err(|_| SyncError {
                message: "mutex poisoned".to_string(),
                kind: SyncErrorKind::MutexPoisoned,
            })
    }
}

impl<T: Clone> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender {
            channel: self.channel.clone(),
        }
    }
}

/// Receiver half of the channel
pub struct Receiver<T> {
    channel: MpscChannel<T>,
}

impl<T> Receiver<T> {
    pub fn new(channel: MpscChannel<T>) -> Self {
        Receiver { channel }
    }

    pub fn recv(&mut self) -> SyncResult<T> {
        let mut queue = self.channel.queue.lock().map_err(|_| SyncError {
            message: "mutex poisoned".to_string(),
            kind: SyncErrorKind::MutexPoisoned,
        })?;

        queue.pop_front().ok_or(SyncError {
            message: "channel is empty".to_string(),
            kind: SyncErrorKind::BufferEmpty,
        })
    }

    pub fn try_recv(&mut self) -> SyncResult<Option<T>> {
        let mut queue = self.channel.queue.lock().map_err(|_| SyncError {
            message: "mutex poisoned".to_string(),
            kind: SyncErrorKind::MutexPoisoned,
        })?;

        Ok(queue.pop_front())
    }

    pub fn pending(&self) -> SyncResult<usize> {
        self.channel.queue.lock()
            .map(|guard| guard.len())
            .map_err(|_| SyncError {
                message: "mutex poisoned".to_string(),
                kind: SyncErrorKind::MutexPoisoned,
            })
    }
}

/// Async-aware mutual exclusion lock
pub struct AsyncMutex<T> {
    data: Arc<StdMutex<T>>,
}

impl<T> AsyncMutex<T> {
    pub fn new(data: T) -> Self {
        AsyncMutex {
            data: Arc::new(StdMutex::new(data)),
        }
    }

    pub fn lock(&self) -> SyncResult<T>
    where
        T: Clone,
    {
        self.data.lock()
            .map(|guard| guard.clone())
            .map_err(|_| SyncError {
                message: "failed to acquire lock".to_string(),
                kind: SyncErrorKind::LockFailed,
            })
    }

    pub fn try_lock(&self) -> SyncResult<Option<T>>
    where
        T: Clone,
    {
        match self.data.try_lock() {
            Ok(guard) => Ok(Some(guard.clone())),
            Err(_) => Ok(None),
        }
    }
}

impl<T: Clone> Clone for AsyncMutex<T> {
    fn clone(&self) -> Self {
        AsyncMutex {
            data: Arc::clone(&self.data),
        }
    }
}

/// Async-aware read-write lock
pub struct AsyncRwLock<T> {
    data: Arc<StdMutex<T>>,
    readers: Arc<StdMutex<usize>>,
}

impl<T> AsyncRwLock<T> {
    pub fn new(data: T) -> Self {
        AsyncRwLock {
            data: Arc::new(StdMutex::new(data)),
            readers: Arc::new(StdMutex::new(0)),
        }
    }

    pub fn read(&self) -> SyncResult<T>
    where
        T: Clone,
    {
        let mut readers = self.readers.lock().map_err(|_| SyncError {
            message: "failed to acquire read lock".to_string(),
            kind: SyncErrorKind::LockFailed,
        })?;

        *readers += 1;

        let data = self.data.lock()
            .map(|guard| guard.clone())
            .map_err(|_| SyncError {
                message: "failed to read data".to_string(),
                kind: SyncErrorKind::LockFailed,
            })?;

        *readers -= 1;
        Ok(data)
    }

    pub fn write(&self) -> SyncResult<T>
    where
        T: Clone,
    {
        self.data.lock()
            .map(|guard| guard.clone())
            .map_err(|_| SyncError {
                message: "failed to acquire write lock".to_string(),
                kind: SyncErrorKind::LockFailed,
            })
    }

    pub fn reader_count(&self) -> SyncResult<usize> {
        self.readers.lock()
            .map(|guard| *guard)
            .map_err(|_| SyncError {
                message: "failed to get reader count".to_string(),
                kind: SyncErrorKind::LockFailed,
            })
    }
}

impl<T: Clone> Clone for AsyncRwLock<T> {
    fn clone(&self) -> Self {
        AsyncRwLock {
            data: Arc::clone(&self.data),
            readers: Arc::clone(&self.readers),
        }
    }
}

/// Synchronization barrier for N tasks
pub struct Barrier {
    count: Arc<StdMutex<usize>>,
    total: usize,
    waiting: Arc<StdMutex<Vec<usize>>>,
}

impl Barrier {
    pub fn new(total: usize) -> Self {
        Barrier {
            count: Arc::new(StdMutex::new(0)),
            total,
            waiting: Arc::new(StdMutex::new(Vec::new())),
        }
    }

    pub fn wait(&self, task_id: usize) -> SyncResult<bool> {
        let mut count = self.count.lock().map_err(|_| SyncError {
            message: "barrier mutex poisoned".to_string(),
            kind: SyncErrorKind::MutexPoisoned,
        })?;

        *count += 1;
        let mut waiting = self.waiting.lock().map_err(|_| SyncError {
            message: "waiting list mutex poisoned".to_string(),
            kind: SyncErrorKind::MutexPoisoned,
        })?;
        waiting.push(task_id);

        if *count >= self.total {
            *count = 0;
            waiting.clear();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn waiting_count(&self) -> SyncResult<usize> {
        self.waiting.lock()
            .map(|guard| guard.len())
            .map_err(|_| SyncError {
                message: "failed to get waiting count".to_string(),
                kind: SyncErrorKind::MutexPoisoned,
            })
    }
}

impl Clone for Barrier {
    fn clone(&self) -> Self {
        Barrier {
            count: Arc::clone(&self.count),
            total: self.total,
            waiting: Arc::clone(&self.waiting),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mpsc_channel_creation() {
        let channel: MpscChannel<i32> = MpscChannel::new(10);
        assert_eq!(channel.capacity(), 10);
    }

    #[test]
    fn test_sender_send() {
        let channel = MpscChannel::new(10);
        let sender = Sender::new(channel.clone());
        assert!(sender.send(42).is_ok());
    }

    #[test]
    fn test_receiver_recv() {
        let channel = MpscChannel::new(10);
        let sender = Sender::new(channel.clone());
        sender.send(42).unwrap();

        let mut receiver = Receiver::new(channel);
        let value = receiver.recv().unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_channel_closed() {
        let channel = MpscChannel::new(10);
        let sender = Sender::new(channel.clone());
        sender.close().unwrap();
        assert!(sender.send(42).is_err());
    }

    #[test]
    fn test_receiver_empty() {
        let channel = MpscChannel::<i32>::new(10);
        let mut receiver = Receiver::new(channel);
        let result = receiver.recv();
        assert!(result.is_err());
    }

    #[test]
    fn test_receiver_try_recv_empty() {
        let channel = MpscChannel::<i32>::new(10);
        let mut receiver = Receiver::new(channel);
        let result = receiver.try_recv().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_async_mutex_lock() {
        let mutex = AsyncMutex::new(42);
        let value = mutex.lock().unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_async_mutex_clone() {
        let mutex = AsyncMutex::new(42);
        let mutex2 = mutex.clone();
        let value = mutex2.lock().unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_async_rwlock_read() {
        let rwlock = AsyncRwLock::new(42);
        let value = rwlock.read().unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_async_rwlock_write() {
        let rwlock = AsyncRwLock::new(42);
        let value = rwlock.write().unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_barrier_creation() {
        let barrier = Barrier::new(3);
        let count = barrier.waiting_count().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_barrier_wait() {
        let barrier = Barrier::new(2);
        let result1 = barrier.wait(0).unwrap();
        let result2 = barrier.wait(1).unwrap();

        assert!(!result1);
        assert!(result2);
    }

    #[test]
    fn test_sender_clone() {
        let channel = MpscChannel::new(10);
        let sender1 = Sender::new(channel.clone());
        let sender2 = sender1.clone();

        sender1.send(42).unwrap();
        sender2.send(43).unwrap();

        let mut receiver = Receiver::new(channel);
        assert_eq!(receiver.recv().unwrap(), 42);
        assert_eq!(receiver.recv().unwrap(), 43);
    }

    #[test]
    fn test_receiver_pending() {
        let channel = MpscChannel::new(10);
        let sender = Sender::new(channel.clone());
        sender.send(42).unwrap();
        sender.send(43).unwrap();

        let receiver = Receiver::new(channel);
        let count = receiver.pending().unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_sync_error_display() {
        let error = SyncError {
            message: "test error".to_string(),
            kind: SyncErrorKind::ChannelClosed,
        };
        let display = format!("{}", error);
        assert!(display.contains("test error"));
    }

    #[test]
    fn test_channel_buffer_full() {
        let channel = MpscChannel::new(1);
        let sender = Sender::new(channel.clone());
        sender.send(42).unwrap();
        let result = sender.send(43);
        assert!(result.is_err());
    }
}
