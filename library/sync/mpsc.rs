//! Channels: values sent from any number of threads to one receiver.

use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

/// What both ends of a channel share: the values in flight, and whether
/// anyone is left at the other end.
struct Shared<T> {
    queue: Mutex<VecDeque<T>>,
    available: Condvar,
    senders: AtomicUsize,
    receiver_alive: AtomicBool,
}

/// The sending end; clone it to send from several threads.
pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

/// A sender for a channel made with `sync_channel`. Sends never block
/// here: the bound is not enforced.
pub type SyncSender<T> = Sender<T>;

/// The receiving end.
pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
}

/// A new channel's two ends.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared {
        queue: Mutex::new(VecDeque::new()),
        available: Condvar::new(),
        senders: AtomicUsize::new(1),
        receiver_alive: AtomicBool::new(true),
    });
    (Sender { shared: Arc::clone(&shared) }, Receiver { shared })
}

pub fn sync_channel<T>(bound: usize) -> (SyncSender<T>, Receiver<T>) {
    channel()
}

/// `send` found the receiver gone; the value is given back.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct SendError<T>(pub T);

impl<T> fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("SendError { .. }")
    }
}

impl<T> fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("sending on a closed channel")
    }
}

/// `recv` found the channel empty with every sender gone.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct RecvError;

impl fmt::Display for RecvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("receiving on a closed channel")
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TryRecvError {
    Empty,
    Disconnected,
}

impl fmt::Display for TryRecvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TryRecvError::Empty => f.write_str("receiving on an empty channel"),
            TryRecvError::Disconnected => f.write_str("receiving on a closed channel"),
        }
    }
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        if !self.shared.receiver_alive.load(Ordering::Acquire) {
            return Err(SendError(value));
        }
        self.shared.queue.lock().unwrap().push_back(value);
        self.shared.available.notify_one();
        Ok(())
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Sender<T> {
        self.shared.senders.fetch_add(1, Ordering::Relaxed);
        Sender { shared: Arc::clone(&self.shared) }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if self.shared.senders.fetch_sub(1, Ordering::AcqRel) == 1 {
            // The receiver may be waiting for a value that will not come.
            let _guard = self.shared.queue.lock().unwrap();
            self.shared.available.notify_all();
        }
    }
}

impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Sender { .. }")
    }
}

impl<T> Receiver<T> {
    /// The next value, waiting for one; an error once the channel is empty
    /// and no sender is left.
    pub fn recv(&self) -> Result<T, RecvError> {
        let mut queue = self.shared.queue.lock().unwrap();
        loop {
            if let Some(value) = queue.pop_front() {
                return Ok(value);
            }
            if self.shared.senders.load(Ordering::Acquire) == 0 {
                return Err(RecvError);
            }
            queue = self.shared.available.wait(queue).unwrap();
        }
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let mut queue = self.shared.queue.lock().unwrap();
        match queue.pop_front() {
            Some(value) => Ok(value),
            None if self.shared.senders.load(Ordering::Acquire) == 0 => Err(TryRecvError::Disconnected),
            None => Err(TryRecvError::Empty),
        }
    }

    /// The values, until every sender is gone.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { receiver: self }
    }

    /// The values already there, without waiting.
    pub fn try_iter(&self) -> TryIter<'_, T> {
        TryIter { receiver: self }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.shared.receiver_alive.store(false, Ordering::Release);
    }
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Receiver { .. }")
    }
}

pub struct Iter<'a, T> {
    receiver: &'a Receiver<T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.receiver.recv().ok()
    }
}

pub struct TryIter<'a, T> {
    receiver: &'a Receiver<T>,
}

impl<'a, T> Iterator for TryIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.receiver.try_recv().ok()
    }
}

pub struct IntoIter<T> {
    receiver: Receiver<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.receiver.recv().ok()
    }
}

impl<T> IntoIterator for Receiver<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> IntoIter<T> {
        IntoIter { receiver: self }
    }
}

impl<'a, T> IntoIterator for &'a Receiver<T> {
    type Item = T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}
