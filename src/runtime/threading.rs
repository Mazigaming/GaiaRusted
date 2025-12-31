//! Threading and concurrency support
//!
//! Provides multi-threaded execution capabilities

use std::sync::{Arc, Mutex, mpsc};
use std::thread;

pub struct Thread<T> {
    id: usize,
    data: Arc<Mutex<T>>,
}

impl<T: Send + 'static> Thread<T> {
    pub fn new(initial_data: T) -> Self {
        static mut NEXT_ID: usize = 0;
        let id = unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        };

        Thread {
            id,
            data: Arc::new(Mutex::new(initial_data)),
        }
    }

    pub fn spawn_with<F, R>(f: F) -> thread::JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        thread::spawn(f)
    }

    pub fn lock(&self) -> Result<std::sync::MutexGuard<T>, String> {
        self.data.lock().map_err(|e| e.to_string())
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

pub struct Sender<T: Send> {
    sender: mpsc::Sender<T>,
}

pub struct Receiver<T: Send> {
    receiver: mpsc::Receiver<T>,
}

impl<T: Send + 'static> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), String> {
        self.sender.send(value).map_err(|_| "Send failed".to_string())
    }
}

impl<T: Send + 'static> Receiver<T> {
    pub fn recv(&self) -> Result<T, String> {
        self.receiver.recv().map_err(|_| "Recv failed".to_string())
    }

    pub fn try_recv(&self) -> Result<Option<T>, String> {
        match self.receiver.try_recv() {
            Ok(v) => Ok(Some(v)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err("Channel disconnected".to_string()),
        }
    }
}

pub fn channel<T: Send + 'static>() -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = mpsc::channel();
    (Sender { sender: tx }, Receiver { receiver: rx })
}

pub fn spawn<F, T>(f: F) -> thread::JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    thread::spawn(f)
}

pub struct Barrier {
    count: Arc<Mutex<usize>>,
    total: usize,
    condvar: Arc<std::sync::Condvar>,
}

impl Barrier {
    pub fn new(count: usize) -> Self {
        Barrier {
            count: Arc::new(Mutex::new(0)),
            total: count,
            condvar: Arc::new(std::sync::Condvar::new()),
        }
    }

    pub fn wait(&self) -> bool {
        let mut count = self.count.lock().unwrap();
        *count += 1;

        if *count == self.total {
            self.condvar.notify_all();
            true
        } else {
            while *count < self.total {
                count = self.condvar.wait(count).unwrap();
            }
            false
        }
    }
}

pub struct RwLock<T> {
    data: Arc<std::sync::RwLock<T>>,
}

impl<T: Send + Sync> RwLock<T> {
    pub fn new(data: T) -> Self {
        RwLock {
            data: Arc::new(std::sync::RwLock::new(data)),
        }
    }

    pub fn read(&self) -> Result<std::sync::RwLockReadGuard<T>, String> {
        self.data.read().map_err(|e| e.to_string())
    }

    pub fn write(&self) -> Result<std::sync::RwLockWriteGuard<T>, String> {
        self.data.write().map_err(|e| e.to_string())
    }
}

pub struct AtomicCounter {
    value: Arc<std::sync::atomic::AtomicUsize>,
}

impl AtomicCounter {
    pub fn new(initial: usize) -> Self {
        AtomicCounter {
            value: Arc::new(std::sync::atomic::AtomicUsize::new(initial)),
        }
    }

    pub fn increment(&self) -> usize {
        self.value
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn decrement(&self) -> usize {
        self.value
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn get(&self) -> usize {
        self.value.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn set(&self, value: usize) {
        self.value
            .store(value, std::sync::atomic::Ordering::SeqCst);
    }
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

enum Message {
    NewJob(Job),
    Terminate,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender
            .send(Message::NewJob(job))
            .map_err(|_| "Failed to send job".to_string())
    }

    pub fn size(&self) -> usize {
        self.workers.len()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            let _ = self.sender.send(Message::Terminate);
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Self {
        let thread = thread::spawn(move || loop {
            // Handle potential mutex poisoning and channel disconnection gracefully
            let receiver_guard = match receiver.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    eprintln!("[Worker {}] ERROR: Mutex poisoned, attempting recovery", id);
                    // Try to recover from poisoned mutex
                    poisoned.into_inner()
                }
            };
            
            match receiver_guard.recv() {
                Ok(message) => {
                    match message {
                        Message::NewJob(job) => {
                            job();
                        }
                        Message::Terminate => {
                            break;
                        }
                    }
                }
                Err(_) => {
                    // Channel disconnected - sender has been dropped
                    eprintln!("[Worker {}] Channel disconnected, terminating", id);
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_creation() {
        let _t = Thread::new(42);
        assert_eq!(_t.id(), 0);
    }

    #[test]
    fn test_channel_send_recv() {
        let (tx, rx) = channel::<i32>();
        tx.send(42).unwrap();
        assert_eq!(rx.recv().unwrap(), 42);
    }

    #[test]
    fn test_atomic_counter() {
        let counter = AtomicCounter::new(0);
        assert_eq!(counter.get(), 0);
        counter.increment();
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn test_rwlock() {
        let lock = RwLock::new(42);
        {
            let value = lock.read().unwrap();
            assert_eq!(*value, 42);
        }
        {
            let mut value = lock.write().unwrap();
            *value = 100;
        }
        assert_eq!(*lock.read().unwrap(), 100);
    }

    #[test]
    fn test_thread_pool() {
        let pool = ThreadPool::new(2);
        let counter = Arc::new(Mutex::new(0));

        for _ in 0..5 {
            let c = Arc::clone(&counter);
            pool.execute(move || {
                let mut num = c.lock().unwrap();
                *num += 1;
            })
            .unwrap();
        }

        drop(pool);
    }
}
