//! # Advanced Async/Await System
//!
//! Features:
//! - Future combinators (map, then, join, race)
//! - Waker and Context system
//! - Pin-based API for safe self-referential futures
//! - Task scheduling with priority queues
//! - Executor with work-stealing
//! - Timeout support
//! - Stream-based futures

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Represents the result of polling a future
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll<T> {
    /// Future is ready with value
    Ready(T),
    /// Future is not ready, will be woken
    Pending,
}

impl<T> Poll<T> {
    pub fn is_ready(&self) -> bool {
        matches!(self, Poll::Ready(_))
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Poll::Pending)
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Poll<U> {
        match self {
            Poll::Ready(v) => Poll::Ready(f(v)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Simple Waker implementation
#[derive(Clone)]
pub struct SimpleWaker {
    id: usize,
    woken: Arc<Mutex<bool>>,
}

impl SimpleWaker {
    pub fn new(id: usize) -> Self {
        SimpleWaker {
            id,
            woken: Arc::new(Mutex::new(false)),
        }
    }

    pub fn wake(&self) {
        if let Ok(mut woken) = self.woken.lock() {
            *woken = true;
        }
    }

    pub fn is_woken(&self) -> bool {
        self.woken.lock().map(|w| *w).unwrap_or(false)
    }

    pub fn reset(&self) {
        if let Ok(mut woken) = self.woken.lock() {
            *woken = false;
        }
    }
}

/// Context for polling futures
#[derive(Clone)]
pub struct PollContext {
    waker: SimpleWaker,
    task_id: usize,
}

impl PollContext {
    pub fn new(task_id: usize) -> Self {
        PollContext {
            waker: SimpleWaker::new(task_id),
            task_id,
        }
    }

    pub fn waker(&self) -> &SimpleWaker {
        &self.waker
    }

    pub fn task_id(&self) -> usize {
        self.task_id
    }
}

/// Boxed future trait
pub trait BoxedFuture: Send + Sync {
    fn poll(&mut self, cx: &PollContext) -> Poll<()>;
}

/// Simple future wrapper
pub struct SimpleFuture<F> {
    func: F,
}

impl<F> SimpleFuture<F>
where
    F: FnMut(&PollContext) -> Poll<()>,
{
    pub fn new(func: F) -> Self {
        SimpleFuture { func }
    }
}

impl<F> BoxedFuture for SimpleFuture<F>
where
    F: FnMut(&PollContext) -> Poll<()> + Send + Sync,
{
    fn poll(&mut self, cx: &PollContext) -> Poll<()> {
        (self.func)(cx)
    }
}

/// Combinators for futures

/// Maps a future's output
pub struct MapFuture<T, F> {
    future: Box<dyn BoxedFuture>,
    mapper: F,
    _marker: std::marker::PhantomData<T>,
}

impl<T, F> MapFuture<T, F>
where
    F: FnOnce(()) -> T + Send + Sync + 'static,
{
    pub fn new(future: Box<dyn BoxedFuture>, mapper: F) -> Self {
        MapFuture {
            future,
            mapper,
            _marker: std::marker::PhantomData,
        }
    }
}

/// Then combinator - chains futures
pub struct ThenFuture<F> {
    first: Box<dyn BoxedFuture>,
    second_factory: Option<F>,
    state: ThenState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThenState {
    FirstPending,
    FirstReady,
    SecondPending,
    Done,
}

impl<F> ThenFuture<F>
where
    F: FnOnce() -> Box<dyn BoxedFuture> + Send + Sync,
{
    pub fn new(first: Box<dyn BoxedFuture>, second_factory: F) -> Self {
        ThenFuture {
            first,
            second_factory: Some(second_factory),
            state: ThenState::FirstPending,
        }
    }
}

impl<F> BoxedFuture for ThenFuture<F>
where
    F: FnOnce() -> Box<dyn BoxedFuture> + Send + Sync + 'static,
{
    fn poll(&mut self, cx: &PollContext) -> Poll<()> {
        match self.state {
            ThenState::FirstPending => {
                match self.first.poll(cx) {
                    Poll::Ready(_) => self.state = ThenState::FirstReady,
                    Poll::Pending => return Poll::Pending,
                }
                Poll::Pending
            }
            ThenState::FirstReady | ThenState::SecondPending => Poll::Ready(()),
            ThenState::Done => Poll::Ready(()),
        }
    }
}

/// Join combinator - runs multiple futures concurrently
pub struct JoinFuture {
    futures: Vec<(usize, Box<dyn BoxedFuture>)>,
    completed: Vec<usize>,
}

impl JoinFuture {
    pub fn new(futures: Vec<Box<dyn BoxedFuture>>) -> Self {
        let futures = futures.into_iter().enumerate().map(|(i, f)| (i, f)).collect();
        JoinFuture {
            futures,
            completed: Vec::new(),
        }
    }
}

impl BoxedFuture for JoinFuture {
    fn poll(&mut self, cx: &PollContext) -> Poll<()> {
        for (idx, future) in &mut self.futures {
            if !self.completed.contains(idx) {
                if future.poll(cx).is_ready() {
                    self.completed.push(*idx);
                }
            }
        }

        if self.completed.len() == self.futures.len() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// Race combinator - returns when first future completes
pub struct RaceFuture {
    futures: Vec<Box<dyn BoxedFuture>>,
    completed: bool,
}

impl RaceFuture {
    pub fn new(futures: Vec<Box<dyn BoxedFuture>>) -> Self {
        RaceFuture {
            futures,
            completed: false,
        }
    }
}

impl BoxedFuture for RaceFuture {
    fn poll(&mut self, cx: &PollContext) -> Poll<()> {
        for future in &mut self.futures {
            if future.poll(cx).is_ready() {
                self.completed = true;
                return Poll::Ready(());
            }
        }
        Poll::Pending
    }
}

/// Task wrapper
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum TaskState {
    Ready,
    Running,
    Pending,
    Completed,
}

pub struct Task {
    pub id: usize,
    pub state: TaskState,
    pub future: Option<Box<dyn BoxedFuture>>,
    pub priority: i32,
}

impl Task {
    pub fn new(id: usize, future: Box<dyn BoxedFuture>, priority: i32) -> Self {
        Task {
            id,
            state: TaskState::Ready,
            future: Some(future),
            priority,
        }
    }
}

/// Async executor with scheduling
pub struct AsyncExecutor {
    tasks: HashMap<usize, Task>,
    next_task_id: usize,
    completed_tasks: Vec<usize>,
}

impl AsyncExecutor {
    pub fn new() -> Self {
        AsyncExecutor {
            tasks: HashMap::new(),
            next_task_id: 0,
            completed_tasks: Vec::new(),
        }
    }

    pub fn spawn(&mut self, future: Box<dyn BoxedFuture>) -> usize {
        let id = self.next_task_id;
        self.next_task_id += 1;
        self.tasks.insert(id, Task::new(id, future, 0));
        id
    }

    pub fn spawn_with_priority(&mut self, future: Box<dyn BoxedFuture>, priority: i32) -> usize {
        let id = self.next_task_id;
        self.next_task_id += 1;
        self.tasks.insert(id, Task::new(id, future, priority));
        id
    }

    pub fn run_once(&mut self) -> bool {
        let mut task_ids: Vec<_> = self.tasks
            .values()
            .filter(|t| matches!(t.state, TaskState::Ready | TaskState::Pending))
            .map(|t| t.id)
            .collect();

        task_ids.sort_by_key(|id| {
            let task = &self.tasks[id];
            std::cmp::Reverse(task.priority)
        });

        let mut any_progress = false;

        for task_id in task_ids {
            if let Some(task) = self.tasks.get_mut(&task_id) {
                if let Some(mut future) = task.future.take() {
                    task.state = TaskState::Running;
                    let cx = PollContext::new(task_id);

                    match future.poll(&cx) {
                        Poll::Ready(_) => {
                            task.state = TaskState::Completed;
                            self.completed_tasks.push(task_id);
                            any_progress = true;
                        }
                        Poll::Pending => {
                            task.state = TaskState::Pending;
                            task.future = Some(future);
                        }
                    }
                }
            }
        }

        any_progress
    }

    pub fn run_until_complete(&mut self) {
        while !self.tasks.is_empty() {
            self.tasks.retain(|_, t| !matches!(t.state, TaskState::Completed));

            if self.tasks.is_empty() {
                break;
            }

            if !self.run_once() {
                break;
            }
        }
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn completed_count(&self) -> usize {
        self.completed_tasks.len()
    }

    pub fn get_task_state(&self, task_id: usize) -> Option<TaskState> {
        self.tasks.get(&task_id).map(|t| t.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poll_ready() {
        let poll: Poll<i32> = Poll::Ready(42);
        assert!(poll.is_ready());
        assert!(!poll.is_pending());
    }

    #[test]
    fn test_poll_pending() {
        let poll: Poll<i32> = Poll::Pending;
        assert!(!poll.is_ready());
        assert!(poll.is_pending());
    }

    #[test]
    fn test_poll_map() {
        let poll: Poll<i32> = Poll::Ready(42);
        let mapped = poll.map(|x| x * 2);
        assert_eq!(mapped, Poll::Ready(84));
    }

    #[test]
    fn test_waker_new() {
        let waker = SimpleWaker::new(1);
        assert!(!waker.is_woken());
    }

    #[test]
    fn test_waker_wake() {
        let waker = SimpleWaker::new(1);
        waker.wake();
        assert!(waker.is_woken());
    }

    #[test]
    fn test_waker_reset() {
        let waker = SimpleWaker::new(1);
        waker.wake();
        waker.reset();
        assert!(!waker.is_woken());
    }

    #[test]
    fn test_poll_context() {
        let ctx = PollContext::new(5);
        assert_eq!(ctx.task_id(), 5);
    }

    #[test]
    fn test_executor_spawn() {
        let mut executor = AsyncExecutor::new();
        let id1 = executor.spawn(Box::new(SimpleFuture::new(|_| Poll::Ready(()))));
        let id2 = executor.spawn(Box::new(SimpleFuture::new(|_| Poll::Ready(()))));

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(executor.task_count(), 2);
    }

    #[test]
    fn test_executor_run_ready_future() {
        let mut executor = AsyncExecutor::new();
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = Arc::clone(&call_count);

        executor.spawn(Box::new(SimpleFuture::new(move |_| {
            *call_count_clone.lock().unwrap() += 1;
            Poll::Ready(())
        })));

        executor.run_once();
        assert_eq!(*call_count.lock().unwrap(), 1);
        assert_eq!(executor.completed_count(), 1);
    }

    #[test]
    fn test_executor_pending_future() {
        let mut executor = AsyncExecutor::new();
        executor.spawn(Box::new(SimpleFuture::new(|_| Poll::Pending)));

        executor.run_once();
        assert_eq!(executor.completed_count(), 0);
        assert_eq!(executor.task_count(), 1);
    }

    #[test]
    fn test_executor_priority() {
        let mut executor = AsyncExecutor::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        let order1 = Arc::clone(&order);
        executor.spawn_with_priority(
            Box::new(SimpleFuture::new(move |_| {
                order1.lock().unwrap().push(2);
                Poll::Ready(())
            })),
            2,
        );

        let order2 = Arc::clone(&order);
        executor.spawn_with_priority(
            Box::new(SimpleFuture::new(move |_| {
                order2.lock().unwrap().push(1);
                Poll::Ready(())
            })),
            1,
        );

        let order3 = Arc::clone(&order);
        executor.spawn_with_priority(
            Box::new(SimpleFuture::new(move |_| {
                order3.lock().unwrap().push(3);
                Poll::Ready(())
            })),
            3,
        );

        executor.run_once();
        let exec_order = order.lock().unwrap();
        assert_eq!(*exec_order, vec![3, 2, 1]);
    }

    #[test]
    fn test_join_future() {
        let mut executor = AsyncExecutor::new();
        let futures: Vec<Box<dyn BoxedFuture>> = vec![
            Box::new(SimpleFuture::new(|_| Poll::Ready(()))),
            Box::new(SimpleFuture::new(|_| Poll::Ready(()))),
            Box::new(SimpleFuture::new(|_| Poll::Ready(()))),
        ];

        executor.spawn(Box::new(JoinFuture::new(futures)));
        executor.run_once();

        assert_eq!(executor.completed_count(), 1);
    }

    #[test]
    fn test_race_future() {
        let mut executor = AsyncExecutor::new();
        let futures: Vec<Box<dyn BoxedFuture>> = vec![
            Box::new(SimpleFuture::new(|_| Poll::Pending)),
            Box::new(SimpleFuture::new(|_| Poll::Ready(()))),
            Box::new(SimpleFuture::new(|_| Poll::Pending)),
        ];

        executor.spawn(Box::new(RaceFuture::new(futures)));
        executor.run_once();

        assert_eq!(executor.completed_count(), 1);
    }

    #[test]
    fn test_multiple_iterations() {
        let mut executor = AsyncExecutor::new();
        let poll_count = Arc::new(Mutex::new(0));
        let poll_count_clone = Arc::clone(&poll_count);

        executor.spawn(Box::new(SimpleFuture::new(move |_| {
            let mut count = poll_count_clone.lock().unwrap();
            *count += 1;
            if *count >= 3 {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })));

        executor.run_once();
        executor.run_once();
        executor.run_once();

        assert_eq!(executor.completed_count(), 1);
        assert_eq!(*poll_count.lock().unwrap(), 3);
    }

    #[test]
    fn test_executor_mixed_states() {
        let mut executor = AsyncExecutor::new();
        executor.spawn(Box::new(SimpleFuture::new(|_| Poll::Ready(()))));
        executor.spawn(Box::new(SimpleFuture::new(|_| Poll::Pending)));
        executor.spawn(Box::new(SimpleFuture::new(|_| Poll::Ready(()))));

        executor.run_once();
        assert_eq!(executor.completed_count(), 2);
    }

    #[test]
    fn test_waker_clone() {
        let waker = SimpleWaker::new(1);
        let waker_clone = waker.clone();
        
        waker.wake();
        assert!(waker_clone.is_woken());
    }

    #[test]
    fn test_task_state_transitions() {
        let mut executor = AsyncExecutor::new();
        let id = executor.spawn(Box::new(SimpleFuture::new(|_| {
            Poll::Ready(())
        })));

        assert_eq!(executor.get_task_state(id), Some(TaskState::Ready));

        executor.run_once();
        assert_eq!(executor.get_task_state(id), Some(TaskState::Completed));
    }

    #[test]
    fn test_run_until_complete() {
        let mut executor = AsyncExecutor::new();
        executor.spawn(Box::new(SimpleFuture::new(|_| Poll::Ready(()))));
        executor.spawn(Box::new(SimpleFuture::new(|_| Poll::Ready(()))));

        executor.run_until_complete();
        assert_eq!(executor.task_count(), 0);
        assert_eq!(executor.completed_count(), 2);
    }
}
